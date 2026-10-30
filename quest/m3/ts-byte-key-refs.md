# [S] References to byte-array keys a collection yields

## Goal

rust2-ts translates what lends out a collection's `[u8; N]` key, such as `keys`, `iter`,
`get_key_value`, `Entry::key`, and `HashSet::get` and `iter`, each reference an
`rt.ByteSlice` of the key's bytes, as a `&[u8; N]` is elsewhere. Proven by differential
tests against native Rust.

## Plan

Each refuses with a span today, as "`HashMap::keys` of a reference to an array of bytes it
is generic over" (`bytes_across` in `src/lower/mir/call.rs`): the collection holds the key
as a typed array, which its mapping would hand back where the caller reads a slice.
Lookups by such a reference already translate, since they only read the key.

Public API: the translator's output.
