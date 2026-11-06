#!/bin/sh
# Makes the Mac installer: a disk image holding "TeXora.app" and a link to Applications, so people
# drag the app across. Built from the zip that scripts/build-mac.sh made, then opened again to check it.
# Usage: scripts/make-dmg.sh arm64|x64
set -eu
NAME="$1"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
OUT="$PWD/dist/$VERSION"
PKG="TeXora-$VERSION-mac-$NAME"
DMG="$OUT/$PKG.dmg"
STAGE=$(mktemp -d)
ditto -x -k "$OUT/$PKG.zip" "$STAGE"
ln -s /Applications "$STAGE/Applications"
rm -f "$DMG"
hdiutil create -quiet -volname "TeXora" -srcfolder "$STAGE" -fs HFS+ -format UDZO -ov "$DMG"
# The disk image is signed ad hoc too, like the app inside it.
codesign --force --sign - "$DMG"
codesign --verify "$DMG"

# Open the image the way a person would, and check the app inside runs.
MOUNT=$(mktemp -d)
hdiutil attach -quiet -nobrowse -readonly -mountpoint "$MOUNT" "$DMG"
APP="$MOUNT/TeXora.app"
OK=0
if [ -x "$APP/Contents/MacOS/TeXora" ] && [ -L "$MOUNT/Applications" ]; then
  case "$NAME" in x64) RUN="arch -x86_64" ;; *) RUN="" ;; esac
  if $RUN "$APP/Contents/MacOS/TeXora" --version | grep -q "TeXora $VERSION"; then OK=1; fi
fi
hdiutil detach -quiet "$MOUNT"
if [ "$OK" = 1 ]; then
  echo "Installer: $PKG.dmg opens, holds TeXora.app and an Applications link, and the app runs; signed ad hoc"
  echo "Installer: $PKG.dmg checked, signed ad hoc" >> "$OUT/$PKG.validation.txt"
else
  echo "FAILED: $PKG.dmg is not right" >&2
  exit 1
fi
