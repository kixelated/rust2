# [L] Byte slices passed and returned as parts

## Goal

A translated function outside the API boundary takes each byte slice parameter as its parts
(`data`, `start`, `length`), and one that returns a `Result` of byte slices and scalars, such
as `Result<(u64, &[u8]), E>`, returns the error or `undefined`, with its payload in a
module-level out slot. Chrome's varint and frame decode on `just ts bench` are faster than
`main`, no workload is meaningfully slower in any engine, and every differential test passes.

## Plan

Decided with the maintainer on 2026-10-28: this absorbs passing slices as parts, which
[#160](https://github.com/kixelated/rust2/pull/160) implemented. Alone, parts gain nothing in
Chrome, since V8 already removes a slice object that doesn't escape once it inlines, and
Node's varint decode is 11-15% slower. They land only with the return ABI ("e1").
[#138](https://github.com/kixelated/rust2/pull/138)'s per-function scratch measured slower
than `main`: copying between scratches when one function returns another's result cost the
most. One out slot per payload type, which a forwarded result already fills, avoids that.

- Only functions outside the API boundary: not roots or closures. A function used as a
  value is wrapped in an arrow that rebuilds the `Result`.
- The callee writes its slot at the JS `return`, after its MIR drops ran; the caller reads
  it at once, before any other call.
- Start the slot's number fields as `undefined`, not `0`, so V8 keeps them tagged. A field
  that starts as a small integer turns into a boxed double once a value past 2^31 is
  stored, and a function too big for V8 to inline (over 460 bytes of bytecode, as
  `read_leading_ones` is) then boxes every write and the caller unboxes every read. This
  is why the first hand edit was slower than `main` on leading-ones varints.
- Measured by hand edits of the codec's translation against `main`, Chrome first (2026-10-28,
  on a busy host, so confirm on a quiet one): with the slot's fields tagged, Chrome's varint
  and frame decode were each about 1.25x faster and group decode 1.06x, Node 1.33x and
  1.40x, Firefox 1.21x and 1.10x, Bun even and 1.12x. Leading-ones varints of 8 bytes
  measured 0.84x, within that run's noise.
- Open: with parameters as parts, Chrome's `time/` convert workloads ran 7-12% slower than
  `main` in every run on 2026-10-28, with the same instruction count and the same inlining.
  Find out why before landing.

Public API: the translator's output for byte slices.
