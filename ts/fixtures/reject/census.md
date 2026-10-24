# Census of `reject-fixture`

- Refusals: 38
- Function instances: 19
- Generic functions no instance reached, lowered over their type parameters: 1

## By construct

| Count | Construct | First |
| ---: | --- | --- |
| 5 | unsupported call to `_` | fixtures/reject/src/lib.rs:36:4 |
| 5 | unsupported type `_` | fixtures/reject/src/lib.rs:6:1 |
| 3 | a `_` binding: a JS value of `_` is a copy, so it can't change | fixtures/reject/src/lib.rs:19:13 |
| 3 | replacing the value behind a reference to `_` | fixtures/reject/src/lib.rs:99:17 |
| 2 | `_`: a JS value of `_` is a copy, so a reference can't change it | fixtures/reject/src/lib.rs:19:13 |
| 1 | `Binary` formatting, as in `{:b}`, is not supported | fixtures/reject/src/lib.rs:63:3 |
| 1 | `LowerExp` formatting, as in `{:e}`, is not supported | fixtures/reject/src/lib.rs:65:3 |
| 1 | `Octal` formatting, as in `{:o}`, is not supported | fixtures/reject/src/lib.rs:64:3 |
| 1 | `Pointer` formatting, as in `{:p}`, is not supported | fixtures/reject/src/lib.rs:68:3 |
| 1 | `_` implements `Drop`, which is not supported yet | fixtures/reject/src/lib.rs:43:1 |
| 1 | `_` is async, which is not supported yet | fixtures/reject/src/lib.rs:53:1 |
| 1 | `_` is not supported: its `None` and `Some` would both be `undefined` | fixtures/reject/src/lib.rs:31:15 |
| 1 | a cast from `_` | fixtures/reject/src/lib.rs:11:2 |
| 1 | a cast to `_` | fixtures/reject/src/lib.rs:110:2 |
| 1 | a closure that copies a mutable variable | fixtures/reject/src/lib.rs:80:18 |
| 1 | a mutable reference to `_`, which is no object it can change in place | fixtures/reject/src/lib.rs:75:17 |
| 1 | a precision, as in `{:.2}`, is not supported | fixtures/reject/src/lib.rs:60:3 |
| 1 | a width or precision from an argument, as in `{:1$}`, is not supported | fixtures/reject/src/lib.rs:67:3 |
| 1 | an or-pattern with bindings | fixtures/reject/src/lib.rs:87:3 |
| 1 | arithmetic on `_` | fixtures/reject/src/lib.rs:7:2 |
| 1 | hex `Debug`, as in `{:x?}`, is not supported | fixtures/reject/src/lib.rs:66:3 |
| 1 | the `+` flag of a format spec is not supported | fixtures/reject/src/lib.rs:61:3 |
| 1 | the `-` flag of a format spec is not supported | fixtures/reject/src/lib.rs:62:3 |
| 1 | unsupported expression: StaticRef | fixtures/reject/src/lib.rs:94:2 |
| 1 | unsupported literal of type `_` | fixtures/reject/src/lib.rs:7:6 |

## By API

The type or module each refused type or path belongs to.

| Count | API | First |
| ---: | --- | --- |
| 6 | `f64` | fixtures/reject/src/lib.rs:6:1 |
| 4 | `std::string::String` | fixtures/reject/src/lib.rs:73:16 |
| 3 | `Meters` | fixtures/reject/src/lib.rs:99:17 |
| 3 | `u32` | fixtures/reject/src/lib.rs:19:13 |
| 2 | `char` | fixtures/reject/src/lib.rs:10:18 |
| 2 | `std::slice` | fixtures/reject/src/lib.rs:36:4 |
| 1 | `Guard` | fixtures/reject/src/lib.rs:43:1 |
| 1 | `Option` | fixtures/reject/src/lib.rs:31:15 |
| 1 | `dyn std::ops::Fn(u8) -> u8` | fixtures/reject/src/lib.rs:14:16 |
| 1 | `later` | fixtures/reject/src/lib.rs:53:1 |
| 1 | `std::iter::Filter` | fixtures/reject/src/lib.rs:40:31 |
| 1 | `std::slice::Iter` | fixtures/reject/src/lib.rs:40:11 |

## By type or path

