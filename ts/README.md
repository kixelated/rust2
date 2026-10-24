# rust2-ts

A rustc driver that translates a subset of Rust to readable TypeScript with source
maps. No WASM: the output is plain ES modules that Node, Bun, and bundlers run as is.

```bash
rust2-ts --config ts.toml (--out dir | --census file.md) [--manifest-path Cargo.toml] [-- cargo args]
```

It runs `cargo check` on the package with itself as `RUSTC_WRAPPER`, for
`wasm32-unknown-unknown`. Every crate compiles as rustc would; the configured library is
also translated, after type and borrow checking, starting from its roots and following
what they reach. A construct outside the subset is a compile error with its span, and
then nothing is written.

```toml
# ts.toml, beside the package's Cargo.toml unless --manifest-path says otherwise.
package = "moq-net"
# Paths from the crate root: a function, or a type with its non-generic methods and its
# impls of the crate's own traits, blanket ones included.
roots = ["coding::Decoder", "lite::Group"]
# Types whose `Display` and `Debug` impls come along, whichever they have, so callers can
# print them. Another crate's trait otherwise comes along only when something calls it.
printable = ["coding::DecodeError"]
# Macros whose invocations are left out, such as logging. Each must evaluate to `()`.
ignore-macros = ["tracing::trace"]
```

moq consumes it as the `ts` package of this repository's flake. In this repository,
`just ts run ...` runs it from source on the pinned nightly.

## Census

`--census file.md` translates nothing. It treats every item in the library as a root,
except derived impls, which come along only when called or printable, and writes a Markdown report of
every construct translation refuses, with counts and spans. Refusals are ranked by
construct, by the type or module they name, and by each type or path. A generic function
that no instance reaches is lowered over its own type parameters, refusing nothing that
hinges on what they are, since each instance decides that. A census succeeds whatever
it finds; translation still fails on any refusal.

`just ts census` runs it on moq-net and kio at the pinned moq revision, into
`target/census`, and the nightly job publishes their tables.

## Output

Each Rust module with translated items becomes a file (`lib.ts`, `coding/varint.ts`),
with a `.ts.map` source map into the Rust it came from. `rt.ts` beside them is the
runtime: what std does that JS does not, with iterators in `rt/iter.ts`.

| Rust | TypeScript |
| --- | --- |
| `bool`, `str`, `String` | `boolean`, `string` |
| `u8`-`u32`, `i8`-`i32`, `usize`, `isize` | `number`, checked: overflow panics |
| `u64`, `i64`, `u128`, `i128` | `bigint`, checked the same way |
| `struct` | a class; methods take `this`, trait methods included |
| `enum` without data | a union of strings: `"Lite01" \| "Lite02"`, or `never` without variants |
| `enum` | a union of objects tagged by variant, with a companion object of functions |
| `Option<T>` | `T \| undefined`; an `Option` of `()` or an `Option` is refused |
| `Result<T, E>` | `rt.Result<T, E>`: `{ ok: true, value }` or `{ ok: false, error }` |
| tuple, `[T; N]`, `&[T]`, `Vec<T>` | an array, `Uint8Array` for bytes, `rt.ByteVec` for `Vec<u8>` |
| `&T`, `&mut T`, `Box<T>` | the value: a `&mut` must point at an object JS can change in place |
| an iterator | `rt.Iter<Item>`, whose adaptors are its methods: `v.iter().map(f)` |
| `a..b`, `a..=b` | `rt.Range<T>`, `rt.RangeInclusive<T>`, which are iterators too |
| closure | an arrow function |
| constant | its value, inlined as Rust inlines it |

`usize` is 32 bits, as on wasm32, so it is exact in a `number`. Arithmetic is checked as
a debug build checks it, so translated code panics where native code would: it never
wraps silently. A panic throws `rt.Panic`, with the message native Rust panics with.

`format!`, `write!`, `panic!`, `assert!`, and the other macros of `format_args!`
translate, as do `Display` and `Debug` impls, derived and thiserror's included. The
runtime ports `core::fmt`, so the text matches native Rust's byte for byte. A placeholder
becomes a function writing its value, and `fmt::Arguments` an array of literal pieces
and such functions: `f.writeFmt(["code=", (f) => rt.u16.display(code, f)])`. A `&dyn
Debug` or `&dyn Display` is such a function too. Format specs take fill, alignment,
width, `#`, `0`, `x`, `X`, and `?`. `Debug` of a string escapes by the JS engine's
Unicode tables, so a character newer than its Unicode version is escaped where Rust
prints it. A caller prints a `printable` type as `to_string()` and `format!("{err:?}")`
would: `rt.fmt.toString((f) => DecodeError.fmt$Display(err, f))` and
`rt.fmt.format([(f) => DecodeError.fmt$Debug(err, f)])`.

An iterator's `next()` returns its next item or `rt.DONE`, never `undefined`, so an
iterator of `Option` keeps each `None` apart from its end. std's adaptors and consumers
are `rt.Iter` methods, lazy and short-circuiting as std's are, and call their closures in
the same order and as often. A `for` loop reads an iterator by that protocol, a slice,
array, `Vec`, or `slice.iter()` with `for ... of`, and a range written in place with a
counter. The crate's own `impl Iterator` returns `rt.DONE` from its `next` too, and joins
std's adaptors through `rt.iter.from`. A caller calls `next()` for one item, or iterates
an `rt.Iter` with `for ... of`.

