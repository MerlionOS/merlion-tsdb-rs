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

use crate::encoding::ReadError;

/// MSB-first bit stream writer.
#[derive(Debug, Default, Clone)]
pub struct BitWriter {
    // TODO(codex): stream: Vec<u8>, count: u8 (free bits in last byte; 0 = need new byte)
}

impl BitWriter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adopts an existing byte buffer. The next write begins on a fresh byte
    /// (Go's count==0 semantics).
    #[must_use]
    pub fn from_bytes(_stream: Vec<u8>) -> Self {
        todo!("see SPEC.md §2.3 — BitWriter::from_bytes")
    }

    pub fn write_bit(&mut self, _bit: bool) {
        todo!("see SPEC.md §2.3 — BitWriter::write_bit")
    }

    pub fn write_byte(&mut self, _byt: u8) {
        todo!("see SPEC.md §2.3 — BitWriter::write_byte (crosses byte boundaries)")
    }

    /// Writes the `nbits` right-most bits of `u`, MSB-first. Requires 0 ≤ nbits ≤ 64.
    pub fn write_bits(&mut self, _u: u64, _nbits: u32) {
        todo!("see SPEC.md §2.3 — BitWriter::write_bits")
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        todo!("see SPEC.md §2.3")
    }

    pub fn bytes_mut(&mut self) -> &mut [u8] {
        todo!("see SPEC.md §2.3 — mutable for XOR header rewrites")
    }

    pub fn reset(&mut self) {
        todo!("see SPEC.md §2.3")
    }

    pub fn into_bytes(self) -> Vec<u8> {
        todo!("see SPEC.md §2.3")
    }
}

/// MSB-first bit stream reader with an 8-byte buffered fast path.
#[derive(Debug)]
pub struct BitReader<'a> {
    // TODO(codex): stream: &'a [u8], stream_offset: usize, buffer: u64,
    //              valid: u8, last: u8 (cached tail byte — see §2.3 TOCTOU guard)
    _phantom: core::marker::PhantomData<&'a [u8]>,
}

impl<'a> BitReader<'a> {
    #[must_use]
    pub fn new(_stream: &'a [u8]) -> Self {
        todo!("see SPEC.md §2.3")
    }

    pub fn read_bit(&mut self) -> Result<bool, ReadError> {
        todo!("see SPEC.md §2.3 — BitReader::read_bit")
    }

    pub fn read_byte(&mut self) -> Result<u8, ReadError> {
        todo!("see SPEC.md §2.3")
    }

    pub fn read_bits(&mut self, _nbits: u8) -> Result<u64, ReadError> {
        todo!("see SPEC.md §2.3 — beware UB at nbits == 64 (don't shift by 64)")
    }

    pub fn read_uvarint(&mut self) -> Result<u64, ReadError> {
        todo!("see SPEC.md §2.2")
    }

    pub fn read_varint(&mut self) -> Result<i64, ReadError> {
        todo!("see SPEC.md §2.2")
    }

    #[must_use]
    pub fn at_end(&self) -> bool {
        todo!("see SPEC.md §2.3")
    }
}
