# Census of `reject-fixture`

- Refusals: 49
- Function instances: 28
- Generic functions no instance reached, lowered over their type parameters: 2

## By construct

| Count | Construct | First |
| ---: | --- | --- |
| 6 | unsupported type `_` | fixtures/reject/src/lib.rs:6:1 |
| 5 | unsupported call to `_` | fixtures/reject/src/lib.rs:36:4 |
| 3 | a `_` binding: a JS value of `_` is a copy, so it can't change | fixtures/reject/src/lib.rs:19:13 |
| 3 | replacing the value behind a reference to `_` | fixtures/reject/src/lib.rs:168:17 |
| 2 | `_`: a JS value of `_` is a copy, so a reference can't change it | fixtures/reject/src/lib.rs:19:13 |
| 2 | dropping `_`, which is not supported yet | fixtures/reject/src/lib.rs:66:31 |
| 1 | `Binary` formatting, as in `{:b}`, is not supported | fixtures/reject/src/lib.rs:132:3 |
| 1 | `LowerExp` formatting, as in `{:e}`, is not supported | fixtures/reject/src/lib.rs:134:3 |
| 1 | `Octal` formatting, as in `{:o}`, is not supported | fixtures/reject/src/lib.rs:133:3 |
| 1 | `Pointer` formatting, as in `{:p}`, is not supported | fixtures/reject/src/lib.rs:137:3 |
| 1 | `_` is generic and implements `Drop`, which is not supported yet | fixtures/reject/src/lib.rs:97:1 |
| 1 | `_` is not supported: its `None` and `Some` would both be `undefined` | fixtures/reject/src/lib.rs:31:15 |
| 1 | a `for` loop over values that drop, which is not supported yet | fixtures/reject/src/lib.rs:76:6 |
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
| 3 | `u32` | fixtures/reject/src/lib.rs:19:13 |
| 2 | `char` | fixtures/reject/src/lib.rs:10:18 |
| 2 | `std::slice` | fixtures/reject/src/lib.rs:36:4 |
| 2 | `{async closure}` | fixtures/reject/src/lib.rs:121:6 |
| 1 | `Boxed` | fixtures/reject/src/lib.rs:97:1 |
| 1 | `Option` | fixtures/reject/src/lib.rs:31:15 |
| 1 | `Slot` | fixtures/reject/src/lib.rs:93:1 |
| 1 | `dyn std::ops::Fn(u8) -> u8` | fixtures/reject/src/lib.rs:14:16 |
| 1 | `std::iter::Filter` | fixtures/reject/src/lib.rs:40:31 |
| 1 | `std::slice::Iter` | fixtures/reject/src/lib.rs:40:11 |
| 1 | `u8` | fixtures/reject/src/lib.rs:113:3 |
| 1 | `{closure}` | fixtures/reject/src/lib.rs:66:31 |

## By type or path

