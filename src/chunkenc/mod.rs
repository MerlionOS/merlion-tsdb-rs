//! Chunk encoders and decoders.
//!
//! See SPEC.md §3 for the wire formats. The first encoder to implement is
//! [`xor`] (Gorilla XOR), which is the default for float samples in v3.x blocks.

pub mod xor;

mod encoding;
pub use encoding::{Encoding, ValueType};
