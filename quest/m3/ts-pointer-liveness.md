# [M] A raw pointer's staleness bound by its liveness

## Goal

[rust2-ts](/ts/README.md) refuses a raw pointer to a variable's object only where the
variable gets another object while the pointer, or a copy of it, may still be read, not
everywhere after the pointer is made. A pointer that is dead before its variable is
assigned again translates, as one in an earlier loop iteration does.

## Plan

`assigned_after` in `ts/src/lower/mir/place.rs` searches forward from the pointer to the
end of the variable's storage. Optimized MIR in the dev profile has no storage markers,
so in a loop that reaches the next iteration's `let` and every later assignment. The
lang fixture's `pointers::cells` replaces its cells through a helper for this reason
since [#277](https://github.com/moq-dev/rust2/pull/277): `mem::replace` of the variable in
one arm is refused by a `get` pointer in another arm, dead by then.

Bound the search by where the pointer's holders are live (rustc's `MaybeLiveLocals`),
giving up where a holder escapes into memory or a call that may keep it. Measure the
census before and after; no moq consumer hits this today, so it waits in m3.
