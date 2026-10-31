# [S] A &mut str to any String

## Goal

[rust2-ts](/ts/README.md) translates a `&mut str` to a `String` wherever the `String` is,
and to a part of one, so `make_ascii_uppercase` and `make_ascii_lowercase` reach every place
std's do. Today they write a local's, a field's, or one behind a `&mut String`; a `String`
a tuple, an array, or a `Vec` holds refuses at `DerefMut` as "no path", and `&mut s[a..b]`
and `String::as_mut_str` refuse as a `&mut` their mapping would copy. Proven by
differential tests against native Rust.

## Plan

Found translating the ASCII methods. A JS string can't change in place, so a `&mut str` is
an `rt.Place<string>` of the whole `String`; a part of one needs the place and its UTF-8
byte range, written back as a new string. No consumer calls them yet; planned as part of std.

Public API: the translator's output.
