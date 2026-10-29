# Census of `reject-fixture`

- Refusals: 95
- Function instances: 104
- Generic functions no instance reached, lowered over their type parameters: 7

## By construct

| Count | Construct | First |
| ---: | --- | --- |
| 16 | unsupported call to `_` | fixtures/reject/src/lib.rs:21:4 |
| 4 | a variant of `_`, which is not supported: either is the same JS string | fixtures/reject/src/lib.rs:382:2 |
| 4 | an assignment through a reference to `_` | fixtures/reject/src/lib.rs:101:2 |
| 3 | a copy of a closure that changes what it captured | fixtures/reject/src/lib.rs:257:14 |
| 3 | replacing the value behind a reference to `_` | fixtures/reject/src/lib.rs:85:2 |
| 3 | unsupported type `_` | fixtures/reject/src/lib.rs:7:14 |
| 2 | `_` is not supported: its `None` and `Some` would both be `rt.None` | fixtures/reject/src/lib.rs:16:15 |
| 2 | `_` of `_`, whose `None` is `rt.None`, which is not supported yet | fixtures/reject/src/lib.rs:41:42 |
| 2 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | fixtures/reject/src/lib.rs:247:16 |
| 2 | `_`, a trait object of the generic trait `_`, which is not supported yet | fixtures/reject/src/lib.rs:479:23 |
| 2 | a coercion to `_`, which is not supported yet | fixtures/reject/src/lib.rs:470:40 |
| 2 | an iterator of the crate implementing `_`, which is not supported yet | fixtures/reject/src/lib.rs:159:1 |
| 2 | constructing `_` from another crate | fixtures/reject/src/lib.rs:145:23 |
| 2 | overriding `_`, which std's adaptors call in place of `next` or beside it | fixtures/reject/src/lib.rs:139:2 |
| 2 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | fixtures/reject/src/lib.rs:369:13 |
| 1 | `Bytes::from_owner` of `_`, which would drop with the last clone of what it makes, which is not supported | fixtures/reject/src/lib.rs:425:2 |
| 1 | `Pointer` formatting, as in `{:p}`, is not supported | fixtures/reject/src/lib.rs:68:2 |
| 1 | `_` is not supported yet | fixtures/reject/src/lib.rs:393:18 |
| 1 | `_` is not supported: its `Pending` and `Ready` would both be `rt.OuterPending` | fixtures/reject/src/lib.rs:105:20 |
| 1 | `_` is not supported: its field `_` is `_`, typed `_`, whose `None` would be `rt.None` as `_` | fixtures/reject/src/lib.rs:31:2 |
| 1 | `_` is not supported: its field `_` is `_`, typed `_`, whose `Pending` would be `rt.OuterPending` as `_` | fixtures/reject/src/lib.rs:517:2 |
| 1 | `_` of `_` by `_`, which is not supported yet | fixtures/reject/src/lib.rs:286:6 |
| 1 | `_` of `_`, which drops `_`, which is not supported yet | fixtures/reject/src/lib.rs:53:13 |
| 1 | `_` of `_`, which is not supported yet | fixtures/reject/src/lib.rs:560:4 |
| 1 | `_` through a held type's own, which is not supported yet | fixtures/reject/src/lib.rs:318:2 |
| 1 | `_` with a mutable reference to a value JS copies, which its mapping would copy | fixtures/reject/src/lib.rs:536:12 |
| 1 | `_`, a trait object of another crate's trait, which is not supported yet | fixtures/reject/src/lib.rs:11:16 |
| 1 | `_`, which is not supported yet | fixtures/reject/src/lib.rs:294:9 |
| 1 | `f64::mul_add`, which rounds once, where JS would round twice | fixtures/reject/src/lib.rs:449:4 |
| 1 | a `zip` of `_` and `_`, which std reads by index, of items that drop, which is not supported yet | fixtures/reject/src/lib.rs:221:30 |
| 1 | a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` | fixtures/reject/src/lib.rs:172:51 |
| 1 | a cast from `_` to `_` | fixtures/reject/src/lib.rs:96:2 |
| 1 | a cast from a pointer to `_` to one to `_`, which reads the bytes of one type as another | fixtures/reject/src/lib.rs:577:27 |
| 1 | a coercion to `_` | fixtures/reject/src/lib.rs:341:2 |
| 1 | a constant of type `_` | fixtures/reject/src/lib.rs:80:2 |
| 1 | a dangling pointer to `_`, a value of no bytes made of more than structs and tuples | fixtures/reject/src/lib.rs:662:11 |
| 1 | a future of `_`, which can be `rt.Pending`, as the future's own `Pending` is | fixtures/reject/src/lib.rs:110:34 |
| 1 | a mutable borrow of a subslice of `_`, which JS would copy, having no views | fixtures/reject/src/lib.rs:226:13 |
| 1 | a mutable reference into a `Result`'s payload, passed on or kept | fixtures/reject/src/lib.rs:413:12 |
| 1 | a pointer's address, which JS has no counterpart of | fixtures/reject/src/lib.rs:582:17 |
| 1 | a printable type must implement `Display` or `Debug` | fixtures/reject/src/lib.rs:92:1 |
| 1 | a raw pointer to `_` behind a reference, which is the object and not where it is kept | fixtures/reject/src/lib.rs:599:10 |
| 1 | a raw pointer to `_` in `_`, which the crate may put another object in place of | fixtures/reject/src/lib.rs:630:10 |
| 1 | a raw pointer to `_` in a variable assigned again while it may point there | fixtures/reject/src/lib.rs:638:10 |
| 1 | a raw pointer to `_` in an element, a payload, or a union field, which is not supported yet | fixtures/reject/src/lib.rs:646:10 |
| 1 | a scalar constant of type `_` | fixtures/reject/src/lib.rs:8:2 |
| 1 | a trait object of `_`, which is not supported yet | fixtures/reject/src/lib.rs:496:26 |
| 1 | a zero-sized constant of type `_` | fixtures/reject/src/lib.rs:62:2 |
| 1 | an upcast from `_` to `_`, which is not supported yet | fixtures/reject/src/lib.rs:554:32 |
| 1 | arithmetic on `_` | fixtures/reject/src/lib.rs:8:2 |
| 1 | collecting `_`, whose `None` is `rt.None`, which is not supported yet | fixtures/reject/src/lib.rs:36:43 |
| 1 | comparing `_` as a key, which is not supported yet | fixtures/reject/src/lib.rs:268:52 |
| 1 | comparing pointers to `()`, which has no bytes and so no address of its own | fixtures/reject/src/lib.rs:653:2 |
| 1 | comparing pointers to `_`, each a place JS makes of its own | fixtures/reject/src/lib.rs:618:2 |
| 1 | constructing `_` | fixtures/reject/src/lib.rs:400:53 |
| 1 | hashing `_` as a key, which is not supported yet | fixtures/reject/src/lib.rs:268:52 |
| 1 | hashing `_` through `_`'s own `Hash::hash_slice`, which is not supported yet | fixtures/reject/src/lib.rs:360:16 |
| 1 | passing `_`, which drops, to `_`, which is not supported yet | fixtures/reject/src/lib.rs:377:4 |
| 1 | union `_`, whose fields overlap in `_` and `f64`, which JS keeps apart | fixtures/reject/src/lib.rs:604:1 |

## By API

The type or module each refused type or path belongs to.

| Count | API | First |
| ---: | --- | --- |
| 7 | `std::borrow::Cow` | fixtures/reject/src/lib.rs:382:2 |
| 6 | `Option` | fixtures/reject/src/lib.rs:16:15 |
| 6 | `f32` | fixtures/reject/src/lib.rs:7:14 |
| 4 | `Iterator` | fixtures/reject/src/lib.rs:41:42 |
| 4 | `std::sync::Arc` | fixtures/reject/src/lib.rs:101:2 |
| 4 | `std::vec::Vec` | fixtures/reject/src/lib.rs:268:52 |
| 4 | `u32` | fixtures/reject/src/lib.rs:226:13 |
| 4 | `u8` | fixtures/reject/src/lib.rs:80:2 |
| 3 | `(u32, u32)` | fixtures/reject/src/lib.rs:630:10 |
| 3 | `{closure}` | fixtures/reject/src/lib.rs:369:13 |
| 2 | `HashMap` | fixtures/reject/src/lib.rs:263:6 |
| 2 | `Meters` | fixtures/reject/src/lib.rs:85:2 |
| 2 | `Poll` | fixtures/reject/src/lib.rs:105:20 |
| 2 | `Scaled` | fixtures/reject/src/lib.rs:479:23 |
| 2 | `dyn Scaled` | fixtures/reject/src/lib.rs:479:23 |
| 2 | `dyn std::any::Any` | fixtures/reject/src/lib.rs:554:32 |
| 2 | `std::any::TypeId` | fixtures/reject/src/lib.rs:560:2 |
| 2 | `std::collections::HashMap` | fixtures/reject/src/lib.rs:53:13 |
| 2 | `std::iter::Iterator` | fixtures/reject/src/lib.rs:247:16 |
| 2 | `std::iter::Map` | fixtures/reject/src/lib.rs:145:23 |
| 2 | `std::ops::Range` | fixtures/reject/src/lib.rs:180:8 |
| 2 | `std::pin::Pin` | fixtures/reject/src/lib.rs:116:2 |
| 2 | `std::slice` | fixtures/reject/src/lib.rs:21:4 |
| 2 | `std::std` | fixtures/reject/src/lib.rs:589:11 |
| 2 | `std::vec::IntoIter` | fixtures/reject/src/lib.rs:444:17 |
| 1 | `Batched` | fixtures/reject/src/lib.rs:360:16 |
| 1 | `Bits` | fixtures/reject/src/lib.rs:604:1 |
| 1 | `DoubleEndedIterator` | fixtures/reject/src/lib.rs:159:1 |
| 1 | `Entry` | fixtures/reject/src/lib.rs:369:13 |
| 1 | `FusedIterator` | fixtures/reject/src/lib.rs:213:1 |
| 1 | `Hasher` | fixtures/reject/src/lib.rs:294:9 |
| 1 | `Maybe` | fixtures/reject/src/lib.rs:31:2 |
| 1 | `Noisy` | fixtures/reject/src/lib.rs:53:13 |
| 1 | `Only` | fixtures/reject/src/lib.rs:662:11 |
| 1 | `Span2` | fixtures/reject/src/lib.rs:630:10 |
| 1 | `T \| undefined` | fixtures/reject/src/lib.rs:31:2 |
| 1 | `Tail` | fixtures/reject/src/lib.rs:506:30 |
| 1 | `Waiting` | fixtures/reject/src/lib.rs:517:2 |
| 1 | `[u16; 1]` | fixtures/reject/src/lib.rs:240:22 |
| 1 | `[u8; 4]` | fixtures/reject/src/lib.rs:496:26 |
| 1 | `dyn Keyed` | fixtures/reject/src/lib.rs:554:32 |
| 1 | `dyn std::ops::Fn(u8) -> u8` | fixtures/reject/src/lib.rs:11:16 |
| 1 | `f64` | fixtures/reject/src/lib.rs:454:2 |
| 1 | `into_keys` | fixtures/reject/src/lib.rs:53:13 |
| 1 | `poll` | fixtures/reject/src/lib.rs:517:2 |
| 1 | `rt.Poll` | fixtures/reject/src/lib.rs:517:2 |
| 1 | `std::any::Any` | fixtures/reject/src/lib.rs:560:4 |
| 1 | `std::iter::Chain` | fixtures/reject/src/lib.rs:172:51 |
| 1 | `std::iter::Rev` | fixtures/reject/src/lib.rs:166:14 |
| 1 | `std::option::Option` | fixtures/reject/src/lib.rs:420:2 |
| 1 | `std::slice::Iter` | fixtures/reject/src/lib.rs:126:11 |
| 1 | `std::std::Layout` | fixtures/reject/src/lib.rs:587:15 |
| 1 | `std::string::String` | fixtures/reject/src/lib.rs:74:10 |
| 1 | `std::sync::MutexGuard` | fixtures/reject/src/lib.rs:121:2 |
| 1 | `std::sync::Mutex` | fixtures/reject/src/lib.rs:536:12 |
| 1 | `std::sync::Weak` | fixtures/reject/src/lib.rs:470:40 |
| 1 | `std::task::Poll` | fixtures/reject/src/lib.rs:110:34 |
| 1 | `str` | fixtures/reject/src/lib.rs:377:4 |
| 1 | `u64` | fixtures/reject/src/lib.rs:96:2 |
| 1 | `value` | fixtures/reject/src/lib.rs:31:2 |
| 1 | `{async closure}` | fixtures/reject/src/lib.rs:62:2 |

## By type or path

| Count | Construct | Type or path | First |
| ---: | --- | --- | --- |
| 4 | a variant of `_`, which is not supported: either is the same JS string | `std::borrow::Cow` | fixtures/reject/src/lib.rs:382:2 |
| 3 | unsupported type `_` | `f32` | fixtures/reject/src/lib.rs:7:14 |
| 2 | `_` is not supported: its `None` and `Some` would both be `rt.None` | `Option` | fixtures/reject/src/lib.rs:16:15 |
| 2 | `_`, a trait object of the generic trait `_`, which is not supported yet | `dyn Scaled`, `Scaled` | fixtures/reject/src/lib.rs:479:23 |
| 2 | an assignment through a reference to `_` | `std::pin::Pin` | fixtures/reject/src/lib.rs:116:2 |
| 2 | replacing the value behind a reference to `_` | `Meters` | fixtures/reject/src/lib.rs:85:2 |
| 2 | unsupported call to `_` | `<std::vec::IntoIter as std::iter::Iterator>::sum` | fixtures/reject/src/lib.rs:444:17 |
| 1 | `Bytes::from_owner` of `_`, which would drop with the last clone of what it makes, which is not supported | `std::sync::Arc` | fixtures/reject/src/lib.rs:425:2 |
| 1 | `_` is not supported yet | `std::borrow::Cow` | fixtures/reject/src/lib.rs:393:18 |
| 1 | `_` is not supported: its `Pending` and `Ready` would both be `rt.OuterPending` | `Poll` | fixtures/reject/src/lib.rs:105:20 |
| 1 | `_` is not supported: its field `_` is `_`, typed `_`, whose `None` would be `rt.None` as `_` | `Maybe`, `value`, `Option`, `T \| undefined`, `Option` | fixtures/reject/src/lib.rs:31:2 |
| 1 | `_` is not supported: its field `_` is `_`, typed `_`, whose `Pending` would be `rt.OuterPending` as `_` | `Waiting`, `poll`, `Poll`, `rt.Poll`, `Poll` | fixtures/reject/src/lib.rs:517:2 |
| 1 | `_` of `_` by `_`, which is not supported yet | `HashMap::contains_key`, `std::collections::HashMap`, `u8` | fixtures/reject/src/lib.rs:286:6 |
| 1 | `_` of `_`, which drops `_`, which is not supported yet | `into_keys`, `std::collections::HashMap`, `Noisy` | fixtures/reject/src/lib.rs:53:13 |
| 1 | `_` of `_`, which is not supported yet | `std::any::Any::type_id`, `dyn std::any::Any` | fixtures/reject/src/lib.rs:560:4 |
| 1 | `_` of `_`, whose `None` is `rt.None`, which is not supported yet | `HashMap::get`, `Option` | fixtures/reject/src/lib.rs:263:6 |
| 1 | `_` of `_`, whose `None` is `rt.None`, which is not supported yet | `Iterator::find`, `Option` | fixtures/reject/src/lib.rs:41:42 |
| 1 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | `std::iter::Iterator::count` | fixtures/reject/src/lib.rs:247:38 |
| 1 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | `std::iter::Iterator::filter` | fixtures/reject/src/lib.rs:247:16 |
| 1 | `_` through a held type's own, which is not supported yet | `<std::sync::Arc as PartialEq>::ne` | fixtures/reject/src/lib.rs:318:2 |
| 1 | `_` with a mutable reference to a value JS copies, which its mapping would copy | `std::sync::Mutex::get_mut` | fixtures/reject/src/lib.rs:536:12 |
| 1 | `_`, a trait object of another crate's trait, which is not supported yet | `dyn std::ops::Fn(u8) -> u8` | fixtures/reject/src/lib.rs:11:16 |
| 1 | `_`, which is not supported yet | `Hasher::write` | fixtures/reject/src/lib.rs:294:9 |
| 1 | a `zip` of `_` and `_`, which std reads by index, of items that drop, which is not supported yet | `std::iter::Map`, `std::ops::Range` | fixtures/reject/src/lib.rs:221:30 |
| 1 | a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` | `std::iter::Chain` | fixtures/reject/src/lib.rs:172:51 |
| 1 | a cast from `_` to `_` | `u64`, `f32` | fixtures/reject/src/lib.rs:96:2 |
| 1 | a cast from a pointer to `_` to one to `_`, which reads the bytes of one type as another | `u32`, `u8` | fixtures/reject/src/lib.rs:577:27 |
| 1 | a coercion to `_` | `std::sync::Arc` | fixtures/reject/src/lib.rs:341:2 |
| 1 | a coercion to `_`, which is not supported yet | `&Tail` | fixtures/reject/src/lib.rs:506:30 |
| 1 | a coercion to `_`, which is not supported yet | `std::sync::Weak` | fixtures/reject/src/lib.rs:470:40 |
| 1 | a constant of type `_` | `&u8` | fixtures/reject/src/lib.rs:80:2 |
| 1 | a dangling pointer to `_`, a value of no bytes made of more than structs and tuples | `Only` | fixtures/reject/src/lib.rs:662:11 |
| 1 | a future of `_`, which can be `rt.Pending`, as the future's own `Pending` is | `std::task::Poll` | fixtures/reject/src/lib.rs:110:34 |
| 1 | a mutable borrow of a subslice of `_`, which JS would copy, having no views | `u32` | fixtures/reject/src/lib.rs:226:13 |
| 1 | a raw pointer to `_` behind a reference, which is the object and not where it is kept | `std::vec::Vec` | fixtures/reject/src/lib.rs:599:10 |
| 1 | a raw pointer to `_` in `_`, which the crate may put another object in place of | `(u32, u32)`, `Span2` | fixtures/reject/src/lib.rs:630:10 |
| 1 | a raw pointer to `_` in a variable assigned again while it may point there | `(u32, u32)` | fixtures/reject/src/lib.rs:638:10 |
| 1 | a raw pointer to `_` in an element, a payload, or a union field, which is not supported yet | `(u32, u32)` | fixtures/reject/src/lib.rs:646:10 |
| 1 | a scalar constant of type `_` | `f32` | fixtures/reject/src/lib.rs:8:2 |
| 1 | a trait object of `_`, which is not supported yet | `[u8; 4]` | fixtures/reject/src/lib.rs:496:26 |
| 1 | a zero-sized constant of type `_` | `{async closure}` | fixtures/reject/src/lib.rs:62:2 |
| 1 | an assignment through a reference to `_` | `std::sync::Arc` | fixtures/reject/src/lib.rs:101:2 |
| 1 | an assignment through a reference to `_` | `std::sync::MutexGuard` | fixtures/reject/src/lib.rs:121:2 |
| 1 | an iterator of the crate implementing `_`, which is not supported yet | `DoubleEndedIterator` | fixtures/reject/src/lib.rs:159:1 |
| 1 | an iterator of the crate implementing `_`, which is not supported yet | `FusedIterator` | fixtures/reject/src/lib.rs:213:1 |
| 1 | an upcast from `_` to `_`, which is not supported yet | `dyn Keyed`, `dyn std::any::Any` | fixtures/reject/src/lib.rs:554:32 |
| 1 | arithmetic on `_` | `f32` | fixtures/reject/src/lib.rs:8:2 |
| 1 | collecting `_`, whose `None` is `rt.None`, which is not supported yet | `Option` | fixtures/reject/src/lib.rs:36:43 |
| 1 | comparing `_` as a key, which is not supported yet | `std::vec::Vec` | fixtures/reject/src/lib.rs:268:52 |
| 1 | comparing pointers to `_`, each a place JS makes of its own | `u32` | fixtures/reject/src/lib.rs:618:2 |
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
| 1 | union `_`, whose fields overlap in `_` and `f64`, which JS keeps apart | `Bits`, `u32` | fixtures/reject/src/lib.rs:604:1 |
| 1 | unsupported call to `_` | `<f64 as std::fmt::Display>::fmt` | fixtures/reject/src/lib.rs:454:2 |
| 1 | unsupported call to `_` | `<std::any::TypeId as std::cmp::PartialEq>::eq` | fixtures/reject/src/lib.rs:560:2 |
| 1 | unsupported call to `_` | `<std::borrow::Cow as std::ops::Deref>::deref` | fixtures/reject/src/lib.rs:394:2 |
| 1 | unsupported call to `_` | `<std::option::Option as std::cmp::PartialEq>::eq` | fixtures/reject/src/lib.rs:420:2 |
| 1 | unsupported call to `_` | `<std::slice::Iter as std::iter::Iterator>::cycle` | fixtures/reject/src/lib.rs:126:11 |
| 1 | unsupported call to `_` | `<std::string::String as std::fmt::Write>::write_fmt` | fixtures/reject/src/lib.rs:74:10 |
| 1 | unsupported call to `_` | `<u8 as std::hash::Hash>::hash` | fixtures/reject/src/lib.rs:351:10 |
| 1 | unsupported call to `_` | `std::any::TypeId::of` | fixtures/reject/src/lib.rs:560:17 |
| 1 | unsupported call to `_` | `std::ops::Range::contains` | fixtures/reject/src/lib.rs:180:8 |
| 1 | unsupported call to `_` | `std::slice::copy_within` | fixtures/reject/src/lib.rs:336:4 |
| 1 | unsupported call to `_` | `std::slice::rotate_left` | fixtures/reject/src/lib.rs:21:4 |
| 1 | unsupported call to `_` | `std::std::Layout::new` | fixtures/reject/src/lib.rs:587:15 |
| 1 | unsupported call to `_` | `std::std::alloc` | fixtures/reject/src/lib.rs:589:11 |
| 1 | unsupported call to `_` | `std::std::dealloc` | fixtures/reject/src/lib.rs:592:3 |

