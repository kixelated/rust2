# Census of `reject-fixture`

- Refusals: 85
- Function instances: 74
- Generic functions no instance reached, lowered over their type parameters: 2

## By construct

| Count | Construct | First |
| ---: | --- | --- |
| 10 | unsupported call to `_` | fixtures/reject/src/lib.rs:28:4 |
| 5 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | fixtures/reject/src/lib.rs:61:11 |
| 4 | a variant of `_`, which is not supported: either is the same JS string | fixtures/reject/src/lib.rs:408:2 |
| 4 | an assignment through a reference to `_` | fixtures/reject/src/lib.rs:125:2 |
| 4 | unsupported type `_` | fixtures/reject/src/lib.rs:7:14 |
| 3 | `_` with a mutable reference to a value JS copies, which its mapping would copy | fixtures/reject/src/lib.rs:17:2 |
| 3 | dropping `_`, which is not supported yet | fixtures/reject/src/lib.rs:61:18 |
| 3 | replacing the value behind a reference to `_` | fixtures/reject/src/lib.rs:109:2 |
| 2 | `_` is not supported: it would be pending two ways with one value | fixtures/reject/src/lib.rs:129:20 |
| 2 | `_` is not supported: its `None` and `Some` would both be `rt.None` | fixtures/reject/src/lib.rs:23:15 |
| 2 | `_` of `_`, whose `None` is `rt.None`, which is not supported yet | fixtures/reject/src/lib.rs:48:42 |
| 2 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | fixtures/reject/src/lib.rs:276:16 |
| 2 | a field of `_` | fixtures/reject/src/lib.rs:292:7 |
| 2 | an iterator of the crate implementing `_`, which is not supported yet | fixtures/reject/src/lib.rs:183:1 |
| 2 | constructing `_` from another crate | fixtures/reject/src/lib.rs:169:23 |
| 2 | overriding `_`, which std's adaptors call in place of `next` or beside it | fixtures/reject/src/lib.rs:163:2 |
| 1 | `Binary` formatting, as in `{:b}`, is not supported | fixtures/reject/src/lib.rs:86:3 |
| 1 | `LowerExp` formatting, as in `{:e}`, is not supported | fixtures/reject/src/lib.rs:88:3 |
| 1 | `Octal` formatting, as in `{:o}`, is not supported | fixtures/reject/src/lib.rs:87:3 |
| 1 | `Pointer` formatting, as in `{:p}`, is not supported | fixtures/reject/src/lib.rs:91:3 |
| 1 | `_` is not supported yet | fixtures/reject/src/lib.rs:419:18 |
| 1 | `_` is not supported: its field `_` is `_`, typed `_`, whose `None` would be `rt.None` as `_` | fixtures/reject/src/lib.rs:38:2 |
| 1 | `_` of `_` by `_`, which is not supported yet | fixtures/reject/src/lib.rs:320:6 |
| 1 | `_` of `_`, which drops `_`, which is not supported yet | fixtures/reject/src/lib.rs:67:13 |
| 1 | `_` through a held type's own, which is not supported yet | fixtures/reject/src/lib.rs:352:2 |
| 1 | `_`, which is not supported yet | fixtures/reject/src/lib.rs:328:9 |
| 1 | `fold` of `_`, which folds a `flatten` whose source and iterators both drop, which is not supported yet | fixtures/reject/src/lib.rs:245:26 |
| 1 | a `_` cast from `_` to `_` | fixtures/reject/src/lib.rs:120:2 |
| 1 | a `zip` of `_` and `_`, which std reads by index, of items that drop, which is not supported yet | fixtures/reject/src/lib.rs:250:30 |
| 1 | a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` | fixtures/reject/src/lib.rs:196:51 |
| 1 | a coercion to `_` | fixtures/reject/src/lib.rs:375:2 |
| 1 | a constant of type `_` | fixtures/reject/src/lib.rs:104:2 |
| 1 | a copy of a closure that changes what it captured | fixtures/reject/src/lib.rs:286:14 |
| 1 | a mutable borrow of a subslice of `_`, which JS would copy, having no views | fixtures/reject/src/lib.rs:255:13 |
| 1 | a mutable reference into a `Result`'s payload, passed on or kept | fixtures/reject/src/lib.rs:457:12 |
| 1 | a precision, as in `{:.2}`, is not supported | fixtures/reject/src/lib.rs:83:3 |
| 1 | a printable type must implement `Display` or `Debug` | fixtures/reject/src/lib.rs:116:1 |
| 1 | a scalar constant of type `_` | fixtures/reject/src/lib.rs:8:2 |
| 1 | a width or precision from an argument, as in `{:1$}`, is not supported | fixtures/reject/src/lib.rs:90:3 |
| 1 | a zero-sized constant of type `_` | fixtures/reject/src/lib.rs:76:2 |
| 1 | an iterator that owns `_`, which drops, which is not supported yet | fixtures/reject/src/lib.rs:402:13 |
| 1 | arithmetic on `_` | fixtures/reject/src/lib.rs:8:2 |
| 1 | collecting `_`, whose `None` is `rt.None`, which is not supported yet | fixtures/reject/src/lib.rs:43:43 |
| 1 | comparing `_` as a key, which is not supported yet | fixtures/reject/src/lib.rs:302:52 |
| 1 | constructing `_` | fixtures/reject/src/lib.rs:426:53 |
| 1 | hashing `_` as a key, which is not supported yet | fixtures/reject/src/lib.rs:302:52 |
| 1 | hex `Debug`, as in `{:x?}`, is not supported | fixtures/reject/src/lib.rs:89:3 |
| 1 | the `+` flag of a format spec is not supported | fixtures/reject/src/lib.rs:84:3 |
| 1 | the `-` flag of a format spec is not supported | fixtures/reject/src/lib.rs:85:3 |

## By API

The type or module each refused type or path belongs to.

| Count | API | First |
| ---: | --- | --- |
| 7 | `std::borrow::Cow` | fixtures/reject/src/lib.rs:408:2 |
| 6 | `Option` | fixtures/reject/src/lib.rs:23:15 |
| 6 | `f64` | fixtures/reject/src/lib.rs:7:14 |
| 5 | `Iterator` | fixtures/reject/src/lib.rs:48:42 |
| 5 | `{closure}` | fixtures/reject/src/lib.rs:61:11 |
| 3 | `std::iter::Map` | fixtures/reject/src/lib.rs:61:18 |
| 3 | `std::slice` | fixtures/reject/src/lib.rs:28:4 |
| 3 | `std::sync::Arc` | fixtures/reject/src/lib.rs:125:2 |
| 2 | `HashMap` | fixtures/reject/src/lib.rs:297:6 |
| 2 | `Meters` | fixtures/reject/src/lib.rs:109:2 |
| 2 | `Noisy` | fixtures/reject/src/lib.rs:67:13 |
| 2 | `Poll` | fixtures/reject/src/lib.rs:129:20 |
| 2 | `std::collections::HashMap` | fixtures/reject/src/lib.rs:67:13 |
| 2 | `std::iter::Iterator` | fixtures/reject/src/lib.rs:276:16 |
| 2 | `std::iter::Take` | fixtures/reject/src/lib.rs:400:79 |
| 2 | `std::ops::Range` | fixtures/reject/src/lib.rs:204:8 |
| 2 | `std::pin::Pin` | fixtures/reject/src/lib.rs:140:2 |
| 2 | `std::vec::Vec` | fixtures/reject/src/lib.rs:302:52 |
| 2 | `u8` | fixtures/reject/src/lib.rs:104:2 |
| 1 | `DoubleEndedIterator` | fixtures/reject/src/lib.rs:183:1 |
| 1 | `Entry` | fixtures/reject/src/lib.rs:391:13 |
| 1 | `FusedIterator` | fixtures/reject/src/lib.rs:237:1 |
| 1 | `Hasher` | fixtures/reject/src/lib.rs:328:9 |
| 1 | `IntToFloat` | fixtures/reject/src/lib.rs:120:2 |
| 1 | `Maybe` | fixtures/reject/src/lib.rs:38:2 |
| 1 | `T \| undefined` | fixtures/reject/src/lib.rs:38:2 |
| 1 | `[u16; 1]` | fixtures/reject/src/lib.rs:269:22 |
| 1 | `dyn std::ops::Fn(u8) -> u8` | fixtures/reject/src/lib.rs:11:16 |
| 1 | `from_fn` | fixtures/reject/src/lib.rs:401:13 |
| 1 | `into_keys` | fixtures/reject/src/lib.rs:67:13 |
| 1 | `std::collections::hash_map::Entry` | fixtures/reject/src/lib.rs:447:15 |
| 1 | `std::iter::Chain` | fixtures/reject/src/lib.rs:196:51 |
| 1 | `std::iter::Flatten` | fixtures/reject/src/lib.rs:245:26 |
| 1 | `std::iter::IntoIterator` | fixtures/reject/src/lib.rs:17:2 |
| 1 | `std::iter::Rev` | fixtures/reject/src/lib.rs:190:14 |
| 1 | `std::mem` | fixtures/reject/src/lib.rs:60:29 |
| 1 | `std::ops::RangeFrom` | fixtures/reject/src/lib.rs:292:48 |
| 1 | `std::ops::RangeTo` | fixtures/reject/src/lib.rs:292:7 |
| 1 | `std::option::Option` | fixtures/reject/src/lib.rs:464:2 |
| 1 | `std::slice::Iter` | fixtures/reject/src/lib.rs:150:11 |
| 1 | `std::str` | fixtures/reject/src/lib.rs:381:4 |
| 1 | `std::string::String` | fixtures/reject/src/lib.rs:98:10 |
| 1 | `std::sync::MutexGuard` | fixtures/reject/src/lib.rs:145:2 |
| 1 | `std::time::Duration` | fixtures/reject/src/lib.rs:434:2 |
| 1 | `successors` | fixtures/reject/src/lib.rs:400:15 |
| 1 | `u32` | fixtures/reject/src/lib.rs:255:13 |
| 1 | `u64` | fixtures/reject/src/lib.rs:120:2 |
| 1 | `value` | fixtures/reject/src/lib.rs:38:2 |
| 1 | `{async closure}` | fixtures/reject/src/lib.rs:76:2 |

## By type or path

| Count | Construct | Type or path | First |
| ---: | --- | --- | --- |
| 4 | a variant of `_`, which is not supported: either is the same JS string | `std::borrow::Cow` | fixtures/reject/src/lib.rs:408:2 |
| 3 | unsupported type `_` | `f64` | fixtures/reject/src/lib.rs:7:14 |
| 2 | `_` is not supported: it would be pending two ways with one value | `Poll` | fixtures/reject/src/lib.rs:129:20 |
| 2 | `_` is not supported: its `None` and `Some` would both be `rt.None` | `Option` | fixtures/reject/src/lib.rs:23:15 |
| 2 | an assignment through a reference to `_` | `std::pin::Pin` | fixtures/reject/src/lib.rs:140:2 |
| 2 | dropping `_`, which is not supported yet | `std::iter::Take` | fixtures/reject/src/lib.rs:400:79 |
| 2 | replacing the value behind a reference to `_` | `Meters` | fixtures/reject/src/lib.rs:109:2 |
| 1 | `_` is not supported yet | `std::borrow::Cow` | fixtures/reject/src/lib.rs:419:18 |
| 1 | `_` is not supported: its field `_` is `_`, typed `_`, whose `None` would be `rt.None` as `_` | `Maybe`, `value`, `Option`, `T \| undefined`, `Option` | fixtures/reject/src/lib.rs:38:2 |
| 1 | `_` of `_` by `_`, which is not supported yet | `HashMap::contains_key`, `std::collections::HashMap`, `u8` | fixtures/reject/src/lib.rs:320:6 |
| 1 | `_` of `_`, which drops `_`, which is not supported yet | `into_keys`, `std::collections::HashMap`, `Noisy` | fixtures/reject/src/lib.rs:67:13 |
| 1 | `_` of `_`, whose `None` is `rt.None`, which is not supported yet | `HashMap::get`, `Option` | fixtures/reject/src/lib.rs:297:6 |
| 1 | `_` of `_`, whose `None` is `rt.None`, which is not supported yet | `Iterator::find`, `Option` | fixtures/reject/src/lib.rs:48:42 |
| 1 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | `std::iter::Iterator::count` | fixtures/reject/src/lib.rs:276:38 |
| 1 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | `std::iter::Iterator::filter` | fixtures/reject/src/lib.rs:276:16 |
| 1 | `_` through a held type's own, which is not supported yet | `<std::sync::Arc as PartialEq>::ne` | fixtures/reject/src/lib.rs:352:2 |
| 1 | `_` with a mutable reference to a value JS copies, which its mapping would copy | `std::collections::hash_map::Entry::and_modify` | fixtures/reject/src/lib.rs:447:15 |
| 1 | `_` with a mutable reference to a value JS copies, which its mapping would copy | `std::iter::IntoIterator::into_iter` | fixtures/reject/src/lib.rs:17:2 |
| 1 | `_` with a mutable reference to a value JS copies, which its mapping would copy | `std::slice::last_mut` | fixtures/reject/src/lib.rs:440:21 |
| 1 | `_`, which is not supported yet | `Hasher::write` | fixtures/reject/src/lib.rs:328:9 |
| 1 | `fold` of `_`, which folds a `flatten` whose source and iterators both drop, which is not supported yet | `std::iter::Flatten` | fixtures/reject/src/lib.rs:245:26 |
| 1 | a `_` cast from `_` to `_` | `IntToFloat`, `u64`, `f64` | fixtures/reject/src/lib.rs:120:2 |
| 1 | a `zip` of `_` and `_`, which std reads by index, of items that drop, which is not supported yet | `std::iter::Map`, `std::ops::Range` | fixtures/reject/src/lib.rs:250:30 |
| 1 | a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` | `std::iter::Chain` | fixtures/reject/src/lib.rs:196:51 |
| 1 | a coercion to `_` | `std::sync::Arc` | fixtures/reject/src/lib.rs:375:2 |
| 1 | a constant of type `_` | `&u8` | fixtures/reject/src/lib.rs:104:2 |
| 1 | a field of `_` | `std::ops::RangeFrom` | fixtures/reject/src/lib.rs:292:48 |
| 1 | a field of `_` | `std::ops::RangeTo` | fixtures/reject/src/lib.rs:292:7 |
| 1 | a mutable borrow of a subslice of `_`, which JS would copy, having no views | `u32` | fixtures/reject/src/lib.rs:255:13 |
| 1 | a scalar constant of type `_` | `f64` | fixtures/reject/src/lib.rs:8:2 |
| 1 | a zero-sized constant of type `_` | `{async closure}` | fixtures/reject/src/lib.rs:76:2 |
| 1 | an assignment through a reference to `_` | `std::sync::Arc` | fixtures/reject/src/lib.rs:125:2 |
| 1 | an assignment through a reference to `_` | `std::sync::MutexGuard` | fixtures/reject/src/lib.rs:145:2 |
| 1 | an iterator of the crate implementing `_`, which is not supported yet | `DoubleEndedIterator` | fixtures/reject/src/lib.rs:183:1 |
| 1 | an iterator of the crate implementing `_`, which is not supported yet | `FusedIterator` | fixtures/reject/src/lib.rs:237:1 |
| 1 | an iterator that owns `_`, which drops, which is not supported yet | `Noisy` | fixtures/reject/src/lib.rs:402:13 |
| 1 | arithmetic on `_` | `f64` | fixtures/reject/src/lib.rs:8:2 |
| 1 | collecting `_`, whose `None` is `rt.None`, which is not supported yet | `Option` | fixtures/reject/src/lib.rs:43:43 |
| 1 | comparing `_` as a key, which is not supported yet | `std::vec::Vec` | fixtures/reject/src/lib.rs:302:52 |
| 1 | constructing `_` | `std::borrow::Cow` | fixtures/reject/src/lib.rs:426:53 |
| 1 | constructing `_` from another crate | `std::iter::Map` | fixtures/reject/src/lib.rs:169:23 |
| 1 | constructing `_` from another crate | `std::iter::Rev` | fixtures/reject/src/lib.rs:190:14 |
| 1 | dropping `_`, which is not supported yet | `std::iter::Map` | fixtures/reject/src/lib.rs:61:18 |
| 1 | hashing `_` as a key, which is not supported yet | `std::vec::Vec` | fixtures/reject/src/lib.rs:302:52 |
| 1 | overriding `_`, which std's adaptors call in place of `next` or beside it | `Iterator::fold` | fixtures/reject/src/lib.rs:163:2 |
| 1 | overriding `_`, which std's adaptors call in place of `next` or beside it | `Iterator::size_hint` | fixtures/reject/src/lib.rs:217:2 |
| 1 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | `{closure}`, `Entry::or_insert_with` | fixtures/reject/src/lib.rs:391:13 |
| 1 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | `{closure}`, `Iterator::any` | fixtures/reject/src/lib.rs:392:11 |
| 1 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | `{closure}`, `Iterator::map` | fixtures/reject/src/lib.rs:61:11 |
| 1 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | `{closure}`, `from_fn::from_fn` | fixtures/reject/src/lib.rs:401:13 |
| 1 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | `{closure}`, `successors::successors` | fixtures/reject/src/lib.rs:400:15 |
| 1 | replacing the value behind a reference to `_` | `[u16; 1]` | fixtures/reject/src/lib.rs:269:22 |
| 1 | unsupported call to `_` | `<std::borrow::Cow as std::ops::Deref>::deref` | fixtures/reject/src/lib.rs:420:2 |
| 1 | unsupported call to `_` | `<std::option::Option as std::cmp::PartialEq>::eq` | fixtures/reject/src/lib.rs:464:2 |
| 1 | unsupported call to `_` | `<std::slice::Iter as std::iter::Iterator>::cycle` | fixtures/reject/src/lib.rs:150:11 |
| 1 | unsupported call to `_` | `<std::string::String as std::fmt::Write>::write_fmt` | fixtures/reject/src/lib.rs:98:10 |
| 1 | unsupported call to `_` | `<std::time::Duration as std::ops::AddAssign>::add_assign` | fixtures/reject/src/lib.rs:434:2 |
| 1 | unsupported call to `_` | `std::mem::size_of_val` | fixtures/reject/src/lib.rs:60:29 |
| 1 | unsupported call to `_` | `std::ops::Range::contains` | fixtures/reject/src/lib.rs:204:8 |
| 1 | unsupported call to `_` | `std::slice::copy_within` | fixtures/reject/src/lib.rs:370:4 |
| 1 | unsupported call to `_` | `std::slice::sort` | fixtures/reject/src/lib.rs:28:4 |
| 1 | unsupported call to `_` | `std::str::split` | fixtures/reject/src/lib.rs:381:4 |
| 1 | unsupported type `_` | `dyn std::ops::Fn(u8) -> u8` | fixtures/reject/src/lib.rs:11:16 |

