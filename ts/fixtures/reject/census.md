# Census of `reject-fixture`

- Refusals: 70
- Function instances: 60
- Generic functions no instance reached, lowered over their type parameters: 3

## By construct

| Count | Construct | First |
| ---: | --- | --- |
| 6 | unsupported type `_` | fixtures/reject/src/lib.rs:6:1 |
| 4 | unsupported call to `_` | fixtures/reject/src/lib.rs:36:4 |
| 3 | `_` is not supported: its `None` and `Some` would both be `undefined` | fixtures/reject/src/lib.rs:31:15 |
| 3 | a `_` binding: a JS value of `_` is a copy, so it can't change | fixtures/reject/src/lib.rs:19:13 |
| 3 | an iterator that owns `_`, which drops, which is not supported yet | fixtures/reject/src/lib.rs:361:4 |
| 3 | dropping `_`, which is not supported yet | fixtures/reject/src/lib.rs:66:31 |
| 3 | replacing the value behind a reference to `_` | fixtures/reject/src/lib.rs:168:17 |
| 2 | `_`: a JS value of `_` is a copy, so a reference can't change it | fixtures/reject/src/lib.rs:19:13 |
| 2 | a `for` loop over values that drop, which is not supported yet | fixtures/reject/src/lib.rs:76:6 |
| 2 | a `match` on a temporary that drops, which it doesn't move out of whole | fixtures/reject/src/lib.rs:217:2 |
| 2 | an iterator of the crate implementing `_`, which is not supported yet | fixtures/reject/src/lib.rs:299:1 |
| 2 | overriding `_`, which std's adaptors call in place of `next` or beside it | fixtures/reject/src/lib.rs:279:2 |
| 2 | replacing the `_` behind a reference, whose JS object is shared or private | fixtures/reject/src/lib.rs:225:2 |
| 1 | `Binary` formatting, as in `{:b}`, is not supported | fixtures/reject/src/lib.rs:132:3 |
| 1 | `LowerExp` formatting, as in `{:e}`, is not supported | fixtures/reject/src/lib.rs:134:3 |
| 1 | `Octal` formatting, as in `{:o}`, is not supported | fixtures/reject/src/lib.rs:133:3 |
| 1 | `Pointer` formatting, as in `{:p}`, is not supported | fixtures/reject/src/lib.rs:137:3 |
| 1 | `_` is generic and implements `Drop`, which is not supported yet | fixtures/reject/src/lib.rs:97:1 |
| 1 | `_` is not supported: it would be pending two ways with one value | fixtures/reject/src/lib.rs:229:20 |
| 1 | a `let` on a temporary that drops, which it doesn't move out of whole | fixtures/reject/src/lib.rs:200:2 |
| 1 | a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` | fixtures/reject/src/lib.rs:312:51 |
| 1 | a cast from `_` | fixtures/reject/src/lib.rs:11:2 |
| 1 | a cast to `_` | fixtures/reject/src/lib.rs:179:2 |
| 1 | a closure of type `_` | fixtures/reject/src/lib.rs:121:10 |
| 1 | a closure that captures a value that drops, which is not supported yet | fixtures/reject/src/lib.rs:68:23 |
| 1 | a closure that copies a mutable variable | fixtures/reject/src/lib.rs:149:18 |
| 1 | a field of a temporary that drops: bind it to a variable first | fixtures/reject/src/lib.rs:62:2 |
| 1 | a method that takes `self` by value that drops, which is not supported yet | fixtures/reject/src/lib.rs:51:20 |
| 1 | a mutable reference to `_`, which is no object it can change in place | fixtures/reject/src/lib.rs:144:17 |
| 1 | a parameter that drops bound by a pattern, which is not supported yet | fixtures/reject/src/lib.rs:83:21 |
| 1 | a precision, as in `{:.2}`, is not supported | fixtures/reject/src/lib.rs:129:3 |
| 1 | a printable type must implement `Display` or `Debug` | fixtures/reject/src/lib.rs:175:1 |
| 1 | a variable from outside the function | fixtures/reject/src/lib.rs:84:2 |
| 1 | a width or precision from an argument, as in `{:1$}`, is not supported | fixtures/reject/src/lib.rs:136:3 |
| 1 | an `.await` in an operand after one that drops, which is not supported yet: bind the awaited value first | fixtures/reject/src/lib.rs:190:13 |
| 1 | an `.await` in the place an assignment of a value that drops writes to, which is not supported yet: bind the awaited value first | fixtures/reject/src/lib.rs:207:2 |
| 1 | an `if let` on a temporary that drops, which it doesn't move out of whole | fixtures/reject/src/lib.rs:241:9 |
| 1 | an async body returning `_` is not supported: its `Ready` could be pending | fixtures/reject/src/lib.rs:245:54 |
| 1 | an async body that borrows `_` mutably, which is no object it can change in place | fixtures/reject/src/lib.rs:113:3 |
| 1 | an or-pattern with bindings | fixtures/reject/src/lib.rs:156:3 |
| 1 | arithmetic on `_` | fixtures/reject/src/lib.rs:7:2 |
| 1 | hex `Debug`, as in `{:x?}`, is not supported | fixtures/reject/src/lib.rs:135:3 |
| 1 | the `+` flag of a format spec is not supported | fixtures/reject/src/lib.rs:130:3 |
| 1 | the `-` flag of a format spec is not supported | fixtures/reject/src/lib.rs:131:3 |
| 1 | unsupported expression: StaticRef | fixtures/reject/src/lib.rs:163:2 |
| 1 | unsupported literal of type `_` | fixtures/reject/src/lib.rs:7:6 |

## By API

The type or module each refused type or path belongs to.

| Count | API | First |
| ---: | --- | --- |
| 6 | `f64` | fixtures/reject/src/lib.rs:6:1 |
| 4 | `std::string::String` | fixtures/reject/src/lib.rs:142:16 |
| 3 | `Meters` | fixtures/reject/src/lib.rs:168:17 |
| 3 | `Option` | fixtures/reject/src/lib.rs:31:15 |
| 3 | `u32` | fixtures/reject/src/lib.rs:19:13 |
| 2 | `Iterator` | fixtures/reject/src/lib.rs:279:2 |
| 2 | `Noisy` | fixtures/reject/src/lib.rs:361:16 |
| 2 | `char` | fixtures/reject/src/lib.rs:10:18 |
| 2 | `std::sync::Arc` | fixtures/reject/src/lib.rs:183:42 |
| 2 | `{async closure}` | fixtures/reject/src/lib.rs:121:6 |
| 1 | `Boxed` | fixtures/reject/src/lib.rs:97:1 |
| 1 | `DoubleEndedIterator` | fixtures/reject/src/lib.rs:299:1 |
| 1 | `FusedIterator` | fixtures/reject/src/lib.rs:353:1 |
| 1 | `Poll` | fixtures/reject/src/lib.rs:229:20 |
| 1 | `Slot` | fixtures/reject/src/lib.rs:93:1 |
| 1 | `dyn std::ops::Fn(u8) -> u8` | fixtures/reject/src/lib.rs:14:16 |
| 1 | `std::iter::Chain` | fixtures/reject/src/lib.rs:312:51 |
| 1 | `std::ops::Range` | fixtures/reject/src/lib.rs:320:8 |
| 1 | `std::pin::Pin` | fixtures/reject/src/lib.rs:261:2 |
| 1 | `std::slice::Iter` | fixtures/reject/src/lib.rs:266:11 |
| 1 | `std::slice` | fixtures/reject/src/lib.rs:36:4 |
| 1 | `std::task::Poll` | fixtures/reject/src/lib.rs:245:54 |
| 1 | `std::vec::Vec` | fixtures/reject/src/lib.rs:361:4 |
| 1 | `u8` | fixtures/reject/src/lib.rs:113:3 |
| 1 | `{closure}` | fixtures/reject/src/lib.rs:66:31 |

## By type or path

| Count | Construct | Type or path | First |
| ---: | --- | --- | --- |
| 3 | `_` is not supported: its `None` and `Some` would both be `undefined` | `Option` | fixtures/reject/src/lib.rs:31:15 |
| 3 | replacing the value behind a reference to `_` | `Meters` | fixtures/reject/src/lib.rs:168:17 |
| 3 | unsupported type `_` | `f64` | fixtures/reject/src/lib.rs:6:1 |
| 2 | a `_` binding: a JS value of `_` is a copy, so it can't change | `&mut u32`, `u32` | fixtures/reject/src/lib.rs:19:13 |
| 2 | an iterator that owns `_`, which drops, which is not supported yet | `Noisy` | fixtures/reject/src/lib.rs:361:16 |
| 1 | `_` is generic and implements `Drop`, which is not supported yet | `Boxed` | fixtures/reject/src/lib.rs:97:1 |
| 1 | `_` is not supported: it would be pending two ways with one value | `Poll` | fixtures/reject/src/lib.rs:229:20 |
| 1 | `_`: a JS value of `_` is a copy, so a reference can't change it | `&mut std::string::String`, `std::string::String` | fixtures/reject/src/lib.rs:142:16 |
| 1 | `_`: a JS value of `_` is a copy, so a reference can't change it | `&mut u32`, `u32` | fixtures/reject/src/lib.rs:19:13 |
| 1 | a `_` binding: a JS value of `_` is a copy, so it can't change | `&mut std::string::String`, `std::string::String` | fixtures/reject/src/lib.rs:142:16 |
| 1 | a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` | `std::iter::Chain` | fixtures/reject/src/lib.rs:312:51 |
| 1 | a cast from `_` | `char` | fixtures/reject/src/lib.rs:11:2 |
| 1 | a cast to `_` | `f64` | fixtures/reject/src/lib.rs:179:2 |
| 1 | a closure of type `_` | `{async closure}` | fixtures/reject/src/lib.rs:121:10 |
| 1 | a mutable reference to `_`, which is no object it can change in place | `std::string::String` | fixtures/reject/src/lib.rs:144:17 |
| 1 | an async body returning `_` is not supported: its `Ready` could be pending | `std::task::Poll` | fixtures/reject/src/lib.rs:245:54 |
| 1 | an async body that borrows `_` mutably, which is no object it can change in place | `u8` | fixtures/reject/src/lib.rs:113:3 |
| 1 | an iterator of the crate implementing `_`, which is not supported yet | `DoubleEndedIterator` | fixtures/reject/src/lib.rs:299:1 |
| 1 | an iterator of the crate implementing `_`, which is not supported yet | `FusedIterator` | fixtures/reject/src/lib.rs:353:1 |
| 1 | an iterator that owns `_`, which drops, which is not supported yet | `std::vec::Vec` | fixtures/reject/src/lib.rs:361:4 |
| 1 | arithmetic on `_` | `f64` | fixtures/reject/src/lib.rs:7:2 |
| 1 | dropping `_`, which is not supported yet | `Slot` | fixtures/reject/src/lib.rs:93:1 |
| 1 | dropping `_`, which is not supported yet | `std::sync::Arc` | fixtures/reject/src/lib.rs:183:42 |
| 1 | dropping `_`, which is not supported yet | `{closure}` | fixtures/reject/src/lib.rs:66:31 |
| 1 | overriding `_`, which std's adaptors call in place of `next` or beside it | `Iterator::fold` | fixtures/reject/src/lib.rs:279:2 |
| 1 | overriding `_`, which std's adaptors call in place of `next` or beside it | `Iterator::size_hint` | fixtures/reject/src/lib.rs:333:2 |
| 1 | replacing the `_` behind a reference, whose JS object is shared or private | `std::pin::Pin` | fixtures/reject/src/lib.rs:261:2 |
| 1 | replacing the `_` behind a reference, whose JS object is shared or private | `std::sync::Arc` | fixtures/reject/src/lib.rs:225:2 |
| 1 | unsupported call to `_` | `<std::slice::Iter as std::iter::Iterator>::cycle` | fixtures/reject/src/lib.rs:266:11 |
| 1 | unsupported call to `_` | `<std::string::String as std::fmt::Write>::write_fmt` | fixtures/reject/src/lib.rs:144:10 |
| 1 | unsupported call to `_` | `std::ops::Range::contains` | fixtures/reject/src/lib.rs:320:8 |
| 1 | unsupported call to `_` | `std::slice::sort` | fixtures/reject/src/lib.rs:36:4 |
| 1 | unsupported literal of type `_` | `f64` | fixtures/reject/src/lib.rs:7:6 |
| 1 | unsupported type `_` | `char` | fixtures/reject/src/lib.rs:10:18 |
| 1 | unsupported type `_` | `dyn std::ops::Fn(u8) -> u8` | fixtures/reject/src/lib.rs:14:16 |
| 1 | unsupported type `_` | `{async closure}` | fixtures/reject/src/lib.rs:121:6 |

