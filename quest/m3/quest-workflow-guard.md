# [S] Port the workflow guard to kixelated/quest

## Goal

kixelated/quest's `test/workflows.test.ts` parses workflows as YAML, as
[`.github/land.test.ts`](/.github/land.test.ts) does here, so an environment named through
an expression, a flow mapping, or another repository's reusable workflow can't slip past
its guard on the `land` environment.

## Plan

Found by [#137](https://github.com/kixelated/rust2/pull/137) on 2026-10-26: quest's guard
has the gaps this repository's closed. The work lands in kixelated/quest, through its own
review; this quest tracks it here and is deleted once it merges there.

Public API: none.
