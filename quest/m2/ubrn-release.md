# [XS] Waiting: ubrn releases UniFFI 0.32 support

## Goal

uniffi-bindgen-react-native (ubrn) publishes a release whose `wasm2` flavour
reads UniFFI 0.32 metadata, so the [binding benchmark](/bench/ffi/README.md)'s
browser runner pins a release instead of an open PR's head. Check:
[jhugman/uniffi-bindgen-react-native#468](https://github.com/jhugman/uniffi-bindgen-react-native/pull/468)
or a successor has merged, and a tag after `0.31.0-6` carries it.

## Plan

Then bump `[ubrn]` in `bench/ffi/pins.toml` to the release in its own change,
and take `@ubjs/core`, `@ubjs/wasm`, and `uniffi-runtime-wasm` from npm and
crates.io instead of building them from source. moq waits on the same release
([moq-dev/moq `quest/m2/rs2ts/ubrn-032.md`](https://github.com/moq-dev/moq/blob/main/quest/m2/rs2ts/ubrn-032.md)).
If ubrn stalls, carrying its TypeScript generator in the fork is the
alternative; that is the maintainer's call.
