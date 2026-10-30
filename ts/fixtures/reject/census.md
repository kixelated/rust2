# Census of `reject-fixture`

- Refusals: 104
- Function instances: 120
- Generic functions no instance reached, lowered over their type parameters: 7

## By construct

| Count | Construct | First |
| ---: | --- | --- |
| 17 | unsupported call to `_` | fixtures/reject/src/lib.rs:21:4 |
| 4 | a variant of `_`, which is not supported: either is the same JS string | fixtures/reject/src/lib.rs:385:2 |
| 3 | `_` of `_`, whose `None` is `rt.None`, which is not supported yet | fixtures/reject/src/lib.rs:41:42 |
| 3 | a copy of a closure that changes what it captured | fixtures/reject/src/lib.rs:260:14 |
| 3 | an assignment through a reference to `_` | fixtures/reject/src/lib.rs:102:2 |
| 3 | replacing the value behind a reference to `_` | fixtures/reject/src/lib.rs:110:2 |
| 3 | unsupported type `_` | fixtures/reject/src/lib.rs:7:14 |
| 2 | `_` is not supported: its `None` and `Some` would both be `rt.None` | fixtures/reject/src/lib.rs:16:15 |
| 2 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | fixtures/reject/src/lib.rs:250:16 |
| 2 | `_`, a trait object of another crate's trait, which is not supported yet | fixtures/reject/src/lib.rs:11:16 |
| 2 | `_`, a trait object of the generic trait `_`, which is not supported yet | fixtures/reject/src/lib.rs:518:23 |
| 2 | a coercion to `_`, which is not supported yet | fixtures/reject/src/lib.rs:509:40 |
| 2 | an iterator of the crate implementing `_`, which is not supported yet | fixtures/reject/src/lib.rs:169:1 |
| 2 | constructing `_` from another crate | fixtures/reject/src/lib.rs:155:23 |
| 2 | overriding `_`, which std's adaptors call in place of `next` or beside it | fixtures/reject/src/lib.rs:149:2 |
| 2 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | fixtures/reject/src/lib.rs:372:13 |
| 2 | passing `_`, which drops, to `_`, which is not supported yet | fixtures/reject/src/lib.rs:380:4 |
| 1 | `Buf::copy_to_bytes` of `_`, a value JS copies, which is not supported | fixtures/reject/src/lib.rs:725:2 |
| 1 | `BufMut::put` of `_` into a `BytesMut`, which picks the methods it calls by a capacity that depends on whether the other handles of its storage dropped, which is not supported | fixtures/reject/src/lib.rs:443:2 |
| 1 | `Bytes::from_owner` of `_`, which would drop with the last clone of what it makes, which is not supported | fixtures/reject/src/lib.rs:428:2 |
| 1 | `Pointer` formatting, as in `{:p}`, is not supported | fixtures/reject/src/lib.rs:68:2 |
| 1 | `_` is not supported yet | fixtures/reject/src/lib.rs:396:18 |
| 1 | `_` is not supported: its `Pending` and `Ready` would both be `rt.OuterPending` | fixtures/reject/src/lib.rs:91:20 |
| 1 | `_` is not supported: its field `_` is `_`, typed `_`, whose `None` would be `rt.None` as `_` | fixtures/reject/src/lib.rs:31:2 |
| 1 | `_` is not supported: its field `_` is `_`, typed `_`, whose `Pending` would be `rt.OuterPending` as `_` | fixtures/reject/src/lib.rs:556:2 |
| 1 | `_` of `_` by `_`, which is not supported yet | fixtures/reject/src/lib.rs:289:6 |
| 1 | `_` of `_`, which drops `_`, which is not supported yet | fixtures/reject/src/lib.rs:53:13 |
| 1 | `_` of `_`, which is not supported yet | fixtures/reject/src/lib.rs:599:4 |
| 1 | `_` through a held type's own, which is not supported yet | fixtures/reject/src/lib.rs:321:2 |
| 1 | `_` with a mutable reference to a value JS copies, which its mapping would copy | fixtures/reject/src/lib.rs:575:12 |
| 1 | `_`, which is not supported yet | fixtures/reject/src/lib.rs:297:9 |
| 1 | `_`, which reads a capacity that depends on whether the other handles of its storage dropped, which is not supported | fixtures/reject/src/lib.rs:437:6 |
| 1 | `f64::mul_add`, which rounds once, where JS would round twice | fixtures/reject/src/lib.rs:488:4 |
| 1 | a `zip` of `_` and `_`, which std reads by index, of items that drop, which is not supported yet | fixtures/reject/src/lib.rs:231:30 |
| 1 | a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` | fixtures/reject/src/lib.rs:182:51 |
| 1 | a cast from `_` to `_` | fixtures/reject/src/lib.rs:87:2 |
| 1 | a cast from a pointer to `_` to one to `_`, which reads the bytes of one type as another | fixtures/reject/src/lib.rs:616:27 |
| 1 | a coercion to `_` | fixtures/reject/src/lib.rs:344:2 |
| 1 | a constant of type `_`, a pointer into memory translation can't name | fixtures/reject/src/lib.rs:747:3 |
| 1 | a dangling pointer to `_`, a value of no bytes made of more than structs and tuples | fixtures/reject/src/lib.rs:701:11 |
| 1 | a future of `_`, which can be `rt.Pending`, as the future's own `Pending` is | fixtures/reject/src/lib.rs:96:34 |
| 1 | a mutable borrow of a subslice of `_`, which JS would copy, having no views | fixtures/reject/src/lib.rs:236:13 |
| 1 | a mutable reference into a `Result`'s payload, passed on or kept | fixtures/reject/src/lib.rs:416:12 |
| 1 | a mutable static, which any code may write | fixtures/reject/src/lib.rs:80:11 |
| 1 | a pointer's address, which JS has no counterpart of | fixtures/reject/src/lib.rs:621:17 |
| 1 | a printable type must implement `Display` or `Debug` | fixtures/reject/src/lib.rs:83:1 |
| 1 | a raw pointer to `_` behind a reference, which is the object and not where it is kept | fixtures/reject/src/lib.rs:638:10 |
| 1 | a raw pointer to `_` in `_`, which the crate may put another object in place of | fixtures/reject/src/lib.rs:669:10 |
| 1 | a raw pointer to `_` in a variable assigned again while it may point there | fixtures/reject/src/lib.rs:677:10 |
| 1 | a raw pointer to `_` in an element, a payload, or a union field, which is not supported yet | fixtures/reject/src/lib.rs:685:10 |
| 1 | a scalar constant of type `_` | fixtures/reject/src/lib.rs:8:2 |
| 1 | a trait object of `_`, which is not supported yet | fixtures/reject/src/lib.rs:535:26 |
| 1 | a zero-sized constant of type `_` | fixtures/reject/src/lib.rs:62:2 |
| 1 | an upcast from `_` to `_`, which is not supported yet | fixtures/reject/src/lib.rs:593:32 |
| 1 | arithmetic on `_` | fixtures/reject/src/lib.rs:8:2 |
| 1 | collecting `_`, whose `None` is `rt.None`, which is not supported yet | fixtures/reject/src/lib.rs:36:43 |
| 1 | comparing `_` as a key, which is not supported yet | fixtures/reject/src/lib.rs:271:52 |
| 1 | comparing pointers to `()`, which has no bytes and so no address of its own | fixtures/reject/src/lib.rs:692:2 |
| 1 | comparing pointers to `_` in an atomic, which JS has no address of | fixtures/reject/src/lib.rs:746:15 |
| 1 | comparing pointers to `_`, each a place JS makes of its own | fixtures/reject/src/lib.rs:657:2 |
| 1 | constructing `_` | fixtures/reject/src/lib.rs:403:53 |
| 1 | hashing `_` as a key, which is not supported yet | fixtures/reject/src/lib.rs:271:52 |
| 1 | hashing `_` through `_`'s own `Hash::hash_slice`, which is not supported yet | fixtures/reject/src/lib.rs:363:16 |
| 1 | union `_`, whose fields overlap in `_` and `f64`, which JS keeps apart | fixtures/reject/src/lib.rs:643:1 |
| 1 | unsupported type `_` from another crate | fixtures/reject/src/lib.rs:492:31 |

## By API

The type or module each refused type or path belongs to.

| Count | API | First |
| ---: | --- | --- |
| 7 | `Option` | fixtures/reject/src/lib.rs:16:15 |
| 7 | `std::borrow::Cow` | fixtures/reject/src/lib.rs:385:2 |
| 6 | `f32` | fixtures/reject/src/lib.rs:7:14 |
| 6 | `u32` | fixtures/reject/src/lib.rs:236:13 |
| 4 | `Iterator` | fixtures/reject/src/lib.rs:41:42 |
| 4 | `std::pin::Pin` | fixtures/reject/src/lib.rs:110:2 |
| 4 | `std::vec::Vec` | fixtures/reject/src/lib.rs:271:52 |
| 3 | `(u32, u32)` | fixtures/reject/src/lib.rs:669:10 |
| 3 | `std::sync::Arc` | fixtures/reject/src/lib.rs:321:2 |
| 3 | `u8` | fixtures/reject/src/lib.rs:289:6 |
| 3 | `{closure}` | fixtures/reject/src/lib.rs:372:13 |
| 2 | `Poll` | fixtures/reject/src/lib.rs:91:20 |
| 2 | `Scaled` | fixtures/reject/src/lib.rs:518:23 |
| 2 | `dyn Scaled` | fixtures/reject/src/lib.rs:518:23 |
| 2 | `dyn std::any::Any` | fixtures/reject/src/lib.rs:593:32 |
| 2 | `dyn std::iter::Iterator` | fixtures/reject/src/lib.rs:11:16 |
| 2 | `std::any::TypeId` | fixtures/reject/src/lib.rs:599:2 |
| 2 | `std::collections::HashMap` | fixtures/reject/src/lib.rs:53:13 |
| 2 | `std::iter::Iterator` | fixtures/reject/src/lib.rs:250:16 |
| 2 | `std::iter::Map` | fixtures/reject/src/lib.rs:155:23 |
| 2 | `std::ops::Range` | fixtures/reject/src/lib.rs:190:8 |
| 2 | `std::slice` | fixtures/reject/src/lib.rs:21:4 |
| 2 | `std::std` | fixtures/reject/src/lib.rs:628:11 |
| 2 | `std::sync::MutexGuard` | fixtures/reject/src/lib.rs:102:2 |
| 2 | `std::vec::IntoIter` | fixtures/reject/src/lib.rs:483:17 |
| 1 | `Batched` | fixtures/reject/src/lib.rs:363:16 |
| 1 | `BinaryHeap` | fixtures/reject/src/lib.rs:730:7 |
| 1 | `Bits` | fixtures/reject/src/lib.rs:643:1 |
| 1 | `Blank` | fixtures/reject/src/lib.rs:725:2 |
| 1 | `BytesMut` | fixtures/reject/src/lib.rs:437:6 |
| 1 | `Counted` | fixtures/reject/src/lib.rs:443:2 |
| 1 | `DoubleEndedIterator` | fixtures/reject/src/lib.rs:169:1 |
| 1 | `Entry` | fixtures/reject/src/lib.rs:372:13 |
| 1 | `FusedIterator` | fixtures/reject/src/lib.rs:223:1 |
| 1 | `HashMap` | fixtures/reject/src/lib.rs:289:6 |
| 1 | `HashSet` | fixtures/reject/src/lib.rs:266:6 |
| 1 | `Hasher` | fixtures/reject/src/lib.rs:297:9 |
| 1 | `Maybe` | fixtures/reject/src/lib.rs:31:2 |
| 1 | `Noisy` | fixtures/reject/src/lib.rs:53:13 |
| 1 | `Only` | fixtures/reject/src/lib.rs:701:11 |
| 1 | `Span2` | fixtures/reject/src/lib.rs:669:10 |
| 1 | `Spent` | fixtures/reject/src/lib.rs:768:4 |
| 1 | `T \| undefined` | fixtures/reject/src/lib.rs:31:2 |
| 1 | `Tail` | fixtures/reject/src/lib.rs:545:30 |
| 1 | `Vec` | fixtures/reject/src/lib.rs:768:4 |
| 1 | `Waiting` | fixtures/reject/src/lib.rs:556:2 |
| 1 | `[u8; 4]` | fixtures/reject/src/lib.rs:535:26 |
| 1 | `dyn Keyed` | fixtures/reject/src/lib.rs:593:32 |
| 1 | `into_keys` | fixtures/reject/src/lib.rs:53:13 |
| 1 | `poll` | fixtures/reject/src/lib.rs:556:2 |
| 1 | `rt.Poll` | fixtures/reject/src/lib.rs:556:2 |
| 1 | `std::any::Any` | fixtures/reject/src/lib.rs:599:4 |
| 1 | `std::iter::Chain` | fixtures/reject/src/lib.rs:182:51 |
| 1 | `std::iter::Rev` | fixtures/reject/src/lib.rs:176:14 |
| 1 | `std::num::ParseIntError` | fixtures/reject/src/lib.rs:492:31 |
| 1 | `std::option::Option` | fixtures/reject/src/lib.rs:423:2 |
| 1 | `std::slice::IterMut` | fixtures/reject/src/lib.rs:735:15 |
| 1 | `std::slice::Iter` | fixtures/reject/src/lib.rs:136:11 |
| 1 | `std::std::Layout` | fixtures/reject/src/lib.rs:626:15 |
| 1 | `std::str` | fixtures/reject/src/lib.rs:493:4 |
| 1 | `std::string::String` | fixtures/reject/src/lib.rs:74:10 |
| 1 | `std::sync::Mutex` | fixtures/reject/src/lib.rs:575:12 |
| 1 | `std::sync::Weak` | fixtures/reject/src/lib.rs:509:40 |
| 1 | `std::task::Poll` | fixtures/reject/src/lib.rs:96:34 |
| 1 | `str` | fixtures/reject/src/lib.rs:380:4 |
| 1 | `u64` | fixtures/reject/src/lib.rs:87:2 |
| 1 | `value` | fixtures/reject/src/lib.rs:31:2 |
| 1 | `{async closure}` | fixtures/reject/src/lib.rs:62:2 |

## By type or path

| Count | Construct | Type or path | First |
| ---: | --- | --- | --- |
| 4 | a variant of `_`, which is not supported: either is the same JS string | `std::borrow::Cow` | fixtures/reject/src/lib.rs:385:2 |
| 3 | unsupported type `_` | `f32` | fixtures/reject/src/lib.rs:7:14 |
| 2 | `_` is not supported: its `None` and `Some` would both be `rt.None` | `Option` | fixtures/reject/src/lib.rs:16:15 |
| 2 | `_`, a trait object of another crate's trait, which is not supported yet | `dyn std::iter::Iterator` | fixtures/reject/src/lib.rs:11:16 |
| 2 | `_`, a trait object of the generic trait `_`, which is not supported yet | `dyn Scaled`, `Scaled` | fixtures/reject/src/lib.rs:518:23 |
| 2 | an assignment through a reference to `_` | `std::pin::Pin` | fixtures/reject/src/lib.rs:126:2 |
| 2 | replacing the value behind a reference to `_` | `std::pin::Pin` | fixtures/reject/src/lib.rs:110:2 |
| 2 | unsupported call to `_` | `<std::vec::IntoIter as std::iter::Iterator>::sum` | fixtures/reject/src/lib.rs:483:17 |
| 1 | `Buf::copy_to_bytes` of `_`, a value JS copies, which is not supported | `Blank` | fixtures/reject/src/lib.rs:725:2 |
| 1 | `BufMut::put` of `_` into a `BytesMut`, which picks the methods it calls by a capacity that depends on whether the other handles of its storage dropped, which is not supported | `Counted` | fixtures/reject/src/lib.rs:443:2 |
| 1 | `Bytes::from_owner` of `_`, which would drop with the last clone of what it makes, which is not supported | `std::sync::Arc` | fixtures/reject/src/lib.rs:428:2 |
| 1 | `_` is not supported yet | `std::borrow::Cow` | fixtures/reject/src/lib.rs:396:18 |
| 1 | `_` is not supported: its `Pending` and `Ready` would both be `rt.OuterPending` | `Poll` | fixtures/reject/src/lib.rs:91:20 |
| 1 | `_` is not supported: its field `_` is `_`, typed `_`, whose `None` would be `rt.None` as `_` | `Maybe`, `value`, `Option`, `T \| undefined`, `Option` | fixtures/reject/src/lib.rs:31:2 |
| 1 | `_` is not supported: its field `_` is `_`, typed `_`, whose `Pending` would be `rt.OuterPending` as `_` | `Waiting`, `poll`, `Poll`, `rt.Poll`, `Poll` | fixtures/reject/src/lib.rs:556:2 |
| 1 | `_` of `_` by `_`, which is not supported yet | `HashMap::contains_key`, `std::collections::HashMap`, `u8` | fixtures/reject/src/lib.rs:289:6 |
| 1 | `_` of `_`, which drops `_`, which is not supported yet | `into_keys`, `std::collections::HashMap`, `Noisy` | fixtures/reject/src/lib.rs:53:13 |
| 1 | `_` of `_`, which is not supported yet | `std::any::Any::type_id`, `dyn std::any::Any` | fixtures/reject/src/lib.rs:599:4 |
| 1 | `_` of `_`, whose `None` is `rt.None`, which is not supported yet | `BinaryHeap::pop`, `Option` | fixtures/reject/src/lib.rs:730:7 |
| 1 | `_` of `_`, whose `None` is `rt.None`, which is not supported yet | `HashSet::get`, `Option` | fixtures/reject/src/lib.rs:266:6 |
| 1 | `_` of `_`, whose `None` is `rt.None`, which is not supported yet | `Iterator::find`, `Option` | fixtures/reject/src/lib.rs:41:42 |
| 1 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | `std::iter::Iterator::count` | fixtures/reject/src/lib.rs:250:38 |
| 1 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | `std::iter::Iterator::filter` | fixtures/reject/src/lib.rs:250:16 |
| 1 | `_` through a held type's own, which is not supported yet | `<std::sync::Arc as PartialEq>::ne` | fixtures/reject/src/lib.rs:321:2 |
| 1 | `_` with a mutable reference to a value JS copies, which its mapping would copy | `std::sync::Mutex::get_mut` | fixtures/reject/src/lib.rs:575:12 |
| 1 | `_`, which is not supported yet | `Hasher::write` | fixtures/reject/src/lib.rs:297:9 |
| 1 | `_`, which reads a capacity that depends on whether the other handles of its storage dropped, which is not supported | `BytesMut::capacity` | fixtures/reject/src/lib.rs:437:6 |
| 1 | a `zip` of `_` and `_`, which std reads by index, of items that drop, which is not supported yet | `std::iter::Map`, `std::ops::Range` | fixtures/reject/src/lib.rs:231:30 |
| 1 | a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` | `std::iter::Chain` | fixtures/reject/src/lib.rs:182:51 |
| 1 | a cast from `_` to `_` | `u64`, `f32` | fixtures/reject/src/lib.rs:87:2 |
| 1 | a cast from a pointer to `_` to one to `_`, which reads the bytes of one type as another | `u32`, `u8` | fixtures/reject/src/lib.rs:616:27 |
| 1 | a coercion to `_` | `std::sync::Arc` | fixtures/reject/src/lib.rs:344:2 |
| 1 | a coercion to `_`, which is not supported yet | `&Tail` | fixtures/reject/src/lib.rs:545:30 |
| 1 | a coercion to `_`, which is not supported yet | `std::sync::Weak` | fixtures/reject/src/lib.rs:509:40 |
| 1 | a constant of type `_`, a pointer into memory translation can't name | `&u32` | fixtures/reject/src/lib.rs:747:3 |
| 1 | a dangling pointer to `_`, a value of no bytes made of more than structs and tuples | `Only` | fixtures/reject/src/lib.rs:701:11 |
| 1 | a future of `_`, which can be `rt.Pending`, as the future's own `Pending` is | `std::task::Poll` | fixtures/reject/src/lib.rs:96:34 |
| 1 | a mutable borrow of a subslice of `_`, which JS would copy, having no views | `u32` | fixtures/reject/src/lib.rs:236:13 |
| 1 | a raw pointer to `_` behind a reference, which is the object and not where it is kept | `std::vec::Vec` | fixtures/reject/src/lib.rs:638:10 |
| 1 | a raw pointer to `_` in `_`, which the crate may put another object in place of | `(u32, u32)`, `Span2` | fixtures/reject/src/lib.rs:669:10 |
| 1 | a raw pointer to `_` in a variable assigned again while it may point there | `(u32, u32)` | fixtures/reject/src/lib.rs:677:10 |
| 1 | a raw pointer to `_` in an element, a payload, or a union field, which is not supported yet | `(u32, u32)` | fixtures/reject/src/lib.rs:685:10 |
| 1 | a scalar constant of type `_` | `f32` | fixtures/reject/src/lib.rs:8:2 |
| 1 | a trait object of `_`, which is not supported yet | `[u8; 4]` | fixtures/reject/src/lib.rs:535:26 |
| 1 | a zero-sized constant of type `_` | `{async closure}` | fixtures/reject/src/lib.rs:62:2 |
| 1 | an assignment through a reference to `_` | `std::sync::MutexGuard` | fixtures/reject/src/lib.rs:102:2 |
| 1 | an iterator of the crate implementing `_`, which is not supported yet | `DoubleEndedIterator` | fixtures/reject/src/lib.rs:169:1 |
| 1 | an iterator of the crate implementing `_`, which is not supported yet | `FusedIterator` | fixtures/reject/src/lib.rs:223:1 |
| 1 | an upcast from `_` to `_`, which is not supported yet | `dyn Keyed`, `dyn std::any::Any` | fixtures/reject/src/lib.rs:593:32 |
| 1 | arithmetic on `_` | `f32` | fixtures/reject/src/lib.rs:8:2 |
| 1 | collecting `_`, whose `None` is `rt.None`, which is not supported yet | `Option` | fixtures/reject/src/lib.rs:36:43 |
| 1 | comparing `_` as a key, which is not supported yet | `std::vec::Vec` | fixtures/reject/src/lib.rs:271:52 |
| 1 | comparing pointers to `_` in an atomic, which JS has no address of | `u32` | fixtures/reject/src/lib.rs:746:15 |
| 1 | comparing pointers to `_`, each a place JS makes of its own | `u32` | fixtures/reject/src/lib.rs:657:2 |
| 1 | constructing `_` | `std::borrow::Cow` | fixtures/reject/src/lib.rs:403:53 |
| 1 | constructing `_` from another crate | `std::iter::Map` | fixtures/reject/src/lib.rs:155:23 |
| 1 | constructing `_` from another crate | `std::iter::Rev` | fixtures/reject/src/lib.rs:176:14 |
| 1 | hashing `_` as a key, which is not supported yet | `std::vec::Vec` | fixtures/reject/src/lib.rs:271:52 |
| 1 | hashing `_` through `_`'s own `Hash::hash_slice`, which is not supported yet | `std::vec::Vec`, `Batched` | fixtures/reject/src/lib.rs:363:16 |
| 1 | overriding `_`, which std's adaptors call in place of `next` or beside it | `Iterator::fold` | fixtures/reject/src/lib.rs:149:2 |
| 1 | overriding `_`, which std's adaptors call in place of `next` or beside it | `Iterator::size_hint` | fixtures/reject/src/lib.rs:203:2 |
| 1 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | `{closure}`, `Entry::or_insert_with` | fixtures/reject/src/lib.rs:372:13 |
| 1 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | `{closure}`, `Iterator::for_each` | fixtures/reject/src/lib.rs:373:16 |
| 1 | passing `_`, which drops, to `_`, which is not supported yet | `Spent`, `Vec::drain` | fixtures/reject/src/lib.rs:768:4 |
| 1 | passing `_`, which drops, to `_`, which is not supported yet | `{closure}`, `str::contains` | fixtures/reject/src/lib.rs:380:4 |
| 1 | replacing the value behind a reference to `_` | `std::sync::MutexGuard` | fixtures/reject/src/lib.rs:131:2 |
| 1 | union `_`, whose fields overlap in `_` and `f64`, which JS keeps apart | `Bits`, `u32` | fixtures/reject/src/lib.rs:643:1 |
| 1 | unsupported call to `_` | `<std::any::TypeId as std::cmp::PartialEq>::eq` | fixtures/reject/src/lib.rs:599:2 |
| 1 | unsupported call to `_` | `<std::borrow::Cow as std::ops::Deref>::deref` | fixtures/reject/src/lib.rs:397:2 |
| 1 | unsupported call to `_` | `<std::option::Option as std::cmp::PartialEq>::eq` | fixtures/reject/src/lib.rs:423:2 |
| 1 | unsupported call to `_` | `<std::slice::Iter as std::iter::Iterator>::cycle` | fixtures/reject/src/lib.rs:136:11 |
| 1 | unsupported call to `_` | `<std::slice::IterMut as std::iter::Iterator>::is_sorted` | fixtures/reject/src/lib.rs:735:15 |
| 1 | unsupported call to `_` | `<std::string::String as std::fmt::Write>::write_fmt` | fixtures/reject/src/lib.rs:74:10 |
| 1 | unsupported call to `_` | `<u8 as std::hash::Hash>::hash` | fixtures/reject/src/lib.rs:354:10 |
| 1 | unsupported call to `_` | `std::any::TypeId::of` | fixtures/reject/src/lib.rs:599:17 |
| 1 | unsupported call to `_` | `std::ops::Range::contains` | fixtures/reject/src/lib.rs:190:8 |
| 1 | unsupported call to `_` | `std::slice::copy_within` | fixtures/reject/src/lib.rs:339:4 |
| 1 | unsupported call to `_` | `std::slice::rotate_left` | fixtures/reject/src/lib.rs:21:4 |
| 1 | unsupported call to `_` | `std::std::Layout::new` | fixtures/reject/src/lib.rs:626:15 |
| 1 | unsupported call to `_` | `std::std::alloc` | fixtures/reject/src/lib.rs:628:11 |
| 1 | unsupported call to `_` | `std::std::dealloc` | fixtures/reject/src/lib.rs:631:3 |
| 1 | unsupported call to `_` | `std::str::parse` | fixtures/reject/src/lib.rs:493:4 |
| 1 | unsupported type `_` from another crate | `std::num::ParseIntError` | fixtures/reject/src/lib.rs:492:31 |

