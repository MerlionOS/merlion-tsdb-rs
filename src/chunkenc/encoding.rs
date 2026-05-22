//! Chunk encoding tags as written to disk.
//!
//! Tag bytes must match Go's `tsdb/chunkenc/chunk.go` `Encoding` iota.
//! Do not reorder or insert variants.

/// Identifier byte for each chunk encoding kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Encoding {
    /// No encoding (sentinel; never written to disk).
    None = 0,
    /// XOR / Gorilla compression for float samples (legacy).
    Xor = 1,
    /// Sparse histogram chunk.
    Histogram = 2,
    /// Sparse float-histogram chunk.
    FloatHistogram = 3,
    /// XOR2 (extended XOR with stale-NaN and start-timestamp support).
    Xor2 = 4,
}

/// Logical value kind exposed by a chunk iterator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ValueType {
    /// Iterator is exhausted.
    None = 0,
    /// Simple float sample.
    Float = 1,
    /// Integer histogram sample.
    Histogram = 2,
    /// Float histogram sample.
    FloatHistogram = 3,
}
