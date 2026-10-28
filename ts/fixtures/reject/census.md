# Census of `reject-fixture`

- Refusals: 77
- Function instances: 84
- Generic functions no instance reached, lowered over their type parameters: 6

## By construct

| Count | Construct | First |
| ---: | --- | --- |
| 11 | unsupported call to `_` | fixtures/reject/src/lib.rs:28:4 |
| 4 | a variant of `_`, which is not supported: either is the same JS string | fixtures/reject/src/lib.rs:424:2 |
| 4 | an assignment through a reference to `_` | fixtures/reject/src/lib.rs:117:2 |
| 4 | unsupported type `_` | fixtures/reject/src/lib.rs:7:14 |
| 3 | `_` with a mutable reference to a value JS copies, which its mapping would copy | fixtures/reject/src/lib.rs:17:2 |
| 3 | comparing `_` through its own `_`, which is not supported yet | fixtures/reject/src/lib.rs:381:4 |
| 3 | replacing the value behind a reference to `_` | fixtures/reject/src/lib.rs:101:2 |
| 2 | `_` is not supported: it would be pending two ways with one value | fixtures/reject/src/lib.rs:121:20 |
| 2 | `_` is not supported: its `None` and `Some` would both be `rt.None` | fixtures/reject/src/lib.rs:23:15 |
| 2 | `_` of `_`, whose `None` is `rt.None`, which is not supported yet | fixtures/reject/src/lib.rs:48:42 |
| 2 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | fixtures/reject/src/lib.rs:268:16 |
| 2 | a coercion to `_` | fixtures/reject/src/lib.rs:362:2 |
| 2 | an iterator of the crate implementing `_`, which is not supported yet | fixtures/reject/src/lib.rs:175:1 |
| 2 | constructing `_` from another crate | fixtures/reject/src/lib.rs:161:23 |
| 2 | overriding `_`, which std's adaptors call in place of `next` or beside it | fixtures/reject/src/lib.rs:155:2 |
| 2 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | fixtures/reject/src/lib.rs:411:13 |
| 1 | `Pointer` formatting, as in `{:p}`, is not supported | fixtures/reject/src/lib.rs:84:2 |
| 1 | `_` is not supported yet | fixtures/reject/src/lib.rs:435:18 |
| 1 | `_` is not supported: its field `_` is `_`, typed `_`, whose `None` would be `rt.None` as `_` | fixtures/reject/src/lib.rs:38:2 |
| 1 | `_` of `_` by `_`, which is not supported yet | fixtures/reject/src/lib.rs:307:6 |
| 1 | `_` of `_`, which drops `_`, which is not supported yet | fixtures/reject/src/lib.rs:69:13 |
| 1 | `_` of `_`, which folds a closure that drops beside what else drops, which is not supported yet | fixtures/reject/src/lib.rs:63:23 |
| 1 | `_` through a held type's own, which is not supported yet | fixtures/reject/src/lib.rs:339:2 |
| 1 | `_`, which is not supported yet | fixtures/reject/src/lib.rs:315:9 |
| 1 | `f64::mul_add`, which rounds once, where JS would round twice | fixtures/reject/src/lib.rs:498:4 |
| 1 | `fold` of `_`, which folds a `flatten` whose source and iterators both drop, which is not supported yet | fixtures/reject/src/lib.rs:237:26 |
| 1 | a `zip` of `_` and `_`, which std reads by index, of items that drop, which is not supported yet | fixtures/reject/src/lib.rs:242:30 |
| 1 | a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` | fixtures/reject/src/lib.rs:188:51 |
| 1 | a cast from `_` to `_` | fixtures/reject/src/lib.rs:112:2 |
| 1 | a constant of type `_` | fixtures/reject/src/lib.rs:96:2 |
| 1 | a copy of a closure that changes what it captured | fixtures/reject/src/lib.rs:278:14 |
| 1 | a mutable borrow of a subslice of `_`, which JS would copy, having no views | fixtures/reject/src/lib.rs:247:13 |
| 1 | a mutable reference into a `Result`'s payload, passed on or kept | fixtures/reject/src/lib.rs:467:12 |
| 1 | a printable type must implement `Display` or `Debug` | fixtures/reject/src/lib.rs:108:1 |
| 1 | a scalar constant of type `_` | fixtures/reject/src/lib.rs:8:2 |
| 1 | a zero-sized constant of type `_` | fixtures/reject/src/lib.rs:78:2 |
| 1 | arithmetic on `_` | fixtures/reject/src/lib.rs:8:2 |
| 1 | collecting `_`, whose `None` is `rt.None`, which is not supported yet | fixtures/reject/src/lib.rs:43:43 |
| 1 | comparing `_` as a key, which is not supported yet | fixtures/reject/src/lib.rs:289:52 |
| 1 | constructing `_` | fixtures/reject/src/lib.rs:442:53 |
| 1 | hashing `_` as a key, which is not supported yet | fixtures/reject/src/lib.rs:289:52 |
| 1 | hashing `_` through `_`'s own `Hash::hash_slice`, which is not supported yet | fixtures/reject/src/lib.rs:402:16 |
| 1 | passing `_`, which drops, to `_`, which is not supported yet | fixtures/reject/src/lib.rs:419:4 |

## By API

The type or module each refused type or path belongs to.

| Count | API | First |
| ---: | --- | --- |
| 7 | `std::borrow::Cow` | fixtures/reject/src/lib.rs:424:2 |
| 6 | `Option` | fixtures/reject/src/lib.rs:23:15 |
| 6 | `f32` | fixtures/reject/src/lib.rs:7:14 |
| 4 | `Iterator` | fixtures/reject/src/lib.rs:48:42 |
| 4 | `std::sync::Arc` | fixtures/reject/src/lib.rs:117:2 |
| 3 | `PartialOrd` | fixtures/reject/src/lib.rs:381:4 |
| 3 | `std::iter::Map` | fixtures/reject/src/lib.rs:63:23 |
| 3 | `std::slice` | fixtures/reject/src/lib.rs:28:4 |
| 3 | `std::vec::Vec` | fixtures/reject/src/lib.rs:289:52 |
| 3 | `u8` | fixtures/reject/src/lib.rs:96:2 |
| 3 | `{closure}` | fixtures/reject/src/lib.rs:411:13 |
| 2 | `HashMap` | fixtures/reject/src/lib.rs:284:6 |
| 2 | `Lopsided` | fixtures/reject/src/lib.rs:381:4 |
| 2 | `Meters` | fixtures/reject/src/lib.rs:101:2 |
| 2 | `Poll` | fixtures/reject/src/lib.rs:121:20 |
| 2 | `std::collections::HashMap` | fixtures/reject/src/lib.rs:69:13 |
| 2 | `std::iter::Iterator` | fixtures/reject/src/lib.rs:268:16 |
| 2 | `std::ops::Range` | fixtures/reject/src/lib.rs:196:8 |
| 2 | `std::pin::Pin` | fixtures/reject/src/lib.rs:132:2 |
| 2 | `std::vec::IntoIter` | fixtures/reject/src/lib.rs:493:17 |
| 1 | `(i32, &Lopsided)` | fixtures/reject/src/lib.rs:382:13 |
| 1 | `Batched` | fixtures/reject/src/lib.rs:402:16 |
| 1 | `DoubleEndedIterator` | fixtures/reject/src/lib.rs:175:1 |
| 1 | `Entry` | fixtures/reject/src/lib.rs:411:13 |
| 1 | `FusedIterator` | fixtures/reject/src/lib.rs:229:1 |
| 1 | `Hasher` | fixtures/reject/src/lib.rs:315:9 |
| 1 | `Maybe` | fixtures/reject/src/lib.rs:38:2 |
| 1 | `Noisy` | fixtures/reject/src/lib.rs:69:13 |
| 1 | `T \| undefined` | fixtures/reject/src/lib.rs:38:2 |
| 1 | `[u16; 1]` | fixtures/reject/src/lib.rs:261:22 |
| 1 | `count` | fixtures/reject/src/lib.rs:63:23 |
| 1 | `dyn std::ops::Fn(u8) -> u8` | fixtures/reject/src/lib.rs:11:16 |
| 1 | `f64` | fixtures/reject/src/lib.rs:503:2 |
| 1 | `into_keys` | fixtures/reject/src/lib.rs:69:13 |
| 1 | `std::collections::hash_map::Entry` | fixtures/reject/src/lib.rs:457:15 |
| 1 | `std::iter::Chain` | fixtures/reject/src/lib.rs:188:51 |
| 1 | `std::iter::Flatten` | fixtures/reject/src/lib.rs:237:26 |
| 1 | `std::iter::IntoIterator` | fixtures/reject/src/lib.rs:17:2 |
| 1 | `std::iter::Rev` | fixtures/reject/src/lib.rs:182:14 |
| 1 | `std::option::Option` | fixtures/reject/src/lib.rs:474:2 |
| 1 | `std::slice::Iter` | fixtures/reject/src/lib.rs:142:11 |
| 1 | `std::string::String` | fixtures/reject/src/lib.rs:90:10 |
| 1 | `std::sync::MutexGuard` | fixtures/reject/src/lib.rs:137:2 |
| 1 | `str` | fixtures/reject/src/lib.rs:419:4 |
| 1 | `u32` | fixtures/reject/src/lib.rs:247:13 |
| 1 | `u64` | fixtures/reject/src/lib.rs:112:2 |
| 1 | `value` | fixtures/reject/src/lib.rs:38:2 |
| 1 | `{async closure}` | fixtures/reject/src/lib.rs:78:2 |

## By type or path

| Count | Construct | Type or path | First |
| ---: | --- | --- | --- |
| 4 | a variant of `_`, which is not supported: either is the same JS string | `std::borrow::Cow` | fixtures/reject/src/lib.rs:424:2 |
| 3 | unsupported type `_` | `f32` | fixtures/reject/src/lib.rs:7:14 |
| 2 | `_` is not supported: it would be pending two ways with one value | `Poll` | fixtures/reject/src/lib.rs:121:20 |
| 2 | `_` is not supported: its `None` and `Some` would both be `rt.None` | `Option` | fixtures/reject/src/lib.rs:23:15 |
| 2 | a coercion to `_` | `std::sync::Arc` | fixtures/reject/src/lib.rs:362:2 |
| 2 | an assignment through a reference to `_` | `std::pin::Pin` | fixtures/reject/src/lib.rs:132:2 |
| 2 | comparing `_` through its own `_`, which is not supported yet | `Lopsided`, `PartialOrd::lt` | fixtures/reject/src/lib.rs:381:4 |
| 2 | replacing the value behind a reference to `_` | `Meters` | fixtures/reject/src/lib.rs:101:2 |
| 2 | unsupported call to `_` | `<std::vec::IntoIter as std::iter::Iterator>::sum` | fixtures/reject/src/lib.rs:493:17 |
| 1 | `_` is not supported yet | `std::borrow::Cow` | fixtures/reject/src/lib.rs:435:18 |
| 1 | `_` is not supported: its field `_` is `_`, typed `_`, whose `None` would be `rt.None` as `_` | `Maybe`, `value`, `Option`, `T \| undefined`, `Option` | fixtures/reject/src/lib.rs:38:2 |
| 1 | `_` of `_` by `_`, which is not supported yet | `HashMap::contains_key`, `std::collections::HashMap`, `u8` | fixtures/reject/src/lib.rs:307:6 |
| 1 | `_` of `_`, which drops `_`, which is not supported yet | `into_keys`, `std::collections::HashMap`, `Noisy` | fixtures/reject/src/lib.rs:69:13 |
| 1 | `_` of `_`, which folds a closure that drops beside what else drops, which is not supported yet | `count`, `std::iter::Map` | fixtures/reject/src/lib.rs:63:23 |
| 1 | `_` of `_`, whose `None` is `rt.None`, which is not supported yet | `HashMap::get`, `Option` | fixtures/reject/src/lib.rs:284:6 |
| 1 | `_` of `_`, whose `None` is `rt.None`, which is not supported yet | `Iterator::find`, `Option` | fixtures/reject/src/lib.rs:48:42 |
| 1 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | `std::iter::Iterator::count` | fixtures/reject/src/lib.rs:268:38 |
| 1 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | `std::iter::Iterator::filter` | fixtures/reject/src/lib.rs:268:16 |
| 1 | `_` through a held type's own, which is not supported yet | `<std::sync::Arc as PartialEq>::ne` | fixtures/reject/src/lib.rs:339:2 |
| 1 | `_` with a mutable reference to a value JS copies, which its mapping would copy | `std::collections::hash_map::Entry::and_modify` | fixtures/reject/src/lib.rs:457:15 |
| 1 | `_` with a mutable reference to a value JS copies, which its mapping would copy | `std::iter::IntoIterator::into_iter` | fixtures/reject/src/lib.rs:17:2 |
| 1 | `_` with a mutable reference to a value JS copies, which its mapping would copy | `std::slice::last_mut` | fixtures/reject/src/lib.rs:450:21 |
| 1 | `_`, which is not supported yet | `Hasher::write` | fixtures/reject/src/lib.rs:315:9 |
| 1 | `fold` of `_`, which folds a `flatten` whose source and iterators both drop, which is not supported yet | `std::iter::Flatten` | fixtures/reject/src/lib.rs:237:26 |
| 1 | a `zip` of `_` and `_`, which std reads by index, of items that drop, which is not supported yet | `std::iter::Map`, `std::ops::Range` | fixtures/reject/src/lib.rs:242:30 |
| 1 | a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` | `std::iter::Chain` | fixtures/reject/src/lib.rs:188:51 |
| 1 | a cast from `_` to `_` | `u64`, `f32` | fixtures/reject/src/lib.rs:112:2 |
| 1 | a constant of type `_` | `&u8` | fixtures/reject/src/lib.rs:96:2 |
| 1 | a mutable borrow of a subslice of `_`, which JS would copy, having no views | `u32` | fixtures/reject/src/lib.rs:247:13 |
| 1 | a scalar constant of type `_` | `f32` | fixtures/reject/src/lib.rs:8:2 |
| 1 | a zero-sized constant of type `_` | `{async closure}` | fixtures/reject/src/lib.rs:78:2 |
| 1 | an assignment through a reference to `_` | `std::sync::Arc` | fixtures/reject/src/lib.rs:117:2 |
| 1 | an assignment through a reference to `_` | `std::sync::MutexGuard` | fixtures/reject/src/lib.rs:137:2 |
| 1 | an iterator of the crate implementing `_`, which is not supported yet | `DoubleEndedIterator` | fixtures/reject/src/lib.rs:175:1 |
| 1 | an iterator of the crate implementing `_`, which is not supported yet | `FusedIterator` | fixtures/reject/src/lib.rs:229:1 |
| 1 | arithmetic on `_` | `f32` | fixtures/reject/src/lib.rs:8:2 |
| 1 | collecting `_`, whose `None` is `rt.None`, which is not supported yet | `Option` | fixtures/reject/src/lib.rs:43:43 |
| 1 | comparing `_` as a key, which is not supported yet | `std::vec::Vec` | fixtures/reject/src/lib.rs:289:52 |
| 1 | comparing `_` through its own `_`, which is not supported yet | `(i32, &Lopsided)`, `PartialOrd::lt` | fixtures/reject/src/lib.rs:382:13 |
| 1 | constructing `_` | `std::borrow::Cow` | fixtures/reject/src/lib.rs:442:53 |
| 1 | constructing `_` from another crate | `std::iter::Map` | fixtures/reject/src/lib.rs:161:23 |
| 1 | constructing `_` from another crate | `std::iter::Rev` | fixtures/reject/src/lib.rs:182:14 |
| 1 | hashing `_` as a key, which is not supported yet | `std::vec::Vec` | fixtures/reject/src/lib.rs:289:52 |
| 1 | hashing `_` through `_`'s own `Hash::hash_slice`, which is not supported yet | `std::vec::Vec`, `Batched` | fixtures/reject/src/lib.rs:402:16 |
| 1 | overriding `_`, which std's adaptors call in place of `next` or beside it | `Iterator::fold` | fixtures/reject/src/lib.rs:155:2 |
| 1 | overriding `_`, which std's adaptors call in place of `next` or beside it | `Iterator::size_hint` | fixtures/reject/src/lib.rs:209:2 |
| 1 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | `{closure}`, `Entry::or_insert_with` | fixtures/reject/src/lib.rs:411:13 |
| 1 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | `{closure}`, `Iterator::for_each` | fixtures/reject/src/lib.rs:412:16 |
| 1 | passing `_`, which drops, to `_`, which is not supported yet | `{closure}`, `str::contains` | fixtures/reject/src/lib.rs:419:4 |
| 1 | replacing the value behind a reference to `_` | `[u16; 1]` | fixtures/reject/src/lib.rs:261:22 |
| 1 | unsupported call to `_` | `<f64 as std::fmt::Display>::fmt` | fixtures/reject/src/lib.rs:503:2 |
| 1 | unsupported call to `_` | `<std::borrow::Cow as std::ops::Deref>::deref` | fixtures/reject/src/lib.rs:436:2 |
| 1 | unsupported call to `_` | `<std::option::Option as std::cmp::PartialEq>::eq` | fixtures/reject/src/lib.rs:474:2 |
| 1 | unsupported call to `_` | `<std::slice::Iter as std::iter::Iterator>::cycle` | fixtures/reject/src/lib.rs:142:11 |
| 1 | unsupported call to `_` | `<std::string::String as std::fmt::Write>::write_fmt` | fixtures/reject/src/lib.rs:90:10 |
| 1 | unsupported call to `_` | `<u8 as std::hash::Hash>::hash` | fixtures/reject/src/lib.rs:393:10 |
| 1 | unsupported call to `_` | `std::ops::Range::contains` | fixtures/reject/src/lib.rs:196:8 |
| 1 | unsupported call to `_` | `std::slice::copy_within` | fixtures/reject/src/lib.rs:357:4 |
| 1 | unsupported call to `_` | `std::slice::sort` | fixtures/reject/src/lib.rs:28:4 |
| 1 | unsupported type `_` | `dyn std::ops::Fn(u8) -> u8` | fixtures/reject/src/lib.rs:11:16 |

