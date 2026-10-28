# Migrating

## Unreleased

- A type parameter whose bounds `Bytes` or `Vec<u8>` meet as well as `String`, such as
  moq's `IntoBytes` frame payloads, exports as bytes rather than text. Kotlin takes a
  `ByteArray` there: pass text as `"text".encodeToByteArray()`. C and C++ keep their
  types, and accept a payload that is not UTF-8 rather than aborting.
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
- The `instances` key is gone, top-level and under `[crates.*]`, and a config that has
  it fails to parse: delete it. A generic item exports at the instances it infers from
  its defaults, its inherent impls, its exports' implementations of the traits bounding
  it, and the signatures that name it. Exclude an instance by its path, such as
  `"Pair<u8, dep_fixture::Size>"`, to leave it out.
- An instance's C name is no longer the config's. It is the generic item's name, then
  the last segment of each argument's, with more segments only where two instances would
  share a name: `catalog::Producer<catalog::hang::Extra>` is `moq_catalog_producer_extra`,
  and `Counter::seeded` at `polar::Point` beside `Point` is
  `api_counter_seeded_polar_point`. An instance an upstream change adds can lengthen
  another's name.
- Every export writes `kotlin/<prefix>.kt`, whose runtime object `Native` makes the C
  name `native` one the naming rule claims. An item Kotlin names as a type its runtime
  uses unqualified, such as `String` or `Unit`, or as another item, such as a type
  `FooException` beside an error type `Foo`, is an error: give it another name with a
  `rename`.
