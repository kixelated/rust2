//! The language the driver translates, exercised against native Rust: every integer
//! width's arithmetic and casts, `f64`, `char`s and the `str` methods over them,
//! `Cow<str>`, `Duration`, control flow, patterns, byte slices split apart, `Copy`
//! aggregates, closures and what they write of what they captured, traits, generic
//! types, formatting, collections, UTF-8 decoding, `&mut` to values JS copies, whole
//! writes through a `&mut` to an object, the standard library calls it maps, `Option`'s
//! and `Result`'s combinators from core's MIR, `Option`s of values that can already be
//! `undefined`, atomics, `static` ones included, trait objects, one-time cells, mutexes,
//! `Cell`s, `RefCell`s, read-write locks, and thread-locals, the defaults of std types,
//! the counts of shared pointers, arrays `array::from_fn` makes, maps of `Option`s,
//! values held back past a write of a variable they name, what `Hash` writes into
//! `DefaultHasher` and `RandomState`, slices' `windows` and chunks, mutable ones written
//! through, `Vec::drain`, the order of bytes, ranges of `Bound`s and of the crate's
//! own `RangeBounds`, and the order whole B-trees drop in.
//!
//! `harness::run` is the entry point both sides of the differential test run, and
//! `fmt::Fault` an error only `caller.ts` prints.

mod arrays;
mod atomics;
mod borrows;
mod bounds;
mod captures;
mod cells;
mod chars;
mod chunks;
mod collections;
mod combinators;
mod counts;
mod cows;
mod defaults;
mod durations;
mod dyns;
mod edits;
mod floats;
mod fmt;
mod generics;
pub mod harness;
mod hashes;
mod holds;
mod ints;
mod iters;
mod keys;
mod locals;
mod locks;
mod nested;
mod objects;
mod orders;
mod pointers;
mod refs;
mod shapes;
mod slices;
mod splits;
mod statics;
mod strs;
mod trees;
mod utf8;
#[cfg(target_arch = "wasm32")]
#[path = "../../wasm.rs"]
mod wasm;
mod wrappers;
#[cfg(target_arch = "wasm32")]
use harness::entry;
