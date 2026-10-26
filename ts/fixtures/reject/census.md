# Census of `reject-fixture`

- Refusals: 69
- Function instances: 61
- Generic functions no instance reached, lowered over their type parameters: 3

## By construct

| Count | Construct | First |
| ---: | --- | --- |
| 6 | an assignment through a reference to `_` | fixtures/reject/src/lib.rs:20:2 |
| 5 | `_`: a JS value of `_` is a copy, so a reference can't change it | fixtures/reject/src/lib.rs:19:1 |
| 5 | unsupported call to `_` | fixtures/reject/src/lib.rs:36:4 |
| 4 | unsupported type `_` | fixtures/reject/src/lib.rs:6:1 |
| 3 | `_` is not supported: its `None` and `Some` would both be `undefined` | fixtures/reject/src/lib.rs:31:1 |
| 3 | an iterator that owns `_`, which drops, which is not supported yet | fixtures/reject/src/lib.rs:59:2 |
| 3 | replacing the value behind a reference to `_` | fixtures/reject/src/lib.rs:133:2 |
| 2 | `_` is not supported: it would be pending two ways with one value | fixtures/reject/src/lib.rs:159:1 |
| 2 | a field of `_` | fixtures/reject/src/lib.rs:326:7 |
| 2 | an iterator of the crate implementing `_`, which is not supported yet | fixtures/reject/src/lib.rs:218:1 |
| 2 | constructing `_` from another crate | fixtures/reject/src/lib.rs:204:23 |
| 2 | overriding `_`, which std's adaptors call in place of `next` or beside it | fixtures/reject/src/lib.rs:198:2 |
| 1 | `Binary` formatting, as in `{:b}`, is not supported | fixtures/reject/src/lib.rs:110:3 |
| 1 | `LowerExp` formatting, as in `{:e}`, is not supported | fixtures/reject/src/lib.rs:112:3 |
| 1 | `Octal` formatting, as in `{:o}`, is not supported | fixtures/reject/src/lib.rs:111:3 |
| 1 | `Pointer` formatting, as in `{:p}`, is not supported | fixtures/reject/src/lib.rs:115:3 |
| 1 | `_` holds `_`, which is `undefined` as `None` is, which is not supported | fixtures/reject/src/lib.rs:336:6 |
| 1 | `_` is generic and implements `Drop`, which is not supported yet | fixtures/reject/src/lib.rs:75:1 |
| 1 | `_` of `_` by `_`, which is not supported yet | fixtures/reject/src/lib.rs:371:6 |
| 1 | `_` of `_`, which may drop a `_`, which is not supported yet | fixtures/reject/src/lib.rs:354:6 |
| 1 | `_` of `_`, which may drop what it holds, which is not supported yet | fixtures/reject/src/lib.rs:331:6 |
| 1 | `_`, which is not supported yet | fixtures/reject/src/lib.rs:379:9 |
| 1 | a `_` cast from `_` to `_` | fixtures/reject/src/lib.rs:144:2 |
| 1 | a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` | fixtures/reject/src/lib.rs:231:51 |
| 1 | a cast from `_` | fixtures/reject/src/lib.rs:11:2 |
| 1 | a constant of type `_` | fixtures/reject/src/lib.rs:128:2 |
| 1 | a copy of a closure that changes what it captured | fixtures/reject/src/lib.rs:320:14 |
| 1 | a mutable borrow of a subslice of `_`, which JS would copy, having no views | fixtures/reject/src/lib.rs:295:13 |
| 1 | a precision, as in `{:.2}`, is not supported | fixtures/reject/src/lib.rs:107:3 |
| 1 | a printable type must implement `Display` or `Debug` | fixtures/reject/src/lib.rs:140:1 |
| 1 | a scalar constant of type `_` | fixtures/reject/src/lib.rs:7:2 |
| 1 | a width or precision from an argument, as in `{:1$}`, is not supported | fixtures/reject/src/lib.rs:114:3 |
| 1 | a zero-sized constant of type `_` | fixtures/reject/src/lib.rs:100:2 |
| 1 | arithmetic on `_` | fixtures/reject/src/lib.rs:7:2 |
| 1 | comparing `_` as a key, which is not supported yet | fixtures/reject/src/lib.rs:341:52 |
| 1 | dropping `_`, which is not supported yet | fixtures/reject/src/lib.rs:151:1 |
| 1 | dropping a closure that owns a value that drops, which is not supported yet | fixtures/reject/src/lib.rs:54:1 |
| 1 | hashing `_` as a key, which is not supported yet | fixtures/reject/src/lib.rs:341:52 |
| 1 | hex `Debug`, as in `{:x?}`, is not supported | fixtures/reject/src/lib.rs:113:3 |
| 1 | passing `_`, which drops, to `_`, which is not supported yet | fixtures/reject/src/lib.rs:290:4 |
| 1 | the `+` flag of a format spec is not supported | fixtures/reject/src/lib.rs:108:3 |
| 1 | the `-` flag of a format spec is not supported | fixtures/reject/src/lib.rs:109:3 |

## By API

The type or module each refused type or path belongs to.

| Count | API | First |
| ---: | --- | --- |
| 5 | `f64` | fixtures/reject/src/lib.rs:6:1 |
| 4 | `Option` | fixtures/reject/src/lib.rs:31:1 |
| 4 | `std::vec::Vec` | fixtures/reject/src/lib.rs:59:2 |
| 4 | `u32` | fixtures/reject/src/lib.rs:19:1 |
| 4 | `u8` | fixtures/reject/src/lib.rs:90:10 |
| 3 | `std::collections::HashMap` | fixtures/reject/src/lib.rs:331:6 |
| 2 | `HashMap` | fixtures/reject/src/lib.rs:331:6 |
| 2 | `Iterator` | fixtures/reject/src/lib.rs:198:2 |
| 2 | `Meters` | fixtures/reject/src/lib.rs:133:2 |
| 2 | `Poll` | fixtures/reject/src/lib.rs:159:1 |
| 2 | `char` | fixtures/reject/src/lib.rs:10:1 |
| 2 | `std::option::Option` | fixtures/reject/src/lib.rs:290:4 |
| 2 | `std::pin::Pin` | fixtures/reject/src/lib.rs:175:2 |
| 2 | `std::string::String` | fixtures/reject/src/lib.rs:120:1 |
| 2 | `std::sync::Arc` | fixtures/reject/src/lib.rs:151:1 |
| 1 | `Boxed` | fixtures/reject/src/lib.rs:75:1 |
| 1 | `DoubleEndedIterator` | fixtures/reject/src/lib.rs:218:1 |
| 1 | `FusedIterator` | fixtures/reject/src/lib.rs:272:1 |
| 1 | `HashSet` | fixtures/reject/src/lib.rs:354:6 |
| 1 | `Hasher` | fixtures/reject/src/lib.rs:379:9 |
| 1 | `IntToFloat` | fixtures/reject/src/lib.rs:144:2 |
| 1 | `Loud` | fixtures/reject/src/lib.rs:354:6 |
| 1 | `Noisy` | fixtures/reject/src/lib.rs:285:20 |
| 1 | `Slot` | fixtures/reject/src/lib.rs:73:1 |
| 1 | `[u8; 1]` | fixtures/reject/src/lib.rs:309:22 |
| 1 | `dyn std::ops::Fn(u8) -> u8` | fixtures/reject/src/lib.rs:14:1 |
| 1 | `std::collections::HashSet` | fixtures/reject/src/lib.rs:354:6 |
| 1 | `std::iter::Chain` | fixtures/reject/src/lib.rs:231:51 |
| 1 | `std::iter::Map` | fixtures/reject/src/lib.rs:204:23 |
| 1 | `std::iter::Rev` | fixtures/reject/src/lib.rs:225:14 |
| 1 | `std::mem` | fixtures/reject/src/lib.rs:52:18 |
| 1 | `std::ops::RangeFrom` | fixtures/reject/src/lib.rs:326:48 |
| 1 | `std::ops::RangeTo` | fixtures/reject/src/lib.rs:326:7 |
| 1 | `std::ops::Range` | fixtures/reject/src/lib.rs:239:8 |
| 1 | `std::slice::Iter` | fixtures/reject/src/lib.rs:185:11 |
| 1 | `std::slice` | fixtures/reject/src/lib.rs:36:4 |
| 1 | `std::sync::MutexGuard` | fixtures/reject/src/lib.rs:180:2 |
| 1 | `u64` | fixtures/reject/src/lib.rs:144:2 |
| 1 | `{async closure}` | fixtures/reject/src/lib.rs:100:2 |

## By type or path

| Count | Construct | Type or path | First |
| ---: | --- | --- | --- |
| 3 | `_` is not supported: its `None` and `Some` would both be `undefined` | `Option` | fixtures/reject/src/lib.rs:31:1 |
| 2 | `_` is not supported: it would be pending two ways with one value | `Poll` | fixtures/reject/src/lib.rs:159:1 |
| 2 | `_`: a JS value of `_` is a copy, so a reference can't change it | `&mut u32`, `u32` | fixtures/reject/src/lib.rs:19:1 |
| 2 | an assignment through a reference to `_` | `std::pin::Pin` | fixtures/reject/src/lib.rs:175:2 |
| 2 | an iterator that owns `_`, which drops, which is not supported yet | `std::vec::Vec` | fixtures/reject/src/lib.rs:59:2 |
| 2 | replacing the value behind a reference to `_` | `Meters` | fixtures/reject/src/lib.rs:133:2 |
| 2 | unsupported type `_` | `f64` | fixtures/reject/src/lib.rs:6:1 |
| 1 | `_` holds `_`, which is `undefined` as `None` is, which is not supported | `std::collections::HashMap`, `std::option::Option` | fixtures/reject/src/lib.rs:336:6 |
| 1 | `_` is generic and implements `Drop`, which is not supported yet | `Boxed` | fixtures/reject/src/lib.rs:75:1 |
| 1 | `_` of `_` by `_`, which is not supported yet | `HashMap::contains_key`, `std::collections::HashMap`, `u8` | fixtures/reject/src/lib.rs:371:6 |
| 1 | `_` of `_`, which may drop a `_`, which is not supported yet | `HashSet::remove`, `std::collections::HashSet`, `Loud` | fixtures/reject/src/lib.rs:354:6 |
| 1 | `_` of `_`, which may drop what it holds, which is not supported yet | `HashMap::clear`, `std::collections::HashMap` | fixtures/reject/src/lib.rs:331:6 |
| 1 | `_`, which is not supported yet | `Hasher::write` | fixtures/reject/src/lib.rs:379:9 |
| 1 | `_`: a JS value of `_` is a copy, so a reference can't change it | `&mut Slot`, `Slot` | fixtures/reject/src/lib.rs:73:1 |
| 1 | `_`: a JS value of `_` is a copy, so a reference can't change it | `&mut std::string::String`, `std::string::String` | fixtures/reject/src/lib.rs:120:1 |
| 1 | `_`: a JS value of `_` is a copy, so a reference can't change it | `&mut u8`, `u8` | fixtures/reject/src/lib.rs:90:10 |
| 1 | a `_` cast from `_` to `_` | `IntToFloat`, `u64`, `f64` | fixtures/reject/src/lib.rs:144:2 |
| 1 | a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` | `std::iter::Chain` | fixtures/reject/src/lib.rs:231:51 |
| 1 | a cast from `_` | `char` | fixtures/reject/src/lib.rs:11:2 |
| 1 | a constant of type `_` | `&u8` | fixtures/reject/src/lib.rs:128:2 |
| 1 | a field of `_` | `std::ops::RangeFrom` | fixtures/reject/src/lib.rs:326:48 |
| 1 | a field of `_` | `std::ops::RangeTo` | fixtures/reject/src/lib.rs:326:7 |
| 1 | a mutable borrow of a subslice of `_`, which JS would copy, having no views | `u32` | fixtures/reject/src/lib.rs:295:13 |
| 1 | a scalar constant of type `_` | `f64` | fixtures/reject/src/lib.rs:7:2 |
| 1 | a zero-sized constant of type `_` | `{async closure}` | fixtures/reject/src/lib.rs:100:2 |
| 1 | an assignment through a reference to `_` | `std::sync::Arc` | fixtures/reject/src/lib.rs:155:2 |
| 1 | an assignment through a reference to `_` | `std::sync::MutexGuard` | fixtures/reject/src/lib.rs:180:2 |
| 1 | an assignment through a reference to `_` | `u32` | fixtures/reject/src/lib.rs:20:2 |
| 1 | an assignment through a reference to `_` | `u8` | fixtures/reject/src/lib.rs:91:3 |
| 1 | an iterator of the crate implementing `_`, which is not supported yet | `DoubleEndedIterator` | fixtures/reject/src/lib.rs:218:1 |
| 1 | an iterator of the crate implementing `_`, which is not supported yet | `FusedIterator` | fixtures/reject/src/lib.rs:272:1 |
| 1 | an iterator that owns `_`, which drops, which is not supported yet | `Noisy` | fixtures/reject/src/lib.rs:285:20 |
| 1 | arithmetic on `_` | `f64` | fixtures/reject/src/lib.rs:7:2 |
| 1 | comparing `_` as a key, which is not supported yet | `std::vec::Vec` | fixtures/reject/src/lib.rs:341:52 |
| 1 | constructing `_` from another crate | `std::iter::Map` | fixtures/reject/src/lib.rs:204:23 |
| 1 | constructing `_` from another crate | `std::iter::Rev` | fixtures/reject/src/lib.rs:225:14 |
| 1 | dropping `_`, which is not supported yet | `std::sync::Arc` | fixtures/reject/src/lib.rs:151:1 |
| 1 | hashing `_` as a key, which is not supported yet | `std::vec::Vec` | fixtures/reject/src/lib.rs:341:52 |
| 1 | overriding `_`, which std's adaptors call in place of `next` or beside it | `Iterator::fold` | fixtures/reject/src/lib.rs:198:2 |
| 1 | overriding `_`, which std's adaptors call in place of `next` or beside it | `Iterator::size_hint` | fixtures/reject/src/lib.rs:252:2 |
| 1 | passing `_`, which drops, to `_`, which is not supported yet | `std::option::Option`, `Option::unwrap_or` | fixtures/reject/src/lib.rs:290:4 |
| 1 | replacing the value behind a reference to `_` | `[u8; 1]` | fixtures/reject/src/lib.rs:309:22 |
| 1 | unsupported call to `_` | `<std::slice::Iter as std::iter::Iterator>::cycle` | fixtures/reject/src/lib.rs:185:11 |
| 1 | unsupported call to `_` | `<std::string::String as std::fmt::Write>::write_fmt` | fixtures/reject/src/lib.rs:122:10 |
| 1 | unsupported call to `_` | `std::mem::size_of_val` | fixtures/reject/src/lib.rs:52:18 |
| 1 | unsupported call to `_` | `std::ops::Range::contains` | fixtures/reject/src/lib.rs:239:8 |
| 1 | unsupported call to `_` | `std::slice::sort` | fixtures/reject/src/lib.rs:36:4 |
| 1 | unsupported type `_` | `char` | fixtures/reject/src/lib.rs:10:1 |
| 1 | unsupported type `_` | `dyn std::ops::Fn(u8) -> u8` | fixtures/reject/src/lib.rs:14:1 |

