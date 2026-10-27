# [S] A tuple read only by its fields is never made

## Goal

[rust2-ts](/ts/README.md) no longer makes a tuple or struct that its function reads only
by its fields, at any arity: a one-argument `format!` reads its argument, not
`([x] as [T])[0]`, as in moq-net's `SessionError` and `DecodeError`. Each operand is
still evaluated once, where Rust evaluates it.

## Plan

Unplanned work, stacked on [#114](https://github.com/kixelated/rust2/pull/114), which
found it.