## Spans

<details><summary>unsupported call to `_` (11)</summary>

- fixtures/reject/src/lib.rs:28:4: unsupported call to `std::slice::<impl [u8]>::sort`
- fixtures/reject/src/lib.rs:90:10: unsupported call to `<std::string::String as std::fmt::Write>::write_fmt`
- fixtures/reject/src/lib.rs:142:11: unsupported call to `<std::slice::Iter<'_, u8> as std::iter::Iterator>::cycle`
- fixtures/reject/src/lib.rs:196:8: unsupported call to `std::ops::Range::<Key>::contains::<Key>`
- fixtures/reject/src/lib.rs:357:4: unsupported call to `core::slice::<impl [(u8, u8)]>::copy_within::<std::ops::Range<usize>>`
- fixtures/reject/src/lib.rs:393:10: unsupported call to `<u8 as std::hash::Hash>::hash::<H>`
- fixtures/reject/src/lib.rs:436:2: unsupported call to `<std::borrow::Cow<'_, [u8]> as std::ops::Deref>::deref`
- fixtures/reject/src/lib.rs:474:2: unsupported call to `<std::option::Option<&mut u32> as std::cmp::PartialEq>::eq`
- fixtures/reject/src/lib.rs:493:17: unsupported call to `<std::vec::IntoIter<Part> as std::iter::Iterator>::sum::<std::time::Duration>`
- fixtures/reject/src/lib.rs:493:38: unsupported call to `<std::vec::IntoIter<Part> as std::iter::Iterator>::sum::<u64>`
- fixtures/reject/src/lib.rs:503:2: unsupported call to `<f64 as std::fmt::Display>::fmt`

