# Migrating

## Unreleased

- `Vec<T>`, `PathBuf`, `SocketAddr`, and `url::Url` map, so a struct or enum whose
  fields were refused only for them is a value now, not an object: moq-tokio's
  `listen::Bind` is the C enum `moq_listen_bind`, freed with `moq_listen_bind_free(&b)`,
  not a handle. An item that clashes with the new names `<prefix>_socket_addr`,
  `<prefix>_url`, or a `<prefix>_vec_<T>` takes a `rename`.

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
