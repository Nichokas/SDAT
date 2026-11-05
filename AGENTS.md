# AGENTS.md

Notes for coding agents working on SDAT, a TIDAL player for Fedora (GTK), iPhone and Android with a shared Rust core.

## Layout

- `sdat-core/tonearm-core` – Rust core: TIDAL API, playback queue, Listen Together (`src/live/`).
- `sdat-core/tonearm-ffi` – UniFFI bindings of the core for Swift (iOS) and Kotlin (Android).
- `sdat-desktop` – GTK4/libadwaita desktop app (Rust), packaged for Fedora (`packaging/fedora/`, COPR).
- `sdat-mobile` – iOS app (SwiftUI, built with xtool).
- `sdat-android` – Android app (Kotlin, Jetpack Compose, Glasmio-style UI).
- `feather/source.json` – Feather source for iOS sideloading; updated by CI, not by hand.
- Root `Cargo.toml` is the workspace (`sdat-desktop`, `tonearm-core`, `tonearm-ffi`) with strict lints.

## Build and test

- Core: `cargo test -p tonearm-core -p tonearm-ffi --locked`
- Desktop: `cargo test -p sdat-desktop --locked` (needs gtk4, libadwaita, gstreamer, libsecret devel packages).
- iOS: `sdat-mobile/scripts/build-rust.sh` rebuilds `libtonearm_ffi.a` and the Swift bindings; run it after any core change. The owner builds and installs iOS apps himself: never run `xtool dev`.
- Android: `scripts/build-rust-android.sh` (cargo-ndk, NDK 27) runs from Gradle; in `sdat-android/`: `./gradlew assembleDebug lintDebug testDebugUnitTest`.

## CI (`.github/workflows/`)

- `rust.yml` – core and desktop tests (desktop in a Fedora container).
- `ios-build.yml` – builds the unsigned `.ipa` on macOS; on `v*` tags attaches it to the release.
- `android-build.yml` – Gradle build, lint and unit tests.
- `feather-source.yml` – after an iOS release, opens a PR updating `feather/source.json`.
- `desktop-fedora.yml` / `desktop-copr.yml` – vendor archive for `desktop-v*` releases and the COPR rebuild.
- `review-bot.yml` – runs the three reviewers on a PR and keeps one comment with the findings, and approves the PR when they are clean; `review-bot-test.yml` tests its script.
- CI must use caches (`Swatinem/rust-cache`, `actions/cache`); PRs restore main's cache without saving their own.

## Releases

- Tags and releases only with the owner's explicit OK.
- Desktop: tag `desktop-vX.Y.Z`. iOS: tag `vX.Y.Z` with the unsigned `.ipa`.
- Version lives in `sdat-desktop/Cargo.toml`, `sdat-desktop/packaging/fedora/sdat-desktop.spec`, `sdat-desktop/build/com.nichokas.SDAT.Desktop.metainfo.xml`, `sdat-desktop/flake.nix` and `sdat-mobile/Info.plist`.
- Android: signed releases only; the signing key lives only in GitHub Actions secrets.

## Listen Together

- Never change playback rate or pitch to correct drift. A few seconds of divergence is fine; a stable, slower join beats a fast, glitchy one.
- Policy in `tonearm-core/src/live/drift.rs`: 3 s tolerance, at most one corrective seek per 10 s, 5 s settle after start or seek.
- The host sends an anchor every 5 s and on every status change; a guest seek goes through the host.

## Android

- `minSdk` 33; ABIs `arm64-v8a` and `x86_64` only.
- Never commit SF Pro. Local fonts go in the gitignored `sdat-android/local-assets/`; fall back to Inter/Material.
- Keep the APK small; no unused native libraries.

## Conventions

- Everything on GitHub (code, comments, commits, PRs, issues) in English. No secrets.
- Commit subject only, no body; explain in the PR description under `## Why`.
- Comments only for a non-obvious why. YAGNI; keep it simple.
- Use `cargo add` / `cargo remove`; never hand-edit dependencies in `Cargo.toml`.

## Pull requests

- The review bot runs on non-draft PRs and on every push to them; each run spends reviewer quota, so push finished batches. Comment `/review` to rerun it without a push. Fix the findings. It approves when every reviewer that ran is clean, and withdraws that approval on a later run that is not; a rate-limited CodeRabbit is skipped and does not block it.
- For a fork PR, or to review before pushing, run the CLIs: Macroscope (`--mode precise`), CodeRabbit (`coderabbit review --agent --committed --base main --deep`) and cubic (`cubic review --base main`).
- Bot accounts are repository secrets of this repo only. Change one with `python3 scripts/review_bot.py set-account <cubic|coderabbit|macroscope>`.
- Squash-merge when reviewers are clean, CI is green and review threads are resolved.
- The CodeRabbit GitHub app is removed: never post `@coderabbitai review`.
