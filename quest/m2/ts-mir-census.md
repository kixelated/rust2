# [S] MIR's own census refusals

## Goal

[rust2-ts](/ts/README.md)'s census of moq-net and kio refuses no function the THIR
lowering translated, and each refusal of a type points at the local or expression that
holds it rather than at its function.

## Plan

At the move to MIR, 16 function instances of moq-net and kio that THIR translated
clean, before collections landed, are refused:

- `unsupported type ... from another crate` for temporaries THIR never declared
  (`Patterns`, `Duration`, kio's `Mut`, `Shared`, and `Consumer`, an atomic,
  `MaybeUninit`), reported at the function's span. Each should carry its local's span,
  from `local_decls`.
- Drops THIR's scopes never saw: `Option<Rank<..>>`, `AdapterSend`, `Box<[..]>`,
  `Box<L>`'s `Drop`, and an `Option<Error>` replaced through a `&mut`.

MIR also refuses constructs THIR wrote differently:

- `ControlFlow` values other than the one `?` makes (27 sites): pick a representation,
  such as `rt.Result`'s shape or a tagged object, by what `try_fold` and moq's own uses
  need.
- A switch on, or a field of, a type from another crate without a mapping
  (`kio::Unused`, `Bound`, `MaybeUninit`, `ManuallyDrop`).
- A switch on `char` (6), once `char` has a representation.

Public API: the translator's output for the functions it newly translates.
