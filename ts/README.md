# rust2-ts

A rustc driver that translates a subset of Rust to TypeScript with source maps, for the browser. No WASM: the output is plain ES modules that bundlers take as is. A native runtime calls Rust through [rust2-ffi](/ffi/README.md) instead.

```bash
rust2-ts --config ts.toml (--out dir | --census file.md) [--manifest-path Cargo.toml] [-- cargo args]
```

It runs `cargo check` on the package with itself as `RUSTC_WRAPPER`, for
`wasm32-unknown-unknown`, with std built from source for it with every function's MIR, which needs the toolchain's `rust-src`. Every crate compiles as rustc
would; the configured library is also translated, after type and borrow checking, starting from its roots and following
what they reach. Every crate it depends on but std's translates with it as one program,
as far as the roots reach: each compiles with every function's MIR, and its types and
calls translate as the package's own do. They compile with `--cfg rust2_ts`, so a crate
can give translation a safe twin of its `unsafe`. Their `unsafe` translates as the
package's does, and what translation can't model is refused where it is, unless a binding
stands in for the item holding it. Only the writes of the dependencies' functions translation reaches decide what it
copies, so it runs again when it reaches one it didn't read. A construct outside the subset is a compile error with its span, and
then nothing is written. A type without a JS value is refused where a value of it is made
or received, such as a parameter, a field, or a call, rather than again at each variable
holding one; any variable or temporary that holds a type whose JS value is ambiguous, such
as `Option<Option<()>>`, is refused at its own span.

```toml
# ts.toml, beside the package's Cargo.toml unless --manifest-path says otherwise.
package = "moq-net"
# Paths from the crate root: a function, or a type with its non-generic methods and its
# impls of the crate's own traits, blanket ones included.
roots = ["coding::Decoder", "lite::Group"]
# Types whose `Display` and `Debug` impls come along, whichever they have, so callers can
# print them. Another crate's trait otherwise comes along only when something calls it.
printable = ["coding::DecodeError"]

# Hand-written TypeScript that stands in for an item, by its path from the crate root or
# else from a crate it depends on: a function, or a type and its functions.
[bind]
"model::flate::Encoder" = "bind/encoder.ts"
"util::TaskSet" = { file = "bind/tasks.ts", drop = true }
```

moq consumes it as the `ts` package of this repository's flake. In this repository,
`just ts run ...` runs it from source on the pinned nightly.

## Census

`--census file.md` translates nothing. It writes a Markdown report of every construct
translation refuses, with counts and spans, ranked by construct, by the type or module
they name, and by each type or path. It starts from the config's roots, as translation
does. A config that names none makes every item in the library a root, except derived
impls, which come along only when called or printable, and lowers a generic function
that no instance reaches over its own type parameters, refusing nothing that hinges on
what they are, since each instance decides that. A census succeeds whatever it finds;
translation still fails on any refusal.

`just ts census` runs it at the pinned moq revision, into `target/census`, on moq-net
whole (`moq/net.toml`), on its model's public types with their boundaries bound
(`moq/model.toml`), and on kio, and the nightly job
publishes their tables.

## Output

Each Rust module with translated items becomes a file (`lib.ts`, `coding/varint.ts`),
and each type a module of its own beside it, holding the type and its functions
(`coding/decode.Decoder.ts`), each with a `.ts.map` source map into the Rust it came
from. `rt.ts` beside them is the
runtime: what std does that JS does not, with iterators in `rt/iter.ts`, `str`'s searches
in `rt/str.ts`, collections in `rt/collections.ts`, ownership, atomics, and async in
`rt/task.ts`, `f64`'s methods in `rt/float.ts`, and `Duration` in `rt/time.ts`. What translates from std's own MIR,
`Option`'s, `Result`'s, and `Poll`'s methods, goes in a module under its crate's name, as
`core/option.ts` holds `map$Option_u64_u64_rescale_closure0`. A dependency's modules go
under its crate's name too, `kio.ts` and `kio/lock.ts`, with the instances of its generic
functions that the package's types make, so a module at the package's root named for a
crate translation reaches is refused, and so are two versions of one crate it reaches.

