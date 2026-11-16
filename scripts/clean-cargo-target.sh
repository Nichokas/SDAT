#!/usr/bin/env bash
# Empties the shared Cargo target directory (see setup-sccache.sh) once it passes a size limit.
# sccache keeps the compiled dependencies, so the next build only recompiles our crates.
# cargo-sweep does not work here: it does not understand Cargo's per-unit build directory layout.
# `cargo clean` takes Cargo's build lock, so it waits for a running build instead of breaking it.
# Usage: clean-cargo-target.sh [limit in GiB, default $SDAT_TARGET_LIMIT_GB or 8]
set -euo pipefail

limit_gb="${1:-${SDAT_TARGET_LIMIT_GB:-8}}"
root="$(cd "$(dirname "$0")/.." && pwd)"
manifest="$root/Cargo.toml"

target_dir="$(cargo metadata --format-version 1 --no-deps --manifest-path "$manifest" |
  python3 -I -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')"
used_kb="$(du -sk "$target_dir" 2>/dev/null | cut -f1 || true)"
used_kb="${used_kb:-0}"

if [ "$used_kb" -le $((limit_gb * 1024 * 1024)) ]; then
  echo "$target_dir uses $((used_kb / 1024 / 1024)) GiB, under the ${limit_gb} GiB limit."
  exit 0
fi
echo "$target_dir uses $((used_kb / 1024 / 1024)) GiB, over the ${limit_gb} GiB limit; cleaning."
cargo clean --manifest-path "$manifest"
