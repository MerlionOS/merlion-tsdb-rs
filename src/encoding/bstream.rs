//! Bit-level stream reader and writer.
//!
//! Bytes are written MSB-first within each byte; the stream grows left-to-right.
//! This layout is **required** for binary compatibility with Prometheus's
//! `tsdb/chunkenc/bstream.go` (derived from `github.com/dgryski/go-tsz`).
//!
//! See SPEC.md §2.3 for the exact wire layout, including the writeByte
//! cross-boundary semantics and the reader's 8-byte fast-path buffer.
//!
//! TODO(codex): implement [`BitWriter`] and [`BitReader`]. The C++ reference
//! lives at `../merlion-tsdb-cpp/src/encoding/bstream.cpp`; the Go reference is
//! at `../prometheus/tsdb/chunkenc/bstream.go`. Behaviour must match both
//! bit-for-bit. Required public surface, error variants, and invariants are
//! all spelled out in SPEC.md §2.3.

#![allow(missing_docs)]
#![allow(dead_code)]
#![allow(clippy::cast_possible_wrap)]

use crate::encoding::ReadError;
use crate::encoding::varint::MAX_VARINT_LEN64;

/// MSB-first bit stream writer.
#[derive(Debug, Default, Clone)]
pub struct BitWriter {
    stream: Vec<u8>,
    count: u8,
}

impl BitWriter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adopts an existing byte buffer. The next write begins on a fresh byte
    /// (Go's count==0 semantics).
    #[must_use]
    pub fn from_bytes(stream: Vec<u8>) -> Self {
        Self { stream, count: 0 }
    }

    pub fn write_bit(&mut self, bit: bool) {
        if self.count == 0 {
            self.stream.push(0);
            self.count = 8;
        }
        if bit {
            let last = self.stream.last_mut().expect("stream has a current byte");
            *last |= 1 << (self.count - 1);
        }
        self.count -= 1;
    }

    pub fn write_byte(&mut self, byt: u8) {
        if self.count == 0 {
            self.stream.push(byt);
            return;
        }
        let last = self.stream.last_mut().expect("stream has a partial byte");
        *last |= byt >> (8 - self.count);
        self.stream.push(byt << self.count);
    }

    /// Writes the `nbits` right-most bits of `u`, MSB-first. Requires 0 ≤ nbits ≤ 64.
    pub fn write_bits(&mut self, mut u: u64, mut nbits: u32) {
        assert!(nbits <= 64);
        if nbits == 0 {
            return;
        }

        u <<= 64 - nbits;
        while nbits >= 8 {
            self.write_byte((u >> 56) as u8);
            u <<= 8;
            nbits -= 8;
        }
        while nbits > 0 {
            self.write_bit((u >> 63) != 0);
            u <<= 1;
            nbits -= 1;
        }
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.stream
    }

    pub fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.stream
    }

    pub(crate) fn capacity(&self) -> usize {
        self.stream.capacity()
    }

    pub fn reset(&mut self) {
        self.stream.clear();
        self.count = 0;
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.stream
    }
}

/// MSB-first bit stream reader with an 8-byte buffered fast path.
#[derive(Debug)]
pub struct BitReader<'a> {
    stream: &'a [u8],
    stream_offset: usize,
    buffer: u64,
    valid: u8,
    last: u8,
}

impl<'a> BitReader<'a> {
    #[must_use]
    pub fn new(stream: &'a [u8]) -> Self {
        Self {
            stream,
            stream_offset: 0,
            buffer: 0,
            valid: 0,
            last: stream.last().copied().unwrap_or(0),
        }
    }

    pub fn read_bit(&mut self) -> Result<bool, ReadError> {
        if self.valid == 0 && !self.load_next_buffer(1) {
            return Err(ReadError::EndOfStream);
        }
        self.valid -= 1;
        Ok((self.buffer & (1_u64 << self.valid)) != 0)
    }

    pub fn read_byte(&mut self) -> Result<u8, ReadError> {
        self.read_bits(8).map(|v| v as u8)
    }

