#!/usr/bin/env python3
"""Confirms a Windows program really carries an icon, by reading its resource table.
A Mac cannot run the program, but this part can be checked without running it."""
import struct
import sys

RT_ICON, RT_GROUP_ICON = 3, 14
data = open(sys.argv[1], "rb").read()
pe = struct.unpack_from("<I", data, 0x3C)[0]
sections = struct.unpack_from("<H", data, pe + 6)[0]
optional = struct.unpack_from("<H", data, pe + 20)[0]
table = pe + 24 + optional
base = None
for i in range(sections):
    at = table + i * 40
    if data[at:at + 8].rstrip(b"\0") == b".rsrc":
        base = struct.unpack_from("<I", data, at + 20)[0]
if base is None:
    sys.exit("App icon: MISSING, the program has no resources")
named, numbered = struct.unpack_from("<HH", data, base + 12)
kinds = [struct.unpack_from("<I", data, base + 16 + i * 8)[0] for i in range(named + numbered)]
if RT_ICON not in kinds or RT_GROUP_ICON not in kinds:
    sys.exit("App icon: MISSING from the program's resources")
print("App icon: inside the program")