## Spans

<details><summary>unsupported call to `_` (17)</summary>

- fixtures/reject/src/lib.rs:21:4: unsupported call to `core::slice::<impl [u8]>::rotate_left`
- fixtures/reject/src/lib.rs:74:10: unsupported call to `<std::string::String as std::fmt::Write>::write_fmt`
- fixtures/reject/src/lib.rs:136:11: unsupported call to `<std::slice::Iter<'_, u8> as std::iter::Iterator>::cycle`
- fixtures/reject/src/lib.rs:190:8: unsupported call to `std::ops::Range::<Key>::contains::<Key>`
- fixtures/reject/src/lib.rs:339:4: unsupported call to `core::slice::<impl [(u8, u8)]>::copy_within::<std::ops::Range<usize>>`
- fixtures/reject/src/lib.rs:354:10: unsupported call to `<u8 as std::hash::Hash>::hash::<H>`
- fixtures/reject/src/lib.rs:397:2: unsupported call to `<std::borrow::Cow<'_, [u8]> as std::ops::Deref>::deref`
- fixtures/reject/src/lib.rs:423:2: unsupported call to `<std::option::Option<&mut u32> as std::cmp::PartialEq>::eq`
- fixtures/reject/src/lib.rs:483:17: unsupported call to `<std::vec::IntoIter<Part> as std::iter::Iterator>::sum::<std::time::Duration>`
- fixtures/reject/src/lib.rs:483:38: unsupported call to `<std::vec::IntoIter<Part> as std::iter::Iterator>::sum::<u64>`
- fixtures/reject/src/lib.rs:493:4: unsupported call to `core::str::<impl str>::parse::<u32>`
- fixtures/reject/src/lib.rs:599:2: unsupported call to `<std::any::TypeId as std::cmp::PartialEq>::eq`
- fixtures/reject/src/lib.rs:599:17: unsupported call to `std::any::TypeId::of::<u8>`
- fixtures/reject/src/lib.rs:626:15: unsupported call to `std::alloc::Layout::new::<u8>`
- fixtures/reject/src/lib.rs:628:11: unsupported call to `std::alloc::alloc`
- fixtures/reject/src/lib.rs:631:3: unsupported call to `std::alloc::dealloc`
- fixtures/reject/src/lib.rs:735:15: unsupported call to `<std::slice::IterMut<'_, u8> as std::iter::Iterator>::is_sorted`