## Spans

<details><summary>unsupported call to `_` (16)</summary>

- fixtures/reject/src/lib.rs:21:4: unsupported call to `core::slice::<impl [u8]>::rotate_left`
- fixtures/reject/src/lib.rs:74:10: unsupported call to `<std::string::String as std::fmt::Write>::write_fmt`
- fixtures/reject/src/lib.rs:126:11: unsupported call to `<std::slice::Iter<'_, u8> as std::iter::Iterator>::cycle`
- fixtures/reject/src/lib.rs:180:8: unsupported call to `std::ops::Range::<Key>::contains::<Key>`
- fixtures/reject/src/lib.rs:336:4: unsupported call to `core::slice::<impl [(u8, u8)]>::copy_within::<std::ops::Range<usize>>`
- fixtures/reject/src/lib.rs:351:10: unsupported call to `<u8 as std::hash::Hash>::hash::<H>`
- fixtures/reject/src/lib.rs:394:2: unsupported call to `<std::borrow::Cow<'_, [u8]> as std::ops::Deref>::deref`
- fixtures/reject/src/lib.rs:420:2: unsupported call to `<std::option::Option<&mut u32> as std::cmp::PartialEq>::eq`
- fixtures/reject/src/lib.rs:444:17: unsupported call to `<std::vec::IntoIter<Part> as std::iter::Iterator>::sum::<std::time::Duration>`
- fixtures/reject/src/lib.rs:444:38: unsupported call to `<std::vec::IntoIter<Part> as std::iter::Iterator>::sum::<u64>`
- fixtures/reject/src/lib.rs:454:2: unsupported call to `<f64 as std::fmt::Display>::fmt`
- fixtures/reject/src/lib.rs:560:2: unsupported call to `<std::any::TypeId as std::cmp::PartialEq>::eq`
- fixtures/reject/src/lib.rs:560:17: unsupported call to `std::any::TypeId::of::<u8>`
- fixtures/reject/src/lib.rs:587:15: unsupported call to `std::alloc::Layout::new::<u8>`
- fixtures/reject/src/lib.rs:589:11: unsupported call to `std::alloc::alloc`
- fixtures/reject/src/lib.rs:592:3: unsupported call to `std::alloc::dealloc`

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