## Spans

<details><summary>unsupported call to `_` (10)</summary>

- fixtures/reject/src/lib.rs:28:4: unsupported call to `std::slice::<impl [u8]>::sort`
- fixtures/reject/src/lib.rs:60:29: unsupported call to `std::mem::size_of_val::<Noisy>`
- fixtures/reject/src/lib.rs:98:10: unsupported call to `<std::string::String as std::fmt::Write>::write_fmt`
- fixtures/reject/src/lib.rs:150:11: unsupported call to `<std::slice::Iter<'_, u8> as std::iter::Iterator>::cycle`
- fixtures/reject/src/lib.rs:204:8: unsupported call to `std::ops::Range::<Key>::contains::<Key>`
- fixtures/reject/src/lib.rs:370:4: unsupported call to `core::slice::<impl [(u8, u8)]>::copy_within::<std::ops::Range<usize>>`
- fixtures/reject/src/lib.rs:381:4: unsupported call to `core::str::<impl str>::split::<[char; 2]>`
- fixtures/reject/src/lib.rs:420:2: unsupported call to `<std::borrow::Cow<'_, [u8]> as std::ops::Deref>::deref`
- fixtures/reject/src/lib.rs:434:2: unsupported call to `<std::time::Duration as std::ops::AddAssign>::add_assign`
- fixtures/reject/src/lib.rs:464:2: unsupported call to `<std::option::Option<&mut u32> as std::cmp::PartialEq>::eq`

