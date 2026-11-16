#!/usr/bin/env bash
# Empties the shared Cargo target directory (see setup-sccache.sh) once it passes a size limit.
# sccache keeps the compiled dependencies, so the next build only recompiles our crates.
# cargo-sweep does not work here: it does not understand Cargo's per-unit build directory layout.
# Nothing here is cleaned while a build runs: this takes the same locks the builds hold (the
# build scripts' .sdat-build.lock and Cargo's .cargo-lock and .cargo-artifact-lock, also under target-triple directories)
# and skips the cleanup if one is taken; the next run tries again. The lock files themselves stay,
# because a build that starts during the cleanup must lock the same file.
# Usage: clean-cargo-target.sh [limit in GiB, default $SDAT_TARGET_LIMIT_GB or 8]
set -euo pipefail

# systemd starts this without the shell profile, so find rustup's cargo.
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"

limit_gb="${1:-${SDAT_TARGET_LIMIT_GB:-8}}"
root="$(cd "$(dirname "$0")/.." && pwd)"

target_dir="$(cargo metadata --format-version 1 --no-deps --manifest-path "$root/Cargo.toml" |
  python3 -I -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')"
if [ ! -d "$target_dir" ]; then
  echo "$target_dir does not exist; nothing to clean."
  exit 0
fi

target_dir="$(cd -P "$target_dir" && pwd)" # find and du must see a symlinked target's contents
used_kb="$(du -sk "$target_dir" | cut -f1)"
if [ "$used_kb" -le $((limit_gb * 1024 * 1024)) ]; then
  echo "$target_dir uses $((used_kb / 1024 / 1024)) GiB, under the ${limit_gb} GiB limit."
  exit 0
fi

while IFS= read -r lock; do
  exec {fd}<>"$lock"
  if ! flock -n "$fd"; then
    echo "A build holds $lock; not cleaning now."
    exit 0
  fi
done < <(find "$target_dir" -maxdepth 3 \( -name ".cargo-*lock" -o -name .sdat-build.lock \) -type f)

echo "$target_dir uses $((used_kb / 1024 / 1024)) GiB, over the ${limit_gb} GiB limit; cleaning."
find "$target_dir" -mindepth 1 ! -type d ! -name ".cargo-*lock" ! -name .sdat-build.lock -delete
find "$target_dir" -mindepth 1 -type d -empty -delete
