# Publish sdat-bin to the AUR

The Release workflow builds the x86_64 Arch binary for each desktop release.
To build it for an existing release, run the **Desktop Arch Linux** workflow
with its `desktop-vX.Y.Z` tag. The workflow attaches the archive to that release
and keeps any previously uploaded copy so the AUR checksum stays valid.

From the SDAT repository root on Arch Linux, download the release archive and
generate the files in your AUR checkout. Set `version` to the desktop version
you are publishing.

```sh
version=2.3.0
archive="sdat-desktop-$version-arch-x86_64.tar.zst"
downloads=$(mktemp -d)
aur_checkout=/path/to/sdat-bin
gh release download "desktop-v$version" --pattern "$archive" --dir "$downloads"
bash sdat-desktop/packaging/arch/make-pkgbuild.sh "$downloads/$archive" "$aur_checkout"
cd "$aur_checkout"
makepkg --printsrcinfo > .SRCINFO
makepkg --syncdeps
```

Commit `PKGBUILD` and `.SRCINFO`, then push to the AUR's `master` branch.
