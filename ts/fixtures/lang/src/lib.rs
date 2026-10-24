//! The language the driver translates, exercised against native Rust: every integer
//! width's arithmetic and casts, control flow, patterns, `Copy` aggregates, closures,
//! traits, formatting, and the standard library calls it maps.
//!
//! `harness::run` is the entry point both sides of the differential test run, and
//! `fmt::Fault` an error only `caller.ts` prints.

mod fmt;
pub mod harness;
mod ints;
mod iters;
mod shapes;
#[cfg(target_arch = "wasm32")]
#[path = "../../wasm.rs"]
mod wasm;
#[cfg(target_arch = "wasm32")]
use harness::entry;