| Count | Construct | Type or path | First |
| ---: | --- | --- | --- |
| 3 | replacing the value behind a reference to `_` | `Meters` | fixtures/reject/src/lib.rs:99:17 |
| 3 | unsupported type `_` | `f64` | fixtures/reject/src/lib.rs:6:1 |
| 2 | a `_` binding: a JS value of `_` is a copy, so it can't change | `&mut u32`, `u32` | fixtures/reject/src/lib.rs:19:13 |
| 1 | `_` implements `Drop`, which is not supported yet | `Guard` | fixtures/reject/src/lib.rs:43:1 |
| 1 | `_` is async, which is not supported yet | `later` | fixtures/reject/src/lib.rs:53:1 |
| 1 | `_` is not supported: its `None` and `Some` would both be `undefined` | `Option` | fixtures/reject/src/lib.rs:31:15 |
| 1 | `_`: a JS value of `_` is a copy, so a reference can't change it | `&mut std::string::String`, `std::string::String` | fixtures/reject/src/lib.rs:73:16 |
| 1 | `_`: a JS value of `_` is a copy, so a reference can't change it | `&mut u32`, `u32` | fixtures/reject/src/lib.rs:19:13 |
| 1 | a `_` binding: a JS value of `_` is a copy, so it can't change | `&mut std::string::String`, `std::string::String` | fixtures/reject/src/lib.rs:73:16 |
| 1 | a cast from `_` | `char` | fixtures/reject/src/lib.rs:11:2 |
| 1 | a cast to `_` | `f64` | fixtures/reject/src/lib.rs:110:2 |
| 1 | a mutable reference to `_`, which is no object it can change in place | `std::string::String` | fixtures/reject/src/lib.rs:75:17 |
| 1 | arithmetic on `_` | `f64` | fixtures/reject/src/lib.rs:7:2 |
| 1 | unsupported call to `_` | `<std::iter::Filter as std::iter::Iterator>::count` | fixtures/reject/src/lib.rs:40:31 |
| 1 | unsupported call to `_` | `<std::slice::Iter as std::iter::Iterator>::filter` | fixtures/reject/src/lib.rs:40:11 |
| 1 | unsupported call to `_` | `<std::string::String as std::fmt::Write>::write_fmt` | fixtures/reject/src/lib.rs:75:10 |
| 1 | unsupported call to `_` | `std::slice::iter` | fixtures/reject/src/lib.rs:40:4 |
| 1 | unsupported call to `_` | `std::slice::sort` | fixtures/reject/src/lib.rs:36:4 |
| 1 | unsupported literal of type `_` | `f64` | fixtures/reject/src/lib.rs:7:6 |
| 1 | unsupported type `_` | `char` | fixtures/reject/src/lib.rs:10:18 |
| 1 | unsupported type `_` | `dyn std::ops::Fn(u8) -> u8` | fixtures/reject/src/lib.rs:14:16 |

## Spans

<details><summary>unsupported call to `_` (5)</summary>

- fixtures/reject/src/lib.rs:36:4: unsupported call to `std::slice::<impl [u8]>::sort`
- fixtures/reject/src/lib.rs:40:4: unsupported call to `core::slice::<impl [u8]>::iter`
- fixtures/reject/src/lib.rs:40:11: unsupported call to `<std::slice::Iter<'_, u8> as std::iter::Iterator>::filter::<{closure@fixtures/reject/src/lib.rs:40:18: 40:21}>`
- fixtures/reject/src/lib.rs:40:31: unsupported call to `<std::iter::Filter<std::slice::Iter<'_, u8>, {closure@fixtures/reject/src/lib.rs:40:18: 40:21}> as std::iter::Iterator>::count`
- fixtures/reject/src/lib.rs:75:10: unsupported call to `<std::string::String as std::fmt::Write>::write_fmt`

</details>

<details><summary>unsupported type `_` (5)</summary>

- fixtures/reject/src/lib.rs:6:1: unsupported type `f64`
- fixtures/reject/src/lib.rs:6:14: unsupported type `f64`
- fixtures/reject/src/lib.rs:10:18: unsupported type `char`
- fixtures/reject/src/lib.rs:14:16: unsupported type `dyn std::ops::Fn(u8) -> u8`
- fixtures/reject/src/lib.rs:108:1: unsupported type `f64`

</details>

<details><summary>a `_` binding: a JS value of `_` is a copy, so it can't change (3)</summary>

