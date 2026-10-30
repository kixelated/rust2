# [M] Instance names that adding code elsewhere keeps

## Goal

A generic instance's translated name depends only on that instance, never on which other
instances the translation reaches. Adding code anywhere renames nothing it doesn't reach, so
snapshots and a consumer's generated diffs change only where the source did. The
`stable_names` fixture test proves it for every way names are chosen today.

## Plan

Found by [moq-dev/rust2#259](https://github.com/moq-dev/rust2/pull/259) on 2026-10-30. A
fixture closure renamed `floats.ts`'s import `mapOr$Option_Ordering_u8_floats_closure0`, so
the fixture was changed to avoid it.

Reproduced on main at a0f08f40: adding
`out.push(1u8.partial_cmp(&2).map_or(7, |o| o as i8 as u8));` to `fmt::added` in the lang
fixture fails `stable_names`. The new instance, `..._added_closure0`, prints the same as
`floats`' and sorts first.

Root cause: `Program::merge` in `ts/src/program.rs` merges std instances that print the same
and keeps the one whose `Symbol::key` sorts first. The kept name carries the closure's
identity (`floats_closure0`), so it depends on which other members the group has. The same
flaw is in `Program::name`. Instances whose suffixes still match, such as over two types of
one name, are numbered `$2` on in key order, so an added instance that sorts earlier
renumbers the others.

`stable_names` missed both because it checks one added root, `fmt::added`. That root reaches
only a local generic (`Wrap::map` over its own closure), which never merges, and no suffix
collision. The test's doc claims the general property, but its one root exercises neither
path.

Recommended direction: name a merged function from what it is merged on, not from a member.
That is the merge key itself (module, base, params, return, and body, with callees by their
own stable names). Any std instance that could merge must carry that name even when it has
no twin, since whether a twin exists is what changes. Options to weigh:

- Replace a closure's or fn item's identity in the suffix with what the output depends on:
  its signature, plus whatever else changes the body, such as a fn item called by name or a
  closure the instance drops. This stays readable, but it must be injective over bodies.
- Append a short hash of the printed output. It is always injective, but it churns whenever
  the translation of std's body changes, and a recursive instance must hash its
  self-reference as a placeholder.

Name collisions get the same treatment: a disambiguator taken from the instance's own type
arguments, such as a module path segment, rather than a counter in key order.

Readability is not a goal ([m2](/quest/m2/README.md#plan)), but measure the gzipped bundle
size against main, as the merge in
[moq-dev/rust2#129](https://github.com/moq-dev/rust2/pull/129) did.

Proof: extend `stable_names`, or add sibling cases, so that an added root reaches (a) a
std instance that merges with an existing one and sorts first, and (b) an instance whose
suffix collides with an existing one and sorts first. Keep the cases fast enough for the
merge gate. Update the naming paragraph in `ts/README.md`.

Public API: the translator's output names. Merged std instances, and collided ones, rename
once.
