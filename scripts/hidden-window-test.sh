#!/bin/sh
# Proves TeXora keeps working while its main window is hidden, as it is behind a fullscreen
# audience window on one screen. Usage: scripts/hidden-window-test.sh <program>
BIN="$1"
OUT=$(mktemp -d)
hide() {
  sleep 2.5
  osascript -e "tell application \"System Events\" to set visible of (first process whose unix id is $1) to false" 2>&1
}

echo "== picture requested while hidden"
TEXORA_CONFIG="$OUT/a.json" "$BIN" --view presenter --script "w:6000" --screenshot "$OUT/a.png" > "$OUT/a.txt" 2>&1 &
PID=$!
hide $PID
for i in $(seq 1 40); do kill -0 $PID 2>/dev/null || break; sleep 0.5; done
if kill -0 $PID 2>/dev/null; then echo "HUNG after 20s"; kill $PID; else wait $PID; echo "finished, exit $?, picture: $(ls "$OUT"/a.png 2>/dev/null | wc -l | tr -d ' ')"; fi

echo "== audience open, slides advanced while the main window is hidden"
TEXORA_CONFIG="$OUT/b.json" "$BIN" --view presenter --script "w:300;k:F5;w:5000;k:ArrowRight;w:400;k:ArrowRight;w:400" --screenshot "$OUT/b.png" > "$OUT/b.txt" 2>&1 &
PID=$!
hide $PID
for i in $(seq 1 40); do kill -0 $PID 2>/dev/null || break; sleep 0.5; done
if kill -0 $PID 2>/dev/null; then echo "HUNG after 20s"; kill $PID; else wait $PID; echo "finished, exit $?: $(grep -oE 'slide=[0-9]+ ' "$OUT/b.txt") $(grep -oE 'audience=[a-z]+' "$OUT/b.txt")"; fi
