#!/usr/bin/env bash
# update-spec.sh VERSION [--no-vendor]
# Bump tonearm-rs.spec to VERSION (Release 1, changelog entry) and build the
# vendor archive into dist/ (to upload as asset of release vVERSION).
set -euo pipefail
VERSION="${1:?usage: update-spec.sh VERSION [--no-vendor]}"; VERSION="${VERSION#v}"
cd "$(dirname "$0")"
SPEC=tonearm-rs.spec
current="$(awk '/^Version:/{print $2; exit}' "$SPEC")"
if [[ "$current" != "$VERSION" ]]; then
  sed -i -E "s/^(Version:[[:space:]]+).*/\1${VERSION}/; s/^(Release:[[:space:]]+)[0-9]+/\11/" "$SPEC"
  entry="* $(LC_ALL=C date -u '+%a %b %d %Y') ${PACKAGER:-tonearm-rs CI <noreply@github.com>} - ${VERSION}-1\n- Update to ${VERSION}\n"
  sed -i "/^%changelog/a ${entry}" "$SPEC"
  echo "spec: ${current} -> ${VERSION}"
else
  echo "spec already at ${VERSION}"
fi
[[ "${2:-}" == "--no-vendor" ]] || bash make-vendor.sh "$VERSION" ../../dist
