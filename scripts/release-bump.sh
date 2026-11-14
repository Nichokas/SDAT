#!/usr/bin/env bash
# release-bump.sh <desktop|ios> <patch|minor|major>
# Bumps the platform's newest release tag and writes the new version into its files.
# With RESUME=1 (a rerun of a failed Release) it reuses what the first attempt did: a tag of this
# platform already on HEAD, or a version already in the files but not yet tagged. Prints the tag.
# Desktop and iOS versions are independent.
set -euo pipefail
platform="${1:?usage: release-bump.sh <desktop|ios> <patch|minor|major>}"
level="${2:?usage: release-bump.sh <desktop|ios> <patch|minor|major>}"
cd "$(dirname "$0")/.."

plist=sdat-mobile/Info.plist
case "$platform" in
  desktop) series=desktop-v; current="$(awk -F'"' '/^version = /{print $2; exit}' sdat-desktop/Cargo.toml)" ;;
  ios) series=v; current="$(grep -A1 '<key>CFBundleShortVersionString</key>' "$plist" | grep -oP '<string>\K[^<]+')" ;;
  *) echo "unknown platform: $platform" >&2; exit 1 ;;
esac
pattern="^${series}[0-9]+\.[0-9]+\.[0-9]+$"
on_head="$(git tag --points-at HEAD | grep -E "$pattern" | sort -V | tail -1 || true)"
if [[ -n "${RESUME:-}" && -n "$on_head" ]]; then echo "reusing $on_head, already on HEAD" >&2; echo "$on_head"; exit 0; fi
latest="$(git tag -l "${series}*" | grep -E "$pattern" | sort -V | tail -1 || true)"
IFS=. read -r major minor patch <<<"${latest#"$series"}"
major="${major:-0}" minor="${minor:-0}" patch="${patch:-0}"
case "$level" in
  major) version="$((major + 1)).0.0" ;;
  minor) version="$major.$((minor + 1)).0" ;;
  patch) version="$major.$minor.$((patch + 1))" ;;
  *) echo "unknown level: $level" >&2; exit 1 ;;
esac
if [[ -n "${RESUME:-}" && "$current" != "${latest#"$series"}" && "$(printf '%s\n%s\n' "$current" "${latest#"$series"}" | sort -V | tail -1)" == "$current" ]]; then
  echo "reusing untagged $current instead of a $level bump" >&2
  version="$current"
fi
tag="$series$version"

if [[ "$platform" == desktop ]]; then
  sed -i -E "0,/^version = \"[^\"]+\"/s//version = \"$version\"/" sdat-desktop/Cargo.toml
  sed -i -E "/^name = \"sdat-desktop\"$/{n;s/^version = \"[^\"]+\"/version = \"$version\"/}" Cargo.lock
  sed -i -E "0,/version = \"[^\"]+\";/s//version = \"$version\";/" sdat-desktop/flake.nix
  meta=sdat-desktop/build/com.nichokas.SDAT.Desktop.metainfo.xml
  # Keep release notes written ahead of time for this version.
  if ! grep -q "<release version=\"$version\"" "$meta"; then
    entry="    <release version=\"$version\" date=\"$(date -u +%F)\">\n        <url type=\"details\">https://github.com/Nichokas/SDAT/releases/tag/$tag</url>\n    </release>"
    sed -i "0,/<releases>/s||<releases>\n$entry|" "$meta"
  fi
  bash sdat-desktop/packaging/fedora/update-spec.sh "$version" --no-vendor >&2
elif [[ "$current" != "$version" ]]; then
  build="$(grep -A1 '<key>CFBundleVersion</key>' "$plist" | grep -oP '<string>\K[0-9]+')"
  sed -i -E "/<key>CFBundleShortVersionString<\/key>/{n;s|<string>[^<]*</string>|<string>$version</string>|}" "$plist"
  sed -i -E "/<key>CFBundleVersion<\/key>/{n;s|<string>[^<]*</string>|<string>$((build + 1))</string>|}" "$plist"
fi
echo "$tag"
