#!/bin/sh
# Runs inside a clean Linux system that holds only the finished package. Unpacks it as a person
# would and proves it works, including a real window on a virtual screen.
set -eu
PACKAGE=$(ls /out/*.tar.gz)
P=$(basename "$PACKAGE" .tar.gz)
LOG="/out/$P.validation.txt"
RUN=$(mktemp -d)
tar -C "$RUN" -xzf "$PACKAGE"
BIN="$RUN/$P/TeXora"
{
  echo "Validated by: running the package in a clean Linux system ($(uname -m))"
  "$BIN" --version
  "$BIN" --self-test
  echo "--- window on a virtual screen ---"
  cd "$RUN"
  TEXORA_CONFIG="$RUN/settings.json" LIBGL_ALWAYS_SOFTWARE=1 \
    timeout 300 xvfb-run -a -s "-screen 0 1500x1000x24 +extension GLX" \
    "$BIN" --view presenter --screenshot "$RUN/window.png" >/dev/null
  python3 - "$RUN/window.png" <<'PY'
import struct, sys, zlib
b = open(sys.argv[1], "rb").read()
w, h = struct.unpack(">II", b[16:24])
data, i = b"", 8
while i < len(b):
    n = struct.unpack(">I", b[i:i+4])[0]
    if b[i+4:i+8] == b"IDAT":
        data += b[i+8:i+8+n]
    i += 12 + n
raw = zlib.decompress(data)
distinct = len(set(raw[j:j+4] for j in range(0, min(len(raw), 4000000), 4)))
print(f"Window picture: {w}x{h}, {distinct} different values")
sys.exit(0 if w > 500 and distinct > 50 else 1)
PY
  echo "RESULT PASS window"
} 2>&1 | tee "$LOG"
cp "$RUN/window.png" "/out/$P.window.png"
grep -q 'RESULT PASS window' "$LOG"
grep -q '^RESULT PASS$' "$LOG"
