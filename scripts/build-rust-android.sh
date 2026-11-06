#!/usr/bin/env bash
# Builds libtonearm_ffi.so for Android with cargo-ndk and regenerates the Kotlin bindings.
# Needs: rustup targets aarch64-linux-android + x86_64-linux-android, cargo-ndk, ANDROID_NDK_HOME (NDK 27).
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
app="${ANDROID_APP_DIR:-$root/sdat-android/app}"
jni="${ANDROID_JNI_DIR:-$app/src/main/jniLibs}"
kotlin="${ANDROID_KOTLIN_DIR:-$app/build/generated/source/uniffi/kotlin}"
api="${ANDROID_API:-33}"
cd "$root"
# 16 KB page alignment is required for apps targeting Android 15+ on arm64.
# Fat LTO with one codegen unit cuts the core by ~2.9 MB per ABI; Android only, so iOS and desktop builds are unchanged.
RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=-Wl,-z,max-page-size=16384" \
CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
  cargo ndk -t arm64-v8a -t x86_64 -P "$api" -o "$jni" build -p tonearm-ffi --lib --release
# cargo-ndk also copies iroh's own cdylibs; libtonearm_ffi.so links iroh statically and never loads them.
rm -f "$jni"/*/libiroh*.so
rm -rf "$kotlin"
# Library mode reads the UniFFI metadata; LTO drops it from the .so, so read the static archive (any ABI works).
cargo run -q -p tonearm-ffi --features bindgen --bin uniffi-bindgen -- \
  generate --library "${CARGO_TARGET_DIR:-$root/target}/aarch64-linux-android/release/libtonearm_ffi.a" --language kotlin --no-format --out-dir "$kotlin"
echo "ok: $(du -h "$jni"/*/libtonearm_ffi.so | tr '\n' ' ')"