## Spans

<details><summary>unsupported type `_` (6)</summary>

- fixtures/reject/src/lib.rs:6:1: unsupported type `f64`
- fixtures/reject/src/lib.rs:6:14: unsupported type `f64`
- fixtures/reject/src/lib.rs:10:18: unsupported type `char`
- fixtures/reject/src/lib.rs:14:16: unsupported type `dyn std::ops::Fn(u8) -> u8`
- fixtures/reject/src/lib.rs:121:6: unsupported type `{async closure@fixtures/reject/src/lib.rs:121:10: 121:18}`
- fixtures/reject/src/lib.rs:177:1: unsupported type `f64`

</details>

<details><summary>unsupported call to `_` (4)</summary>

- fixtures/reject/src/lib.rs:36:4: unsupported call to `std::slice::<impl [u8]>::sort`
- fixtures/reject/src/lib.rs:144:10: unsupported call to `<std::string::String as std::fmt::Write>::write_fmt`
- fixtures/reject/src/lib.rs:266:11: unsupported call to `<std::slice::Iter<'_, u8> as std::iter::Iterator>::cycle`
- fixtures/reject/src/lib.rs:320:8: unsupported call to `std::ops::Range::<Key>::contains::<Key>`

</details>

<details><summary>`_` is not supported: its `None` and `Some` would both be `undefined` (3)</summary>

