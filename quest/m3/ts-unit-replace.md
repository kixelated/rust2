# [XS] mem::replace and mem::swap of ()

## Goal

`mem::replace` and `mem::swap` through a `&mut ()` translate to output `tsc` accepts and
that matches native Rust, proved by a differential test. Found while testing writes through
a dangling `&mut ()` for [waker identity](https://github.com/moq-dev/rust2/pull/334): in the
task fixture, `mem::replace(Box::leak(Box::new(())), ())` with its result unused declared an
`old` that nothing reads (TS6133), and `mem::swap(Box::leak(Box::new(())), &mut ())`
assigned to `undefined` (TS2539).
