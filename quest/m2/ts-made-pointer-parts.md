# [S] A pointer std makes to a part that gets another object

## Goal

[rust2-ts](/ts/README.md) refuses, with its span, the pointer `NonNull::from` of a
reference or `MaybeUninit::as_ptr` and `as_mut_ptr` make to a part kept as an object,
where the crate may give that part another object while the pointer may point there. A
translated program never silently reads the old object, and hashlink's
`node.as_mut().entry.as_mut_ptr().write(..)` keeps translating.

## Plan

In m2: like [the callee's pointer](/quest/m2/ts-pointer-rebind.md), it may be a silent
miscompile, not a refusal. [#277](https://github.com/moq-dev/rust2/pull/277) judges these
pointers by the variable they are in, but not by the parts on their path, as
`out_of_step` in `ts/src/lower/mir/place.rs` judges `&raw`: its `replaced` judgment is
per containing type, and `Node<V>`'s `link` written whole marks every part of a `Node`,
so the lang fixture's `pointers::lists` (hashlink's shape) would be refused at
`entry.as_mut_ptr()`. Make the part judgment precise enough, for instance per field
rather than per type, then apply it to these pointers too.
