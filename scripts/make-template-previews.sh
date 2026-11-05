#!/bin/sh
# Remakes the preview pictures of the templates in assets/templates/ after a template changes.
# Each template is compiled in a temporary folder, then TeXora draws its title slide (<id>-1.png)
# and its "Key Ideas" slide (<id>-5.png) at 400 pixels wide. Needs pdflatex and a built TeXora.
# Usage: sh scripts/make-template-previews.sh
set -eu
cd "$(dirname "$0")/.."
[ -f .toolchain/env.sh ] && . ./.toolchain/env.sh
cargo build --quiet
BIN=./target/debug/TeXora
WORK=$(mktemp -d)
for tex in assets/templates/*.tex; do
  id=$(basename "$tex" .tex)
  cp "$tex" "$WORK/$id.tex"
  (cd "$WORK" && pdflatex -interaction=nonstopmode "$id.tex" >/dev/null && pdflatex -interaction=nonstopmode "$id.tex" >/dev/null)
  # The window matches the slide shape, so the picture has no bars.
  if grep -q 'aspectratio=43' "$tex"; then size=840x630; else size=1120x630; fi
  for pair in 0:1 4:5; do
    page=${pair%:*}; name=${pair#*:}
    echo '{"check_updates":false}' > "$WORK/settings.json"
    TEXORA_CONFIG="$WORK/settings.json" "$BIN" "$WORK/$id.pdf" --view audience --goto "$page" --window "$size" --screenshot "$WORK/$id-$name.png" >/dev/null
    sips -Z 400 "$WORK/$id-$name.png" --out "assets/templates/$id-$name.png" >/dev/null
  done
  echo "Previews made for $id"
done
