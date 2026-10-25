# [M] How often a semantic API hash holds

## Goal

A report of how many of moq's real edits keep a crate's semantic API hash
unchanged, replayed over moq's git history the way the early cutoff spike
replayed rust#156940's hash. The hash covers what a dependent can observe or
codegen (public signatures, the auto traits and size of public opaque types such
as an `async fn`'s future, and every body a dependent may codegen or inline),
not span positions or MIR that only the defining crate codegens.
If most edits keep it, the report recommends a follow-up quest: a
`RUSTC_WRAPPER` that stops dependents from rebuilding when the hash holds.

## Plan

Decided with the maintainer on 2026-10-25: measure first, and build the wrapper
only if the numbers justify it.

The [relink quest](/quest/m3/relink.md) records why upstream's hash rarely holds
on moq: rmeta encodes every `async fn`'s coroutine MIR, and span positions are
hashed. A dependent observes an `async fn` body through its future's auto
traits (`Send`, `Sync`, `Unpin`) and size, and through any of the coroutine's
MIR it codegens: a generic `async fn` is monomorphized downstream, and a small
coroutine may be inlined cross-crate. Hashing only the MIR a dependent may
codegen keeps the cutoff sound where hashing all of it is needlessly strict.
Things to settle while measuring:

- Which bodies a dependent may codegen or inline under each profile moq builds,
  since cross-crate inlining depends on the opt-level; those, `const` values,
  trait impls, and macros belong in the hash.
- A two-crate differential test before the report recommends a wrapper: a body
  edit the hash ignores, such as `pub async fn value() -> u32` returning 2
  instead of 1, must change what the dependent observes once relinked.
- Count the moq-net edits that change the hash only through coroutine MIR: they
  would cut off once moq's poll-first plan ([moq-dev/moq#6814](https://github.com/moq-dev/moq/pull/6814),
  `quest/m1/poll-first`) gates its `async fn` wrappers behind an `async` feature.
- Compute the hash with [rust2-driver](/driver/src/lib.rs), so no rustc patch is needed.

Public API: none.

## Related

- [Relink without rebuilding](/quest/m3/relink.md) - upstream's version of this lever
