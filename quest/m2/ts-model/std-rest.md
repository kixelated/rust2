# [M] The rest of std the model reaches

## Goal

rust2-ts translates the std calls the model's census still refuses after
[std gaps](https://github.com/kixelated/rust2/pull/124) and [core's MIR](https://github.com/kixelated/rust2/pull/129),
outside [Places](https://github.com/kixelated/rust2/pull/149), proven by differential tests against
native Rust.

## Plan

Found by [#124](https://github.com/kixelated/rust2/pull/124) and #129 on 2026-10-27:
iterator `min_by_key` and `max_by_key`, `slice::split_last`, integer `to_le_bytes`,
`Box<T>::clone`, `retain` and `sort` of byte vectors, `Debug` of `Reverse` and `Bound`,
slicing by `..=`, and `Option`'s and `Result`'s `PartialEq` and `From` impls and the std
calls core's bodies reach (`Default::default`, a payload's `Clone`).

- Rerun the model's census first, since #115, #128, and #129 moved it, and take what it
  refuses in std rather than only this list.

Public API: the translator's output.
