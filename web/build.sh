#!/usr/bin/env bash
# Build the web version into dist/:
#   dist/index.html, dist/gl.js, dist/audio.js, dist/net.js, dist/rusty-majong.wasm,
#   dist/assets/{tile,faces,icon}.png, dist/assets/sets.txt, dist/assets/sets/<name>/*.png,
#   dist/assets/palettes.txt, dist/assets/palettes/*.png, dist/assets/sounds/*.wav
# Only the game's PNGs, WAVs and the set list are copied (no .psd or other work files).
# Serve dist/ with any static file server, e.g.:
#   npx --yes serve@14 dist --listen tcp://0.0.0.0:8080
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build --release --target wasm32-unknown-unknown

rm -rf dist
mkdir -p dist/assets
cp web/index.html web/gl.js web/audio.js web/net.js dist/
cp target/wasm32-unknown-unknown/release/rusty-majong.wasm dist/
for f in tile.png faces.png icon.png sets.txt; do
    if [ -f "assets/$f" ]; then cp "assets/$f" dist/assets/; fi
done
# Colour schemes listed in palettes.txt (8x2 PNGs).
if [ -f assets/palettes.txt ]; then
    cp assets/palettes.txt dist/assets/
    mkdir -p dist/assets/palettes
    sed -e 's/#.*//' -e 's/[[:space:]]//g' assets/palettes.txt | grep -E '^[A-Za-z0-9_-]+$' | while read -r pal; do
        if [ -f "assets/palettes/$pal.png" ]; then cp "assets/palettes/$pal.png" dist/assets/palettes/; fi
    done
fi
# Typing click sounds.
if [ -d assets/sounds ]; then
    mkdir -p dist/assets/sounds
    cp assets/sounds/*.wav dist/assets/sounds/
fi
# Tile sets listed in sets.txt (comments/blank lines skipped, same rules as the game).
if [ -f assets/sets.txt ]; then
    sed -e 's/#.*//' -e 's/[[:space:]]//g' assets/sets.txt | grep -E '^[A-Za-z0-9_-]+$' | while read -r set; do
        mkdir -p "dist/assets/sets/$set"
        for f in tile.png faces.png; do
            if [ -f "assets/sets/$set/$f" ]; then cp "assets/sets/$set/$f" "dist/assets/sets/$set/"; fi
        done
    done
fi

echo "built dist/ ($(du -sh dist | cut -f1))"
