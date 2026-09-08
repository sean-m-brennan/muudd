#!/usr/bin/env bash
#  Copyright 2026 Sean M. Brennan and contributors
#
#   Licensed under the Apache License, Version 2.0 (the "License");
#   you may not use this file except in compliance with the License.
#   You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
#   Unless required by applicable law or agreed to in writing, software
#   distributed under the License is distributed on an "AS IS" BASIS,
#   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#   See the License for the specific language governing permissions and
#   limitations under the License.

# Build the single-PDF edition of the MUUDD narrative chain.
#
#   tools/pdf/build.sh [output.pdf]
#
# The document set and its order come from doc/narrative-order.json. Output
# defaults to doc/MUUDD.pdf, which the repository ignores. Intermediates live
# in $WORK, outside the repository, and are reused between runs so that a
# rebuild neither reinstalls mermaid-cli nor re-renders unchanged diagrams.
#
# Needs: pandoc, xelatex with fontspec and fvextra, the TeX Gyre Pagella and
# DejaVu fonts, python3, and node with npm. See README.md.
set -e

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
WORK=${WORK:-${TMPDIR:-/tmp}/muudd-pdf}
OUTPDF=${1:-$REPO/doc/MUUDD.pdf}

[ -f "$REPO/doc/narrative-order.json" ] || {
  echo "no doc/narrative-order.json under $REPO; is this the muudd repository?" >&2
  exit 1
}
mkdir -p "$WORK"
COMMIT=$(cd "$REPO" && git rev-parse --short HEAD)

# mermaid-cli is the one dependency not expected on the system, so it is
# installed into the work directory rather than anywhere permanent
cp "$HERE/package.json" "$WORK/package.json"
if [ ! -d "$WORK/node_modules/@mermaid-js" ]; then
  echo "installing mermaid-cli into $WORK"
  (cd "$WORK" && npm install --silent > npm.log 2>&1) || { tail -20 "$WORK/npm.log"; exit 1; }
fi

export MERMAID_DIR="$WORK/mermaid"
export NPM_PREFIX="$WORK"
python3 "$HERE/mermaid.py" "$REPO" "$MERMAID_DIR"

python3 "$HERE/assemble.py" "$REPO" "$WORK/combined.md"

cat > "$WORK/meta.yaml" <<EOF
title: "MUUDD"
subtitle: "Mesh-routed User-owned User-controlled Dynamic Domains: the complete documentation"
author: "Sean M. Brennan and contributors"
date: "Generated $(date +%Y-%m-%d) from commit $COMMIT"
documentclass: book
classoption: [oneside, openany, 11pt]
papersize: letter
geometry: [margin=1in, includeheadfoot]
mainfont: "TeX Gyre Pagella"
sansfont: "DejaVu Sans"
monofont: "DejaVu Sans Mono"
monofontoptions: [Scale=0.85]
colorlinks: true
linkcolor: linkblue
urlcolor: linkblue
toccolor: black
lang: en-US
EOF

pandoc "$WORK/combined.md" \
  --metadata-file="$WORK/meta.yaml" \
  --from=markdown+raw_tex+autolink_bare_uris-yaml_metadata_block \
  --to=latex --standalone \
  --top-level-division=chapter \
  --number-sections \
  --toc --toc-depth=1 \
  --highlight-style=tango \
  --columns=50 \
  --include-in-header="$HERE/preamble.tex" \
  --resource-path="$REPO" \
  -o "$WORK/combined.tex" 2> "$WORK/pandoc.err" || { tail -20 "$WORK/pandoc.err"; exit 1; }
[ -s "$WORK/pandoc.err" ] && { echo "--- pandoc warnings ---"; cat "$WORK/pandoc.err"; }

cd "$WORK"
# three passes: the table of contents and the cross-references between
# documents both need the labels from the pass before
for pass in 1 2 3; do
  xelatex -interaction=nonstopmode -halt-on-error -file-line-error combined.tex \
    > "xelatex.$pass.out" 2>&1 || { echo "xelatex pass $pass FAILED"; grep -E "^\./|^!|Fatal" combined.log | head -30; exit 1; }
  echo "pass $pass done"
done
mkdir -p "$(dirname "$OUTPDF")"
cp combined.pdf "$OUTPDF"
echo "wrote $OUTPDF"
pdfinfo "$OUTPDF" 2>/dev/null | grep -E "Pages|File size" || true
echo "--- diagnostics ---"
grep -c "Overfull \\\\hbox" combined.log | sed 's/^/overfull hboxes: /'
grep -o "Missing character: There is no [^ ]* (U+[0-9A-F]*)" combined.log | sort | uniq -c | sort -rn | head -20
echo "undefined references: $(grep -c "Reference .* undefined" combined.log)"
echo "multiply defined labels: $(grep -c "multiply defined" combined.log)"
