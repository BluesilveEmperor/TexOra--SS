#!/usr/bin/env python3
"""Makes the Linux installer: a .deb for Debian, Ubuntu and their relatives, from the .tar.gz package
that scripts/build-cross.sh made. Pure Python, so it runs on a Mac too.

Installs to /opt/texora, with `texora` on the PATH, a menu entry and an icon.
Usage: python3 scripts/make-deb.py linux-x64|linux-arm64
"""
import gzip
import io
import pathlib
import re
import sys
import tarfile
import time

ROOT = pathlib.Path(__file__).resolve().parent.parent
KIND = sys.argv[1]
ARCH = {"linux-x64": "amd64", "linux-arm64": "arm64"}[KIND]
VERSION = re.search(r'^version = "(.*)"', (ROOT / "Cargo.toml").read_text(), re.M).group(1)
OUT = ROOT / "dist" / VERSION
PKG = f"TeXora-{VERSION}-{KIND}"
NOW = int(time.time())


def add(tar, name, data, mode=0o644):
    info = tarfile.TarInfo(name)
    info.size, info.mode, info.mtime = len(data), mode, NOW
    info.uid = info.gid = 0
    info.uname = info.gname = "root"
    tar.addfile(info, io.BytesIO(data))


def add_dir(tar, name):
    info = tarfile.TarInfo(name)
    info.type, info.mode, info.mtime = tarfile.DIRTYPE, 0o755, NOW
    info.uid = info.gid = 0
    info.uname = info.gname = "root"
    tar.addfile(info)


def add_link(tar, name, target):
    info = tarfile.TarInfo(name)
    info.type, info.linkname, info.mode, info.mtime = tarfile.SYMTYPE, target, 0o777, NOW
    info.uid = info.gid = 0
    info.uname = info.gname = "root"
    tar.addfile(info)


def targz(fill):
    raw = io.BytesIO()
    with tarfile.open(fileobj=raw, mode="w", format=tarfile.GNU_FORMAT) as tar:
        fill(tar)
    return gzip.compress(raw.getvalue(), mtime=NOW)


def ar_member(name, data):
    # Debian's format: names padded with spaces and no "/" at the end, as dpkg requires.
    header = f"{name:<16}{NOW:<12}{0:<6}{0:<6}{'100644':<8}{len(data):<10}`\n".encode()
    assert len(header) == 60
    return header + data + (b"\n" if len(data) % 2 else b"")


def main():
    files = {}
    with tarfile.open(OUT / f"{PKG}.tar.gz") as src:
        for m in src.getmembers():
            # Skip any hidden macOS "._" metadata files.
            if m.isfile() and not pathlib.PurePosixPath(m.name).name.startswith("._"):
                files[pathlib.PurePosixPath(m.name).name] = (src.extractfile(m).read(), m.mode)
    for need in ("TeXora", "libpdfium.so", "TeXora.png"):
        if need not in files:
            sys.exit(f"{PKG}.tar.gz has no {need}")

    desktop = """[Desktop Entry]
Type=Application
Name=TeXora
GenericName=Presentation presenter
Comment=Present PDF and LaTeX Beamer slides
Exec=/opt/texora/TeXora %f
Icon=texora
Terminal=false
Categories=Office;Presentation;Education;
MimeType=application/pdf;text/x-tex;
""".encode()

    def data(tar):
        for d in ["./opt", "./opt/texora", "./usr", "./usr/bin", "./usr/share", "./usr/share/applications",
                  "./usr/share/pixmaps", "./usr/share/doc", "./usr/share/doc/texora"]:
            add_dir(tar, d)
        for name, (body, mode) in sorted(files.items()):
            if name.endswith(".desktop"):
                continue
            executable = name == "TeXora" or bool(mode & 0o111)
            add(tar, f"./opt/texora/{name}", body, 0o755 if executable else 0o644)
        add_link(tar, "./usr/bin/texora", "/opt/texora/TeXora")
        add(tar, "./usr/share/applications/texora.desktop", desktop)
        add(tar, "./usr/share/pixmaps/texora.png", files["TeXora.png"][0])
        add(tar, "./usr/share/doc/texora/copyright", (ROOT / "LICENSE").read_bytes())

    data_tgz = targz(data)
    size_kb = sum(len(b) for b, _ in files.values()) // 1024 + 1
    control = f"""Package: texora
Version: {VERSION}
Architecture: {ARCH}
Maintainer: Khalid Alkhaldi <https://github.com/BluesilveEmperor/TexOra--SS>
Installed-Size: {size_kb}
Depends: libc6 (>= 2.31), libxcb1
Recommends: libgl1, libxkbcommon0, xdg-desktop-portal
Suggests: texlive-latex-extra
Section: misc
Priority: optional
Homepage: https://github.com/BluesilveEmperor/TexOra--SS
Description: Present PDF and LaTeX Beamer slides
 TeXora shows your notes, the current and next slide and the time,
 while the audience sees only the slide. It compiles LaTeX Beamer source
 and rebuilds the slides each time the file is saved.
""".encode()
    control_tgz = targz(lambda tar: add(tar, "./control", control))

    deb = OUT / f"texora_{VERSION}_{ARCH}.deb"
    deb.write_bytes(b"!<arch>\n" + ar_member("debian-binary", b"2.0\n") + ar_member("control.tar.gz", control_tgz) + ar_member("data.tar.gz", data_tgz))
    print(f"Installer: {deb.name} ({deb.stat().st_size // 1024} KB)")
    with open(OUT / f"{PKG}.validation.txt", "a") as log:
        log.write(f"Installer: {deb.name} made\n")


main()
