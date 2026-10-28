# [S] A closure writing through a captured mutable reference

## Goal

A closure that captures an integer by `&mut` and writes it, called once or more, through
`FnOnce`, `FnMut`, or a generic, leaves the caller's variable or field written, as native
Rust does, proven by differential tests against native Rust.

## Plan

Found writing [#147](https://github.com/kixelated/rust2/pull/147)'s stock fixture: a closure
capturing a `u8` by `&mut`, called once, loses its write. Find where lowering decides the
captured place needs no `rt.Place` and fix it at the source for every capture path.