## Spans

<details><summary>an assignment through a reference to `_` (6)</summary>

- fixtures/reject/src/lib.rs:20:2: an assignment through a reference to `u32`
- fixtures/reject/src/lib.rs:91:3: an assignment through a reference to `u8`
- fixtures/reject/src/lib.rs:155:2: an assignment through a reference to `std::sync::Arc<u8>`
- fixtures/reject/src/lib.rs:175:2: an assignment through a reference to `std::pin::Pin<std::sync::Arc<u8>>`
- fixtures/reject/src/lib.rs:180:2: an assignment through a reference to `std::sync::MutexGuard<'_, u8>`
- fixtures/reject/src/lib.rs:303:2: an assignment through a reference to `std::pin::Pin<&mut Noisy>`

</details>

<details><summary>`_`: a JS value of `_` is a copy, so a reference can't change it (5)</summary>

- fixtures/reject/src/lib.rs:19:1: `&mut u32`: a JS value of `u32` is a copy, so a reference can't change it
- fixtures/reject/src/lib.rs:24:1: `&mut u32`: a JS value of `u32` is a copy, so a reference can't change it
- fixtures/reject/src/lib.rs:73:1: `&mut Slot`: a JS value of `Slot` is a copy, so a reference can't change it
- fixtures/reject/src/lib.rs:90:10: `&mut u8`: a JS value of `u8` is a copy, so a reference can't change it
- fixtures/reject/src/lib.rs:120:1: `&mut std::string::String`: a JS value of `std::string::String` is a copy, so a reference can't change it