</details>

<details><summary>a variant of `_`, which is not supported: either is the same JS string (4)</summary>

- fixtures/reject/src/lib.rs:424:2: a variant of `std::borrow::Cow<'_, str>`, which is not supported: either is the same JS string
- fixtures/reject/src/lib.rs:425:30: a variant of `std::borrow::Cow<'_, str>`, which is not supported: either is the same JS string
- fixtures/reject/src/lib.rs:426:27: a variant of `std::borrow::Cow<'_, str>`, which is not supported: either is the same JS string
- fixtures/reject/src/lib.rs:431:2: a variant of `std::borrow::Cow<'_, str>`, which is not supported: either is the same JS string

</details>

<details><summary>an assignment through a reference to `_` (4)</summary>

- fixtures/reject/src/lib.rs:117:2: an assignment through a reference to `std::sync::Arc<u8>`
- fixtures/reject/src/lib.rs:132:2: an assignment through a reference to `std::pin::Pin<std::sync::Arc<u8>>`
- fixtures/reject/src/lib.rs:137:2: an assignment through a reference to `std::sync::MutexGuard<'_, u8>`
- fixtures/reject/src/lib.rs:255:2: an assignment through a reference to `std::pin::Pin<&mut Noisy>`

</details>

<details><summary>unsupported type `_` (4)</summary>