Functions and members are camelCase. Functions are monomorphized: each instance is its
own function, and instances that share a name are suffixed by type, as in
`roundtripLite$Group` or `get$Wrap_Pair`. Types are translated once, generically, so a
generic class holds a method per instance, each declaring the instance it takes as
`this`: `fmt$Pair(this: Wrap<Pair>, f: rt.Formatter)`. A `Copy` struct, tuple, or
array, or an `Option` or `Result` holding one, is copied where Rust copies it, since JS
would share the object.

A file's items, and a class's or companion's methods, follow the Rust source, whatever
order translation reaches them in, so adding a root moves nothing it does not reach.
What a macro makes sits where the macro is called. The instances of one function sit
together, sorted by their suffix (`Pair` in `fmt$Pair`) and then by their full type
arguments.

Imports name the `.ts` file, which Node, Bun, and bundlers run as is. A `tsc` build sets
`rewriteRelativeImportExtensions`, or `allowImportingTsExtensions` with `noEmit`.

## Design

- 64-bit integers are native `bigint`: lossless and readable. A pair of `u32` halves is
  faster but waits until `just ts bench` measures `bigint` as the cost.
- A generic type is one class or union, the type TS callers name, and a method on it is
  one instance's, typed by its `this`. A class per instance (`Wrap$Pair`) would rename
  the type for callers, and TS generics with trait dictionaries would undo
  monomorphization.
- `Result` is a plain `{ ok, value } | { ok, error }` object rather than an exception, so
  `?` and `match` keep Rust's control flow exactly. Making no `Ok` objects at all measured
  no faster on `just ts bench`, in V8 or JavaScriptCore.
- Iterators are the runtime's lazy classes rather than generators or ES iterator
  helpers: std's adaptors have no THIR to lower, ES helpers lack most of them
  (`rev`, `zip`, `peekable`), and a `next` returning an item or `rt.DONE` allocates
  nothing per item. `just ts bench` measures chains of `map`, `filter`, `any`, `skip`,
  and `take` 1.4-8x faster than the same ES helpers in Node, Bun, and Chrome.
- Bodies lower from THIR, which keeps loops, `match`, and `?` shaped as written. It does
  not say where values drop; [Drop](/quest/m2/ts-drop.md) decides how to get that from
  MIR.

## Never

Outside the subset for good: `unsafe` and raw pointers, and trait objects other than
`&dyn Debug` and `&dyn Display`. A boundary that needs one, such as a boxed future, will
bind to hand-written TS through config ([model](/quest/m2/ts-model.md)).

## Not yet

These fail translation: floats, `char`, statics, `async`,
types that implement `Drop`, iterators of std's collections and the adaptors without a
mapping in `src/lower/iter.rs` (such as `cycle`, `unzip`, and `partition`), an own
`impl Iterator` with `next_back`, `len`, or another provided method of its own,
`while let` or `match` on the `next` of an iterator of `Option`, or-patterns that bind, other format specs (a precision, `+`, `{:b}`, `{:x?}`,
and widths from arguments), writing to a `String` with `write!`, a `move` closure
copying a mutable variable, `mem::swap`, `replace`, or `take` of a reference's whole
pointee, and std functions without a mapping in `src/lower/library.rs`.

## Tests

```bash
just ts test                  # translate each fixture, compare, type-check, and diff
UPDATE_EXPECT=1 just ts test  # rewrite the snapshots in fixtures/*/expected
just ts moq                   # translate moq-net's codec at a pinned revision
just ts census                # census moq-net and kio at the same revision
```

Each fixture under `fixtures/` is translated and compared with its `expected/` snapshot,
then type-checked with `tsc`. A fixture with a `harness::run(&[u8]) -> Vec<u8>` is also
built natively for wasm32 and run beside its translation on the same inputs by
`tests/differential.ts`, which fails on any difference in output or panic messages.

- `codec` vendors moq-net's sync codec, which a harness outside the crate could not reach,
  and writes its errors' `Display` and `Debug`. It is also translated without
  `bench::run` and then with it as the first root, which must keep every line in order.
- `lang` covers every integer width's arithmetic and casts, control flow, patterns,
  `Copy` aggregates, formatting, methods on generic types, and iterators, with closures
  that log what each adaptor reads. Its differential test runs through `caller.ts`, a
  hand-written caller that also prints a `printable` error nothing translated prints.
- `reject` holds constructs outside the subset; its expected errors are `expected.txt`,
  and its expected census is `census.md`.

## Bench

```bash
nix develop .#bench --command just ts bench  # report to target/bench/report.md
```

The codec fixture's `bench::run` holds the cases moq's own TypeScript benchmarks time
(`js/net/bench/varint.ts` and `lite-varint.ts`), at the pinned moq revision, and
iterator chains over bytes. Each runs four ways: natively, as wasm32 at moq-wasm's
size-optimized profile, hand-written (moq's `js/net` in `bench/moq.ts`, and ES iterator
helpers in `bench/iter.ts`), and translated; every route's output must match
native Rust's first. The JS routes run in Node, Bun, and Chrome, with the WASM route's
copies across the boundary timed apart from its compute, and an A/A copy of the
translation to show the noise. The report records the host and engine versions, and
each route's bundle size raw and gzipped. It sets no threshold; the nightly workflow
runs it and posts the report to its summary.

The bench shell adds Bun, Chromium, and the wasm-bindgen that builds moq-wasm's package.
