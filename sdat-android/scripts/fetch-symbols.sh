#!/usr/bin/env bash
# Fetches the Material Symbols Rounded fonts, subset by Google Fonts to the ligatures in Symbols.kt.
# Run it after adding an icon to Symbols.kt.
set -euo pipefail
app="$(cd "$(dirname "$0")/../app/src/main" && pwd)"
names=$(grep -o 'to "[a-z0-9_]*"' "$app/kotlin/com/nichokas/sdat/ui/Symbols.kt" | cut -d'"' -f2 | LC_ALL=C sort -u | paste -sd, -)
fetch() {
    local css
    css=$(curl -sSf "https://fonts.googleapis.com/css2?family=Material+Symbols+Rounded:opsz,wght,FILL,GRAD@24,400,$1,0&icon_names=$names")
    curl -sSf -o "$app/res/font/$2" "$(grep -o 'https://[^)]*' <<<"$css")"
}
fetch 0 material_symbols_rounded.ttf
fetch 1 material_symbols_rounded_fill.ttf
