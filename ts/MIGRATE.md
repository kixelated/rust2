# Migrating

## Unreleased

- A struct is a plain object typed by an interface, unless it may drop: build one as a
  literal, `{ subscribe, sequence, frameStart: 0 }` rather than
  `new Group({ subscribe, sequence, frameStart: 0 })`.
- A function of a type is a module-level function named after both, in the type's
  module, taking `self` first: `Decoder.new(buf, form)` is `decoderNew(buf, form)`,
  `dec.slice(4)` is `decoderSlice(dec, 4)`, and `group.encode(w, v)` is
  `groupEncode(group, w, v)`.
- An enum has no companion object: `DecodeError.fmt$Display(err, f)` is
  `decodeErrorFmt$Display(err, f)`.
