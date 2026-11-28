#!/bin/bash
# Build the Zenodo drop for "The Lowest Note: A Geometric Theory of the Yang-Mills Mass Gap".
#
# usage: scripts/make_zenodo_lowest_note.sh OUT_DIR [OLD_RECORD_DOI] [VERSION] [PYTHON]
#   OLD_RECORD_DOI  DOI of the published version this one replaces (the withdrawn proof). Filled into metadata.json.
#   VERSION         Zenodo version field (e.g. 2). Filled into metadata.json.
#   PYTHON          interpreter with numpy/scipy for the rerun checks (default: python3)
#   RUST=1          also build helike-lab from the code archive and run an exact zero-point check (slower).
#
# Produces in OUT_DIR/zenodo-upload: the-lowest-note.pdf, the-lowest-note-latex.zip,
# the-lowest-note-code-and-data.zip, MANIFEST.txt. Also, not uploaded: description.html, metadata.json, UPLOAD.md.
# Then it unzips both archives into empty folders: the LaTeX must compile, the threshold must reproduce its receipt,
# and the background dressing must reproduce background_deep.json from the saved spectra.
set -euo pipefail
OUT=$(mkdir -p "$1" && cd "$1" && pwd); OLD=${2:-OLD_RECORD_DOI}; VER=${3:-VERSION_NUMBER}; PY=${4:-python3}
REPO=$(cd "$(dirname "$0")/.." && pwd); cd "$REPO"
P=docs/papers/ym-theory-statement; Z=$P/zenodo
UP=$OUT/zenodo-upload; mkdir -p "$UP"
STAGE=$(mktemp -d); trap 'rm -rf "$STAGE"' EXIT

# 1. the PDF
(cd $P && pdflatex -interaction=nonstopmode -halt-on-error statement.tex >/dev/null \
       && pdflatex -interaction=nonstopmode -halt-on-error statement.tex >/dev/null)
grep -q "That claim is withdrawn." $P/statement.tex || { echo "withdrawal notice missing from the title block"; exit 1; }
cp $P/statement.pdf "$UP/the-lowest-note.pdf"

