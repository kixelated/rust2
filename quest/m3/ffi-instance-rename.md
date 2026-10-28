# [XS] Rename one instance

## Goal

A `rename` keyed by an instance's path, such as
`"catalog::Producer<moq_mux::catalog::hang::Extra>"`, renames that instance alone, as
`exclude` already takes an instance's path. Today `rename` takes only the generic item's
path, which renames every instance.

## Plan

Suggested by [#152](https://github.com/kixelated/rust2/pull/152) on 2026-10-27. No consumer
needs it yet: a clash between an instance and an unrelated item is fixed today by renaming
the unrelated item or the generic one. Start once a consumer's clash can't be fixed that
way.

- Prefer a naming rule that avoids the clash over a new config key, since every config key
  a consumer needs is a gap; take this only if no rule does.

Public API: `ffi.toml`'s `rename` keys.