<details><summary>a copy of a closure that changes what it captured (3)</summary>

- fixtures/reject/src/lib.rs:257:14: a copy of a closure that changes what it captured
- fixtures/reject/src/lib.rs:529:14: a copy of a closure that changes what it captured
- fixtures/reject/src/lib.rs:530:14: a copy of a closure that changes what it captured

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

- fixtures/reject/src/lib.rs:479:23: `dyn Scaled<u8>`, a trait object of the generic trait `Scaled`, which is not supported yet
- fixtures/reject/src/lib.rs:480:4: `dyn Scaled<u8>`, a trait object of the generic trait `Scaled`, which is not supported yet

</details>

<details><summary>a coercion to `_`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:470:40: a coercion to `std::sync::Weak<dyn Reading>`, which is not supported yet
- fixtures/reject/src/lib.rs:506:30: a coercion to `&Tail<dyn Reading>`, which is not supported yet

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

<details><summary>`Bytes::from_owner` of `_`, which would drop with the last clone of what it makes, which is not supported (1)</summary>

- fixtures/reject/src/lib.rs:425:2: `Bytes::from_owner` of `std::sync::Arc<[u8]>`, which would drop with the last clone of what it makes, which is not supported

</details>

<details><summary>`Pointer` formatting, as in `{:p}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:68:2: `Pointer` formatting, as in `{:p}`, is not supported

</details>

<details><summary>`_` is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:393:18: `std::borrow::Cow<'_, [u8]>` is not supported yet

