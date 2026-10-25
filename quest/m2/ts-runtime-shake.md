# [M] Tree-shakable runtime

## Goal

[rust2-ts](/ts/README.md)'s runtime is free functions: `rt.Iter`'s adaptors,
`rt.Formatter`, and similar become individually exported functions, so a consumer's
bundler drops what translated code never calls. It lands only if `just ts bench`
measures it within noise of today's class form or faster, with a CI tree-shake test:
a consumer importing one function, asserting nothing unused survives in the final
minified JS.

## Plan

Decided with the maintainer on 2026-10-24: the runtime is always free functions,
as translated types' functions now are, because callers never touch `rt` directly.
Bundle size counts the consumer's final JS: a Bun production build (`bun build
--minify`, which tree-shakes) of a small consumer, raw and gzipped. The codec test's
tree-shake check leaves the runtime out of its bundle today; it can take the runtime
in once the runtime shakes. Today the runtime is most of a small caller's bundle: about
15 KB minified, 5 KB gzipped, for a caller of one varint function in moq-net.

Public API: the runtime's exports, which translated code calls.
