//! The language the driver translates, exercised against native Rust: every integer
//! width's arithmetic and casts, `char`s and the `str` methods over them, `Cow<str>`,
//! `Duration`, control flow, patterns, byte slices split apart, `Copy` aggregates,
//! closures, traits, generic types, formatting, collections, UTF-8 decoding, `&mut` to
//! values JS copies, the standard library calls it maps, and `Option`'s and `Result`'s
//! combinators from core's MIR.
//!
//! `harness::run` is the entry point both sides of the differential test run, and
//! `fmt::Fault` an error only `caller.ts` prints.

mod chars;
mod chunks;
mod collections;
mod combinators;
mod cows;
mod durations;
mod fmt;
mod generics;
pub mod harness;
mod ints;
mod iters;
mod keys;
mod refs;
mod shapes;
mod splits;
mod strs;
mod utf8;
#[cfg(target_arch = "wasm32")]
#[path = "../../wasm.rs"]
mod wasm;
#[cfg(target_arch = "wasm32")]
use harness::entry;
