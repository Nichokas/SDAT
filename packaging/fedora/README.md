# Fedora / COPR packaging

COPR project: <https://copr.fedorainfracloud.org/coprs/nichokas/tonearm-rs/>

```
sudo dnf copr enable nichokas/tonearm-rs
sudo dnf install tonearm-rs
```

The RPM is `tonearm-rs` (binary `/usr/bin/tonearm`); it conflicts with the Go
package `tonearm` from `nichokas/tonearm`.

- `tonearm-rs.spec` builds with `TONEARM_VERSION=%{version} cargo build --release`
  (release build, no "devel" header), offline from a `cargo vendor` archive.
- `make-vendor.sh VERSION` creates `tonearm-rs-VERSION-vendor.tar.xz`, uploaded
  as an asset of GitHub release `vVERSION`.
- `.github/workflows/fedora-auto-update.yml`: on a published release (or daily /
  manually) bumps the spec, uploads the vendor asset, commits and triggers COPR.
- `.github/workflows/copr-publish.yml`: calls the COPR custom webhook. Needs the
  secret `COPR_WEBHOOK_TOKEN` (COPR → Settings → Integrations) and the variable
  `COPR_PROJECT_ID`.

COPR package source: SCM, `https://github.com/Nichokas/tonearm-rs.git`, branch
`main`, subdirectory `packaging/fedora`, spec `tonearm-rs.spec`, method rpkg.
