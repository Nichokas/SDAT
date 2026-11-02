#!/usr/bin/env bash
# Linker for aarch64-apple-ios when cross-building the Rust core on Linux:
# some dependencies (iroh-relay) also emit a dylib, which needs a real
# Apple-target link step. clang + ld64.lld from the swiftly toolchain.
sdk="$HOME/.swiftpm/swift-sdks/darwin.artifactbundle/Developer/Platforms/iPhoneOS.platform/Developer/SDKs/iPhoneOS.sdk"
exec clang -fuse-ld="$HOME/.swiftpm/swift-sdks/darwin.artifactbundle/toolset/bin/ld64.lld" -isysroot "$sdk" -Wl,-arch,arm64 -Wl,-platform_version,ios,17.0,17.0 -Wl,-syslibroot,"$sdk" "$@"
