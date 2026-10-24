# UniFFI fork

A hard fork of [UniFFI](https://github.com/mozilla/uniffi-rs) 0.32.2 with every generator
[moq](https://github.com/moq-dev/moq) uses: Kotlin, Swift, Python, Go, Dart, C++, and C.
It never merges upstream; fixes are cherry-picked by hand.

## Use

Nothing is published to crates.io yet, so depend on a revision of this repository:

```toml
[dependencies]
uniffi = { git = "https://github.com/kixelated/rust2", rev = "<rev>" }
```

Every crate keeps its upstream name (`uniffi`, `uniffi_core`, `uniffi_bindgen`, ...), so code
written for UniFFI 0.32 compiles as is. Do not link upstream UniFFI into the same library.

The generators read a cdylib built against this runtime:

| Binary | Languages | Package |
| --- | --- | --- |
| `uniffi-bindgen` | Kotlin, Swift, Python | `uniffi` with `--features cli` |
| `uniffi-bindgen-swift` | Swift modules and xcframeworks | `uniffi` with `--features cli` |
| `uniffi-bindgen-go` | Go | `uniffi-bindgen-go` |
| `uniffi_bindgen_dart` | Dart | `uniffi-dart` with `--features binary` |
| `uniffi-bindgen-cpp` | C++, and C with `--lang c` | `uniffi-bindgen-cpp` |

The flake builds them all as `packages.uniffi`, and each one alone as `uniffi-bindgen`,
`uniffi-bindgen-go`, `uniffi-bindgen-dart`, and `uniffi-bindgen-cpp`. Without Nix:
`cargo install --git https://github.com/kixelated/rust2 --rev <rev> uniffi-bindgen-go`.

Upstream's [user guide](https://mozilla.github.io/uniffi-rs/0.32/) applies, minus Ruby. The
generators document their options in [go/docs](go/docs/CONFIGURATION.md),
[cpp/README.md](cpp/README.md), and [cpp/docs](cpp/docs/CONFIGURATION.md).

## Contract

The FFI contract is still upstream 0.32's (contract version 30), which is what keeps moq's
generated bindings byte-identical. The first change to the contract gives it its own
namespace, so a library and bindings from different forks fail at load time instead of
mixing silently.

## Imports

| Directory | Upstream | Imported revision | License |
| --- | --- | --- | --- |
| `uniffi*/`, `examples/`, `fixtures/`, `bindgen-tests/` | [mozilla/uniffi-rs](https://github.com/mozilla/uniffi-rs) | [kixelated/uniffi-rs@88d835d](https://github.com/kixelated/uniffi-rs/commit/88d835d66abc6357b0bcd290b4c2adb35096cd10) (v0.32.2 plus one fix) | [MPL-2.0](LICENSE) |
| `go/` | [NordSecurity/uniffi-bindgen-go](https://github.com/NordSecurity/uniffi-bindgen-go) | [kixelated/uniffi-bindgen-go@v0.10.0-kixelated.1+v0.32.0](https://github.com/kixelated/uniffi-bindgen-go/tree/v0.10.0-kixelated.1%2Bv0.32.0) | [MPL-2.0](go/LICENSE) |
| `dart/` | [acterglobal/uniffi-dart](https://github.com/acterglobal/uniffi-dart) | [kixelated/uniffi-dart@v0.3.1-kixelated.6+v0.32.0](https://github.com/kixelated/uniffi-dart/tree/v0.3.1-kixelated.6%2Bv0.32.0) | [MIT](dart/LICENSE) |
| `cpp/` | [NordSecurity/uniffi-bindgen-cpp](https://github.com/NordSecurity/uniffi-bindgen-cpp) | [kixelated/uniffi-bindgen-cpp@v0.11.0-kixelated.5+v0.32.2](https://github.com/kixelated/uniffi-bindgen-cpp/tree/v0.11.0-kixelated.5%2Bv0.32.2) | [MPL-2.0](cpp/LICENSE) |

Each directory keeps its upstream layout, so an upstream commit applies with
`git am --directory=uniffi/<dir>` (no `<dir>` for uniffi-rs). Their crates build in the
repository's root workspace, in place of each upstream's own.

Left out: the Ruby generator and its tests, weedle2 (used from crates.io), upstream's docs,
CI, and Docker setup, the Android, iOS, and fxa-client examples, the Dart fixtures upstream
does not build, and the C++ scaffolding tests, which need the Go and C# generators as
submodules.

## Carried changes

Offering a change upstream needs maintainer approval each time, so most have no upstream PR.

uniffi-rs:

- Kotlin refuses a destroyed object passed as an argument (kixelated/uniffi-rs@88d835d).
- Ruby is removed.
- `toml` is pinned to major 1. Upstream's `>=0.9, <2` spans two majors, so a generator that
  resolved the other one got a second `toml::Value` that no longer matched the trait.
- Clippy 1.98 fixes, including an allow on the UDL metadata const, which it flags in every UDL
  consumer.
- Test expectations that hold a path relative to the workspace root (`file!()` output and
  trybuild snapshots) gain the `uniffi/` prefix.
- Scaffolding is written to a temporary file and renamed into place. Two trybuild cases
  expanding `generate_and_include_scaffolding!` on one UDL write it concurrently, and the
  in-place truncate let one include a partial file.

uniffi-bindgen-go, on top of upstream v0.7.1+v0.31.0:

- The UniFFI 0.32 port ([NordSecurity/uniffi-bindgen-go#98](https://github.com/NordSecurity/uniffi-bindgen-go/pull/98)).
- Generated async calls take a `context.Context` and cancel the Rust future.
- An exported `Display` on an enum error renders as its `Error()`.
- `Types.go` renders its object, enum, error, record, callback interface, and custom type
  templates in their own functions, with byte-identical output. Inlined into one, the borrow
  checker took about 16 GB to check it, more than a CI runner has.

uniffi-dart, on top of upstream `main` at acterglobal/uniffi-dart@72d26c1:

- The UniFFI 0.32 port and the library-mode CLI, by Sasha Denisov.
- RustBuffer and async result leak fixes, `liftAndFree` type arguments, non-null optionals
  lowered through `toRustBuffer`, an exported `Display` as an enum's `toString()`, and
  disposed objects refused as arguments.
- Record errors keep their converter name, and futures cancel per operation (moq's
  `nix/uniffi-dart-record-error.patch` and `nix/uniffi-dart-cancellation.patch`).

uniffi-bindgen-cpp, on top of upstream `main` at NordSecurity/uniffi-bindgen-cpp@20bef75:

- LiveKit's async support (livekit/uniffi-bindgen-cpp, by Alan George), ported to UniFFI 0.32.
- `error_style = "expected"`, `strip_type_prefix`, exported `Display` and `Debug` on enums,
  newline-terminated files, released async results, and the C backend (`--lang c`).
- The CMake test suite finds the fixtures in the root workspace's target directory.
