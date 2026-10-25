# Migrating

## Unreleased

- `uniffi_bindgen`: a name no longer identifies a type across crates.
  `ComponentInterface::is_name_used_as_error(name)` is now `is_used_as_error(&ty)`,
  `ComponentInterface::get_type(name)` finds only the crate's own types, and
  `VisitMut::visit_error_name` takes the module path of the crate that defines the error.
- `uniffi-bindgen-cpp`: the `--scaffolding` flag is gone. It only stubbed out generation.
- Generated C++: async needs `error_style = "expected"`, and the default exceptions style refuses
  an async function, constructor, method, or callback interface method. Its `uniffi::Future<T>`,
  `FutureResult`, `ForeignFuture`, `AsyncCancelledError`, and `AsyncDispatcherError`, and the
  `<Trait>Foreign` interface of a trait with async methods, are gone. Switch to the expected
  style, where an async call returns `uniffi::Future<T, E>`.
- Generated C: every type and function name starts with the crate's namespace, so a crate
  whose items did not already start with it gets renamed C symbols.
- Generated C: methods and named constructors gain `_method_` after their type's name
  (`app_my_object_do_thing` is now `app_my_object_method_do_thing`), and so do their async
  callback typedefs. The primary constructor `_new`, `_free`, and top-level functions keep their
  names.