</details>

<details><summary>unsupported call to `_` (5)</summary>

- fixtures/reject/src/lib.rs:36:4: unsupported call to `std::slice::<impl [u8]>::sort`
- fixtures/reject/src/lib.rs:52:18: unsupported call to `std::mem::size_of_val::<Noisy>`
- fixtures/reject/src/lib.rs:122:10: unsupported call to `<std::string::String as std::fmt::Write>::write_fmt`
- fixtures/reject/src/lib.rs:185:11: unsupported call to `<std::slice::Iter<'_, u8> as std::iter::Iterator>::cycle`
- fixtures/reject/src/lib.rs:239:8: unsupported call to `std::ops::Range::<Key>::contains::<Key>`

</details>

<details><summary>unsupported type `_` (4)</summary>

- fixtures/reject/src/lib.rs:6:1: unsupported type `f64`
- fixtures/reject/src/lib.rs:10:1: unsupported type `char`
- fixtures/reject/src/lib.rs:14:1: unsupported type `dyn std::ops::Fn(u8) -> u8`
- fixtures/reject/src/lib.rs:142:1: unsupported type `f64`

</details>

<details><summary>`_` is not supported: its `None` and `Some` would both be `undefined` (3)</summary>

- fixtures/reject/src/lib.rs:31:1: `Option<std::option::Option<u8>>` is not supported: its `None` and `Some` would both be `undefined`
- fixtures/reject/src/lib.rs:40:42: `Option<std::option::Option<u8>>` is not supported: its `None` and `Some` would both be `undefined`
- fixtures/reject/src/lib.rs:164:1: `Option<std::task::Poll<()>>` is not supported: its `None` and `Some` would both be `undefined`

