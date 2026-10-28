# [M] Kotlin bindings on Android

## Goal

The generated Kotlin runs on Android, not only the JVM: an app shrunk by R8 keeps the
methods JNI looks up by name, and an Android emulator run of the Kotlin tests proves it in
CI.

## Plan

Found by [#184](https://github.com/kixelated/rust2/pull/184) on 2026-10-28, which builds and
tests the Kotlin on the JVM only.

- R8 and ProGuard rename or strip `Native`'s dispatchers, which the JNI glue finds by name.
  Ship a keep rule with the generated Kotlin, in the form an Android library consumes
  (`consumer-rules.pro` or the AAR's `proguard.txt`), and test a minified build with it.
- The emulator run is slow: put it in the nightly workflow, not the merge gate.
- Check what the Android API levels moq targets give the Kotlin, such as the
  `java.lang.ref.Cleaner` question in [the Kotlin quest](/quest/m1/ffi-export/kotlin.md).

Public API: the keep rule shipped beside the generated Kotlin.
