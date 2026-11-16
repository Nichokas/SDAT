#!/usr/bin/env bash
# Runs a CI workflow on Depot's machines against this working tree (unpushed changes included), so a build
# or test run does not use this machine's CPU. Waits for the result and downloads the artifacts to out/depot/.
#   scripts/depot-run.sh android   # assembleDebug, lintDebug, unit tests and assembleRelease
#   scripts/depot-run.sh rust      # core and desktop tests; add a job name (core, desktop) to run only that one
# Needs the Depot CLI logged in (depot login). It cannot build iOS: Depot CI has no macOS runners.
set -euo pipefail
export PATH="$HOME/.depot/bin:$PATH"
command -v depot >/dev/null || { echo "Install the Depot CLI (https://depot.dev/docs/cli/installation) and run 'depot login'." >&2; exit 1; }
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

target="${1:-}"
shift || true
case "$target" in
  android) workflow=android-build.yml; jobs=(build) ;;
  rust) workflow=rust.yml; jobs=("$@"); [ "${#jobs[@]}" -gt 0 ] || jobs=(core desktop) ;;
  *) echo "usage: $0 android | rust [core|desktop]" >&2; exit 2 ;;
esac

# depot ci run patches in tracked changes only, so a new file would be tested as missing and the run could still pass.
untracked="$(git ls-files --others --exclude-standard | grep -v '^\.depot/' || true)"
if [ -n "$untracked" ]; then
  echo "Untracked files would be left out of the Depot run. Run 'git add -N <file>' (or commit them) first:" >&2
  echo "$untracked" >&2
  exit 1
fi

args=(--workflow ".depot/workflows/$workflow")
for job in "${jobs[@]}"; do args+=(--job "$job"); done
output="$(depot ci run "${args[@]}")"
echo "$output"
run="$(sed -n 's/^Run: //p' <<<"$output" | head -1)"
[ -n "$run" ] || { echo "Could not read the run id from depot ci run." >&2; exit 1; }

if ! depot ci status "$run" --wait >/dev/null 2>&1; then
  depot ci diagnose --run "$run" || true
  echo "Run $run failed. Logs: depot ci status $run" >&2
  exit 1
fi
echo "Run $run passed."

out="$root/out/depot/$run"
mkdir -p "$out"
# Columns: artifact id, name, size, ...; the header and the "No artifacts found." line carry no uuid.
depot ci artifacts list "$run" | awk '$1 ~ /^[0-9a-f]+-[0-9a-f]+-/ { print $1, $2 }' | while read -r id name; do
  depot ci artifacts download "$id" --output-file "$out/$name.zip"
  echo "$out/$name.zip"
done
