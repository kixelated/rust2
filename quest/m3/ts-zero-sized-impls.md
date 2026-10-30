# [S] std's impls of its zero-sized types

## Goal

`PhantomData`'s `Debug`, `Default`, `PartialEq`, and the other impls `#[derive]` calls on a
field of it translate, and `()` is a hash or B-tree key and the item of a
`collect::<Option<_>>`, proven by differential tests against native Rust.

## Plan

Found by [#304](https://github.com/moq-dev/rust2/pull/304) on 2026-10-30: a struct deriving
`Debug`, `Default`, or `PartialEq` with a `PhantomData` field is refused as an unsupported
call to its impl, and `HashSet<()>`, `BTreeSet<()>`, `HashMap<(), _>`, and collecting
`Option<()>` items refuse `()` as a key, as an ordered value, and as an `Option` whose
`None` is `rt.None`. The crate's own unit structs already translate in each. Each impl
of `PhantomData` reads nothing; its `Debug` writes its type's name, `PhantomData<u8>`.

Public API: the translator's output.