| Rust | TypeScript |
| --- | --- |
| `bool`, `str`, `String`, `Cow<str>` | `boolean`, `string` |
| `char` | `rt.char`: a `number` branded so, its Unicode scalar value |
| `u8`-`u32`, `i8`-`i32`, `usize`, `isize` | `rt.u8` and so on: a `number` branded with its type, checked: overflow panics |
| `u64`, `i64`, `u128`, `i128` | `rt.u64` and so on: a branded `number` while a safe integer, 32-bit limbs past 2^53, checked the same way |
| `f64` | `rt.f64`: a `number` branded so, which is the same IEEE 754 double |
| `struct` | a plain object, typed by an interface; a class if it may drop |
| `enum` without data | a union of strings: `"Lite01" \| "Lite02"`, or `never` without variants |
| `enum` | a union of objects tagged by variant |
| `Option<T>` | `T \| undefined`; `T \| rt.None` where `T` can already be `undefined`, as `()` and `Option` can, so `Option<()>` and `Option<Option<u64>>` keep `None` apart from each `Some` |
| `Result<T, E>` | `rt.Result<T, E>`: `{ ok: true, value }` or `{ ok: false, error }`; `rt.OK` or `rt.ERR` where the payload is `undefined`, as in `Ok(())` |
| `ControlFlow<B, C>` | `rt.Result<C, B>`, `Ok` to continue; what `?` branches on is the value it was applied to, but for a `Poll`'s `Pending` and `Ready(None)`, which continue as `Ok` of themselves |
| tuple, `[T; N]`, `&[T]`, `Vec<T>` | an array; for bytes, `Uint8Array` for `[u8; N]`, `rt.ByteSlice` for `&[u8]` and `&[u8; N]`, and `rt.ByteVec` for `Vec<u8>` |
| `&T`, `&mut T`, `Box<T>`, `Reverse<T>`, `NonZero<T>` | the value; a `&mut` to a value JS copies, such as a number, an `Option`, or a `String`, is an `rt.Place<T>` |
| `*const T`, `*mut T`, `NonNull<T>` | what a `&mut T` is: the object, or an `rt.Place<T>` to a value JS copies and to a library type such as a `Vec` or an `Arc`; `rt.NULL` for null and `rt.DANGLING` for dangling. A write of a whole object copies into it one object deep, as an assignment through a `&mut` does, so a pointer to a part kept as an object, such as a field of a tuple or struct type, is refused wherever the crate may put another object there; a read copies what the crate writes into in place |
| `MaybeUninit<T>` | `T`: `undefined` until written, or an object whose parts kept as objects are there, holding `undefined` |
| `union` | an object of slots named by offset, `at0` and `at8`, each holding the field part kept there; fields must overlap in the same type, or in a non-null pointer and an `Option` of it |
| an iterator | `rt.Iter<Item>`, whose adaptors are runtime functions: `rt.iterMap(rt.iterSlice(v), f)` |
| `a..b`, `a..=b` | `rt.Range<T>`, `rt.RangeInclusive<T>`, which are iterators too |
| `a..`, `..b`, `..=b`, `Bound<T>` | `{ start }`, `{ end }`, `{ end, inclusive: true }`; `rt.RangeBound<T>`, a union tagged by variant |
| `HashMap`, `HashSet`, `BTreeMap`, `BTreeSet`, `VecDeque` | `rt.HashMap<K, V>` and so on, each a plain object whose functions are the runtime's: `rt.hashGet(map, key)` |
| `Ordering`, `DefaultHasher` | `rt.Ordering`, which is `-1`, `0`, or `1`; `rt.Hasher` |
| `Duration` | `rt.Duration`: `{ secs, nanos }`, std's whole seconds as an `rt.u64` and the nanoseconds past them, never changed once made |
| closure | an arrow function, with `[Symbol.dispose]` if it owns a value that drops |
| `&dyn Trait`, `&mut dyn Trait`, `Box<dyn Trait>`, `Arc<dyn Trait>` | an object with the trait's methods, its supertraits' included: `shape.area$Shape()`; behind an `Arc`, an `rt.ArcDyn` sharing the count of the `Arc` it was made from |
| `dyn Any` | an instance of the class its value's type was wrapped in, which a downcast checks for: `a instanceof Any.Square ? a.v : undefined` |
| `dyn Fn`, `dyn FnMut`, `dyn FnOnce` | the closure or function it was made of: `(a0: rt.u8) => rt.u32` |
| constant | its value, inlined as Rust inlines it |
| a type that drops | a class with `[Symbol.dispose]`: its `Drop` impl, then its fields |
| a type's function | a function of the type's module, taking `self` first: `Decoder.slice(dec, 4)` after `import * as Decoder` |
| `Arc<T>`, `Rc<T>`, `Weak<T>`, `Mutex<T>` | `rt.Arc` for either, with its strong and weak counts, and `new_cyclic`'s `Weak` upgrading only once its value is made; `rt.Weak`, counted in its `rt.Arc` until it drops, and of no `Arc` once made by `Weak::new`; `rt.Mutex`, its own guard, which a lock gives as `Ok` and the guard's drop unlocks: locking it again while held panics and `try_lock` gives `WouldBlock`, as wasm32's do, and a panic aborts, so nothing poisons it and `thread::panicking` is `false` |
| `PoisonError<T>`, `TryLockError<T>` | the `T` it holds; `rt.TryLockError<T>`, tagged as `Bound` is |
| `OnceCell<T>`, `OnceLock<T>` | `rt.OnceCell` and `rt.OnceLock`, holding the `Option<T>` in `v`; a `OnceLock` initialized from within its own initialization panics, as wasm32's does |
| `AtomicU32` and the others of `bool` and integers | `rt.Atomic<T>`, the `rt.Place` of its value; an `atomic::Ordering` is its discriminant, and one an operation can't take panics as std's does |
| `async fn` | a function returning `rt.Future<T>`: a class of its own, its coroutine's state machine |
| `Poll<T>`, `Waker`, `Context` | `T \| typeof rt.Pending`; `T \| rt.OuterPending` where `T` can already be pending, as `Poll<Poll<u8>>` can, so `Ready(Pending)` keeps apart from `Pending`; `rt.Waker`, which is also the context |
| `pin!(x)`, `poll_fn(f)`, `future::pending()` | `x`; `rt.PollFn`, which drops `f` with itself; `rt.NEVER` |
| `Box::pin(x)`, `dyn Future<Output = T>` | `x`; `rt.Future<T>`: an `async` body or std's future as it is, and another future as an `rt.DynFuture` over its `poll`, which drops it with itself unless borrowed |
| `bytes::Bytes` | `rt.ByteSlice`, as `&[u8]` is: its clones and slices share bytes that never change |
| `bytes::BytesMut` | `rt.BytesMut`: a window of storage, which what it splits off and freezes share without overlapping |
| `rand::rngs::ThreadRng` | `rt.ThreadRng`, which holds no state |

`Vec<u8>` storage and `[x; N]` byte arrays up to 1 KiB share 8 KiB `ArrayBuffer`s, as Node's
`Buffer` pool does, so the `buffer` behind a view holds other bytes too: read it through
`byteOffset` and `byteLength`, and copy it before transferring it.

An `rt.ByteSlice` is a range of a `Uint8Array`: its `data`, from `start`, for `length`
bytes. A caller passes bytes as `rt.byteSliceFrom(array)` and reads a slice it gets as
`rt.byteSliceView(slice)`, a view of it.

