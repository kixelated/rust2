# Census of `reject-fixture`

- Refusals: 79
- Function instances: 64
- Generic functions no instance reached, lowered over their type parameters: 2

## By construct

| Count | Construct | First |
| ---: | --- | --- |
| 7 | an assignment through a reference to `_` | fixtures/reject/src/lib.rs:16:2 |
| 7 | unsupported call to `_` | fixtures/reject/src/lib.rs:32:4 |
| 6 | `_` is not supported: its `None` and `Some` would both be `undefined` | fixtures/reject/src/lib.rs:27:15 |
| 6 | `_`: a JS value of `_` is a copy, so a reference can't change it | fixtures/reject/src/lib.rs:15:13 |
| 4 | replacing the value behind a reference to `_` | fixtures/reject/src/lib.rs:107:2 |
| 4 | unsupported type `_` | fixtures/reject/src/lib.rs:6:14 |
| 3 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | fixtures/reject/src/lib.rs:49:11 |
| 2 | `_` is not supported: it would be pending two ways with one value | fixtures/reject/src/lib.rs:127:20 |
| 2 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | fixtures/reject/src/lib.rs:279:16 |
| 2 | a field of `_` | fixtures/reject/src/lib.rs:295:7 |
| 2 | an iterator of the crate implementing `_`, which is not supported yet | fixtures/reject/src/lib.rs:186:1 |
| 2 | constructing `_` from another crate | fixtures/reject/src/lib.rs:172:23 |
| 2 | overriding `_`, which std's adaptors call in place of `next` or beside it | fixtures/reject/src/lib.rs:166:2 |
| 1 | `Binary` formatting, as in `{:b}`, is not supported | fixtures/reject/src/lib.rs:84:3 |
| 1 | `LowerExp` formatting, as in `{:e}`, is not supported | fixtures/reject/src/lib.rs:86:3 |
| 1 | `Octal` formatting, as in `{:o}`, is not supported | fixtures/reject/src/lib.rs:85:3 |
| 1 | `Pointer` formatting, as in `{:p}`, is not supported | fixtures/reject/src/lib.rs:89:3 |
| 1 | `_` holds `_`, which is `undefined` as `None` is, which is not supported | fixtures/reject/src/lib.rs:300:6 |
| 1 | `_` of `_` by `_`, which is not supported yet | fixtures/reject/src/lib.rs:323:6 |
| 1 | `_` of `_`, which drops `_`, which is not supported yet | fixtures/reject/src/lib.rs:55:13 |
| 1 | `_` through a held type's own, which is not supported yet | fixtures/reject/src/lib.rs:355:2 |
| 1 | `_`, a mutable reference to a value a map holds, kept other than to read and write it | fixtures/reject/src/lib.rs:365:6 |
| 1 | `_`, which is not supported yet | fixtures/reject/src/lib.rs:331:9 |
| 1 | `fold` of `_`, which folds a `flatten` whose source and iterators both drop, which is not supported yet | fixtures/reject/src/lib.rs:248:26 |
| 1 | a `_` cast from `_` to `_` | fixtures/reject/src/lib.rs:118:2 |
| 1 | a `zip` of `_` and `_`, which std reads by index, of items that drop, which is not supported yet | fixtures/reject/src/lib.rs:253:30 |
| 1 | a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` | fixtures/reject/src/lib.rs:199:51 |
| 1 | a coercion to `_` | fixtures/reject/src/lib.rs:378:2 |
| 1 | a constant of type `_` | fixtures/reject/src/lib.rs:102:2 |
| 1 | a copy of a closure that changes what it captured | fixtures/reject/src/lib.rs:289:14 |
| 1 | a mutable borrow of a subslice of `_`, which JS would copy, having no views | fixtures/reject/src/lib.rs:258:13 |
| 1 | a precision, as in `{:.2}`, is not supported | fixtures/reject/src/lib.rs:81:3 |
| 1 | a printable type must implement `Display` or `Debug` | fixtures/reject/src/lib.rs:114:1 |
| 1 | a scalar constant of type `_` | fixtures/reject/src/lib.rs:7:2 |
| 1 | a width or precision from an argument, as in `{:1$}`, is not supported | fixtures/reject/src/lib.rs:88:3 |
| 1 | a zero-sized constant of type `_` | fixtures/reject/src/lib.rs:74:2 |
| 1 | arithmetic on `_` | fixtures/reject/src/lib.rs:7:2 |
| 1 | comparing `_` as a key, which is not supported yet | fixtures/reject/src/lib.rs:305:52 |
| 1 | dropping `_`, which is not supported yet | fixtures/reject/src/lib.rs:49:18 |
| 1 | hashing `_` as a key, which is not supported yet | fixtures/reject/src/lib.rs:305:52 |
| 1 | hex `Debug`, as in `{:x?}`, is not supported | fixtures/reject/src/lib.rs:87:3 |
| 1 | the `+` flag of a format spec is not supported | fixtures/reject/src/lib.rs:82:3 |
| 1 | the `-` flag of a format spec is not supported | fixtures/reject/src/lib.rs:83:3 |

## By API

The type or module each refused type or path belongs to.

| Count | API | First |
| ---: | --- | --- |
| 6 | `Option` | fixtures/reject/src/lib.rs:27:15 |
| 6 | `f64` | fixtures/reject/src/lib.rs:6:14 |
| 6 | `u8` | fixtures/reject/src/lib.rs:64:10 |
| 4 | `Iterator` | fixtures/reject/src/lib.rs:49:11 |
| 4 | `std::string::String` | fixtures/reject/src/lib.rs:94:16 |
| 4 | `u32` | fixtures/reject/src/lib.rs:15:13 |
| 3 | `std::collections::HashMap` | fixtures/reject/src/lib.rs:55:13 |
| 3 | `std::iter::Map` | fixtures/reject/src/lib.rs:49:18 |
| 3 | `std::sync::Arc` | fixtures/reject/src/lib.rs:123:2 |
| 3 | `{closure}` | fixtures/reject/src/lib.rs:49:11 |
| 2 | `Meters` | fixtures/reject/src/lib.rs:107:2 |
| 2 | `Poll` | fixtures/reject/src/lib.rs:127:20 |
| 2 | `std::iter::Iterator` | fixtures/reject/src/lib.rs:279:16 |
| 2 | `std::ops::Range` | fixtures/reject/src/lib.rs:207:8 |
| 2 | `std::option::Option` | fixtures/reject/src/lib.rs:300:6 |
| 2 | `std::pin::Pin` | fixtures/reject/src/lib.rs:143:2 |
| 2 | `std::slice` | fixtures/reject/src/lib.rs:32:4 |
| 2 | `std::vec::Vec` | fixtures/reject/src/lib.rs:305:52 |
| 1 | `DoubleEndedIterator` | fixtures/reject/src/lib.rs:186:1 |
| 1 | `Entry` | fixtures/reject/src/lib.rs:399:13 |
| 1 | `FusedIterator` | fixtures/reject/src/lib.rs:240:1 |
| 1 | `HashMap` | fixtures/reject/src/lib.rs:323:6 |
| 1 | `Hasher` | fixtures/reject/src/lib.rs:331:9 |
| 1 | `IntToFloat` | fixtures/reject/src/lib.rs:118:2 |
| 1 | `Noisy` | fixtures/reject/src/lib.rs:55:13 |
| 1 | `[u16; 1]` | fixtures/reject/src/lib.rs:272:22 |
| 1 | `dyn std::ops::Fn(u8) -> u8` | fixtures/reject/src/lib.rs:10:16 |
| 1 | `into_keys` | fixtures/reject/src/lib.rs:55:13 |
| 1 | `std::iter::Chain` | fixtures/reject/src/lib.rs:199:51 |
| 1 | `std::iter::Flatten` | fixtures/reject/src/lib.rs:248:26 |
| 1 | `std::iter::Rev` | fixtures/reject/src/lib.rs:193:14 |
| 1 | `std::mem` | fixtures/reject/src/lib.rs:48:29 |
| 1 | `std::ops::RangeFrom` | fixtures/reject/src/lib.rs:295:48 |
| 1 | `std::ops::RangeTo` | fixtures/reject/src/lib.rs:295:7 |
| 1 | `std::slice::Iter` | fixtures/reject/src/lib.rs:153:11 |
| 1 | `std::str` | fixtures/reject/src/lib.rs:389:4 |
| 1 | `std::sync::MutexGuard` | fixtures/reject/src/lib.rs:148:2 |
| 1 | `u64` | fixtures/reject/src/lib.rs:118:2 |
| 1 | `{async closure}` | fixtures/reject/src/lib.rs:74:2 |

## By type or path

| Count | Construct | Type or path | First |
| ---: | --- | --- | --- |
| 6 | `_` is not supported: its `None` and `Some` would both be `undefined` | `Option` | fixtures/reject/src/lib.rs:27:15 |
| 3 | unsupported type `_` | `f64` | fixtures/reject/src/lib.rs:6:14 |
| 2 | `_` is not supported: it would be pending two ways with one value | `Poll` | fixtures/reject/src/lib.rs:127:20 |
| 2 | `_`: a JS value of `_` is a copy, so a reference can't change it | `&mut std::string::String`, `std::string::String` | fixtures/reject/src/lib.rs:94:16 |
| 2 | `_`: a JS value of `_` is a copy, so a reference can't change it | `&mut u32`, `u32` | fixtures/reject/src/lib.rs:15:13 |
| 2 | `_`: a JS value of `_` is a copy, so a reference can't change it | `&mut u8`, `u8` | fixtures/reject/src/lib.rs:64:10 |
| 2 | an assignment through a reference to `_` | `std::pin::Pin` | fixtures/reject/src/lib.rs:143:2 |
| 2 | an assignment through a reference to `_` | `u8` | fixtures/reject/src/lib.rs:65:3 |
| 2 | replacing the value behind a reference to `_` | `Meters` | fixtures/reject/src/lib.rs:107:2 |
| 1 | `_` holds `_`, which is `undefined` as `None` is, which is not supported | `std::collections::HashMap`, `std::option::Option` | fixtures/reject/src/lib.rs:300:6 |
| 1 | `_` of `_` by `_`, which is not supported yet | `HashMap::contains_key`, `std::collections::HashMap`, `u8` | fixtures/reject/src/lib.rs:323:6 |
| 1 | `_` of `_`, which drops `_`, which is not supported yet | `into_keys`, `std::collections::HashMap`, `Noisy` | fixtures/reject/src/lib.rs:55:13 |
| 1 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | `std::iter::Iterator::count` | fixtures/reject/src/lib.rs:279:38 |
| 1 | `_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it | `std::iter::Iterator::filter` | fixtures/reject/src/lib.rs:279:16 |
| 1 | `_` through a held type's own, which is not supported yet | `<std::sync::Arc as PartialEq>::ne` | fixtures/reject/src/lib.rs:355:2 |
| 1 | `_`, a mutable reference to a value a map holds, kept other than to read and write it | `std::option::Option` | fixtures/reject/src/lib.rs:365:6 |
| 1 | `_`, which is not supported yet | `Hasher::write` | fixtures/reject/src/lib.rs:331:9 |
| 1 | `fold` of `_`, which folds a `flatten` whose source and iterators both drop, which is not supported yet | `std::iter::Flatten` | fixtures/reject/src/lib.rs:248:26 |
| 1 | a `_` cast from `_` to `_` | `IntToFloat`, `u64`, `f64` | fixtures/reject/src/lib.rs:118:2 |
| 1 | a `zip` of `_` and `_`, which std reads by index, of items that drop, which is not supported yet | `std::iter::Map`, `std::ops::Range` | fixtures/reject/src/lib.rs:253:30 |
| 1 | a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` | `std::iter::Chain` | fixtures/reject/src/lib.rs:199:51 |
| 1 | a coercion to `_` | `std::sync::Arc` | fixtures/reject/src/lib.rs:378:2 |
| 1 | a constant of type `_` | `&u8` | fixtures/reject/src/lib.rs:102:2 |
| 1 | a field of `_` | `std::ops::RangeFrom` | fixtures/reject/src/lib.rs:295:48 |
| 1 | a field of `_` | `std::ops::RangeTo` | fixtures/reject/src/lib.rs:295:7 |
| 1 | a mutable borrow of a subslice of `_`, which JS would copy, having no views | `u32` | fixtures/reject/src/lib.rs:258:13 |
| 1 | a scalar constant of type `_` | `f64` | fixtures/reject/src/lib.rs:7:2 |
| 1 | a zero-sized constant of type `_` | `{async closure}` | fixtures/reject/src/lib.rs:74:2 |
| 1 | an assignment through a reference to `_` | `std::sync::Arc` | fixtures/reject/src/lib.rs:123:2 |
| 1 | an assignment through a reference to `_` | `std::sync::MutexGuard` | fixtures/reject/src/lib.rs:148:2 |
| 1 | an assignment through a reference to `_` | `u32` | fixtures/reject/src/lib.rs:16:2 |
| 1 | an iterator of the crate implementing `_`, which is not supported yet | `DoubleEndedIterator` | fixtures/reject/src/lib.rs:186:1 |
| 1 | an iterator of the crate implementing `_`, which is not supported yet | `FusedIterator` | fixtures/reject/src/lib.rs:240:1 |
| 1 | arithmetic on `_` | `f64` | fixtures/reject/src/lib.rs:7:2 |
| 1 | comparing `_` as a key, which is not supported yet | `std::vec::Vec` | fixtures/reject/src/lib.rs:305:52 |
| 1 | constructing `_` from another crate | `std::iter::Map` | fixtures/reject/src/lib.rs:172:23 |
| 1 | constructing `_` from another crate | `std::iter::Rev` | fixtures/reject/src/lib.rs:193:14 |
| 1 | dropping `_`, which is not supported yet | `std::iter::Map` | fixtures/reject/src/lib.rs:49:18 |
| 1 | hashing `_` as a key, which is not supported yet | `std::vec::Vec` | fixtures/reject/src/lib.rs:305:52 |
| 1 | overriding `_`, which std's adaptors call in place of `next` or beside it | `Iterator::fold` | fixtures/reject/src/lib.rs:166:2 |
| 1 | overriding `_`, which std's adaptors call in place of `next` or beside it | `Iterator::size_hint` | fixtures/reject/src/lib.rs:220:2 |
| 1 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | `{closure}`, `Entry::or_insert_with` | fixtures/reject/src/lib.rs:399:13 |
| 1 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | `{closure}`, `Iterator::any` | fixtures/reject/src/lib.rs:400:11 |
| 1 | passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet | `{closure}`, `Iterator::map` | fixtures/reject/src/lib.rs:49:11 |
| 1 | replacing the value behind a reference to `_` | `[u16; 1]` | fixtures/reject/src/lib.rs:272:22 |
| 1 | replacing the value behind a reference to `_` | `std::string::String` | fixtures/reject/src/lib.rs:383:4 |
| 1 | unsupported call to `_` | `<std::slice::Iter as std::iter::Iterator>::cycle` | fixtures/reject/src/lib.rs:153:11 |
| 1 | unsupported call to `_` | `<std::string::String as std::fmt::Write>::write_fmt` | fixtures/reject/src/lib.rs:96:10 |
| 1 | unsupported call to `_` | `std::mem::size_of_val` | fixtures/reject/src/lib.rs:48:29 |
| 1 | unsupported call to `_` | `std::ops::Range::contains` | fixtures/reject/src/lib.rs:207:8 |
| 1 | unsupported call to `_` | `std::slice::copy_within` | fixtures/reject/src/lib.rs:373:4 |
| 1 | unsupported call to `_` | `std::slice::sort` | fixtures/reject/src/lib.rs:32:4 |
| 1 | unsupported call to `_` | `std::str::split` | fixtures/reject/src/lib.rs:389:4 |
| 1 | unsupported type `_` | `dyn std::ops::Fn(u8) -> u8` | fixtures/reject/src/lib.rs:10:16 |

## Spans

<details><summary>an assignment through a reference to `_` (7)</summary>

- fixtures/reject/src/lib.rs:16:2: an assignment through a reference to `u32`
- fixtures/reject/src/lib.rs:65:3: an assignment through a reference to `u8`
- fixtures/reject/src/lib.rs:123:2: an assignment through a reference to `std::sync::Arc<u8>`
- fixtures/reject/src/lib.rs:143:2: an assignment through a reference to `std::pin::Pin<std::sync::Arc<u8>>`
- fixtures/reject/src/lib.rs:148:2: an assignment through a reference to `std::sync::MutexGuard<'_, u8>`
- fixtures/reject/src/lib.rs:266:2: an assignment through a reference to `std::pin::Pin<&mut Noisy>`
- fixtures/reject/src/lib.rs:366:3: an assignment through a reference to `u8`

