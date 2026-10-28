# Census of `reject-fixture`

- Refusals: 73
- Function instances: 83
- Generic functions no instance reached, lowered over their type parameters: 6

## By construct

| Count | Construct | First |
| ---: | --- | --- |
| 11 | unsupported call to `_` | fixtures/reject/src/lib.rs:21:4 |
| 4 | a variant of `_`, which is not supported: either is the same JS string | fixtures/reject/src/lib.rs:382:2 |
| 4 | an assignment through a reference to `_` | fixtures/reject/src/lib.rs:101:2 |
| 3 | replacing the value behind a reference to `_` | fixtures/reject/src/lib.rs:85:2 |
| 3 | unsupported type `_` | fixtures/reject/src/lib.rs:7:14 |
| 2 | `_` is not supported: it would be pending two ways with one value | fixtures/reject/src/lib.rs:105:20 |
| 2 | `_` is not supported: its `None` and `Some` would both be `rt.None` | fixtures/reject/src/lib.rs:16:15 |
| 2 | `_` of `_`, whose `None` is `rt.None`, which is not supported yet | fixtures/reject/src/lib.rs:41:42 |
| 2 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | fixtures/reject/src/lib.rs:247:16 |
| 2 | `_`, a trait object of the generic trait `_`, which is not supported yet | fixtures/reject/src/lib.rs:474:23 |
| 2 | a coercion to `_`, which is not supported yet | fixtures/reject/src/lib.rs:465:40 |
| 2 | an iterator of the crate implementing `_`, which is not supported yet | fixtures/reject/src/lib.rs:159:1 |
| 2 | constructing `_` from another crate | fixtures/reject/src/lib.rs:145:23 |
| 2 | overriding `_`, which std's adaptors call in place of `next` or beside it | fixtures/reject/src/lib.rs:139:2 |
| 2 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | fixtures/reject/src/lib.rs:369:13 |
| 1 | `Pointer` formatting, as in `{:p}`, is not supported | fixtures/reject/src/lib.rs:68:2 |
| 1 | `_` is not supported yet | fixtures/reject/src/lib.rs:393:18 |
| 1 | `_` is not supported: its field `_` is `_`, typed `_`, whose `None` would be `rt.None` as `_` | fixtures/reject/src/lib.rs:31:2 |
| 1 | `_` of `_` by `_`, which is not supported yet | fixtures/reject/src/lib.rs:286:6 |
| 1 | `_` of `_`, which drops `_`, which is not supported yet | fixtures/reject/src/lib.rs:53:13 |
| 1 | `_` through a held type's own, which is not supported yet | fixtures/reject/src/lib.rs:318:2 |
| 1 | `_`, a trait object of another crate's trait, which is not supported yet | fixtures/reject/src/lib.rs:11:16 |
| 1 | `_`, which is not supported yet | fixtures/reject/src/lib.rs:294:9 |
| 1 | `f64::mul_add`, which rounds once, where JS would round twice | fixtures/reject/src/lib.rs:444:4 |
| 1 | a `zip` of `_` and `_`, which std reads by index, of items that drop, which is not supported yet | fixtures/reject/src/lib.rs:221:30 |
| 1 | a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` | fixtures/reject/src/lib.rs:172:51 |
| 1 | a cast from `_` to `_` | fixtures/reject/src/lib.rs:96:2 |
| 1 | a coercion to `_` | fixtures/reject/src/lib.rs:341:2 |
| 1 | a constant of type `_` | fixtures/reject/src/lib.rs:80:2 |
| 1 | a copy of a closure that changes what it captured | fixtures/reject/src/lib.rs:257:14 |
| 1 | a mutable borrow of a subslice of `_`, which JS would copy, having no views | fixtures/reject/src/lib.rs:226:13 |
| 1 | a mutable reference into a `Result`'s payload, passed on or kept | fixtures/reject/src/lib.rs:413:12 |
| 1 | a printable type must implement `Display` or `Debug` | fixtures/reject/src/lib.rs:92:1 |
| 1 | a scalar constant of type `_` | fixtures/reject/src/lib.rs:8:2 |
| 1 | a trait object of `_`, which is not supported yet | fixtures/reject/src/lib.rs:491:26 |
| 1 | a zero-sized constant of type `_` | fixtures/reject/src/lib.rs:62:2 |
| 1 | arithmetic on `_` | fixtures/reject/src/lib.rs:8:2 |
| 1 | collecting `_`, whose `None` is `rt.None`, which is not supported yet | fixtures/reject/src/lib.rs:36:43 |
| 1 | comparing `_` as a key, which is not supported yet | fixtures/reject/src/lib.rs:268:52 |
| 1 | constructing `_` | fixtures/reject/src/lib.rs:400:53 |
| 1 | hashing `_` as a key, which is not supported yet | fixtures/reject/src/lib.rs:268:52 |
| 1 | hashing `_` through `_`'s own `Hash::hash_slice`, which is not supported yet | fixtures/reject/src/lib.rs:360:16 |
| 1 | passing `_`, which drops, to `_`, which is not supported yet | fixtures/reject/src/lib.rs:377:4 |

## By API

The type or module each refused type or path belongs to.

| Count | API | First |
| ---: | --- | --- |
| 7 | `std::borrow::Cow` | fixtures/reject/src/lib.rs:382:2 |
| 6 | `Option` | fixtures/reject/src/lib.rs:16:15 |
| 6 | `f32` | fixtures/reject/src/lib.rs:7:14 |
| 4 | `Iterator` | fixtures/reject/src/lib.rs:41:42 |
| 3 | `std::sync::Arc` | fixtures/reject/src/lib.rs:101:2 |
| 3 | `std::vec::Vec` | fixtures/reject/src/lib.rs:268:52 |
| 3 | `u8` | fixtures/reject/src/lib.rs:80:2 |
| 3 | `{closure}` | fixtures/reject/src/lib.rs:369:13 |
| 2 | `HashMap` | fixtures/reject/src/lib.rs:263:6 |
| 2 | `Meters` | fixtures/reject/src/lib.rs:85:2 |
| 2 | `Poll` | fixtures/reject/src/lib.rs:105:20 |
| 2 | `Scaled` | fixtures/reject/src/lib.rs:474:23 |
| 2 | `dyn Scaled` | fixtures/reject/src/lib.rs:474:23 |
| 2 | `std::collections::HashMap` | fixtures/reject/src/lib.rs:53:13 |
| 2 | `std::iter::Iterator` | fixtures/reject/src/lib.rs:247:16 |
| 2 | `std::iter::Map` | fixtures/reject/src/lib.rs:145:23 |
| 2 | `std::ops::Range` | fixtures/reject/src/lib.rs:180:8 |
| 2 | `std::pin::Pin` | fixtures/reject/src/lib.rs:116:2 |
| 2 | `std::slice` | fixtures/reject/src/lib.rs:21:4 |
| 2 | `std::vec::IntoIter` | fixtures/reject/src/lib.rs:439:17 |
| 1 | `Batched` | fixtures/reject/src/lib.rs:360:16 |
| 1 | `DoubleEndedIterator` | fixtures/reject/src/lib.rs:159:1 |
| 1 | `Entry` | fixtures/reject/src/lib.rs:369:13 |
| 1 | `FusedIterator` | fixtures/reject/src/lib.rs:213:1 |
| 1 | `Hasher` | fixtures/reject/src/lib.rs:294:9 |
| 1 | `Maybe` | fixtures/reject/src/lib.rs:31:2 |
| 1 | `Noisy` | fixtures/reject/src/lib.rs:53:13 |
| 1 | `T \| undefined` | fixtures/reject/src/lib.rs:31:2 |
| 1 | `Tail` | fixtures/reject/src/lib.rs:501:30 |
| 1 | `[u16; 1]` | fixtures/reject/src/lib.rs:240:22 |
| 1 | `[u8; 4]` | fixtures/reject/src/lib.rs:491:26 |
| 1 | `dyn std::ops::Fn(u8) -> u8` | fixtures/reject/src/lib.rs:11:16 |
| 1 | `f64` | fixtures/reject/src/lib.rs:449:2 |
| 1 | `into_keys` | fixtures/reject/src/lib.rs:53:13 |
| 1 | `std::iter::Chain` | fixtures/reject/src/lib.rs:172:51 |
| 1 | `std::iter::Rev` | fixtures/reject/src/lib.rs:166:14 |
| 1 | `std::option::Option` | fixtures/reject/src/lib.rs:420:2 |
| 1 | `std::slice::Iter` | fixtures/reject/src/lib.rs:126:11 |
| 1 | `std::string::String` | fixtures/reject/src/lib.rs:74:10 |
| 1 | `std::sync::MutexGuard` | fixtures/reject/src/lib.rs:121:2 |
| 1 | `std::sync::Weak` | fixtures/reject/src/lib.rs:465:40 |
| 1 | `str` | fixtures/reject/src/lib.rs:377:4 |
| 1 | `u32` | fixtures/reject/src/lib.rs:226:13 |
| 1 | `u64` | fixtures/reject/src/lib.rs:96:2 |
| 1 | `value` | fixtures/reject/src/lib.rs:31:2 |
| 1 | `{async closure}` | fixtures/reject/src/lib.rs:62:2 |

## By type or path

| Count | Construct | Type or path | First |
| ---: | --- | --- | --- |
| 4 | a variant of `_`, which is not supported: either is the same JS string | `std::borrow::Cow` | fixtures/reject/src/lib.rs:382:2 |
| 3 | unsupported type `_` | `f32` | fixtures/reject/src/lib.rs:7:14 |
| 2 | `_` is not supported: it would be pending two ways with one value | `Poll` | fixtures/reject/src/lib.rs:105:20 |
| 2 | `_` is not supported: its `None` and `Some` would both be `rt.None` | `Option` | fixtures/reject/src/lib.rs:16:15 |
| 2 | `_`, a trait object of the generic trait `_`, which is not supported yet | `dyn Scaled`, `Scaled` | fixtures/reject/src/lib.rs:474:23 |
| 2 | an assignment through a reference to `_` | `std::pin::Pin` | fixtures/reject/src/lib.rs:116:2 |
| 2 | replacing the value behind a reference to `_` | `Meters` | fixtures/reject/src/lib.rs:85:2 |
| 2 | unsupported call to `_` | `<std::vec::IntoIter as std::iter::Iterator>::sum` | fixtures/reject/src/lib.rs:439:17 |
| 1 | `_` is not supported yet | `std::borrow::Cow` | fixtures/reject/src/lib.rs:393:18 |
| 1 | `_` is not supported: its field `_` is `_`, typed `_`, whose `None` would be `rt.None` as `_` | `Maybe`, `value`, `Option`, `T \| undefined`, `Option` | fixtures/reject/src/lib.rs:31:2 |
| 1 | `_` of `_` by `_`, which is not supported yet | `HashMap::contains_key`, `std::collections::HashMap`, `u8` | fixtures/reject/src/lib.rs:286:6 |
| 1 | `_` of `_`, which drops `_`, which is not supported yet | `into_keys`, `std::collections::HashMap`, `Noisy` | fixtures/reject/src/lib.rs:53:13 |
| 1 | `_` of `_`, whose `None` is `rt.None`, which is not supported yet | `HashMap::get`, `Option` | fixtures/reject/src/lib.rs:263:6 |
| 1 | `_` of `_`, whose `None` is `rt.None`, which is not supported yet | `Iterator::find`, `Option` | fixtures/reject/src/lib.rs:41:42 |
| 1 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | `std::iter::Iterator::count` | fixtures/reject/src/lib.rs:247:38 |
| 1 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | `std::iter::Iterator::filter` | fixtures/reject/src/lib.rs:247:16 |
| 1 | `_` through a held type's own, which is not supported yet | `<std::sync::Arc as PartialEq>::ne` | fixtures/reject/src/lib.rs:318:2 |
| 1 | `_`, a trait object of another crate's trait, which is not supported yet | `dyn std::ops::Fn(u8) -> u8` | fixtures/reject/src/lib.rs:11:16 |
| 1 | `_`, which is not supported yet | `Hasher::write` | fixtures/reject/src/lib.rs:294:9 |
| 1 | a `zip` of `_` and `_`, which std reads by index, of items that drop, which is not supported yet | `std::iter::Map`, `std::ops::Range` | fixtures/reject/src/lib.rs:221:30 |
| 1 | a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` | `std::iter::Chain` | fixtures/reject/src/lib.rs:172:51 |
| 1 | a cast from `_` to `_` | `u64`, `f32` | fixtures/reject/src/lib.rs:96:2 |
| 1 | a coercion to `_` | `std::sync::Arc` | fixtures/reject/src/lib.rs:341:2 |
| 1 | a coercion to `_`, which is not supported yet | `&Tail` | fixtures/reject/src/lib.rs:501:30 |
| 1 | a coercion to `_`, which is not supported yet | `std::sync::Weak` | fixtures/reject/src/lib.rs:465:40 |
| 1 | a constant of type `_` | `&u8` | fixtures/reject/src/lib.rs:80:2 |
| 1 | a mutable borrow of a subslice of `_`, which JS would copy, having no views | `u32` | fixtures/reject/src/lib.rs:226:13 |
| 1 | a scalar constant of type `_` | `f32` | fixtures/reject/src/lib.rs:8:2 |
| 1 | a trait object of `_`, which is not supported yet | `[u8; 4]` | fixtures/reject/src/lib.rs:491:26 |
| 1 | a zero-sized constant of type `_` | `{async closure}` | fixtures/reject/src/lib.rs:62:2 |
| 1 | an assignment through a reference to `_` | `std::sync::Arc` | fixtures/reject/src/lib.rs:101:2 |
| 1 | an assignment through a reference to `_` | `std::sync::MutexGuard` | fixtures/reject/src/lib.rs:121:2 |
| 1 | an iterator of the crate implementing `_`, which is not supported yet | `DoubleEndedIterator` | fixtures/reject/src/lib.rs:159:1 |
| 1 | an iterator of the crate implementing `_`, which is not supported yet | `FusedIterator` | fixtures/reject/src/lib.rs:213:1 |
| 1 | arithmetic on `_` | `f32` | fixtures/reject/src/lib.rs:8:2 |
| 1 | collecting `_`, whose `None` is `rt.None`, which is not supported yet | `Option` | fixtures/reject/src/lib.rs:36:43 |
| 1 | comparing `_` as a key, which is not supported yet | `std::vec::Vec` | fixtures/reject/src/lib.rs:268:52 |
| 1 | constructing `_` | `std::borrow::Cow` | fixtures/reject/src/lib.rs:400:53 |
| 1 | constructing `_` from another crate | `std::iter::Map` | fixtures/reject/src/lib.rs:145:23 |
| 1 | constructing `_` from another crate | `std::iter::Rev` | fixtures/reject/src/lib.rs:166:14 |
| 1 | hashing `_` as a key, which is not supported yet | `std::vec::Vec` | fixtures/reject/src/lib.rs:268:52 |
| 1 | hashing `_` through `_`'s own `Hash::hash_slice`, which is not supported yet | `std::vec::Vec`, `Batched` | fixtures/reject/src/lib.rs:360:16 |
| 1 | overriding `_`, which std's adaptors call in place of `next` or beside it | `Iterator::fold` | fixtures/reject/src/lib.rs:139:2 |
| 1 | overriding `_`, which std's adaptors call in place of `next` or beside it | `Iterator::size_hint` | fixtures/reject/src/lib.rs:193:2 |
| 1 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | `{closure}`, `Entry::or_insert_with` | fixtures/reject/src/lib.rs:369:13 |
| 1 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | `{closure}`, `Iterator::for_each` | fixtures/reject/src/lib.rs:370:16 |
| 1 | passing `_`, which drops, to `_`, which is not supported yet | `{closure}`, `str::contains` | fixtures/reject/src/lib.rs:377:4 |
| 1 | replacing the value behind a reference to `_` | `[u16; 1]` | fixtures/reject/src/lib.rs:240:22 |
| 1 | unsupported call to `_` | `<f64 as std::fmt::Display>::fmt` | fixtures/reject/src/lib.rs:449:2 |
| 1 | unsupported call to `_` | `<std::borrow::Cow as std::ops::Deref>::deref` | fixtures/reject/src/lib.rs:394:2 |
| 1 | unsupported call to `_` | `<std::option::Option as std::cmp::PartialEq>::eq` | fixtures/reject/src/lib.rs:420:2 |
| 1 | unsupported call to `_` | `<std::slice::Iter as std::iter::Iterator>::cycle` | fixtures/reject/src/lib.rs:126:11 |
| 1 | unsupported call to `_` | `<std::string::String as std::fmt::Write>::write_fmt` | fixtures/reject/src/lib.rs:74:10 |
| 1 | unsupported call to `_` | `<u8 as std::hash::Hash>::hash` | fixtures/reject/src/lib.rs:351:10 |
| 1 | unsupported call to `_` | `std::ops::Range::contains` | fixtures/reject/src/lib.rs:180:8 |
| 1 | unsupported call to `_` | `std::slice::copy_within` | fixtures/reject/src/lib.rs:336:4 |
| 1 | unsupported call to `_` | `std::slice::sort` | fixtures/reject/src/lib.rs:21:4 |

