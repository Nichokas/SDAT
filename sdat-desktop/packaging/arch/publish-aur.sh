#!/usr/bin/env bash
set -euo pipefail

tag=${1:?Usage: publish-aur.sh desktop-vX.Y.Z}
if [[ ! $tag =~ ^desktop-v([0-9]+\.[0-9]+\.[0-9]+)$ ]]; then
    echo "Invalid desktop tag: $tag" >&2
    exit 1
fi
version=${BASH_REMATCH[1]}
packaging_dir=$(dirname "$(realpath "$0")")
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT

printf '%s\n' "${AUR_SSH_KEY:?Set the AUR_SSH_KEY secret in the aur environment}" > "$work/key"
chmod 600 "$work/key"
unset AUR_SSH_KEY
cat > "$work/known_hosts" <<'EOF'
aur.archlinux.org ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEuBKrPzbawxA/k2g6NcyV5jmqwJ2s+zpgZGZ7tpLIcN
EOF
export GIT_SSH_COMMAND="ssh -o BatchMode=yes -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=$work/known_hosts -i $work/key"
git clone --depth=1 --branch master ssh://aur@aur.archlinux.org/sdat-bin.git "$work/sdat-bin"
cd "$work/sdat-bin"

current_version=$(awk '$1 == "pkgver" && $2 == "=" { print $3; exit }' .SRCINFO)
order=$(vercmp "$version" "${current_version:?Missing pkgver in AUR .SRCINFO}")
if (( order < 0 )); then
    echo "AUR already has sdat-bin $current_version. Skipping $version."
    exit 0
fi

archive="sdat-desktop-$version-arch-x86_64.tar.zst"
# A retry can rebuild a different archive. Use the copy users will download.
curl --fail --silent --show-error --location --retry 3 \
    "https://github.com/Nichokas/SDAT/releases/download/$tag/$archive" -o "$archive"
bash "$packaging_dir/make-pkgbuild.sh" "$archive" .
makepkg --printsrcinfo > .SRCINFO

if git diff --quiet -- PKGBUILD .SRCINFO; then
    echo "sdat-bin $version is already published."
    exit 0
fi
if (( order == 0 )); then
    echo "AUR already has $version with different package metadata." >&2
    exit 1
fi

git add PKGBUILD .SRCINFO
git -c user.name='github-actions[bot]' \
    -c user.email='41898282+github-actions[bot]@users.noreply.github.com' \
    commit -m "Update sdat-bin to $version"
git push origin HEAD:master
