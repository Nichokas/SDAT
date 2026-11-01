#!/usr/bin/env python3
"""Add or refresh one GitHub release in feather/source.json (AltStore/Feather source format).

Usage: update-feather-source.py <tag> [--source feather/source.json] [--repo owner/name]
Needs the `gh` CLI (authenticated) and git history containing the tag.
"""
import argparse, json, plistlib, re, subprocess, sys

def sh(*args):
    return subprocess.run(args, check=True, capture_output=True, text=True).stdout

def main():
    p = argparse.ArgumentParser()
    p.add_argument("tag")
    p.add_argument("--source", default="feather/source.json")
    p.add_argument("--repo", default="Nichokas/SDAT")
    a = p.parse_args()

    rel = json.loads(sh("gh", "api", f"repos/{a.repo}/releases/tags/{a.tag}"))
    if rel.get("draft") or rel.get("prerelease"):
        print(f"{a.tag} is a draft/prerelease, skipping"); return 0
    ipas = [x for x in rel["assets"] if x["name"].endswith(".ipa") and x["state"] == "uploaded"]
    if not ipas:
        print(f"{a.tag} has no .ipa asset yet, nothing to do"); return 0
    # Prefer the post-rename name (SDAT-<tag>-unsigned.ipa), fall back to any .ipa.
    ipas.sort(key=lambda x: (not x["name"].startswith("SDAT-"), x["name"]))
    ipa = ipas[0]

    version = re.sub(r"^v", "", a.tag)
    build = "1"
    try:
        info = plistlib.loads(sh("git", "show", f"{a.tag}:Info.plist").encode())
        version = info.get("CFBundleShortVersionString", version)
        build = str(info.get("CFBundleVersion", build))
    except Exception as e:
        print(f"warning: could not read Info.plist at {a.tag}: {e}", file=sys.stderr)

    with open(a.source) as f:
        src = json.load(f)
    app = src["apps"][0]
    entry = {
        "version": version,
        "buildVersion": build,
        "date": rel["published_at"],
        "localizedDescription": (rel.get("body") or "").strip() or f"SDAT {version}",
        "downloadURL": ipa["browser_download_url"],
        "size": ipa["size"],
        "minOSVersion": "26.0",
    }
    versions = [v for v in app.get("versions", []) if v["version"] != version]
    versions.append(entry)
    versions.sort(key=lambda v: v["date"], reverse=True)
    app["versions"] = versions
    latest = versions[0]
    # Legacy flat fields, still read by older AltStore-style clients.
    app.update(version=latest["version"], versionDate=latest["date"],
               versionDescription=latest["localizedDescription"],
               downloadURL=latest["downloadURL"], size=latest["size"])
    with open(a.source, "w") as f:
        json.dump(src, f, indent=2, ensure_ascii=False); f.write("\n")
    print(f"source updated with {version} ({ipa['name']}, {ipa['size']} bytes)")
    return 0

if __name__ == "__main__":
    sys.exit(main())