- fixtures/reject/src/lib.rs:7:14: unsupported type `f32`
- fixtures/reject/src/lib.rs:7:25: unsupported type `f32`
- fixtures/reject/src/lib.rs:11:16: unsupported type `dyn std::ops::Fn(u8) -> u8`
- fixtures/reject/src/lib.rs:110:51: unsupported type `f32`

</details>

<details><summary>`_` with a mutable reference to a value JS copies, which its mapping would copy (3)</summary>

- fixtures/reject/src/lib.rs:17:2: `std::iter::IntoIterator::into_iter` with a mutable reference to a value JS copies, which its mapping would copy
- fixtures/reject/src/lib.rs:450:21: `core::slice::<impl [T]>::last_mut` with a mutable reference to a value JS copies, which its mapping would copy
- fixtures/reject/src/lib.rs:457:15: `std::collections::hash_map::Entry::<'a, K, V, A>::and_modify` with a mutable reference to a value JS copies, which its mapping would copy

</details>

<details><summary>comparing `_` through its own `_`, which is not supported yet (3)</summary>

- fixtures/reject/src/lib.rs:381:4: comparing `Lopsided` through its own `PartialOrd::lt`, which is not supported yet
- fixtures/reject/src/lib.rs:382:13: comparing `(i32, &Lopsided)` through its own `PartialOrd::lt`, which is not supported yet
- fixtures/reject/src/lib.rs:383:12: comparing `Lopsided` through its own `PartialOrd::lt`, which is not supported yet

