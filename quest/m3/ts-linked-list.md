# [M] LinkedList

## Goal

[rust2-ts](/ts/README.md) translates `std::collections::LinkedList` and its methods,
`Debug` included, proven by differential tests against native Rust.

## Plan

Found auditing std collections' `Debug`: any use refuses with `unsupported type
std::collections::LinkedList from another crate`. No consumer calls it yet; it is planned
here as part of std.

Public API: the translator's output.