</details>

<details><summary>a variant of `_`, which is not supported: either is the same JS string (4)</summary>

- fixtures/reject/src/lib.rs:385:2: a variant of `std::borrow::Cow<'_, str>`, which is not supported: either is the same JS string
- fixtures/reject/src/lib.rs:386:30: a variant of `std::borrow::Cow<'_, str>`, which is not supported: either is the same JS string
- fixtures/reject/src/lib.rs:387:27: a variant of `std::borrow::Cow<'_, str>`, which is not supported: either is the same JS string
- fixtures/reject/src/lib.rs:392:2: a variant of `std::borrow::Cow<'_, str>`, which is not supported: either is the same JS string

</details>

<details><summary>`_` of `_`, whose `None` is `rt.None`, which is not supported yet (3)</summary>

- fixtures/reject/src/lib.rs:41:42: `Iterator::find` of `Option<std::option::Option<u8>>`, whose `None` is `rt.None`, which is not supported yet
- fixtures/reject/src/lib.rs:266:6: `HashSet::get` of `Option<&std::option::Option<u8>>`, whose `None` is `rt.None`, which is not supported yet
- fixtures/reject/src/lib.rs:730:7: `BinaryHeap::pop` of `Option<std::option::Option<u8>>`, whose `None` is `rt.None`, which is not supported yet

