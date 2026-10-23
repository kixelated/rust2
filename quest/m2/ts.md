# [L] Translate moq-net's codec to TypeScript

## Goal

`ts/` holds an own rustc driver that translates a documented Rust subset to
readable TypeScript with source maps, without WASM. It translates moq-net's
codec (varints, zigzag, message encode and decode), and differential tests
against native Rust prove identical results, including full-width `u64` and
malformed input. An unsupported construct fails generation with a source span.

## Plan

Decided with the maintainer on 2026-10-23
([moq-dev/moq#6578](https://github.com/moq-dev/moq/pull/6578)):

- **Own rustc driver**, not Charon and not a dependency on rust-js. Charon has
  no async and loses loop structure; [rust-js](https://github.com/rust-js-lang/rust-js)
  is a rustc driver too, but runs futures eagerly. Read and borrow from it
  (MIT), especially its oxc printer for formatting and source maps.
- **Research track**: moq still ships WASM first and evaluates wasm2js. Once
  this translates reachable moq-net code, moq compares it as the
  source-translator route. moq consumes it as a pinned flake input.
- **Codec first, fail-closed**: sync code only. Model, kio, and async come
  after, as their own quests; async must keep Rust's lazy futures.
- **Generic**: map by Rust type and construct, never by moq-net names.
  Project-specific bindings (a Rust item to hand-written TS) live in a small
  config.
- The driver needs rustc-dev on a pinned nightly; pin it in the flake and
  distribute through the flake, not crates.io.

Mapping guidance carried from moq's 2026-10-11 rs2ts planning, for the agent to
re-check against the codec:

- Structs become classes, enums discriminated unions, traits interfaces.
  `Option<T>` is `T | undefined`; reject a nested `Option`, which would collapse.
  `&[u8]` is a `Uint8Array` view with no copy.
- `u64` and `i64` never map to a lossy `number`. Integers up to 32 bits map to
  `number` with checked arithmetic that throws on overflow; never wrap silently.
- `Drop` becomes an explicit `drop()` exposed as `[Symbol.dispose]`.
- Tests: snapshot the generated TS from sample crates, plus differential runs
  against native Rust. A shared npm runtime (for `U64` and checked ops) is
  published only once two generated packages need it.

Public API: a new tool. Adds the Cargo workspace if it is the first crate.