</details>

<details><summary>unsupported call to `_` (7)</summary>

- fixtures/reject/src/lib.rs:32:4: unsupported call to `std::slice::<impl [u8]>::sort`
- fixtures/reject/src/lib.rs:48:29: unsupported call to `std::mem::size_of_val::<Noisy>`
- fixtures/reject/src/lib.rs:96:10: unsupported call to `<std::string::String as std::fmt::Write>::write_fmt`
- fixtures/reject/src/lib.rs:153:11: unsupported call to `<std::slice::Iter<'_, u8> as std::iter::Iterator>::cycle`
- fixtures/reject/src/lib.rs:207:8: unsupported call to `std::ops::Range::<Key>::contains::<Key>`
- fixtures/reject/src/lib.rs:373:4: unsupported call to `core::slice::<impl [(u8, u8)]>::copy_within::<std::ops::Range<usize>>`
- fixtures/reject/src/lib.rs:389:4: unsupported call to `core::str::<impl str>::split::<[char; 2]>`

</details>

<details><summary>`_` is not supported: its `None` and `Some` would both be `undefined` (6)</summary>

- fixtures/reject/src/lib.rs:27:15: `Option<std::option::Option<u8>>` is not supported: its `None` and `Some` would both be `undefined`
- fixtures/reject/src/lib.rs:28:2: `Option<std::option::Option<u8>>` is not supported: its `None` and `Some` would both be `undefined`
- fixtures/reject/src/lib.rs:36:2: `Option<std::option::Option<u8>>` is not supported: its `None` and `Some` would both be `undefined`
- fixtures/reject/src/lib.rs:36:42: `Option<std::option::Option<u8>>` is not supported: its `None` and `Some` would both be `undefined`
- fixtures/reject/src/lib.rs:132:19: `Option<std::task::Poll<()>>` is not supported: its `None` and `Some` would both be `undefined`
- fixtures/reject/src/lib.rs:300:2: `Option<&std::option::Option<u8>>` is not supported: its `None` and `Some` would both be `undefined`

