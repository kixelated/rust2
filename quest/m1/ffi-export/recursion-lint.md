# [S] The recursion depth lint on moq's export

## Goal

`just ffi moq` builds with no `recursion_depth_exceeding_limit` warning, before the pinned
nightly turns that future-compat lint into a hard error and the export stops building.

## Plan

Found by [#144](https://github.com/kixelated/rust2/pull/144) on 2026-10-27: the lint fires on
`Send` checks in moq-net, moq-tokio, and the generated `moq_ffi`.

- Find which types' `Send` checks recurse that deep, and whether the generated glue makes
  them deeper than moq's own code does.
- Fix the generated part here. A part in moq's own crates is moq's change, recorded in
  moq's tree; a `recursion_limit` raise is a stopgap only if neither can avoid the depth.
- Fail the build on the lint once it is gone, so it does not come back.

Public API: none.
