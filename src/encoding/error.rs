use thiserror::Error;

/// Errors returned by [`crate::encoding`] readers.
///
/// Mirrors the categories used by upstream Go:
/// - `io.EOF` → [`ReadError::EndOfStream`]
/// - `io.ErrUnexpectedEOF` → [`ReadError::UnexpectedEnd`]
/// - varint overflow → [`ReadError::VarintOverflow`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ReadError {
    /// Reached the end of the stream at a clean byte boundary.
    #[error("end of stream")]
    EndOfStream,
    /// Stream ended in the middle of a multi-byte value.
    #[error("unexpected end of stream")]
    UnexpectedEnd,
    /// Varint payload exceeded the 10-byte maximum for uint64.
    #[error("varint overflow")]
    VarintOverflow,
}