## Spans

<details><summary>unsupported call to `_` (11)</summary>

- fixtures/reject/src/lib.rs:21:4: unsupported call to `std::slice::<impl [u8]>::sort`
- fixtures/reject/src/lib.rs:74:10: unsupported call to `<std::string::String as std::fmt::Write>::write_fmt`
- fixtures/reject/src/lib.rs:126:11: unsupported call to `<std::slice::Iter<'_, u8> as std::iter::Iterator>::cycle`
- fixtures/reject/src/lib.rs:180:8: unsupported call to `std::ops::Range::<Key>::contains::<Key>`
- fixtures/reject/src/lib.rs:336:4: unsupported call to `core::slice::<impl [(u8, u8)]>::copy_within::<std::ops::Range<usize>>`
- fixtures/reject/src/lib.rs:351:10: unsupported call to `<u8 as std::hash::Hash>::hash::<H>`
- fixtures/reject/src/lib.rs:394:2: unsupported call to `<std::borrow::Cow<'_, [u8]> as std::ops::Deref>::deref`
- fixtures/reject/src/lib.rs:420:2: unsupported call to `<std::option::Option<&mut u32> as std::cmp::PartialEq>::eq`
- fixtures/reject/src/lib.rs:439:17: unsupported call to `<std::vec::IntoIter<Part> as std::iter::Iterator>::sum::<std::time::Duration>`
- fixtures/reject/src/lib.rs:439:38: unsupported call to `<std::vec::IntoIter<Part> as std::iter::Iterator>::sum::<u64>`
- fixtures/reject/src/lib.rs:449:2: unsupported call to `<f64 as std::fmt::Display>::fmt`

