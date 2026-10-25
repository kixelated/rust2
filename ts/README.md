# rust2-ts

A rustc driver that translates a subset of Rust to TypeScript with source maps. No WASM: the output is plain ES modules that Node, Bun, and bundlers run as is.

```bash
rust2-ts --config ts.toml (--out dir | --census file.md) [--manifest-path Cargo.toml] [--lower thir|mir] [-- cargo args]
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

`--lower mir` lowers bodies from MIR instead of THIR: the prototype the MIR spike
built, which the translator is moving to ([quest](/quest/m2/ts-mir.md)). The rest of
this page describes the THIR lowering, the default, except where it says MIR.

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
and each type a module of its own beside it, holding the type and its functions
(`coding/decode.Decoder.ts`), each with a `.ts.map` source map into the Rust it came
from. `rt.ts` beside them is the
runtime: what std does that JS does not, with iterators in `rt/iter.ts`, and ownership
and async in `rt/task.ts`.

| Rust | TypeScript |
| --- | --- |
| `bool`, `str`, `String` | `boolean`, `string` |
| `u8`-`u32`, `i8`-`i32`, `usize`, `isize` | `rt.u8` and so on: a `number` branded with its type, checked: overflow panics |
| `u64`, `i64`, `u128`, `i128` | `rt.u64` and so on: a branded `number` while a safe integer, 32-bit limbs past 2^53, checked the same way |
| `struct` | a plain object, typed by an interface; a class if it may drop |
| `enum` without data | a union of strings: `"Lite01" \| "Lite02"`, or `never` without variants |
| `enum` | a union of objects tagged by variant |
| `Option<T>` | `T \| undefined`; refused if `T` can already be `undefined`, as `()` and `Option` can |
| `Result<T, E>` | `rt.Result<T, E>`: `{ ok: true, value }` or `{ ok: false, error }` |
| tuple, `[T; N]`, `&[T]`, `Vec<T>` | an array, `Uint8Array` for bytes, `rt.ByteVec` for `Vec<u8>` |
| `&T`, `&mut T`, `Box<T>` | the value: a `&mut` must point at an object JS can change in place |
| an iterator | `rt.Iter<Item>`, whose adaptors are runtime functions: `rt.iterMap(rt.iterSlice(v), f)` |
| `a..b`, `a..=b` | `rt.Range<T>`, `rt.RangeInclusive<T>`, which are iterators too |
| closure | an arrow function |
| constant | its value, inlined as Rust inlines it |
| a type that drops | a class with `[Symbol.dispose]`: its `Drop` impl, then its fields |
| a type's function | a function of the type's module, taking `self` first: `Decoder.slice(dec, 4)` after `import * as Decoder` |
| `Arc<T>`, `Weak<T>`, `Mutex<T>` | `rt.Arc`, counted; `rt.Weak`; `rt.Mutex`, its own guard |
| `async fn` | a function returning `rt.Future<T>`, around a generator |
| `Poll<T>`, `Waker`, `Context` | `T \| typeof rt.Pending`, refused if `T` can already be pending; `rt.Waker`, which is also the context |

`Vec<u8>` storage and `[x; N]` byte arrays up to 1 KiB share 8 KiB `ArrayBuffer`s, as Node's
`Buffer` pool does, so the `buffer` behind a view holds other bytes too: read it through
`byteOffset` and `byteLength`, and copy it before transferring it.

`usize` is 32 bits, as on wasm32, so it is exact in a `number`. Arithmetic is checked as
a debug build checks it, so translated code panics where native code would: it never
wraps silently. A panic throws `rt.Panic`, with the message native Rust panics with.

Each integer type is its own type to `tsc`, named as Rust's: `rt.u8` is a `number` with a
type-only brand, so a `u16` where a `u8` goes fails the type check, while a plain
number, such as a literal, passes for any. A cast that keeps the value is `x as rt.u16`,
which costs nothing at run time.

A wider integer is a `number` up to 2^53 - 1 in magnitude, and past that an immutable
object of 32-bit limbs, `{ hi, lo }`, with `w2` and `w1` between them at 128 bits and
`hi` signed in a signed type. Each value has one form, so `x === 0` is exact; a caller
makes one past 2^53 with `rt.u64Of(18446744073709551615n)`. Its other operators and
comparisons are runtime calls, `rt.u64Add(a, b)` and `rt.u64Lt(a, b)`, and `x as u32`
is `rt.u64Low(x)`.

`format!`, `write!`, `panic!`, `assert!`, and the other macros of `format_args!`
translate, as do `Display` and `Debug` impls, derived and thiserror's included. The
runtime ports `core::fmt`, so the text matches native Rust's byte for byte. A placeholder
becomes a function writing its value, and `fmt::Arguments` an array of literal pieces
and such functions: `rt.fmtWriteFmt(f, ["code=", (f) => rt.u16Display(code, f)])`. A `&dyn
Debug` or `&dyn Display` is such a function too. Format specs take fill, alignment,
width, `#`, `0`, `x`, `X`, and `?`. `Debug` of a string escapes by the JS engine's
Unicode tables, so a character newer than its Unicode version is escaped where Rust
prints it. A caller prints a `printable` type as `to_string()` and `format!("{err:?}")`
would: `rt.fmtToString((f) => DecodeError.fmt$Display(err, f))` and
`rt.fmtFormat([(f) => DecodeError.fmt$Debug(err, f)])`, after
`import * as DecodeError from "./coding/decode.DecodeError.ts"`.

