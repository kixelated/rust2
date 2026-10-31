# [M] Cow of a slice, and Wrapping

## Goal

[rust2-ts](/ts/README.md) translates `Cow<[T]>` and `Wrapping<T>` and their std impls,
`PartialEq` included, proven by differential tests against native Rust.

## Plan

Found auditing std's `PartialEq` impls that forward `ne`: constructing a `Cow<[T]>`
refuses, as `Cow<str>` does not, and any `Wrapping` refuses as an unsupported type from
another crate. No consumer uses either yet; they are planned here as part of std. A
`Cow`'s and a `Wrapping`'s `ne` are `!eq`, though a `Cow<[T]>`'s `eq` still calls each
item's `ne`.