</details>

<details><summary>a variant of `_`, which is not supported: either is the same JS string (4)</summary>

- fixtures/reject/src/lib.rs:382:2: a variant of `std::borrow::Cow<'_, str>`, which is not supported: either is the same JS string
- fixtures/reject/src/lib.rs:383:30: a variant of `std::borrow::Cow<'_, str>`, which is not supported: either is the same JS string
- fixtures/reject/src/lib.rs:384:27: a variant of `std::borrow::Cow<'_, str>`, which is not supported: either is the same JS string
- fixtures/reject/src/lib.rs:389:2: a variant of `std::borrow::Cow<'_, str>`, which is not supported: either is the same JS string

</details>

<details><summary>an assignment through a reference to `_` (4)</summary>

- fixtures/reject/src/lib.rs:101:2: an assignment through a reference to `std::sync::Arc<u8>`
- fixtures/reject/src/lib.rs:116:2: an assignment through a reference to `std::pin::Pin<std::sync::Arc<u8>>`
- fixtures/reject/src/lib.rs:121:2: an assignment through a reference to `std::sync::MutexGuard<'_, u8>`
- fixtures/reject/src/lib.rs:234:2: an assignment through a reference to `std::pin::Pin<&mut Noisy>`

</details>

<details><summary>replacing the value behind a reference to `_` (3)</summary>

