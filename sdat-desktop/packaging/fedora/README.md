# Fedora / COPR packaging

COPR project (not renamed yet): <https://copr.fedorainfracloud.org/coprs/nichokas/tonearm-rs/>

```
sudo dnf copr enable nichokas/tonearm-rs
sudo dnf install sdat-desktop
```

The RPM is `sdat-desktop` (binary `/usr/bin/sdat-desktop`). It obsoletes and provides
`tonearm-rs`, so existing installs upgrade to it, and it conflicts with the Go package
`tonearm` from `nichokas/tonearm`.

- `sdat-desktop.spec` builds the whole monorepo workspace tarball of tag
  `desktop-vVERSION` with `cargo build --release -p sdat-desktop`, offline from a
  `cargo vendor` archive.
- `make-vendor.sh VERSION` creates `sdat-desktop-VERSION-vendor.tar.xz`, uploaded as an
  asset of GitHub release `desktop-vVERSION`.
- `.github/workflows/desktop-fedora.yml` (repo root): on a published `desktop-v*`
  release (or daily / manually) bumps the spec, uploads the vendor asset, commits and
  triggers COPR. On PRs it smoke-builds the RPM from the checkout.
- `.github/workflows/desktop-copr.yml`: calls the COPR custom webhook. Needs the secret
  `COPR_WEBHOOK_TOKEN` and the variable `COPR_PROJECT_ID` in this repo.

COPR package source (to be switched): SCM, `https://github.com/Nichokas/SDAT.git`, branch
`main`, subdirectory `sdat-desktop/packaging/fedora`, spec `sdat-desktop.spec`, method rpkg.
