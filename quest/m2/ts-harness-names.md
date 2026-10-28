# [S] Fixture cases by name

## Goal

Adding a module to a TS fixture's differential harness never renumbers another's, so two
PRs that each add one merge without editing the dispatch, and a targeted run in
`caller.ts` can't land on the wrong module.

## Plan

Found on 2026-10-27: the lang fixture dispatches by hand-numbered `match op % N` in
`ts/fixtures/lang/src/harness.rs`, which `caller.ts` targets by number. Every concurrent
ts PR that day collided on the numbers and on `N`, and one collision routed
`durations`' targeted cases to another module, which only review caught.

Decided with the maintainer on 2026-10-27: m2, after the swap fix, since it slows every
ts PR now.

- Name each case, or derive the dispatch from one list, so the count follows it.
- Keep each case's share of the random run and the harness's coverage; a dispatch change
  must not shrink what the differential run reaches.
- Apply it to every fixture with the same pattern.

Public API: none.