- fixtures/reject/src/lib.rs:31:15: `Option<std::option::Option<u8>>` is not supported: its `None` and `Some` would both be `undefined`
- fixtures/reject/src/lib.rs:40:42: `Option<std::option::Option<u8>>` is not supported: its `None` and `Some` would both be `undefined`
- fixtures/reject/src/lib.rs:234:19: `Option<std::task::Poll<()>>` is not supported: its `None` and `Some` would both be `undefined`

</details>

<details><summary>a `_` binding: a JS value of `_` is a copy, so it can't change (3)</summary>

- fixtures/reject/src/lib.rs:19:13: a `&mut u32` binding: a JS value of `u32` is a copy, so it can't change
- fixtures/reject/src/lib.rs:25:6: a `&mut u32` binding: a JS value of `u32` is a copy, so it can't change
- fixtures/reject/src/lib.rs:142:16: a `&mut std::string::String` binding: a JS value of `std::string::String` is a copy, so it can't change

</details>

<details><summary>an iterator that owns `_`, which drops, which is not supported yet (3)</summary>

- fixtures/reject/src/lib.rs:361:4: an iterator that owns `std::vec::Vec<Noisy>`, which drops, which is not supported yet
- fixtures/reject/src/lib.rs:361:16: an iterator that owns `Noisy`, which drops, which is not supported yet
- fixtures/reject/src/lib.rs:366:20: an iterator that owns `Noisy`, which drops, which is not supported yet