</details>

<details><summary>an iterator that owns `_`, which drops, which is not supported yet (3)</summary>

- fixtures/reject/src/lib.rs:59:2: an iterator that owns `std::vec::Vec<Noisy>`, which drops, which is not supported yet
- fixtures/reject/src/lib.rs:280:4: an iterator that owns `std::vec::Vec<Noisy>`, which drops, which is not supported yet
- fixtures/reject/src/lib.rs:285:20: an iterator that owns `Noisy`, which drops, which is not supported yet

</details>

<details><summary>replacing the value behind a reference to `_` (3)</summary>

- fixtures/reject/src/lib.rs:133:2: replacing the value behind a reference to `Meters`
- fixtures/reject/src/lib.rs:137:2: replacing the value behind a reference to `Meters`
- fixtures/reject/src/lib.rs:309:22: replacing the value behind a reference to `[u8; 1]`

</details>

<details><summary>`_` is not supported: it would be pending two ways with one value (2)</summary>

- fixtures/reject/src/lib.rs:159:1: `Poll<std::option::Option<std::task::Poll<u8>>>` is not supported: it would be pending two ways with one value
- fixtures/reject/src/lib.rs:169:54: `Poll<std::task::Poll<u8>>` is not supported: it would be pending two ways with one value

</details>

