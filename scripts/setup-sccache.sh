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

# tomllib knows every spelling of a key (bare, quoted, dotted, inline table); a regex does not.
build_key_state() {
  python3 -I -c 'import sys, tomllib
build = tomllib.load(open(sys.argv[1], "rb")).get("build")
print("no-build" if build is None else "set" if sys.argv[2] in build else "unset")' "$config" "$1"  2>/dev/null
}

set_build_key() {
  local key="$1" value="$2" state
  state="$(build_key_state "$key")" || { echo "$config is not valid TOML; fix it and run this again." >&2; exit 1; }
  if [ "$state" = set ]; then
    echo "$key is already set in $config"
  elif [ "$state" = no-build ]; then
    printf '\n[build]\n%s = "%s"\n' "$key" "$value" >>"$config"
    echo "Set $key in $config"
  elif grep -Eq '^[[:space:]]*\[[[:space:]]*build[[:space:]]*\][[:space:]]*(#.*)?$' "$config"; then
    sed -i "/^[[:space:]]*\[[[:space:]]*build[[:space:]]*\]/a $key = \"$value\"" "$config"
    echo "Set $key in $config"
  else
    echo "$config defines build in a form this script does not edit; add this under [build] yourself:" >&2
    echo "$key = \"$value\"" >&2
    exit 1
  fi
}

set_build_key rustc-wrapper "$sccache"
set_build_key target-dir "$target_dir"
