#!/usr/bin/env python3
"""Lists the libraries a built TeXora program needs from the system it runs on.

Usage: python3 scripts/list-system-libs.py <program> [--only-os]

Works on macOS (Mach-O, through otool), Linux (ELF) and Windows (PE) programs, from a Mac.
With --only-os it fails (exit 1) if the program needs anything outside the operating system itself:
macOS's /System and /usr/lib, the C library and display libraries every Linux desktop has, or
Windows's own DLLs. Libraries TeXora loads later (the PDF engine beside it) are not linked, so they
do not appear here.
"""
import struct
import subprocess
import sys

LINUX_OS = {"libc.so.6", "libm.so.6", "libpthread.so.0", "libdl.so.2", "librt.so.1", "libgcc_s.so.1",
            "ld-linux-x86-64.so.2", "ld-linux-aarch64.so.1", "libxcb.so.1"}


def elf_needed(b):
    is64 = b[4] == 2
    if not is64:
        raise SystemExit("32-bit programs are not expected")
    shoff = struct.unpack_from("<Q", b, 0x28)[0]
    shentsize, shnum = struct.unpack_from("<HH", b, 0x3A)
    secs = [struct.unpack_from("<IIQQQQIIQQ", b, shoff + i * shentsize) for i in range(shnum)]
    dyn = [s for s in secs if s[1] == 6]
    if not dyn:
        return []
    dyn = dyn[0]
    dynstr = secs[dyn[6]]
    out = []
    for i in range(0, dyn[5], 16):
        tag, val = struct.unpack_from("<qQ", b, dyn[4] + i)
        if tag == 0:
            break
        if tag == 1:
            o = dynstr[4] + val
            out.append(b[o:b.index(b"\0", o)].decode())
    return out


def pe_imports(b):
    pe = struct.unpack_from("<I", b, 0x3C)[0]
    nsec = struct.unpack_from("<H", b, pe + 6)[0]
    optsz = struct.unpack_from("<H", b, pe + 20)[0]
    opt = pe + 24
    magic = struct.unpack_from("<H", b, opt)[0]
    dirs = opt + (112 if magic == 0x20B else 96)
    imp_rva = struct.unpack_from("<I", b, dirs + 8)[0]
    secs = [struct.unpack_from("<8sIIII", b, opt + optsz + i * 40) for i in range(nsec)]

    def off(rva):
        for _, vs, va, rs, rp in secs:
            if va <= rva < va + max(vs, rs):
                return rva - va + rp
        raise ValueError("address outside the file")

    o, names = off(imp_rva), []
    while True:
        e = struct.unpack_from("<IIIII", b, o)
        if e[3] == 0:
            break
        n = off(e[3])
        names.append(b[n:b.index(b"\0", n)].decode())
        o += 20
    return list(dict.fromkeys(names))


def windows_os(name):
    n = name.lower()
    return n.startswith("api-ms-win-") or n in {
        "advapi32.dll", "bcrypt.dll", "bcryptprimitives.dll", "combase.dll", "dwmapi.dll", "dwrite.dll", "gdi32.dll",
        "imm32.dll", "kernel32.dll", "ntdll.dll", "ole32.dll", "oleaut32.dll", "opengl32.dll",
        "shell32.dll", "shlwapi.dll", "user32.dll", "uxtheme.dll", "ws2_32.dll", "comctl32.dll",
        "comdlg32.dll", "crypt32.dll", "secur32.dll", "userenv.dll", "version.dll", "winmm.dll", "msvcrt.dll"}


def main():
    path, only_os = sys.argv[1], "--only-os" in sys.argv
    b = open(path, "rb").read()
    if b[:4] == b"\x7fELF":
        libs, kind = elf_needed(b), "linux"
        outside = [l for l in libs if l not in LINUX_OS]
    elif b[:2] == b"MZ":
        libs, kind = pe_imports(b), "windows"
        outside = [l for l in libs if not windows_os(l)]
    else:
        out = subprocess.run(["otool", "-L", path], capture_output=True, text=True, check=True).stdout
        libs = [l.strip().split(" ")[0] for l in out.splitlines()[1:] if l.strip()]
        kind = "mac"
        outside = [l for l in libs if not (l.startswith("/System/") or l.startswith("/usr/lib/"))]
    short = sorted({l.rsplit("/", 1)[-1] for l in libs})
    print(f"Needs from the system ({kind}): " + ", ".join(short))
    if outside:
        print("Outside the operating system: " + ", ".join(outside))
        if only_os:
            sys.exit(1)
    else:
        print("Outside the operating system: nothing")


main()
