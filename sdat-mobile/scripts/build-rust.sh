#!/usr/bin/env bash
# Rebuilds libtonearm_ffi.a for iOS from the monorepo Cargo workspace (sdat-core/) and regenerates the Swift bindings.
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
core="${CORE_DIR:-$here/..}"
sdk="$HOME/.swiftpm/swift-sdks/darwin.artifactbundle/Developer/Platforms/iPhoneOS.platform/Developer/SDKs/iPhoneOS.sdk"
export PATH="$HOME/.local/share/swiftly/bin:$PATH"
cd "$core"
# Cargo may use a shared target directory (scripts/setup-sccache.sh), so ask it instead of assuming ./target.
target="$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')"
mkdir -p "$target"
# One lock per target directory: another worktree must not rebuild the shared archive between our build and our reads of it.
if command -v flock >/dev/null; then
  exec 9>"$target/.sdat-build.lock"; flock 9
elif [ "$target" != "$(pwd -P)/target" ]; then
  echo "flock is required to share $target between worktrees; install util-linux or unset build.target-dir." >&2; exit 1
fi
# On Linux, link Apple dylibs (iroh-relay) with clang + ld64.lld.
if [ "$(uname)" = Linux ]; then export CARGO_TARGET_AARCH64_APPLE_IOS_LINKER="$here/scripts/ios-linker.sh"; fi
SDKROOT="$sdk" IPHONEOS_DEPLOYMENT_TARGET=17.0 CC_aarch64_apple_ios=clang AR_aarch64_apple_ios=llvm-ar \
  cargo rustc -p tonearm-ffi --lib --release --target aarch64-apple-ios --crate-type staticlib
lib="$target/aarch64-apple-ios/release/libtonearm_ffi.a"
cp "$lib" "$here/RustLib/libtonearm_ffi.a"
tmp="$(mktemp -d)"
cargo run -q -p tonearm-ffi --features bindgen --bin uniffi-bindgen-swift -- \
  --swift-sources --headers "$lib" "$tmp"
cp "$tmp/tonearm_ffiFFI.h" "$here/Sources/tonearm_ffiFFI/include/"
cp "$tmp/tonearm_ffi.swift" "$here/Sources/SDAT/Core/TonearmFFI.generated.swift"
rm -rf "$tmp"
echo "ok: $(du -h "$here/RustLib/libtonearm_ffi.a" | cut -f1)"
