# [S] Binding type errors keep their span through a symlinked output directory

## Goal

[rust2-ts](/ts/README.md) reports each binding that fails `tsc` at its Rust item even when
the output directory sits behind a symlink, so `just ci` passes in a worktree whose
`ts/target` is a symlink, as mbx makes it.

## Plan

In m0: the `bind` fixture test fails in a worktree on this host where mbx links
`ts/target` into `~/.cache/mbx/targets/v1/<hash>`, so the local gate fails there. Found in
[#286](https://github.com/moq-dev/rust2/pull/286), which doesn't touch bindings: its
branch failed `bind` in such a worktree and passed in one with a real `ts/target`, as main
did.

Reproduction: in such a worktree, `just ts test --test fixtures bind`. Each expected
`the binding of ... doesn't type-check: ... at fixtures/bind/...` turns into a bare
`tsc: ../../../../../../../../worktrees/<...>/ts/target/tmp/bind/wrong/rt/bind.ts(19,2): ...`.

Cause: `typecheck` in `ts/src/lower/bind.rs` runs `tsc -p` with `current_dir` set to the
runtime directory under `out`. The OS resolves the process's working directory through the
symlink, while `-p` names the project by the linked path. tsc then prints each file
relative to the real directory, as a run of `../` back to the linked path. The cleanup
strips a leading `../` and the absolute `out` and `out.canonicalize()`, so it matches
neither form. With no `bind.ts(` prefix, no check is found and the message loses its span.

Fix it at the source: canonicalize the runtime directory once, and pass that same real
path as both `-p` and `current_dir`, so tsc prints `bind.ts(line,col)`. Drop whatever path
stripping that leaves dead. Do not add another pattern to strip. Cover it in
`ts/tests/fixtures.rs`: run the `bind` fixture's `wrong` config through an output
directory reached by a symlink under `CARGO_TARGET_TMPDIR`, so CI fails without the fix
whether or not its target is linked.
