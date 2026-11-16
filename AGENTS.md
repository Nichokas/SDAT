# AGENTS.md

Notes for coding agents working on SDAT, a TIDAL player for Fedora (GTK), iPhone and Android with a shared Rust core.

## Layout

- `sdat-core/tonearm-core` – Rust core: TIDAL API, playback queue, Listen Together (`src/live/`).
- `sdat-core/tonearm-ffi` – UniFFI bindings of the core for Swift (iOS) and Kotlin (Android).
- `sdat-desktop` – GTK4/libadwaita desktop app (Rust), packaged for Fedora (`packaging/fedora/`, COPR).
- `sdat-mobile` – iOS app (SwiftUI, built with xtool).
- `sdat-android` – Android app (Kotlin, Jetpack Compose, Glasmio-style UI).
- `obtainium/README.md` – how to add the Android app to Obtainium; it reads the `android-v*` GitHub releases directly, so there is no source file to maintain.
- `feather/source.json` – Feather source for iOS sideloading; updated by CI, not by hand.
- Root `Cargo.toml` is the workspace (`sdat-desktop`, `tonearm-core`, `tonearm-ffi`) with strict lints.

## Build and test

- `scripts/setup-dev-vm.sh` provisions a fresh Ubuntu 25.10+ machine with every toolchain below (Rust, Android SDK/NDK, Swift, xtool, GTK dev packages).
- Run `scripts/setup-sccache.sh` once per machine: it enables sccache and points Cargo at one shared target directory (`~/.cache/cargo-target`), so worktrees neither recompile dependencies nor each keep GBs in `target/`. Cargo locks that directory, so builds from different worktrees run one at a time. Scripts must ask `cargo metadata` for the target directory, never assume `./target`.
- Core: `cargo test -p tonearm-core -p tonearm-ffi --locked`
- Desktop: `cargo test -p sdat-desktop --locked` (needs gtk4, libadwaita, gstreamer, libsecret devel packages).
- iOS: `sdat-mobile/scripts/build-rust.sh` rebuilds `libtonearm_ffi.a` and the Swift bindings; run it after any core change. The owner builds and installs iOS apps himself: never run `xtool dev`.
- Android: `scripts/build-rust-android.sh` (cargo-ndk, NDK 27) runs from Gradle; in `sdat-android/`: `./gradlew assembleDebug lintDebug testDebugUnitTest`.

## CI (`.github/workflows/`)

- `rust.yml` – core and desktop tests (desktop in a Fedora container).
- `ios-build.yml` – builds the unsigned `.ipa` on macOS; on `v*` tags attaches it to the release.
- `android-build.yml` – Gradle build, lint and unit tests.
- `android-release.yml` – signed per-ABI release APKs for an `android-v*` release, called by `release.yml`.
- `feather-source.yml` – after an iOS release, commits the `feather/source.json` update straight to main.
- `desktop-fedora.yml` / `desktop-copr.yml` – vendor archive for `desktop-v*` releases and the COPR rebuild.
- `release.yml` – manual release: pick a bump (`patch`, `minor`, `major` or `none`) per platform; it bumps from that platform's newest tag via `scripts/release-bump.sh`, commits to main, creates the releases with GitHub-generated notes, then calls the desktop (vendor archive, COPR), iOS build, Feather and Android release workflows.
- `review-bot.yml` – runs the reviewers on a PR and keeps one comment with the findings, and approves the PR when they are clean; `review-bot-test.yml` tests its script.
- On PRs, `rust.yml`, `ios-build.yml` and `android-build.yml` start with `review-gate.yml`, a called workflow that waits for the review bot and fails when it reports findings or a failed reviewer; the builds run only after it passes. `review-gate-rerun.yml` reruns the failed gates after a passing `/review`.
- `.depot/workflows/` – Depot CI copies of `rust.yml`, `android-build.yml` and `review-gate.yml`, made with `depot ci migrate workflows`; they run beside the GitHub ones until Depot is verified. The release chain stays on GitHub because iOS needs macOS (Depot CI is Ubuntu-only) and `feather-source.yml` / `desktop-fedora.yml` use the unsupported `release` trigger. The review bot and `review-gate-rerun.yml` also stay on GitHub; the rerun only finds GitHub Actions runs, so the Depot gate runs with `--keep-waiting` and releases itself when a `/review` passes. `scripts/migrate-depot-secrets.sh` imports into a new Depot org only the secrets and variables the `.depot/workflows/` files reference.
- CI must use caches (`Swatinem/rust-cache`, `actions/cache`); PRs restore main's cache without saving their own.

