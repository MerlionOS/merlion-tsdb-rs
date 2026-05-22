//! Low-level encoding primitives shared across TSDB subsystems.
//!
//! - [`bstream`] — MSB-first bit stream reader/writer (matches Go's
//!   `tsdb/chunkenc/bstream.go`).
//! - [`varint`] — Go-style LEB128 uvarint / zigzag varint (matches
//!   `encoding/binary.PutUvarint` / `PutVarint`).
//!
//! See SPEC.md §2 (Wire format conventions) for byte-level guarantees.

pub mod bstream;
pub mod varint;

mod error;
pub use error::ReadError;
