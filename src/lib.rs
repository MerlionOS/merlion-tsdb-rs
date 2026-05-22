//! Modern Rust reimplementation of the Prometheus TSDB storage engine.
//!
//! This crate is wire-format compatible with upstream Prometheus v3.x blocks,
//! WAL segments, and chunk files. The on-disk formats are specified in
//! [SPEC.md](../SPEC.md); the upstream Go reference lives in the sibling
//! `prometheus/prometheus/tsdb/` directory.
//!
//! See also [`merlion-tsdb-cpp`](https://github.com/MerlionOS/merlion-tsdb-cpp)
//! for the parallel C++ implementation.

#![warn(missing_docs)]

pub mod encoding;
pub mod chunkenc;
pub mod wal;
pub mod head;
pub mod block;
