# [S] Whole writes std makes through a shared reference

## Goal

[rust2-ts](/ts/README.md) refuses, with its span, a raw pointer into a part of a `T`
that std writes whole through a `&` it is given, as `RefCell::replace` does, while the
pointer may still be read. A translated program never silently reads the old object.

## Plan

Found on 2026-10-31 while proving that a pointer kept in memory writes nothing whole.
`Writes` in `ts/src/lower/copy.rs` counts a std call's whole writes only through a
`&mut` or `*mut` its signature names. `RefCell::replace` takes `&self`, and the runtime's
`refCellReplace` puts the new value in place of the old object, so this translates:

```rust
struct Held(UnsafeCell<u16>);
fn part(held: &Held) -> *mut u16 { held.0.get() }
let whole = RefCell::new((n, Held(UnsafeCell::new(n))));
let p = part(&whole.borrow().1);
drop(whole.replace((0, Held(UnsafeCell::new(0)))));
unsafe { *p } // `n` in JS; `0` natively, if the read is defined
```

Reproduce first with a differential test against native wasm32, and check the native
read under Miri's Tree Borrows: under Stacked Borrows, `replace`'s `&mut` invalidates
`p`. If the read is undefined under both, delete this quest.

The rest of std that rust2-ts maps as writing a whole value through `&self` has the same
shape: `RefCell::take`, an `RwLock`'s, and a thread-local `RefCell`'s `set`, `replace`, and
`take`. `Cell`'s are refused today only because `Cell::as_ptr` is unmapped and
`Cell::get_mut` counts its `&mut`.
