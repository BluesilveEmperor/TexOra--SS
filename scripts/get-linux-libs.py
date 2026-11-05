#!/usr/bin/env python3
"""Fetches Debian's libxcb for each Linux target, checking it against Debian's published list.

Cross-building for Linux needs this one library at the final step. On a person's Linux computer
TeXora uses their own copy, so it is not shipped in the package.
Usage: get-linux-libs.py <output folder>
"""
import hashlib
import lzma
import pathlib
import subprocess
import sys
import tempfile
import urllib.request

MIRROR = "https://deb.debian.org/debian"
SUITE = "bookworm"
out = pathlib.Path(sys.argv[1])


def fetch(url):
    with urllib.request.urlopen(url, timeout=120) as r:
        return r.read()


for arch, folder in (("amd64", "x86_64"), ("arm64", "aarch64")):
    dest = out / folder / "libxcb.so"
    if dest.exists():
        continue
    packages = lzma.decompress(fetch(f"{MIRROR}/dists/{SUITE}/main/binary-{arch}/Packages.xz")).decode()
    entry = next(e for e in packages.split("\n\n") if e.startswith("Package: libxcb1\n"))
    fields = dict(line.split(": ", 1) for line in entry.splitlines() if ": " in line)
    deb = fetch(f"{MIRROR}/{fields['Filename']}")
    digest = hashlib.sha256(deb).hexdigest()
    if digest != fields["SHA256"]:
        sys.exit(f"Checksum mismatch for {fields['Filename']}")
    with tempfile.TemporaryDirectory() as tmp:
        t = pathlib.Path(tmp)
        (t / "p.deb").write_bytes(deb)
        subprocess.run(["ar", "x", "p.deb"], cwd=t, check=True)
        data = next(t.glob("data.tar.*"))
        subprocess.run(["tar", "-xf", data.name], cwd=t, check=True)
        lib = next(p for p in t.rglob("libxcb.so.1*") if p.is_file() and not p.is_symlink())
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(lib.read_bytes())
    print(f"{arch}: libxcb {fields['Version']} verified")
