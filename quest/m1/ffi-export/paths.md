# [S] Paths without a lifetime

## Goal

The exporter exports moq's owned `Path`: the census exports `Path::new`, `strip_prefix`,
and `Relative::new`, [the exclude list](/ffi/moq/ffi.toml) drops `AsPath` and the `borrow`
methods, and only `Path::from_static`'s `'static` borrow is refused.

## Plan

[moq-dev/moq#6843](https://github.com/moq-dev/moq/pull/6843), which makes `Path` owned,
merged on 2026-10-27. Bump the moq pin past it and report the census before and after.

Public API: the C names moq's bindings get.