An iterator's `next()` returns its next item or `rt.DONE`, never `undefined`, so an
iterator of `Option` keeps each `None` apart from its end. std's adaptors and consumers
are runtime functions taking the iterator first, `rt.iterMap(it, f)`, lazy and
short-circuiting as std's are, and call their closures in the same order and as often. A `for` loop reads an iterator by that protocol, a slice,
array, `Vec`, or `slice.iter()` with `for ... of`, and a range written in place with a
counter. The crate's own `impl Iterator` returns `rt.DONE` from its `next` too, and joins
std's adaptors through `rt.iterFrom`. A caller calls `next()` for one item, or iterates
an `rt.Iter` with `for ... of`.

Functions and members are camelCase. Functions are monomorphized: each instance is its
own function, suffixed after a `$` by its trait, then by its type's type arguments and
its own, as in `roundtripLite$Group`, `Wrap.get$Wrap_Pair`, `Pair.fmt$Debug`, and
`DecodeError.from$From_BoundsExceeded`. The suffix is there even when no other instance
is, so reaching one renames nothing. A closure type goes by where it is defined,
`methods_closure1` for `methods::{closure#1}`, and instances whose suffixes still match,
such as of two types of one name, are numbered `$2` on in the order of their full type
arguments. Types are translated once, generically, so a function of a generic type is one
instance's, declaring the instance it takes as `self`:
`fmt$Debug_Pair(self: Wrap<Pair.Pair>, f: rt.Formatter)`. A type's module exports the
type by its own name, which a caller writes `Decoder.Decoder`, and a function whose name
and exports it as its own: `function new_` is `Decoder.new`. A `Copy` struct, tuple, or
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

A file's items follow the Rust source, whatever
order translation reaches them in, so adding a root moves nothing it does not reach.
What a macro makes sits where the macro is called. The instances of one function sit
together, sorted by their suffix (`Debug_Pair` in `fmt$Debug_Pair`) and then by their
full type arguments.

Imports name the `.ts` file, which Node, Bun, and bundlers run as is. A `tsc` build sets
`rewriteRelativeImportExtensions`, or `allowImportingTsExtensions` with `noEmit`.

## Design

Output is ranked correctness first, speed second, and bundle size third: the
consumer's final JS, after their bundler tree-shakes and minifies it, so type-only
constructs such as brand casts and annotations are free. Readable output is not a goal.
It stays `.ts`, so `tsc` keeps checking the internals, and minifying is the consumer's
bundler's job. Types must be solid at the API boundary: the `roots` items and
`printable` types use Rust's branded types (`rt.u64`), while internals cast as freely as
they need.

- 64- and 128-bit integers are numbers while they are safe integers, which most values
  are: lengths, IDs, sequence numbers, and timestamps. `just ts bench` measured it
  against `bigint`, against limbs for every value, and against unboxed halves, two
  numbers in locals, fields, and arguments. It ties unboxed halves on the codec and
  beats `bigint` there in Node, Bun, and Chrome: varint encode 13-65% faster, frame
  decode 8-32%, and message decode 22-31%, though Chrome's 8-byte varint decode is
  25-45% slower. A loop of sums and comparisons past 2^32 measured 2-6x faster. Limbs
  for every value allocate an object each, which costs what a `bigint` does in V8, so
  they lost decode; unboxed halves would lower every value to two. Past 2^53 an
  operation allocates limbs: a product past 2^53 divided back down is 1.5-1.9x slower
  than `bigint`, and 128-bit arithmetic there goes through `bigint` inside the runtime.
  The runtime adds 6 KB to the codec's bundle, 2 KB gzipped. Brands tell `u64` from
  `i64` and `u32` from `usize` at no cost, since TypeScript erases them; a required
  brand would cast every literal, so a plain number passes for any type.
