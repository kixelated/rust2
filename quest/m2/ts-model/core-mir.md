# [M] Std combinators from core's MIR

## Goal

rust2-ts lowers `Option`'s and `Result`'s combinators that take `self` or `&self`
(`is_some_and`, `is_none_or`, `and_then`, `filter`, `or_else`, `map_or_else`,
`unwrap_or_default`, `zip`, `Clone`, and the ones `library` maps today) from core's own
MIR instead of a runtime function each, with drops and unwinding exact, proven by
differential tests against native Rust, and `just ts bench` and the codec's bundle no
worse than today's.

## Plan

Spiked on 2026-10-27 on a local branch: lowering these from core's MIR as the crate's own
instances passed the differential tests, cut moq-net's refusals of these calls from 1,175
to 225, and tied the runtime in Chrome, Node, and Bun once an `Ok(x)` rebuilt from an
`Ok(x)` reused the object (Node measured 1.31x slower without that).

Decided with the maintainer on 2026-10-27:

- Build std from source for exact unwinding: the wasm32 sysroot is `panic=abort`, so
  core's MIR has no cleanup blocks, and lowered as is, `task`'s unwinding scripts lose
  drops. Pass `-Zbuild-std=core,alloc,std,panic_unwind,panic_abort`, with
  `-Cpanic=unwind -Zalways-encode-mir` on core, alloc, and std through cargo's profiles
  (`-Zprofile-rustflags`) so cargo's fingerprint sees them. `panic_abort` stays aborting,
  since a `cdylib` dependency such as `tracing-wasm` links it. The driver's `cargo clean`
  needs `CARGO_UNSTABLE_PROFILE_RUSTFLAGS=true`. The spike measured moq-net's cold
  check 42 s against 33 s, and warm unchanged.
- Keep one instance per closure, so each call site stays monomorphic, and merge
  functions whose output is identical: an instance per closure grew moq-net's census
  from 4,922 to 6,385 functions.
- `Option` and `Result` only. `Bound`, `Reverse`, `NonZero`, and `Duration` need core's
  types translated, with pattern types and niche transmutes.

Also:

- Integer methods stay in the runtime: core's plain MIR `Add` wraps, and the translator
  lowers it as checked. `UbChecks` is false, `cold_path` is nothing, and a
  `precondition_check` block is unreachable.
- Refusals inside a core body point at the crate's call site.
- `&mut self` methods (`take`, `replace`, `insert`, `get_or_insert*`, `as_mut`), `unwrap`
  and `expect`, and the single-expression mappings (`is_some`, `as_ref`) keep `library`'s.

Public API: the translator's output; the translation's build compiles std from source.
