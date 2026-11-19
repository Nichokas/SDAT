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
case "$target_dir" in
  *[\"\\]*) echo "SDAT_TARGET_DIR must not contain quotes or backslashes: $target_dir" >&2; exit 1 ;;
esac
command -v sccache >/dev/null || cargo install sccache --locked
sccache="$(command -v sccache || echo "$cargo_home/bin/sccache")"
"$sccache" --version

config="$cargo_home/config.toml"
# Cargo prefers the extensionless file when both names exist.
if [ -e "$cargo_home/config" ]; then
  config="$cargo_home/config"
fi
mkdir -p "$cargo_home" "$target_dir"
touch "$config"
# A dotfiles-managed config is a symlink; edit its destination so the link survives the atomic replace below.
config="$(readlink -f "$config" 2>/dev/null || echo "$config")"

python3 -c 'import tomllib' 2>/dev/null || { echo "python3 3.11 or newer is required to read $config." >&2; exit 1; }

# tomllib knows every spelling of a key (bare, quoted, dotted, inline table); a regex does not.
# Prints "no-build", "unset", or "set" followed by the current value.
build_key_state() {
  python3 -I -c 'import sys, tomllib
build = tomllib.load(open(sys.argv[1], "rb")).get("build")
key = sys.argv[2]
print("no-build" if build is None else f"set {build[key]}" if key in build else "unset")' "$config" "$1" 2>/dev/null
}

set_build_key() {
  local key="$1" value="$2" state tmp
  state="$(build_key_state "$key")" || { echo "$config is not valid TOML; fix it and run this again." >&2; exit 1; }
  if [ "${state%% *}" = set ]; then
    echo "$key is already set in $config"
    [ "${state#set }" = "$value" ] || echo "Note: it is '${state#set }', not '$value'; left unchanged, so edit it if worktrees should share build output." >&2
  elif [ "$state" = no-build ]; then
    printf '\n[build]\n%s = "%s"\n' "$key" "$value" >>"$config"
    echo "Set $key in $config"
  elif grep -Eq '^[[:space:]]*\[[[:space:]]*build[[:space:]]*\][[:space:]]*(#.*)?$' "$config"; then
    # No sed -i: its syntax differs between GNU and BSD.
    tmp="$(mktemp "$config.XXXXXX")"
    LINE="$key = \"$value\"" awk '{ print } !done && /^[[:space:]]*\[[[:space:]]*build[[:space:]]*\]/ { print ENVIRON["LINE"]; done = 1 }' "$config" >"$tmp"
    mv "$tmp" "$config" # same directory, so the swap is atomic and a full disk cannot leave a truncated config
    echo "Set $key in $config"
  else
    echo "$config defines build in a form this script does not edit; add this under [build] yourself:" >&2
    echo "$key = \"$value\"" >&2
    exit 1
  fi
}

set_build_key rustc-wrapper "$sccache"
set_build_key target-dir "$target_dir"

# The shared target directory only grows, so clean it daily once it passes a size limit.
# The timer runs the script from the main checkout: a worktree can be deleted when its thread settles.
main_root="$(git -C "$(dirname "$0")" rev-parse --path-format=absolute --git-common-dir)"
main_root="${main_root%/.git}"
if ! systemctl --user show-environment >/dev/null 2>&1; then
  echo "No systemd user session; run $main_root/scripts/clean-cargo-target.sh now and then to keep $target_dir small."
  exit 0
fi
unit_dir="$HOME/.config/systemd/user"
mkdir -p "$unit_dir"
cat >"$unit_dir/sdat-clean-cargo-target.service" <<UNIT
[Unit]
Description=Clean the shared Cargo target directory when it is too big

[Service]
Type=oneshot
Environment="CARGO_HOME=$cargo_home"
ExecStart="$main_root/scripts/clean-cargo-target.sh"
UNIT
cat >"$unit_dir/sdat-clean-cargo-target.timer" <<UNIT
[Unit]
Description=Daily Cargo target directory cleanup

[Timer]
OnCalendar=daily
Persistent=true

[Install]
WantedBy=timers.target
UNIT
systemctl --user daemon-reload
systemctl --user enable --now sdat-clean-cargo-target.timer
