#!/usr/bin/env bash
# detect-version.sh — newest desktop-vX.Y.Z tag of the monorepo vs. the spec Version.
# (Plain vX.Y.Z tags are iOS releases and are ignored here.)
# Writes version/current/is_new to $GITHUB_OUTPUT when set.
set -euo pipefail
cd "$(dirname "$0")"
git fetch --tags --quiet origin 2>/dev/null || true
latest="$(git tag -l 'desktop-v*' | grep -E '^desktop-v[0-9]+\.[0-9]+\.[0-9]+$' | sed 's/^desktop-v//' | sort -V | tail -1)"
[[ -n "$latest" ]] || { echo "no release tags found" >&2; exit 1; }
current="$(awk '/^Version:/{print $2; exit}' sdat-desktop.spec)"
is_new=false
[[ "$(printf '%s\n%s\n' "$current" "$latest" | sort -V | tail -1)" != "$current" ]] && is_new=true
if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  { echo "version=$latest"; echo "current=$current"; echo "is_new=$is_new"; } >> "$GITHUB_OUTPUT"
fi
echo "current=$current latest=$latest is_new=$is_new" >&2
echo "$latest"