</details>

<details><summary>a copy of a closure that changes what it captured (3)</summary>

- fixtures/reject/src/lib.rs:260:14: a copy of a closure that changes what it captured
- fixtures/reject/src/lib.rs:568:14: a copy of a closure that changes what it captured
- fixtures/reject/src/lib.rs:569:14: a copy of a closure that changes what it captured

</details>

<details><summary>an assignment through a reference to `_` (3)</summary>

- fixtures/reject/src/lib.rs:102:2: an assignment through a reference to `std::sync::MutexGuard<'_, u8>`
- fixtures/reject/src/lib.rs:126:2: an assignment through a reference to `std::pin::Pin<std::boxed::Box<dyn std::future::Future<Output = u8>>>`
- fixtures/reject/src/lib.rs:244:2: an assignment through a reference to `std::pin::Pin<&mut Noisy>`

</details>

<details><summary>replacing the value behind a reference to `_` (3)</summary>

- fixtures/reject/src/lib.rs:110:2: replacing the value behind a reference to `std::pin::Pin<std::boxed::Box<std::future::PollFn<for<'a, 'b> fn(&'a mut std::task::Context<'b>) -> std::task::Poll<u8>>>>`
- fixtures/reject/src/lib.rs:118:2: replacing the value behind a reference to `std::pin::Pin<std::boxed::Box<[u8; 2]>>`
- fixtures/reject/src/lib.rs:131:2: replacing the value behind a reference to `std::sync::MutexGuard<'_, u8>`

