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
#![allow(clippy::cast_possible_wrap)]

use crate::encoding::ReadError;

/// Maximum number of bytes a uvarint-encoded `u64` can occupy.
pub const MAX_VARINT_LEN64: usize = 10;

/// Encodes `x` into `buf`. Returns the number of bytes written.
///
/// # Panics
/// Panics if `buf.len() < MAX_VARINT_LEN64`.
pub fn put_uvarint(buf: &mut [u8], mut x: u64) -> usize {
    assert!(buf.len() >= MAX_VARINT_LEN64);
    let mut i = 0;
    while x >= 0x80 {
        buf[i] = (x as u8) | 0x80;
        x >>= 7;
        i += 1;
    }
    buf[i] = x as u8;
    i + 1
}

/// Encodes `x` (zigzag then uvarint) into `buf`. Returns bytes written.
pub fn put_varint(buf: &mut [u8], x: i64) -> usize {
    let ux = ((x as u64) << 1) ^ ((x >> 63) as u64);
    put_uvarint(buf, ux)
}

/// Decodes a uvarint from the start of `buf`. Returns `(value, bytes consumed)`.
pub fn read_uvarint(buf: &[u8]) -> Result<(u64, usize), ReadError> {
    if buf.is_empty() {
        return Err(ReadError::EndOfStream);
    }

    let mut x = 0_u64;
    let mut s = 0_u32;
    for (i, &b) in buf.iter().enumerate() {
        if i == MAX_VARINT_LEN64 {
            return Err(ReadError::VarintOverflow);
        }
        if b < 0x80 {
            if i == MAX_VARINT_LEN64 - 1 && b > 1 {
                return Err(ReadError::VarintOverflow);
            }
            return Ok((x | ((b as u64) << s), i + 1));
        }
        x |= ((b & 0x7f) as u64) << s;
        s += 7;
    }
    Err(ReadError::UnexpectedEnd)
}

/// Decodes a varint from the start of `buf`. Returns `(value, bytes consumed)`.
pub fn read_varint(buf: &[u8]) -> Result<(i64, usize), ReadError> {
    let (ux, n) = read_uvarint(buf)?;
    let mut x = (ux >> 1) as i64;
    if ux & 1 != 0 {
        x = !x;
    }
    Ok((x, n))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_uvarint(x: u64) -> Vec<u8> {
        let mut buf = [0; MAX_VARINT_LEN64];
        let n = put_uvarint(&mut buf, x);
        buf[..n].to_vec()
    }

    fn encode_varint(x: i64) -> Vec<u8> {
        let mut buf = [0; MAX_VARINT_LEN64];
        let n = put_varint(&mut buf, x);
        buf[..n].to_vec()
    }

    #[test]
    fn put_uvarint_known_bytes() {
        assert_eq!(encode_uvarint(0), [0x00]);
        assert_eq!(encode_uvarint(1), [0x01]);
        assert_eq!(encode_uvarint(127), [0x7f]);
        assert_eq!(encode_uvarint(128), [0x80, 0x01]);
        assert_eq!(encode_uvarint(300), [0xac, 0x02]);
        assert_eq!(encode_uvarint(16_384), [0x80, 0x80, 0x01]);
        assert_eq!(
            encode_uvarint(u64::MAX),
            [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01]
        );
    }

    #[test]
    fn put_varint_known_bytes() {
        assert_eq!(encode_varint(0), [0x00]);
        assert_eq!(encode_varint(-1), [0x01]);
        assert_eq!(encode_varint(1), [0x02]);
        assert_eq!(encode_varint(-2), [0x03]);
        assert_eq!(encode_varint(63), [0x7e]);
        assert_eq!(encode_varint(-64), [0x7f]);
        assert_eq!(encode_varint(64), [0x80, 0x01]);
    }

    #[test]
    fn read_uvarint_roundtrip_and_consumed_len() {
        for v in [0, 1, 127, 128, 300, 16_384, u64::MAX] {
            let bytes = encode_uvarint(v);
            assert_eq!(read_uvarint(&bytes), Ok((v, bytes.len())));
        }
    }

    #[test]
    fn read_varint_roundtrip_and_consumed_len() {
        for v in [i64::MIN, -1_000_000, -64, -1, 0, 1, 64, 1_000_000, i64::MAX] {
            let bytes = encode_varint(v);
            assert_eq!(read_varint(&bytes), Ok((v, bytes.len())));
        }
    }

    #[test]
    fn read_uvarint_errors() {
        assert_eq!(read_uvarint(&[]), Err(ReadError::EndOfStream));
        assert_eq!(read_uvarint(&[0x80, 0x80]), Err(ReadError::UnexpectedEnd));
        assert_eq!(read_uvarint(&[0x80; 11]), Err(ReadError::VarintOverflow));
        assert_eq!(
            read_uvarint(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x02]),
            Err(ReadError::VarintOverflow)
        );
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn put_uvarint_requires_ten_bytes() {
        let mut buf = [0; MAX_VARINT_LEN64 - 1];
        let _ = put_uvarint(&mut buf, 1);
    }
}
