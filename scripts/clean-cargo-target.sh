#!/usr/bin/env bash
# Empties the shared Cargo target directory (see setup-sccache.sh) once it passes a size limit.
# sccache keeps the compiled dependencies, so the next build only recompiles our crates.
# cargo-sweep does not work here: it does not understand Cargo's per-unit build directory layout.
# `cargo clean` takes no lock, so this takes the .cargo-lock that every build holds and skips the
# cleanup while one runs; the next run tries again.
# Usage: clean-cargo-target.sh [limit in GiB, default $SDAT_TARGET_LIMIT_GB or 8]
set -euo pipefail

# systemd starts this without the shell profile, so find rustup's cargo.
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"

limit_gb="${1:-${SDAT_TARGET_LIMIT_GB:-8}}"
root="$(cd "$(dirname "$0")/.." && pwd)"
manifest="$root/Cargo.toml"

target_dir="$(cargo metadata --format-version 1 --no-deps --manifest-path "$manifest" |
  python3 -I -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')"
if [ ! -d "$target_dir" ]; then
  echo "$target_dir does not exist; nothing to clean."
  exit 0
fi

used_kb="$(du -sLk "$target_dir" | cut -f1)"
if [ "$used_kb" -le $((limit_gb * 1024 * 1024)) ]; then
  echo "$target_dir uses $((used_kb / 1024 / 1024)) GiB, under the ${limit_gb} GiB limit."
  exit 0
fi

for lock in "$target_dir"/*/.cargo-lock; do
  [ -e "$lock" ] || continue
  exec {fd}<>"$lock"
  if ! flock -n "$fd"; then
    echo "A build is running in $target_dir; not cleaning now."
    exit 0
  fi
done
echo "$target_dir uses $((used_kb / 1024 / 1024)) GiB, over the ${limit_gb} GiB limit; cleaning."
cargo clean --manifest-path "$manifest"