</details>

<details><summary>unsupported type `_` (3)</summary>

- fixtures/reject/src/lib.rs:7:14: unsupported type `f32`
- fixtures/reject/src/lib.rs:7:25: unsupported type `f32`
- fixtures/reject/src/lib.rs:85:51: unsupported type `f32`

</details>

<details><summary>`_` is not supported: its `None` and `Some` would both be `rt.None` (2)</summary>

- fixtures/reject/src/lib.rs:16:15: `Option<std::option::Option<std::option::Option<u8>>>` is not supported: its `None` and `Some` would both be `rt.None`
- fixtures/reject/src/lib.rs:17:2: `Option<std::option::Option<std::option::Option<u8>>>` is not supported: its `None` and `Some` would both be `rt.None`

</details>

<details><summary>`_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it (2)</summary>

- fixtures/reject/src/lib.rs:250:16: `std::iter::Iterator::filter` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it
- fixtures/reject/src/lib.rs:250:38: `std::iter::Iterator::count` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it

</details>

<details><summary>`_`, a trait object of another crate's trait, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:11:16: `dyn std::iter::Iterator<Item = u8>`, a trait object of another crate's trait, which is not supported yet
- fixtures/reject/src/lib.rs:12:4: `dyn std::iter::Iterator<Item = u8>`, a trait object of another crate's trait, which is not supported yet

</details>

<details><summary>`_`, a trait object of the generic trait `_`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:518:23: `dyn Scaled<u8>`, a trait object of the generic trait `Scaled`, which is not supported yet
- fixtures/reject/src/lib.rs:519:4: `dyn Scaled<u8>`, a trait object of the generic trait `Scaled`, which is not supported yet

</details>

<details><summary>a coercion to `_`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:509:40: a coercion to `std::sync::Weak<dyn Reading>`, which is not supported yet
- fixtures/reject/src/lib.rs:545:30: a coercion to `&Tail<dyn Reading>`, which is not supported yet

</details>

<details><summary>an iterator of the crate implementing `_`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:169:1: an iterator of the crate implementing `DoubleEndedIterator`, which is not supported yet
- fixtures/reject/src/lib.rs:223:1: an iterator of the crate implementing `FusedIterator`, which is not supported yet

</details>

<details><summary>constructing `_` from another crate (2)</summary>

- fixtures/reject/src/lib.rs:155:23: constructing `std::iter::Map<Folds, {closure@fixtures/reject/src/lib.rs:155:12: 155:15}>` from another crate
- fixtures/reject/src/lib.rs:176:14: constructing `std::iter::Rev<Backs>` from another crate

</details>

<details><summary>overriding `_`, which std's adaptors call in place of `next` or beside it (2)</summary>

- fixtures/reject/src/lib.rs:149:2: overriding `Iterator::fold`, which std's adaptors call in place of `next` or beside it
- fixtures/reject/src/lib.rs:203:2: overriding `Iterator::size_hint`, which std's adaptors call in place of `next` or beside it

</details>

<details><summary>passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:372:13: passing `{closure@fixtures/reject/src/lib.rs:372:28: 372:35}`, a closure that owns a value that drops, to `Entry::or_insert_with`, which is not supported yet
- fixtures/reject/src/lib.rs:373:16: passing `{closure@fixtures/reject/src/lib.rs:373:25: 373:33}`, a closure that owns a value that drops, to `Iterator::for_each`, which is not supported yet

</details>

<details><summary>passing `_`, which drops, to `_`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:380:4: passing `{closure@fixtures/reject/src/lib.rs:380:13: 380:27}`, which drops, to `str::contains`, which is not supported yet
- fixtures/reject/src/lib.rs:768:4: passing `Spent`, which drops, to `Vec::drain`, which is not supported yet

</details>

<details><summary>`Buf::copy_to_bytes` of `_`, a value JS copies, which is not supported (1)</summary>

- fixtures/reject/src/lib.rs:725:2: `Buf::copy_to_bytes` of `Blank`, a value JS copies, which is not supported

</details>

<details><summary>`BufMut::put` of `_` into a `BytesMut`, which picks the methods it calls by a capacity that depends on whether the other handles of its storage dropped, which is not supported (1)</summary>

- fixtures/reject/src/lib.rs:443:2: `BufMut::put` of `Counted` into a `BytesMut`, which picks the methods it calls by a capacity that depends on whether the other handles of its storage dropped, which is not supported

</details>

<details><summary>`Bytes::from_owner` of `_`, which would drop with the last clone of what it makes, which is not supported (1)</summary>

- fixtures/reject/src/lib.rs:428:2: `Bytes::from_owner` of `std::sync::Arc<[u8]>`, which would drop with the last clone of what it makes, which is not supported

</details>

<details><summary>`Pointer` formatting, as in `{:p}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:68:2: `Pointer` formatting, as in `{:p}`, is not supported

