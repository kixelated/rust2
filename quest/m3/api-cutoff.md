# [M] How often a semantic API hash holds

## Goal

A report of how many of moq's real edits keep a crate's semantic API hash
unchanged, replayed over moq's git history the way the early cutoff spike
replayed rust#156940's hash. The hash covers what a dependent can observe
(public signatures, and the auto traits and other facts of public opaque types
such as an `async fn`'s future), not the coroutine's layout or span positions.
If most edits keep it, the report recommends a follow-up quest: a
`RUSTC_WRAPPER` that stops dependents from rebuilding when the hash holds.

## Plan

Decided with the maintainer on 2026-10-25: measure first, and build the wrapper
only if the numbers justify it.

The [relink quest](/quest/m3/relink.md) records why upstream's hash rarely holds
on moq: rmeta encodes every `async fn`'s coroutine MIR, and span positions are
hashed. A dependent can observe an `async fn` body only through its future's
auto traits (`Send`, `Sync`, `Unpin`) and its size, so a hash of those facts is
sound where a hash of the MIR is needlessly strict. Things to settle while
measuring:

- Everything else a dependent can see: inlined and generic bodies (they are
  codegenned in the dependent, so they belong in the hash), `const` values,
  trait impls, and macros.
- Whether size belongs in the hash: rustc lays out a dependent's types from the
  future's size, so a body edit that changes it must still rebuild dependents.
- Compute the hash with [rust2-driver](/driver/src/lib.rs), so no rustc patch is needed.

Public API: none.

## Related

- [Relink without rebuilding](/quest/m3/relink.md) - upstream's version of this lever
