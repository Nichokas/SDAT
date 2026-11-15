# Obtainium

SDAT for Android can be installed and updated with [Obtainium](https://github.com/ImranR98/Obtainium). Obtainium has no source file like Feather's `source.json`: it reads the GitHub releases of this repository and installs the APK attached to the newest one.

## Add SDAT

Open this link on the phone with Obtainium installed:

[Add SDAT to Obtainium](obtainium://app/%7B%22id%22%3A%22com.nichokas.sdat%22%2C%22url%22%3A%22https%3A%2F%2Fgithub.com%2FNichokas%2FSDAT%22%2C%22author%22%3A%22Nichokas%22%2C%22name%22%3A%22SDAT%22%2C%22preferredApkIndex%22%3A0%2C%22additionalSettings%22%3A%22%7B%5C%22includePrereleases%5C%22%3Afalse%2C%5C%22fallbackToOlderReleases%5C%22%3Atrue%2C%5C%22filterReleaseTitlesByRegEx%5C%22%3A%5C%22%5ESDAT%20Android%5C%22%2C%5C%22versionExtractionRegEx%5C%22%3A%5C%22%5C%5C%5C%5Cd%2B%5C%5C%5C%5C.%5C%5C%5C%5Cd%2B%5C%5C%5C%5C.%5C%5C%5C%5Cd%2B%5C%22%2C%5C%22matchGroupToUse%5C%22%3A%5C%22%5C%22%2C%5C%22autoApkFilterByArch%5C%22%3Atrue%2C%5C%22trackOnly%5C%22%3Afalse%7D%22%7D)

Or add it by hand: **Add App**, source URL `https://github.com/Nichokas/SDAT`, then set these options under **Additional options**:

| Option | Value |
|---|---|
| Filter release titles by regex | `^SDAT Android` |
| Extract version with regex | `\d+\.\d+\.\d+` |

This repository also publishes iOS (`vX.Y.Z`) and desktop (`desktop-vX.Y.Z`) releases. The title filter keeps Obtainium on the Android ones (titled "SDAT Android X.Y.Z"). Obtainium takes the version from the release tag unless "Use release title as version" is on, so the version regex turns `android-v1.0.0` into `1.0.0`, which is what the installed app reports. Leave that option off.

Each Android release carries two APKs, `arm64-v8a` and `x86_64`. Obtainium picks the one that matches the device; leave **Filter APKs by regex** empty.

## Updates and signing

There is nothing to maintain here: the `Release` workflow attaches the signed APKs to every `android-v*` release (see `.github/workflows/android-release.yml`), and Obtainium finds them on its next check.

The APKs are signed with the key kept in this repository's Actions secrets. A build signed with another key, such as a local debug build, cannot be updated by Obtainium; uninstall it first.
