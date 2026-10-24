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
runtime: what std does that JS does not, with ownership and async in `rt/task.ts`.

| Rust | TypeScript |
| --- | --- |
| `bool`, `str`, `String` | `boolean`, `string` |
| `u8`-`u32`, `i8`-`i32`, `usize`, `isize` | `number`, checked: overflow panics |
| `u64`, `i64`, `u128`, `i128` | `bigint`, checked the same way |
| `struct` | a class; methods take `this`, trait methods included |
| `enum` without data | a union of strings: `"Lite01" \| "Lite02"`, or `never` without variants |
| `enum` | a union of objects tagged by variant, with a companion object of functions |
| `Option<T>` | `T \| undefined`; refused if `T` can already be `undefined`, as `()` and `Option` can |
| `Result<T, E>` | `rt.Result<T, E>`: `{ ok: true, value }` or `{ ok: false, error }` |
| tuple, `[T; N]`, `&[T]`, `Vec<T>` | an array, `Uint8Array` for bytes, `rt.ByteVec` for `Vec<u8>` |
| `&T`, `&mut T`, `Box<T>` | the value: a `&mut` must point at an object JS can change in place |
| closure | an arrow function |
| constant | its value, inlined as Rust inlines it |
| a type that drops | a class with `[Symbol.dispose]`: its `Drop` impl, then its fields |
| `Arc<T>`, `Weak<T>`, `Mutex<T>` | `rt.Arc`, counted; `rt.Weak`; `rt.Mutex`, its own guard |
| `async fn` | a function returning `rt.Future<T>`, around a generator |
| `Poll<T>`, `Waker`, `Context` | `T \| typeof rt.Pending`, refused if `T` can already be pending; `rt.Waker`, which is also the context |

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

Functions and members are camelCase. Functions are monomorphized: each instance is its
own function, and instances that share a name are suffixed by type, as in
`roundtripLite$Group` or `get$Wrap_Pair`. Types are translated once, generically, so a
generic class holds a method per instance, each declaring the instance it takes as
`this`: `fmt$Pair(this: Wrap<Pair>, f: rt.Formatter)`. A `Copy` struct, tuple, or
array, or an `Option` or `Result` holding one, is copied where Rust copies it, since JS
would share the object.

A value drops where Rust drops it, if dropping it does something JS can see: a `Drop`
impl, an `Arc`'s count, or a waker. A variable drops at the end of its scope through a
`finally`, so an early return, `?`, a panic, or a cancelled future drops it too, and a
move leaves `undefined` where it was, which drops as nothing.

An `async fn` returns a future without running anything: `rt.future(body, drop,
...upvars)`, whose body is a module-level generator of what the function captured. Each
`.await` polls in a loop around `yield`. A poll after the future returned panics, and
dropping it cancels it: unpolled, it drops what it captured by value; suspended, its
generator returns through the `finally` blocks that drop its locals and captures.
`rt.spawn(future)` runs one as a promise, polling woken tasks in a batch from a
microtask and yielding to a macrotask after 128 polls.

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
- Bodies lower from THIR, which keeps loops, `match`, and `?` shaped as written. Drops
  come from its scopes, and async bodies are generators: MIR state machines were about
  twice as fast per await level, but read as basic blocks and would need a second
  lowering ([#16](https://github.com/kixelated/rust2/pull/16) measured both).

## Never

Outside the subset for good: `unsafe` and raw pointers, and trait objects other than
`&dyn Debug` and `&dyn Display`. A boundary that needs one, such as a boxed future, will
bind to hand-written TS through config ([model](/quest/m2/ts-model.md)).

## Not yet

These fail translation: floats, `char`, statics, async blocks that borrow a number
mutably and async closures, values that drop as temporaries not moved whole, in a
closure's captures, in a `for` loop, in an enum, tuple, or generic `Drop` type, borrowed
in an `Arc` or `Mutex`, or moved ahead of an `.await` in the same call or assignment
([Drop](/quest/m2/ts-drop.md)), iterator adaptors (`for` over ranges, slices, arrays, and
`Vec` works), or-patterns that bind, other format specs (a precision, `+`, `{:b}`, `{:x?}`,
and widths from arguments), writing to a `String` with `write!`, a `move` closure
copying a mutable variable, `mem::swap`, `replace`, or `take` of a reference's whole
pointee, assigning an `Arc`, a waker, or a future through a reference, and std functions
without a mapping in `src/lower/library.rs`.

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
  and writes its errors' `Display` and `Debug`.
- `lang` covers every integer width's arithmetic and casts, control flow, patterns,
  `Copy` aggregates, formatting, and methods on generic types. Its differential test runs
  through `caller.ts`, a hand-written caller that also prints a `printable` error nothing
  translated prints.
- `task` trims moq's kio to a channel of `Arc`ed state, waiters, and wakers, under async
  functions. Its caller is hand-written TS (`tests/task.ts`): a FIFO executor that runs a
  script of spawns, sends, polls, and drops, and logs every poll, wake, and drop, as the
  fixture's native executor does. The logs must match byte for byte.
- `reject` holds constructs outside the subset; its expected errors are `expected.txt`,
  and its expected census is `census.md`.

`tests/spawn.ts` checks that `rt.spawn` yields to a timer after its 128-poll budget, and
rejects when a future panics as it drops.

## Bench

```bash
nix develop .#bench --command just ts bench  # report to target/bench/report.md
```

The codec fixture's `bench::run` holds the cases moq's own TypeScript benchmarks time
(`js/net/bench/varint.ts` and `lite-varint.ts`), at the pinned moq revision. Each runs
four ways: natively, as wasm32 at moq-wasm's size-optimized profile, as moq's
hand-written `js/net` (`bench/moq.ts`), and translated; every route's output must match
native Rust's first. The JS routes run in Node, Bun, and Chrome, with the WASM route's
copies across the boundary timed apart from its compute, and an A/A copy of the
translation to show the noise. The report records the host and engine versions, and
each route's bundle size raw and gzipped. It sets no threshold; the nightly workflow
runs it and posts the report to its summary.

The task fixture's `bench::run` times an async handoff: a value sent to a task parked on
its channel, read through 1, 4, or 16 async functions. Its translation runs polled at
once, as natively, and on `rt.spawn`; its hand-written route is the same handoff with
promises, as moq's TypeScript writes it.

The bench shell adds Bun, Chromium, and the wasm-bindgen that builds moq-wasm's package.
