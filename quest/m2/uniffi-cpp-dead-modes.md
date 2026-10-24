# [S] Delete the C++ generator's dead modes

## Goal

[The fork](/uniffi/README.md)'s C++ generator drops two modes no consumer uses:
its scaffolding mode, and async under `error_style = "exceptions"`. The deletion
is listed under the fork README's "Carried changes".

## Plan

- Scaffolding mode (`--scaffolding`) has no consumer and has gone untested since
  the import, which left out the C++ scaffolding tests.
- Exception-style async has no consumer: moq uses the expected style. Codex found
  a cancel/free race and a result leak in it.

Public API: the generator's `--scaffolding` flag, and async under the exceptions
error style.