</details>

<details><summary>`_` is not supported: its `Pending` and `Ready` would both be `rt.OuterPending` (1)</summary>

- fixtures/reject/src/lib.rs:105:20: `Poll<std::option::Option<std::task::Poll<std::task::Poll<u8>>>>` is not supported: its `Pending` and `Ready` would both be `rt.OuterPending`

</details>

<details><summary>`_` is not supported: its field `_` is `_`, typed `_`, whose `None` would be `rt.None` as `_` (1)</summary>

- fixtures/reject/src/lib.rs:31:2: `Maybe<()>` is not supported: its field `value` is `Option<T>`, typed `T | undefined`, whose `None` would be `rt.None` as `Option<()>`

</details>

<details><summary>`_` is not supported: its field `_` is `_`, typed `_`, whose `Pending` would be `rt.OuterPending` as `_` (1)</summary>

- fixtures/reject/src/lib.rs:517:2: `Waiting<std::task::Poll<u8>>` is not supported: its field `poll` is `Poll<T>`, typed `rt.Poll<T>`, whose `Pending` would be `rt.OuterPending` as `Poll<std::task::Poll<u8>>`

</details>

<details><summary>`_` of `_` by `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:286:6: `HashMap::contains_key` of `std::collections::HashMap<Id, u8>` by `u8`, which is not supported yet

</details>

<details><summary>`_` of `_`, which drops `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:53:13: `into_keys` of `std::collections::HashMap<u8, Noisy>`, which drops `Noisy`, which is not supported yet

