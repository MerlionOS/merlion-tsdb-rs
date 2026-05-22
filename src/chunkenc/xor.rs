//! XOR / Gorilla chunk encoder for float samples.
//!
//! Wire format and algorithm are specified in SPEC.md §3.1. Reference
//! implementations live at:
//! - Go:  `../prometheus/tsdb/chunkenc/xor.go`
//! - C++: `../merlion-tsdb-cpp/src/chunkenc/xor.cpp` (planned)
//!
//! Required surface:
//! - [`XorChunk`] — owns a [`BitWriter`](crate::encoding::bstream::BitWriter)
//!   and exposes `num_samples`, `bytes`, `appender`, `iterator`.
//! - [`XorAppender`] — borrows the chunk's writer and the running encoder
//!   state (last timestamp, last delta, last value, leading/trailing zero
//!   counts). `append(t: i64, v: f64)`.
//! - [`XorIterator`] — owns a [`BitReader`](crate::encoding::bstream::BitReader)
//!   plus running decoder state. `next() -> Option<(i64, f64)>`.
//!
//! TODO(codex): implement per SPEC.md §3.1.

#![allow(missing_docs)]
#![allow(dead_code)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::should_implement_trait)]

use thiserror::Error;

use crate::encoding::ReadError;
use crate::encoding::bstream::{BitReader, BitWriter};
use crate::encoding::varint::{MAX_VARINT_LEN64, put_uvarint, put_varint};

const CHUNK_HEADER_SIZE: usize = 2;
const CHUNK_ALLOCATION_SIZE: usize = 128;
const CHUNK_COMPACT_CAPACITY_THRESHOLD: usize = 32;
const LEADING_SENTINEL: u8 = 0xff;

/// Maximum byte size used by Prometheus for XOR chunks.
pub const MAX_BYTES_PER_XOR_CHUNK: usize = 1024;

/// Errors returned by XOR chunk append/replay operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum XorError {
    /// The encoded chunk could not be read while reconstructing appender state.
    #[error(transparent)]
    Read(#[from] ReadError),
    /// The two-byte sample counter is already at its maximum.
    #[error("xor chunk sample capacity exceeded")]
    ChunkFull,
    /// Timestamps in an XOR chunk must be monotonically non-decreasing.
    #[error("timestamp is older than the previous sample")]
    TimestampOutOfOrder,
}

/// XOR-encoded chunk of `(timestamp, float)` samples.
#[derive(Debug)]
pub struct XorChunk {
    b: BitWriter,
}

/// State machine for appending samples to an [`XorChunk`].
#[derive(Debug)]
pub struct XorAppender<'a> {
    b: &'a mut BitWriter,
    t: i64,
    v: f64,
    t_delta: u64,
    leading: u8,
    trailing: u8,
}

/// Forward-only iterator over samples in an [`XorChunk`].
#[derive(Debug)]
pub struct XorIterator<'a> {
    br: BitReader<'a>,
    num_total: u16,
    num_read: u16,
    t: i64,
    value: f64,
    t_delta: u64,
    leading: u8,
    trailing: u8,
}

impl Default for XorChunk {
    fn default() -> Self {
        Self::new()
    }
}

impl XorChunk {
    #[must_use]
    pub fn new() -> Self {
        let mut stream = Vec::with_capacity(CHUNK_ALLOCATION_SIZE);
        stream.extend_from_slice(&[0, 0]);
        Self {
            b: BitWriter::from_bytes(stream),
        }
    }

