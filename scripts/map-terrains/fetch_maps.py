#!/usr/bin/env python3
"""Downloads the map-terrain archives of the catalog and records which mirrors
serve them. Local tool for refreshing the pins of src-tauri/maps; it is not part
of the app or of CI.

    python3 -I scripts/map-terrains/fetch_maps.py src/webCatalog.json <out-dir> [--verify-mirrors]

For every `terrains` catalog item whose file is a ZIP it

  * sends a HEAD request to both mirrors the app may use (the Dota2PornFx
    project's GitHub repository and its Hugging Face dataset) and records the
    HTTP status and Content-Length of each;
  * downloads the archive from the first mirror that serves it to
    <out-dir>/zips/<catalog id>.zip (an archive already there is kept);
  * writes <out-dir>/fetch_report.json with the sizes and SHA-256 of the files;
  * with --verify-mirrors, also downloads the archive from each mirror that
    answered 200 and records that mirror's SHA-256, so the allowlist can say a
    mirror was checked by its bytes and not only by its Content-Length.

Then the allowlist is regenerated from that folder with the ignored test
`map_audit::generate_map_allowlist` (see the header of src-tauri/src/map_audit.rs
for the environment variables). The downloaded archives are untrusted data:
keep <out-dir> outside the repository and never commit it.
"""
import hashlib
import json
import os
import subprocess
import sys
import urllib.parse

GITHUB = "https://raw.githubusercontent.com/h6rd/Dota2PornFxWeb/main/assets/files/terrains/"
HUGGING_FACE = "https://huggingface.co/datasets/hrdq/Dota2PornFx/resolve/main/assets/files/terrains/"
MIRRORS = (("github", GITHUB), ("huggingface", HUGGING_FACE))


def head(url):
    """(status, content length) of a HEAD request that follows redirects."""
    result = subprocess.run(
        ["curl", "-sIL", "--max-time", "60", "-o", "/dev/null", "-w", "%{http_code} %header{content-length}", url],
        capture_output=True,
        text=True,
    )
    parts = result.stdout.split()
    status = int(parts[0]) if parts and parts[0].isdigit() else 0
    length = int(parts[1]) if len(parts) > 1 and parts[1].isdigit() else -1
    return status, length


def mirror_sha256(url):
    """SHA-256 of what a mirror serves for a URL, or None when it cannot be fetched."""
    result = subprocess.run(
        ["curl", "-sL", "--fail", "--max-time", "900", url], capture_output=True
    )
    return hashlib.sha256(result.stdout).hexdigest() if result.returncode == 0 else None


def main():
    catalog_path, out_dir = sys.argv[1], sys.argv[2]
    verify = "--verify-mirrors" in sys.argv[3:]
    with open(catalog_path, encoding="utf-8") as handle:
        catalog = json.load(handle)
    items = [
        item for item in catalog["items"]
        if item["category"] == "terrains" and item["file"].lower().endswith(".zip")
    ]
    os.makedirs(os.path.join(out_dir, "zips"), exist_ok=True)
    report = {}
    for item in items:
        quoted = urllib.parse.quote(item["file"])
        destination = os.path.join(out_dir, "zips", item["id"] + ".zip")
        row = {"file": item["file"], "mirrors": {}}
        for name, base in MIRRORS:
            row["mirrors"][name] = head(base + quoted)
        if not os.path.exists(destination):
            for name, base in MIRRORS:
                code = subprocess.run(
                    ["curl", "-sL", "--fail", "--max-time", "900", "-o", destination + ".part", base + quoted]
                ).returncode
                if code == 0 and os.path.getsize(destination + ".part") > 100:
                    os.rename(destination + ".part", destination)
                    row["fetched_from"] = name
                    break
        if os.path.exists(destination):
            with open(destination, "rb") as handle:
                data = handle.read()
            row["bytes"] = len(data)
            row["sha256"] = hashlib.sha256(data).hexdigest()
        if verify:
            row["mirror_sha256"] = {
                name: mirror_sha256(base + quoted)
                for name, base in MIRRORS
                if row["mirrors"][name][0] == 200
            }
        report[item["id"]] = row
        print(item["id"], row, flush=True)
    with open(os.path.join(out_dir, "fetch_report.json"), "w", encoding="utf-8") as handle:
        json.dump(report, handle, indent=1)


if __name__ == "__main__":
    main()
