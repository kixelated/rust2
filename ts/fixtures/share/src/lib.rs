//! `Copy` aggregates whose only write in the crate makes them copied, where any other
//! write into the same type, as `lang` has, would hide a write the analysis misses.
//!
//! `harness::run` is the entry point both sides of the differential test run.

pub mod harness;
#[cfg(target_arch = "wasm32")]
#[path = "../../wasm.rs"]
mod wasm;
#[cfg(target_arch = "wasm32")]
use harness::run as entry;
