//! Contains encoding and decoding helpers.

// Trimmed from moq-net's coding/mod.rs to the sync codec: the async reader, writer,
// stream, and error-code registries are left out.

mod decode;
mod encode;
pub mod varint;
mod version;

pub use decode::*;
pub use encode::*;
pub use varint::BoundsExceeded;
pub(crate) use varint::Form;
pub use version::*;
