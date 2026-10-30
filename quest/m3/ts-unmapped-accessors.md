# [M] std accessors with no mapping

## Goal

`VecDeque::swap_remove_front` and `swap_remove_back`, `HashSet::replace`,
`BTreeSet::get`, `take`, and `replace`, `Vec::pop_if`, and `Arc::into_inner` and
`Rc::into_inner` translate for every element type, including one whose `None` is
`rt.None`, proven by differential tests against native Rust that cover drop order.

## Plan

Reproduced on `dadd60a9` over `u32` elements: each is refused at its span as an
unsupported call, and `Vec::pop_if` of `u32`s as "with a mutable reference to a value
JS copies, which its mapping would copy" (of a `Vec<Vec<u32>>`, as an unsupported call).
Of an element that can be `undefined`, each but `pop_if` is refused instead as "`_` of
`_`, whose `None` is `rt.None`".

- Each that returns an `Option` of an element takes the `None` last through
  `none_last` in `ts/src/lower/library.rs`, as #292's accessors do, and is tested over
  `Option<u8>` and `()` elements beside the others in the lang fixture's
  `nested::elements`.
- `pop_if` hands its closure the last element's `&mut`: a place where JS copies the
  element, as `last_mut` lends it.
- `HashSet::replace` and `BTreeSet::replace` return the key they held and keep the new
  one; nothing drops in the call.
- `into_inner` returns the value only from the last strong reference, and otherwise
  drops this one: `rt.Arc`'s counts in `ts/runtime/rt/task.ts` decide it, and a `Weak`
  left over no longer upgrades.

The reject fixture's `inner_option` and `inner_unit` use `Arc::into_inner` and
`Rc::into_inner` as examples of that refusal, as does the `ts/README.md` list of what
fails translation; replace them with something still refused.

Found by [#292](https://github.com/moq-dev/rust2/pull/292).

Public API: the translator's output.
