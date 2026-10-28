# Census of `reject-fixture`

- Refusals: 74
- Function instances: 80
- Generic functions no instance reached, lowered over their type parameters: 6

## By construct

| Count | Construct | First |
| ---: | --- | --- |
| 11 | unsupported call to `_` | fixtures/reject/src/lib.rs:28:4 |
| 4 | a variant of `_`, which is not supported: either is the same JS string | fixtures/reject/src/lib.rs:410:2 |
| 4 | an assignment through a reference to `_` | fixtures/reject/src/lib.rs:108:2 |
| 4 | unsupported type `_` | fixtures/reject/src/lib.rs:7:14 |
| 3 | `_` with a mutable reference to a value JS copies, which its mapping would copy | fixtures/reject/src/lib.rs:17:2 |
| 3 | comparing `_` through its own `_`, which is not supported yet | fixtures/reject/src/lib.rs:367:4 |
| 3 | replacing the value behind a reference to `_` | fixtures/reject/src/lib.rs:92:2 |
| 2 | `_` is not supported: it would be pending two ways with one value | fixtures/reject/src/lib.rs:112:20 |
| 2 | `_` is not supported: its `None` and `Some` would both be `rt.None` | fixtures/reject/src/lib.rs:23:15 |
| 2 | `_` of `_`, whose `None` is `rt.None`, which is not supported yet | fixtures/reject/src/lib.rs:48:42 |
| 2 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | fixtures/reject/src/lib.rs:254:16 |
| 2 | an iterator of the crate implementing `_`, which is not supported yet | fixtures/reject/src/lib.rs:166:1 |
| 2 | constructing `_` from another crate | fixtures/reject/src/lib.rs:152:23 |
| 2 | overriding `_`, which std's adaptors call in place of `next` or beside it | fixtures/reject/src/lib.rs:146:2 |
| 2 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | fixtures/reject/src/lib.rs:397:13 |
| 1 | `Pointer` formatting, as in `{:p}`, is not supported | fixtures/reject/src/lib.rs:75:2 |
| 1 | `_` is not supported yet | fixtures/reject/src/lib.rs:421:18 |
| 1 | `_` is not supported: its field `_` is `_`, typed `_`, whose `None` would be `rt.None` as `_` | fixtures/reject/src/lib.rs:38:2 |
| 1 | `_` of `_` by `_`, which is not supported yet | fixtures/reject/src/lib.rs:293:6 |
| 1 | `_` of `_`, which drops `_`, which is not supported yet | fixtures/reject/src/lib.rs:60:13 |
| 1 | `_` through a held type's own, which is not supported yet | fixtures/reject/src/lib.rs:325:2 |
| 1 | `_`, which is not supported yet | fixtures/reject/src/lib.rs:301:9 |
| 1 | `f64::mul_add`, which rounds once, where JS would round twice | fixtures/reject/src/lib.rs:484:4 |
| 1 | a `zip` of `_` and `_`, which std reads by index, of items that drop, which is not supported yet | fixtures/reject/src/lib.rs:228:30 |
| 1 | a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` | fixtures/reject/src/lib.rs:179:51 |
| 1 | a cast from `_` to `_` | fixtures/reject/src/lib.rs:103:2 |
| 1 | a coercion to `_` | fixtures/reject/src/lib.rs:348:2 |
| 1 | a constant of type `_` | fixtures/reject/src/lib.rs:87:2 |
| 1 | a copy of a closure that changes what it captured | fixtures/reject/src/lib.rs:264:14 |
| 1 | a mutable borrow of a subslice of `_`, which JS would copy, having no views | fixtures/reject/src/lib.rs:233:13 |
| 1 | a mutable reference into a `Result`'s payload, passed on or kept | fixtures/reject/src/lib.rs:453:12 |
| 1 | a printable type must implement `Display` or `Debug` | fixtures/reject/src/lib.rs:99:1 |
| 1 | a scalar constant of type `_` | fixtures/reject/src/lib.rs:8:2 |
| 1 | a zero-sized constant of type `_` | fixtures/reject/src/lib.rs:69:2 |
| 1 | arithmetic on `_` | fixtures/reject/src/lib.rs:8:2 |
| 1 | collecting `_`, whose `None` is `rt.None`, which is not supported yet | fixtures/reject/src/lib.rs:43:43 |
| 1 | comparing `_` as a key, which is not supported yet | fixtures/reject/src/lib.rs:275:52 |
| 1 | constructing `_` | fixtures/reject/src/lib.rs:428:53 |
| 1 | hashing `_` as a key, which is not supported yet | fixtures/reject/src/lib.rs:275:52 |
| 1 | hashing `_` through `_`'s own `Hash::hash_slice`, which is not supported yet | fixtures/reject/src/lib.rs:388:16 |
| 1 | passing `_`, which drops, to `_`, which is not supported yet | fixtures/reject/src/lib.rs:405:4 |

## By API

The type or module each refused type or path belongs to.

| Count | API | First |
| ---: | --- | --- |
| 7 | `std::borrow::Cow` | fixtures/reject/src/lib.rs:410:2 |
| 6 | `Option` | fixtures/reject/src/lib.rs:23:15 |
| 6 | `f32` | fixtures/reject/src/lib.rs:7:14 |
| 4 | `Iterator` | fixtures/reject/src/lib.rs:48:42 |
| 3 | `PartialOrd` | fixtures/reject/src/lib.rs:367:4 |
| 3 | `std::slice` | fixtures/reject/src/lib.rs:28:4 |
| 3 | `std::sync::Arc` | fixtures/reject/src/lib.rs:108:2 |
| 3 | `std::vec::Vec` | fixtures/reject/src/lib.rs:275:52 |
| 3 | `u8` | fixtures/reject/src/lib.rs:87:2 |
| 3 | `{closure}` | fixtures/reject/src/lib.rs:397:13 |
| 2 | `HashMap` | fixtures/reject/src/lib.rs:270:6 |
| 2 | `Lopsided` | fixtures/reject/src/lib.rs:367:4 |
| 2 | `Meters` | fixtures/reject/src/lib.rs:92:2 |
| 2 | `Poll` | fixtures/reject/src/lib.rs:112:20 |
| 2 | `std::collections::HashMap` | fixtures/reject/src/lib.rs:60:13 |
| 2 | `std::iter::Iterator` | fixtures/reject/src/lib.rs:254:16 |
| 2 | `std::iter::Map` | fixtures/reject/src/lib.rs:152:23 |
| 2 | `std::ops::Range` | fixtures/reject/src/lib.rs:187:8 |
| 2 | `std::pin::Pin` | fixtures/reject/src/lib.rs:123:2 |
| 2 | `std::vec::IntoIter` | fixtures/reject/src/lib.rs:479:17 |
| 1 | `(i32, &Lopsided)` | fixtures/reject/src/lib.rs:368:13 |
| 1 | `Batched` | fixtures/reject/src/lib.rs:388:16 |
| 1 | `DoubleEndedIterator` | fixtures/reject/src/lib.rs:166:1 |
| 1 | `Entry` | fixtures/reject/src/lib.rs:397:13 |
| 1 | `FusedIterator` | fixtures/reject/src/lib.rs:220:1 |
| 1 | `Hasher` | fixtures/reject/src/lib.rs:301:9 |
| 1 | `Maybe` | fixtures/reject/src/lib.rs:38:2 |
| 1 | `Noisy` | fixtures/reject/src/lib.rs:60:13 |
| 1 | `T \| undefined` | fixtures/reject/src/lib.rs:38:2 |
| 1 | `[u16; 1]` | fixtures/reject/src/lib.rs:247:22 |
| 1 | `dyn std::ops::Fn(u8) -> u8` | fixtures/reject/src/lib.rs:11:16 |
| 1 | `f64` | fixtures/reject/src/lib.rs:489:2 |
| 1 | `into_keys` | fixtures/reject/src/lib.rs:60:13 |
| 1 | `std::collections::hash_map::Entry` | fixtures/reject/src/lib.rs:443:15 |
| 1 | `std::iter::Chain` | fixtures/reject/src/lib.rs:179:51 |
| 1 | `std::iter::IntoIterator` | fixtures/reject/src/lib.rs:17:2 |
| 1 | `std::iter::Rev` | fixtures/reject/src/lib.rs:173:14 |
| 1 | `std::option::Option` | fixtures/reject/src/lib.rs:460:2 |
| 1 | `std::slice::Iter` | fixtures/reject/src/lib.rs:133:11 |
| 1 | `std::string::String` | fixtures/reject/src/lib.rs:81:10 |
| 1 | `std::sync::MutexGuard` | fixtures/reject/src/lib.rs:128:2 |
| 1 | `str` | fixtures/reject/src/lib.rs:405:4 |
| 1 | `u32` | fixtures/reject/src/lib.rs:233:13 |
| 1 | `u64` | fixtures/reject/src/lib.rs:103:2 |
| 1 | `value` | fixtures/reject/src/lib.rs:38:2 |
| 1 | `{async closure}` | fixtures/reject/src/lib.rs:69:2 |

## By type or path

| Count | Construct | Type or path | First |
| ---: | --- | --- | --- |
| 4 | a variant of `_`, which is not supported: either is the same JS string | `std::borrow::Cow` | fixtures/reject/src/lib.rs:410:2 |
| 3 | unsupported type `_` | `f32` | fixtures/reject/src/lib.rs:7:14 |
| 2 | `_` is not supported: it would be pending two ways with one value | `Poll` | fixtures/reject/src/lib.rs:112:20 |
| 2 | `_` is not supported: its `None` and `Some` would both be `rt.None` | `Option` | fixtures/reject/src/lib.rs:23:15 |
| 2 | an assignment through a reference to `_` | `std::pin::Pin` | fixtures/reject/src/lib.rs:123:2 |
| 2 | comparing `_` through its own `_`, which is not supported yet | `Lopsided`, `PartialOrd::lt` | fixtures/reject/src/lib.rs:367:4 |
| 2 | replacing the value behind a reference to `_` | `Meters` | fixtures/reject/src/lib.rs:92:2 |
| 2 | unsupported call to `_` | `<std::vec::IntoIter as std::iter::Iterator>::sum` | fixtures/reject/src/lib.rs:479:17 |
| 1 | `_` is not supported yet | `std::borrow::Cow` | fixtures/reject/src/lib.rs:421:18 |
| 1 | `_` is not supported: its field `_` is `_`, typed `_`, whose `None` would be `rt.None` as `_` | `Maybe`, `value`, `Option`, `T \| undefined`, `Option` | fixtures/reject/src/lib.rs:38:2 |
| 1 | `_` of `_` by `_`, which is not supported yet | `HashMap::contains_key`, `std::collections::HashMap`, `u8` | fixtures/reject/src/lib.rs:293:6 |
| 1 | `_` of `_`, which drops `_`, which is not supported yet | `into_keys`, `std::collections::HashMap`, `Noisy` | fixtures/reject/src/lib.rs:60:13 |
| 1 | `_` of `_`, whose `None` is `rt.None`, which is not supported yet | `HashMap::get`, `Option` | fixtures/reject/src/lib.rs:270:6 |
| 1 | `_` of `_`, whose `None` is `rt.None`, which is not supported yet | `Iterator::find`, `Option` | fixtures/reject/src/lib.rs:48:42 |
| 1 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | `std::iter::Iterator::count` | fixtures/reject/src/lib.rs:254:38 |
| 1 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | `std::iter::Iterator::filter` | fixtures/reject/src/lib.rs:254:16 |
| 1 | `_` through a held type's own, which is not supported yet | `<std::sync::Arc as PartialEq>::ne` | fixtures/reject/src/lib.rs:325:2 |
| 1 | `_` with a mutable reference to a value JS copies, which its mapping would copy | `std::collections::hash_map::Entry::and_modify` | fixtures/reject/src/lib.rs:443:15 |
| 1 | `_` with a mutable reference to a value JS copies, which its mapping would copy | `std::iter::IntoIterator::into_iter` | fixtures/reject/src/lib.rs:17:2 |
| 1 | `_` with a mutable reference to a value JS copies, which its mapping would copy | `std::slice::last_mut` | fixtures/reject/src/lib.rs:436:21 |
| 1 | `_`, which is not supported yet | `Hasher::write` | fixtures/reject/src/lib.rs:301:9 |
| 1 | a `zip` of `_` and `_`, which std reads by index, of items that drop, which is not supported yet | `std::iter::Map`, `std::ops::Range` | fixtures/reject/src/lib.rs:228:30 |
| 1 | a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` | `std::iter::Chain` | fixtures/reject/src/lib.rs:179:51 |
| 1 | a cast from `_` to `_` | `u64`, `f32` | fixtures/reject/src/lib.rs:103:2 |
| 1 | a coercion to `_` | `std::sync::Arc` | fixtures/reject/src/lib.rs:348:2 |
| 1 | a constant of type `_` | `&u8` | fixtures/reject/src/lib.rs:87:2 |
| 1 | a mutable borrow of a subslice of `_`, which JS would copy, having no views | `u32` | fixtures/reject/src/lib.rs:233:13 |
| 1 | a scalar constant of type `_` | `f32` | fixtures/reject/src/lib.rs:8:2 |
| 1 | a zero-sized constant of type `_` | `{async closure}` | fixtures/reject/src/lib.rs:69:2 |
| 1 | an assignment through a reference to `_` | `std::sync::Arc` | fixtures/reject/src/lib.rs:108:2 |
| 1 | an assignment through a reference to `_` | `std::sync::MutexGuard` | fixtures/reject/src/lib.rs:128:2 |
| 1 | an iterator of the crate implementing `_`, which is not supported yet | `DoubleEndedIterator` | fixtures/reject/src/lib.rs:166:1 |
| 1 | an iterator of the crate implementing `_`, which is not supported yet | `FusedIterator` | fixtures/reject/src/lib.rs:220:1 |
| 1 | arithmetic on `_` | `f32` | fixtures/reject/src/lib.rs:8:2 |
| 1 | collecting `_`, whose `None` is `rt.None`, which is not supported yet | `Option` | fixtures/reject/src/lib.rs:43:43 |
| 1 | comparing `_` as a key, which is not supported yet | `std::vec::Vec` | fixtures/reject/src/lib.rs:275:52 |
| 1 | comparing `_` through its own `_`, which is not supported yet | `(i32, &Lopsided)`, `PartialOrd::lt` | fixtures/reject/src/lib.rs:368:13 |
| 1 | constructing `_` | `std::borrow::Cow` | fixtures/reject/src/lib.rs:428:53 |
| 1 | constructing `_` from another crate | `std::iter::Map` | fixtures/reject/src/lib.rs:152:23 |
| 1 | constructing `_` from another crate | `std::iter::Rev` | fixtures/reject/src/lib.rs:173:14 |
| 1 | hashing `_` as a key, which is not supported yet | `std::vec::Vec` | fixtures/reject/src/lib.rs:275:52 |
| 1 | hashing `_` through `_`'s own `Hash::hash_slice`, which is not supported yet | `std::vec::Vec`, `Batched` | fixtures/reject/src/lib.rs:388:16 |
| 1 | overriding `_`, which std's adaptors call in place of `next` or beside it | `Iterator::fold` | fixtures/reject/src/lib.rs:146:2 |
| 1 | overriding `_`, which std's adaptors call in place of `next` or beside it | `Iterator::size_hint` | fixtures/reject/src/lib.rs:200:2 |
| 1 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | `{closure}`, `Entry::or_insert_with` | fixtures/reject/src/lib.rs:397:13 |
| 1 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | `{closure}`, `Iterator::for_each` | fixtures/reject/src/lib.rs:398:16 |
| 1 | passing `_`, which drops, to `_`, which is not supported yet | `{closure}`, `str::contains` | fixtures/reject/src/lib.rs:405:4 |
| 1 | replacing the value behind a reference to `_` | `[u16; 1]` | fixtures/reject/src/lib.rs:247:22 |
| 1 | unsupported call to `_` | `<f64 as std::fmt::Display>::fmt` | fixtures/reject/src/lib.rs:489:2 |
| 1 | unsupported call to `_` | `<std::borrow::Cow as std::ops::Deref>::deref` | fixtures/reject/src/lib.rs:422:2 |
| 1 | unsupported call to `_` | `<std::option::Option as std::cmp::PartialEq>::eq` | fixtures/reject/src/lib.rs:460:2 |
| 1 | unsupported call to `_` | `<std::slice::Iter as std::iter::Iterator>::cycle` | fixtures/reject/src/lib.rs:133:11 |
| 1 | unsupported call to `_` | `<std::string::String as std::fmt::Write>::write_fmt` | fixtures/reject/src/lib.rs:81:10 |
| 1 | unsupported call to `_` | `<u8 as std::hash::Hash>::hash` | fixtures/reject/src/lib.rs:379:10 |
| 1 | unsupported call to `_` | `std::ops::Range::contains` | fixtures/reject/src/lib.rs:187:8 |
| 1 | unsupported call to `_` | `std::slice::copy_within` | fixtures/reject/src/lib.rs:343:4 |
| 1 | unsupported call to `_` | `std::slice::sort` | fixtures/reject/src/lib.rs:28:4 |
| 1 | unsupported type `_` | `dyn std::ops::Fn(u8) -> u8` | fixtures/reject/src/lib.rs:11:16 |

