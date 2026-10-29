# [S] Glue that builds without warnings

## Goal

The Rust glue the exporter writes builds without warnings, and `just ffi test` builds the
api fixture's glue with `-D warnings` so a new one fails.

## Plan

Found on 2026-10-28 bumping the moq pin: a callback that returns `()`, as
`moq_mux::catalog::Producer::mutate` takes, emits `let result = ...; ()`, which warns
`unused variable: result` three times in moq's glue. `lift` of `Type::Unit` in
`ffi/src/glue.rs` drops its expression; passing it through as `e` fixes the closure and the
callback vtable alike. The api fixture has no unit-returning closure yet; add one.