</details>

<details><summary>passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet (5)</summary>

- fixtures/reject/src/lib.rs:61:11: passing `{closure@fixtures/reject/src/lib.rs:60:10: 60:23}`, a closure that owns a value that drops, to `Iterator::map`, which is not supported yet
- fixtures/reject/src/lib.rs:391:13: passing `{closure@fixtures/reject/src/lib.rs:391:28: 391:35}`, a closure that owns a value that drops, to `Entry::or_insert_with`, which is not supported yet
- fixtures/reject/src/lib.rs:392:11: passing `{closure@fixtures/reject/src/lib.rs:392:15: 392:24}`, a closure that owns a value that drops, to `Iterator::any`, which is not supported yet
- fixtures/reject/src/lib.rs:400:15: passing `{closure@fixtures/reject/src/lib.rs:400:48: 400:56}`, a closure that owns a value that drops, to `successors::successors`, which is not supported yet
- fixtures/reject/src/lib.rs:401:13: passing `{closure@fixtures/reject/src/lib.rs:401:32: 401:39}`, a closure that owns a value that drops, to `from_fn::from_fn`, which is not supported yet

</details>

<details><summary>a variant of `_`, which is not supported: either is the same JS string (4)</summary>

- fixtures/reject/src/lib.rs:408:2: a variant of `std::borrow::Cow<'_, str>`, which is not supported: either is the same JS string
- fixtures/reject/src/lib.rs:409:30: a variant of `std::borrow::Cow<'_, str>`, which is not supported: either is the same JS string
- fixtures/reject/src/lib.rs:410:27: a variant of `std::borrow::Cow<'_, str>`, which is not supported: either is the same JS string
- fixtures/reject/src/lib.rs:415:2: a variant of `std::borrow::Cow<'_, str>`, which is not supported: either is the same JS string

