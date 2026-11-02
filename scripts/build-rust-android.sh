#!/usr/bin/env bash
# Builds libtonearm_ffi.so for Android with cargo-ndk and regenerates the Kotlin bindings.
# Needs: rustup targets aarch64-linux-android + x86_64-linux-android, cargo-ndk, ANDROID_NDK_HOME (NDK 27).
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
app="${ANDROID_APP_DIR:-$root/sdat-android/app}"
jni="$app/src/main/jniLibs"
kotlin="$app/build/generated/source/uniffi/kotlin"
api="${ANDROID_API:-33}"
cd "$root"
# 16 KB page alignment is required for apps targeting Android 15+ on arm64.
RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=-Wl,-z,max-page-size=16384" \
  cargo ndk -t arm64-v8a -t x86_64 -P "$api" -o "$jni" build -p tonearm-ffi --lib --release
rm -rf "$kotlin"
# Library mode reads the UniFFI metadata embedded in the .so, so any ABI works.
cargo run -q -p tonearm-ffi --features bindgen --bin uniffi-bindgen -- \
  generate --library "$jni/arm64-v8a/libtonearm_ffi.so" --language kotlin --no-format --out-dir "$kotlin"
echo "ok: $(du -h "$jni"/*/libtonearm_ffi.so | tr '\n' ' ')"