`usize` is 32 bits, as on wasm32, so it is exact in a `number`. Arithmetic is checked as
a debug build checks it, so translated code panics where native code would: it never
wraps silently. A panic throws `rt.Panic`, with the message native Rust panics with, and
aborts as wasm32's does: it runs no cleanup, so what the translation held is undefined
after one, as a wasm trap leaves it, and a caller that catches `rt.Panic` must not use it
again.

Each integer type is its own type to `tsc`, named as Rust's: `rt.u8` is a `number` with a
type-only brand, so a `u16` where a `u8` goes fails the type check, while a plain
number, such as a literal, passes for any. A cast that keeps the value is `x as rt.u16`,
which costs nothing at run time.

A wider integer is a `number` up to 2^53 - 1 in magnitude, and past that an immutable
object of 32-bit limbs, `{ hi, lo }`, with `w2` and `w1` between them at 128 bits and
`hi` signed in a signed type. Each value has one form, so `x === 0` is exact; a caller
makes one past 2^53 with `rt.u64Of(18446744073709551615n)`. Its other operators and
comparisons are runtime calls, `rt.u64Add(a, b)` and `rt.u64Lt(a, b)`, and `x as u32`
is `rt.u64Low(x)`. `((hi as u64) << 32) | lo as u64` of halves of at most 32 bits is
one call, `rt.u64FromHalves(hi, lo)`.

An `f64` is a JS number, so its operators are JS's, which round as Rust's do. Where
JS differs from std, the runtime has std's: `round` halves away from zero, `max` and
`min` ignore a NaN, and a cast to an integer truncates, saturates at the type's bounds,
and sends NaN to 0. A 64- or 128-bit integer `as f64` rounds once, to the nearest. The
functions std leaves to the platform's libm, such as `sin`, `exp`, `ln`, `powf`, and
`hypot`, are `Math`'s, which the tests find within 2 ULPs of native Rust's in Node;
`mul_add`, which rounds once, is refused. `Duration`'s `f64` methods run std's integer
algorithm, to the nearest nanosecond with its errors and panics. JS doesn't say which
NaN an operation makes, so a NaN's payload and sign are the engine's: Node and Chrome
keep them through `from_bits` and `to_bits`, but V8 quiets a signaling NaN stored in an
array, as a `Vec<f64>` holds it.

`format!`, `write!`, `panic!`, `assert!`, and the other macros of `format_args!`
translate, as do `Display` and `Debug` impls, derived and thiserror's included. The
runtime ports `core::fmt`, so the text matches native Rust's byte for byte. A placeholder
becomes a function writing its value, and `fmt::Arguments` an array of literal pieces
and such functions: `rt.fmtWriteFmt(f, ["code=", (f) => rt.u16Display(code, f)])`. A `&dyn
Debug` or `&dyn Display` is such a function too. Format specs take every option but `{:p}`:
fill, alignment, `+`, `#`, `0`, `x?` and `X?`, a width and a precision, each also from an
argument, and `x`, `X`, `o`, `b`, `e`, and `E` for integers. `Debug` of a string or `char` escapes by the JS
engine's Unicode tables, so a character newer than its Unicode version is escaped where
Rust prints it. A caller prints a `printable` type as `to_string()` and `format!("{err:?}")`
would: `rt.fmtToString((f) => DecodeError.fmt$Display(err, f))` and
`rt.fmtFormat([(f) => DecodeError.fmt$Debug(err, f)])`, after
`import * as DecodeError from "./coding/decode.DecodeError.ts"`.

An iterator's `next()` returns its next item or `rt.DONE`, never `undefined`, so an
iterator of `Option` keeps each `None` apart from its end. std's adaptors and consumers
are runtime functions taking the iterator first, `rt.iterMap(it, f)`, lazy and
short-circuiting as std's are, and call their closures, and read what they wrap, in
the same order and as often, a skip by `nth` included. `max`, `min`, and their `_by_key`
forms test each next item's key for `<` the best's, as std's do, and a B-tree's iterators
read their ends.
An iterator over a slice, array, or `Vec`, or a small range, that only its `next` and
moves read steps in place, as two variables: the array and an index into it, or the
range's bounds, with no `rt.Iter` made. The crate's own `impl Iterator` returns `rt.DONE` from its `next` too, and joins
std's adaptors through `rt.iterFrom`. A caller calls `next()` for one item, or iterates
an `rt.Iter` with `for ... of`.

A `str` is a JS string, which is UTF-16: `chars` steps by code point, and a search by a
`char`, a `&str`, a predicate, or an array or slice of chars, as in `rt.strSplit(s, "/")`, takes the text or the
closure, which it calls on each char it reads, in the order std reads them. An offset it
returns, from `rfind` or `match_indices`, counts UTF-8 bytes, as Rust's does, and so does
one `&s[a..b]` takes, which reads the string up to it and panics as std does inside a
char. A JS string can't change in place, so `String::push` assigns a longer one to the
local or field the `&mut` borrows. A `Cow<str>` is its text, borrowed or owned, so
`to_mut` borrows the `Cow`'s own place, and code that tells its variants apart, a `match`
on them or `Cow::is_borrowed`, is refused.

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
and exports it as its own: `function new_` is `Decoder.new`. A `Copy` struct, enum, tuple,
or array, or an `Option`, `Result`, or `Poll` holding one, is copied where Rust copies
it, since JS would share the object: by a function of `lib.ts` made for its type, such
as `copy$Point`, which copies what it holds by value and shares what it holds by
reference. Only a type the crate writes into in place is copied, or one holding such a
type by value: a value of any other type never changes once made, so it is shared. A
write counts through a field, an element, a reference, or a box, and a mutable reference
a std function's parameters take, where the crate calls it or takes it as a value, or
one kept other than in a local, counts as a write into what it points at. A write over a type parameter counts for every type its bounds allow. A caller must
not change in place what it passes to or gets from the translation either.