## Spans

<details><summary>unsupported call to `_` (11)</summary>

- fixtures/reject/src/lib.rs:28:4: unsupported call to `std::slice::<impl [u8]>::sort`
- fixtures/reject/src/lib.rs:81:10: unsupported call to `<std::string::String as std::fmt::Write>::write_fmt`
- fixtures/reject/src/lib.rs:133:11: unsupported call to `<std::slice::Iter<'_, u8> as std::iter::Iterator>::cycle`
- fixtures/reject/src/lib.rs:187:8: unsupported call to `std::ops::Range::<Key>::contains::<Key>`
- fixtures/reject/src/lib.rs:343:4: unsupported call to `core::slice::<impl [(u8, u8)]>::copy_within::<std::ops::Range<usize>>`
- fixtures/reject/src/lib.rs:379:10: unsupported call to `<u8 as std::hash::Hash>::hash::<H>`
- fixtures/reject/src/lib.rs:422:2: unsupported call to `<std::borrow::Cow<'_, [u8]> as std::ops::Deref>::deref`
- fixtures/reject/src/lib.rs:460:2: unsupported call to `<std::option::Option<&mut u32> as std::cmp::PartialEq>::eq`
- fixtures/reject/src/lib.rs:479:17: unsupported call to `<std::vec::IntoIter<Part> as std::iter::Iterator>::sum::<std::time::Duration>`
- fixtures/reject/src/lib.rs:479:38: unsupported call to `<std::vec::IntoIter<Part> as std::iter::Iterator>::sum::<u64>`
- fixtures/reject/src/lib.rs:489:2: unsupported call to `<f64 as std::fmt::Display>::fmt`

