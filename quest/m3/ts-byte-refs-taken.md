# [M] References to byte arrays std takes

## Goal

The calls still refused as "of a reference to an array of bytes it is generic over"
translate, proven by differential tests against native Rust: a collection of `[u8; N]`
extended by references to arrays, std's adaptors handing their closure a reference to a
`[u8; N]` item they hold (`filter` over owned arrays, `map` then `collect` over a map's
`iter`), and a collection holding a `&[u8; N]` beside a `[u8; N]`.

## Plan

The reject fixture's `byte_extended`, `filter_owned_bytes`, and `byte_key` show each.
`extend` would store the `rt.ByteSlice` it is given where the collection holds a
`Uint8Array`, so it copies each. An adaptor's closure takes a slice of the item, as
`collections::lend_closure` wraps a collection's. A collection holding both forms can't
tell a held `&[u8; N]` from a lent one by the instantiated type alone; the method's generic
signature, `&K` against `K`, can.

Public API: the translator's output.