</details>

<details><summary>`_`: a JS value of `_` is a copy, so a reference can't change it (6)</summary>

- fixtures/reject/src/lib.rs:15:13: `&mut u32`: a JS value of `u32` is a copy, so a reference can't change it
- fixtures/reject/src/lib.rs:21:2: `&mut u32`: a JS value of `u32` is a copy, so a reference can't change it
- fixtures/reject/src/lib.rs:64:10: `&mut u8`: a JS value of `u8` is a copy, so a reference can't change it
- fixtures/reject/src/lib.rs:94:16: `&mut std::string::String`: a JS value of `std::string::String` is a copy, so a reference can't change it
- fixtures/reject/src/lib.rs:365:29: `&mut u8`: a JS value of `u8` is a copy, so a reference can't change it
- fixtures/reject/src/lib.rs:382:15: `&mut std::string::String`: a JS value of `std::string::String` is a copy, so a reference can't change it

</details>

<details><summary>replacing the value behind a reference to `_` (4)</summary>

- fixtures/reject/src/lib.rs:107:2: replacing the value behind a reference to `Meters`
- fixtures/reject/src/lib.rs:111:2: replacing the value behind a reference to `Meters`
- fixtures/reject/src/lib.rs:272:22: replacing the value behind a reference to `[u16; 1]`
- fixtures/reject/src/lib.rs:383:4: replacing the value behind a reference to `std::string::String`