- A struct is a plain object, and each function of a type, trait impls included, is its
  own function in the type's module, so a caller's bundler drops every one it never
  calls: no bundler drops a class's unused methods or an object's unused properties. A struct that
  may drop, or holds a type parameter that may, keeps a class for its `[Symbol.dispose]`.
  `just ts bench` measured it against classes everywhere and against a class facade on
  the root types over free functions: all three within noise in Node, Bun, and Chrome.
  A caller of moq-net's translation that decodes a group header bundles to 26.5 KB
  minified and 8.3 KB gzipped, against 32.3 and 9.5 KB as classes; one that encodes
  one, to 22.5 and 7.5 KB against 31.6 and 9.5 KB, each with the whole runtime, which
  did not shake then. The facade was the largest,
  since its methods keep every function they call. RxJS, Zod Mini, Valibot, Effect,
  and date-fns made the same move for the same reason.
- A type's functions are a module of their own, which callers import whole and call as
  `Decoder.slice(dec, 4)`, as Effect's `Option.map` reads: Bun, esbuild, and Rollup drop
  every function of an `import * as` the caller never calls, and the bundles measured
  the same as flat names (`decoderSlice`) within 30 bytes. A barrel that re-exports a
  type's module (`export * as Decoder`) still shakes in Bun and Rollup but not in
  esbuild ([evanw/esbuild#1420](https://github.com/evanw/esbuild/issues/1420)), which
  keeps the whole module: 10.2 KB gzipped against 8.3 for the decoding caller.
- The runtime is free functions too, flat exports of `rt.ts` named for what they act on
  (`rt.u64Add`, `rt.iterMap`, `rt.fmtPad`), so a bundler drops every one translated code
  never calls. `rt.ts` re-exports its parts with `export *`, since esbuild keeps a whole
  `export * as` namespace. A method stays only where a call dispatches on its receiver:
  an iterator's `next`, a future's `poll`, a waker's, and `[Symbol.dispose]`. Each
  integer type's functions pass their bounds or width to shared helpers, with a wide
  type's number fast path inline in its own: through the shared helper alone, V8 took
  up to 1.7x as long on frame encode. A caller of moq-net's translation that decodes a
  group header bundles to 6.9 KB minified and 2.2 KB gzipped, against 21.6 and 6.9 KB
  with the runtime's namespaces and methods; one that reads a varint, to 4.7 and 1.5 KB
  against 19.4 and 6.2. `just ts bench` measured it within noise of them in Node, Bun,
  and Chrome: 1.01, 0.97, and 1.00 times their time at the geometric mean, where the
  A/A route measured 1.00, 0.97, and 0.99. The whole runtime, every export kept, is 52
  KB minified against 36.
- A generic type is one interface, class, or union, the type TS callers name, and a
  function of it is one instance's, typed by its `self`. A type per instance
  (`Wrap$Pair`) would rename the type for callers, and TS generics with trait
  dictionaries would undo monomorphization.
- `Result` is a plain `{ ok, value } | { ok, error }` object rather than an exception, so
  `?` and `match` keep Rust's control flow exactly. Making no `Ok` objects at all measured
  no faster on `just ts bench`, in V8 or JavaScriptCore.
- Iterators are the runtime's lazy classes rather than generators or ES iterator
  helpers: std's adaptors have no THIR to lower, ES helpers lack most of them
  (`rev`, `zip`, `peekable`), and a `next` returning an item or `rt.DONE` allocates
  nothing per item. `just ts bench` measures chains of `map`, `filter`, `any`, `skip`,
  and `take` 1.4-8x faster than the same ES helpers in Node, Bun, and Chrome.
- Bodies lower from MIR ([moving](/quest/m2/ts-mir.md)): drops are where drop
  elaboration put them, exact by construction, with no `finally` per scope or
  `undefined` left by a move, and a panic runs the cleanup blocks `-C panic=unwind`
  MIR has, from a `catch`. Async bodies are the coroutine transform's state machines,
  a class each. The MIR spike ([#36](https://github.com/kixelated/rust2/pull/36)) ported
  the whole lowering and measured it against THIR's: every fixture's differential test
  passes either way, but MIR closes about 1,100 of the 1,400 drop refusals in moq-net's
  census, runs an async handoff 1.4-5.5x faster at depths 1 to 16 in Node, Bun, and
  Chrome, and runs the codec 2-12% slower than THIR, for 9% more gzip on the codec's
  bundle and 35% on the async fixture's. Blocks become nested blocks, loops, and `if`s by Ramsey's
  "Beyond Relooper", except a coroutine's resume function, which enters loops in the
  middle and becomes a loop over a `switch` on the next block. Temporaries assigned
  once and read once fold back into the expression that reads them unless that
  would reorder an effect.

## Never

Outside the subset for good: `unsafe` and raw pointers, and trait objects other than
`&dyn Debug` and `&dyn Display`. A boundary that needs one, such as a boxed future, will
bind to hand-written TS through config ([model](/quest/m2/ts-model.md)).

## Not yet

These fail translation: floats, `char`, statics, async blocks that borrow a number
mutably and async closures, values that drop as temporaries not moved whole, in a
closure's captures, in an iterator, in an enum, tuple, or generic `Drop` type, passed to
a std function that may discard them (`unwrap_or`), borrowed in an `Arc` or `Mutex`, or
moved ahead of an `.await` in the same call or assignment ([Drop](/quest/m2/ts-drop.md)), iterators of std's collections and the adaptors without
a mapping in `src/lower/iter.rs` (such as `cycle`, `unzip`, and `partition`), an own
`impl Iterator` with `next_back`, `len`, or another provided method of its own, `while
let` or `match` on the `next` of an iterator of `Option`, or-patterns that bind, other
format specs (a precision, `+`, `{:b}`, `{:x?}`, and widths from arguments), writing to a
`String` with `write!`, a `move` closure copying a mutable variable, `mem::swap`,
`replace`, or `take` of a reference's whole pointee, assigning an `Arc`, a waker, a
`MutexGuard`, a pinned reference, or a future through a reference, and std functions
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
  and writes its errors' `Display` and `Debug`. It is also translated without
  `bench::run` and then with it as the first root, which must keep every line in order.
  Bun bundles a caller of `Decoder.u8` and `Decoder.new` through `import * as Decoder`,
  which must keep only those and what they call.
- `lang` covers every integer width's arithmetic and casts, control flow, patterns,
  `Copy` aggregates, formatting, methods on generic types, and iterators, with closures
  that log what each adaptor reads. Its differential test runs through `caller.ts`, a
  hand-written caller that also prints a `printable` error nothing translated prints,
  and adds every 64- and 128-bit case over each pair of values next to a power of two,
  where the runtime switches between numbers and limbs, and its limbs carry and borrow.
  `brands.ts`, another caller, mixes integer types where `tsc` must refuse them.
- `task` trims moq's kio to a channel of `Arc`ed state, waiters, and wakers, under async
  functions. Its caller is hand-written TS (`tests/task.ts`): a FIFO executor that runs a
  script of spawns, sends, polls, and drops, and logs every poll, wake, and drop, as the
  fixture's native executor does. The logs must match byte for byte. `tests/unwind.ts`
  also runs the scripts natively on the host (`examples/unwind.rs`), where a panic
  unwinds, catching and logging each step's panic and going on, so what a panic drops
  must match too.
- `reject` holds constructs outside the subset; its expected errors are `expected.txt`,
  those from MIR `expected-mir.txt`, and its expected census is `census.md`.

The codec, lang, and task fixtures also run their differential tests from MIR
(`codec_mir`, `lang_mir`, and `task_mir`), without snapshots.

`tests/spawn.ts` checks that `rt.spawn` yields to a timer after its 128-poll budget, and
rejects when a future panics as it drops.

## Bench

```bash
nix develop .#bench --command just ts bench  # report to target/bench/report.md
```

The codec fixture's `bench::run` holds the cases moq's own TypeScript benchmarks time
(`js/net/bench/varint.ts` and `lite-varint.ts`), at the pinned moq revision, the same
frame headers decoded as they arrive a few bytes at a time, which prices a decode that
comes up `Short` and retries, and iterator chains over bytes. Each runs four ways:
natively, as wasm32 at moq-wasm's size-optimized profile, hand-written (moq's `js/net` in `bench/moq.ts`, and ES iterator
helpers in `bench/iter.ts`), and translated; every route's output must match
native Rust's first. The JS routes run in Node, Bun, and Chrome, with the WASM route's
copies across the boundary timed apart from its compute, and an A/A copy of the
translation to show the noise. Each round runs the JS routes and workloads in a new
random order, so no route keeps a slot that runs it faster or slower than identical code
in another; the report records the seed. It also records the host and engine versions,
and each route's bundle size raw and gzipped. It sets no threshold; the nightly workflow
runs it and posts the report to its summary. Chrome's page reports each step it starts,
and one that runs 30 s fails the run, naming it.

The task fixture's `bench::run` times an async handoff: a value sent to a task parked on
its channel, read through 1, 4, or 16 async functions. Its translation runs polled at
once, as natively, and on `rt.spawn`; its hand-written route is the same handoff with
promises, as moq's TypeScript writes it.

An A/B run times another translator's output as one more route: run `just ts bench` in
the other checkout, then pass its output directory, absolute or relative to `ts/`.

```bash
nix develop .#bench --command just ts bench --route main="$HOME/rust2-main/ts/target/bench"
```

The route runs both fixtures' translations from that directory's `translated/` and
`task/`, so a hand-edited translation can be timed by copying the directory and editing it.
Read its gap from `translated` against A/A's, the noise of the same run. No route holds a
fixed slot, so a gap past that noise is the change, and no reversed run is needed.

The bench shell adds Chromium and the wasm-bindgen that builds moq-wasm's package.
