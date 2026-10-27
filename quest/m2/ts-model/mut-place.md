# [M] Mutable references to values JS copies

## Goal

rust2-ts translates a `&mut` to a local, field, or element whose JS value is a copy (a
number, an `Option` of one, a `Box`) when it is passed on or kept, proven by differential
tests against native Rust.

## Plan

Found by the [model's](/quest/m2/ts-model/README.md) census on 2026-10-27: 74 refusals
over 36 items in the model (`&mut Option<Position>` in track.rs, `&mut u64` in origin.rs,
`&mut Box<Subscriber>` in resume.rs), 9 in moq-pattern's cursors, and 3 in kio.
`rt.Place` already does this for a map's slot.

Measure on the [TS benchmark](/ts/README.md#bench): code that needs no place gets none.

Public API: the translator's output.
