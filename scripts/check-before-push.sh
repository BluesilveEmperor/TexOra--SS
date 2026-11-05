#!/bin/sh
# Run this before every push to GitHub. It checks that this folder is the free TeXora project, that it
# goes to khalidt/TeXora, and that what Git would publish holds only TeXora's own files: no build output,
# no packages, no paid-edition code and no private paths or keys.
# Usage, from this folder:  sh scripts/check-before-push.sh
set -u
cd "$(dirname "$0")/.."
FAIL=0
bad() { echo "STOP: $1"; FAIL=1; }

[ -f Cargo.toml ] && grep -q '^name = "texora"' Cargo.toml || bad "this is not the free TeXora folder (Cargo.toml name is not texora)."
[ -d .git ] || bad "this folder is not its own Git project."
URL=$(git remote get-url origin 2>/dev/null || echo none)
case "$URL" in
  https://github.com/khalidt/TeXora.git|https://github.com/khalidt/TeXora|git@github.com:khalidt/TeXora.git) ;;
  *) bad "the GitHub address is '$URL', not khalidt/TeXora." ;;
esac

# Every file Git would publish: what is already saved in Git plus new files it does not ignore.
LIST=$(mktemp)
{ git ls-files; git ls-files -o --exclude-standard; } | sort -u > "$LIST"
COUNT=$(wc -l < "$LIST" | tr -d ' ')

# Folders that must never be published.
if grep -E '^(target|\.toolchain|dist|vendor|backup|shots)(/|$)' "$LIST" >/dev/null; then
  bad "build output, packages, backups or tools would be published:"
  grep -E '^(target|\.toolchain|dist|vendor|backup|shots)(/|$)' "$LIST" | head -5
fi
# Paid-edition parts.
for f in src/ai.rs src/brains.rs src/ui_studio.rs docs/book.toml; do
  grep -qx "$f" "$LIST" && bad "a TeXora Pro file would be published: $f"
done
# Names, this computer's own folders and keys inside the files.
ME=$(id -un)
while IFS= read -r f; do
  [ -f "$f" ] || continue
  [ "$f" = scripts/check-before-push.sh ] && continue
  if grep -IqE "TeXora Pro|TeXoraPro|TeXora[ -]Free|texora-free|/Users/$ME/|/home/$ME/|sk-ant-[A-Za-z0-9]|AIza[0-9A-Za-z_-]{20}|sk-[A-Za-z0-9]{32}" "$f"; then
    bad "$f mentions TeXora Pro, the old Free name, your own computer folder or a key."
  fi
  SIZE=$(wc -c < "$f" | tr -d ' ')
  [ "$SIZE" -gt 20000000 ] && bad "$f is larger than 20 MB."
done < "$LIST"
rm -f "$LIST"

if [ "$FAIL" = 0 ]; then
  echo "OK: $COUNT files of the free TeXora would be published to $URL. Nothing else is included."
  exit 0
fi
echo "Not safe to push yet. Fix the lines above, then run this check again."
exit 1