## Releases

- Tags and releases only with the owner's explicit OK.
- Release by running the `Release` workflow on main; it bumps the versions itself. Desktop, iOS and Android versions are independent. After a failure use "Re-run failed jobs", never "Re-run all jobs", which would bump again.
- Desktop: tag `desktop-vX.Y.Z`. iOS: tag `vX.Y.Z` with the unsigned `.ipa`. Android: tag `android-vX.Y.Z` with signed APKs for `arm64-v8a` and `x86_64`.
- Version lives in `sdat-desktop/Cargo.toml`, `sdat-desktop/packaging/fedora/sdat-desktop.spec`, `sdat-desktop/build/com.nichokas.SDAT.Desktop.metainfo.xml`, `sdat-desktop/flake.nix`, `sdat-mobile/Info.plist` and `sdat-android/app/build.gradle.kts` (`versionName`, `versionCode`).
- Android: signed releases only; the signing key lives only in GitHub Actions secrets (`ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`). Gradle signs a release build only when `SDAT_KEYSTORE` and its passwords are set in the environment.

## Listen Together

- Never change playback rate or pitch to correct drift. A few seconds of divergence is fine; a stable, slower join beats a fast, glitchy one.
- Policy in `tonearm-core/src/live/drift.rs`: 3 s tolerance, at most one corrective seek per 10 s, 5 s settle after start or seek.
- The host sends an anchor every 5 s and on every status change; a guest seek goes through the host.
- Player positions (GStreamer, AVPlayer) already are what is heard: guests add only the user's manual delay, never the system output latency.

## Android

- `minSdk` 33; ABIs `arm64-v8a` and `x86_64` only.
- Fonts are Inter for text and Material Symbols Rounded for icons. Never ship SF Pro: its license allows Apple platforms only.
- Keep the APK small; no unused native libraries.

## Conventions

- Everything on GitHub (code, comments, commits, PRs, issues) in English. No secrets.
- Commit subject only, no body; explain in the PR description under `## Why`.
- Comments only for a non-obvious why. YAGNI; keep it simple.
- Use `cargo add` / `cargo remove`; never hand-edit dependencies in `Cargo.toml`.

## Pull requests

- The review bot runs on non-draft PRs and on every push to them; each run spends reviewer quota, so push finished batches. Comment `/review` to rerun it without a push. Fix the findings. It approves when every reviewer that ran is clean, and withdraws that approval on a later run that is not.
- For a fork PR, or to review before pushing, run the CLIs: Macroscope (`--mode precise`) and cubic (`cubic review --base main`).
- Bot accounts are repository secrets of this repo only. Change one with `python3 scripts/review_bot.py set-account <cubic|macroscope>`.
- Squash-merge when reviewers are clean, CI is green and review threads are resolved.
- Agents watch every PR they open or push to until it merges or is closed: fix CI and the findings, answer the rest on their threads, and squash-merge as soon as the conditions above hold, without asking: the owner gave that standing OK.
- The CodeRabbit GitHub app is removed: never post `@coderabbitai review`.

## Web search

- If a dedicated search or scraping CLI is installed, such as `tavily-cli` or `firecrawl-cli`, prefer it over the default built-in web search tool.