<details><summary>a field of `_` (2)</summary>

- fixtures/reject/src/lib.rs:326:7: a field of `std::ops::RangeTo<usize>`
- fixtures/reject/src/lib.rs:326:48: a field of `std::ops::RangeFrom<usize>`

</details>

<details><summary>an iterator of the crate implementing `_`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:218:1: an iterator of the crate implementing `DoubleEndedIterator`, which is not supported yet
- fixtures/reject/src/lib.rs:272:1: an iterator of the crate implementing `FusedIterator`, which is not supported yet

</details>

<details><summary>constructing `_` from another crate (2)</summary>

- fixtures/reject/src/lib.rs:204:23: constructing `std::iter::Map<Folds, {closure@fixtures/reject/src/lib.rs:204:12: 204:15}>` from another crate
- fixtures/reject/src/lib.rs:225:14: constructing `std::iter::Rev<Backs>` from another crate

</details>

<details><summary>overriding `_`, which std's adaptors call in place of `next` or beside it (2)</summary>

- fixtures/reject/src/lib.rs:198:2: overriding `Iterator::fold`, which std's adaptors call in place of `next` or beside it
- fixtures/reject/src/lib.rs:252:2: overriding `Iterator::size_hint`, which std's adaptors call in place of `next` or beside it

</details>

<details><summary>`Binary` formatting, as in `{:b}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:110:3: `Binary` formatting, as in `{:b}`, is not supported

</details>

<details><summary>`LowerExp` formatting, as in `{:e}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:112:3: `LowerExp` formatting, as in `{:e}`, is not supported

</details>

<details><summary>`Octal` formatting, as in `{:o}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:111:3: `Octal` formatting, as in `{:o}`, is not supported

</details>

<details><summary>`Pointer` formatting, as in `{:p}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:115:3: `Pointer` formatting, as in `{:p}`, is not supported

