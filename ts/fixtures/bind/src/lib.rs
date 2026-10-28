//! Items outside the subset, bound to hand-written TypeScript: `blob::Blob`, which
//! contains `unsafe`, and the crate's own function and type that read through raw
//! pointers, one of them dropping where Rust drops it.
//!
//! `harness::run` is the entry point both sides of the differential test run.

pub mod harness;
pub mod raw;
/// A second path to the function, which a binding can't also name.
pub use raw::checksum;
#[cfg(target_arch = "wasm32")]
#[path = "../../wasm.rs"]
mod wasm;
#[cfg(target_arch = "wasm32")]
use harness::run as entry;