</details>

<details><summary>a variant of `_`, which is not supported: either is the same JS string (4)</summary>

- fixtures/reject/src/lib.rs:410:2: a variant of `std::borrow::Cow<'_, str>`, which is not supported: either is the same JS string
- fixtures/reject/src/lib.rs:411:30: a variant of `std::borrow::Cow<'_, str>`, which is not supported: either is the same JS string
- fixtures/reject/src/lib.rs:412:27: a variant of `std::borrow::Cow<'_, str>`, which is not supported: either is the same JS string
- fixtures/reject/src/lib.rs:417:2: a variant of `std::borrow::Cow<'_, str>`, which is not supported: either is the same JS string

</details>

<details><summary>an assignment through a reference to `_` (4)</summary>

- fixtures/reject/src/lib.rs:108:2: an assignment through a reference to `std::sync::Arc<u8>`
- fixtures/reject/src/lib.rs:123:2: an assignment through a reference to `std::pin::Pin<std::sync::Arc<u8>>`
- fixtures/reject/src/lib.rs:128:2: an assignment through a reference to `std::sync::MutexGuard<'_, u8>`
- fixtures/reject/src/lib.rs:241:2: an assignment through a reference to `std::pin::Pin<&mut Noisy>`

</details>

<details><summary>unsupported type `_` (4)</summary>

