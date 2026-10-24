# [S] Formatting impls of root types

## Goal

A `ts.toml` option makes a root type bring its `Display` and `Debug` impls into
[rust2-ts](/ts/README.md)'s output, so moq's TypeScript callers can print a
moq-net error such as `DecodeError` exactly as native Rust does.

## Plan

Decided with the maintainer on 2026-10-23, from the formatting PR (#8): roots
bring only the crate's own traits' impls along, so std formatting impls on a
root type are dropped unless something translated calls them.

Public API: a new `ts.toml` option.
