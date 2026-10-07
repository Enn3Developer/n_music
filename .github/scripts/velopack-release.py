#!/usr/bin/env python3
"""Velopack release files for one channel of the desktop app.

previous <owner/repo> <channel> <package id> <version> <folder>
    Downloads the newest full package of <package id> released on GitHub in <channel> before
    <version> into <folder>, where `vpk pack` builds a delta package from it. When there is
    none it says so, and when GitHub cannot be reached it warns; either way the release then
    has full packages only. Reads GH_TOKEN, if set, to stay clear of GitHub's API rate limit.

collect <folder> <channel> <output>
    Copies the files `vpk pack` just made in <folder> to <output>: the packages listed in
    assets.<channel>.json, that list, and the feeds releases.<channel>.json and
    RELEASES-<channel> limited to those packages. The feeds `vpk pack` writes also list the
    package `previous` downloaded, which an older release already has. Velopack's updater
    reads releases.<channel>.json from each of the newest GitHub releases and adds them up.
"""

import hashlib
import json
import os
import shutil
import sys
import urllib.request
from pathlib import Path


def semver(text):
    """A sort key for a SemVer version: prereleases come before their release."""
    core, _, prerelease = text.split("+", 1)[0].partition("-")
    numbers = tuple(int(part) for part in core.split("."))
    if not prerelease:
        return numbers, (1,)
    identifiers = tuple((0, int(part), "") if part.isdigit() else (1, 0, part) for part in prerelease.split("."))
    return numbers, (0, identifiers)


def request(url, accept):
    req = urllib.request.Request(url, headers={"Accept": accept, "X-GitHub-Api-Version": "2022-11-28"})
    if token := os.environ.get("GH_TOKEN"):
        # Only for api.github.com: an asset download redirects to storage that refuses it.
        req.add_unredirected_header("Authorization", f"Bearer {token}")
    with urllib.request.urlopen(req, timeout=120) as response:
        return response.read()


def previous(repository, channel, package_id, version, folder):
    feed_name = f"releases.{channel}.json"
    current = semver(version)
    best = None
    releases = json.loads(request(f"https://api.github.com/repos/{repository}/releases?per_page=30", "application/vnd.github+json"))
    for release in releases:
        assets = {asset["name"]: asset for asset in release["assets"]}
        if release["draft"] or feed_name not in assets:
            continue
        feed = json.loads(request(assets[feed_name]["url"], "application/octet-stream").decode("utf-8-sig"))
        for entry in feed["Assets"]:
            if entry["Type"] != "Full" or entry["PackageId"] != package_id or entry["FileName"] not in assets:
                continue
            key = semver(entry["Version"])
            if key < current and (best is None or key > best[0]):
                best = key, entry, assets[entry["FileName"]], release["tag_name"]
    if best is None:
        print(f"::notice::No {package_id} release in channel {channel} before {version}, so no delta package")
        return
    _, entry, asset, tag = best
    data = request(asset["url"], "application/octet-stream")
    if len(data) != entry["Size"] or hashlib.sha256(data).hexdigest().upper() != entry["SHA256"].upper():
        raise ValueError(f"{entry['FileName']} from {tag} does not match its feed")
    Path(folder, entry["FileName"]).write_bytes(data)
    print(f"Downloaded {entry['FileName']} from {tag} to build a delta package from")


def collect(folder, channel, output):
    folder, output = Path(folder), Path(output)
    output.mkdir(parents=True, exist_ok=True)
    listing = folder / f"assets.{channel}.json"
    names = {asset["RelativeFileName"] for asset in json.loads(listing.read_text(encoding="utf-8-sig"))}
    for name in sorted(names) + [listing.name]:
        shutil.copy2(folder / name, output / name)
        print(f"Release file: {name}")

    feed_path = folder / f"releases.{channel}.json"
    feed = json.loads(feed_path.read_text(encoding="utf-8-sig"))
    kept = [entry for entry in feed["Assets"] if entry["FileName"] in names]
    if len(kept) == len(feed["Assets"]):
        shutil.copy2(feed_path, output / feed_path.name)
    else:
        feed["Assets"] = kept
        (output / feed_path.name).write_text(json.dumps(feed, separators=(",", ":"), ensure_ascii=False), encoding="utf-8")

    # Velopack writes the legacy feed only for versions it can express.
    legacy_path = folder / f"RELEASES-{channel}"
    if legacy_path.exists():
        raw = legacy_path.read_bytes()
        bom = raw.startswith(b"\xef\xbb\xbf")
        text = raw.decode("utf-8-sig")
        newline = "\r\n" if "\r\n" in text else "\n"
        lines = text.splitlines()
        kept_lines = [line for line in lines if len(line.split(" ")) > 1 and line.split(" ")[1] in names]
        if len(kept_lines) == len(lines):
            shutil.copy2(legacy_path, output / legacy_path.name)
        else:
            (output / legacy_path.name).write_bytes((b"\xef\xbb\xbf" if bom else b"") + newline.join(kept_lines).encode())
    print(f"Feed entries: {', '.join(entry['FileName'] for entry in kept)}")


def main():
    command, args = (sys.argv[1], sys.argv[2:]) if len(sys.argv) > 1 else ("", [])
    if command == "previous" and len(args) == 5:
        try:
            previous(*args)
        except Exception as error:  # noqa: BLE001 - a missing delta must not stop the release.
            print(f"::warning::Could not fetch the previous release for a delta package: {error}")
    elif command == "collect" and len(args) == 3:
        collect(*args)
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