</details>

<details><summary>`_` holds `_`, which is `undefined` as `None` is, which is not supported (1)</summary>

- fixtures/reject/src/lib.rs:336:6: `std::collections::HashMap<u8, std::option::Option<u8>>` holds `std::option::Option<u8>`, which is `undefined` as `None` is, which is not supported

</details>

<details><summary>`_` is generic and implements `Drop`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:75:1: `Boxed` is generic and implements `Drop`, which is not supported yet

</details>

<details><summary>`_` of `_` by `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:371:6: `HashMap::contains_key` of `std::collections::HashMap<Id, u8>` by `u8`, which is not supported yet

</details>

<details><summary>`_` of `_`, which may drop a `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:354:6: `HashSet::remove` of `std::collections::HashSet<Loud>`, which may drop a `Loud`, which is not supported yet

</details>

<details><summary>`_` of `_`, which may drop what it holds, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:331:6: `HashMap::clear` of `std::collections::HashMap<u8, Noisy>`, which may drop what it holds, which is not supported yet

</details>

<details><summary>`_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:379:9: `Hasher::write`, which is not supported yet

</details>

<details><summary>a `_` cast from `_` to `_` (1)</summary>

- fixtures/reject/src/lib.rs:144:2: a `IntToFloat` cast from `u64` to `f64`

</details>

<details><summary>a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` (1)</summary>

- fixtures/reject/src/lib.rs:231:51: a `zip` of `std::iter::Chain<std::iter::Map<std::slice::Iter<'_, u8>, {closure@fixtures/reject/src/lib.rs:231:15: 231:18}>, std::iter::Copied<std::slice::Iter<'_, u8>>>`, whose closures std calls one item further through `next` than through `fold`

</details>

<details><summary>a cast from `_` (1)</summary>

- fixtures/reject/src/lib.rs:11:2: a cast from `char`

</details>

<details><summary>a constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:128:2: a constant of type `&u8`

</details>

<details><summary>a copy of a closure that changes what it captured (1)</summary>

- fixtures/reject/src/lib.rs:320:14: a copy of a closure that changes what it captured

</details>

<details><summary>a mutable borrow of a subslice of `_`, which JS would copy, having no views (1)</summary>

- fixtures/reject/src/lib.rs:295:13: a mutable borrow of a subslice of `u32`, which JS would copy, having no views

</details>

<details><summary>a precision, as in `{:.2}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:107:3: a precision, as in `{:.2}`, is not supported

</details>

<details><summary>a printable type must implement `Display` or `Debug` (1)</summary>

- fixtures/reject/src/lib.rs:140:1: a printable type must implement `Display` or `Debug`

</details>

<details><summary>a scalar constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:7:2: a scalar constant of type `f64`

</details>

<details><summary>a width or precision from an argument, as in `{:1$}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:114:3: a width or precision from an argument, as in `{:1$}`, is not supported

</details>

<details><summary>a zero-sized constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:100:2: a zero-sized constant of type `{async closure@fixtures/reject/src/lib.rs:99:10: 99:18}`

</details>

<details><summary>arithmetic on `_` (1)</summary>

- fixtures/reject/src/lib.rs:7:2: arithmetic on `f64`

</details>

<details><summary>comparing `_` as a key, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:341:52: comparing `std::vec::Vec<u8>` as a key, which is not supported yet

</details>

<details><summary>dropping `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:151:1: dropping `std::sync::Arc<&Noisy>`, which is not supported yet

</details>

<details><summary>dropping a closure that owns a value that drops, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:54:1: dropping a closure that owns a value that drops, which is not supported yet

</details>

<details><summary>hashing `_` as a key, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:341:52: hashing `std::vec::Vec<u8>` as a key, which is not supported yet

</details>

<details><summary>hex `Debug`, as in `{:x?}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:113:3: hex `Debug`, as in `{:x?}`, is not supported

</details>

<details><summary>passing `_`, which drops, to `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:290:4: passing `std::option::Option<Noisy>`, which drops, to `Option::unwrap_or`, which is not supported yet

</details>

<details><summary>the `+` flag of a format spec is not supported (1)</summary>

- fixtures/reject/src/lib.rs:108:3: the `+` flag of a format spec is not supported

</details>

<details><summary>the `-` flag of a format spec is not supported (1)</summary>

- fixtures/reject/src/lib.rs:109:3: the `-` flag of a format spec is not supported

</details>
