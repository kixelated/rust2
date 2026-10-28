# [M] Well-known crates without config

## Goal

[rust2-ts](/ts/README.md) translates calls into `bytes`, `rand`, and `tracing` with no
binding or `ignore-macros` from the consumer, as it does std's, proven by differential
tests against native Rust. `ts/moq/model.toml` drops its entries for them.

## Plan

Decided with the maintainer on 2026-10-27: a stock crate translates with no annotations or
config, so a consumer's config holds only its own platform boundaries. moq's model binds
`bytes::Bytes`, `bytes::BytesMut`, and `rand::rng`, and ignores `tracing`'s macros.

- rust2-ts ships these mappings itself, as it ships std's. Each matches the crate's
  semantics exactly; where the crate depends on the platform, such as `rand`'s source of
  randomness, say what the mapping uses.
- A mapping stands in only where the crate cannot translate from source: `bytes` has
  `unsafe` inside. Prefer translating from source once
  [unsafe code the translator can model](/quest/m3/ts-unsafe.md) covers it.
- `tracing`'s macros need a meaning without config: dropping them changes nothing
  observable unless a subscriber is installed. Decide what an installed subscriber sees.
- Report the model's census before and after.

Public API: the translator's output; `ignore-macros` and `[bind]` entries for these crates
become unnecessary.
