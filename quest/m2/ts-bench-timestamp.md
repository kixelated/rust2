# [XS] Time Timestamp's checked_add and Ord in the TS benchmark

## Goal

`just ts bench` times moq-net's `Timestamp::checked_add` and `Ord`, whose `u128`
cross products the wide runtime now computes in limbs, in Chrome, Firefox and Bun
with the A/A control.

## Plan

Decided with the maintainer on 2026-10-26, from
[#89](https://github.com/kixelated/rust2/pull/89): its Node/Bun microbench put
`checked_add` and `saturating_sub` 20-100x faster, but no bench workload reaches them,
so Chrome never timed them.

- Add the workload beside `time/u128`, drawing its inputs from the wasm32 build as
  the other workloads do.
- The remaining 128-bit `bigint` paths (bitwise ops, shifts, `leading_*`, `try_from`)
  stay until a consumer reaches them.

Public API: none.