- fixtures/reject/src/lib.rs:7:14: unsupported type `f32`
- fixtures/reject/src/lib.rs:7:25: unsupported type `f32`
- fixtures/reject/src/lib.rs:11:16: unsupported type `dyn std::ops::Fn(u8) -> u8`
- fixtures/reject/src/lib.rs:101:51: unsupported type `f32`

</details>

<details><summary>`_` with a mutable reference to a value JS copies, which its mapping would copy (3)</summary>

- fixtures/reject/src/lib.rs:17:2: `std::iter::IntoIterator::into_iter` with a mutable reference to a value JS copies, which its mapping would copy
- fixtures/reject/src/lib.rs:436:21: `core::slice::<impl [T]>::last_mut` with a mutable reference to a value JS copies, which its mapping would copy
- fixtures/reject/src/lib.rs:443:15: `std::collections::hash_map::Entry::<'a, K, V, A>::and_modify` with a mutable reference to a value JS copies, which its mapping would copy

</details>

<details><summary>comparing `_` through its own `_`, which is not supported yet (3)</summary>

- fixtures/reject/src/lib.rs:367:4: comparing `Lopsided` through its own `PartialOrd::lt`, which is not supported yet
- fixtures/reject/src/lib.rs:368:13: comparing `(i32, &Lopsided)` through its own `PartialOrd::lt`, which is not supported yet
- fixtures/reject/src/lib.rs:369:12: comparing `Lopsided` through its own `PartialOrd::lt`, which is not supported yet

