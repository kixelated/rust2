# [XS] Waiting: moq's poll-native async

## Goal

moq has landed its poll-native async migration
([moq-dev/moq `quest/m2/ffi-poll-migrate.md`](https://github.com/moq-dev/moq/blob/main/quest/m2/ffi-poll-migrate.md)):
moq-ffi's kio-backed async exports are a `kio::wait` over their `poll_*`, with
`ffi::Task` gone from those paths. Check: that quest file is gone from moq's
`main`. Delete this quest once it holds.

## Plan

Decided with the maintainer on 2026-10-23: [foreign poll](/quest/m1/ffi-poll-protocol.md)
moved here with the rest of the fork's perf work, but it wraps the polls this
moq migration exposes. This quest stands in for that cross-repository blocker
so foreign poll resurfaces when it clears.
