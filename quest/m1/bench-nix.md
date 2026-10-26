# [S] Measure moq's fixed crane in the Nix benchmark

## Goal

`just bench-nix` ([bench/build/README.md](/bench/build/README.md#nix-builds))
measures moq's Nix release builds with crane as moq builds it after its layering
fixes, beside crane with mbx, crate2nix, and cargo with mbx, and the report says
which one moq should adopt, as a follow-up quest in moq.

## Plan

The harness and the other three variants landed, measured against moq's crane
before its fixes. Once moq-dev/moq `quest/m0/ci/crane-layers.md` merges, bump
nix.toml's pin to it, on its own since numbers on either side of a pin do not
compare, and compare the nightly numbers.

The first numbers bound what the fixes can win: a shared dependency layer can
only cut the second package's cost, while a lockfile bump still rebuilds every
dependency, three to five times what the per-crate caches pay. If moq adopts a
per-crate cache first, its crane fixes, and this quest, are moot.
