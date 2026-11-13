#!/bin/sh
# Builds the Linux and Windows packages, from a Mac or (for Linux) on Linux itself. Each one is inspected
# here and labelled "built, not run"; .github/workflows/release.yml then runs them on real machines.
# Usage: scripts/build-cross.sh linux-x64|linux-arm64|windows-x64
set -eu
KIND="$1"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
OUT="$PWD/dist/$VERSION"
P="TeXora-$VERSION-$KIND"
LOG="$OUT/$P.validation.txt"
STAGE=$(mktemp -d)
mkdir -p "$OUT" "$STAGE/$P"
export PATH="$PWD/.toolchain/zig:$PATH"

case "$KIND" in
  linux-x64) TRIPLE=x86_64-unknown-linux-gnu ; ARCH=x86_64 ; WANT="ELF 64-bit.*x86-64" ;;
  linux-arm64) TRIPLE=aarch64-unknown-linux-gnu ; ARCH=aarch64 ; WANT="ELF 64-bit.*aarch64|ELF 64-bit.*ARM aarch64" ;;
  windows-x64) TRIPLE=x86_64-pc-windows-gnu ; ARCH=x86_64 ; WANT="PE32\+ executable.*x86-64" ;;
  *) echo "Unknown kind: $KIND" >&2 ; exit 2 ;;
esac

case "$KIND" in
  linux-*)
    python3 scripts/get-linux-libs.py .toolchain/linux-libs
    # glibc 2.31 means it also runs on Linux systems several years old.
    CARGO_ENCODED_RUSTFLAGS="-Lnative=$PWD/.toolchain/linux-libs/$ARCH" cargo zigbuild --release --locked --target "$TRIPLE.2.31"
    cp "target/$TRIPLE/release/TeXora" "target/$TRIPLE/release/libpdfium.so" README.md README.en.md CREDITS.md THIRD-PARTY-LICENSES.md LICENSE assets/TeXora.desktop "$STAGE/$P/"
    cp assets/icon.png "$STAGE/$P/TeXora.png"
    # No hidden macOS "._" metadata files in a Linux package.
    COPYFILE_DISABLE=1 tar -C "$STAGE" -czf "$OUT/$P.tar.gz" "$P"
    BIN="$STAGE/$P/TeXora" ; LIB="$STAGE/$P/libpdfium.so" ; ARCHIVE="$P.tar.gz"
    ;;
  windows-*)
    cargo zigbuild --release --locked --target "$TRIPLE"
    cp "target/$TRIPLE/release/TeXora.exe" "target/$TRIPLE/release/pdfium.dll" README.md README.en.md CREDITS.md THIRD-PARTY-LICENSES.md LICENSE "$STAGE/$P/"
    # No hidden macOS "._" metadata files in a Windows package.
    (cd "$STAGE" && ditto -c -k --norsrc --noextattr --noqtn --keepParent "$P" "$OUT/$P.zip")
    BIN="$STAGE/$P/TeXora.exe" ; LIB="$STAGE/$P/pdfium.dll" ; ARCHIVE="$P.zip"
    ;;
esac

{
  echo "Validated by: inspecting the package on $(uname -s) $(uname -m). BUILT, NOT RUN HERE."
  echo "Program: $(file -b "$BIN")"
  echo "PDF engine: $(file -b "$LIB")"
  # People using TeXora install nothing: the program and its PDF engine may only need the system itself.
  python3 scripts/list-system-libs.py "$BIN" --only-os
  python3 scripts/list-system-libs.py "$LIB" --only-os > "$STAGE/engine-libs.txt"
  sed 's/^/PDF engine /' "$STAGE/engine-libs.txt"
  file -b "$BIN" | grep -Eq "$WANT"
  echo "Version inside: $(strings "$BIN" | grep -m1 -E '^TeXora: a LaTeX|TeXora/' || true) $VERSION"
  strings "$BIN" | grep -q 'About TeXora' && echo "Contains the TeXora interface: yes"
  case "$KIND" in
    windows-*) python3 scripts/check-exe-icon.py "$BIN" ;;
    *) echo "App icon: $(file -b "$STAGE/$P/TeXora.png" | cut -d, -f1-2)" ;;
  esac
  echo "RESULT BUILT"
} 2>&1 | tee "$LOG"
grep -q 'RESULT BUILT' "$LOG"
echo "Packaged $ARCHIVE"
