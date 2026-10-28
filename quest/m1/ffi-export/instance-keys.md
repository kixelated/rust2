# [M] Instances the config can't name yet

## Goal

The exporter's `instances` key names the instances moq-ffi still wraps by hand: a `&T`
argument (`with_publisher` at `&origin::Producer`, as moq-ffi passes it),
`impl IntoIterator<Item = X>` exported as a `Vec` of `X`, `serde_json::Value` as the `T` of
`catalog::Producer`'s `json_snapshot` and `json_stream`, and `DEFAULT_NAME` on
`hang::Catalog<()>`.

## Plan

Left out of [#126](https://github.com/kixelated/rust2/pull/126), which exports generic
functions at config-named instances; decided with the maintainer on 2026-10-27 to plan
them all.

- `IntoIterator` as a `Vec` needs `Vec` of records to map first.
- `serde_json::Value` crosses the C ABI as something every generator can read; decide
  its shape (JSON text is the obvious one) when the quest starts.
- Report moq-ffi's coverage and the census before and after.

Public API: the exporter's config and the C names it emits.