    #[must_use]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        assert!(bytes.len() >= CHUNK_HEADER_SIZE);
        Self {
            b: BitWriter::from_bytes(bytes),
        }
    }

    #[must_use]
    pub fn num_samples(&self) -> u16 {
        u16::from_be_bytes([self.bytes()[0], self.bytes()[1]])
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.b.bytes()
    }

    pub fn compact(&mut self) {
        let len = self.b.bytes().len();
        if self.b.capacity() > len + CHUNK_COMPACT_CAPACITY_THRESHOLD {
            let bytes = self.b.bytes().to_vec();
            self.b = BitWriter::from_bytes(bytes);
        }
    }

    pub fn appender(&mut self) -> Result<XorAppender<'_>, XorError> {
        if self.num_samples() == 0 {
            return Ok(XorAppender {
                b: &mut self.b,
                t: i64::MIN,
                v: 0.0,
                t_delta: 0,
                leading: LEADING_SENTINEL,
                trailing: 0,
            });
        }

        let samples = self.iter_samples()?;
        let mut rebuilt = Self::new();
        let (t, v, t_delta, leading, trailing) = {
            let mut app = rebuilt.appender()?;
            for (t, v) in samples {
                app.append(t, v)?;
            }
            (app.t, app.v, app.t_delta, app.leading, app.trailing)
        };
        self.b = rebuilt.b;

        Ok(XorAppender {
            b: &mut self.b,
            t,
            v,
            t_delta,
            leading,
            trailing,
        })
    }

    #[must_use]
    pub fn iterator(&self) -> XorIterator<'_> {
        XorIterator {
            br: BitReader::new(&self.bytes()[CHUNK_HEADER_SIZE..]),
            num_total: self.num_samples(),
            num_read: 0,
            t: i64::MIN,
            value: 0.0,
            t_delta: 0,
            leading: 0,
            trailing: 0,
        }
    }

    fn iter_samples(&self) -> Result<Vec<(i64, f64)>, ReadError> {
        let mut it = self.iterator();
        let mut samples = Vec::with_capacity(self.num_samples() as usize);
        while let Some(sample) = it.next()? {
            samples.push(sample);
        }
        Ok(samples)
    }
}

impl XorAppender<'_> {
    pub fn append(&mut self, t: i64, v: f64) -> Result<(), XorError> {
        let num = u16::from_be_bytes([self.b.bytes()[0], self.b.bytes()[1]]);
        let mut t_delta = 0_u64;

        match num {
            0 => {
                let mut buf = [0; MAX_VARINT_LEN64];
                let n = put_varint(&mut buf, t);
                for &b in &buf[..n] {
                    self.b.write_byte(b);
                }
                self.b.write_bits(v.to_bits(), 64);
            }
            1 => {
                t_delta = timestamp_delta(t, self.t)?;
                let mut buf = [0; MAX_VARINT_LEN64];
                let n = put_uvarint(&mut buf, t_delta);
                for &b in &buf[..n] {
                    self.b.write_byte(b);
                }
                xor_write(self.b, v, self.v, &mut self.leading, &mut self.trailing);
            }
            u16::MAX => return Err(XorError::ChunkFull),
            _ => {
                t_delta = timestamp_delta(t, self.t)?;
                let dod = (t_delta as i64).wrapping_sub(self.t_delta as i64);

                if dod == 0 {
                    self.b.write_bit(false);
                } else if bit_range(dod, 14) {
                    self.b
                        .write_byte(0b10 << 6 | (((dod >> 8) as u8) & ((1 << 6) - 1)));
                    self.b.write_byte(dod as u8);
                } else if bit_range(dod, 17) {
                    self.b.write_bits(0b110, 3);
                    self.b.write_bits(dod as u64, 17);
                } else if bit_range(dod, 20) {
                    self.b.write_bits(0b1110, 4);
                    self.b.write_bits(dod as u64, 20);
                } else {
                    self.b.write_bits(0b1111, 4);
                    self.b.write_bits(dod as u64, 64);
                }

                xor_write(self.b, v, self.v, &mut self.leading, &mut self.trailing);
            }
        }

        self.t = t;
        self.v = v;
        self.t_delta = t_delta;
        self.b.bytes_mut()[..2].copy_from_slice(&(num + 1).to_be_bytes());
        Ok(())
    }
}