</details>

<details><summary>`_` is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:396:18: `std::borrow::Cow<'_, [u8]>` is not supported yet

</details>

<details><summary>`_` is not supported: its `Pending` and `Ready` would both be `rt.OuterPending` (1)</summary>

- fixtures/reject/src/lib.rs:91:20: `Poll<std::option::Option<std::task::Poll<std::task::Poll<u8>>>>` is not supported: its `Pending` and `Ready` would both be `rt.OuterPending`

</details>

<details><summary>`_` is not supported: its field `_` is `_`, typed `_`, whose `None` would be `rt.None` as `_` (1)</summary>

- fixtures/reject/src/lib.rs:31:2: `Maybe<()>` is not supported: its field `value` is `Option<T>`, typed `T | undefined`, whose `None` would be `rt.None` as `Option<()>`

</details>

<details><summary>`_` is not supported: its field `_` is `_`, typed `_`, whose `Pending` would be `rt.OuterPending` as `_` (1)</summary>

- fixtures/reject/src/lib.rs:556:2: `Waiting<std::task::Poll<u8>>` is not supported: its field `poll` is `Poll<T>`, typed `rt.Poll<T>`, whose `Pending` would be `rt.OuterPending` as `Poll<std::task::Poll<u8>>`

</details>

<details><summary>`_` of `_` by `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:289:6: `HashMap::contains_key` of `std::collections::HashMap<Id, u8>` by `u8`, which is not supported yet

</details>

<details><summary>`_` of `_`, which drops `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:53:13: `into_keys` of `std::collections::HashMap<u8, Noisy>`, which drops `Noisy`, which is not supported yet

