#!/usr/bin/env python3
"""Writes THIRD-PARTY-LICENSES.md: every library built into the program, with its version, licence and
home page, read from Cargo's own records (the libraries the program uses at run time, on any of Mac,
Linux and Windows). Also checks that none of them is a video decoder or codec: TeXora contains none,
and plays MP4 through VLC or ffmpeg installed on the person's computer.

Usage, from the project folder:  python3 scripts/third-party.py [--check]
--check only checks for codecs and exits 1 if one is found.
"""
import json
import pathlib
import subprocess
import sys

# Names that must never be inside TeXora: video decoders, encoders and players.
FORBIDDEN = ("openh264", "ffmpeg", "x264", "x265", "libvlc", "vlc", "gstreamer", "dav1d", "libde265", "mp4", "avcodec", "rav1e", "vpx")

meta = json.loads(subprocess.run(["cargo", "metadata", "--format-version", "1", "--locked"], capture_output=True, text=True, check=True).stdout)
packages = {p["id"]: p for p in meta["packages"]}
nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
root = meta["resolve"]["root"]

# Libraries reached through normal dependencies only (not build tools or test helpers).
seen, todo = set(), [root]
while todo:
    pid = todo.pop()
    if pid in seen:
        continue
    seen.add(pid)
    for d in nodes[pid]["deps"]:
        if any(k["kind"] is None for k in d["dep_kinds"]):
            todo.append(d["pkg"])
seen.discard(root)

libs = sorted((packages[i] for i in seen), key=lambda p: (p["name"].lower(), p["version"]))
bad = [p["name"] for p in libs if any(f == p["name"].lower() or p["name"].lower().startswith(f + "-") or p["name"].lower().startswith(f + "_") for f in FORBIDDEN)]
if bad:
    print("STOP: video decoder or codec libraries found: " + ", ".join(sorted(set(bad))))
    sys.exit(1)
if "--check" in sys.argv:
    print(f"OK: {len(libs)} libraries, no video decoder or codec among them.")
    sys.exit(0)

name = packages[root]["name"]
app = "TeXora"
lines = [
    "# Third-party licences",
    "",
    f"{app} is built from the libraries below; each keeps its own licence. This list is made by",
    "`scripts/third-party.py` from Cargo's records, so it matches the program exactly.",
    "",
    "## Not included",
    "",
    f"{app} contains **no video decoder, encoder or player**: no H.264 or other codec, no ffmpeg, no VLC",
    "or libvlc. GIFs are decoded by the `image` library. **MP4 playback uses VLC or ffmpeg installed on",
    f"your computer**, started as separate programs; {app} never downloads, installs, ships or updates them.",
    "ffmpeg and VLC have their own licences (LGPL/GPL), which apply to the copies you install.",
    "",
    "## Shipped beside the program",
    "",
    "| Component | Licence | Source |",
    "|---|---|---|",
    "| PDFium (prebuilt by bblanchon/pdfium-binaries) | BSD-3-Clause and Apache-2.0 | https://github.com/bblanchon/pdfium-binaries |",
    "| Inter font (Rasmus Andersson) | SIL Open Font License 1.1 | https://rsms.me/inter/ |",
    "| Hack and Ubuntu Light fonts (inside egui) | MIT; Ubuntu Font Licence | https://github.com/emilk/egui |",
    "",
    f"## Rust libraries built into the program ({len(libs)})",
    "",
    "| Library | Version | Licence | Source |",
    "|---|---|---|---|",
]
for p in libs:
    lic = (p.get("license") or p.get("license_file") or "see source").replace("|", "/")
    src = p.get("repository") or p.get("homepage") or f"https://crates.io/crates/{p['name']}"
    lines.append(f"| {p['name']} | {p['version']} | {lic} | {src} |")
pathlib.Path("THIRD-PARTY-LICENSES.md").write_text("\n".join(lines) + "\n")
print(f"Wrote THIRD-PARTY-LICENSES.md: {len(libs)} libraries, no video decoder or codec.")