impl XorIterator<'_> {
    pub fn next(&mut self) -> Result<Option<(i64, f64)>, ReadError> {
        if self.num_read == self.num_total {
            return Ok(None);
        }

        if self.num_read == 0 {
            self.t = self.br.read_varint()?;
            let v = self.br.read_bits(64)?;
            self.value = f64::from_bits(v);
            self.num_read += 1;
            return Ok(Some((self.t, self.value)));
        }

        if self.num_read == 1 {
            self.t_delta = self.br.read_uvarint()?;
            self.t = self.t.wrapping_add(self.t_delta as i64);
            return self.read_value();
        }

        let dod = self.read_delta_of_delta()?;
        self.t_delta = (self.t_delta as i64).wrapping_add(dod) as u64;
        self.t = self.t.wrapping_add(self.t_delta as i64);
        self.read_value()
    }

    fn read_value(&mut self) -> Result<Option<(i64, f64)>, ReadError> {
        xor_read(
            &mut self.br,
            &mut self.value,
            &mut self.leading,
            &mut self.trailing,
        )?;
        self.num_read += 1;
        Ok(Some((self.t, self.value)))
    }

    fn read_delta_of_delta(&mut self) -> Result<i64, ReadError> {
        let mut d = 0_u8;
        for _ in 0..4 {
            d <<= 1;
            if !self.br.read_bit()? {
                break;
            }
            d |= 1;
        }

        let sz = match d {
            0b0 => return Ok(0),
            0b10 => 14,
            0b110 => 17,
            0b1110 => 20,
            0b1111 => return Ok(self.br.read_bits(64)? as i64),
            _ => unreachable!("prefix reader only emits valid XOR prefixes"),
        };

        let mut bits = self.br.read_bits(sz)?;
        if bits > (1_u64 << (sz - 1)) {
            bits = bits.wrapping_sub(1_u64 << sz);
        }
        Ok(bits as i64)
    }
}

fn timestamp_delta(t: i64, prev: i64) -> Result<u64, XorError> {
    if t < prev {
        return Err(XorError::TimestampOutOfOrder);
    }
    Ok(t.wrapping_sub(prev) as u64)
}

fn bit_range(x: i64, nbits: u8) -> bool {
    -((1_i64 << (nbits - 1)) - 1) <= x && x <= (1_i64 << (nbits - 1))
}

fn xor_write(
    b: &mut BitWriter,
    new_value: f64,
    current_value: f64,
    leading: &mut u8,
    trailing: &mut u8,
) {
    let delta = new_value.to_bits() ^ current_value.to_bits();
    if delta == 0 {
        b.write_bit(false);
        return;
    }

    b.write_bit(true);
    let mut new_leading = delta.leading_zeros() as u8;
    let new_trailing = delta.trailing_zeros() as u8;
    if new_leading >= 32 {
        new_leading = 31;
    }

    if *leading != LEADING_SENTINEL && new_leading >= *leading && new_trailing >= *trailing {
        b.write_bit(false);
        b.write_bits(delta >> *trailing, (64 - *leading - *trailing) as u32);
        return;
    }

    *leading = new_leading;
    *trailing = new_trailing;

    b.write_bit(true);
    b.write_bits(new_leading as u64, 5);

    let sigbits = 64 - new_leading - new_trailing;
    b.write_bits(sigbits as u64, 6);
    b.write_bits(delta >> new_trailing, sigbits as u32);
}

