# [S] The ASCII methods of u8 and char

## Goal

[rust2-ts](/ts/README.md) translates the inherent ASCII methods of `u8` and `char` with
std's semantics, proven by differential tests against native Rust over every byte and the
chars at each edge of ASCII.

## Plan

Found enumerating integers' inherent methods: each of `u8`'s `is_ascii`, its
`is_ascii_*` tests, `to_ascii_uppercase`, `to_ascii_lowercase`, `eq_ignore_ascii_case`,
`make_ascii_uppercase`, `make_ascii_lowercase`, and `escape_ascii` refuses as "unsupported
call", with its span, and so does each of `char`'s but `is_ascii_digit`. They are a
table of predicates and case maps over one value, apart from the two that change their
receiver in place and `escape_ascii`, whose iterator `Display` also writes. No consumer
calls them yet; they are planned here as part of std.

Public API: the translator's output.
