# Migrating

## Unreleased

- `Mutex::lock`'s result is an `rt.Result` of the guard, always `Ok`, rather than the
  guard itself.
- `tsc` checks a binding of a function returning `()` against `=> undefined`, not
  `=> void`: type its export `: undefined`.
- `rt.ordMax`, `rt.ordMin`, and `rt.sliceSortByKey` take a `PartialOrd::lt` rather than a
  `partial_cmp`, and `rt.ordClamp` takes `le`, `lt`, and `gt`; `rt.cmpLt` makes one from a
  `partial_cmp`. `rt.sliceSortBy` takes only an `Ord::cmp`-like comparator: sort by an
  `lt` with `rt.sliceSort`.
- A trait object in a function's suffix is named for its trait and the types it binds:
  `show$dynstdfmtDebug` is `show$dyn_Debug`, and `poll$dyn_Future_u64` is a
  `dyn Future<Output = u64>`'s.
- A runtime consumer that takes an iterator by value is told whether it drops rather than
  given `rt.drop`: `iterFold`, `iterForEach`, `iterSum`, `iterProduct`, `iterMax`,
  `iterMin`, `iterMaxByKey`, `iterMinByKey`, and `durationSum` take `true`, and
  `ownedCount` and `ownedLast` take only what drops an item. `iterToArray`,
  `iterToByteVec`, `iterExtend`, `iterExtendBytes`, `dequeExtend`, `hashExtend`,
  `hashExtendKeys`, `btreeExtend`, and `btreeExtendKeys` take an `rt.Take`, `"next"` or
  `"fold"`, for how std's reads it; `iterToArray` takes `"place"` where its `inPlace` was
  `true`.
- `rt.FromUtf8Error` holds its `rt.Utf8Error` as std's does: `e.validUpTo` and
  `e.errorLen` are `e.error.validUpTo` and `e.error.errorLen`.
- The runtime no longer exports `ownedNth`, `ownedNthBack`, `ownedSkip`, `ownedTake`, or
  `ownedStepBy`: `iterNth`, `iterNthBack`, `iterSkip`, `iterTake`, and `iterStepBy` take
  what drops an item last. `rt.Iter`'s `nthItem` and `nthBackItem` are `nth` and
  `nthBack`, which return `rt.DONE` past the end.
- `bool::then` and `then_some` lower from core's MIR into `core/bool.ts`, so the runtime
  no longer exports `boolThen` or `thenSome`.
- Translation builds std from source for `wasm32-unknown-unknown`, so the toolchain needs
  its `rust-src` component, as the flake's has, and the first run compiles std.
- `Option`'s and `Result`'s combinators lower from core's MIR into `core/option.ts` and
  `core/result.ts`, so the runtime no longer exports `optionMap`, `optionMapOr`,
  `optionUnwrapOr`, `optionOkOr`, `resultOk`, `resultErr`, `resultMap`, `resultMapErr`,
  `resultAndThen`, `resultInspectErr`, or `resultUnwrapOr`.
- `--census` starts from the config's roots, as translation does. For the census of every
  item, census with a config that names no roots: `roots = []`.
- A struct is a plain object typed by an interface, unless it may drop: build one as a
  literal, `{ subscribe, sequence, frameStart: 0 }` rather than
  `new Group({ subscribe, sequence, frameStart: 0 })`.
- Each type has a module of its own beside its Rust module's, holding the type and its
  functions: `coding/decode.Decoder.ts` for `coding::decode::Decoder`. Import it whole,
  `import * as Decoder from "./coding/decode.Decoder.ts"`, and name the type
  `Decoder.Decoder`.
- A function of a type takes `self` first: `dec.slice(4)` is `Decoder.slice(dec, 4)`,
  `group.encode(w, v)` is `Group.encode$Encode_Version(group, w, v)`, and
  `Decoder.new(buf, form)` keeps its spelling, from the type's module.
- An enum's functions move from its companion object to its module:
  `DecodeError.fmt$Display(err, f)` keeps its spelling, after
  `import * as DecodeError from "./coding/decode.DecodeError.ts"`.
- A function's suffix no longer depends on what else is reached. A trait impl's function
  always names its trait, with the trait's type arguments, and a generic instance or a
  function of a generic type always names its type arguments: `Log.clone` is
  `Log.clone$Clone`, `Version.encode` is `Version.encode$Encode_Version`, and
  `Wrap.fmt$Pair` is `Wrap.fmt$Debug_Pair`. A closure type in a suffix is named for where
  it is defined, `decode_closure0`.
