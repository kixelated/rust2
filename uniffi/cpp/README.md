# uniffi-bindgen-cpp - UniFFI C++ and C bindings generator

Generate bindings for C++, or C with `--lang c`. See [the fork's README](../README.md) for how to
build it and where it came from.

# How to generate bindings

## Generating with a single UDL file

```bash
uniffi-bindgen-cpp path/to/definitions.udl
```

## Generating using a library file

```bash
uniffi-bindgen-cpp --library your_rust_library.so --out-dir output_directory
```

Regardless of the generation method, these files are produced:

* `path/to/definitions.hpp`
* `path/to/definitions.cpp`
* `path/to/definitions_scaffolding.hpp`

# How to integrate bindings

To integrate the bindings into your projects, simply add the generated bindings files to your project.
C++17 or newer is required to compile the bindings.

Async calls need `error_style = "expected"` (see [the configuration](docs/CONFIGURATION.md)); the
default exceptions style refuses to generate them. An async call returns a move-only
`uniffi::Future<T, E>`. It supports `get()`, `wait()`, `wait_for()`, and `cancel()`. Destroying an
incomplete future cancels it and releases the Rust future.

To adapt without a blocking waiter thread, consume the future with
`std::move(future).then(executor, callback)`. The returned `uniffi::FutureContinuation` owns
cancellation until completion. Destroying that token early cancels the Rust operation.

Rust-future continuations use one background dispatcher by default. Before making any async
call, applications can call `uniffi::set_async_dispatcher(dispatch, shutdown)` to use their own
executor. Before unloading the executor or generated bindings, call
`uniffi::shutdown_async_dispatcher()`; it rejects new continuations and waits for the registered
shutdown function to drain accepted work.

# C bindings

`--lang c` generates a C11 header, `<namespace>.h`, over the C++ bindings. It writes the
C++ bindings in the `error_style = "expected"` style too, plus `<namespace>_c.cpp`, which
converts between the two. Compile `<namespace>.cpp` and `<namespace>_c.cpp` as C++17 (exceptions
and RTTI may be off) and include only `<namespace>.h` from C.

```bash
uniffi-bindgen-cpp --lang c --library your_rust_library.so --out-dir output_directory
```

Names are the uniffi names in snake case, prefixed with the namespace unless they already start
with it, since every crate's C API shares one scope: in namespace `app`, `MyObject.do_thing` is
`app_my_object_do_thing`, and its handle is `app_my_object *`, released with
`app_my_object_free`; `AppObject` is `app_object`. The rest follows from a few rules:

* A call that can fail returns an error struct, or NULL on success, and writes its result to a
  trailing `out` pointer. An error is a tagged union with a `message`; free it with `<error>_free`.
* Records are plain structs, data-carrying enums are a `tag` plus an anonymous union of
  `<enum>_<variant>` structs (`shape->circle.radius`), and flat enums are C enums. A record
  with `#[uniffi(default)]` fields gets `<record>_default()`.
* Arguments are borrowed for the call. Everything returned is owned by the caller and freed with
  the matching `_free` (`<namespace>_string_free` for strings).
* Optional strings, objects, and structs are pointers that are NULL when absent; optional scalars
  are `{ has_value, value }` structs. Bytes, lists, and maps are pointer and length structs.
* An async call takes a callback and `user_data` and returns a `<namespace>_task *`. The callback
  runs once on the dispatcher with the result, which it owns, unless the dispatcher refuses the
  work or has shut down, which abandons the call. Free every task: freeing a pending
  one cancels the Rust future, and once `<namespace>_task_free` returns the callback has finished
  or will never run.
* Callbacks run on the default dispatcher thread unless `<namespace>_set_dispatcher` installs a
  host's own, which receives `<namespace>_work *` items to run with `<namespace>_work_run`.
* A type from another crate keeps that crate's C name and `_free`, and its header is included.
  Generate the C API of every crate involved (one `--crate` each, or all at once) and compile
  each one's `<namespace>.cpp` and `<namespace>_c.cpp`; a crate's implementation converts its
  own types for the others.

The C backend refuses what it cannot render faithfully, such as callback interfaces, custom
types, timestamps, durations, and exported traits other than `Display` on errors, including in
another crate whose types it uses.

# Configuration options

It's possible to [configure some settings](docs/CONFIGURATION.md) by passing `--config`
argument to the generator.

Note: configuration is not supported when using library mode

```bash
uniffi-bindgen-cpp path/to/definitions.udl --config path/to/uniffi.toml
```
