# Migrating

## Unreleased

- In `exclude` and `rename`, a path that a field shares with a method or constant of its
  type, such as `Backoff::initial`, names only the method, as in rustdoc. Name the field
  `field@Backoff::initial`: an `exclude` that left out both now needs both entries.
- Every export defines `<prefix>_contract` and `<PREFIX>_CONTRACT`, so an exported item
  that the naming rule names either, such as a crate root's `pub fn contract` or
  `pub const CONTRACT`, clashes with them: give it another name with a `rename`.
- A field, parameter, or variant named a C++ keyword, such as moq's `namespace`, takes
  a leading `_` in C as a C keyword does (`_namespace`), so the header compiles as C++.
- The C++ header's runtime names (`view`, `result`, `task`, `future`, `poll_t`, and
  `detail`) and every variant's payload name (`<enum>_<variant>`, even without fields)
  join the names the naming rule claims, so an exported item named one clashes: give it
  another name with a `rename`.
- Every export writes `kotlin/<prefix>.kt`, whose runtime object `Native` makes the C
  name `native` one the naming rule claims. An item Kotlin names as a type its runtime
  uses unqualified, such as `String` or `Unit`, or as another item, such as a type
  `FooException` beside an error type `Foo`, is an error: give it another name with a
  `rename`.
