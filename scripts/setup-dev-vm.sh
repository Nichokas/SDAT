#!/usr/bin/env bash
# Provisions a fresh Ubuntu 25.10+ VM to build and test SDAT: core, GTK desktop app and Android app.
# For iOS it installs Swift and xtool; you still run `xtool setup` with Xcode.xip yourself.
# Safe to run again. Run as your normal user; it calls sudo for apt.
#
# Needs Ubuntu 25.10 or newer: the desktop crate requires GTK 4.20 and libadwaita 1.8, which older releases lack.
# Recommended VM: 6+ vCPU, 12+ GB RAM, 80 GB disk.
set -euo pipefail

ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
NDK_VERSION="27.1.12297006"
BUILD_TOOLS="37.0.0"
CMDLINE_TOOLS_ZIP="commandlinetools-linux-11076708_latest.zip"

export DEBIAN_FRONTEND=noninteractive
sudo apt-get update
sudo apt-get install -y \
  git build-essential cmake pkg-config clang unzip zip tar zstd curl ca-certificates gnupg \
  openjdk-21-jdk-headless \
  libgtk-4-dev libadwaita-1-dev libglib2.0-dev libssl-dev libsecret-1-dev \
  libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
  gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-libav \
  gh python3 \
  libfuse2t64 usbmuxd libimobiledevice-utils libcurl4-openssl-dev libxml2-dev libncurses-dev

if ! pkg-config --atleast-version=4.20 gtk4 2>/dev/null || ! pkg-config --atleast-version=1.8 libadwaita-1 2>/dev/null; then
  echo "Found GTK $(pkg-config --modversion gtk4 2>/dev/null || echo none) and libadwaita $(pkg-config --modversion libadwaita-1 2>/dev/null || echo none); the desktop app needs GTK 4.20 and libadwaita 1.8 (Ubuntu 25.10+)." >&2
  exit 1
fi

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
  # swiftly only knows LTS releases; a toolchain built for 24.04 needs the oldest glibc, so it runs on every release this script supports.
  "$tmp/swiftly" init --assume-yes --skip-install --quiet-shell-followup --platform ubuntu24.04
  rm -rf "$tmp"
fi
if [ -f "$HOME/.local/share/swiftly/env.sh" ]; then
  # shellcheck disable=SC1091
  . "$HOME/.local/share/swiftly/env.sh"
fi
post_install="$(mktemp)"
swiftly install --assume-yes --post-install-file "$post_install" "$(cat "$repo/sdat-mobile/.swift-version")"
if [ -s "$post_install" ]; then sudo bash "$post_install"; fi
rm -f "$post_install"
# The toolchain links libxml2.so.2, but Ubuntu 25.10+ ships only libxml2.so.16. A symlink next to the toolchain's libraries fixes it without touching the system.
if ! ldconfig -p | grep -q 'libxml2\.so\.2 '; then
  swift_lib="$(swiftly use --print-location)/usr/lib/swift/linux"
  ln -sf "$(ldconfig -p | awk '/libxml2\.so\.16 / {print $NF; exit}')" "$swift_lib/libxml2.so.2"
fi
if ! command -v xtool >/dev/null; then
  curl -fL "https://github.com/xtool-org/xtool/releases/latest/download/xtool-$(uname -m).AppImage" -o /tmp/xtool
  chmod +x /tmp/xtool
  sudo mv /tmp/xtool /usr/local/bin/xtool
fi

if [ ! -x "$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" ]; then
  rm -rf "$ANDROID_HOME/cmdline-tools/latest"
  mkdir -p "$ANDROID_HOME/cmdline-tools"
  tmp="$(mktemp -d)"
  curl -fsSL -o "$tmp/tools.zip" "https://dl.google.com/android/repository/$CMDLINE_TOOLS_ZIP"
  unzip -q "$tmp/tools.zip" -d "$tmp"
  mv "$tmp/cmdline-tools" "$ANDROID_HOME/cmdline-tools/latest"
  rm -rf "$tmp"
fi
sdkmanager="$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager"
yes | "$sdkmanager" --licenses >/dev/null || true
# The compileSdk platform is left out on purpose: Gradle downloads the right one on the first build, as in CI.
"$sdkmanager" "platform-tools" "build-tools;$BUILD_TOOLS" "ndk;$NDK_VERSION" < <(yes)

marker="# >>> sdat dev vm >>>"
touch "$HOME/.bashrc"
sed -i "/$marker/,/# <<< sdat dev vm <<</d" "$HOME/.bashrc"
{
  cat <<EOF
$marker
export JAVA_HOME="\$(dirname "\$(dirname "\$(readlink -f "\$(command -v javac)")")")"
export ANDROID_HOME="$ANDROID_HOME"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/$NDK_VERSION"
export PATH="\$HOME/.cargo/bin:\$HOME/.local/share/swiftly/bin:\$ANDROID_HOME/platform-tools:\$ANDROID_HOME/cmdline-tools/latest/bin:\$PATH"
# <<< sdat dev vm <<<
EOF
} >>"$HOME/.bashrc"

"$(dirname "$0")/setup-sccache.sh"

echo "Done. Open a new shell, then: cargo test -p tonearm-core -p tonearm-ffi --locked"
echo "iOS needs one manual step: download Xcode.xip from developer.apple.com (needs your Apple ID), then run 'xtool setup'."
