# [XS] Fail CI on an instance not inferred

## Goal

CI fails when any crate's census reports `instance not inferred`, so a gap in the
exporter's instance inference is caught when it appears instead of sitting in a census no
one reads.

## Plan

[#188](https://github.com/kixelated/rust2/pull/188) made inference one pass before typing.
Where inference misses an instance typing reaches, typing refuses the item as
`instance not inferred`. That refusal means the exporter has a bug, not that the crate uses
something unsupported. No crate reports it today, so the check passes from the start.

- Check every census the exporter runs: the fixtures' in the merge gate, and moq's in the
  job that already runs it.
- Fail with the item and its span, so the report says what to fix.

Public API: none.