</details>

<details><summary>`_` of `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:560:4: `std::any::Any::type_id` of `dyn std::any::Any`, which is not supported yet

</details>

<details><summary>`_` through a held type's own, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:318:2: `<std::sync::Arc<Picky> as PartialEq>::ne` through a held type's own, which is not supported yet

</details>

<details><summary>`_` with a mutable reference to a value JS copies, which its mapping would copy (1)</summary>

- fixtures/reject/src/lib.rs:536:12: `std::sync::Mutex::<T>::get_mut` with a mutable reference to a value JS copies, which its mapping would copy

</details>

<details><summary>`_`, a trait object of another crate's trait, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:11:16: `dyn std::ops::Fn(u8) -> u8`, a trait object of another crate's trait, which is not supported yet

</details>

<details><summary>`_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:294:9: `Hasher::write`, which is not supported yet

</details>

<details><summary>`f64::mul_add`, which rounds once, where JS would round twice (1)</summary>

- fixtures/reject/src/lib.rs:449:4: `f64::mul_add`, which rounds once, where JS would round twice

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

<details><summary>a cast from a pointer to `_` to one to `_`, which reads the bytes of one type as another (1)</summary>

- fixtures/reject/src/lib.rs:577:27: a cast from a pointer to `u32` to one to `u8`, which reads the bytes of one type as another