- fixtures/reject/src/lib.rs:85:2: replacing the value behind a reference to `Meters`
- fixtures/reject/src/lib.rs:89:2: replacing the value behind a reference to `Meters`
- fixtures/reject/src/lib.rs:240:22: replacing the value behind a reference to `[u16; 1]`

</details>

<details><summary>unsupported type `_` (3)</summary>

- fixtures/reject/src/lib.rs:7:14: unsupported type `f32`
- fixtures/reject/src/lib.rs:7:25: unsupported type `f32`
- fixtures/reject/src/lib.rs:94:51: unsupported type `f32`

</details>

<details><summary>`_` is not supported: it would be pending two ways with one value (2)</summary>

- fixtures/reject/src/lib.rs:105:20: `Poll<std::option::Option<std::task::Poll<u8>>>` is not supported: it would be pending two ways with one value
- fixtures/reject/src/lib.rs:110:54: `Poll<std::task::Poll<u8>>` is not supported: it would be pending two ways with one value

</details>

<details><summary>`_` is not supported: its `None` and `Some` would both be `rt.None` (2)</summary>

- fixtures/reject/src/lib.rs:16:15: `Option<std::option::Option<std::option::Option<u8>>>` is not supported: its `None` and `Some` would both be `rt.None`
- fixtures/reject/src/lib.rs:17:2: `Option<std::option::Option<std::option::Option<u8>>>` is not supported: its `None` and `Some` would both be `rt.None`

