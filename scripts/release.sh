#!/bin/sh
# Run this after every new version: scripts/release.sh
# Tests the code, builds all five packages and the installers (Mac disk images, Linux .deb files) into
# dist/<version>/, and writes an honest report of what was actually run and what was only built.
# The Windows installer needs Windows, so only .github/workflows/release.yml makes it.
set -eu
cd "$(dirname "$0")/.."
[ -f .toolchain/env.sh ] && . ./.toolchain/env.sh
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
OUT="dist/$VERSION"
mkdir -p "$OUT"

echo "== Making icons from assets/icon.png"
python3 scripts/make-icons.py
echo "== No video decoder or codec inside, and the licence list up to date"
python3 scripts/third-party.py
echo "== Testing the code"
cargo test --locked
echo "== Keeps working while hidden"
cargo build --locked
HIDDEN=$(sh scripts/hidden-window-test.sh ./target/debug/TeXora 2>&1)
echo "$HIDDEN"
if echo "$HIDDEN" | grep -q 'HUNG'; then
  echo "FAILED: TeXora stopped while its window was hidden"
  exit 1
fi
echo "== Mac (Apple)"
sh scripts/build-mac.sh aarch64
echo "== Mac (Intel, run through Rosetta)"
sh scripts/build-mac.sh x86_64
echo "== Mac disk images"
sh scripts/make-dmg.sh arm64
sh scripts/make-dmg.sh x64
for kind in linux-x64 linux-arm64 windows-x64; do
  echo "== $kind"
  sh scripts/build-cross.sh "$kind"
done
echo "== Linux .deb installers"
python3 scripts/make-deb.py linux-x64
python3 scripts/make-deb.py linux-arm64

echo "== No ffmpeg, VLC or codec library in any package"
sh scripts/check-no-codecs.sh "$OUT"

(cd "$OUT" && shasum -a 256 TeXora-*.zip TeXora-*.tar.gz TeXora-*.dmg texora_*.deb > SHA256SUMS.txt)


REPORT="$OUT/RELEASE-REPORT.txt"
{
  echo "TeXora $VERSION"
  echo
  for f in "$OUT"/TeXora-*.validation.txt; do
    name=$(basename "$f" .validation.txt)
    if grep -q '^RESULT PASS$' "$f" && grep -q 'RESULT PASS window' "$f"; then
      echo "RUN AND PASSED   $name: all self-test checks and a real window"
    elif grep -q '^RESULT BUILT' "$f"; then
      echo "BUILT, NOT RUN   $name: inspected only; the GitHub workflow runs it on a real machine"
    else
      echo "FAILED           $name"
    fi
  done
  echo
  echo "Installers made here:"
  (cd "$OUT" && ls TeXora-*.dmg texora_*.deb) | sed 's/^/  /'
  echo "  (Windows installer: made and tested by the GitHub workflow)"
} > "$REPORT"
cat "$REPORT"
if grep -q '^FAILED' "$REPORT"; then
  exit 1
fi

# Every version that passes is also saved in backup/: source, packages and this report.
echo "== Backup"
BACKUP=$(sh scripts/backup.sh)
echo "$BACKUP"
echo "Backup: $(echo "$BACKUP" | head -1 | sed 's/^Saved //')" >> "$REPORT"
