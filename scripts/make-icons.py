#!/usr/bin/env python3
"""Makes every icon file from assets/icon.png, the one picture to replace to change the app's icon.

Writes assets/icon.icns for Mac and assets/icon.ico for Windows. The picture should be a square PNG
of at least 1024 by 1024 pixels; other shapes are padded to a square. If assets/icon.png is missing,
the green T is drawn so a build always has an icon. Needs a Mac (sips and iconutil).
"""
import math
import pathlib
import shutil
import struct
import subprocess
import tempfile
import zlib

ASSETS = pathlib.Path(__file__).resolve().parent.parent / "assets"
SOURCE = ASSETS / "icon.png"


def box_distance(px, py, x0, y0, x1, y1, radius):
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    qx = abs(px - cx) - ((x1 - x0) / 2 - radius)
    qy = abs(py - cy) - ((y1 - y0) / 2 - radius)
    outside = math.hypot(max(qx, 0.0), max(qy, 0.0))
    return outside + min(max(qx, qy), 0.0) - radius


def draw_t(path, n=1024):
    """The green tile with a white T."""
    unit = n / 24
    leaf, white = (74, 132, 58), (255, 255, 255)
    rows = []
    for y in range(n):
        row = bytearray([0])
        for x in range(n):
            px, py = x + 0.5, y + 0.5
            tile = box_distance(px, py, 0, 0, n, n, n * 0.235)
            alpha = min(max(0.5 - tile, 0.0), 1.0)
            bar = box_distance(px, py, 5.1 * unit, 6.1 * unit, 18.9 * unit, 8.9 * unit, 0.0)
            stem = box_distance(px, py, 10.6 * unit, 6.1 * unit, 13.4 * unit, 19.0 * unit, 0.0)
            mark = min(max(0.5 - min(bar, stem), 0.0), 1.0)
            rgb = [round(leaf[i] + (white[i] - leaf[i]) * mark) for i in range(3)]
            row += bytes(rgb + [round(alpha * 255)])
        rows.append(bytes(row))

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    header = struct.pack(">IIBBBBB", n, n, 8, 6, 0, 0, 0)
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(b"".join(rows), 9)) + chunk(b"IEND", b"")
    path.write_bytes(png)


def size_of(path):
    out = subprocess.run(["sips", "-g", "pixelWidth", "-g", "pixelHeight", str(path)], capture_output=True, text=True, check=True).stdout
    values = [int(line.split(":")[1]) for line in out.splitlines() if "pixel" in line]
    return values[0], values[1]


def scaled(source, side, out):
    subprocess.run(["sips", "-s", "format", "png", "-z", str(side), str(side), str(source), "--out", str(out)], check=True, capture_output=True)


def write_ico(source, out, work):
    """An .ico file that holds PNG pictures, which every current Windows reads."""
    images = []
    for side in (16, 24, 32, 48, 64, 128, 256):
        p = work / f"ico{side}.png"
        scaled(source, side, p)
        images.append((side, p.read_bytes()))
    offset = 6 + 16 * len(images)
    data = struct.pack("<HHH", 0, 1, len(images))
    for side, png in images:
        data += struct.pack("<BBBBHHII", side % 256, side % 256, 0, 0, 1, 32, len(png), offset)
        offset += len(png)
    out.write_bytes(data + b"".join(p for _, p in images))


def main():
    if not SOURCE.exists():
        print("assets/icon.png not found, so drawing the green T")
        draw_t(SOURCE)
    width, height = size_of(SOURCE)
    if min(width, height) < 512:
        print(f"Note: the picture is only {width}x{height}, so large icons may look soft. 1024x1024 is best.")
    with tempfile.TemporaryDirectory() as tmp:
        work = pathlib.Path(tmp)
        square = work / "square.png"
        shutil.copy(SOURCE, square)
        if width != height:
            side = max(width, height)
            print(f"The picture is {width}x{height}, so it is padded to a {side}x{side} square.")
            subprocess.run(["sips", "-p", str(side), str(side), str(square)], check=True, capture_output=True)
        iconset = work / "icon.iconset"
        iconset.mkdir()
        for side in (16, 32, 128, 256, 512):
            scaled(square, side, iconset / f"icon_{side}x{side}.png")
            scaled(square, side * 2, iconset / f"icon_{side}x{side}@2x.png")
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(ASSETS / "icon.icns")], check=True)
        write_ico(square, ASSETS / "icon.ico", work)
    print("Made assets/icon.icns and assets/icon.ico")


main()
