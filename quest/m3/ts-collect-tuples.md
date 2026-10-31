# [S] collect into tuples of collections

## Goal

[rust2-ts](/ts/README.md) translates `collect` into a tuple of collections, such as
`(Vec<A>, Vec<B>, Vec<C>)`, through std's `FromIterator` and `Extend` of tuples,
proven by differential tests against native Rust.

## Plan

Found writing a fixture: collecting an iterator of triples into three `Vec`s refuses as
an unsupported call. Check which tuple arities and collections `unzip` and `collect`
already take, and match std's order of extending each.