</details>

<details><summary>dropping `_`, which is not supported yet (3)</summary>

- fixtures/reject/src/lib.rs:66:31: dropping `{closure@fixtures/reject/src/lib.rs:68:10: 68:17}`, which is not supported yet
- fixtures/reject/src/lib.rs:93:1: dropping `Slot`, which is not supported yet
- fixtures/reject/src/lib.rs:183:42: dropping `std::sync::Arc<&Noisy>`, which is not supported yet

</details>

<details><summary>replacing the value behind a reference to `_` (3)</summary>

- fixtures/reject/src/lib.rs:168:17: replacing the value behind a reference to `Meters`
- fixtures/reject/src/lib.rs:168:20: replacing the value behind a reference to `Meters`
- fixtures/reject/src/lib.rs:172:25: replacing the value behind a reference to `Meters`

</details>

<details><summary>`_`: a JS value of `_` is a copy, so a reference can't change it (2)</summary>

- fixtures/reject/src/lib.rs:19:13: `&mut u32`: a JS value of `u32` is a copy, so a reference can't change it
- fixtures/reject/src/lib.rs:142:16: `&mut std::string::String`: a JS value of `std::string::String` is a copy, so a reference can't change it

</details>

<details><summary>a `for` loop over values that drop, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:76:6: a `for` loop over values that drop, which is not supported yet
- fixtures/reject/src/lib.rs:382:6: a `for` loop over values that drop, which is not supported yet