</details>

<details><summary>an assignment through a reference to `_` (4)</summary>

- fixtures/reject/src/lib.rs:125:2: an assignment through a reference to `std::sync::Arc<u8>`
- fixtures/reject/src/lib.rs:140:2: an assignment through a reference to `std::pin::Pin<std::sync::Arc<u8>>`
- fixtures/reject/src/lib.rs:145:2: an assignment through a reference to `std::sync::MutexGuard<'_, u8>`
- fixtures/reject/src/lib.rs:263:2: an assignment through a reference to `std::pin::Pin<&mut Noisy>`

</details>

<details><summary>unsupported type `_` (4)</summary>

- fixtures/reject/src/lib.rs:7:14: unsupported type `f64`
- fixtures/reject/src/lib.rs:7:25: unsupported type `f64`
- fixtures/reject/src/lib.rs:11:16: unsupported type `dyn std::ops::Fn(u8) -> u8`
- fixtures/reject/src/lib.rs:118:51: unsupported type `f64`

</details>

<details><summary>`_` with a mutable reference to a value JS copies, which its mapping would copy (3)</summary>

- fixtures/reject/src/lib.rs:17:2: `std::iter::IntoIterator::into_iter` with a mutable reference to a value JS copies, which its mapping would copy
- fixtures/reject/src/lib.rs:440:21: `core::slice::<impl [T]>::last_mut` with a mutable reference to a value JS copies, which its mapping would copy
- fixtures/reject/src/lib.rs:447:15: `std::collections::hash_map::Entry::<'a, K, V, A>::and_modify` with a mutable reference to a value JS copies, which its mapping would copy