A `&mut` to a value JS copies, which no JS value can point at, such as a number, an
`Option`, an enum, a tuple, a `Box`, or a `String`, is an `rt.Place` whose `v` reads and
writes where it points. Where it is only read and written through, in the function that
borrows it, it is the place it borrows, so a body that passes none on makes none. Passed
on, returned, kept in a field, or captured by a value, it is
`rt.place(() => x, (value) => x = value)` over that place, the objects and index it goes
through held as they were when it was borrowed; a reborrow of one is the same
`rt.Place`. A caller passes one as `{ v }`, or as any object whose `v` it reads and writes.
std's mappings take a `&mut` parameter as what it points at.

A collection keeps Rust's key semantics. A hash map or set keeps a key that is a JS
number, string, or boolean in a JS `Map`, which compares those as Rust's `Eq` does, and
finds any other key, a 64-bit one past 2^53 included, through its translated `Hash` and
`Eq`; a B-tree orders every key by its translated `Ord`. std's impls are the runtime's: an
`Option` orders `None` first, and an `Arc` compares, orders, and hashes as what it holds,
so a map of `Arc<str>` finds a `&str`. A `&mut` to a value a map holds that JS copies,
such as a number, from `get_mut` or the entry API, is an `rt.Place` whose `v` reads and
writes the map's slot. A hash map iterates in an order
of the runtime's, mostly insertion order, which std leaves unspecified, as it does
`DefaultHasher`'s values, which are the runtime's own too. So is how often the runtime
calls a key's `Hash`, `Eq`, or `Ord`.
Its iterators are `rt.Iter`s, so std's adaptors run over them.

