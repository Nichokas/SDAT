## Issues

We use Codeberg issues to track public bugs. Please ensure your description is clear and has sufficient instructions to be able to reproduce the issue.

## License

By contributing to Tonearm, you agree that your contributions will be licensed under the LICENSE file in the root directory of this source tree.

## Code of Conduct

We expect all contributors to adhere to the [GNOME Code of Conduct](https://conduct.gnome.org/).

## Development Environment

Tonearm provides a standardized development environment in the form of a direnv file as well as a Nix dev-shell. Please ensure that you have direnv and nix, the package manager, installed on your system.

The dev-shell provides `cargo`, `rustc`, `clippy`, `rustfmt` and `rust-analyzer`. Before submitting changes, make sure these pass:

```sh
cargo build --release
cargo clippy
cargo test
```

We recommend using Zed to edit the codebase, as that is what we have been using for development. But anything that supports direnv should be fine.

## UI / UX

Tonearm aims to follow the [GNOME Human Interface Guidelines (HIG)](https://developer.gnome.org/hig/) and use existing widgets and functionality where possible. In case you need to create something entirely new, please consider the GNOME HIG and try to follow their recommendations.

Tonearm is written in Rust on top of gtk4-rs and libadwaita-rs. UI code lives in `src/ui`; small builder helpers in `src/ui/dsl.rs` mirror the old Schwifty syntax so the widget trees, CSS classes and margins stay identical to the original Go version. Styles and icons are the compiled GResources in `internal/styles` and `internal/icons`, translations are the `.po` files in `internal/gettext/locales`.

## Updating App Metadata & Screenshots
When contributing to Tonearm, please ensure that you update the app metadata and screenshots accordingly. This includes updating the app icon, splash screen, and any other visual elements that may be affected by your changes.

### Screenshots
We aim to make screenshots follow the recommendations by [Flatpak](https://docs.flathub.org/docs/for-app-authors/metainfo-guidelines/quality-guidelines#screenshots) and [GNOME](https://gitlab.gnome.org/GNOME/Initiatives/-/wikis/Update-App-Screenshots). Some important recommendations include:

- Show off the app with realistic and good-looking example content
- Use GNOME system defaults (stylesheet, font, icons, window decorations, accent color, etc.)
- Do not manipulate the screenshot to make it look better than it is
- Use the GNOME screenshot tool in window mode to capture screenshots
- Write a one-line caption that describes the screenshot

It is recommended to make screenshots in the 16:9 aspect ratio. We recommend scaling the window to 1270x815px resolution when refreshing the screenshots.

## Release builds (packagers)

Set `TONEARM_VERSION` at compile time to mark a build as a release (Rust equivalent of Go's `-ldflags "-X …/ui.Version=…"`):

```sh
TONEARM_VERSION=1.0.0 cargo build --release
```

- With `TONEARM_VERSION` set, the About dialog shows that version and the striped "devel" header bar is hidden.
- Without it, the build is a development build: the header bar is striped and the version comes from `git describe` (`TONEARM_COMMIT` overrides it), falling back to `<Cargo.toml version>-dev`. As in Go, an exact tag checkout (`vX.Y.Z-0-g…`) also counts as stable.
- COPR RPM spec: `TONEARM_VERSION=%{version} cargo build --release` in `%build`. Arch PKGBUILD: `TONEARM_VERSION="$pkgver" cargo build --release` in `build()`. Flatpak: add `TONEARM_VERSION: "1.0.0"` under the module's `build-options.env`.
