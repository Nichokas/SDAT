# Feather source

SDAT is published as an AltStore-style source that [Feather](https://github.com/CLARATION/Feather) can read.

**Source URL:**

```
https://raw.githubusercontent.com/Nichokas/SDAT/main/feather/source.json
```

To add it in Feather: open the **Sources** tab, tap **+**, paste the URL above and confirm.
SDAT then shows up in the list and can be installed from there.

The `.ipa` files are **unsigned**; Feather signs them with your own certificate when installing.

## How it is updated

`.github/workflows/feather-source.yml` runs when a release is published or edited, and after the
"iOS build" workflow attaches the `.ipa` to a `v*` tag. It runs `scripts/update-feather-source.py`,
which adds the release (version, build, date, notes, `.ipa` URL and size) to `versions[]`, keeping
older versions, and opens a pull request with the change. It can also be run by hand from the
Actions tab (`workflow_dispatch`, optional tag).

Local run: `python3 scripts/update-feather-source.py v1.0.0` (needs `gh` logged in).
