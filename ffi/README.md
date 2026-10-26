# rust2-ffi

A rustc driver that exports a crate's `pub` API as a C ABI and a C header, with no
attributes in the Rust source. It replaces [the UniFFI fork](../uniffi/README.md) for moq,
starting from C, which every other language's generator will build on.

```bash
rust2-ffi --config ffi.toml (--out dir | --census dir) [--manifest-path Cargo.toml] [-- cargo args]
```

It runs `cargo check` on the package with itself as `RUSTC_WRAPPER`, on the host, through
the front end it shares with [rust2-ts](../ts/README.md) in [`driver/`](../driver). Once
rustc has checked the configured library, it walks every `pub` item reachable from the
crate root and exports it by fixed rules, or refuses it with a compile error at its span.
It walks each other package the config lists the same way, from the metadata of the
crate the library builds with, so one library holds them all and an export names
another's types across crates. `--out` writes a crate of `extern "C"` functions over the
libraries, its runtime beside it, and the header in `include/<prefix>.h`; `cargo build`
on that crate makes a static and a shared library.

```toml
# ffi.toml, beside the package's Cargo.toml unless --manifest-path says otherwise.
package = "moq-net"
# The prefix of every C name, the package's name in snake case by default.
prefix = "moq"
# Paths from the crate root left out: a module, an item, or one method or field. Each
# must name something exported. Nothing else leaves an item out.
exclude = ["track::Producer::poll_closed"]

# C names, without the prefix, for paths whose names the naming rule makes clash.
[rename]
"session::Stats" = "session_stats_snapshot"

# Another package exported into the same library, which the package builds with: its
# own `exclude` and `rename`, by paths from its crate root.
[crates.moq-pattern]
exclude = ["Patterns::iter"]
```

## Rules

Every `pub` item reachable from an exported crate's root, by its shortest path, is
exported or refused. `#[doc(hidden)]` items are no part of the API and are left out; type
aliases export nothing of their own. A re-export of another export's item names that
export's, and adds nothing; another crate's type is refused.

| Rust | C |
| --- | --- |
| a struct whose fields are all `pub`, not `#[non_exhaustive]`, which map, with no `Drop` of its own | a record: a struct, copied across |
| an enum whose fields map, with no `Drop` of its own | a C enum without fields, or a struct of a tag and a union |
| any other type, if `Send` | an object: an opaque handle to an `Arc`, freed with `_free` |
| a type whose only generic parameters are lifetimes, such as `Path<'a>` | an object holding its `'static` form |
| a `pub` field of an object | a getter returning a copy |
| a `pub` trait | a callback interface: a table of C functions and their data |
| `fn` and inherent `pub` methods | functions named through their type |
| `Result<T, E>` | `bool f(..., T *out, E *err)` |
| a returned tuple `(A, B)` | one out pointer each: `void f(..., A *out0, B *out1)` |
| `async fn`, or a return that implements `Future` | a future: `f` makes it, `f_poll` polls it, `f_future_free` drops it; a function that is not `async` runs at the call, as in Rust |
| `poll_x`, when `x` exports as a future | left out: the future is its poll |
| `bool`, integers but 128-bit, floats, `char` | `bool`, `<stdint.h>` types, `float`, `double`, a `uint32_t` code point |
| `&str`, `String`, `&[u8]`, `Vec<u8>`, `bytes::Bytes` | `<prefix>_buf`: a view passed in, an owned buffer returned |
| `Duration` | `<prefix>_duration` |
| `Range<T>`, `Bound<T>` of an integer | `<prefix>_range_<T>`, `<prefix>_bound_<T>`: a `<prefix>_bound` kind and a value |
| `impl RangeBounds<T>` of an integer | `<prefix>_bounds_<T>`: its start and end bounds |
| `Option<T>` | a null handle or buffer, or `<prefix>_option_<T>` |
| a `pub const` | a `#define` if a `bool` or integer, else a function returning it |
| `impl Into<X>`, `impl AsRef<X>` | `X` |
| another type parameter | the first of `String`, `Bytes`, and `Vec<u8>` meeting its bounds, or a foreign implementation of its one exported trait |