</details>

<details><summary>unsupported type `_` (4)</summary>

- fixtures/reject/src/lib.rs:6:14: unsupported type `f64`
- fixtures/reject/src/lib.rs:6:25: unsupported type `f64`
- fixtures/reject/src/lib.rs:10:16: unsupported type `dyn std::ops::Fn(u8) -> u8`
- fixtures/reject/src/lib.rs:116:51: unsupported type `f64`

</details>

<details><summary>passing `_`, a closure that owns a value that drops, to `_`, which is not supported yet (3)</summary>

- fixtures/reject/src/lib.rs:49:11: passing `{closure@fixtures/reject/src/lib.rs:48:10: 48:23}`, a closure that owns a value that drops, to `Iterator::map`, which is not supported yet
- fixtures/reject/src/lib.rs:399:13: passing `{closure@fixtures/reject/src/lib.rs:399:28: 399:35}`, a closure that owns a value that drops, to `Entry::or_insert_with`, which is not supported yet
- fixtures/reject/src/lib.rs:400:11: passing `{closure@fixtures/reject/src/lib.rs:400:15: 400:24}`, a closure that owns a value that drops, to `Iterator::any`, which is not supported yet

</details>

<details><summary>`_` is not supported: it would be pending two ways with one value (2)</summary>

- fixtures/reject/src/lib.rs:127:20: `Poll<std::option::Option<std::task::Poll<u8>>>` is not supported: it would be pending two ways with one value
- fixtures/reject/src/lib.rs:137:54: `Poll<std::task::Poll<u8>>` is not supported: it would be pending two ways with one value