</details>

<details><summary>replacing the value behind a reference to `_` (3)</summary>

- fixtures/reject/src/lib.rs:101:2: replacing the value behind a reference to `Meters`
- fixtures/reject/src/lib.rs:105:2: replacing the value behind a reference to `Meters`
- fixtures/reject/src/lib.rs:261:22: replacing the value behind a reference to `[u16; 1]`

</details>

<details><summary>`_` is not supported: it would be pending two ways with one value (2)</summary>

- fixtures/reject/src/lib.rs:121:20: `Poll<std::option::Option<std::task::Poll<u8>>>` is not supported: it would be pending two ways with one value
- fixtures/reject/src/lib.rs:126:54: `Poll<std::task::Poll<u8>>` is not supported: it would be pending two ways with one value

</details>

<details><summary>`_` is not supported: its `None` and `Some` would both be `rt.None` (2)</summary>

- fixtures/reject/src/lib.rs:23:15: `Option<std::option::Option<std::option::Option<u8>>>` is not supported: its `None` and `Some` would both be `rt.None`
- fixtures/reject/src/lib.rs:24:2: `Option<std::option::Option<std::option::Option<u8>>>` is not supported: its `None` and `Some` would both be `rt.None`

</details>

<details><summary>`_` of `_`, whose `None` is `rt.None`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:48:42: `Iterator::find` of `Option<std::option::Option<u8>>`, whose `None` is `rt.None`, which is not supported yet
- fixtures/reject/src/lib.rs:284:6: `HashMap::get` of `Option<&std::option::Option<u8>>`, whose `None` is `rt.None`, which is not supported yet

