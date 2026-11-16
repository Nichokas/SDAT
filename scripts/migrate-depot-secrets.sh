#!/usr/bin/env bash
# Imports the GitHub secrets and variables the workflows use into a Depot CI org, for example
# after moving to another Depot account. Depot cannot export secret values, so the source is
# always GitHub. Run it from the repo root after `depot login` for the target account:
#   scripts/migrate-depot-secrets.sh [ORG_ID]    (or DEPOT_ORG=ORG_ID)
# `depot ci migrate secrets-and-vars` makes a temporary local branch and prints a `git push`
# command; push it within five minutes and the one-shot workflow does the import.
set -euo pipefail

export PATH="$HOME/.depot/bin:$PATH"
command -v depot >/dev/null || { echo "Install the Depot CLI: curl -L https://depot.dev/install-cli.sh | sh" >&2; exit 1; }

org="${1:-${DEPOT_ORG:-}}"
org_flag=()
[[ -n $org ]] && org_flag=(--org "$org")

depot ci migrate preflight "${org_flag[@]}"
depot ci migrate secrets-and-vars "${org_flag[@]}"