</details>

<details><summary>a `match` on a temporary that drops, which it doesn't move out of whole (2)</summary>

- fixtures/reject/src/lib.rs:217:2: a `match` on a temporary that drops, which it doesn't move out of whole
- fixtures/reject/src/lib.rs:252:16: a `match` on a temporary that drops, which it doesn't move out of whole

</details>

<details><summary>an iterator of the crate implementing `_`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:299:1: an iterator of the crate implementing `DoubleEndedIterator`, which is not supported yet
- fixtures/reject/src/lib.rs:353:1: an iterator of the crate implementing `FusedIterator`, which is not supported yet

</details>

<details><summary>overriding `_`, which std's adaptors call in place of `next` or beside it (2)</summary>

- fixtures/reject/src/lib.rs:279:2: overriding `Iterator::fold`, which std's adaptors call in place of `next` or beside it
- fixtures/reject/src/lib.rs:333:2: overriding `Iterator::size_hint`, which std's adaptors call in place of `next` or beside it

</details>

<details><summary>replacing the `_` behind a reference, whose JS object is shared or private (2)</summary>

- fixtures/reject/src/lib.rs:225:2: replacing the `std::sync::Arc<u8>` behind a reference, whose JS object is shared or private
- fixtures/reject/src/lib.rs:261:2: replacing the `std::pin::Pin<std::sync::Arc<u8>>` behind a reference, whose JS object is shared or private

</details>

<details><summary>`Binary` formatting, as in `{:b}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:132:3: `Binary` formatting, as in `{:b}`, is not supported

</details>

<details><summary>`LowerExp` formatting, as in `{:e}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:134:3: `LowerExp` formatting, as in `{:e}`, is not supported

</details>

<details><summary>`Octal` formatting, as in `{:o}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:133:3: `Octal` formatting, as in `{:o}`, is not supported

</details>

<details><summary>`Pointer` formatting, as in `{:p}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:137:3: `Pointer` formatting, as in `{:p}`, is not supported

</details>

<details><summary>`_` is generic and implements `Drop`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:97:1: `Boxed` is generic and implements `Drop`, which is not supported yet

</details>

<details><summary>`_` is not supported: it would be pending two ways with one value (1)</summary>

- fixtures/reject/src/lib.rs:229:20: `Poll<std::option::Option<std::task::Poll<u8>>>` is not supported: it would be pending two ways with one value

</details>

<details><summary>a `let` on a temporary that drops, which it doesn't move out of whole (1)</summary>

- fixtures/reject/src/lib.rs:200:2: a `let` on a temporary that drops, which it doesn't move out of whole

</details>

<details><summary>a `zip` of `_`, whose closures std calls one item further through `next` than through `fold` (1)</summary>

- fixtures/reject/src/lib.rs:312:51: a `zip` of `std::iter::Chain<std::iter::Map<std::slice::Iter<'_, u8>, {closure@fixtures/reject/src/lib.rs:312:15: 312:18}>, std::iter::Copied<std::slice::Iter<'_, u8>>>`, whose closures std calls one item further through `next` than through `fold`

</details>

<details><summary>a cast from `_` (1)</summary>

- fixtures/reject/src/lib.rs:11:2: a cast from `char`

</details>

