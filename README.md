# Torneam

TIDAL player for iOS 26 (SwiftUI, Liquid Glass), built with xtool for personal sideloading.
Uses tonearm-core from `../Tonearm` through `crates/tonearm-ffi` (UniFFI).

## Build

    scripts/build-rust.sh   # libtonearm_ffi.a for aarch64-apple-ios + Swift bindings
    xtool dev build         # needs Swift 6.4.2 (.swift-version) to match the iPhoneOS 27.1 SDK
    xtool dev               # build + install on a connected iPhone

Code written with an AI agent through Grok Bot. Design by Nichokas in Figma.
