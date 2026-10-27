//! The language the driver translates, exercised against native Rust: every integer
//! width's arithmetic and casts, `char`s and the `str` methods over them, control flow,
//! patterns, `Copy` aggregates, closures, traits, generic types, formatting, collections,
//! and the standard library calls it maps.
//!
//! `harness::run` is the entry point both sides of the differential test run, and
//! `fmt::Fault` an error only `caller.ts` prints.

mod chars;
mod chunks;
mod collections;
mod fmt;
mod generics;
pub mod harness;
mod ints;
mod iters;
mod keys;
mod shapes;
mod strs;
#[cfg(target_arch = "wasm32")]
#[path = "../../wasm.rs"]
mod wasm;
#[cfg(target_arch = "wasm32")]
use harness::entry;
