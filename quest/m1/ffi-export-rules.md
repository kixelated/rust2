# [M] Exporter rules the census asks for

## Goal

[rust2-ffi](/ffi/README.md) exports the moq-net constructs its census refuses most, each by a
fixed rule proven by a fixture that round-trips through C against native Rust:

- a tuple return, as one out parameter per element (`origin::Producer::new`, so an origin
  can be made);
- a type whose only generic parameters are lifetimes (`Path<'a>`), as an object holding its
  `'static` form;
- `Range<u64>`, and a `RangeBounds<u64>` parameter;
- a borrowed return of an object that is not `Copy`, as a new handle to its clone (the
  `info()` getters).

Every construct the rules still refuse stays a compile error at its span.

## Plan

Split from the [exporter](/quest/m1/ffi-export.md) as its first slice.
