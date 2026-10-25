# [M] What macros cost moq

## Goal

A report on what macro expansion costs moq's builds and which levers cut it. It
covers each proc macro and derive moq uses (serde, `usage_derive`, tokio's
`#[tokio::test]` and `#[moq_clock::test]`, thiserror, and others the data turns
up), and for each lever reports CPU saved on moq's clean build and moq-net edit:

- `-Zmacro-stats` for expansion size per macro;
- `-Zcache-proc-macros`, which the benchmark already measures at -4% to -6% of
  the hub edit;
- `build-override` opt-level for proc-macro crates;
- declarative derives
  ([RFC 3697](https://github.com/rust-lang/rfcs/pull/3697) and
  [RFC 3698](https://github.com/rust-lang/rfcs/pull/3698));
- facet in place of serde;
- moving test attributes to a thin non-generic runner.

It answers the maintainer's question: whether macro cost is a design oversight
that a subset can avoid, or something a lever already removes.

## Plan

Decided with the maintainer on 2026-10-25: research, not a ban. Expansion was
25% of the moq-net edit's rustc time in the
[edit attribution](https://github.com/kixelated/rust2/pull/22), but derive
expansion is about 1.5-2% of moq-net's build, so separate the two costs:

- running proc macros;
- type-checking and compiling what they emit.

Public API: none.
