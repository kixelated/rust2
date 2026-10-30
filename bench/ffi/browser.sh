#!/usr/bin/env bash
# The browser runner: builds moq-ffi for wasm32 in a moq checkout, generates its
# TypeScript over ubrn's wasm2 player, and runs browser-page.ts in headless Chromium.
# Runs inside moq's dev shell, which carries cargo, wasm-bindgen, bun, and Chromium.
# Usage: browser.sh CHECKOUT DIR UBRN, with MOQ_BENCH_DIR and MOQ_BENCH_SECONDS set.
set -euo pipefail

bench=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
checkout=$1 dir=$2 ubrn=$3
mkdir -p "$dir"

(cd "$checkout" && cargo build --locked -q -p moq-ffi --profile wasm-release --target wasm32-unknown-unknown)

# ubrn wants a package.json and its config beside it. Playwright matches the shell's Chromium.
jq -n --arg pkgs "$ubrn/pkgs" --arg playwright "$PLAYWRIGHT_VERSION" '{
    name: "moq-ffi-bench",
    private: true,
    repository: {type: "git", url: "https://github.com/moq-dev/rust2"},
    dependencies: {
        "@ubjs/core": "file:\($pkgs)/ubjs-core.tgz",
        "@ubjs/wasm": "file:\($pkgs)/ubjs-wasm.tgz",
        playwright: $playwright
    }
}' >"$dir/package.json"
cat >"$dir/ubrn.config.yaml" <<EOF
rust:
  directory: $checkout
  manifestPath: rs/moq-ffi/Cargo.toml
wasm2:
  ts: $dir/dist
EOF
cp "$bench/browser-page.ts" "$dir/page.ts"
cp "$bench/browser.ts" "$dir/driver.ts"

cd "$dir"
bun install --silent
rm -rf dist bundle
CARGO_TARGET_DIR=$checkout/target "$ubrn/bin/uniffi-bindgen-react-native" build wasm2 \
    --config ubrn.config.yaml --profile wasm-release --no-wasm-build >/dev/null
bun build page.ts --outfile bundle/page.js --target browser >/dev/null
cp dist/moq_ffi.wasm bundle/
bun driver.ts bundle
