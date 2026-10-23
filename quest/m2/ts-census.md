# [S] Coverage census for the TypeScript translator

## Goal

[rust2-ts](/ts/README.md) can report every construct it refuses in a crate, with
counts and spans, instead of stopping at the first. A nightly job runs it on
moq-net and kio at the pinned moq revision and publishes the ranked table, so
the next feature to build, or to drop from the subset, is picked from numbers.

## Plan

Decided with the maintainer on 2026-10-23. Researched counts (2026-10-23, rough
regex over non-test code) to check the census against: Poll/Waker about 2,400,
`async`/`.await` 1,200, iterator adaptors 1,100, `Arc` 500, `Duration`/`Instant`
460, `Bytes` 270, collections about 300, `impl Drop` 78, `u128` 44, `dyn` 24,
`unsafe` 19.

- The census is a mode of the driver, not a separate tool, so its notion of
  "refused" never drifts from translation's. Translation itself still fails
  closed.
- Group refusals by construct (and by std or crate API for unmapped paths), so
  the table names the mapping to add.

Public API: a new driver flag.