</details>

<details><summary>`_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it (2)</summary>

- fixtures/reject/src/lib.rs:268:16: `std::iter::Iterator::filter` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it
- fixtures/reject/src/lib.rs:268:38: `std::iter::Iterator::count` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it

</details>

<details><summary>a coercion to `_` (2)</summary>

- fixtures/reject/src/lib.rs:362:2: a coercion to `std::sync::Arc<[u8]>`
- fixtures/reject/src/lib.rs:509:2: a coercion to `std::sync::Arc<dyn std::future::Future<Output = u8>>`

</details>

<details><summary>an iterator of the crate implementing `_`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:175:1: an iterator of the crate implementing `DoubleEndedIterator`, which is not supported yet
- fixtures/reject/src/lib.rs:229:1: an iterator of the crate implementing `FusedIterator`, which is not supported yet

</details>

<details><summary>constructing `_` from another crate (2)</summary>

- fixtures/reject/src/lib.rs:161:23: constructing `std::iter::Map<Folds, {closure@fixtures/reject/src/lib.rs:161:12: 161:15}>` from another crate
- fixtures/reject/src/lib.rs:182:14: constructing `std::iter::Rev<Backs>` from another crate

</details>

<details><summary>overriding `_`, which std's adaptors call in place of `next` or beside it (2)</summary>

- fixtures/reject/src/lib.rs:155:2: overriding `Iterator::fold`, which std's adaptors call in place of `next` or beside it
- fixtures/reject/src/lib.rs:209:2: overriding `Iterator::size_hint`, which std's adaptors call in place of `next` or beside it

</details>

<details><summary>passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:411:13: passing `{closure@fixtures/reject/src/lib.rs:411:28: 411:35}`, a closure that owns a value that drops, to `Entry::or_insert_with`, which is not supported yet
- fixtures/reject/src/lib.rs:412:16: passing `{closure@fixtures/reject/src/lib.rs:412:25: 412:33}`, a closure that owns a value that drops, to `Iterator::for_each`, which is not supported yet

</details>

<details><summary>`Pointer` formatting, as in `{:p}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:84:2: `Pointer` formatting, as in `{:p}`, is not supported

