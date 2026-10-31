# [XS] A tuple's cmp typed as total

## Goal

`rt.cmpTuple` of fields that each order totally types its result as `Ordering`, so
[rust2-ts](/ts/README.md)'s output of `let o: Ordering = (x,).cmp(&(y,))` type-checks
for a crate's `x`.

## Plan

Found ordering composites of references: `rt.cmpTuple([Local.cmp$Ord])` infers its
result as `Ordering | undefined`, since its first overload infers only the tuple type
from the fields, and `tsc` rejects assigning it to an `Ordering`. Fixtures dodge it by
wrapping each in `Some`. An overload ahead of it for total fields, or one inferring the
fields' return type, broke inference where a generic `rt.cmp` is a field.