| Count | Construct | Type or path | First |
| ---: | --- | --- | --- |
| 3 | replacing the value behind a reference to `_` | `Meters` | fixtures/reject/src/lib.rs:168:17 |
| 3 | unsupported type `_` | `f64` | fixtures/reject/src/lib.rs:6:1 |
| 2 | a `_` binding: a JS value of `_` is a copy, so it can't change | `&mut u32`, `u32` | fixtures/reject/src/lib.rs:19:13 |
| 1 | `_` is generic and implements `Drop`, which is not supported yet | `Boxed` | fixtures/reject/src/lib.rs:97:1 |
| 1 | `_` is not supported: its `None` and `Some` would both be `undefined` | `Option` | fixtures/reject/src/lib.rs:31:15 |
| 1 | `_`: a JS value of `_` is a copy, so a reference can't change it | `&mut std::string::String`, `std::string::String` | fixtures/reject/src/lib.rs:142:16 |
| 1 | `_`: a JS value of `_` is a copy, so a reference can't change it | `&mut u32`, `u32` | fixtures/reject/src/lib.rs:19:13 |
| 1 | a `_` binding: a JS value of `_` is a copy, so it can't change | `&mut std::string::String`, `std::string::String` | fixtures/reject/src/lib.rs:142:16 |
| 1 | a cast from `_` | `char` | fixtures/reject/src/lib.rs:11:2 |
| 1 | a cast to `_` | `f64` | fixtures/reject/src/lib.rs:179:2 |
| 1 | a closure of type `_` | `{async closure}` | fixtures/reject/src/lib.rs:121:10 |
| 1 | a mutable reference to `_`, which is no object it can change in place | `std::string::String` | fixtures/reject/src/lib.rs:144:17 |
| 1 | an async body that borrows `_` mutably, which is no object it can change in place | `u8` | fixtures/reject/src/lib.rs:113:3 |
| 1 | arithmetic on `_` | `f64` | fixtures/reject/src/lib.rs:7:2 |
| 1 | dropping `_`, which is not supported yet | `Slot` | fixtures/reject/src/lib.rs:93:1 |
| 1 | dropping `_`, which is not supported yet | `{closure}` | fixtures/reject/src/lib.rs:66:31 |
| 1 | unsupported call to `_` | `<std::iter::Filter as std::iter::Iterator>::count` | fixtures/reject/src/lib.rs:40:31 |
| 1 | unsupported call to `_` | `<std::slice::Iter as std::iter::Iterator>::filter` | fixtures/reject/src/lib.rs:40:11 |
| 1 | unsupported call to `_` | `<std::string::String as std::fmt::Write>::write_fmt` | fixtures/reject/src/lib.rs:144:10 |
| 1 | unsupported call to `_` | `std::slice::iter` | fixtures/reject/src/lib.rs:40:4 |
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

<details><summary>unsupported call to `_` (5)</summary>

- fixtures/reject/src/lib.rs:36:4: unsupported call to `std::slice::<impl [u8]>::sort`
- fixtures/reject/src/lib.rs:40:4: unsupported call to `core::slice::<impl [u8]>::iter`
- fixtures/reject/src/lib.rs:40:11: unsupported call to `<std::slice::Iter<'_, u8> as std::iter::Iterator>::filter::<{closure@fixtures/reject/src/lib.rs:40:18: 40:21}>`
- fixtures/reject/src/lib.rs:40:31: unsupported call to `<std::iter::Filter<std::slice::Iter<'_, u8>, {closure@fixtures/reject/src/lib.rs:40:18: 40:21}> as std::iter::Iterator>::count`
- fixtures/reject/src/lib.rs:144:10: unsupported call to `<std::string::String as std::fmt::Write>::write_fmt`

</details>

<details><summary>a `_` binding: a JS value of `_` is a copy, so it can't change (3)</summary>

- fixtures/reject/src/lib.rs:19:13: a `&mut u32` binding: a JS value of `u32` is a copy, so it can't change
- fixtures/reject/src/lib.rs:25:6: a `&mut u32` binding: a JS value of `u32` is a copy, so it can't change
- fixtures/reject/src/lib.rs:142:16: a `&mut std::string::String` binding: a JS value of `std::string::String` is a copy, so it can't change

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

<details><summary>dropping `_`, which is not supported yet (2)</summary>

- fixtures/reject/src/lib.rs:66:31: dropping `{closure@fixtures/reject/src/lib.rs:68:10: 68:17}`, which is not supported yet
- fixtures/reject/src/lib.rs:93:1: dropping `Slot`, which is not supported yet

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

<details><summary>`_` is not supported: its `None` and `Some` would both be `undefined` (1)</summary>

- fixtures/reject/src/lib.rs:31:15: `Option<std::option::Option<u8>>` is not supported: its `None` and `Some` would both be `undefined`

</details>

<details><summary>a `for` loop over values that drop, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:76:6: a `for` loop over values that drop, which is not supported yet

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