</details>

<details><summary>replacing the value behind a reference to `_` (3)</summary>

- fixtures/reject/src/lib.rs:92:2: replacing the value behind a reference to `Meters`
- fixtures/reject/src/lib.rs:96:2: replacing the value behind a reference to `Meters`
- fixtures/reject/src/lib.rs:247:22: replacing the value behind a reference to `[u16; 1]`

</details>

<details><summary>`_` is not supported: it would be pending two ways with one value (2)</summary>

- fixtures/reject/src/lib.rs:112:20: `Poll<std::option::Option<std::task::Poll<u8>>>` is not supported: it would be pending two ways with one value
- fixtures/reject/src/lib.rs:117:54: `Poll<std::task::Poll<u8>>` is not supported: it would be pending two ways with one value

</details>

<details><summary>`_` is not supported: its `None` and `Some` would both be `rt.None` (2)</summary>

- fixtures/reject/src/lib.rs:23:15: `Option<std::option::Option<std::option::Option<u8>>>` is not supported: its `None` and `Some` would both be `rt.None`
- fixtures/reject/src/lib.rs:24:2: `Option<std::option::Option<std::option::Option<u8>>>` is not supported: its `None` and `Some` would both be `rt.None`

</details>

<details><summary>`_` of `_`, whose `None` is `rt.None`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:48:42: `Iterator::find` of `Option<std::option::Option<u8>>`, whose `None` is `rt.None`, which is not supported yet
- fixtures/reject/src/lib.rs:270:6: `HashMap::get` of `Option<&std::option::Option<u8>>`, whose `None` is `rt.None`, which is not supported yet

