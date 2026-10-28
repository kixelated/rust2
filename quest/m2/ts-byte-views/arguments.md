# [M] Byte slices passed as parts

## Goal

A translated function that takes a byte slice outside the API boundary takes its parts
(`data`, `start`, `length`) as arguments instead of an `rt.ByteSlice`, and Chrome's varint
and frame decode on `just ts bench` are faster than `main`, with every differential test
passing.

## Plan

Decided with the maintainer on 2026-10-27, from
[#138](https://github.com/kixelated/rust2/pull/138): its per-function scratch return made
Chrome slower than `main` (varint quic/1B 12.0 to 15.2 ns, frames 3B-reads 152 to 208 ns),
since V8 already removes most of `main`'s temporaries once it inlines. A hand edit that
also passed slices as parts looked 20-40% faster on varint and 30% on frames lite-06, but
18% slower on group lite-06, on an overloaded host, so measure on a quiet one first.

- Only functions outside the API boundary that are never used as a value, as for returns.
- Report each workload in Chrome, Firefox, Node, and Bun; Chrome decides.

Public API: the translator's output for byte slices.