</details>

<details><summary>dropping `_`, which is not supported yet (3)</summary>

- fixtures/reject/src/lib.rs:61:18: dropping `std::iter::Map<std::slice::Iter<'_, u8>, {closure@fixtures/reject/src/lib.rs:60:10: 60:23}>`, which is not supported yet
- fixtures/reject/src/lib.rs:400:79: dropping `std::iter::Take<std::iter::Successors<u8, {closure@fixtures/reject/src/lib.rs:400:48: 400:56}>>`, which is not supported yet
- fixtures/reject/src/lib.rs:401:62: dropping `std::iter::Take<std::iter::FromFn<{closure@fixtures/reject/src/lib.rs:401:32: 401:39}>>`, which is not supported yet

</details>

<details><summary>replacing the value behind a reference to `_` (3)</summary>

- fixtures/reject/src/lib.rs:109:2: replacing the value behind a reference to `Meters`
- fixtures/reject/src/lib.rs:113:2: replacing the value behind a reference to `Meters`
- fixtures/reject/src/lib.rs:269:22: replacing the value behind a reference to `[u16; 1]`

</details>

<details><summary>`_` is not supported: it would be pending two ways with one value (2)</summary>

- fixtures/reject/src/lib.rs:129:20: `Poll<std::option::Option<std::task::Poll<u8>>>` is not supported: it would be pending two ways with one value
- fixtures/reject/src/lib.rs:134:54: `Poll<std::task::Poll<u8>>` is not supported: it would be pending two ways with one value