</details>

<details><summary>`_` is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:435:18: `std::borrow::Cow<'_, [u8]>` is not supported yet

</details>

<details><summary>`_` is not supported: its field `_` is `_`, typed `_`, whose `None` would be `rt.None` as `_` (1)</summary>

- fixtures/reject/src/lib.rs:38:2: `Maybe<()>` is not supported: its field `value` is `Option<T>`, typed `T | undefined`, whose `None` would be `rt.None` as `Option<()>`

</details>

<details><summary>`_` of `_` by `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:307:6: `HashMap::contains_key` of `std::collections::HashMap<Id, u8>` by `u8`, which is not supported yet

</details>

<details><summary>`_` of `_`, which drops `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:69:13: `into_keys` of `std::collections::HashMap<u8, Noisy>`, which drops `Noisy`, which is not supported yet

</details>

<details><summary>`_` of `_`, which folds a closure that drops beside what else drops, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:63:23: `count` of `std::iter::Map<std::vec::IntoIter<Noisy>, {closure@fixtures/reject/src/lib.rs:62:10: 62:25}>`, which folds a closure that drops beside what else drops, which is not supported yet

</details>

<details><summary>`_` through a held type's own, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:339:2: `<std::sync::Arc<Picky> as PartialEq>::ne` through a held type's own, which is not supported yet

