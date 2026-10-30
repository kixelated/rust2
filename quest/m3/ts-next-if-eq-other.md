# [XS] Peekable::next_if_eq against another type

## Goal

`Peekable::next_if_eq` translates where the expected value is of another type than the
item, as `next_if_eq("a")` of `String` items is, comparing by the item's `PartialEq<T>`,
proven by a differential test against native Rust.

## Plan

Refused at its span today, as the reject fixture's `peeked_str` is: the mapping compares
by `eq_fn` of the item's own type. std's string types compare by content across types,
and a crate's own `PartialEq<T>` resolves as any trait method does.

Public API: the translator's output.