</details>

<details><summary>`_` of `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:599:4: `std::any::Any::type_id` of `dyn std::any::Any`, which is not supported yet

</details>

<details><summary>`_` through a held type's own, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:321:2: `<std::sync::Arc<Picky> as PartialEq>::ne` through a held type's own, which is not supported yet

</details>

<details><summary>`_` with a mutable reference to a value JS copies, which its mapping would copy (1)</summary>

- fixtures/reject/src/lib.rs:575:12: `std::sync::Mutex::<T>::get_mut` with a mutable reference to a value JS copies, which its mapping would copy

</details>

<details><summary>`_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:297:9: `Hasher::write`, which is not supported yet

</details>

<details><summary>`_`, which reads a capacity that depends on whether the other handles of its storage dropped, which is not supported (1)</summary>

- fixtures/reject/src/lib.rs:437:6: `BytesMut::capacity`, which reads a capacity that depends on whether the other handles of its storage dropped, which is not supported

</details>

<details><summary>`f64::mul_add`, which rounds once, where JS would round twice (1)</summary>

- fixtures/reject/src/lib.rs:488:4: `f64::mul_add`, which rounds once, where JS would round twice

</details>

<details><summary>a `zip` of `_` and `_`, which std reads by index, of items that drop, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:231:30: a `zip` of `std::iter::Map<std::slice::Iter<'_, u8>, {closure@fixtures/reject/src/lib.rs:231:15: 231:19}>` and `std::ops::Range<i32>`, which std reads by index, of items that drop, which is not supported yet

</details>