- fixtures/reject/src/lib.rs:19:13: a `&mut u32` binding: a JS value of `u32` is a copy, so it can't change
- fixtures/reject/src/lib.rs:25:6: a `&mut u32` binding: a JS value of `u32` is a copy, so it can't change
- fixtures/reject/src/lib.rs:73:16: a `&mut std::string::String` binding: a JS value of `std::string::String` is a copy, so it can't change

</details>

<details><summary>replacing the value behind a reference to `_` (3)</summary>

- fixtures/reject/src/lib.rs:99:17: replacing the value behind a reference to `Meters`
- fixtures/reject/src/lib.rs:99:20: replacing the value behind a reference to `Meters`
- fixtures/reject/src/lib.rs:103:25: replacing the value behind a reference to `Meters`

</details>

<details><summary>`_`: a JS value of `_` is a copy, so a reference can't change it (2)</summary>

- fixtures/reject/src/lib.rs:19:13: `&mut u32`: a JS value of `u32` is a copy, so a reference can't change it
- fixtures/reject/src/lib.rs:73:16: `&mut std::string::String`: a JS value of `std::string::String` is a copy, so a reference can't change it

</details>

<details><summary>`Binary` formatting, as in `{:b}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:63:3: `Binary` formatting, as in `{:b}`, is not supported

</details>

<details><summary>`LowerExp` formatting, as in `{:e}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:65:3: `LowerExp` formatting, as in `{:e}`, is not supported

</details>

<details><summary>`Octal` formatting, as in `{:o}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:64:3: `Octal` formatting, as in `{:o}`, is not supported

</details>

<details><summary>`Pointer` formatting, as in `{:p}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:68:3: `Pointer` formatting, as in `{:p}`, is not supported

</details>

<details><summary>`_` implements `Drop`, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:43:1: `Guard` implements `Drop`, which is not supported yet

</details>

<details><summary>`_` is async, which is not supported yet (1)</summary>

- fixtures/reject/src/lib.rs:53:1: `later` is async, which is not supported yet

</details>

<details><summary>`_` is not supported: its `None` and `Some` would both be `undefined` (1)</summary>

- fixtures/reject/src/lib.rs:31:15: `Option<std::option::Option<u8>>` is not supported: its `None` and `Some` would both be `undefined`

</details>

<details><summary>a cast from `_` (1)</summary>

- fixtures/reject/src/lib.rs:11:2: a cast from `char`

</details>

<details><summary>a cast to `_` (1)</summary>

- fixtures/reject/src/lib.rs:110:2: a cast to `f64`

</details>

<details><summary>a closure that copies a mutable variable (1)</summary>

- fixtures/reject/src/lib.rs:80:18: a closure that copies a mutable variable

</details>

<details><summary>a mutable reference to `_`, which is no object it can change in place (1)</summary>

- fixtures/reject/src/lib.rs:75:17: a mutable reference to `std::string::String`, which is no object it can change in place

</details>

<details><summary>a precision, as in `{:.2}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:60:3: a precision, as in `{:.2}`, is not supported

</details>

<details><summary>a width or precision from an argument, as in `{:1$}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:67:3: a width or precision from an argument, as in `{:1$}`, is not supported

</details>

<details><summary>an or-pattern with bindings (1)</summary>

- fixtures/reject/src/lib.rs:87:3: an or-pattern with bindings

</details>

<details><summary>arithmetic on `_` (1)</summary>

- fixtures/reject/src/lib.rs:7:2: arithmetic on `f64`

</details>

<details><summary>hex `Debug`, as in `{:x?}`, is not supported (1)</summary>

- fixtures/reject/src/lib.rs:66:3: hex `Debug`, as in `{:x?}`, is not supported

</details>

<details><summary>the `+` flag of a format spec is not supported (1)</summary>

- fixtures/reject/src/lib.rs:61:3: the `+` flag of a format spec is not supported

</details>

<details><summary>the `-` flag of a format spec is not supported (1)</summary>

- fixtures/reject/src/lib.rs:62:3: the `-` flag of a format spec is not supported

</details>

<details><summary>unsupported expression: StaticRef (1)</summary>

- fixtures/reject/src/lib.rs:94:2: unsupported expression: StaticRef

</details>

<details><summary>unsupported literal of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:7:6: unsupported literal of type `f64`

</details>
