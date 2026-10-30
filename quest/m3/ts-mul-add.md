# [M] mul_add rounded once

## Goal

[rust2-ts](/ts/README.md) translates `f64::mul_add` and `f32::mul_add`, rounding
`a * b + c` once as std's fused multiply-add does. Differential tests against native
wasm32 prove it, including operands where rounding the product and then the sum would
give another result.

## Plan

Left by [#296](https://github.com/moq-dev/rust2/pull/296), which refused `f32::mul_add` as
`f64`'s already was. Planned without a moq consumer, under the maintainer's 2026-10-27
decision that rust2 aims to support all of std.

Reproduction at `cecbf0d7`: the reject fixture's `fma` root, and a scratch crate's
`a.mul_add(b, c)`:

```text
`f64::mul_add`, which rounds once, where JS would round twice at src/lib.rs:10:4
```

`f32::mul_add` fails the same way once #296 lands.

- First confirm what native wasm32 runs. wasm has no fused multiply-add, so the call
  should be compiler-builtins' software `fma` and `fmaf`, which round once.
- `f32`: a double holds `a * b` exactly (48 bits). Add `c` with TwoSum to get the rounded
  sum and its exact error. If the error is nonzero and the sum's last bit is even, step
  the sum one double toward the error; that is rounding to odd. Then `Math.fround` rounds
  once, since 53 >= 24 + 2. Infinities and NaN take the plain `Math.fround(a * b + c)`.
- `f64`: there is no wider type. Use an error-free product (Dekker's split, or a `BigInt`
  of both significands) and TwoSum, with an exact `BigInt` fallback wherever the fast path
  can't decide. That covers a product that overflows while the sum does not, and a
  subnormal result. Correctness first, since nothing in moq calls it, then report its
  cost per call against `a * b + c`.
- Move the reject fixture's `fma` root to another construct that is still refused, or
  drop it, and update the README's `mul_add` refusal note.

Public API: the translator's output.
