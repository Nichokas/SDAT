#!/usr/bin/env bash
# Run after changing Symbols.kt. Requires woff2_decompress because Google Fonts serves variable subsets as WOFF2.
set -euo pipefail
app="$(cd "$(dirname "$0")/../app/src/main" && pwd)"
names=$(grep -o 'to "[a-z0-9_]*"' "$app/kotlin/com/nichokas/sdat/ui/Symbols.kt" | cut -d'"' -f2 | LC_ALL=C sort -u | paste -sd, -)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
css=$(curl -fsS -A 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/131.0.0.0 Safari/537.36' \
    "https://fonts.googleapis.com/css2?family=Material+Symbols+Rounded:opsz,wght,FILL,GRAD@24,400..700,0..1,0&icon_names=$names")
curl -fsS -o "$tmp/symbols.woff2" "$(grep -o 'https://[^)]*' <<<"$css")"
woff2_decompress "$tmp/symbols.woff2"
cp "$tmp/symbols.ttf" "$app/res/font/material_symbols_rounded.ttf"
