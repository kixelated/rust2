# [S] Import bindings that adding code elsewhere keeps

## Goal

The name a module binds an import by depends only on what it imports, never on which other
imports the translation reaches. An added import renames no other binding and no local, so
a module's generated diff changes only where its own code did. `stable_names` proves it.

## Plan

Found while working on [ts-stable-names](/quest/m2/ts-stable-names.md) on 2026-10-30.

`print::Bindings` gives each imported name and each type module imported whole its plain
name, or else `{module}_{name}` and then a number, taking them in the order of the
imported symbols' keys. Two imports of one name in a module therefore bind in key order:
lang's `core/option.ts` imports `counts.Noisy` as `counts_Noisy` and `locals.Noisy` as
`locals_Noisy`. Reaching another import of a taken name that sorts first can move the
plain binding to it and rename the import that held it, and every use in the module with
it. Locals renamed away from a module's bindings (`x_`) shift the same way when a binding
is added.

This is independent of how instances are named, which the other quest settles: a binding
can collide even when every exported name is stable.

Recommended direction: derive each binding from the imported symbol and its module alone,
for example by qualifying any import whose name another symbol the module might import
could share, rather than only the second one reached. Weigh how many bindings that renames
against how rarely a module imports two of one name.

Proof: extend `stable_names` so an added root makes a module import a second symbol of a
name it already imports, sorting before the first.

Public API: none beyond the generated files' import lines and local names.
