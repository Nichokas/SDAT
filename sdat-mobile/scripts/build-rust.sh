#!/usr/bin/env bash
# Rebuilds libtonearm_ffi.a for iOS from the monorepo Cargo workspace (sdat-core/) and regenerates the Swift bindings.
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
core="${CORE_DIR:-$here/..}"
sdk="$HOME/.swiftpm/swift-sdks/darwin.artifactbundle/Developer/Platforms/iPhoneOS.platform/Developer/SDKs/iPhoneOS.sdk"
export PATH="$HOME/.local/share/swiftly/bin:$PATH"
cd "$core"
SDKROOT="$sdk" IPHONEOS_DEPLOYMENT_TARGET=17.0 CC_aarch64_apple_ios=clang AR_aarch64_apple_ios=llvm-ar \
  cargo rustc -p tonearm-ffi --lib --release --target aarch64-apple-ios --crate-type staticlib
lib="$core/target/aarch64-apple-ios/release/libtonearm_ffi.a"
cp "$lib" "$here/RustLib/libtonearm_ffi.a"
tmp="$(mktemp -d)"
cargo run -q -p tonearm-ffi --features bindgen --bin uniffi-bindgen-swift -- \
  --swift-sources --headers "$lib" "$tmp"
cp "$tmp/tonearm_ffiFFI.h" "$here/Sources/tonearm_ffiFFI/include/"
cp "$tmp/tonearm_ffi.swift" "$here/Sources/SDAT/Core/TonearmFFI.generated.swift"
rm -rf "$tmp"
echo "ok: $(du -h "$here/RustLib/libtonearm_ffi.a" | cut -f1)"