</details>

<details><summary>`_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it (2)</summary>

- fixtures/reject/src/lib.rs:254:16: `std::iter::Iterator::filter` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it
- fixtures/reject/src/lib.rs:254:38: `std::iter::Iterator::count` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it

</details>

<details><summary>an iterator of the crate implementing `_`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:166:1: an iterator of the crate implementing `DoubleEndedIterator`, which is not supported yet
- fixtures/reject/src/lib.rs:220:1: an iterator of the crate implementing `FusedIterator`, which is not supported yet

</details>

<details><summary>constructing `_` from another crate (2)</summary>

- fixtures/reject/src/lib.rs:152:23: constructing `std::iter::Map<Folds, {closure@fixtures/reject/src/lib.rs:152:12: 152:15}>` from another crate
- fixtures/reject/src/lib.rs:173:14: constructing `std::iter::Rev<Backs>` from another crate

</details>

<details><summary>overriding `_`, which std's adaptors call in place of `next` or beside it (2)</summary>

- fixtures/reject/src/lib.rs:146:2: overriding `Iterator::fold`, which std's adaptors call in place of `next` or beside it
- fixtures/reject/src/lib.rs:200:2: overriding `Iterator::size_hint`, which std's adaptors call in place of `next` or beside it

</details>

<details><summary>passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:397:13: passing `{closure@fixtures/reject/src/lib.rs:397:28: 397:35}`, a closure that owns a value that drops, to `Entry::or_insert_with`, which is not supported yet
- fixtures/reject/src/lib.rs:398:16: passing `{closure@fixtures/reject/src/lib.rs:398:25: 398:33}`, a closure that owns a value that drops, to `Iterator::for_each`, which is not supported yet

</details>

<details><summary>`Pointer` formatting, as in `{:p}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:75:2: `Pointer` formatting, as in `{:p}`, is not supported

</details>

<details><summary>`_` is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:421:18: `std::borrow::Cow<'_, [u8]>` is not supported yet

</details>

<details><summary>`_` is not supported: its field `_` is `_`, typed `_`, whose `None` would be `rt.None` as `_` (1)</summary>

- fixtures/reject/src/lib.rs:38:2: `Maybe<()>` is not supported: its field `value` is `Option<T>`, typed `T | undefined`, whose `None` would be `rt.None` as `Option<()>`

