# [M] A pointer a callee takes to a variable that gets another object

## Goal

[rust2-ts](/ts/README.md) refuses, with its span, a raw pointer to a variable's object,
or the `rt.Cell` `UnsafeCell::get` gives, that a callee took through a reference and
that outlives the call, where the caller then gives the variable another object. A
translated program never silently reads the old object.

## Plan

In m2 rather than m3: it is a silent miscompile today, not a refusal. Found by
[#268](https://github.com/moq-dev/rust2/pull/268) on 2026-10-30; the same-function cases
(`mem::replace`, `mem::swap`, `mem::take`, a whole write through an alias, and pointers
made through a borrow of the variable) landed with
[#277](https://github.com/moq-dev/rust2/pull/277).

```rust
fn pointer(d: &mut Duo) -> *mut Duo { &raw mut *d } // or `c.get()` of a `&UnsafeCell<T>`
let mut d = Duo(n, n);
let q = pointer(&mut d);
d = Duo(0, 0);
unsafe { (*q).0 } // `n` in JS, `0` natively
```

`out_of_step` in `ts/src/lower/mir/place.rs` checks nothing before a pointer's last
`Deref`, so a pointer taken through a reference never sees the variable it came from.

Judging it by type as first planned (where the program takes a pointer through a
reference into a type kept as an object, refuse a borrow of a variable of that type
assigned again after) refused about 100 valid borrows in the lang fixture:

- Optimized MIR in the dev profile has no storage markers, so a loop-local `let`, or a
  temporary such as `to_be_bytes()`'s, counts as assigned again on the next iteration.
- Pointers that never outlive the call, such as `ptr::eq` of two references, fill the
  type set.

Open decision: how to make it precise enough that valid pointers keep translating:

1. The type judgment with both fixed: keep storage markers in the profile
   (`-Zmir-enable-passes=-RemoveStorageMarkers`, which rebuilds the std sysroot), and
   record only pointers through a reference that may outlive the call: returned, stored
   behind a pointer, or passed to a call other than std's pointer reads and writes.
2. Refuse in the callee: a raw pointer made through a reference to an object or an
   `rt.Cell` that may outlive the call, by the same escape check, with no caller analysis.
   Simpler, but refuses accessors like `fn as_ptr(&self) -> *mut T` whatever the callers do.
