# [M] Instance names that adding code elsewhere keeps

## Goal

A generic instance's translated name depends only on that instance, never on which other
instances the translation reaches. Adding code anywhere renames nothing it doesn't reach, so
snapshots and a consumer's generated diffs change only where the source did. The
`stable_names` fixture test proves it for every way names are chosen today.

## Plan

Found by [moq-dev/rust2#259](https://github.com/moq-dev/rust2/pull/259) on 2026-10-30.

The merge path is done in [moq-dev/rust2#265](https://github.com/moq-dev/rust2/pull/265): an
instance of a generic std function is named `base$hash` of its printed text, with what it
calls by name, and `stable_names`' `fmt::added` reaches one that merges and sorts first.
A signature in the suffix was measured and rejected: one signature gives several bodies
(a drop flag, `rt.callOnce` for an `FnOnce`, a fn item called by name), so it collided in
7 places across the fixtures.

What remains is the collision path in `Program::name`. Instances whose suffixes still
match are numbered `$2` on in key order, so an added instance that sorts earlier renumbers
the others. In the fixtures the suffix loses: `&T` and `&mut T` against `T` (most cases),
array lengths, two types or closures of one name in different modules (`copy$Pair` and
`pointers::Pair`), fn-local items of one name (`captures`' `through$2`..`$12`), and the
coroutine classes of one function (`blocksFuture$2`..). moq-net has none today.

Decided by the maintainer on 2026-10-30: qualify every type, trait, and closure in a
suffix by its path: the crate's own by module path, a dependency's under its crate, std's
by its public path with a repeated module dropped (`Option`, `fmt_Display`, `io_Error`).
It is the only rule that keeps a name a pure function of the instance, since callers call
these names; every name over a type outside the crate root renames once, and closures in
impls need the impl's type and trait. Qualifying only names another definition shares was
rejected here: defining a type of a taken name would rename the other's instances.
Import bindings take the other rule ([ts-stable-imports](/quest/m2/ts-stable-imports.md)).

Either way, spell out `&`, `&mut`, and array lengths, and name fn-local items and
coroutine classes by their enclosing items rather than a counter.

Proof: extend `stable_names` so an added root reaches an instance whose suffix collides
with an existing one and sorts first, such as an impl over `&mut &mut u32` beside one over
`&mut u32`. Update the naming paragraph in `ts/README.md`.

Public API: the translator's output names. Collided instances rename once.
