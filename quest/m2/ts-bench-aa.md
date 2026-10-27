# [S] An A/A control that measures noise

## Goal

`just ts bench`'s `A/A` route times the same code as `translated` with nothing else
different, so its spread is the run's noise on every engine, Chrome, Firefox, and Bun.

## Plan

Found by [#87](https://github.com/kixelated/rust2/pull/87) and
[#97](https://github.com/kixelated/rust2/pull/97) on 2026-10-27:

- In JavaScriptCore, the `A/A` bundle and a route's bundle with identical text appear
  to share type profiles, so a call site polymorphic in one is polymorphic in both. A
  route that differs from `A/A` in text is then not directly comparable with it.
- In Firefox, the handoff's `A/A` runs 6-13% faster than `translated` on main, because
  `translated` shares its module with `spawned` and `A/A` does not.

Give every route the same module layout and run each in its own process or realm, so
neither shares state with another. Show the `A/A` spread before and after on each
engine.

Public API: none.
