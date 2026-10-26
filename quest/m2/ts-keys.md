# [M] Collection keys and element references

## Goal

rust2-ts translates moq-net's collections whose keys or elements it refuses today,
proven by differential tests against native Rust:

- `Arc<str>` keys: about 12 maps in moq-net's model, ordered in `front.rs` and hashed
  in `origin.rs`.
- Lookups by a borrowed type through `Borrow`, such as `&str` on an `Arc<str>` map
  (7 refused sites: `front.rs`, `requests.rs`, `weak_cache.rs`).
- `Option` in a key, as in `FrontKey`'s derived `Hash` over `Option<Hop>`.
- `&mut` to a `Copy` value held in a collection, such as `*map.entry(k).or_default()
  += 1` in `track/demand.rs`, and `iter_mut` over a fixed array in `group.rs`.

## Plan

Decided with the maintainer on 2026-10-25, from the census at moq 2f146e83 after
[#46](https://github.com/kixelated/rust2/pull/46)'s B+ tree:

- Keep to what moq-net's census refuses. Not in scope, with no use in moq-net:
  - `Hasher::write` of bytes;
  - copy-on-write `clone`;
  - sets without value arrays.
- Drop in collections stays in [ts-drop](/quest/m2/ts-drop.md).
- Keys keep Rust's ordering and equality exactly: `Arc<str>` orders by its text, and
  `Option` orders `None` first.

Public API: the translator's output.