</details>

<details><summary>`_` of `_`, whose `None` is `rt.None`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:41:42: `Iterator::find` of `Option<std::option::Option<u8>>`, whose `None` is `rt.None`, which is not supported yet
- fixtures/reject/src/lib.rs:263:6: `HashMap::get` of `Option<&std::option::Option<u8>>`, whose `None` is `rt.None`, which is not supported yet

</details>

<details><summary>`_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it (2)</summary>

- fixtures/reject/src/lib.rs:247:16: `std::iter::Iterator::filter` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it
- fixtures/reject/src/lib.rs:247:38: `std::iter::Iterator::count` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it

</details>

<details><summary>`_`, a trait object of the generic trait `_`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:474:23: `dyn Scaled<u8>`, a trait object of the generic trait `Scaled`, which is not supported yet
- fixtures/reject/src/lib.rs:475:4: `dyn Scaled<u8>`, a trait object of the generic trait `Scaled`, which is not supported yet

</details>

<details><summary>a coercion to `_`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:465:40: a coercion to `std::sync::Weak<dyn Reading>`, which is not supported yet
- fixtures/reject/src/lib.rs:501:30: a coercion to `&Tail<dyn Reading>`, which is not supported yet

</details>

<details><summary>an iterator of the crate implementing `_`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:159:1: an iterator of the crate implementing `DoubleEndedIterator`, which is not supported yet
- fixtures/reject/src/lib.rs:213:1: an iterator of the crate implementing `FusedIterator`, which is not supported yet

</details>

<details><summary>constructing `_` from another crate (2)</summary>

- fixtures/reject/src/lib.rs:145:23: constructing `std::iter::Map<Folds, {closure@fixtures/reject/src/lib.rs:145:12: 145:15}>` from another crate
- fixtures/reject/src/lib.rs:166:14: constructing `std::iter::Rev<Backs>` from another crate

</details>

<details><summary>overriding `_`, which std's adaptors call in place of `next` or beside it (2)</summary>

- fixtures/reject/src/lib.rs:139:2: overriding `Iterator::fold`, which std's adaptors call in place of `next` or beside it
- fixtures/reject/src/lib.rs:193:2: overriding `Iterator::size_hint`, which std's adaptors call in place of `next` or beside it

</details>

<details><summary>passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:369:13: passing `{closure@fixtures/reject/src/lib.rs:369:28: 369:35}`, a closure that owns a value that drops, to `Entry::or_insert_with`, which is not supported yet
- fixtures/reject/src/lib.rs:370:16: passing `{closure@fixtures/reject/src/lib.rs:370:25: 370:33}`, a closure that owns a value that drops, to `Iterator::for_each`, which is not supported yet

</details>

<details><summary>`Pointer` formatting, as in `{:p}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:68:2: `Pointer` formatting, as in `{:p}`, is not supported