</details>

<details><summary>`_` is not supported: its `None` and `Some` would both be `rt.None` (2)</summary>

- fixtures/reject/src/lib.rs:23:15: `Option<std::option::Option<std::option::Option<u8>>>` is not supported: its `None` and `Some` would both be `rt.None`
- fixtures/reject/src/lib.rs:24:2: `Option<std::option::Option<std::option::Option<u8>>>` is not supported: its `None` and `Some` would both be `rt.None`

</details>

<details><summary>`_` of `_`, whose `None` is `rt.None`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:48:42: `Iterator::find` of `Option<std::option::Option<u8>>`, whose `None` is `rt.None`, which is not supported yet
- fixtures/reject/src/lib.rs:297:6: `HashMap::get` of `Option<&std::option::Option<u8>>`, whose `None` is `rt.None`, which is not supported yet

</details>

<details><summary>`_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it (2)</summary>

- fixtures/reject/src/lib.rs:276:16: `std::iter::Iterator::filter` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it
- fixtures/reject/src/lib.rs:276:38: `std::iter::Iterator::count` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it

</details>

<details><summary>a field of `_` (2)</summary>

- fixtures/reject/src/lib.rs:292:7: a field of `std::ops::RangeTo<usize>`
- fixtures/reject/src/lib.rs:292:48: a field of `std::ops::RangeFrom<usize>`

</details>

<details><summary>an iterator of the crate implementing `_`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:183:1: an iterator of the crate implementing `DoubleEndedIterator`, which is not supported yet
- fixtures/reject/src/lib.rs:237:1: an iterator of the crate implementing `FusedIterator`, which is not supported yet

</details>

<details><summary>constructing `_` from another crate (2)</summary>

- fixtures/reject/src/lib.rs:169:23: constructing `std::iter::Map<Folds, {closure@fixtures/reject/src/lib.rs:169:12: 169:15}>` from another crate
- fixtures/reject/src/lib.rs:190:14: constructing `std::iter::Rev<Backs>` from another crate

</details>

<details><summary>overriding `_`, which std's adaptors call in place of `next` or beside it (2)</summary>

- fixtures/reject/src/lib.rs:163:2: overriding `Iterator::fold`, which std's adaptors call in place of `next` or beside it
- fixtures/reject/src/lib.rs:217:2: overriding `Iterator::size_hint`, which std's adaptors call in place of `next` or beside it

</details>

<details><summary>`Binary` formatting, as in `{:b}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:86:3: `Binary` formatting, as in `{:b}`, is not supported

