# [S] Port the workflow guard to kixelated/quest

## Goal

kixelated/quest's `test/workflows.test.ts` parses workflows as YAML, as rust2's
[former `land.test.ts`](https://github.com/moq-dev/rust2/blob/f78c588fe61bd9ea0db8f98f1b774cf42280a046/.github/land.test.ts)
did, so an environment named through an expression, a flow mapping, or another
repository's reusable workflow can't slip past its guard on the `land` environment.

## Plan

Found by [#137](https://github.com/moq-dev/rust2/pull/137) on 2026-10-26: quest's guard
has the gaps this repository's closed. The work lands in kixelated/quest, through its own
review; this quest tracks it here and is deleted once it merges there.

Public API: none.
