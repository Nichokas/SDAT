#!/usr/bin/env bash
# Installs sccache and enables it in the user's Cargo config, so every git worktree
# reuses the dependencies another one already compiled. Safe to run again.
set -euo pipefail

cargo_home="${CARGO_HOME:-$HOME/.cargo}"
command -v sccache >/dev/null || cargo install sccache --locked
sccache="$(command -v sccache || echo "$cargo_home/bin/sccache")"
"$sccache" --version

config="$cargo_home/config.toml"
mkdir -p "$cargo_home"
touch "$config"

# Only ever append a fresh [build] table; anything already there is left for the user to edit.
if grep -Eq 'rustc-wrapper.*sccache' "$config"; then
  echo "sccache is already enabled in $config"
elif grep -Eq 'rustc-wrapper|^[[:space:]]*(\[[[:space:]]*"?build"?[[:space:]]*\]|"?build"?[[:space:]]*\.)' "$config"; then
  echo "$config already configures build; add this under [build] yourself:" >&2
  echo "rustc-wrapper = \"$sccache\"" >&2
  exit 1
else
  printf '\n[build]\nrustc-wrapper = "%s"\n' "$sccache" >>"$config"
  echo "Enabled sccache in $config"
fi
