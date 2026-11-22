# Publish sdat-bin to the AUR

When you run **Release** with a desktop version bump, GitHub Actions builds the
x86_64 Arch binary, attaches it to the GitHub release, and updates
[`sdat-bin`](https://aur.archlinux.org/packages/sdat-bin) in the AUR. The AUR
package downloads the release archive from GitHub.

To publish an existing desktop release, run **Desktop Arch Linux** from
`main` and enter its `desktop-vX.Y.Z` tag. To retry a failed publication, use
**Re-run failed jobs**. The workflow keeps the existing GitHub release archive
and skips AUR updates for older versions.

The publishing job uses the `AUR_SSH_KEY` secret in the `aur` GitHub Actions
environment. Restrict the environment to deployments from `main` and use an
SSH key registered to an AUR maintainer of `sdat-bin`.
