#!/bin/bash
# Build the Zenodo record for "The Floor Theorem for the Yang-Mills Hessian at the Berger Field".
#
# usage: scripts/make_zenodo_floor_theorem.sh OUT_DIR [DOI] [PYTHON]
#   DOI     the DOI Zenodo reserved for this record (e.g. 10.5281/zenodo.12345678). When given, the paper's
#           availability line is rewritten to cite it and the PDF is rebuilt before packaging.
#   PYTHON  interpreter with numpy/sympy/mpmath for the rerun check (default: python3)
#
# Produces in OUT_DIR: paper.pdf, floor-theorem-v1-latex.zip, floor-theorem-v1-code-and-data.zip, MANIFEST.txt,
# and (not uploaded) description.html, metadata.json, UPLOAD.md. Then it unzips both archives into empty
# folders: the LaTeX must compile, and every proof script must reproduce its committed receipt.
set -euo pipefail
OUT=$(mkdir -p "$1" && cd "$1" && pwd); DOI=${2:-}; PY=${3:-python3}
REPO=$(cd "$(dirname "$0")/.." && pwd); cd "$REPO"
P=docs/papers/berger-spectrum; Z=$P/zenodo; R=research/su2-left-invariant-sector-20261119

if [ -n "$DOI" ]; then
  python3 - "$P/paper.tex" "$DOI" <<'EOF'
import sys
p, doi = sys.argv[1], sys.argv[2]
s = open(p).read()
old = "A versioned snapshot will be archived with a DOI on publication."
new = f"This paper, its source, and the code and data above are archived at \\url{{https://doi.org/{doi}}}."
if old in s:
    s = s.replace(old, new)
elif doi not in s:
    sys.exit("availability line not found and DOI not already present")
open(p, "w").write(s)
EOF
  (cd $P && pdflatex -interaction=nonstopmode -halt-on-error paper.tex >/dev/null && pdflatex -interaction=nonstopmode -halt-on-error paper.tex >/dev/null)
fi