<details><summary>a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` (1)</summary>

- fixtures/reject/src/lib.rs:182:51: a `zip` of `std::iter::Chain<std::iter::Map<std::slice::Iter<'_, u8>, {closure@fixtures/reject/src/lib.rs:182:15: 182:18}>, std::iter::Copied<std::slice::Iter<'_, u8>>>`, whose closures std calls one item further through `next` than through `fold`

</details>

<details><summary>a cast from `_` to `_` (1)</summary>

- fixtures/reject/src/lib.rs:87:2: a cast from `u64` to `f32`

</details>

<details><summary>a cast from a pointer to `_` to one to `_`, which reads the bytes of one type as another (1)</summary>

- fixtures/reject/src/lib.rs:616:27: a cast from a pointer to `u32` to one to `u8`, which reads the bytes of one type as another

</details>

<details><summary>a coercion to `_` (1)</summary>

- fixtures/reject/src/lib.rs:344:2: a coercion to `std::sync::Arc<[u8]>`

</details>

<details><summary>a constant of type `_`, a pointer into memory translation can't name (1)</summary>

- fixtures/reject/src/lib.rs:747:3: a constant of type `&u32`, a pointer into memory translation can't name

</details>

<details><summary>a dangling pointer to `_`, a value of no bytes made of more than structs and tuples (1)</summary>

- fixtures/reject/src/lib.rs:701:11: a dangling pointer to `Only`, a value of no bytes made of more than structs and tuples

</details>

<details><summary>a future of `_`, which can be `rt.Pending`, as the future's own `Pending` is (1)</summary>

- fixtures/reject/src/lib.rs:96:34: a future of `std::task::Poll<u8>`, which can be `rt.Pending`, as the future's own `Pending` is

</details>

<details><summary>a mutable borrow of a subslice of `_`, which JS would copy, having no views (1)</summary>

- fixtures/reject/src/lib.rs:236:13: a mutable borrow of a subslice of `u32`, which JS would copy, having no views

</details>

<details><summary>a mutable reference into a `Result`'s payload, passed on or kept (1)</summary>

- fixtures/reject/src/lib.rs:416:12: a mutable reference into a `Result`'s payload, passed on or kept

</details>

<details><summary>a mutable static, which any code may write (1)</summary>

- fixtures/reject/src/lib.rs:80:11: a mutable static, which any code may write

</details>

<details><summary>a pointer's address, which JS has no counterpart of (1)</summary>

- fixtures/reject/src/lib.rs:621:17: a pointer's address, which JS has no counterpart of

</details>

<details><summary>a printable type must implement `Display` or `Debug` (1)</summary>

- fixtures/reject/src/lib.rs:83:1: a printable type must implement `Display` or `Debug`

</details>

<details><summary>a raw pointer to `_` behind a reference, which is the object and not where it is kept (1)</summary>

- fixtures/reject/src/lib.rs:638:10: a raw pointer to `std::vec::Vec<u8>` behind a reference, which is the object and not where it is kept

</details>

<details><summary>a raw pointer to `_` in `_`, which the crate may put another object in place of (1)</summary>

- fixtures/reject/src/lib.rs:669:10: a raw pointer to `(u32, u32)` in `Span2`, which the crate may put another object in place of

</details>

<details><summary>a raw pointer to `_` in a variable assigned again while it may point there (1)</summary>

- fixtures/reject/src/lib.rs:677:10: a raw pointer to `(u32, u32)` in a variable assigned again while it may point there

</details>

<details><summary>a raw pointer to `_` in an element, a payload, or a union field, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:685:10: a raw pointer to `(u32, u32)` in an element, a payload, or a union field, which is not supported yet

</details>

<details><summary>a scalar constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:8:2: a scalar constant of type `f32`

</details>

<details><summary>a trait object of `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:535:26: a trait object of `[u8; 4]`, which is not supported yet

</details>

<details><summary>a zero-sized constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:62:2: a zero-sized constant of type `{async closure@fixtures/reject/src/lib.rs:61:10: 61:18}`

</details>

<details><summary>an upcast from `_` to `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:593:32: an upcast from `dyn Keyed` to `dyn std::any::Any`, which is not supported yet

</details>

<details><summary>arithmetic on `_` (1)</summary>

- fixtures/reject/src/lib.rs:8:2: arithmetic on `f32`

</details>

<details><summary>collecting `_`, whose `None` is `rt.None`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:36:43: collecting `Option<()>`, whose `None` is `rt.None`, which is not supported yet

</details>

<details><summary>comparing `_` as a key, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:271:52: comparing `std::vec::Vec<u8>` as a key, which is not supported yet

</details>

<details><summary>comparing pointers to `()`, which has no bytes and so no address of its own (1)</summary>

- fixtures/reject/src/lib.rs:692:2: comparing pointers to `()`, which has no bytes and so no address of its own

</details>

<details><summary>comparing pointers to `_` in an atomic, which JS has no address of (1)</summary>

- fixtures/reject/src/lib.rs:746:15: comparing pointers to `u32` in an atomic, which JS has no address of

</details>

<details><summary>comparing pointers to `_`, each a place JS makes of its own (1)</summary>

- fixtures/reject/src/lib.rs:657:2: comparing pointers to `u32`, each a place JS makes of its own

</details>

<details><summary>constructing `_` (1)</summary>

- fixtures/reject/src/lib.rs:403:53: constructing `std::borrow::Cow<'_, std::sync::Arc<u8>>`

</details>

<details><summary>hashing `_` as a key, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:271:52: hashing `std::vec::Vec<u8>` as a key, which is not supported yet

</details>

<details><summary>hashing `_` through `_`'s own `Hash::hash_slice`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:363:16: hashing `std::vec::Vec<Batched>` through `Batched`'s own `Hash::hash_slice`, which is not supported yet

</details>

<details><summary>union `_`, whose fields overlap in `_` and `f64`, which JS keeps apart (1)</summary>

- fixtures/reject/src/lib.rs:643:1: union `Bits`, whose fields overlap in `u32` and `f64`, which JS keeps apart

</details>

<details><summary>unsupported type `_` from another crate (1)</summary>

- fixtures/reject/src/lib.rs:492:31: unsupported type `std::num::ParseIntError` from another crate

</details>
