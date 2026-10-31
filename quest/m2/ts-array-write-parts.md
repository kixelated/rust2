# [S] A pointer into an element's part across a whole write of its array

## Goal

[rust2-ts](/ts/README.md) refuses, with its span, a raw pointer into a part of an
array's element, such as a field of a struct it holds, where the crate may write the
whole array while that pointer may still be read. A translated program never silently
reads the element object the write took out of the array.

## Plan

Found while landing fresh writes through std's iterators: a whole write of a `[C; N]`
through a pointer records only `[C; N]` as a type the crate may put another object in
place of, and `out_of_step` in `ts/src/lower/mir/place.rs` asks about `C`, the type the
part is in. `rt.ptrWrite` assigns the array's items, so each element gets another
object. Translation accepts this, which suggests a silent miscompile; reproduce it with
a differential test before fixing:

```rust
impl Clone for Copied {
	fn clone(&self) -> Self {
		KEPT.with(|k| k.set(&raw const self.span)); // into `slots`' element
		Copied { span: self.span }
	}
}
let mut slots = MaybeUninit::<[Copied; 2]>::uninit();
slots.write([Copied { span: (n, n) }; 2]);
let _: Vec<Copied> = unsafe { slots.assume_init_ref() }.iter().cloned().collect();
slots.write([Copied { span: (0, 0) }; 2]); // `0` natively, `n` in JS
```

`stored` in `ts/src/lower/copy.rs` could record the element types a whole array write
reaches one object deep, or `out_of_step` could ask about the arrays that hold `C`.
