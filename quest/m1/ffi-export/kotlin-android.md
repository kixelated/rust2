# [XS] Kotlin bindings on Android in the nightly

## Goal

The nightly's `Export to Kotlin on Android` job passes: `just ffi android` runs the Kotlin
test, shrunk by R8 with the generated `<prefix>.pro`, on an x86_64 emulator at API 24 and 35.

## Plan

- The job landed before any run of its own; it passed on local emulators at both levels.
  Read the first nightly's run, and fix what it finds at the source, never by loosening
  the test.
- Unproven on the runner: KVM after the udev rule, the image's SDK and NDK through
  `ANDROID_HOME` and `ANDROID_NDK_HOME`, its newest build tools' R8, and `nix develop`
  inside `android-emulator-runner`'s script.
