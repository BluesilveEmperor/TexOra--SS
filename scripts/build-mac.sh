#!/bin/sh
# Builds "TeXora.app" for one Mac type, then proves the packaged copy works.
# Usage: scripts/build-mac.sh aarch64|x86_64
set -eu
ARCH="$1"
case "$ARCH" in
  aarch64) NAME=arm64 ; TARGET=aarch64-apple-darwin ; RUN="" ;;
  x86_64) NAME=x64 ; TARGET=x86_64-apple-darwin ; RUN="arch -x86_64" ;;
  *) echo "Unknown Mac type: $ARCH" >&2 ; exit 2 ;;
esac
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
OUT="$PWD/dist/$VERSION"
PKG="TeXora-$VERSION-mac-$NAME"
LOG="$OUT/$PKG.validation.txt"
mkdir -p "$OUT"

cargo build --release --locked --target "$TARGET"

STAGE=$(mktemp -d)
APP="$STAGE/TeXora.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "target/$TARGET/release/TeXora" "$APP/Contents/MacOS/TeXora"
cp "target/$TARGET/release/libpdfium.dylib" "$APP/Contents/MacOS/libpdfium.dylib"
cp README.md README.en.md CREDITS.md THIRD-PARTY-LICENSES.md LICENSE "$APP/Contents/Resources/"
cp assets/icon.icns "$APP/Contents/Resources/TeXora.icns"
cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>TeXora</string>
  <key>CFBundleDisplayName</key><string>TeXora</string>
  <key>CFBundleExecutable</key><string>TeXora</string>
  <key>CFBundleIconFile</key><string>TeXora</string>
  <key>CFBundleIdentifier</key><string>org.texora.TeXora</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>CFBundleDocumentTypes</key>
  <array>
    <dict>
      <key>CFBundleTypeName</key><string>Slides</string>
      <key>CFBundleTypeRole</key><string>Viewer</string>
      <key>LSItemContentTypes</key><array><string>com.adobe.pdf</string></array>
    </dict>
  </array>
</dict>
</plist>
EOF
# Signed ad hoc (no paid Apple Developer ID): the PDF engine first, then the app around it.
codesign --force --sign - "$APP/Contents/MacOS/libpdfium.dylib"
codesign --force --deep --sign - "$APP"
codesign --verify --deep --strict "$APP"
ditto -c -k --keepParent "$APP" "$OUT/$PKG.zip"

# Validate the copy a person would download, unpacked somewhere else.
CHECK=$(mktemp -d)
ditto -x -k "$OUT/$PKG.zip" "$CHECK"
BIN="$CHECK/TeXora.app/Contents/MacOS/TeXora"
{
  echo "Validated by: running the package on this Mac ($(file -b "$BIN" | cut -d' ' -f3-))"
  echo "Signature: $(codesign -dv "$CHECK/TeXora.app" 2>&1 | sed -n 's/^Signature=//p'), checked with codesign --verify --deep --strict"
  codesign --verify --deep --strict "$CHECK/TeXora.app" || { echo "FAILED: signature"; exit 1; }
  echo "App icon: $(iconutil -c iconset "$CHECK/TeXora.app/Contents/Resources/TeXora.icns" -o "$CHECK/shown.iconset" && ls "$CHECK/shown.iconset" | wc -l | tr -d ' ') sizes"
  # People using TeXora install nothing: the program may only need macOS itself.
  if ! python3 scripts/list-system-libs.py "$BIN" --only-os; then echo "FAILED: needs libraries outside macOS"; exit 1; fi
  $RUN "$BIN" --version
  $RUN "$BIN" --self-test
  echo "--- window ---"
  TEXORA_CONFIG="$CHECK/settings.json" $RUN "$BIN" --view presenter --screenshot "$CHECK/window.png" >/dev/null
  sips -g pixelWidth -g pixelHeight "$CHECK/window.png" | tail -2
  echo "RESULT PASS window"
} 2>&1 | tee "$LOG"
cp "$CHECK/window.png" "$OUT/$PKG.window.png"
grep -q 'RESULT PASS window' "$LOG"
