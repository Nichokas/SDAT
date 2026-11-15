#!/usr/bin/env bash
# Installs sccache and enables it in the user's Cargo config, so every git worktree
# reuses the dependencies another one already compiled. Safe to run again.
set -euo pipefail

command -v sccache >/dev/null || cargo install sccache --locked

config="${CARGO_HOME:-$HOME/.cargo}/config.toml"
mkdir -p "$(dirname "$config")"
touch "$config"

if grep -Eq '^[[:space:]]*rustc-wrapper[[:space:]]*=' "$config"; then
  echo "rustc-wrapper is already set in $config"
elif grep -Eq '^[[:space:]]*\[build\][[:space:]]*$' "$config"; then
  tmp="$(mktemp)"
  awk '{ print } /^[[:space:]]*\[build\][[:space:]]*$/ { print "rustc-wrapper = \"sccache\"" }' "$config" >"$tmp"
  cat "$tmp" >"$config"
  rm "$tmp"
  echo "Enabled sccache in $config"
else
  printf '\n[build]\nrustc-wrapper = "sccache"\n' >>"$config"
  echo "Enabled sccache in $config"
fi

sccache --version
