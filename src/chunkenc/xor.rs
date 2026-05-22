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

/// XOR-encoded chunk of `(timestamp, float)` samples.
#[derive(Debug)]
pub struct XorChunk {
    // TODO(codex): BitWriter
}

/// State machine for appending samples to an [`XorChunk`].
#[derive(Debug)]
pub struct XorAppender<'a> {
    _phantom: core::marker::PhantomData<&'a mut XorChunk>,
}

/// Forward-only iterator over samples in an [`XorChunk`].
#[derive(Debug)]
pub struct XorIterator<'a> {
    _phantom: core::marker::PhantomData<&'a [u8]>,
}