</details>

<details><summary>a coercion to `_` (1)</summary>

- fixtures/reject/src/lib.rs:341:2: a coercion to `std::sync::Arc<[u8]>`

</details>

<details><summary>a constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:80:2: a constant of type `&u8`

</details>

<details><summary>a dangling pointer to `_`, a value of no bytes made of more than structs and tuples (1)</summary>

- fixtures/reject/src/lib.rs:662:11: a dangling pointer to `Only`, a value of no bytes made of more than structs and tuples

</details>

<details><summary>a future of `_`, which can be `rt.Pending`, as the future's own `Pending` is (1)</summary>

- fixtures/reject/src/lib.rs:110:34: a future of `std::task::Poll<u8>`, which can be `rt.Pending`, as the future's own `Pending` is

</details>

<details><summary>a mutable borrow of a subslice of `_`, which JS would copy, having no views (1)</summary>

- fixtures/reject/src/lib.rs:226:13: a mutable borrow of a subslice of `u32`, which JS would copy, having no views

</details>

<details><summary>a mutable reference into a `Result`'s payload, passed on or kept (1)</summary>

- fixtures/reject/src/lib.rs:413:12: a mutable reference into a `Result`'s payload, passed on or kept

</details>

<details><summary>a pointer's address, which JS has no counterpart of (1)</summary>

