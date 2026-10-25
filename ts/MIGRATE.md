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
  `group.encode(w, v)` is `Group.encode$Encode_Version(group, w, v)`, and
  `Decoder.new(buf, form)` keeps its spelling, from the type's module.
- An enum's functions move from its companion object to its module:
  `DecodeError.fmt$Display(err, f)` keeps its spelling, after
  `import * as DecodeError from "./coding/decode.DecodeError.ts"`.
- The runtime's helpers are functions of `rt.ts`, named for what they act on, which they
  take first: `rt.u64.of(x)` is `rt.u64Of(x)`, `rt.u64.add(a, b)` is `rt.u64Add(a, b)`,
  `rt.fmt.toString(f)` is `rt.fmtToString(f)`, `rt.fmt.format(args)` is
  `rt.fmtFormat(args)`, and `rt.ByteVec.from(bytes)` is `rt.byteVecFrom(bytes)`.
  `rt.u64`, `rt.fmt` and the other namespaces are gone, with the integer types'
  `MIN`, `MAX` and `BITS`.
- An `rt.ByteVec`'s methods are functions: `v.view()` is `rt.byteVecView(v)`, and
  `v.push(b)` is `rt.byteVecPush(v, b)`.
- An `rt.Iter`'s adaptors and consumers are functions: `it.map(f).count()` is
  `rt.iterCount(rt.iterMap(it, f))`. `next`, `nextBack` and `len` stay methods, and
  `for ... of` still iterates one.
- An `rt.Formatter` is a plain object, `{ out, spec: {} }` rather than
  `new rt.Formatter(out)`, whose methods are functions: `f.pad(s)` is
  `rt.fmtPad(f, s)`, and `f.debugStruct(name)` is `rt.fmtDebugStruct(f, name)`.
- `rt.Arc`'s and `rt.Weak`'s methods are functions: `arc.clone()` is
  `rt.arcClone(arc)`, and `weak.upgrade()` is `rt.weakUpgrade(weak)`.
- A function's suffix no longer depends on what else is reached. A trait impl's function
  always names its trait, with the trait's type arguments, and a generic instance or a
  function of a generic type always names its type arguments: `Log.clone` is
  `Log.clone$Clone`, `Version.encode` is `Version.encode$Encode_Version`, and
  `Wrap.fmt$Pair` is `Wrap.fmt$Debug_Pair`. A closure type in a suffix is named for where
  it is defined, `decode_closure0`.
