# rust2

Rust to other places.

- **TypeScript**: an own rustc driver that translates a documented Rust subset to readable TypeScript, without WASM.
- **Any language**: a hard fork of [UniFFI](https://github.com/mozilla/uniffi-rs) with every generator [moq](https://github.com/moq-dev/moq) uses.
- **Faster builds**: a reproducible build-speed benchmark, then a pure-Rust subset (no build scripts, no C linking) that builds faster.

Everything here is early; the [quest tree](quest/README.md) holds the plan.

## Development

```bash
nix develop   # or direnv allow
just check
```

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option, unless a directory's own license says otherwise (the UniFFI fork keeps MPL-2.0).
