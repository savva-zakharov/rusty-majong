#!/usr/bin/env bash
# Build the web version and copy the game into another site's folder, e.g.
#   bash web/export.sh ../pano-gallery/mahjong
# Copies gl.js, rusty-majong.wasm and assets/ (the game PNGs and sets.txt).
# The destination's index.html is left alone: the host site owns its page.
# If the destination has no index.html yet, web/index.html is copied as a start.
set -euo pipefail
cd "$(dirname "$0")/.."

dest="${1:?usage: bash web/export.sh <destination folder>}"

bash web/build.sh

mkdir -p "$dest"
rm -rf "$dest/assets"
cp dist/gl.js dist/rusty-majong.wasm "$dest/"
cp -r dist/assets "$dest/assets"
if [ ! -f "$dest/index.html" ]; then
    cp dist/index.html "$dest/index.html"
    echo "no index.html in $dest: copied the standalone page as a starting point"
fi

echo "exported game to $dest"
