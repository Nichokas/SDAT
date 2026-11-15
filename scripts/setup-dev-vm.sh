#!/usr/bin/env bash
# Provisions a fresh Ubuntu 24.04+ VM to build and test SDAT: core, GTK desktop app and Android app.
# For iOS it installs Swift and xtool; you still run `xtool setup` with Xcode.xip yourself.
# Safe to run again. Run as your normal user; it calls sudo for apt.
#
# Needs Ubuntu 24.04 or newer: older releases ship a GTK4/libadwaita too old for the desktop app.
# Recommended VM: 6+ vCPU, 12+ GB RAM, 80 GB disk.
set -euo pipefail

ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
NDK_VERSION="27.1.12297006"
PLATFORM="android-37"
BUILD_TOOLS="37.0.0"
CMDLINE_TOOLS_ZIP="commandlinetools-linux-11076708_latest.zip"

export DEBIAN_FRONTEND=noninteractive
sudo apt-get update
sudo apt-get install -y \
  git build-essential cmake pkg-config clang unzip zip tar zstd curl ca-certificates \
  openjdk-21-jdk-headless \
  libgtk-4-dev libadwaita-1-dev libglib2.0-dev libssl-dev libsecret-1-dev \
  libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
  gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-libav \
  gh python3 \
  libfuse2t64 usbmuxd libimobiledevice-utils libcurl4-openssl-dev libxml2-dev libncurses-dev

if ! command -v rustup >/dev/null; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal -c clippy -c rustfmt
fi
# shellcheck disable=SC1091
. "$HOME/.cargo/env"
rustup update stable
rustup target add aarch64-linux-android x86_64-linux-android aarch64-apple-ios
command -v cargo-ndk >/dev/null || cargo install cargo-ndk --locked

# iOS: Swift via swiftly (version from sdat-mobile/.swift-version) and the xtool AppImage.
repo="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$HOME/.local/share/swiftly/bin:$PATH"
if ! command -v swiftly >/dev/null; then
  tmp="$(mktemp -d)"
  curl -fsSL -o "$tmp/swiftly.tar.gz" "https://download.swift.org/swiftly/linux/swiftly-$(uname -m).tar.gz"
  tar -xzf "$tmp/swiftly.tar.gz" -C "$tmp"
  "$tmp/swiftly" init --assume-yes --skip-install --quiet-shell-followup
  rm -rf "$tmp"
  # shellcheck disable=SC1091
  . "$HOME/.local/share/swiftly/env.sh"
fi
swiftly install --assume-yes "$(cat "$repo/sdat-mobile/.swift-version")"
if ! command -v xtool >/dev/null; then
  curl -fL "https://github.com/xtool-org/xtool/releases/latest/download/xtool-$(uname -m).AppImage" -o /tmp/xtool
  chmod +x /tmp/xtool
  sudo mv /tmp/xtool /usr/local/bin/xtool
fi

if [ ! -x "$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" ]; then
  mkdir -p "$ANDROID_HOME/cmdline-tools"
  tmp="$(mktemp -d)"
  curl -fsSL -o "$tmp/tools.zip" "https://dl.google.com/android/repository/$CMDLINE_TOOLS_ZIP"
  unzip -q "$tmp/tools.zip" -d "$tmp"
  mv "$tmp/cmdline-tools" "$ANDROID_HOME/cmdline-tools/latest"
  rm -rf "$tmp"
fi
sdkmanager="$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager"
yes | "$sdkmanager" --licenses >/dev/null || true
"$sdkmanager" "platform-tools" "platforms;$PLATFORM" "build-tools;$BUILD_TOOLS" "ndk;$NDK_VERSION"

marker="# >>> sdat dev vm >>>"
if ! grep -qF "$marker" "$HOME/.bashrc"; then
  cat >>"$HOME/.bashrc" <<EOF
$marker
export JAVA_HOME="\$(dirname "\$(dirname "\$(readlink -f "\$(command -v javac)")")")"
export ANDROID_HOME="$ANDROID_HOME"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/$NDK_VERSION"
export PATH="\$HOME/.cargo/bin:\$HOME/.local/share/swiftly/bin:\$ANDROID_HOME/platform-tools:\$ANDROID_HOME/cmdline-tools/latest/bin:\$PATH"
# <<< sdat dev vm <<<
EOF
fi

"$(dirname "$0")/setup-sccache.sh"

echo "Done. Open a new shell, then: cargo test -p tonearm-core -p tonearm-ffi --locked"
echo "iOS needs one manual step: download Xcode.xip from developer.apple.com (needs your Apple ID), then run 'xtool setup'."