# 2. the LaTeX archive
L=$STAGE/latex/the-lowest-note; mkdir -p $L/sections
cp $P/statement.tex $P/refs.tex $L/; cp $P/sections/*.tex $L/sections/
mkdir -p $L/figures; cp $P/figures/*.pdf $L/figures/
cat > $L/README.txt <<'EOF'
The Lowest Note: A Geometric Theory of the Yang-Mills Mass Gap. LaTeX source.
Compile:  pdflatex statement.tex && pdflatex statement.tex   (TeX Live 2025; newpx fonts)
EOF
(cd $STAGE/latex && zip -qr "$UP/the-lowest-note-latex.zip" the-lowest-note)

# 3. the code-and-data archive
C=$STAGE/code/the-lowest-note-code-and-data; mkdir -p $C
FILES=(
  LICENSE
  scripts/helike_m3_zero_point.py scripts/helike_m3_zeta.py scripts/helike_m3_background.py
  scripts/helike_m3_deep_analysis.py scripts/gigi_m3_demo.py scripts/gigi_lattice_gauge_demo.py
  scripts/gigi_grid_demos.py scripts/lowest_note_threshold.py scripts/check_lowest_note_review.py
  scripts/build_lowest_note_doublet.py scripts/check_lowest_note_doublet.py scripts/check_h7_cubic_quartic.py
  docs/research/h7-cubic-quartic-proof-2026-11-27.md docs/research/h7-cubic-quartic-contract-2026-11-27.md
  scripts/check_h8_origin_fock.py scripts/check_h6_wick_numeric.py
  docs/papers/ym-theory-statement/figures/helike-doublet.json
  research/dressed-chart-20261126/controls/report.json
  research/lowest-note-threshold-20261127/receipt.json
  docs/research/helike-bundle-framing-math-2026-11-27.md
  docs/research/helike-flux-background-registration-2026-11-24.md
)
for d in research/m3-zero-point-20261127 research/gigi-m3-20261127 research/gigi-lattice-gauge-20261127 \
         research/gigi-grid-demos-20261127 research/lowest-note-review-20261127 research/h7-cubic-quartic-20261127 research/independent-checks-20261127 \
         docs/research/closed-set-20261127; do
  while IFS= read -r f; do FILES+=("$f"); done < <(git ls-files "$d")
done
for f in $(git ls-files 'docs/research/h6-*-2026-11-27.md' 'docs/research/h7-*-2026-11-27.md' 'docs/research/h8-*-2026-11-27.md' \
             'scripts/check_h6_*.py' 'scripts/check_h7_*.py' 'scripts/check_h8_*.py' 'scripts/helike_wick_triangle.py'); do FILES+=("$f"); done
FILES+=(docs/research/h8-small-coupling-switch-deduction-2026-11-27.md)
for f in "${FILES[@]}"; do mkdir -p "$C/$(dirname "$f")"; cp "$f" "$C/$f"; done
# the Rust workspace: solver crates only (the viewer is not needed and pulls in a GUI stack)
H=$C/helike-lab; mkdir -p $H/crates
cp helike-lab/Cargo.lock $H/
sed 's#, "crates/helike-view"##' helike-lab/Cargo.toml > $H/Cargo.toml
for k in helike-core helike-quantum helike-stream helike-cli; do
  mkdir -p $H/crates/$k; (cd helike-lab/crates/$k && git ls-files | while IFS= read -r f; do mkdir -p "$H/crates/$k/$(dirname "$f")"; cp "$f" "$H/crates/$k/$f"; done)
done
cp $Z/code-README.md $C/README.md; cp $Z/requirements.txt $C/
(cd $STAGE/code && zip -qr "$UP/the-lowest-note-code-and-data.zip" the-lowest-note-code-and-data)

# 4. manifest and the not-uploaded files
(cd "$UP" && shasum -a 256 the-lowest-note.pdf the-lowest-note-latex.zip the-lowest-note-code-and-data.zip > MANIFEST.txt)
cp $Z/description.html $Z/UPLOAD.md "$OUT/"
sed -e "s#OLD_RECORD_DOI#$OLD#" -e "s#VERSION_NUMBER#$VER#" $Z/metadata.json > "$OUT/metadata.json"

# 5. verification from empty folders
V=$STAGE/verify; mkdir -p $V/latex $V/code
(cd $V/latex && unzip -q "$UP/the-lowest-note-latex.zip" && cd the-lowest-note \
   && pdflatex -interaction=nonstopmode -halt-on-error statement.tex >/dev/null \
   && pdflatex -interaction=nonstopmode -halt-on-error statement.tex >/dev/null) || { echo "LaTeX archive does not compile"; exit 1; }
echo "latex archive compiles"
(cd $V/code && unzip -q "$UP/the-lowest-note-code-and-data.zip")
CC=$V/code/the-lowest-note-code-and-data
(cd $CC && $PY scripts/lowest_note_threshold.py $V/threshold.json >/dev/null)
$PY - "$CC/research/lowest-note-threshold-20261127/receipt.json" "$V/threshold.json" <<'EOF'
import json, sys
a, b = (json.load(open(p)) for p in sys.argv[1:])
def flat(x, p=""):
    if isinstance(x, dict):
        for k, v in x.items(): yield from flat(v, p + "/" + k)
    else: yield p, x
fa, fb = dict(flat(a)), dict(flat(b))
assert fa.keys() == fb.keys(), "threshold receipt keys differ"
bad = [k for k in fa if abs(fa[k] - fb[k]) > 1e-12 * max(1, abs(fa[k]))]
assert not bad, f"threshold receipt differs at {bad}"
print("threshold reproduces its receipt")
EOF
(cd $CC/research/m3-zero-point-20261127 && gunzip -k zp_t3.0_j200_*.json.gz)
(cd $CC && PYTHONPATH=scripts $PY scripts/helike_m3_deep_analysis.py research/m3-zero-point-20261127 $V/background.json >/dev/null)
$PY - "$CC/research/m3-zero-point-20261127/background_deep.json" "$V/background.json" <<'EOF'
import json, sys
a, b = (json.load(open(p)) for p in sys.argv[1:])
for ma, mb in zip(a["modes"], b["modes"]):
    for k in ("H_median", "H_min", "H_max"):
        assert abs(ma[k] - mb[k]) < 1e-9 * max(1, abs(ma[k])), (ma["name"], k, ma[k], mb[k])
for g in ("g2_0.002", "g2_0.004"):
    assert abs(a[g]["separation_with_fibre"] - b[g]["separation_with_fibre"]) < 1e-12, g
print("background dressing reproduces background_deep.json")
EOF
if [ "${RUST:-0}" = "1" ]; then
  (cd $CC/helike-lab && cargo build -q --release -p helike-cli && ./target/release/helike --out $V/zp.json zero-point --t 1 --lams 0,1 --jmax 12 >/dev/null)
  $PY - "$V/zp.json" <<'EOF'
import json, sys
from collections import Counter
P = json.load(open(sys.argv[1]))["results"]["points"]
def mult(p):
    c = Counter()
    for s in p["sectors"]:
        for m in s["slice"]: c[round(m, 6)] += s["mult"]
    return c
m0, m1 = mult(P[0]), mult(P[1])
for k in range(1, 9):
    mu = float((k + 1) ** 2)
    assert m0[mu] == 6 * k * (k + 2), (k, m0[mu])
    assert m1[mu] == m0[mu], (k, m1[mu], m0[mu])
print("rust zero-point: round-sphere law and flat-point equality hold")
EOF
fi
echo "built: $UP"; cat "$UP/MANIFEST.txt"
