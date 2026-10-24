# Census of `reject-fixture`

- Refusals: 26
- Function instances: 18
- Generic functions no instance reached, lowered over their type parameters: 1

## By construct

| Count | Construct | First |
| ---: | --- | --- |
| 5 | unsupported type `_` | fixtures/reject/src/lib.rs:6:1 |
| 4 | unsupported call to `_` | fixtures/reject/src/lib.rs:36:4 |
| 3 | replacing the value behind a reference to `_` | fixtures/reject/src/lib.rs:84:17 |
| 2 | a `_` binding: a JS value of `_` is a copy, so it can't change | fixtures/reject/src/lib.rs:19:13 |
| 1 | `_` implements `Drop`, which is not supported yet | fixtures/reject/src/lib.rs:43:1 |
| 1 | `_` is async, which is not supported yet | fixtures/reject/src/lib.rs:53:1 |
| 1 | `_` is not supported: its `None` and `Some` would both be `undefined` | fixtures/reject/src/lib.rs:31:15 |
| 1 | `_`: a JS value of `_` is a copy, so a reference can't change it | fixtures/reject/src/lib.rs:19:13 |
| 1 | a cast from `_` | fixtures/reject/src/lib.rs:11:2 |
| 1 | a cast to `_` | fixtures/reject/src/lib.rs:95:2 |
| 1 | a closure that copies a mutable variable | fixtures/reject/src/lib.rs:65:18 |
| 1 | a panic message that formats values is not supported | fixtures/reject/src/lib.rs:59:3 |
| 1 | an or-pattern with bindings | fixtures/reject/src/lib.rs:72:3 |
| 1 | arithmetic on `_` | fixtures/reject/src/lib.rs:7:2 |
| 1 | unsupported expression: StaticRef | fixtures/reject/src/lib.rs:79:2 |
| 1 | unsupported literal of type `_` | fixtures/reject/src/lib.rs:7:6 |

## By API

The type or module each refused type or path belongs to.

| Count | API | First |
| ---: | --- | --- |
| 6 | `f64` | fixtures/reject/src/lib.rs:6:1 |
| 3 | `Meters` | fixtures/reject/src/lib.rs:84:17 |
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
| 3 | replacing the value behind a reference to `_` | `Meters` | fixtures/reject/src/lib.rs:84:17 |
| 3 | unsupported type `_` | `f64` | fixtures/reject/src/lib.rs:6:1 |
| 2 | a `_` binding: a JS value of `_` is a copy, so it can't change | `&mut u32`, `u32` | fixtures/reject/src/lib.rs:19:13 |
| 1 | `_` implements `Drop`, which is not supported yet | `Guard` | fixtures/reject/src/lib.rs:43:1 |
| 1 | `_` is async, which is not supported yet | `later` | fixtures/reject/src/lib.rs:53:1 |
| 1 | `_` is not supported: its `None` and `Some` would both be `undefined` | `Option` | fixtures/reject/src/lib.rs:31:15 |
| 1 | `_`: a JS value of `_` is a copy, so a reference can't change it | `&mut u32`, `u32` | fixtures/reject/src/lib.rs:19:13 |
| 1 | a cast from `_` | `char` | fixtures/reject/src/lib.rs:11:2 |
| 1 | a cast to `_` | `f64` | fixtures/reject/src/lib.rs:95:2 |
| 1 | arithmetic on `_` | `f64` | fixtures/reject/src/lib.rs:7:2 |
| 1 | unsupported call to `_` | `<std::iter::Filter as std::iter::Iterator>::count` | fixtures/reject/src/lib.rs:40:31 |
| 1 | unsupported call to `_` | `<std::slice::Iter as std::iter::Iterator>::filter` | fixtures/reject/src/lib.rs:40:11 |
| 1 | unsupported call to `_` | `std::slice::iter` | fixtures/reject/src/lib.rs:40:4 |
| 1 | unsupported call to `_` | `std::slice::sort` | fixtures/reject/src/lib.rs:36:4 |
| 1 | unsupported literal of type `_` | `f64` | fixtures/reject/src/lib.rs:7:6 |
| 1 | unsupported type `_` | `char` | fixtures/reject/src/lib.rs:10:18 |
| 1 | unsupported type `_` | `dyn std::ops::Fn(u8) -> u8` | fixtures/reject/src/lib.rs:14:16 |

