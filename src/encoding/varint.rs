//! Variable-length integer encoding matching Go's `encoding/binary`:
//!
//! - `uvarint`  — LEB128: 7-bit groups, low-to-high, MSB=1 means more bytes follow.
//! - `varint`   — zigzag-then-uvarint:  `(x << 1) ^ (x >> 63)` for `i64 → u64`.
//!
//! Maximum uvarint64 length is 10 bytes (9 full + 1 with stop bit).
//!
//! See SPEC.md §2.2 for byte-level conformance tests.
//!
//! TODO(codex): implement. Cross-check byte output against the table in
//! SPEC.md §2.2 and against the C++ tests at
//! `../merlion-tsdb-cpp/tests/encoding/varint_test.cpp`.

#![allow(missing_docs)]
#![allow(dead_code)]

use crate::encoding::ReadError;

/// Maximum number of bytes a uvarint-encoded `u64` can occupy.
pub const MAX_VARINT_LEN64: usize = 10;

/// Encodes `x` into `buf`. Returns the number of bytes written.
///
/// # Panics
/// Panics if `buf.len() < MAX_VARINT_LEN64`.
pub fn put_uvarint(_buf: &mut [u8], _x: u64) -> usize {
    todo!("see SPEC.md §2.2")
}

/// Encodes `x` (zigzag then uvarint) into `buf`. Returns bytes written.
pub fn put_varint(_buf: &mut [u8], _x: i64) -> usize {
    todo!("see SPEC.md §2.2")
}

/// Decodes a uvarint from the start of `buf`. Returns `(value, bytes consumed)`.
pub fn read_uvarint(_buf: &[u8]) -> Result<(u64, usize), ReadError> {
    todo!("see SPEC.md §2.2")
}

/// Decodes a varint from the start of `buf`. Returns `(value, bytes consumed)`.
pub fn read_varint(_buf: &[u8]) -> Result<(i64, usize), ReadError> {
    todo!("see SPEC.md §2.2")
}