    pub fn read_bits(&mut self, nbits: u8) -> Result<u64, ReadError> {
        assert!(nbits <= 64);
        if nbits == 0 {
            return Ok(0);
        }

        if self.valid == 0 && !self.load_next_buffer(nbits) {
            return Err(ReadError::EndOfStream);
        }

        if nbits <= self.valid {
            let mask = if nbits == 64 {
                u64::MAX
            } else {
                (1_u64 << nbits) - 1
            };
            self.valid -= nbits;
            return Ok((self.buffer >> self.valid) & mask);
        }

        let low_mask = (1_u64 << self.valid) - 1;
        let remaining = nbits - self.valid;
        let mut v = (self.buffer & low_mask) << remaining;
        self.valid = 0;

        if !self.load_next_buffer(remaining) {
            return Err(ReadError::EndOfStream);
        }
        let hi_mask = (1_u64 << remaining) - 1;
        v |= (self.buffer >> (self.valid - remaining)) & hi_mask;
        self.valid -= remaining;
        Ok(v)
    }

    pub fn read_uvarint(&mut self) -> Result<u64, ReadError> {
        let mut x = 0_u64;
        let mut s = 0_u32;
        for i in 0..MAX_VARINT_LEN64 {
            let b = match self.read_byte() {
                Ok(b) => b,
                Err(ReadError::EndOfStream) if i > 0 => return Err(ReadError::UnexpectedEnd),
                Err(e) => return Err(e),
            };
            if b < 0x80 {
                if i == MAX_VARINT_LEN64 - 1 && b > 1 {
                    return Err(ReadError::VarintOverflow);
                }
                return Ok(x | ((b as u64) << s));
            }
            x |= ((b & 0x7f) as u64) << s;
            s += 7;
        }
        Err(ReadError::VarintOverflow)
    }

    pub fn read_varint(&mut self) -> Result<i64, ReadError> {
        let ux = self.read_uvarint()?;
        let mut x = (ux >> 1) as i64;
        if ux & 1 != 0 {
            x = !x;
        }
        Ok(x)
    }

    #[must_use]
    pub fn at_end(&self) -> bool {
        self.valid == 0 && self.stream_offset >= self.stream.len()
    }