</details>

<details><summary>`LowerExp` formatting, as in `{:e}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:88:3: `LowerExp` formatting, as in `{:e}`, is not supported

</details>

<details><summary>`Octal` formatting, as in `{:o}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:87:3: `Octal` formatting, as in `{:o}`, is not supported

</details>

<details><summary>`Pointer` formatting, as in `{:p}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:91:3: `Pointer` formatting, as in `{:p}`, is not supported

</details>

<details><summary>`_` is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:419:18: `std::borrow::Cow<'_, [u8]>` is not supported yet

</details>

<details><summary>`_` is not supported: its field `_` is `_`, typed `_`, whose `None` would be `rt.None` as `_` (1)</summary>

- fixtures/reject/src/lib.rs:38:2: `Maybe<()>` is not supported: its field `value` is `Option<T>`, typed `T | undefined`, whose `None` would be `rt.None` as `Option<()>`

</details>

<details><summary>`_` of `_` by `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:320:6: `HashMap::contains_key` of `std::collections::HashMap<Id, u8>` by `u8`, which is not supported yet

</details>

<details><summary>`_` of `_`, which drops `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:67:13: `into_keys` of `std::collections::HashMap<u8, Noisy>`, which drops `Noisy`, which is not supported yet

</details>

<details><summary>`_` through a held type's own, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:352:2: `<std::sync::Arc<Picky> as PartialEq>::ne` through a held type's own, which is not supported yet

</details>

