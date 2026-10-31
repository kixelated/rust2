# [XS] ptr::replace whose result nothing reads

## Goal

[rust2-ts](/ts/README.md) translates `ptr::replace` whose result nothing reads, such as
`let _ = p.replace(1)` through a `*mut u8`, to output `tsc` accepts, proven by a
differential test against native Rust for a pointer to a value JS copies and to an object.

## Plan

Found while making writes of no bytes write nothing: the mapping in
`ts/src/lower/pointer.rs` declares the old value as `old` before writing, and a library
mapping doesn't see that its result goes unread, so `tsc` refuses `old` as unread
(TS6133). `mem::replace` and `Option::take` already skip the declaration when their
destination is unread. A runtime function that returns what it replaced, as
`rt.cellReplace` does for an `rt.Place`, would let the result go unread, with its own
copy for a pointer to an object.