- fixtures/reject/src/lib.rs:582:17: a pointer's address, which JS has no counterpart of

</details>

<details><summary>a printable type must implement `Display` or `Debug` (1)</summary>

- fixtures/reject/src/lib.rs:92:1: a printable type must implement `Display` or `Debug`

</details>

<details><summary>a raw pointer to `_` behind a reference, which is the object and not where it is kept (1)</summary>

- fixtures/reject/src/lib.rs:599:10: a raw pointer to `std::vec::Vec<u8>` behind a reference, which is the object and not where it is kept

</details>

<details><summary>a raw pointer to `_` in `_`, which the crate may put another object in place of (1)</summary>

- fixtures/reject/src/lib.rs:630:10: a raw pointer to `(u32, u32)` in `Span2`, which the crate may put another object in place of

</details>

<details><summary>a raw pointer to `_` in a variable assigned again while it may point there (1)</summary>

- fixtures/reject/src/lib.rs:638:10: a raw pointer to `(u32, u32)` in a variable assigned again while it may point there

</details>

<details><summary>a raw pointer to `_` in an element, a payload, or a union field, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:646:10: a raw pointer to `(u32, u32)` in an element, a payload, or a union field, which is not supported yet

</details>

<details><summary>a scalar constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:8:2: a scalar constant of type `f32`

</details>

<details><summary>a trait object of `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:496:26: a trait object of `[u8; 4]`, which is not supported yet

</details>

<details><summary>a zero-sized constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:62:2: a zero-sized constant of type `{async closure@fixtures/reject/src/lib.rs:61:10: 61:18}`

</details>

<details><summary>an upcast from `_` to `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:554:32: an upcast from `dyn Keyed` to `dyn std::any::Any`, which is not supported yet

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

<details><summary>comparing pointers to `()`, which has no bytes and so no address of its own (1)</summary>

- fixtures/reject/src/lib.rs:653:2: comparing pointers to `()`, which has no bytes and so no address of its own

</details>

<details><summary>comparing pointers to `_`, each a place JS makes of its own (1)</summary>

- fixtures/reject/src/lib.rs:618:2: comparing pointers to `u32`, each a place JS makes of its own

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

<details><summary>union `_`, whose fields overlap in `_` and `f64`, which JS keeps apart (1)</summary>

- fixtures/reject/src/lib.rs:604:1: union `Bits`, whose fields overlap in `u32` and `f64`, which JS keeps apart

</details>
