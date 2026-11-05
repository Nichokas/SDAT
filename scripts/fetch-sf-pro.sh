#!/usr/bin/env bash
# Puts SF Pro from Apple's download into the gitignored sdat-android/local-assets/; never commit the output.
# Needs curl, 7z, unzip, javap and uvx.
set -euo pipefail
android="$(cd "$(dirname "$0")/../sdat-android" && pwd)"
out="$android/local-assets/SF-Pro.ttf"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cd "$work"

curl -sSfL -o sf.dmg https://devimages-cdn.apple.com/design/resources/download/SF-Pro.dmg
7z e -y sf.dmg SFProFonts.pkg >/dev/null
7z e -y SFProFonts.pkg 'Payload~' >/dev/null
7z e -y 'Payload~' ./Library/Fonts/SF-Pro.ttf >/dev/null

# The full font is 21 MB, almost all SF Symbols. Keep the text scripts plus the symbols the app draws:
# its own (Symbols.kt) and Glasmio's catalog, which renders the tab bar and controls from the same typeface.
glasmio="$(sed -nE 's/^glasmio = "(.*)"/\1/p' "$android/gradle/libs.versions.toml")"
curl -sSfL -o icons.aar "https://repo1.maven.org/maven2/io/github/neoruaa/glasmio/glasmio-icons/$glasmio/glasmio-icons-$glasmio.aar"
unzip -q icons.aar classes.jar
unzip -q classes.jar io/github/neoruaa/glasmio/icons/SfSymbolCatalog.class
{
  echo U+0000-052F U+1E00-1FFF U+2000-2BFF
  grep -oE '0x10[0-9A-Fa-f]{4}' "$android/app/src/main/kotlin/com/nichokas/sdat/ui/Symbols.kt" | sed 's/^0x/U+/'
  javap -c -constants io/github/neoruaa/glasmio/icons/SfSymbolCatalog.class \
    | grep -oE 'int [0-9]+' | awk '$2 >= 1048576 && $2 <= 1114111 { printf "U+%X\n", $2 }'
} > unicodes.txt
[ "$(grep -c '^U+10' unicodes.txt)" -gt 100 ] || { echo "too few SF Symbols found; did Glasmio's catalog move?" >&2; exit 1; }

mkdir -p "$(dirname "$out")"
uvx -q --from fonttools pyftsubset SF-Pro.ttf --unicodes-file=unicodes.txt --layout-features='*' --output-file="$out" 2>/dev/null
echo "ok: $(du -h "$out")"