<details><summary>a cast to `_` (1)</summary>

- fixtures/reject/src/lib.rs:179:2: a cast to `f64`

</details>

<details><summary>a closure of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:121:10: a closure of type `{async closure@fixtures/reject/src/lib.rs:121:10: 121:18}`

</details>

<details><summary>a closure that captures a value that drops, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:68:23: a closure that captures a value that drops, which is not supported yet

</details>

<details><summary>a closure that copies a mutable variable (1)</summary>

- fixtures/reject/src/lib.rs:149:18: a closure that copies a mutable variable

</details>

<details><summary>a field of a temporary that drops: bind it to a variable first (1)</summary>

- fixtures/reject/src/lib.rs:62:2: a field of a temporary that drops: bind it to a variable first

</details>

<details><summary>a method that takes `self` by value that drops, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:51:20: a method that takes `self` by value that drops, which is not supported yet

</details>

<details><summary>a mutable reference to `_`, which is no object it can change in place (1)</summary>

- fixtures/reject/src/lib.rs:144:17: a mutable reference to `std::string::String`, which is no object it can change in place

</details>

<details><summary>a parameter that drops bound by a pattern, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:83:21: a parameter that drops bound by a pattern, which is not supported yet

</details>

<details><summary>a precision, as in `{:.2}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:129:3: a precision, as in `{:.2}`, is not supported

</details>

<details><summary>a printable type must implement `Display` or `Debug` (1)</summary>

- fixtures/reject/src/lib.rs:175:1: a printable type must implement `Display` or `Debug`

</details>

<details><summary>a variable from outside the function (1)</summary>

- fixtures/reject/src/lib.rs:84:2: a variable from outside the function

</details>

<details><summary>a width or precision from an argument, as in `{:1$}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:136:3: a width or precision from an argument, as in `{:1$}`, is not supported

</details>

<details><summary>an `.await` in an operand after one that drops, which is not supported yet: bind the awaited value first (1)</summary>

- fixtures/reject/src/lib.rs:190:13: an `.await` in an operand after one that drops, which is not supported yet: bind the awaited value first

</details>

<details><summary>an `.await` in the place an assignment of a value that drops writes to, which is not supported yet: bind the awaited value first (1)</summary>

- fixtures/reject/src/lib.rs:207:2: an `.await` in the place an assignment of a value that drops writes to, which is not supported yet: bind the awaited value first

</details>

<details><summary>an `if let` on a temporary that drops, which it doesn't move out of whole (1)</summary>

- fixtures/reject/src/lib.rs:241:9: an `if let` on a temporary that drops, which it doesn't move out of whole

</details>

<details><summary>an async body returning `_` is not supported: its `Ready` could be pending (1)</summary>

- fixtures/reject/src/lib.rs:245:54: an async body returning `std::task::Poll<u8>` is not supported: its `Ready` could be pending

</details>

<details><summary>an async body that borrows `_` mutably, which is no object it can change in place (1)</summary>

- fixtures/reject/src/lib.rs:113:3: an async body that borrows `u8` mutably, which is no object it can change in place

</details>

<details><summary>an or-pattern with bindings (1)</summary>

- fixtures/reject/src/lib.rs:156:3: an or-pattern with bindings

</details>

<details><summary>arithmetic on `_` (1)</summary>

- fixtures/reject/src/lib.rs:7:2: arithmetic on `f64`

</details>

<details><summary>hex `Debug`, as in `{:x?}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:135:3: hex `Debug`, as in `{:x?}`, is not supported

</details>

<details><summary>the `+` flag of a format spec is not supported (1)</summary>

- fixtures/reject/src/lib.rs:130:3: the `+` flag of a format spec is not supported

</details>

<details><summary>the `-` flag of a format spec is not supported (1)</summary>

- fixtures/reject/src/lib.rs:131:3: the `-` flag of a format spec is not supported

</details>

<details><summary>unsupported expression: StaticRef (1)</summary>

- fixtures/reject/src/lib.rs:163:2: unsupported expression: StaticRef

</details>

<details><summary>unsupported literal of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:7:6: unsupported literal of type `f64`

</details>
