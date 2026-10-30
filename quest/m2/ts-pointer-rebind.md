# [M] A pointer to a variable that gets another object

## Goal

[rust2-ts](/ts/README.md) refuses, with its span, a raw pointer to a variable's object,
or the `rt.Cell` `UnsafeCell::get` gives, where the variable gets another object while
the pointer may point there and the pointer's check can't see it: `mem::replace`,
`mem::swap`, or `mem::take` of a `&mut` to the variable, or a caller's assignment after a
callee took the pointer through a reference. A translated program never silently reads
the old object.

## Plan

In m2 rather than m3: it is a silent miscompile today, not a refusal, and the subset's
promise is that translation fails loud wherever it can't match native Rust. Found by
[#268](https://github.com/moq-dev/rust2/pull/268) on 2026-10-30:

```rust
let mut pair = (n, n);
let p = &raw mut pair;
let old = std::mem::replace(&mut pair, (0, 0));
unsafe { (*p).0 + old.1 } // `n` in JS, `0` natively

fn pointer(d: &mut Duo) -> *mut Duo { &raw mut *d } // or `c.get()` of a `&UnsafeCell<T>`
let mut d = Duo(n, n);
let q = pointer(&mut d);
d = Duo(0, 0);
unsafe { (*q).0 } // `n` in JS, `0` natively
```

- `mem::replace` of a local lowers as `old = pair; pair = [0, 0]`, but `assigned_after`
  in `ts/src/lower/mir/place.rs` counts only MIR's assignments and call destinations.
  Count the calls whose mapping rebinds the variable a `&mut` argument borrows.
- `out_of_step` checks nothing before a pointer's last `Deref`, so a pointer taken
  through a reference never sees the variable it came from. Judge that by type, as
  `replaced` judges parts: where the program takes a pointer through a reference to a
  type kept as an object (or an `rt.Cell`), refuse assigning a variable of that type
  again once a borrow of it reached a call.
- Add each case to the reject fixture beside `variable_reassigned` and `cell_reassigned`.

Public API: the translator's refusals.
