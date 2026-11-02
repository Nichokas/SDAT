# SDAT

Unofficial TIDAL players sharing one Rust core.

| Directory | What | Built by |
|---|---|---|
| `sdat-mobile/` | iPhone app (SwiftUI, iOS 26+, Liquid Glass), sideloaded with xtool / Feather | `.github/workflows/ios-build.yml` |
| `sdat-desktop/` | Linux desktop app (GTK4 / libadwaita), Rust port of [Tonearm](https://codeberg.org/dergs/Tonearm) | `.github/workflows/rust.yml`, `desktop-fedora.yml`, `desktop-copr.yml` |
| `sdat-core/` | Shared Rust core (`tonearm-core`) and the Swift FFI (`tonearm-ffi`, UniFFI) | both |
| `feather/` | Feather / AltStore source (`source.json`), kept at the root so existing source URLs keep working | `feather-source.yml` |

The root `Cargo.toml` is a workspace with `sdat-core/tonearm-core`, `sdat-core/tonearm-ffi` and `sdat-desktop`.

```sh
cargo test -p tonearm-core -p tonearm-ffi   # core
cargo run -p sdat-desktop                   # desktop (needs GTK4, libadwaita, GStreamer, libsecret)
sdat-mobile/scripts/build-rust.sh           # iOS static lib + Swift bindings, then `xtool dev` in sdat-mobile/
```

## Releases

- `vX.Y.Z` tags: iOS. The unsigned `.ipa` is attached to the release, and a PR is opened to update `feather/source.json`.
- `desktop-vX.Y.Z` tags (from 1.2.0): desktop. Fedora spec bump, vendor archive and COPR rebuild. The spec already ships 1.2.0, so `desktop-v1.2.0` only publishes the release and vendor archive; the first COPR build is triggered by hand and the automatic spec bump starts with the next version.

## History and licence

`sdat-mobile/` keeps the history of this repository; `sdat-desktop/` and `sdat-core/` keep the history of
[Nichokas/tonearm-rs](https://github.com/Nichokas/tonearm-rs), which itself descends from Tonearm by the dergs team.
The desktop app and core are GPL-3.0-or-later (see `sdat-desktop/LICENSE`); original authors are credited in the
desktop About dialog and metainfo.

Code written with an AI agent through Grok Bot. Design by Nichokas in Figma.
