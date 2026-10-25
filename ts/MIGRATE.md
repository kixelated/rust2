# Migrating

## Unreleased

- A struct is a plain object typed by an interface, unless it may drop: build one as a
  literal, `{ subscribe, sequence, frameStart: 0 }` rather than
  `new Group({ subscribe, sequence, frameStart: 0 })`.
- Each type has a module of its own beside its Rust module's, holding the type and its
  functions: `coding/decode.Decoder.ts` for `coding::decode::Decoder`. Import it whole,
  `import * as Decoder from "./coding/decode.Decoder.ts"`, and name the type
  `Decoder.Decoder`.
- A function of a type takes `self` first: `dec.slice(4)` is `Decoder.slice(dec, 4)`,
  `group.encode(w, v)` is `Group.encode(group, w, v)`, and `Decoder.new(buf, form)`
  keeps its spelling, from the type's module.
- An enum's functions move from its companion object to its module:
  `DecodeError.fmt$Display(err, f)` keeps its spelling, after
  `import * as DecodeError from "./coding/decode.DecodeError.ts"`.
