#!/usr/bin/env bash
# Build the web version into dist/:
#   dist/index.html, dist/gl.js, dist/rusty-majong.wasm, dist/assets/*.png
# Serve dist/ with any static file server, e.g.:
#   python -m http.server 8080 --directory dist
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build --release --target wasm32-unknown-unknown

rm -rf dist
mkdir -p dist/assets
cp web/index.html web/gl.js dist/
cp target/wasm32-unknown-unknown/release/rusty-majong.wasm dist/
for f in tile.png faces.png; do
    if [ -f "assets/$f" ]; then cp "assets/$f" dist/assets/; fi
done

echo "built dist/ ($(du -sh dist | cut -f1))"