</details>

<details><summary>`_` of `_` by `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:293:6: `HashMap::contains_key` of `std::collections::HashMap<Id, u8>` by `u8`, which is not supported yet

</details>

<details><summary>`_` of `_`, which drops `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:60:13: `into_keys` of `std::collections::HashMap<u8, Noisy>`, which drops `Noisy`, which is not supported yet

</details>

<details><summary>`_` through a held type's own, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:325:2: `<std::sync::Arc<Picky> as PartialEq>::ne` through a held type's own, which is not supported yet

</details>

<details><summary>`_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:301:9: `Hasher::write`, which is not supported yet

</details>

<details><summary>`f64::mul_add`, which rounds once, where JS would round twice (1)</summary>

- fixtures/reject/src/lib.rs:484:4: `f64::mul_add`, which rounds once, where JS would round twice

</details>

<details><summary>a `zip` of `_` and `_`, which std reads by index, of items that drop, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:228:30: a `zip` of `std::iter::Map<std::slice::Iter<'_, u8>, {closure@fixtures/reject/src/lib.rs:228:15: 228:19}>` and `std::ops::Range<i32>`, which std reads by index, of items that drop, which is not supported yet

</details>

<details><summary>a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` (1)</summary>

- fixtures/reject/src/lib.rs:179:51: a `zip` of `std::iter::Chain<std::iter::Map<std::slice::Iter<'_, u8>, {closure@fixtures/reject/src/lib.rs:179:15: 179:18}>, std::iter::Copied<std::slice::Iter<'_, u8>>>`, whose closures std calls one item further through `next` than through `fold`

</details>

<details><summary>a cast from `_` to `_` (1)</summary>

- fixtures/reject/src/lib.rs:103:2: a cast from `u64` to `f32`

</details>

<details><summary>a coercion to `_` (1)</summary>

- fixtures/reject/src/lib.rs:348:2: a coercion to `std::sync::Arc<[u8]>`

</details>

<details><summary>a constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:87:2: a constant of type `&u8`

</details>

<details><summary>a copy of a closure that changes what it captured (1)</summary>

- fixtures/reject/src/lib.rs:264:14: a copy of a closure that changes what it captured

</details>

<details><summary>a mutable borrow of a subslice of `_`, which JS would copy, having no views (1)</summary>

- fixtures/reject/src/lib.rs:233:13: a mutable borrow of a subslice of `u32`, which JS would copy, having no views

</details>

<details><summary>a mutable reference into a `Result`'s payload, passed on or kept (1)</summary>

- fixtures/reject/src/lib.rs:453:12: a mutable reference into a `Result`'s payload, passed on or kept

</details>

<details><summary>a printable type must implement `Display` or `Debug` (1)</summary>

- fixtures/reject/src/lib.rs:99:1: a printable type must implement `Display` or `Debug`

</details>

<details><summary>a scalar constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:8:2: a scalar constant of type `f32`

</details>

<details><summary>a zero-sized constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:69:2: a zero-sized constant of type `{async closure@fixtures/reject/src/lib.rs:68:10: 68:18}`

</details>

<details><summary>arithmetic on `_` (1)</summary>

- fixtures/reject/src/lib.rs:8:2: arithmetic on `f32`

</details>

<details><summary>collecting `_`, whose `None` is `rt.None`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:43:43: collecting `Option<()>`, whose `None` is `rt.None`, which is not supported yet

</details>

<details><summary>comparing `_` as a key, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:275:52: comparing `std::vec::Vec<u8>` as a key, which is not supported yet

</details>

<details><summary>constructing `_` (1)</summary>

- fixtures/reject/src/lib.rs:428:53: constructing `std::borrow::Cow<'_, std::sync::Arc<u8>>`

</details>

<details><summary>hashing `_` as a key, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:275:52: hashing `std::vec::Vec<u8>` as a key, which is not supported yet

</details>

<details><summary>hashing `_` through `_`'s own `Hash::hash_slice`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:388:16: hashing `std::vec::Vec<Batched>` through `Batched`'s own `Hash::hash_slice`, which is not supported yet

</details>

<details><summary>passing `_`, which drops, to `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:405:4: passing `{closure@fixtures/reject/src/lib.rs:405:13: 405:27}`, which drops, to `str::contains`, which is not supported yet

</details>