STAGE=$(mktemp -d); trap 'rm -rf "$STAGE"' EXIT
# LaTeX archive
L=$STAGE/latex; mkdir -p $L/docs/papers/berger-spectrum/figures $L/docs/papers/two-ring-pg
cp docs/papers/dgstyle.tex $L/docs/papers/
cp $P/paper.tex $L/docs/papers/berger-spectrum/
cp $P/figures/fig_spectrum_ladder.pdf $L/docs/papers/berger-spectrum/figures/
cp docs/papers/two-ring-pg/make_figure_spectrum_ladder.py $L/docs/papers/two-ring-pg/
cat > $L/README.txt <<'EOF'
The Floor Theorem for the Yang-Mills Hessian at the Berger Field: LaTeX source.
Compile:  cd docs/papers/berger-spectrum && pdflatex paper.tex && pdflatex paper.tex
The figure is redrawn by docs/papers/two-ring-pg/make_figure_spectrum_ladder.py from the receipts in the
code-and-data archive (it writes to docs/papers/two-ring-pg/figures/; copy the PDF into berger-spectrum/figures/).
EOF
# code-and-data archive
C=$STAGE/code; mkdir -p $C
FILES=(
  scripts/ricci_su2_left_invariant_sector.py scripts/ricci_su2_spin_sector.py
  scripts/ricci_su2_sector_gauge_check.py scripts/ricci_su2_sector_formula.py
  scripts/ricci_su2_sector_minimality.py scripts/ricci_su2_gap_certificates.py
  scripts/ricci_su2_sector_eigvec_check.py
  docs/papers/two-ring-pg/make_figure_spectrum_ladder.py
  docs/research/su2-exact-sector-2026-11-19.md docs/research/su2-exact-sector-gauge-check-2026-11-19.md
  LICENSE
)
for f in $R/*; do FILES+=("$f"); done
for t in L0_native L0_pg L1_native L1_pg; do FILES+=(research/su2-pg-stability-20261117/$t.eigs.npy research/su2-pg-stability-20261117/$t.json); done
for t in L2_native L2_pg; do FILES+=(research/su2-spectrum-n3-20261119/$t.eigs.npy research/su2-spectrum-n3-20261119/$t.json); done
for f in "${FILES[@]}"; do mkdir -p "$C/$(dirname "$f")"; cp "$f" "$C/$f"; done
mkdir -p $C/docs/papers/two-ring-pg/figures
cp $Z/code-README.md $C/README.md; cp $Z/requirements.txt $C/requirements.txt

rm -f "$OUT"/*.zip
(cd $L && zip -qrX "$OUT/floor-theorem-v1-latex.zip" .)
(cd $C && zip -qrX "$OUT/floor-theorem-v1-code-and-data.zip" .)
cp $P/paper.pdf "$OUT/paper.pdf"
cp $Z/description.html $Z/metadata.json $Z/UPLOAD.md "$OUT/"

# checks: the LaTeX compiles from its archive; the fast proof scripts reproduce their receipts
T=$STAGE/check; mkdir -p $T/l $T/c
(cd $T/l && unzip -q "$OUT/floor-theorem-v1-latex.zip" && cd docs/papers/berger-spectrum \
  && pdflatex -interaction=nonstopmode -halt-on-error paper.tex >/dev/null && pdflatex -interaction=nonstopmode -halt-on-error paper.tex >/dev/null \
  && echo "latex: compiles from the archive, $(pdfinfo paper.pdf 2>/dev/null | awk '/Pages/{print $2}') pages")
(cd $T/c && unzip -q "$OUT/floor-theorem-v1-code-and-data.zip" && mkdir out \
  && "$PY" -I scripts/ricci_su2_left_invariant_sector.py out/sector.json >/dev/null \
  && "$PY" -I scripts/ricci_su2_spin_sector.py out/spin_sectors.json >/dev/null \
  && "$PY" -I scripts/ricci_su2_sector_gauge_check.py out/gauge_check.json >/dev/null \
  && "$PY" -I scripts/ricci_su2_sector_formula.py > out/sector_formula.txt \
  && "$PY" -I scripts/ricci_su2_gap_certificates.py out/gap_certificates.json >/dev/null \
  && "$PY" -I scripts/ricci_su2_sector_minimality.py out/minimality.json 120 >/dev/null \
  && "$PY" - "$R" <<'EOF'
import json, sys
R = sys.argv[1]
def strip(d):
    if isinstance(d, dict):
        return {k: strip(v) for k, v in d.items() if k not in ("seconds", "time", "elapsed")}
    if isinstance(d, list):
        return [strip(x) for x in d]
    return d
def close(a, b, tol=1e-9):
    if isinstance(a, dict):
        return a.keys() == b.keys() and all(close(a[k], b[k], tol) for k in a)
    if isinstance(a, list):
        return len(a) == len(b) and all(close(x, y, tol) for x, y in zip(a, b))
    if isinstance(a, float) or isinstance(b, float):
        return abs(a - b) <= tol * max(1.0, abs(a), abs(b))
    return a == b
ok = True
for f in ("sector.json", "spin_sectors.json", "gauge_check.json", "gap_certificates.json", "minimality.json"):
    same = close(strip(json.load(open(f"out/{f}"))), strip(json.load(open(f"{R}/{f}"))))
    ok &= same; print(f"rerun {f}: {'matches the committed receipt' if same else 'DIFFERS'}")
same = open("out/sector_formula.txt").read() == open(f"{R}/sector_formula.txt").read()
ok &= same; print(f"rerun sector_formula.txt: {'identical' if same else 'DIFFERS'}")
sys.exit(0 if ok else 1)
EOF
)
cd "$OUT"
{
  echo "# The Floor Theorem at the Berger Field, version 1: files for Zenodo"
  echo "# built from commit $(git -C "$REPO" rev-parse HEAD)${DOI:+ (DOI $DOI)}"
  echo
  for f in paper.pdf floor-theorem-v1-latex.zip floor-theorem-v1-code-and-data.zip; do
    printf "%s  %12s  %s\n" "$(shasum -a 256 "$f" | cut -d' ' -f1)" "$(python3 -c "import os,sys;print(f'{os.path.getsize(sys.argv[1]):,}')" "$f")" "$f"
  done
  echo
  echo "code-and-data archive: $(unzip -Z1 floor-theorem-v1-code-and-data.zip | grep -vc '/$') files"
  unzip -Z1 floor-theorem-v1-code-and-data.zip | grep -v '/$' | sort | sed 's/^/  /'
} > MANIFEST.txt
echo "built $OUT"; ls -la "$OUT"
