# Migrating

## Unreleased

- A field, parameter, or variant named a C++ keyword, such as moq's `namespace`, takes
  a leading `_` in C as a C keyword does (`_namespace`), so the header compiles as C++.
- The C++ header's runtime names (`view`, `result`, `task`, `future`, `poll_t`, and
  `detail`) and every variant's payload name (`<enum>_<variant>`, even without fields)
  join the names the naming rule claims, so an exported item named one clashes: give it
  another name with a `rename`.
