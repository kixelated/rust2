# [S] Fold a match a bool decided

## Goal

rust2-ts drops the dead arms of a match that a `bool` already decided, as
[#175](https://github.com/moq-dev/rust2/pull/175) does for a match an `if` or guard sits
in front of: `if matches!(s, Shape::Dot) { match s { .. } }`, and `==` of a crate enum's
derived `PartialEq`. Output only shrinks; the differential tests stay equal.

## Plan

Suggested by #175 on 2026-10-27: the `bool` joins two edges before the `if`, so folding it
means recording, in `decided.rs`'s dataflow, which variants a `bool` local holds `true`
for. Measure the size change on moq-net and the TS benchmark's bundles.

Public API: the translator's output.