</details>

<details><summary>`_` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it (2)</summary>

- fixtures/reject/src/lib.rs:279:16: `std::iter::Iterator::filter` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it
- fixtures/reject/src/lib.rs:279:38: `std::iter::Iterator::count` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it

</details>

<details><summary>a field of `_` (2)</summary>

- fixtures/reject/src/lib.rs:295:7: a field of `std::ops::RangeTo<usize>`
- fixtures/reject/src/lib.rs:295:48: a field of `std::ops::RangeFrom<usize>`

</details>

<details><summary>an iterator of the crate implementing `_`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:186:1: an iterator of the crate implementing `DoubleEndedIterator`, which is not supported yet
- fixtures/reject/src/lib.rs:240:1: an iterator of the crate implementing `FusedIterator`, which is not supported yet

</details>

<details><summary>constructing `_` from another crate (2)</summary>

- fixtures/reject/src/lib.rs:172:23: constructing `std::iter::Map<Folds, {closure@fixtures/reject/src/lib.rs:172:12: 172:15}>` from another crate
- fixtures/reject/src/lib.rs:193:14: constructing `std::iter::Rev<Backs>` from another crate

</details>

<details><summary>overriding `_`, which std's adaptors call in place of `next` or beside it (2)</summary>

- fixtures/reject/src/lib.rs:166:2: overriding `Iterator::fold`, which std's adaptors call in place of `next` or beside it
- fixtures/reject/src/lib.rs:220:2: overriding `Iterator::size_hint`, which std's adaptors call in place of `next` or beside it

