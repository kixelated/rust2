# [S] A/B routes in the TS benchmark

## Goal

`just ts bench --route name=dir` loads another translation as a route beside the
current one, so an A/B comparison of two translator versions is one reproducible
command.

## Plan

[#15](https://github.com/kixelated/rust2/pull/15) and
[#26](https://github.com/kixelated/rust2/pull/26) both patched `bench/main.ts` by
hand to time another translation (main's, or a rejected representation's).

Public API: a `just ts bench` option.
