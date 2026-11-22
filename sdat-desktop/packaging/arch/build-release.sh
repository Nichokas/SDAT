#!/usr/bin/env bash
set -euo pipefail

tag=${1:?Usage: build-release.sh desktop-vX.Y.Z SOURCE_DIR OUTPUT_DIR}
source_dir=${2:?Missing source directory}
output_dir=${3:?Missing output directory}
if [[ ! $tag =~ ^desktop-v([0-9]+\.[0-9]+\.[0-9]+)$ ]]; then
    echo "Invalid desktop tag: $tag" >&2
    exit 1
fi
export TONEARM_VERSION=${BASH_REMATCH[1]}
export TONEARM_COMMIT="v$TONEARM_VERSION"

mkdir -p "$output_dir"
output_dir=$(realpath "$output_dir")
cd "$source_dir"
target_dir=$(cargo metadata --locked --no-deps --format-version 1 | jq -er .target_directory)
cargo build --release --locked -p sdat-desktop

stage=$(mktemp -d)
trap 'rm -rf -- "$stage"' EXIT
share="$stage/usr/share"
appid=com.nichokas.SDAT.Desktop
desktop=sdat-desktop
icons="$desktop/internal/icons/hicolor"
settings="$desktop/internal/settings"

install -Dm755 "$target_dir/release/sdat-desktop" "$stage/usr/bin/sdat-desktop"
install -Dm644 "$desktop/build/$appid.desktop" -t "$share/applications"
install -Dm644 "$desktop/build/$appid.metainfo.xml" -t "$share/metainfo"
mkdir -p "$share/icons/hicolor/256x256/apps"
magick sdat-mobile/Resources/AppIcon.png -resize 256x256 -strip \
    "$share/icons/hicolor/256x256/apps/$appid.png"
install -Dm644 "$icons/symbolic/apps/$appid-symbolic.svg" -t "$share/icons/hicolor/symbolic/apps"
install -Dm644 "$settings/$appid.gschema.xml" "$settings/dev.dergs.Tonearm.gschema.xml" -t "$share/glib-2.0/schemas"
install -Dm644 "$desktop/LICENSE" -t "$share/licenses/sdat-bin"

desktop-file-validate "$share/applications/$appid.desktop"
glib-compile-schemas --strict --dry-run "$share/glib-2.0/schemas"
tar --zstd --owner=0 --group=0 -C "$stage" -cf "$output_dir/sdat-desktop-$TONEARM_VERSION-arch-$(uname -m).tar.zst" usr
