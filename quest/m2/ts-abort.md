# [L] Abort on panic in the TypeScript translator

## Goal

[rust2-ts](/ts/README.md) aborts on panic, as native `wasm32-unknown-unknown` does:
a panic throws `rt.Panic` with std's message and runs no cleanup, and translated
state after a panic is undefined, like a wasm trap. Everything that existed only for
unwinding is deleted: the MIR cleanup lowering, the runtime's drop-on-panic paths,
std built with `-C panic=unwind`, the host unwind harness, and refusals that existed
only because unwinding was hard. Output without panics is unchanged, proven by the
differential tests.

## Plan

Decided with the maintainer on 2026-10-28: a hard crash for panics is fine. It
matches native wasm32 and deletes code. Measure `just ts bench` and `just ts census`
before and after, and a cold `cargo check` if build-std changes.
