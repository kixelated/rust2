# rust2-ts

A rustc driver that translates a subset of Rust to readable TypeScript with source
maps. No WASM: the output is plain ES modules that Node, Bun, and bundlers run as is.

```bash
rust2-ts --config ts.toml --out dir [--manifest-path Cargo.toml] [-- cargo args]
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
# Macros whose invocations are left out, such as logging. Each must evaluate to `()`.
ignore-macros = ["tracing::trace"]
```

moq consumes it as the `ts` package of this repository's flake. In this repository,
`just ts run ...` runs it from source on the pinned nightly.

## Output

Each Rust module with translated items becomes a file (`lib.ts`, `coding/varint.ts`),
with a `.ts.map` source map into the Rust it came from. `rt.ts` beside them is the
runtime: what std does that JS does not.

| Rust | TypeScript |
| --- | --- |
| `bool`, `str`, `String` | `boolean`, `string` |
| `u8`-`u32`, `i8`-`i32`, `usize`, `isize` | `number`, checked: overflow panics |
| `u64`, `i64`, `u128`, `i128` | `bigint`, checked the same way |
| `struct` | a class; methods take `this`, trait methods included |
| `enum` without data | a union of strings: `"Lite01" \| "Lite02"` |
| `enum` | a union of objects tagged by variant, with a companion object of functions |
| `Option<T>` | `T \| undefined`; an `Option` of `()` or an `Option` is refused |
| `Result<T, E>` | `rt.Result<T, E>`, and `?` returns early as written |
| tuple, `[T; N]`, `&[T]`, `Vec<T>` | an array, `Uint8Array` for bytes, `rt.ByteVec` for `Vec<u8>` |
| `&T`, `&mut T`, `Box<T>` | the value: a `&mut` must point at an object JS can change in place |
| closure | an arrow function |
| constant | its value, inlined as Rust inlines it |

`usize` is 32 bits, as on wasm32, so it is exact in a `number`. Arithmetic is checked as
a debug build checks it, so translated code panics where native code would: it never
wraps silently. A panic throws `rt.Panic`.

Functions are monomorphized: each instance is its own function, named with a `$` suffix
when two share a name. Types are translated once, generically. A `Copy` struct, tuple, or
array is copied where Rust copies it, since JS would share the object.

## Not yet

These fail translation: floats, `char`, trait objects, raw pointers, statics, `async`,
types that implement `Drop`, iterator adaptors (`for` over ranges, slices, arrays, and
`Vec` works), or-patterns that bind, panic messages that format values, a `move` closure
copying a mutable variable, and std functions without a mapping in `src/lower/library.rs`.

## Tests

```bash
just ts test                  # translate each fixture, compare, type-check, and diff
UPDATE_EXPECT=1 just ts test  # rewrite the snapshots in fixtures/*/expected
just ts moq                   # translate moq-net's codec at a pinned revision
```

Each fixture under `fixtures/` is translated and compared with its `expected/` snapshot,
then type-checked with `tsc`. A fixture with a `harness::run(&[u8]) -> Vec<u8>` is also
built natively for wasm32 and run beside its translation on the same inputs by
`tests/differential.ts`, which fails on any difference in output or panics.

- `codec` vendors moq-net's sync codec, which a harness outside the crate could not reach.
- `lang` covers every integer width's arithmetic and casts, control flow, patterns, and
  `Copy` aggregates.
- `reject` holds constructs outside the subset; its expected errors are `expected.txt`.
