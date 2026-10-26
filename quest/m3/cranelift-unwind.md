# [XS] Retrial Cranelift on moq's tests once they pass

## Goal

Waits on upstream. 42 of moq's tests fail under the Cranelift backend that
ships with nightly, so moq's CI cannot build its tests with it:

- It builds every panic as an abort, so a test that catches one fails.
  Unwinding on Unix is a flag on cg_clif's own build, off by default over its
  compile-time cost
  ([rustc_codegen_cranelift#1567](https://github.com/rust-lang/rustc_codegen_cranelift/issues/1567)).
- It lacks the x86 intrinsic `llvm.x86.sse2.pmulhu.w`
  ([rustc_codegen_cranelift#171](https://github.com/rust-lang/rustc_codegen_cranelift/issues/171)).

Check the issues, or the [build benchmark](/bench/build/README.md)'s moq
report after a nightly pin bump: once `test-run` cranelift fails no tests,
decide again from the test scenarios whether moq's CI builds its tests with
Cranelift, and delete this quest.
