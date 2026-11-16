#!/usr/bin/env bash
# Sets up the user's Cargo config so every git worktree shares build output:
#  - sccache reuses the dependencies another worktree already compiled, and
#  - one shared target directory, because sccache alone still copies every artifact into
#    each worktree's own target/ (several GB per worktree).
# Cargo locks the shared target directory, so builds from different worktrees run one at a time.
# Safe to run again; keys you already set under [build] are left alone.
set -euo pipefail

cargo_home="${CARGO_HOME:-$HOME/.cargo}"
target_dir="${SDAT_TARGET_DIR:-$HOME/.cache/cargo-target}"
command -v sccache >/dev/null || cargo install sccache --locked
sccache="$(command -v sccache || echo "$cargo_home/bin/sccache")"
"$sccache" --version

config="$cargo_home/config.toml"
mkdir -p "$cargo_home" "$target_dir"
touch "$config"

if grep -Eq '^[[:space:]]*(\[[[:space:]]*"build"[[:space:]]*\]|"?build"?[[:space:]]*\.)' "$config"; then
  echo "$config configures build in a form this script does not edit; add this under [build] yourself:" >&2
  printf 'rustc-wrapper = "%s"\ntarget-dir = "%s"\n' "$sccache" "$target_dir" >&2
  exit 1
fi

has_build_key() {
  awk -v key="$1" '
    /^[[:space:]]*\[/ { in_build = ($0 ~ /^[[:space:]]*\[[[:space:]]*build[[:space:]]*\]/) }
    in_build && $0 ~ "^[[:space:]]*" key "[[:space:]]*=" { found = 1 }
    END { exit !found }' "$config"
}

set_build_key() {
  local key="$1" value="$2"
  if has_build_key "$key"; then
    echo "$key is already set in $config"
  elif grep -Eq '^[[:space:]]*\[[[:space:]]*build[[:space:]]*\]' "$config"; then
    sed -i "/^[[:space:]]*\[[[:space:]]*build[[:space:]]*\]/a $key = \"$value\"" "$config"
    echo "Set $key in $config"
  else
    printf '\n[build]\n%s = "%s"\n' "$key" "$value" >>"$config"
    echo "Set $key in $config"
  fi
}

set_build_key rustc-wrapper "$sccache"
set_build_key target-dir "$target_dir"
