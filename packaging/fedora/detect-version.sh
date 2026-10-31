#!/usr/bin/env bash
# detect-version.sh — newest vX.Y.Z tag of this repo vs. the spec Version.
# Writes version/current/is_new to $GITHUB_OUTPUT when set.
set -euo pipefail
cd "$(dirname "$0")"
git fetch --tags --quiet origin 2>/dev/null || true
latest="$(git tag -l 'v*' | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sed 's/^v//' | sort -V | tail -1)"
[[ -n "$latest" ]] || { echo "no release tags found" >&2; exit 1; }
current="$(awk '/^Version:/{print $2; exit}' tonearm-rs.spec)"
is_new=false
[[ "$(printf '%s\n%s\n' "$current" "$latest" | sort -V | tail -1)" != "$current" ]] && is_new=true
if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  { echo "version=$latest"; echo "current=$current"; echo "is_new=$is_new"; } >> "$GITHUB_OUTPUT"
fi
echo "current=$current latest=$latest is_new=$is_new" >&2
echo "$latest"