## Spans

<details><summary>unsupported type `_` (5)</summary>

- fixtures/reject/src/lib.rs:6:1: unsupported type `f64`
- fixtures/reject/src/lib.rs:6:14: unsupported type `f64`
- fixtures/reject/src/lib.rs:10:18: unsupported type `char`
- fixtures/reject/src/lib.rs:14:16: unsupported type `dyn std::ops::Fn(u8) -> u8`
- fixtures/reject/src/lib.rs:93:1: unsupported type `f64`

</details>

<details><summary>unsupported call to `_` (4)</summary>

- fixtures/reject/src/lib.rs:36:4: unsupported call to `std::slice::<impl [u8]>::sort`
- fixtures/reject/src/lib.rs:40:4: unsupported call to `core::slice::<impl [u8]>::iter`
- fixtures/reject/src/lib.rs:40:11: unsupported call to `<std::slice::Iter<'_, u8> as std::iter::Iterator>::filter::<{closure@fixtures/reject/src/lib.rs:40:18: 40:21}>`
- fixtures/reject/src/lib.rs:40:31: unsupported call to `<std::iter::Filter<std::slice::Iter<'_, u8>, {closure@fixtures/reject/src/lib.rs:40:18: 40:21}> as std::iter::Iterator>::count`

</details>

<details><summary>replacing the value behind a reference to `_` (3)</summary>

- fixtures/reject/src/lib.rs:84:17: replacing the value behind a reference to `Meters`
- fixtures/reject/src/lib.rs:84:20: replacing the value behind a reference to `Meters`
- fixtures/reject/src/lib.rs:88:25: replacing the value behind a reference to `Meters`

</details>

<details><summary>a `_` binding: a JS value of `_` is a copy, so it can't change (2)</summary>

- fixtures/reject/src/lib.rs:19:13: a `&mut u32` binding: a JS value of `u32` is a copy, so it can't change
- fixtures/reject/src/lib.rs:25:6: a `&mut u32` binding: a JS value of `u32` is a copy, so it can't change

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

<details><summary>`_`: a JS value of `_` is a copy, so a reference can't change it (1)</summary>

- fixtures/reject/src/lib.rs:19:13: `&mut u32`: a JS value of `u32` is a copy, so a reference can't change it

</details>

<details><summary>a cast from `_` (1)</summary>

- fixtures/reject/src/lib.rs:11:2: a cast from `char`

</details>

<details><summary>a cast to `_` (1)</summary>

- fixtures/reject/src/lib.rs:95:2: a cast to `f64`

</details>

<details><summary>a closure that copies a mutable variable (1)</summary>

- fixtures/reject/src/lib.rs:65:18: a closure that copies a mutable variable

</details>

<details><summary>a panic message that formats values is not supported (1)</summary>

- fixtures/reject/src/lib.rs:59:3: a panic message that formats values is not supported

</details>

<details><summary>an or-pattern with bindings (1)</summary>

- fixtures/reject/src/lib.rs:72:3: an or-pattern with bindings

</details>

<details><summary>arithmetic on `_` (1)</summary>

- fixtures/reject/src/lib.rs:7:2: arithmetic on `f64`

</details>

<details><summary>unsupported expression: StaticRef (1)</summary>

- fixtures/reject/src/lib.rs:79:2: unsupported expression: StaticRef

</details>

<details><summary>unsupported literal of type `_` (1)</summary>

- fixtures/reject/src/lib.rs:7:6: unsupported literal of type `f64`

</details>
