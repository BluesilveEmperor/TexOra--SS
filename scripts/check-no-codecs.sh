#!/bin/sh
# Checks the packages of one version for any video decoder, ffmpeg, VLC or codec library, and stops if
# one is found. TeXora ships none of these: MP4 playback uses VLC or ffmpeg installed on the computer.
# Two checks: no such file in any package, and no such code built into any file (a decoder compiled
# into the program leaves its name inside it, as OpenH264 did). Sample videos inside the program are
# files, not decoders, so the name x264 writes into every video it makes is not searched for.
# Usage: scripts/check-no-codecs.sh dist/<version>
set -eu
OUT="$1"
WORK=$(mktemp -d)
for f in "$OUT"/*.zip; do [ -e "$f" ] && unzip -q -o "$f" -d "$WORK/$(basename "$f")"; done
for f in "$OUT"/*.tar.gz; do [ -e "$f" ] && mkdir -p "$WORK/$(basename "$f")" && tar -xzf "$f" -C "$WORK/$(basename "$f")"; done
FILES=$(find "$WORK" -type f | wc -l | tr -d ' ')
NAMES=$(find "$WORK" -type f | grep -iE '/(lib)?(ffmpeg|ffprobe|avcodec|avformat|avutil|swscale|vlc|libvlc|libvlccore|openh264|x264|x265|dav1d|de265|vpx)[^/]*$' || true)
# The guide and licence pages may mention these names in words; only programs and libraries are searched.
INSIDE=$(find "$WORK" -type f ! -name '*.md' ! -name '*.html' ! -name '*.js' ! -name '*.txt' ! -name '*.css' ! -name '*.json' ! -name '*.svg' -exec grep -l -a -E 'OpenH264|WelsDecoder|WelsCreateDecoder|libavcodec|avcodec_send_packet|libvlc_new' {} + 2>/dev/null || true)
rm -rf "$WORK"
if [ -n "$NAMES$INSIDE" ]; then
  echo "STOP: a video decoder or codec is in the packages of $OUT:"
  [ -n "$NAMES" ] && echo "$NAMES" | sed "s|$WORK/||"
  [ -n "$INSIDE" ] && echo "$INSIDE" | sed "s|$WORK/||; s|$| (built in)|"
  exit 1
fi
echo "OK: $FILES files in the packages of $OUT; no ffmpeg, VLC, decoder or codec library, as a file or built in."
