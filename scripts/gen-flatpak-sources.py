#!/usr/bin/env python3
"""Regenerate packaging/flatpak/cargo-sources.json from Cargo.lock.

Usage: scripts/gen-flatpak-sources.py [--check]
Exit codes: 0 ok / up to date, 1 stale (--check only), 2 bad input.
Registry crates only; git sources are pinned in the manifest.
"""
import json
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "packaging" / "flatpak" / "cargo-sources.json"
CONFIG = ('[source.vendored-sources]\ndirectory = "cargo/vendor"\n\n'
          '[source.crates-io]\nreplace-with = "vendored-sources"\n')


def build():
    lock = tomllib.loads((ROOT / "Cargo.lock").read_text(encoding="utf-8"))
    out = []
    pkgs = [p for p in lock["package"] if p.get("source", "").startswith("registry+")]
    for p in sorted(pkgs, key=lambda p: (p["name"], p["version"])):
        name, ver, sha = p["name"], p["version"], p["checksum"]
        dest = f"cargo/vendor/{name}-{ver}"
        out.append({"type": "archive", "archive-type": "tar-gzip",
                    "url": f"https://static.crates.io/crates/{name}/{name}-{ver}.crate",
                    "sha256": sha, "dest": dest})
        out.append({"type": "inline",
                    "contents": json.dumps({"package": sha, "files": {}}),
                    "dest": dest, "dest-filename": ".cargo-checksum.json"})
    out.append({"type": "inline", "contents": CONFIG, "dest": "cargo",
                "dest-filename": "config"})
    return json.dumps(out, indent=4) + "\n"


def main(argv):
    if argv not in ([], ["--check"]):
        print(__doc__, file=sys.stderr)
        return 0 if argv in (["-h"], ["--help"]) else 2
    new = build()
    if argv == ["--check"]:
        if OUT.read_text(encoding="utf-8") != new:
            print(f"{OUT} is stale; run scripts/gen-flatpak-sources.py", file=sys.stderr)
            return 1
        return 0
    OUT.write_text(new, encoding="utf-8")
    return 0


sys.exit(main(sys.argv[1:]))
