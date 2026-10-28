//! A package that translates with its dependency, `crates-dep`, as one program.
//!
//! `harness::run` is the entry point both sides of the differential test run.

pub mod harness;
#[cfg(feature = "shared")]
mod crates_dep {}
#[cfg(target_arch = "wasm32")]
#[path = "../../wasm.rs"]
mod wasm;
#[cfg(target_arch = "wasm32")]
use harness::run as entry;