fn xor_read(
    br: &mut BitReader<'_>,
    value: &mut f64,
    leading: &mut u8,
    trailing: &mut u8,
) -> Result<(), ReadError> {
    if !br.read_bit()? {
        return Ok(());
    }

    let (new_trailing, mbits) = if br.read_bit()? {
        let new_leading = br.read_bits(5)? as u8;
        let mut mbits = br.read_bits(6)? as u8;
        if mbits == 0 {
            mbits = 64;
        }
        let new_trailing = 64 - new_leading - mbits;
        *leading = new_leading;
        *trailing = new_trailing;
        (new_trailing, mbits)
    } else {
        (*trailing, 64 - *leading - *trailing)
    };

    let bits = br.read_bits(mbits)?;
    *value = f64::from_bits(value.to_bits() ^ (bits << new_trailing));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(chunk: &XorChunk) -> Result<Vec<(i64, u64)>, ReadError> {
        let mut it = chunk.iterator();
        let mut out = Vec::new();
        while let Some((t, v)) = it.next()? {
            out.push((t, v.to_bits()));
        }
        Ok(out)
    }

    #[test]
    fn new_chunk_has_empty_header() {
        let chunk = XorChunk::new();
        assert_eq!(chunk.num_samples(), 0);
        assert_eq!(chunk.bytes(), &[0, 0]);
    }

    #[test]
    fn default_chunk_matches_new() {
        assert_eq!(XorChunk::default().bytes(), XorChunk::new().bytes());
    }

    #[test]
    fn from_bytes_reads_existing_chunk() {
        let mut chunk = XorChunk::new();
        chunk.appender().unwrap().append(123, 4.5).unwrap();
        let copy = XorChunk::from_bytes(chunk.bytes().to_vec());
        assert_eq!(copy.num_samples(), 1);
        assert_eq!(collect(&copy).unwrap(), vec![(123, 4.5_f64.to_bits())]);
    }

    #[test]
    fn num_samples_tracks_header() {
        let mut chunk = XorChunk::new();
        {
            let mut app = chunk.appender().unwrap();
            app.append(1, 1.0).unwrap();
            app.append(2, 2.0).unwrap();
        }
        assert_eq!(chunk.num_samples(), 2);
        assert_eq!(&chunk.bytes()[..2], &[0, 2]);
    }

    #[test]
    fn bytes_match_first_sample_wire_format() {
        let mut chunk = XorChunk::new();
        chunk.appender().unwrap().append(0, 1.0).unwrap();
        assert_eq!(
            chunk.bytes(),
            &[0x00, 0x01, 0x00, 0x3f, 0xf0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn compact_preserves_bytes() {
        let mut chunk = XorChunk::new();
        chunk.appender().unwrap().append(0, 0.0).unwrap();
        let before = chunk.bytes().to_vec();
        chunk.compact();
        assert_eq!(chunk.bytes(), before);
    }

    #[test]
    fn appender_replays_non_empty_chunk_state() {
        let mut chunk = XorChunk::new();
        {
            let mut app = chunk.appender().unwrap();
            app.append(100, 1.0).unwrap();
            app.append(110, 1.5).unwrap();
        }
        chunk.appender().unwrap().append(120, 2.0).unwrap();
        assert_eq!(
            collect(&chunk).unwrap(),
            vec![
                (100, 1.0_f64.to_bits()),
                (110, 1.5_f64.to_bits()),
                (120, 2.0_f64.to_bits())
            ]
        );
    }

    #[test]
    fn appender_rejects_decreasing_timestamps() {
        let mut chunk = XorChunk::new();
        let mut app = chunk.appender().unwrap();
        app.append(10, 1.0).unwrap();
        assert_eq!(app.append(9, 2.0), Err(XorError::TimestampOutOfOrder));
    }

    #[test]
    fn appender_rejects_sample_count_overflow() {
        let mut chunk = XorChunk::new();
        let mut app = chunk.appender().unwrap();
        for i in 0..u16::MAX {
            app.append(i64::from(i), 0.0).unwrap();
        }
        assert_eq!(
            app.append(i64::from(u16::MAX), 0.0),
            Err(XorError::ChunkFull)
        );
    }

    #[test]
    fn iterator_on_empty_chunk_returns_none() {
        let chunk = XorChunk::new();
        let mut it = chunk.iterator();
        assert_eq!(it.next(), Ok(None));
    }

    #[test]
    fn iterator_roundtrips_delta_of_delta_cases() {
        let samples = [
            (1_000, 1.0),
            (1_010, 1.0),
            (1_020, 2.0),
            (1_031, 2.5),
            (10_000, 3.5),
            (200_000, 4.5),
            (10_000_000, 5.5),
        ];
        let mut chunk = XorChunk::new();
        {
            let mut app = chunk.appender().unwrap();
            for (t, v) in samples {
                app.append(t, v).unwrap();
            }
        }
        assert_eq!(
            collect(&chunk).unwrap(),
            samples.map(|(t, v)| (t, v.to_bits())).to_vec()
        );
    }

    #[test]
    fn xor_value_reader_handles_equal_reused_and_full_width_deltas() {
        let samples = [
            (0, 0.0_f64),
            (1, 0.0_f64),
            (2, 1.0_f64),
            (3, 1.5_f64),
            (4, f64::from_bits(u64::MAX)),
        ];
        let mut chunk = XorChunk::new();
        {
            let mut app = chunk.appender().unwrap();
            for (t, v) in samples {
                app.append(t, v).unwrap();
            }
        }
        assert_eq!(
            collect(&chunk).unwrap(),
            samples.map(|(t, v)| (t, v.to_bits())).to_vec()
        );
    }

    #[test]
    fn malformed_iterator_returns_read_error() {
        let chunk = XorChunk::from_bytes(vec![0, 1]);
        let mut it = chunk.iterator();
        assert_eq!(it.next(), Err(ReadError::EndOfStream));
    }

    #[test]
    fn max_bytes_constant_matches_spec() {
        assert_eq!(MAX_BYTES_PER_XOR_CHUNK, 1024);
    }
}