    fn load_next_buffer(&mut self, nbits_min: u8) -> bool {
        if self.stream_offset >= self.stream.len() {
            return false;
        }

        if self.stream_offset + 8 < self.stream.len() {
            let bytes: [u8; 8] = self.stream[self.stream_offset..self.stream_offset + 8]
                .try_into()
                .expect("slice length is exactly 8");
            self.buffer = u64::from_be_bytes(bytes);
            self.stream_offset += 8;
            self.valid = 64;
            return true;
        }

        let remaining = self.stream.len() - self.stream_offset;
        let nbytes = usize::min((nbits_min as usize / 8) + 1, remaining);
        let mut buffer = 0_u64;
        let mut skip = 0;
        if self.stream_offset + nbytes == self.stream.len() {
            buffer |= self.last as u64;
            skip = 1;
        }

        for i in 0..(nbytes - skip) {
            buffer |= (self.stream[self.stream_offset + i] as u64) << (8 * (nbytes - i - 1));
        }

        self.buffer = buffer;
        self.stream_offset += nbytes;
        self.valid = (nbytes * 8) as u8;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::varint::{put_uvarint, put_varint};

    #[test]
    fn new_starts_empty() {
        let w = BitWriter::new();
        assert!(w.bytes().is_empty());
    }

    #[test]
    fn from_bytes_adopts_aligned_buffer() {
        let mut w = BitWriter::from_bytes(vec![0xab]);
        w.write_bit(true);
        assert_eq!(w.bytes(), &[0xab, 0x80]);
    }

    #[test]
    fn write_bit_packs_msb_first() {
        let mut w = BitWriter::new();
        for b in [true, false, true, true, false, false, true, false] {
            w.write_bit(b);
        }
        assert_eq!(w.bytes(), &[0xb2]);
    }

    #[test]
    fn write_byte_crosses_partial_boundary() {
        let mut w = BitWriter::new();
        w.write_bit(true);
        w.write_byte(0xab);
        assert_eq!(w.bytes(), &[0xd5, 0x80]);
    }

    #[test]
    fn write_bits_zero_is_noop() {
        let mut w = BitWriter::new();
        w.write_bits(0xffff, 0);
        assert!(w.bytes().is_empty());
    }

    #[test]
    fn write_bits_preserves_byte_semantics() {
        let mut a = BitWriter::new();
        let mut b = BitWriter::new();
        a.write_bits(0xbeef, 16);
        b.write_byte(0xbe);
        b.write_byte(0xef);
        assert_eq!(a.bytes(), b.bytes());
    }

    #[test]
    fn bytes_mut_can_rewrite_existing_bytes() {
        let mut w = BitWriter::from_bytes(vec![0, 0]);
        w.bytes_mut().copy_from_slice(&[0x12, 0x34]);
        assert_eq!(w.bytes(), &[0x12, 0x34]);
    }

    #[test]
    fn reset_clears_stream() {
        let mut w = BitWriter::new();
        w.write_byte(0xff);
        w.reset();
        assert!(w.bytes().is_empty());
    }

    #[test]
    fn into_bytes_returns_owned_stream() {
        let mut w = BitWriter::new();
        w.write_byte(0xab);
        assert_eq!(w.into_bytes(), vec![0xab]);
    }

    #[test]
    fn read_bit_roundtrips_single_bits() {
        let mut w = BitWriter::new();
        let bits = [true, false, true, false, true, true, false, false, true];
        for bit in bits {
            w.write_bit(bit);
        }
        let mut r = BitReader::new(w.bytes());
        for bit in bits {
            assert_eq!(r.read_bit(), Ok(bit));
        }
    }

    #[test]
    fn read_byte_reads_aligned_and_unaligned_bytes() {
        let mut w = BitWriter::new();
        w.write_byte(0xab);
        w.write_bit(true);
        w.write_byte(0x55);
        let mut r = BitReader::new(w.bytes());
        assert_eq!(r.read_byte(), Ok(0xab));
        assert_eq!(r.read_bit(), Ok(true));
        assert_eq!(r.read_byte(), Ok(0x55));
    }

    #[test]
    fn read_bits_roundtrips_variable_lengths() {
        let writes = [
            (0, 0),
            (1, 1),
            (0b10, 2),
            (0xbeef, 16),
            (0x0123_4567_89ab_cdef, 64),
            (0x12345, 17),
            (0, 3),
        ];
        let mut w = BitWriter::new();
        for (v, nbits) in writes {
            w.write_bits(v, nbits);
        }
        let mut r = BitReader::new(w.bytes());
        for (v, nbits) in writes {
            assert_eq!(r.read_bits(nbits as u8), Ok(v));
        }
    }

    #[test]
    fn read_bits_spans_buffer_boundary() {
        let mut w = BitWriter::new();
        for _ in 0..20 {
            w.write_bits(0xdead_beef_cafe_babe, 64);
        }
        let mut r = BitReader::new(w.bytes());
        for _ in 0..20 {
            assert_eq!(r.read_bits(64), Ok(0xdead_beef_cafe_babe));
        }
    }

    #[test]
    fn read_uvarint_roundtrips_through_bit_stream() {
        let mut w = BitWriter::new();
        let mut buf = [0; MAX_VARINT_LEN64];
        for value in [7, 300, u64::MAX] {
            let n = put_uvarint(&mut buf, value);
            for &b in &buf[..n] {
                w.write_byte(b);
            }
        }
        let mut r = BitReader::new(w.bytes());
        assert_eq!(r.read_uvarint(), Ok(7));
        assert_eq!(r.read_uvarint(), Ok(300));
        assert_eq!(r.read_uvarint(), Ok(u64::MAX));
    }

    #[test]
    fn read_varint_roundtrips_through_bit_stream() {
        let mut w = BitWriter::new();
        let mut buf = [0; MAX_VARINT_LEN64];
        for value in [0, -1, 1, -42, 1_000_000] {
            let n = put_varint(&mut buf, value);
            for &b in &buf[..n] {
                w.write_byte(b);
            }
        }
        let mut r = BitReader::new(w.bytes());
        for value in [0, -1, 1, -42, 1_000_000] {
            assert_eq!(r.read_varint(), Ok(value));
        }
    }

    #[test]
    fn read_errors_at_end_of_stream() {
        let mut w = BitWriter::new();
        w.write_byte(0xff);
        let mut r = BitReader::new(w.bytes());
        assert_eq!(r.read_bits(8), Ok(0xff));
        assert_eq!(r.read_bit(), Err(ReadError::EndOfStream));
        assert!(r.at_end());
    }

    #[test]
    fn read_uvarint_reports_truncated_after_continuation() {
        let mut w = BitWriter::new();
        w.write_byte(0x80);
        let mut r = BitReader::new(w.bytes());
        assert_eq!(r.read_uvarint(), Err(ReadError::UnexpectedEnd));
    }
}
