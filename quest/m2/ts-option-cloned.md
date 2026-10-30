# [S] Option::cloned through Clone

## Goal

[rust2-ts](/ts/README.md) translates `Option::cloned` through the payload's `Clone`, as
std does. An `Option<&Arc<T>>` adds the strong count its clone later drops, and a `Clone`
written by hand runs. Differential tests against native wasm32 prove it.

## Plan

Found on 2026-10-29 while writing [#238](https://github.com/kixelated/rust2/pull/238)'s
fixture. `library.rs` maps `("Option", "copied" | "cloned")` to `Translator::copied`, which
copies JS values but never calls `Clone`. So `v.first().cloned()` of a `Vec<Arc<T>>` took
no count, and its drop released one the `Arc`'s owners still held. The value then dropped
early, which the lang differential caught as a different drop order. This is a silent
miscompile: the output type-checks.

- `cloned` maps the payload as `Iterator::cloned` does in `iter.rs`. A type whose `Clone`
  only copies (`clone_copies`) goes through the copy shim, and any other through
  `Clone::clone`. `None` passes through. `copied` stays as it is.
- `clone_copies` assumes a `Copy` type. For a std type it checks only the type arguments,
  so `Arc<u8>` and `Vec<Arc<T>>` pass as copying. `Iterator::cloned` still reaches `Clone`
  for an `Arc`, but takes the copy shim for a `Vec<Arc<T>>` or a derived struct holding an
  `Arc`. Check whether that shim clones each `Arc`, and fix every `cloned` that shares
  the gap here.
- Tests extend `ts/fixtures/lang/src/counts.rs`. They cover `cloned` of an `Option<&Arc>`,
  an `Option<&Rc>`, an `Option<&mut Arc>`, a struct holding an `Arc` with a derived
  `Clone`, and a hand-written `Clone`, both through `Option` and through `Iterator`. They
  check the counts and the drop log.
- Milestone m2: moq-net and kio, the first consumers, call `Option::cloned` on map lookups
  and handles, for example `kio/src/log.rs` and moq-net's `ietf/subscriber.rs`. So
  [the model's translation](/quest/m2/ts-model/README.md) would miscount silently. Not m0,
  since no consumer runs translated output yet. Not m3, which holds std that no consumer
  calls.

## Related

- [Shared ownership](/quest/m2/ts-shared.md) - the exact `Arc` counts this breaks
- [Translate moq-net's model to TypeScript](/quest/m2/ts-model/README.md) - calls `Option::cloned`
