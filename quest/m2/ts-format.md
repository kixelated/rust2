# [M] Formatting in the TypeScript translator

## Goal

[rust2-ts](/ts/README.md) translates `format_args!` and its macros (`format!`,
`write!`, `panic!`), `Display` and `Debug` impls (derived ones included), and
std's own panic messages, so every message matches native Rust byte for byte,
proven by differential tests that compare panic and error text.

## Plan

Decided with the maintainer on 2026-10-23: translate formatting rather than
emit static messages, since error paths must match native exactly and moq
surfaces thiserror `Display` text to callers.

- Cover the format spec moq uses (width, fill, alignment, `{:x}`, `{:?}` and
  `{:#?}`) and refuse the rest with a span.
- std's messages, such as `unwrap` on `None`, are part of the contract too.

Public API: the translator's output for formatting.
