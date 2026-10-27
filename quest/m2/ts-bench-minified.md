# [XS] Time the minified bundles

## Goal

`just ts bench` times the same minified bundles it measures for size, for every route
and engine, so speed calls judge what consumers ship.

## Plan

Decided with the maintainer on 2026-10-26, from
[#85](https://github.com/kixelated/rust2/pull/85): `ts/bench/main.ts` reports sizes
from `bundle(..., { minify: true })` but serves the pages unminified bundles, so a
minifier's renaming and restructuring never reaches the timings.

- Serve each route's minified bundle, the hand-written and wasm routes included, so
  every route is timed the same way.
- Record in the PR the A/A spread and each route's change against the unminified
  timings, in Chrome first, then Firefox and Bun.

Public API: none.
