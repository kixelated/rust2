# [S] String's edits

## Goal

[rust2-ts](/ts/README.md) translates `String`'s edits in place, such as `push_str`,
`clear`, `insert_str`, `truncate`, and `pop`, proven by differential tests against native
Rust.

## Plan

Found by [#260](https://github.com/moq-dev/rust2/pull/260) on 2026-10-30: `push_str` and
`clear` on a local `String` are refused, while `push` translates. moq-net and kio call
neither, so this is m3 under the all-of-std decision.

- A `&mut String` is already an `rt.Place`, so each edit writes the new string through it.
- `truncate` and `insert_str` panic off a char boundary, with std's message.

Public API: the translator's output.
