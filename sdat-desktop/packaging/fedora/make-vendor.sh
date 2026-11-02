#!/usr/bin/env bash
# make-vendor.sh VERSION [OUTDIR]
# Build sdat-desktop-VERSION-vendor.tar.xz (`cargo vendor` of tag desktop-vVERSION's
# Cargo.lock) so the RPM build can run offline. Deterministic tar.
set -euo pipefail
VERSION="${1:?usage: make-vendor.sh VERSION [OUTDIR]}"; VERSION="${VERSION#v}"
OUTDIR="$(realpath -m "${2:-.}")"
REPO="${GITHUB_REPOSITORY:-Nichokas/SDAT}"
mkdir -p "$OUTDIR"
work="$(mktemp -d)"; trap 'rm -rf "$work"' EXIT
curl -fsSL "https://github.com/${REPO}/archive/desktop-v${VERSION}/SDAT-desktop-v${VERSION}.tar.gz" | tar -xz -C "$work"
cd "$work"/*/
cargo vendor --locked --versioned-dirs vendor >/dev/null
out="$OUTDIR/sdat-desktop-${VERSION}-vendor.tar.xz"
tar --sort=name --mtime='2000-01-01 00:00Z' --owner=0 --group=0 --numeric-owner -cJf "$out" vendor
echo "$out"