</details>

<details><summary>`Binary` formatting, as in `{:b}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:84:3: `Binary` formatting, as in `{:b}`, is not supported

</details>

<details><summary>`LowerExp` formatting, as in `{:e}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:86:3: `LowerExp` formatting, as in `{:e}`, is not supported

</details>

<details><summary>`Octal` formatting, as in `{:o}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:85:3: `Octal` formatting, as in `{:o}`, is not supported

</details>

<details><summary>`Pointer` formatting, as in `{:p}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:89:3: `Pointer` formatting, as in `{:p}`, is not supported

</details>

<details><summary>`_` holds `_`, which is `undefined` as `None` is, which is not supported (1)</summary>

- fixtures/reject/src/lib.rs:300:6: `std::collections::HashMap<u8, std::option::Option<u8>>` holds `std::option::Option<u8>`, which is `undefined` as `None` is, which is not supported

</details>

<details><summary>`_` of `_` by `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:323:6: `HashMap::contains_key` of `std::collections::HashMap<Id, u8>` by `u8`, which is not supported yet

</details>

<details><summary>`_` of `_`, which drops `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:55:13: `into_keys` of `std::collections::HashMap<u8, Noisy>`, which drops `Noisy`, which is not supported yet

</details>

<details><summary>`_` through a held type's own, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:355:2: `<std::sync::Arc<Picky> as PartialEq>::ne` through a held type's own, which is not supported yet

