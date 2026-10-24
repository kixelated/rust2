# Migrating

## Unreleased

- `uniffi_bindgen`: a name no longer identifies a type across crates.
  `ComponentInterface::is_name_used_as_error(name)` is now `is_used_as_error(&ty)`,
  `ComponentInterface::get_type(name)` finds only the crate's own types, and
  `VisitMut::visit_error_name` takes the module path of the crate that defines the error.
- Generated C: every type and function name starts with the crate's namespace, so a crate
  whose items did not already start with it gets renamed C symbols.