<details><summary>`_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:328:9: `Hasher::write`, which is not supported yet

</details>

<details><summary>`fold` of `_`, which folds a `flatten` whose source and iterators both drop, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:245:26: `fold` of `std::iter::Flatten<std::vec::IntoIter<std::vec::Vec<Noisy>>>`, which folds a `flatten` whose source and iterators both drop, which is not supported yet

</details>

<details><summary>a `_` cast from `_` to `_` (1)</summary>

- fixtures/reject/src/lib.rs:120:2: a `IntToFloat` cast from `u64` to `f64`

</details>

<details><summary>a `zip` of `_` and `_`, which std reads by index, of items that drop, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:250:30: a `zip` of `std::iter::Map<std::slice::Iter<'_, u8>, {closure@fixtures/reject/src/lib.rs:250:15: 250:19}>` and `std::ops::Range<i32>`, which std reads by index, of items that drop, which is not supported yet

</details>

<details><summary>a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` (1)</summary>

- fixtures/reject/src/lib.rs:196:51: a `zip` of `std::iter::Chain<std::iter::Map<std::slice::Iter<'_, u8>, {closure@fixtures/reject/src/lib.rs:196:15: 196:18}>, std::iter::Copied<std::slice::Iter<'_, u8>>>`, whose closures std calls one item further through `next` than through `fold`

</details>

<details><summary>a coercion to `_` (1)</summary>

- fixtures/reject/src/lib.rs:375:2: a coercion to `std::sync::Arc<[u8]>`

</details>

<details><summary>a constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:104:2: a constant of type `&u8`

</details>

<details><summary>a copy of a closure that changes what it captured (1)</summary>

- fixtures/reject/src/lib.rs:286:14: a copy of a closure that changes what it captured

</details>

<details><summary>a mutable borrow of a subslice of `_`, which JS would copy, having no views (1)</summary>

- fixtures/reject/src/lib.rs:255:13: a mutable borrow of a subslice of `u32`, which JS would copy, having no views

</details>

<details><summary>a mutable reference into a `Result`'s payload, passed on or kept (1)</summary>

- fixtures/reject/src/lib.rs:457:12: a mutable reference into a `Result`'s payload, passed on or kept

</details>

<details><summary>a precision, as in `{:.2}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:83:3: a precision, as in `{:.2}`, is not supported

</details>

<details><summary>a printable type must implement `Display` or `Debug` (1)</summary>

- fixtures/reject/src/lib.rs:116:1: a printable type must implement `Display` or `Debug`

</details>

<details><summary>a scalar constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:8:2: a scalar constant of type `f64`

</details>

<details><summary>a width or precision from an argument, as in `{:1$}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:90:3: a width or precision from an argument, as in `{:1$}`, is not supported

</details>

<details><summary>a zero-sized constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:76:2: a zero-sized constant of type `{async closure@fixtures/reject/src/lib.rs:75:10: 75:18}`

</details>

<details><summary>an iterator that owns `_`, which drops, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:402:13: an iterator that owns `Noisy`, which drops, which is not supported yet

</details>

<details><summary>arithmetic on `_` (1)</summary>

- fixtures/reject/src/lib.rs:8:2: arithmetic on `f64`

</details>

<details><summary>collecting `_`, whose `None` is `rt.None`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:43:43: collecting `Option<()>`, whose `None` is `rt.None`, which is not supported yet

</details>

<details><summary>comparing `_` as a key, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:302:52: comparing `std::vec::Vec<u8>` as a key, which is not supported yet

</details>

<details><summary>constructing `_` (1)</summary>

- fixtures/reject/src/lib.rs:426:53: constructing `std::borrow::Cow<'_, std::sync::Arc<u8>>`

</details>

<details><summary>hashing `_` as a key, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:302:52: hashing `std::vec::Vec<u8>` as a key, which is not supported yet

</details>

<details><summary>hex `Debug`, as in `{:x?}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:89:3: hex `Debug`, as in `{:x?}`, is not supported

</details>

<details><summary>the `+` flag of a format spec is not supported (1)</summary>

- fixtures/reject/src/lib.rs:84:3: the `+` flag of a format spec is not supported

</details>

<details><summary>the `-` flag of a format spec is not supported (1)</summary>

- fixtures/reject/src/lib.rs:85:3: the `-` flag of a format spec is not supported

</details>
