# [M] Options of values that can be undefined

## Goal

rust2-ts translates an `Option<T>` whose `T` can already be `undefined`, such as
`Option<Option<u64>>` and `Option<()>`, keeping `None` apart from `Some(None)`, proven by
differential tests against native Rust.

## Plan

Found by the [model's](/quest/m2/ts-model/README.md) census on 2026-10-27: 73 refusals
over 37 items, 33 in `resume.rs`.

Decided with the maintainer on 2026-10-27: only where `T` can be `undefined`, the outer
`None` is a runtime sentinel, as `rt.Pending` and `rt.DONE` are. Every other `Option`
stays `T | undefined`, which keeps `?.` and `??` working for TS callers. A uniform
`rt.None` for every `Option` was considered and left out.

Public API: the translator's output.
