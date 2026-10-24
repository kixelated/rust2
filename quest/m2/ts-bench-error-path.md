# [S] Error-path workload in the TS benchmark

## Goal

`just ts bench` times a codec path that fails and retries: a frame split across
reads, decoded again on `Short`, so the cost of `?` reusing the incoming `Err` is
measured against native, WASM, and hand-written routes.

## Plan

From [#15](https://github.com/kixelated/rust2/pull/15): every workload today
decodes a complete buffer, so no error path is priced.

Public API: none.
