<div align="center">

  <a href="https://codeberg.org/dergs/Tonearm">
    <img src="internal/icons/hicolor/128x128/apps/com.nichokas.SDAT.Desktop.png" alt="Logo" width="80" height="80">
  </a>
  <h2 align="center">SDAT desktop (formerly tonearm-rs)</h2>

  [![Forks][forks-shield]][forks-url]
  [![Stargazers][stars-shield]][stars-url]
  [![Issues][issues-shield]][issues-url]
  [![License][license-shield]][license-url]

  <p align="center">
    <strong>SDAT desktop</strong> is a Rust port of Tonearm, an <strong>unofficial</strong> native GTK4 / Adwaita music streaming client for <a href="https://tidal.com">TIDAL</a>
    <br />
    <a href="#installation"><strong>How to Install »</strong></a>
    <br />
    <br />
    <a href="#features">Features</a>
    &middot;
    <a href="https://github.com/Nichokas/SDAT/issues/new">Report Bug or Request Feature</a>
    &middot;
    <a href="https://matrix.to/#/%23tonearm:derg.social">Join the Matrix Room</a>
  </p>

  <img width="700" src=".forgejo/images/home.png" alt="Screenshot of the home page of Tonearm">

</div>

## Credits — this is a Rust port of Tonearm