</details>

<details><summary>`_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:315:9: `Hasher::write`, which is not supported yet

</details>

<details><summary>`f64::mul_add`, which rounds once, where JS would round twice (1)</summary>

- fixtures/reject/src/lib.rs:498:4: `f64::mul_add`, which rounds once, where JS would round twice

</details>

<details><summary>`fold` of `_`, which folds a `flatten` whose source and iterators both drop, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:237:26: `fold` of `std::iter::Flatten<std::vec::IntoIter<std::vec::Vec<Noisy>>>`, which folds a `flatten` whose source and iterators both drop, which is not supported yet

</details>

<details><summary>a `zip` of `_` and `_`, which std reads by index, of items that drop, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:242:30: a `zip` of `std::iter::Map<std::slice::Iter<'_, u8>, {closure@fixtures/reject/src/lib.rs:242:15: 242:19}>` and `std::ops::Range<i32>`, which std reads by index, of items that drop, which is not supported yet

</details>

<details><summary>a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` (1)</summary>

- fixtures/reject/src/lib.rs:188:51: a `zip` of `std::iter::Chain<std::iter::Map<std::slice::Iter<'_, u8>, {closure@fixtures/reject/src/lib.rs:188:15: 188:18}>, std::iter::Copied<std::slice::Iter<'_, u8>>>`, whose closures std calls one item further through `next` than through `fold`

</details>

<details><summary>a cast from `_` to `_` (1)</summary>

- fixtures/reject/src/lib.rs:112:2: a cast from `u64` to `f32`

</details>

<details><summary>a constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:96:2: a constant of type `&u8`

</details>

<details><summary>a copy of a closure that changes what it captured (1)</summary>

- fixtures/reject/src/lib.rs:278:14: a copy of a closure that changes what it captured

</details>

<details><summary>a mutable borrow of a subslice of `_`, which JS would copy, having no views (1)</summary>

- fixtures/reject/src/lib.rs:247:13: a mutable borrow of a subslice of `u32`, which JS would copy, having no views

</details>

<details><summary>a mutable reference into a `Result`'s payload, passed on or kept (1)</summary>

- fixtures/reject/src/lib.rs:467:12: a mutable reference into a `Result`'s payload, passed on or kept

</details>

<details><summary>a printable type must implement `Display` or `Debug` (1)</summary>

- fixtures/reject/src/lib.rs:108:1: a printable type must implement `Display` or `Debug`

</details>

<details><summary>a scalar constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:8:2: a scalar constant of type `f32`

</details>

<details><summary>a zero-sized constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:78:2: a zero-sized constant of type `{async closure@fixtures/reject/src/lib.rs:77:10: 77:18}`

</details>

<details><summary>arithmetic on `_` (1)</summary>

- fixtures/reject/src/lib.rs:8:2: arithmetic on `f32`

</details>

<details><summary>collecting `_`, whose `None` is `rt.None`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:43:43: collecting `Option<()>`, whose `None` is `rt.None`, which is not supported yet

</details>

<details><summary>comparing `_` as a key, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:289:52: comparing `std::vec::Vec<u8>` as a key, which is not supported yet

</details>

<details><summary>constructing `_` (1)</summary>

- fixtures/reject/src/lib.rs:442:53: constructing `std::borrow::Cow<'_, std::sync::Arc<u8>>`

</details>

<details><summary>hashing `_` as a key, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:289:52: hashing `std::vec::Vec<u8>` as a key, which is not supported yet

</details>

<details><summary>hashing `_` through `_`'s own `Hash::hash_slice`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:402:16: hashing `std::vec::Vec<Batched>` through `Batched`'s own `Hash::hash_slice`, which is not supported yet

</details>

<details><summary>passing `_`, which drops, to `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:419:4: passing `{closure@fixtures/reject/src/lib.rs:419:13: 419:27}`, which drops, to `str::contains`, which is not supported yet

</details>