A value drops where Rust drops it, if dropping it does something JS can see: a `Drop`
impl, an `Arc`'s count, or a waker. Bodies lower from MIR, so each drop is where drop
elaboration put it. A panic drops nothing. A value without `[Symbol.dispose]` of its own, such as
an enum or a tuple, drops through glue lowered from rustc's `drop_in_place` shim, a
function of its type's own (`drop$Held`). `bool`'s, `Option`'s, `Result`'s, and `Poll`'s
methods lower from core's MIR, so they drop what core's do, where core's do. The runtime
functions for std's collections, `Arc`, and the std functions that may discard an operand
(`OnceCell::get_or_init`, a map's `insert`) take what drops each value they may drop, and
drop it where std's do. So do iterators: one that owns its items, as a `Vec`'s `into_iter` or a
map's `drain` does, drops those it still holds by `[Symbol.dispose]`, an adaptor drops
what it skips or rejects, and a consumer that takes an iterator by value drops it:
through each `rt.Iter`'s own `fold` where std's folds, which drops each adaptor's closure
as its fold ends, before what it wraps, as std's does.

A trait object of the crate's own trait, or a dependency's, behind `&`, `&mut`, `Box`, or `Arc`, is an object with
a method for each function a call can dispatch to, its supertraits' included, named for
its trait as an impl's function is and taking what follows `self`: `shape.area$Shape()`.
The trait's module exports its type, `Shape.Shape` from `dyns.Shape.ts`, which a TS
caller can implement. A coercion wraps the value in a class of its type's, `new
Shape.Square(sq)`, whose methods call that type's impl; one from a `&mut` to a value JS
copies holds its `rt.Place`, so `&mut self` writes where the value is. An upcast is the
same object. A `Box<dyn Trait>` drops what it holds through its `[Symbol.dispose]`, which
a TS caller's object may leave out. An `Arc<T>` unsized to an `Arc<dyn Trait>`, or to an
`Arc<dyn Future>`, is an `rt.ArcDyn`: the object a box of its value would be, and the
`Arc<T>` it was made from, whose one count both sides' clones and drops change, and the
last of which drops the value. `Arc::ptr_eq` compares the `Arc<T>`s.

A `dyn Any` is such an object with no methods, whose class `is`, `downcast_ref`,
`downcast_mut`, and `Box`'s and `Arc`'s `downcast` check for: `a instanceof Any.Square ?
a.v : undefined`. A `&mut` one of a value JS copies is the class holding its `rt.Place`,
or the class itself behind a `Box`, as a `&mut dyn Trait`'s is, and `Arc::downcast` gives
back the `Arc<T>` the object was made from.

A `dyn Fn`, `dyn FnMut`, or `dyn FnOnce` is the closure or function it was made of, which
a call calls as it does the closure, and which a box drops as it does the closure. One
that `Fn` or `FnMut` calls, boxed as a `dyn FnOnce`, is `rt.once` of it, which drops it
after the call, as an `FnOnce` closure's body drops what it owns, and a box of a
reference to one is `rt.lent` of it, which never drops it. A trait object of std's
or a mapped crate's trait other than `&dyn Debug`, `&dyn Display`, `dyn Future`,
`dyn Any`, and the function traits, of a generic
trait or one with an associated type, of a byte array, or behind any other pointer, such
as a `Weak` or a struct's last field, is refused, and so are an upcast to `dyn Any` and
`Any::type_id`.

An `async fn` returns a future without running anything: an instance of a class of its
own (`sumFuture`), the state machine MIR's coroutine transform makes. Its fields are the
arguments and the locals held across an `.await`, and its `poll` resumes at the state it
stopped in. A poll after the future returned panics, and dropping it cancels it,
dropping what its state holds. `rt.spawn(future)` runs one as a promise, polling woken tasks in a batch from a
microtask and yielding to a macrotask after 128 polls. `Waker::from(arc)`, of an `Arc` of
the crate's `Wake` impl, is an `rt.ArcWaker`, which holds a count of the `Arc` and wakes
through the impl, and `Waker::noop()` is `rt.NOOP_WAKER`.

A file's items follow the Rust source, whatever
order translation reaches them in, so adding a root moves nothing it does not reach.
What a macro makes sits where the macro is called. The instances of one function sit
together, sorted by their suffix (`Debug_Pair` in `fmt$Debug_Pair`) and then by their
full type arguments.

Imports name the `.ts` file, which Node, Bun, and bundlers run as is. A `tsc` build sets
`rewriteRelativeImportExtensions`, or `allowImportingTsExtensions` with `noEmit`.

## Well-known crates

rust2-ts maps a few crates itself, as it maps std, so a consumer binds and configures none
of them:

- `bytes::Bytes` is an `rt.ByteSlice`. Its constructors, `len`, `is_empty`, `slice`,
  `Clone`, `Deref`, `AsRef`, `PartialEq` with bytes, `Default`, `Debug`, and `From`
  between it and `Vec<u8>`, `String`, `&'static [u8]`, and `&'static str` map to
  `rt/bytes.ts`, panicking as `bytes` does, and so do `split_to`, `split_off`,
  `truncate`, `clear`, and `slice_ref`, whose panic names no addresses. `Bytes::from_owner`
  reads its owner's bytes once, as `bytes` does, and is refused for an owner that drops,
  which would drop with the last clone of what it makes.
- `bytes::BytesMut` is an `rt.BytesMut`: a window of storage that the handles `split_to`
  and `split` make and the `Bytes` that `freeze` makes share without overlapping, as
  `bytes`' do. Its constructors, `len`, `is_empty`, `freeze`, `split_to`, `split`,
  `truncate`, `clear`, `resize`, `reserve`, `extend_from_slice`, `unsplit`, `Clone`,
  `Deref`, `DerefMut`, `AsRef`, `AsMut`, `PartialEq` with bytes, `Default`, `Debug`, and
  `From` between it and `Bytes`, `Vec<u8>`, `&[u8]`, and `&str` map, panicking as `bytes`
  does. Its capacity is refused, and so are `split_off`, `try_reclaim`, `try_unsplit`,
  and `spare_capacity_mut`, which read it: `bytes`' depends on whether the other handles
  of its storage dropped, such as a `Bytes` it froze, which nothing here counts.
- `Buf` and `BufMut` of a `Bytes`, a `BytesMut`, a `&[u8]`, and a `Vec<u8>` map: the
  required methods, `has_remaining`, `copy_to_slice`, `copy_to_bytes`, each `get_` and
  `put_` of an integer or `f64`, `put_slice`, `put_bytes`, and `put`, whose panics are
  `bytes`'. `put` of the crate's own `Buf` reads it through its methods as `bytes` does.
  The rest of `bytes`, such as the provided methods of the crate's own `Buf` and the impls
  forwarding through a `&mut` or a `Box`, translates from its MIR, refused where that is.
- `rand::rng()` is an `rt.ThreadRng`, whose `random` of an integer or `bool` and
  `random_range` of an integer range map to `rt/rand.ts`. Each value comes from
  `crypto.getRandomValues`, uniform as `rand`'s are, though no seed reproduces them, as
  none reproduces `rand::rng()`'s. An empty range panics as `rand`'s does. A seeded
  generator, such as `StdRng`, is refused, since its values would have to be `rand`'s.
- `tracing`'s events, `trace!` through `error!` and `event!`, are left out where they are
  written. Translated code installs no subscriber, and without one an event evaluates none
  of its fields and does nothing, so leaving it out is exact. Installing one, as
  `tracing::subscriber::set_global_default` does, is refused as any unmapped call is.

## Bindings

A binding stands in for what the subset refuses, such as `unsafe` code or a boxed future.
Its path goes from the crate root, or else from a crate it depends on by the name the crate
gives it, as `buf::Blob` for `buf = { package = "blob" }`. A crate it reaches only through
another goes by its own name, where one crate of that name is built.
Its file, relative to `ts.toml`, is copied to the same path in the output, beside the
runtime. It imports `rt.ts` as the output's modules do, `../rt.ts` from `bind/blob.ts`,
and may import the translated modules and npm packages too. Callers import it whole, as a
type's module, `import * as Blob from "./bind/blob.ts"`.

A bound type is the file's export of its name, `Blob.Blob`, with the Rust type's type
parameters. A call of a bound function, or of a function that an impl for a bound type in
the type's own crate defines, calls the file's export. The export is named as the
translated function would be, without the type's or the function's type arguments, so one
export serves every instance and a generic one is a generic TS function. `Blob::slice` is
`Blob.slice`, `<Blob as PartialEq>::eq` is `Blob.eq$PartialEq`, and
`<Blob as From<Vec<u8>>>::from` is `Blob.from$From_Vec_u8`. A trait's provided function
that the impl doesn't override, such as `ne`, and a blanket impl's, such as `ToOwned`'s,
stay the trait's, as for any other type. A generic export sees a type argument only in the
values it takes, so a function with a bounded type parameter that no argument holds, such
as `fn zero<T: Default>() -> T`, or with a const parameter, can't be bound. `tsc` checks a
generic bound type's value at each of its instantiations translation reaches.

A type with a representation of its own, std's or a well-known crate's, can't be bound.

Translation can't see into a bound type, so a field, variant, or constructor of one is
refused, as are iterating one, keying a collection by one, and assigning a whole one
through a `&mut`. Its value is never `undefined` or `rt.Pending`, which `None` and
`Poll::Pending` are, and is an object unless the Rust type is `Copy`. A `&mut` to one
passes that object, which the binding's functions change in place. Where the type is
`Copy`, such a `&mut` is refused, since JS shares the value where Rust copies it, and a
binding never changes a `Copy` value in place. Nor does a binding change in place what it
takes by value or by `&`, or share with a clone what a `&mut` changes, and an integer it
returns is in its Rust type's range.

Once the output is written, `tsc` 7.0 or later, from `PATH`, checks each export against the
translated signature of every instance translation calls, and each bound type's value. One
returning `()` is checked against `=> undefined`, not `=> void`, which accepts any value,
so its export is typed `: undefined`. A missing or mismatched binding fails the
translation at the Rust item's span, and the output is removed. Only the binding files translation reaches are copied, so one imports
another only where translation reaches both. A binding inside the output is refused, since
a translation replaces it.

A bound type with `drop = true` drops where Rust drops it, through its
`[Symbol.dispose]`, which `tsc` checks it has. Without it, dropping one does nothing JS
sees, `tsc` checks it has no `[Symbol.dispose]`, and a type argument that drops is refused.

## Design

Output is ranked correctness first, speed second, and bundle size third: the
consumer's final JS, after their bundler tree-shakes and minifies it, so type-only
constructs such as brand casts and annotations are free. Readable output is not a goal.
Speed is a browser's: `just ts bench` runs Chrome and Firefox, and Node and Bun only as
stand-ins for V8 and JavaScriptCore, as Safari runs it; where any engine disagrees with
Chrome, Chrome decides.
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
  25-45% slower; joining a varint's halves in one call, `rt.u64FromHalves`, makes that
  decode 1.05-1.2x faster in Chrome. A loop of sums and comparisons past 2^32 measured
  2-6x faster. Limbs for every value allocate an object each, which costs what a
  `bigint` does in V8, so they lost decode; unboxed halves would lower every value to
  two. Past 2^53 an operation allocates limbs, computed on their 32-bit words: sums,
  differences, comparisons, casts, products of 64-bit values or of two numbers, and
  division by a divisor below 2^32, with the rest of 128-bit arithmetic through `bigint`
  inside the runtime. A literal past 2^53 is written out as limbs, which cost what a
  hoisted constant does. Converting timestamps between timescales as moq-net does,
  products past 2^53 divided back down, takes 0.8-1.0x the hand-written `bigint` route's
  time in Chrome in `u64`, and 1.2-1.5x in `u128`. `u128` and `i128` as
  native `bigint`s measured 1.4-1.8x slower than limbs on that in Chrome and 1.2-1.5x in
  Node, and within noise in Bun, for a bundle 80 bytes larger. The runtime adds 6 KB to
  the codec's bundle, 2 KB gzipped. Brands tell `u64` from `i64` and `u32` from `usize` at no cost, since
  TypeScript erases them; a required brand would cast every literal, so a plain number
  passes for any type.
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
- `None` is `undefined`, so a TS caller reads an `Option` with `?.` and `??`, except where
  the `Some` can already be `undefined`: there it is `rt.None`, as `Pending` is
  `rt.Pending`, chosen by the type alone, so every function agrees on it. A `rt.None`
  for every `Option` would cost callers `?.` and `??` everywhere for the few that need it.
- A `Cow<str>` is a JS string, as `str` and `String` are, rather than an object tagged by
  its variant: no `Cow<str>` in moq-net or kio tells its variants apart, and a tag would
  wrap every value and unwrap it at every read.
- A `Duration` keeps std's fields, whole seconds and nanoseconds, rather than one count
  of nanoseconds, so its overflow, `checked_*`, and ordering are std's exactly, and a
  lowering of its methods from core's MIR would read the same fields.
- `Result` is a plain `{ ok, value } | { ok, error }` object rather than an exception, so
  `?` and `match` keep Rust's control flow exactly. One whose payload is `undefined`, such
  as `Ok(())`, is one frozen object every evaluation shares, since nothing writes into a
  result in place and engines don't remove one returned through a call they don't inline:
  `just ts bench` measured the codec's encode workloads at 0.90 times their time without
  it in Chrome, 0.75 in Firefox, and 0.75 in Bun at the geometric mean, with A/A within
  0.03 in Chrome and Node within its noise. Sharing a result of any other constant, such
  as `Err(DecodeError::Short)`, measured no faster on the decodes that return it.
- A `&[u8]` is an `rt.ByteSlice` rather than a `Uint8Array` view, since a view costs
  several times what an object does to make in V8 and JavaScriptCore, and a varint's
  decode splits its slice once to four times. `just ts bench` measured the codec's decode
  workloads at 0.75 times the views' time in Chrome, 0.81-0.84 in Node, and 0.82-0.85 in
  Bun at the geometric mean, over two runs whose A/A route measured within 0.05 of them,
  and encode no slower. A `&[u8; N]`, such as `split_first_chunk`'s chunk, is an
  `rt.ByteSlice` too, its bytes read in place: the decode workloads then took 0.72 times
  a view's time in Chrome, 0.67 in Firefox, 0.87 in Node, and 0.71 in Bun at the
  geometric mean, with the A/A route within 0.03, and encode no slower.
- A local built of byte slices and scalars, in tuples, `Option`s, `Result`s, and
  `ControlFlow`s, is a JS value per part: a slice's `data`, `start`, and `length`, and an
  enum's variant as a flag, when nothing borrows or writes into it and only std's splits,
  `ok_or`, `?`, aggregates, and copies assign it. A read that takes it apart reads its
  parts; any other makes its object there, and a local whose object would be made more
  often than before stays one. `just ts bench` measured the codec's varint decode at 0.49
  times the objects' time in Chrome, 0.57 in Firefox, 0.64 in Node, and 0.56 in Bun at
  the geometric mean, and frame decode at 0.61, 0.56, 0.70, and 0.61, with the A/A route
  within 0.04 and encode no slower. A parameter, a return, and a field are still a
  `ByteSlice` ([byte slices without objects](/quest/m2/ts-byte-views/README.md)).
- A `BTreeMap` is a B+ tree of up to 64 keys a node, O(log n) as std's B-tree is: its
  leaves hold the keys and values and link to their neighbors, which iterators walk from
  either end, and its branches hold each child's lower bound. A node's slots shift in a
  loop, which beat `splice` 2-3x at these sizes. On `just ts bench`'s `btree/` workloads
  it takes 0.5-0.8x the time two sorted arrays did, from 16 keys to 4096, in Node, Bun,
  and Chrome; 64 keys a node measured best of 16 to 128. A hash map's JS primitive keys
  are a JS `Map`'s, the engine's own hash table; its other keys go into a second `Map`
  by their translated hash, chained by `Eq`. `VecDeque` is a ring buffer over an array
  whose length is a power of two.
- A slice's `sort` and `sort_unstable` are one stable merge sort of the runtime's own,
  calling only the comparator, as std permits of an unstable sort: JS's `sort` moves
  `undefined`, a `None`, to the end without comparing it, and each engine orders its
  comparisons differently.
- Iterators are the runtime's lazy classes rather than generators or ES iterator
  helpers: std's adaptors would lower from std's own bodies, whose slice iterators are
  raw pointers, ES helpers lack most of them
  (`rev`, `zip`, `peekable`), and a `next` returning an item or `rt.DONE` allocates
  nothing per item. `just ts bench` measures chains of `map`, `filter`, `any`, `skip`,
  and `take` 1.4-8x faster than the same ES helpers in Node, Bun, and Chrome.
- Bodies lower from MIR, not THIR: drops are where drop elaboration put them, exact by
  construction, with no `finally` per scope or `undefined` left by a move. Async bodies are
  the coroutine transform's state machines, a class each. Against the THIR lowering it
  replaced, MIR closes about 1,100 of the 1,400 drop refusals in moq-net's census, and
  runs `just ts bench`'s codec 2-6% faster at the geometric mean and the async handoff
  1.4-5.9x faster at depths 1 to 16 in Node, Bun, and Chrome, in bundles no larger
  gzipped. Blocks become nested blocks, loops, and `if`s by
  Ramsey's "Beyond Relooper", except a coroutine's resume function, which enters loops
  in the middle and becomes a loop over a `switch` on the next block. Temporaries
  assigned once and read once fold back into the expression that reads them unless that
  would reorder an effect.
- `Option`'s and `Result`'s combinators lower from core's own MIR rather than runtime
  functions, each of which restated core's drops by hand. Each closure gets
  its own instance, keeping call sites monomorphic, and instances that print the same are
  merged into one. An `Ok` rebuilt from a stable `Result`'s payload is that `Result`,
  without which Node ran the codec 1.31x slower. Against the runtime functions, the codec
  and async handoff bundles are within 25 bytes gzipped, and `just ts bench` measures
  them within the A/A route's spread in Node, Bun, Chrome, and Firefox.

## Not yet

Byte reinterpretation, pointer arithmetic and addresses, raw allocation, and unions whose
fields overlap in different types ([unsafe](/quest/m3/ts-unsafe.md)), and trait objects other than
`&dyn Debug`, `&dyn Display`, `dyn Future`, `dyn Any`, `dyn Fn*`, and the translated crates' traits'
behind `&`, `&mut`, `Box`, or `Arc` ([trait objects](/quest/m3/ts-dyn.md)), fail translation. Until they translate, an item that needs one, such as a `dyn Error`, binds to
hand-written TS ([bindings](#bindings)).

These fail translation: formatting and parsing an `f64`, and `f64::clamp`, whose panic formats them ([float formatting](/quest/m2/ts-model/floats.md)), `f32` ([`f32`](/quest/m3/ts-f32.md)), statics, async closures, a `Poll` of a `Poll` of a `Poll`, a future whose output can be pending, an instance of a generic type whose field is a `Poll` of a type parameter that can be pending, a closure that owns a value
that drops given to a std function that drops it beside another value that drops, as
`for_each` of items that drop does, or keeps it other than in an iterator adaptor, and
values that drop in some iterators ([drop order](/quest/m3/ts-drop-order.md)), a `Cow` of other than `str`,
a collection holding a type that can be `undefined`, as `()` and `Option` can, a std
function without a mapping of its own that makes or reads an `Option` whose `None` is
`rt.None`, as `find` over `Option`s does, an `Option` of an `Option` of `()` or another
`Option`, an instance of a generic type whose field is an `Option` of a type parameter
that can be `undefined`, a collection's `capacity`, looking up a key by
another type it borrows as through the crate's own `Borrow`, a `&mut` to a value JS
copies that std makes, other than a map's, or hands a closure, as `iter_mut` and
`and_modify` do, one into a `Result`'s payload passed on or kept, `Hasher::write` of bytes, the adaptors
without a mapping in `src/lower/iter.rs` (such as `cycle`, `unzip`, and `partition`), an
own `impl Iterator` with `next_back`, `len`, or another provided method of its own,
a pointer's address (`{:p}`), writing to a
`String` with `write!`, a `str` pattern other than a
`char`, a `&str`, a predicate, or chars, a mutable `str` slice, a copy of a closure that changes
what it captured, `mem::swap`, `replace`, or `take` of a reference's whole pointee,
assigning an `Arc`, a waker, a `MutexGuard`, a pinned reference, or a future through a
reference, a switch on or a field of another crate's type without a mapping, and std
functions without a mapping in `src/lower/library.rs`, but for `bool`'s, `Option`'s,
`Result`'s, and `Poll`'s methods that take no `&mut`, which lower from core's MIR.

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
- `lang` covers every integer width's arithmetic and casts, `char`s next to each
  encoding's edges and the surrogates, `Cow<str>` borrowed and owned, `Duration`s whose
  seconds pass 2^53 or reach `u64::MAX`, control flow,
  patterns, `Copy` aggregates, those a container stores included and those written into
  only in another module, behind a type parameter, or through a box, formatting, methods
  on generic types, iterators, with
  closures that log what each
  adaptor reads, `Option`'s and `Result`'s combinators, with closures that log each call,
  and std's collections, over keys whose own `Hash`, `Eq`, and `Ord`
  disagree with JS's, `vec![x; n]` of types whose `Clone` counts its clones, `&mut`
  to values JS copies, of locals, fields, elements, and a map's values, passed on,
  returned, kept in a struct, and captured, a `Vec`'s edits and sorts, with closures
  that log what each is called with, std's `Bound`, `Reverse`, and `NonZero`, and raw
  pointers: a linked list of boxed nodes as `hashlink` keeps one, through a union and a
  `MaybeUninit` entry, and pointers to locals, fields, and whole structs. Its
  differential test runs through `caller.ts`, a
  hand-written caller that also prints a `printable` error nothing translated prints,
  and adds every 64- and 128-bit case over each pair of values next to a power of two,
  where the runtime switches between numbers and limbs, and its limbs carry and borrow,
  with products of those values at 128 bits, 128-bit quotients by divisors below 2^32,
  and their halves joined back.
  `brands.ts`, another caller, mixes integer types where `tsc` must refuse them.
- `share` holds a `Copy` aggregate whose only write in the crate makes it copied, where
  `lang`'s other writes into the same type would hide a write the analysis misses: a
  byte array overwritten whole through a type parameter, or, under its features, by std
  called directly or through a pointer a generic function calls.
- `task` trims moq's kio to a dependency of its own, `kio`, reached across crates as moq-net
  reaches kio: a channel of `Arc`ed state, waiters, and wakers, kio's `Task`, `Pending`, and
  `Waiter::poll_future`, and a `Consumer::poll` whose closure is itself pending, a
  `Poll<Poll<_>>`. Async functions and std's future helpers run over it, with wakers of
  the crate's `Wake` impls, beside values that drop inside std's types. Its caller is hand-written TS (`tests/task.ts`): a FIFO executor that runs a
  script of spawns, sends, polls, and drops, and logs every poll, wake, and drop, as the
  fixture's native executor does. The logs must match byte for byte.
- `stock` calls `bytes`, `rand`, and `tracing` with no config, against native Rust, whose
  `rand::rng()` reads the fixture's own getrandom backend on wasm32. It checks `Bytes`'s
  slices and panics, how each is made and printed, that no event evaluates a field, and
  that each random value is in its range, every value of a small range drawn. It splits,
  freezes, grows, and writes a `BytesMut` by ops its input picks, ending with every
  piece's bytes, and reads and writes through `Buf` and `BufMut` generically, a buffer of
  its own included.
- `bind` binds `buf::Blob`, a renamed dependency's type with `unsafe` inside, and a function and a type of its own that read through raw
  pointers, the type dropping an `Arc` where Rust drops it, to the TS in `bind/`. Its
  differential test runs them against native Rust. `wrong.toml` binds TS that doesn't match
  the Rust signatures, which `tsc` refuses at each item, with the errors in `wrong.txt`.
  `refused.toml` holds bindings the config can't make, with the errors in `refused.txt`.
- `crates` translates with `crates/dep`, which writes into a `Copy` type only the package
  copies, ranks the package's type in a generic function, drops, iterates, and reads
  through a raw pointer behind a safe twin under `rust2_ts` and in a function
  `bind/last.ts` stands in for. `bind/slots.ts` stands in for a type whose method alone
  writes into an array the package copies. Its differential test runs them against native Rust.
  `refused.toml` drops the binding, its `rev` feature makes the dependency's iterator run
  from either end, and its `shared` feature adds a module of the dependency's name, with
  the errors in `refused.txt`, `rev.txt`, and `shared.txt`.
- `reject` holds constructs outside the subset; its expected errors are `expected.txt`,
  and its expected census is `census.md`.

`tests/spawn.ts` checks that `rt.spawn` yields to a timer after its 128-poll budget, and
rejects when a future panics as it drops. `tests/retain.ts` runs a collection's `retain`
and `Vec::dedup_by` over every keep mask of up to 6 items, and `retain` checks that each
leaves and drops what std's does.

## Bench

```bash
nix develop .#bench --command just ts bench  # report to target/bench/report.md
```

The codec fixture's `bench::run` holds the cases moq's own TypeScript benchmarks time
(`js/net/bench/varint.ts` and `lite-varint.ts`), at the pinned moq revision, the same
frame headers decoded as they arrive a few bytes at a time, which prices a decode that
comes up `Short` and retries, iterator chains over bytes, a `BTreeMap` of 16, 256, and
4096 keys, and timestamps converted between timescales as moq-net's `Timestamp::convert`
does, whose products pass 2^53, then added by its `checked_add` and ordered by its `Ord`,
whose cross products do too. Each runs four ways: natively, as wasm32 at moq-wasm's
size-optimized profile, hand-written (moq's `js/net` in `bench/moq.ts` and its `Timestamp`
in `bench/timestamp.ts`, ES iterator helpers in `bench/iter.ts`, a sorted array in
`bench/sorted.ts`, and `bigint` in `bench/rescale.ts`, as moq's `js/hang` converts
timescales), and translated; every route's output must match
native Rust's first. The JS routes run minified, the same bundles whose size it reports,
in Node, Bun, Chrome, and Firefox, with the WASM route's copies across the boundary timed
apart from its compute, and an A/A copy of the
translation to show the noise. Each route runs alone in a worker of its own, since routes
in one realm share type profiles and run slower loaded second. Each round runs the JS
routes and workloads in a new random order, so no route keeps a slot that runs it faster or slower than identical code
in another; the report records the seed. It also records the host and engine versions,
and each route's bundle size raw and gzipped. It sets no threshold; the nightly workflow
runs it and posts the report to its summary. Each browser's page reports each step it
starts, and one that runs 30 s fails the run, naming it.

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

The bench shell adds Chromium, Firefox, and the wasm-bindgen that builds moq-wasm's package.