> [!IMPORTANT]
> This branch is an **unofficial Rust port** of **[Tonearm](https://codeberg.org/dergs/Tonearm)**.
> Tonearm was designed and built by the **dergs / Dergs team and all of its contributors**.
> Every bit of the user interface, the design, the TIDAL integration and the features you see here is
> their work. This port only translates their Go code to Rust, and it keeps the same UI and design.
>
> - Original project: <https://codeberg.org/dergs/Tonearm>. Please star it, report bugs upstream,
>   join their [Matrix room](https://matrix.to/#/%23tonearm:derg.social) and support the original authors.
> - License: like the original, this port is **GPL-3.0-or-later** (see [LICENSE](LICENSE)). All upstream
>   copyright notices, license headers and credits are kept.
> - The port is not affiliated with or endorsed by the original Tonearm authors or by TIDAL.
> - The port was done with the help of **Grok Bot**.

## What's new compared to the original
- **Add to Playlist** submenu in the track menu.
- **Right-click / long-press** context menu on tracks (rows and track cards); menus stay inside the window.
- **Explore**: featured album, artist and external-link cards.
- **Last.fm scrobbling** follows the Last.fm rules: tracks longer than 30 s, scrobbled after half the track or 4 minutes, with the start timestamp.
- **Smooth carousel arrows** (animated scrolling).
- **Infinite scroll prefetching**: the next pages load before you reach the end.
- **Background cache/prefetch** of playlists, albums and favourite tracks, so they open and play instantly.
- **Country / Region** preference for the TIDAL catalogue (defaults to your account's country).
- Smoother scrolling: track menus are built lazily and covers are decoded off the main thread.
- Release builds via `TONEARM_VERSION` (no "devel" header).

## Disclaimer
Tonearm is not affiliated with or endorsed by TIDAL. Tonearm is provided as-is without any warranty or guarantees. We explicitly do not offer and are not planning to offer any kind of offline playback functionality, Tonearm is not a downloader. A paid TIDAL account is required for full-length playback.

## Features
- Background Playback
  - Can be brought back to the front by MPRIS or by simply starting the app again
- Configurable through dconf
- Gapless Playback
- MPRIS playback information and player controls
- Position-aware Lyrics Viewer
  - Fallback to plain lyrics viewer if the song does not have timestamped lyrics
- Scrobbling to ListenBrainz-compatible servers
- Sign-in to your account via QR code / device linking code
  - Also works without signing in, with limited playback duration (Same as TIDAL web) 
- Supports playback of tracks in Max (AKA Master) quality
  - Currently, Tonearm will always play at the highest available quality
- Works with "Open in TIDAL" links (E.g. [tidal://my-collection](tidal://my-collection))

## About the Project
We set out to create a native, modern looking streaming client for TIDAL built with GTK4 and Libadwaita. We oriented a lot on existing GNOME apps (both official and third-party) to ensure the client really fits into the GNOME desktop and its applications, something web apps and electron apps often struggle with. We also try to keep the general spirit of TIDAL's UI so users can understand the app relatively quickly.

Tonearm came to existence as a learning project on how to implement GTK apps using GoLang, a rather unconventional choice. It was also an attempt at re-creating the look and feel of writing GUI applications in SwiftUI. This resulted in the development of the UI library [schwifty](https://codeberg.org/dergs/Tonearm/src/branch/main/pkg/schwifty). The application has since been ported to Rust (gtk4-rs, libadwaita-rs and gstreamer-rs) while keeping the exact same interface; the original Go sources remain in `cmd/`, `internal/` and `pkg/` for reference, and `internal/` still provides the shared assets (icons, styles, gschema, translations).

The current design for Tonearm originally came to be when I tried myself on a re-design of High Tide. This is why the sidebar features a very similar player layout. The rest of the application has largely been designed after common GNOME app practices and the TIDAL web interface.

## Installation
### Building from source
The SDAT desktop app is part of the SDAT monorepo Cargo workspace (run commands from the repository root). You need a Rust toolchain (edition 2024, Rust 1.85+) and the development files for GTK 4.20+, libadwaita 1.8+, GStreamer (base, good and bad plugins) and OpenSSL. On Fedora:
```sh
sudo dnf install cargo rust clippy gtk4-devel libadwaita-devel gstreamer1-devel gstreamer1-plugins-base-devel gstreamer1-plugins-good gstreamer1-plugins-bad-free openssl-devel
cargo build --release -p sdat-desktop
./target/release/sdat-desktop
```
The GSettings schema is compiled by `build.rs`, so the binary runs from the build tree without installing the schema. For a system install, also install `sdat-desktop/internal/settings/com.nichokas.SDAT.Desktop.gschema.xml` (plus the legacy `dev.dergs.Tonearm.gschema.xml`, read once to migrate old settings), the desktop file and metainfo from `sdat-desktop/build/`, and the icons from `sdat-desktop/internal/icons/hicolor`, then run `glib-compile-schemas`.

Packages for Fedora (COPR) and Arch Linux are coming soon.

### Flatpak
```sh
flatpak-builder --user --install build-dir sdat-desktop/build/flatpak/com.nichokas.SDAT.Desktop.yaml
```
`build/flatpak/cargo-sources.json` is generated from `Cargo.lock` with [flatpak-cargo-generator](https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo) and must be regenerated whenever dependencies change.

Currently the only tested installation method is the Nix flake provided in the repository. If you want to package this software for another distro or marketplace, please do open an issue so we can coordinate.

### NixOS (Flake)
This assumes that your system configuration is already done using a system flake. First add this repository as an input to your flake.
```nix
inputs = {

    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    ... your other inputs ...

    tonearm = {
      url = "git+https://codeberg.org/dergs/Tonearm.git";
      inputs.nixpkgs.follows = "nixpkgs";
    };

}
```
then add this anywhere in your system configuration as you see fit
```nix
{ inputs, ... }:

{

  # System Packages
  environment.systemPackages = [
    inputs.tonearm.packages.${pkgs.stdenv.hostPlatform.system}.tonearm
  ];

  # Or if you prefer via Home Manager
  home.packages = [
    inputs.tonearm.packages.${pkgs.stdenv.hostPlatform.system}.tonearm
  ];

}
```

### Arch Linux (AUR) 
You will require an AUR helper, such as yay or paru.

This assumes you are using the [yay helper](https://github.com/Jguer/yay). If using paru, adapt the commands accordingly.
```
yay -S tonearm
```

### Building a release

Packagers should build with `TONEARM_VERSION=1.0.0 cargo build --release` so the app is marked as a release (no "devel" header). See [CONTRIBUTING.md](CONTRIBUTING.md#release-builds-packagers).

## Translations
You can bring Tonearm to even more people by helping us translate it into their native language! To make this easier, Tonearm's translations can be edited on [Codeberg's Weblate instance](https://translate.codeberg.org/projects/tonearm/tonearm/)!

## Screenshots
<details>
  <summary>Explore Page</summary>
  <img src=".forgejo/images/explore.png" alt="Screenshot of the explore page">
</details>

<details>
  <summary>Search + Lyrics</summary>
  <img src=".forgejo/images/search_lyrics.png" alt="Screenshot of the search page and lyrics">
</details>

## Acknowledgements
The following projects and resources served as inspiration or were helpful during the development of Tonearm.
- [High Tide](https://github.com/Nokse22/high-tide/) for the original design of the player in the sidebar
- [puregotk](https://codeberg.org/puregotk/puregotk) for making the original Go version possible with only minimal CGO bindings
- [gtk-rs](https://gtk-rs.org/) and [gstreamer-rs](https://gitlab.freedesktop.org/gstreamer/gstreamer-rs) for the Rust bindings
- [TIDAL](https://tidal.com/) for the overall page designs, which we adapted for GTK


[license-shield]: https://img.shields.io/badge/license-gpl3%20or%20later-brightgreen?style=for-the-badge
[license-url]: https://codeberg.org/dergs/Tonearm/src/branch/main/LICENSE
[stars-shield]: https://img.shields.io/gitea/stars/dergs/Tonearm?gitea_url=https%3A%2F%2Fcodeberg.org&style=for-the-badge
[stars-url]: https://codeberg.org/dergs/Tonearm/stars
[forks-shield]: https://img.shields.io/gitea/forks/dergs/Tonearm?gitea_url=https%3A%2F%2Fcodeberg.org&style=for-the-badge
[forks-url]: https://codeberg.org/dergs/Tonearm/forks
[issues-shield]: https://img.shields.io/gitea/issues/open/dergs/Tonearm?gitea_url=https%3A%2F%2Fcodeberg.org&style=for-the-badge
[issues-url]: https://codeberg.org/dergs/Tonearm/issues
