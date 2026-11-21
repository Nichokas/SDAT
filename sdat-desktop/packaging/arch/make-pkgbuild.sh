#!/usr/bin/env bash
set -euo pipefail

archive=${1:?Usage: make-pkgbuild.sh ARCHIVE OUTPUT_DIR}
output_dir=${2:?Missing output directory}
if [[ ! ${archive##*/} =~ ^sdat-desktop-([0-9]+\.[0-9]+\.[0-9]+)-arch-x86_64\.tar\.zst$ ]]; then
    echo "Unexpected SDAT archive name: $archive" >&2
    exit 1
fi
version=${BASH_REMATCH[1]}
checksum=$(sha256sum < "$archive")
mkdir -p "$output_dir"
sed -e "s/@VERSION@/$version/g" -e "s/@SHA256@/${checksum%% *}/g" \
    "$(dirname "$0")/PKGBUILD.in" > "$output_dir/PKGBUILD"