</details>

<details><summary>`_` is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:393:18: `std::borrow::Cow<'_, [u8]>` is not supported yet

</details>

<details><summary>`_` is not supported: its field `_` is `_`, typed `_`, whose `None` would be `rt.None` as `_` (1)</summary>

- fixtures/reject/src/lib.rs:31:2: `Maybe<()>` is not supported: its field `value` is `Option<T>`, typed `T | undefined`, whose `None` would be `rt.None` as `Option<()>`

</details>

<details><summary>`_` of `_` by `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:286:6: `HashMap::contains_key` of `std::collections::HashMap<Id, u8>` by `u8`, which is not supported yet

</details>

<details><summary>`_` of `_`, which drops `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:53:13: `into_keys` of `std::collections::HashMap<u8, Noisy>`, which drops `Noisy`, which is not supported yet

</details>

<details><summary>`_` through a held type's own, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:318:2: `<std::sync::Arc<Picky> as PartialEq>::ne` through a held type's own, which is not supported yet

</details>

<details><summary>`_`, a trait object of another crate's trait, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:11:16: `dyn std::ops::Fn(u8) -> u8`, a trait object of another crate's trait, which is not supported yet

</details>

<details><summary>`_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:294:9: `Hasher::write`, which is not supported yet

</details>

<details><summary>`f64::mul_add`, which rounds once, where JS would round twice (1)</summary>

- fixtures/reject/src/lib.rs:444:4: `f64::mul_add`, which rounds once, where JS would round twice

</details>

<details><summary>a `zip` of `_` and `_`, which std reads by index, of items that drop, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:221:30: a `zip` of `std::iter::Map<std::slice::Iter<'_, u8>, {closure@fixtures/reject/src/lib.rs:221:15: 221:19}>` and `std::ops::Range<i32>`, which std reads by index, of items that drop, which is not supported yet

