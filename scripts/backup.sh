#!/bin/sh
# Saves a dated zip of the current version into backup/:
#   source/   every file of the project that Git tracks or would track (code, assets, docs, scripts),
#             as it is now, including changes not yet committed
#   release/  this version's packages, guide and RELEASE-REPORT.txt from dist/<version>/, if built
# Build output, the toolchain, the PDF engine download and test pictures are left out; they can be
# made again. Usage: sh scripts/backup.sh
set -eu
cd "$(dirname "$0")/.."
ROOT="$PWD"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
STAMP=$(date +%Y-%m-%d_%H%M)
NAME="TeXora-$VERSION-backup-$STAMP"
STAGE=$(mktemp -d)
mkdir -p "$STAGE/$NAME/source" backup

# Source: tracked files plus new files Git does not ignore.
git ls-files -co --exclude-standard | while IFS= read -r f; do
  [ -e "$f" ] || continue
  mkdir -p "$STAGE/$NAME/source/$(dirname "$f")"
  cp -p "$f" "$STAGE/$NAME/source/$f"
done
FILES=$(find "$STAGE/$NAME/source" -type f | wc -l | tr -d ' ')

if [ -d "dist/$VERSION" ]; then
  cp -R "dist/$VERSION" "$STAGE/$NAME/release"
  RELEASE="release/: the packages, guide and report from dist/$VERSION"
else
  RELEASE="release/: not included, because dist/$VERSION has not been built (run sh scripts/release.sh)"
fi

COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo none)
CHANGED=$(git status --porcelain | wc -l | tr -d ' ')
cat > "$STAGE/$NAME/README-BACKUP.txt" <<EOF
TeXora $VERSION backup, made $(date '+%Y-%m-%d %H:%M')

source/: the complete project as it was ($FILES files). Last Git commit: $COMMIT,
  with $CHANGED changed or new files since then, all included here.
$RELEASE

To rebuild from source/:
  1. Install Rust (rustup), then in source/:  cargo build --release
     The PDF engine is downloaded by build.rs on the first build.
  2. To make all packages and the guide again:  sh scripts/release.sh
     (needs Zig and cargo-zigbuild; see README.md)
To use a package straight away, unpack the one for your system from release/.
EOF

ZIP="$ROOT/backup/$NAME.zip"
(cd "$STAGE" && zip -qry "$ZIP" "$NAME")
(cd backup && shasum -a 256 "$NAME.zip" > "$NAME.zip.sha256")
unzip -tq "$ZIP" >/dev/null
echo "Saved backup/$NAME.zip ($(du -h "$ZIP" | cut -f1 | tr -d ' '), $FILES source files)"
echo "Checked: the zip reads back without errors. Checksum in backup/$NAME.zip.sha256"
