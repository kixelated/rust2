# [S] A pointer to a variable that std's mapping rebinds

## Goal

[rust2-ts](/ts/README.md) refuses, with its span, a raw pointer to a variable's object,
or the `rt.Cell` `UnsafeCell::get` gives, where `mem::replace`, `mem::swap`, or `mem::take`
of a `&mut` to that variable gives it another object, as it refuses one where the
variable is assigned again; a translated program never silently reads the old object.

## Plan

In m2 rather than m3: it is a silent miscompile today, not a refusal, and the subset's
promise is that translation fails loud wherever it can't match native Rust. Found by
[#268](https://github.com/moq-dev/rust2/pull/268) on 2026-10-30:

```rust
let mut pair = (n, n);
let p = &raw mut pair;
let old = std::mem::replace(&mut pair, (0, 0));
unsafe { (*p).0 + old.1 } // `n` in JS, `0` natively
```

`mem::replace` of a local lowers as `old = pair; pair = [0, 0]`, but `assigned_after` in
`ts/src/lower/mir/place.rs` counts only MIR's assignments and call destinations. Count
the calls whose mapping rebinds the variable a `&mut` argument borrows, and add the case
to the reject fixture beside `variable_reassigned` and `cell_reassigned`.

Public API: the translator's refusals.
