#!/usr/bin/env bash
set -euxo pipefail

cargo build --example main --release --target wasm32-unknown-unknown
wasm-bindgen --no-typescript --target web \
    --out-dir ./out/ \
    --out-name "main" \
    ./target/wasm32-unknown-unknown/release/examples/main.wasm
cp ./target/wasm32-unknown-unknown/release/examples/main.wasm ./out/
cp static/index.html ./out/
cp assets/* ./out/
# rsync artifacts to server
rsync -zaP ./out/* do:static/bevy-hover/