</details>

<details><summary>`_`, a mutable reference to a value a map holds, kept other than to read and write it (1)</summary>

- fixtures/reject/src/lib.rs:365:6: `std::option::Option<&mut u8>`, a mutable reference to a value a map holds, kept other than to read and write it

</details>

<details><summary>`_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:331:9: `Hasher::write`, which is not supported yet

</details>

<details><summary>`fold` of `_`, which folds a `flatten` whose source and iterators both drop, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:248:26: `fold` of `std::iter::Flatten<std::vec::IntoIter<std::vec::Vec<Noisy>>>`, which folds a `flatten` whose source and iterators both drop, which is not supported yet

</details>

<details><summary>a `_` cast from `_` to `_` (1)</summary>

- fixtures/reject/src/lib.rs:118:2: a `IntToFloat` cast from `u64` to `f64`

</details>

<details><summary>a `zip` of `_` and `_`, which std reads by index, of items that drop, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:253:30: a `zip` of `std::iter::Map<std::slice::Iter<'_, u8>, {closure@fixtures/reject/src/lib.rs:253:15: 253:19}>` and `std::ops::Range<i32>`, which std reads by index, of items that drop, which is not supported yet

</details>

<details><summary>a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` (1)</summary>

- fixtures/reject/src/lib.rs:199:51: a `zip` of `std::iter::Chain<std::iter::Map<std::slice::Iter<'_, u8>, {closure@fixtures/reject/src/lib.rs:199:15: 199:18}>, std::iter::Copied<std::slice::Iter<'_, u8>>>`, whose closures std calls one item further through `next` than through `fold`

</details>

<details><summary>a coercion to `_` (1)</summary>

- fixtures/reject/src/lib.rs:378:2: a coercion to `std::sync::Arc<[u8]>`

</details>

<details><summary>a constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:102:2: a constant of type `&u8`

</details>

<details><summary>a copy of a closure that changes what it captured (1)</summary>

- fixtures/reject/src/lib.rs:289:14: a copy of a closure that changes what it captured

</details>

<details><summary>a mutable borrow of a subslice of `_`, which JS would copy, having no views (1)</summary>

- fixtures/reject/src/lib.rs:258:13: a mutable borrow of a subslice of `u32`, which JS would copy, having no views

</details>

<details><summary>a precision, as in `{:.2}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:81:3: a precision, as in `{:.2}`, is not supported

</details>

<details><summary>a printable type must implement `Display` or `Debug` (1)</summary>

- fixtures/reject/src/lib.rs:114:1: a printable type must implement `Display` or `Debug`

</details>

<details><summary>a scalar constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:7:2: a scalar constant of type `f64`

</details>

<details><summary>a width or precision from an argument, as in `{:1$}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:88:3: a width or precision from an argument, as in `{:1$}`, is not supported

</details>

<details><summary>a zero-sized constant of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:74:2: a zero-sized constant of type `{async closure@fixtures/reject/src/lib.rs:73:10: 73:18}`

</details>

<details><summary>arithmetic on `_` (1)</summary>

- fixtures/reject/src/lib.rs:7:2: arithmetic on `f64`

</details>

<details><summary>comparing `_` as a key, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:305:52: comparing `std::vec::Vec<u8>` as a key, which is not supported yet

</details>

<details><summary>dropping `_`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:49:18: dropping `std::iter::Map<std::slice::Iter<'_, u8>, {closure@fixtures/reject/src/lib.rs:48:10: 48:23}>`, which is not supported yet

</details>

<details><summary>hashing `_` as a key, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:305:52: hashing `std::vec::Vec<u8>` as a key, which is not supported yet

</details>

<details><summary>hex `Debug`, as in `{:x?}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:87:3: hex `Debug`, as in `{:x?}`, is not supported

</details>

<details><summary>the `+` flag of a format spec is not supported (1)</summary>

- fixtures/reject/src/lib.rs:82:3: the `+` flag of a format spec is not supported

</details>

<details><summary>the `-` flag of a format spec is not supported (1)</summary>

- fixtures/reject/src/lib.rs:83:3: the `-` flag of a format spec is not supported

</details>
