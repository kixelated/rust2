# [S] Land main through quest land

## Goal

PRs to `main` land only through `quest land` on a hosted runner: every queued
PR goes into one chain of squash commits, checked once with the repository's
Check steps, and `main` fast-forwards to the longest passing prefix. GitHub's
merge button cannot land around it.

## Plan

Decided 2026-10-26 (maintainer), planning moq-dev/moq's merge throughput
([moq-dev/moq#6854](https://github.com/moq-dev/moq/pull/6854)): the batched
gate is shared through kixelated/quest, and this repository adopts it.

- The push credential is GitHub Actions in a new `main` ruleset's bypass
  list, guarded by a test that refuses `contents: write` anywhere but the
  land job, as moq.pro's `queue.test.ts` does. The admin role keeps its
  emergency bypass. Ruleset writes are the maintainer's.
- The ruleset requires a check that PR heads never carry, so auto-merge only
  waits for the gate. `check.yml` drops its `merge_group` trigger.
- Update the contributor docs that describe landing.

## Required

- [Bump quest to quest land](/quest/m1/land-bump.md) - condition: kixelated/quest ships `quest land`; then bump the flake input
