# [M] MIR's codec as fast as THIR's

## Goal

Lowering moq-net's codec from MIR runs it no slower than THIR's lowering on
`just ts bench`, in Node, Bun, and Chromium, before MIR becomes the only lowering.

## Plan

Decided with the maintainer on 2026-10-24: this blocks making MIR the default. On
[#36](https://github.com/kixelated/rust2/pull/36)'s head, after its review fixes, the
codec's geometric mean from MIR was 1.10x THIR's in Node, 1.02x in Bun, and 1.12x in
Chromium, against A/A controls of 1.00. Group encode was 1.9-2.6x slower in every
engine, frames encode 1.3-1.4x in Chromium, and small varint encodes up to 1.6x in
Node (but 0.7x in Bun). An earlier run, before the review fixes and the bench's
random order, put the codec within 8% apart from group encode, so start by
separating what the review fixes' copies cost from what was always there.
