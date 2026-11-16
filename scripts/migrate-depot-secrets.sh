#!/usr/bin/env bash
# Imports from GitHub into a Depot CI org the secrets and variables that the workflows in
# .depot/workflows/ reference, for example after moving to another Depot account. Names used only
# by workflows still on GitHub are left out on purpose, so credentials like the Android signing
# key never reach Depot before a Depot workflow needs them. Depot cannot export secret values,
# so GitHub is always the source. Run it from the repo root after `depot login` for the target:
#   scripts/migrate-depot-secrets.sh [ORG_ID]    (or DEPOT_ORG=ORG_ID)
# `depot ci migrate secrets-and-vars` makes a temporary local branch and prints a `git push`
# command; push it within five minutes and the one-shot workflow does the import.
set -euo pipefail

export PATH="$HOME/.depot/bin:$PATH"
command -v depot >/dev/null || { echo "Install the Depot CLI: curl -L https://depot.dev/install-cli.sh | sh" >&2; exit 1; }

names() { # $1: secrets or vars
  grep -rhoE "\b$1\.[A-Za-z_][A-Za-z0-9_]*" .depot/workflows | cut -d. -f2 |
    grep -vxE 'GITHUB_TOKEN|DEPOT_TOKEN' | sort -u || true
}

args=()
for kind in secrets vars; do
  while IFS= read -r name; do
    [[ -n $name ]] && args+=("--$kind" "$name")
  done < <(names "$kind")
done
if ((${#args[@]} == 0)); then
  echo "No .depot/workflows/ file references a secret or variable, so there is nothing to import."
  exit 0
fi

org="${1:-${DEPOT_ORG:-}}"
org_flag=()
[[ -n $org ]] && org_flag=(--org "$org")

depot ci migrate preflight ${org_flag[@]+"${org_flag[@]}"}
depot ci migrate secrets-and-vars ${org_flag[@]+"${org_flag[@]}"} "${args[@]}"
