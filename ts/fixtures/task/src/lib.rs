//! A kio-shaped channel and the async code moq builds on it, exercised against native
//! Rust: laziness, poll and wake order, and every drop, including a future's on cancel.
//!
//! `kio` trims moq's kio to what its futures hinge on: a `Waiter` holding the task's
//! waker behind an `Arc` that `WaiterList`s hold weakly, `wait`, and a channel whose
//! last producer closes it on drop. `model` holds the async functions. `exec` is a
//! deterministic FIFO executor that logs every poll, wake, and waker clone and drop, and
//! `harness` runs a script of executor steps through it; `tests/task.ts` runs the same
//! script on the translation with its own executor. Neither is translated.

pub mod exec;
pub mod harness;
pub mod kio;
pub mod log;
pub mod model;
#[cfg(target_arch = "wasm32")]
#[path = "../../wasm.rs"]
mod wasm;
#[cfg(target_arch = "wasm32")]
use harness::run as entry;