</details>

<details><summary>a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` (1)</summary>

- fixtures/reject/src/lib.rs:172:51: a `zip` of `std::iter::Chain<std::iter::Map<std::slice::Iter<'_, u8>, {closure@fixtures/reject/src/lib.rs:172:15: 172:18}>, std::iter::Copied<std::slice::Iter<'_, u8>>>`, whose closures std calls one item further through `next` than through `fold`

</details>

<details><summary>a cast from `_` to `_` (1)</summary>

- fixtures/reject/src/lib.rs:96:2: a cast from `u64` to `f32`

</details>

<details><summary>a coercion to `_` (1)</summary>

- fixtures/reject/src/lib.rs:341:2: a coercion to `std::sync::Arc<[u8]>`

</details>

<details><summary>a constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:80:2: a constant of type `&u8`

</details>

<details><summary>a copy of a closure that changes what it captured (1)</summary>

- fixtures/reject/src/lib.rs:257:14: a copy of a closure that changes what it captured

</details>

<details><summary>a mutable borrow of a subslice of `_`, which JS would copy, having no views (1)</summary>

- fixtures/reject/src/lib.rs:226:13: a mutable borrow of a subslice of `u32`, which JS would copy, having no views

</details>

<details><summary>a mutable reference into a `Result`'s payload, passed on or kept (1)</summary>

- fixtures/reject/src/lib.rs:413:12: a mutable reference into a `Result`'s payload, passed on or kept

</details>

<details><summary>a printable type must implement `Display` or `Debug` (1)</summary>

- fixtures/reject/src/lib.rs:92:1: a printable type must implement `Display` or `Debug`

</details>

<details><summary>a scalar constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:8:2: a scalar constant of type `f32`

</details>

<details><summary>a trait object of `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:491:26: a trait object of `[u8; 4]`, which is not supported yet

</details>

<details><summary>a zero-sized constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:62:2: a zero-sized constant of type `{async closure@fixtures/reject/src/lib.rs:61:10: 61:18}`

</details>

<details><summary>arithmetic on `_` (1)</summary>

- fixtures/reject/src/lib.rs:8:2: arithmetic on `f32`

</details>

<details><summary>collecting `_`, whose `None` is `rt.None`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:36:43: collecting `Option<()>`, whose `None` is `rt.None`, which is not supported yet

</details>

<details><summary>comparing `_` as a key, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:268:52: comparing `std::vec::Vec<u8>` as a key, which is not supported yet

</details>

<details><summary>constructing `_` (1)</summary>

- fixtures/reject/src/lib.rs:400:53: constructing `std::borrow::Cow<'_, std::sync::Arc<u8>>`

</details>

<details><summary>hashing `_` as a key, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:268:52: hashing `std::vec::Vec<u8>` as a key, which is not supported yet

</details>

<details><summary>hashing `_` through `_`'s own `Hash::hash_slice`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:360:16: hashing `std::vec::Vec<Batched>` through `Batched`'s own `Hash::hash_slice`, which is not supported yet

</details>

<details><summary>passing `_`, which drops, to `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:377:4: passing `{closure@fixtures/reject/src/lib.rs:377:13: 377:27}`, which drops, to `str::contains`, which is not supported yet

</details>
