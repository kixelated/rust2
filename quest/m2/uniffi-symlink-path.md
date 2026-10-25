# [S] The fork builds through a symlinked path

## Goal

A consumer that depends on [the fork](/uniffi/README.md) by a path through a
symlink (macOS's `/var` and `/tmp` are both) builds `uniffi_bindgen`.

## Plan

Seen on macOS CI of the [binding benchmark](/bench/ffi/README.md): with the
fork at `/var/tmp/.../uniffi`, askama's derive could not read `askama.toml`,
reaching for a relative path that climbs out of `/var/tmp/...` and back into
`/private/var/tmp/...`. The benchmark now builds from physical paths, so it no
longer hits this. Reproduce with a path dependency through a symlink, find
whether the mix comes from askama (0.16) or from how the fork names its config,
and fix it at that source.