A record or enum holds only values and `Copy` handles: one with a field that does not map,
or that holds itself, is an object instead, and the census lists it. A borrowed return is
returned as a clone, so it must be `Clone`, and a handle in it is a new handle to the
clone. A handle outlives every call, so every lifetime an object takes is `'static`, as is
every lifetime that must outlive one; any other lifetime is the call's. A signature may
not lend C's borrow for one that is `'static`, nor return an object that borrows. A
parameter is a reference only at its top, not inside an `Option` or `Box`, and an object
no export C can call returns is refused, since C could never hold one. Everything else is
refused, and the census counts why.

## The C ABI

- **Handles** are `Arc`s, and a call borrows the value as Rust's receiver or parameter
  does, checked at run time: shared for `&T`, exclusive for `&mut T`, and moved out for
  `T`, after which the handle is only freed. A `Copy` value is copied out instead, and a
  value that is not `Sync` is shared on one thread at a time, and held exclusively by a
  future, which may be polled on any thread. A borrow Rust would
  refuse at compile time aborts, naming the function, as does a null or moved-out handle:
  there is no Rust caller to return an error to.
- **Values** are copied in from the caller's memory, which Rust never frees, and lowered
  into memory the caller frees once with the type's `_free`.
- **Futures** run no runtime: C polls with a `<prefix>_waker` and polls again when woken.
  A poll that finishes writes the result through the poll's own out pointers, so a
  cancelled read never loses a value. The future reuses the last waker when given the same
  one again, and owns what it borrowed: the handle it was called on stays borrowed until
  it finishes or is freed.
- **Callbacks** are a struct of a table and its data. Rust calls `free` once it drops the
  implementation, and may call the others from any thread. A callback takes scalars and
  buffers lent for the call, and returns a scalar.
- **Panics** abort, as an `extern "C"` function's do.
- **Names** are the prefix, then the item's path from its crate's root, each segment in
  snake case, joined by `_`: `track::Producer::append_group` is
  `moq_track_producer_append_group`, and moq-pattern's `Patterns` is `moq_patterns`.
  Joining loses where segments split, and every export shares the prefix, so every C
  name is registered, and two items claiming one is a compile error at both spans, fixed
  by a `rename`: moq-net's `session::Stats` and `Session::stats` are both
  `moq_session_stats`.

## IR

[`src/ir.rs`](src/ir.rs) is the API as each generator reads it, with no language decided:
records, enums, objects, callbacks, functions, and constants of every export, each by
its Rust path, its span, and its docs. A type is in the exact form Rust takes or returns it
(`&str` apart from `String`, `Box<dyn Trait>` apart from `impl Trait`), so a generator can
borrow where Rust borrows. [`src/abi.rs`](src/abi.rs) decides the C ABI from it once,
names and signatures, so [`src/c.rs`](src/c.rs) and [`src/glue.rs`](src/glue.rs) agree;
the runtime the glue includes is [`runtime/rt.rs`](runtime/rt.rs).

## Census

`--census dir` exports nothing. It writes `<package>.md` for each exported package,
which counts what exports and ranks every refusal by construct and by the type it names,
lists each refused item at its span, and ends with the `exclude` list that exports the
rest. It ignores the config's `exclude`.

## Tests

```bash
just ffi test     # export the fixtures, build them, and drive api from C
just ffi moq      # export moq-net and its exports at the pinned revision, build them, and run moq/moq.c
just ffi census   # census each into target/census
nix develop .#bench --command just ffi bench  # frame writes, through the export and moq's rs/moq-c
```

The `api` fixture covers each rule once, and names the `dep` fixture's types, which
export into the same library. `tests/api.c` runs the same script as the
fixture's `examples/transcript.rs`, and the two transcripts must match line for line;
then it breaks each runtime check, which must abort with its message. The `reject`
fixture's expected errors, each with its notes, are `fixtures/reject/expected.txt`.
moq-net's `moq/moq.c` publishes and subscribes to a track in one process, polling each
read, and scopes an origin to moq-pattern's patterns.
