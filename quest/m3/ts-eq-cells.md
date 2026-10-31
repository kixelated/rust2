# [S] Equality of Cell and RefCell

## Goal

[rust2-ts](/ts/README.md) translates `==` and `!=` of `Cell`s and `RefCell`s, proven by
differential tests against native Rust, a `RefCell` mutably borrowed while compared
included.

## Plan

Found auditing std's `PartialEq` impls that forward `ne`: both types translate, but
`<RefCell<u8> as PartialEq>::eq` and `<Cell<u8> as PartialEq>::eq` refuse as an
unsupported call. std's compare what they hold by `eq`, and their `ne` is `!eq`, even
where the held type overrides `ne`; a `RefCell`'s borrows each side and panics if either
is mutably borrowed.
