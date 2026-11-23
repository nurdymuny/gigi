# How the results were derived: three papers, claim by claim

> **Fixed on 23 November, before posting** (commit after `db52326`): the edge paper's placeholders (tag now `edge-paper-v1`, Lean tag `edge-paper-v1-lean`, DOI line removed, date no longer "draft"), its receipts sentence, Tests E and F in its receipts table; Floor v3's duplicate reference, its slice count wording, its tag (`floor-theorem-v3`) and its code-README numbering and runtimes; the Three Ways Zenodo description. Items so fixed are marked below. The Russian edition of Three Ways (`ru/part5.tex`) still says 2.5–36% and is not being reposted.


Review package, 23 November 2026. Bee Rosa Davis (Davis Geometric).

This file is for a reviewer auditing three papers from another machine. For each paper it lists what is claimed,
how each claim was derived, the script that produces it, the receipt that records its output, the command that
reproduces it, how long that takes, and the status the paper gives it. It then gives the build, the dependency chain,
and the caveats and amendments I want checked.

| # | Paper | Source | PDF to review | Posting |
| --- | --- | --- | --- | --- |
| 1 | *By Enough, But Not Too Much: Where the Twist Is the Floor of the Yang–Mills Hessian on Berger Spheres* | `docs/papers/by-enough/paper.tex` | `docs/papers/by-enough/paper.pdf` (43 pp.) | new, first posting |
| 2 | *The Floor Theorem for the Yang–Mills Hessian at the Berger Field: A Floor Law Across the Berger Spheres*, version 3 | `docs/papers/berger-spectrum/paper.tex` | clean `paper_v3.pdf` (20 pp.); redline `paper_v3_draft_redline.pdf` (20 pp.); published v2 `paper.pdf` (15 pp.) | repost (new version of doi:10.5281/zenodo.24386613) |
| 3 | *Three Ways Across a Turn: Chord, Arc and Tangent*, version 2 | `docs/papers/chord-arc-tangent/paper.tex` | `docs/papers/chord-arc-tangent/paper.pdf` (19 pp.) | repost (new version of doi:10.5281/zenodo.24375338); English only, `ru/` unchanged |

**Repository and revision.** <https://github.com/nurdymuny/gigi>, branch `feat/reading-the-curve-update`, at commit
`8b7371c` when this file was written. All paths below are relative to the repository root. Every path in this file
was checked with `ls` (or `git ls-tree` for another branch) at that commit; anything I could not find is marked
**SOURCE NOT FOUND**. The Lean files of paper 1 are on another branch (`feat/helike-lean`); see §1.5.

**What "verified" means here.** Numbers marked *receipt-checked* were read back from the receipt JSON and agree with
the paper. Scripts marked *re-run* were run again on 23 November 2026 (macOS, Python 3.11, `OMP_NUM_THREADS=1`) into
a scratch directory, and their output compared with the committed receipt; "identical" means equal after removing
timing fields.

---

## 0. Environment

```bash
git clone https://github.com/nurdymuny/gigi && cd gigi
git checkout feat/reading-the-curve-update
python3.11 -m venv .venv && . .venv/bin/activate
pip install -r docs/papers/berger-spectrum/zenodo/requirements.txt   # numpy 2.3.5, scipy 1.16.3, sympy 1.14.0, mpmath 1.3.0, matplotlib 3.10.7
export OMP_NUM_THREADS=1
mkdir -p out
```

Every script below is a standalone Python file. None of the three papers' results is computed by a GIGI engine verb
(paper 3's Table 3 says which ideas *are* in the engine; those are descriptions of code, not computed results).

**Convention used in commands.** Most scripts take the output path as their first argument. Write to `out/…` and
compare with the committed receipt; ignore `seconds` fields and the last one or two digits of double-precision
diagnostics.

**Status labels.** Paper 1 uses three: **PROVED** (exact symbolic computation or rigorous interval arithmetic),
**COMPUTED** (double precision), and **PROVED(T)** (proved, except that the proof runs through a code transcription
of the proved rows that is checked only in double precision; condition (T), §1.4). Papers 2 and 3 do not use labels;
for them I give the method and whether the text calls the result certified, exact, or numerical.

**Runtime classes.** *s* = seconds, *min* = minutes, *h* = hours. Where a receipt records a time I give it; where
nothing records a time I write "not recorded".

---

## 1. *By Enough, But Not Too Much* (the edge paper)

### 1.1 Build

```bash
cd docs/papers/by-enough
pdflatex paper.tex && pdflatex paper.tex     # 43 pages; inline thebibliography, no BibTeX
```

- House style: `\input{../dgstyle}` (`docs/papers/dgstyle.tex`), which loads `newpxtext`, `newpxmath`, `microtype`,
  `xcolor`, `titlesec`, `caption`, `fancyhdr`, `enumitem`. The paper itself loads `geometry`, `amsmath`, `amsthm`,
  `amssymb`, `graphicx`, `booktabs`, `longtable`, `array`, `url`, `hyperref`. On a minimal TeX Live:
  `tlmgr --usermode install titlesec newpx newtx enumitem fontaxes xstring kastrup`.
- The committed `paper.log` (TeX Live 2026, 23 Nov 06:08) has no LaTeX warnings.
- Figures (all five, `figures/fig_{helike,berger,region,prereg,su3}.pdf`, each with a `.png`):

  ```bash
  OMP_NUM_THREADS=1 python scripts/edge_paper_figures.py              # draws from the cached receipts
  OMP_NUM_THREADS=1 python scripts/edge_paper_figures.py --recompute  # recomputes the three caches in research/edge-paper-figures-20261122/
  ```

  Inputs: `research/edge-paper-figures-20261122/{region_j12,berger_path,su2_margins_t4}.json` (caches),
  `research/su2-floor-family-20261119/exact_edge.json` (t_A, t*), `research/edge-theorem-20261121/wp4_heldout.json`
  and `wp4_general_fields.json` (Tests A, B), `wp4_su3_diagnostic_posthoc.json` (Test C diagnostic). Run time with the
  caches: seconds; with `--recompute`: not recorded.
- Placeholders still in the source: tag `[TAG TO BE SUPPLIED]` and DOI `[TO BE ASSIGNED]` (Code and data availability),  **[FIXED 23 Nov, before posting.]**
  and the date line reads "November 2026 (draft)".

### 1.2 Where things are

The paper's §10 (*Data, receipts and reproducibility*) is a claim-by-claim table. Its abbreviations are:

| Abbrev. | Directory | Files (all present) |
| --- | --- | --- |
| F | `research/su2-floor-family-20261119/` | 50 |
| E | `research/edge-theorem-20261121/` | 68 |
| P | `research/edge-theorem-20261122-proofs/` | 7 |
| G | `research/edge-paper-figures-20261122/` | 3 |
| D | `research/edge-theorem-20261122-d/` | 4 |
| (Test F) | `research/edge-theorem-20261122-f/` | 6 (in the paper's table as `W`, since 23 Nov) |

All 34 scripts named in the paper's table exist in `scripts/`. All receipts it names exist.

### 1.3 Claims, methods, scripts, receipts, commands

#### Setting (§2)

| Claim | Method | Script | Receipt | Command | Time | Status |
| --- | --- | --- | --- | --- | --- | --- |
| Prop. 2.1, generality up to constant gauge: every left-invariant field on (t,1,1) invariant under the right circle about e₀, lifted by a constant gauge circle, is constant-gauge-equivalent to M₀=xτ₀, M₁=yτ₁, M₂=yτ₂, y≥0 | symbolic commutant (exact) + numerical check of the gauge maps to 2.2e-16 | `scripts/edge_wp4_design_checks.py` | `research/edge-theorem-20261121/wp4_design_checks.json` | `python scripts/edge_wp4_design_checks.py out/wp4_design_checks.json` | not recorded | PROVED (algebra); maps COMPUTED |
| Lemma 2.2, Weitzenböck identity H+GG* = ΣXᵢ*Xᵢ + R₀, R₀ = Ric⊗I − 2[F,·], for all real x, y, t>0 | exact symbolic computation in U(su(2))⊗Mat₉, PBW normal ordering; two mutation tests | `scripts/ricci_su2_weitzenbock_family_symbolic.py` | `research/edge-theorem-20261122-proofs/weitzenbock_family.json` (`status: PROVED`) | `python scripts/ricci_su2_weitzenbock_family_symbolic.py out/weitzenbock_family.json` | 21.7 s (paper); 7.9 s on re-run | PROVED. *Re-run: identical.* |
| Lemma 2.3, charge blocks: the conserved charge q splits each sector into blocks of ≤ 9 states, rows in closed form; twist row alone in a 1×1 block | exact symbolic | `scripts/ricci_su2_weitzenbock_family_symbolic.py`, `scripts/ricci_su2_exact_edge.py` | `research/edge-theorem-20261122-proofs/weitzenbock_family.json`, `research/su2-floor-family-20261119/charge_selection_symbolic.txt` | as above | s | PROVED |
| Code rows = proved rows: `symbolic_rows` of `ricci_su2_exact_edge.py`, the domain-certificate rows, the rebuilt rows of `edge_wp1_block_polynomials.py` and `edge_wp3_slice_iff.py`, and the edge-polynomial block B₄,₀−λ₄ equal the rows of Lemma 2.3 | exact sympy equality of the expressions | `scripts/ricci_su2_rows_transcription_check.py` | `research/edge-theorem-20261122-proofs/rows_transcription.json` (`all_exact: true`) | `python scripts/ricci_su2_rows_transcription_check.py out/rows_transcription.json` | 9.3 s on re-run | PROVED. *Re-run: identical.* |
| Condition (T): the near-end matrices of `edge_wp2_interval_proof.py` and the slice construction of `edge_wp3_slice_iff.py` equal the objects of Lemma 2.3 | double precision only: near-end matrices vs blocks 4.5e-16; exact slice blocks vs double blocks 2.8e-14; blocks vs general builder at 12 random points 1.47e-15 | `scripts/ricci_su2_weitzenbock_family_symbolic.py`, `scripts/ricci_su2_exact_edge.py` | `weitzenbock_family.json` (part 3), `research/su2-floor-family-20261119/exact_edge.json` (`blocks_vs_full_spectra_max_rel_diff` = 1.47e-15, receipt-checked) | see the two rows above and §"edge" below | s | COMPUTED |

#### The twist (§3)

| Claim | Method | Script | Receipt | Command | Time | Status |
| --- | --- | --- | --- | --- | --- | --- |
| Prop. 3.1, the twist is an exact gauge-orthogonal eigenvector of H+GG* and H in every sector, multiplicity ≥ 2 | counting argument (charge l+2 is maximal; gauge charges reach ≤ l+1) | none (proof in text) | none | — | — | PROVED |
| Multiplicity exactly 2 where checked; mirror copy present over x∈[−40,1], t∈{3,5,10}, y∈{0.5,1.5,3}, j≤12 (≥ −6.8e-13) | double precision | not named in the paper | not named | — | — | COMPUTED (text only) |
| Prop. 3.2, closed form λ = (x+(j+2)/√t)² + 2√t·x − y²; Berger value λ_t(j); value on A = sθ | exact symbolic; numeric check at 40 random points to 1.4e-15 (receipt: 1.44e-15); direct check from Q for j = 0..6 | `scripts/ricci_su2_twist_family_closed_form.py`, `scripts/ricci_su2_direct_rayleigh.py` | `research/su2-floor-family-20261119/twist_family_closed_form.txt`, `research/edge-theorem-20261122-proofs/direct_rayleigh.json` | `python scripts/ricci_su2_twist_family_closed_form.py out/twist_family_closed_form.txt`; `python scripts/ricci_su2_direct_rayleigh.py out/direct_rayleigh.json 6` | direct_rayleigh 3.6 s on re-run; other not recorded | PROVED. *direct_rayleigh re-run: identical except last digits of the numpy cross-check.* |
| Cor. 3.3, lift cost (j²+4j)/t + 2xj/√t; discount iff x<0 | algebra on Prop. 3.2 | — | — | — | — | (corollary of PROVED) |
| Fig. 1 (`fig_helike`): her shape and her lift term | closed forms | `scripts/edge_paper_figures.py` | — | §1.1 | s | closed forms |
| Prop. 3.4, anatomy: X₀b = iκb, κ = (t−2)/√t − x − j/√t; lowest Landau level; R₀β = (Ric⊥ − 2B)β; λ = κ² + (2j+t+y²) + (Ric⊥−2B); Berger values | exact symbolic; items (4), (6) also in Lean | `scripts/ricci_su2_twist_anatomy.py` (reads `exact_edge.json`); Lean `Helike/Price.lean` | `research/su2-floor-family-20261119/twist_anatomy.json` | `python scripts/ricci_su2_twist_anatomy.py out/twist_anatomy.json` | not recorded | PROVED |
| Rem. 3.6, split of R₀ on the meron: field-strength share +1.2614, +0.9265, +0.7334, +0.5183, +0.4010 at t = 2,3,4,6,8 | double precision (meron coordinates are numerical) | `scripts/ricci_su2_gyromagnetic_split.py` | `research/su2-floor-family-20261119/gyromagnetic_split.txt` | `python scripts/ricci_su2_gyromagnetic_split.py out/gyromagnetic_split.txt` | about a minute (code-README of paper 2) | COMPUTED |

#### The edge (§4)

| Claim | Method | Script | Receipt | Command | Time | Status |
| --- | --- | --- | --- | --- | --- | --- |
| Table 1, edge x*_A / x*_S in j ≤ 12 at t = 2,3,4,6,8; gauge-orthogonal edges agree with the earlier edge test at 21 points to 3e-5 | double precision, Brent to 1e-12 | `scripts/ricci_su2_exact_edge.py` | `research/su2-floor-family-20261119/exact_edge.json` | `python scripts/ricci_su2_exact_edge.py out/exact_edge.json` | not recorded | COMPUTED |
| Berger crossings: H+GG* edge at t = 2.7311222782381144 (within 8e-16 of t_A); gauge-orthogonal edge at 2.6980269405211303 (t* to ~1e-12) | double precision on exact ingredients | `scripts/ricci_su2_exact_edge.py` | `exact_edge.json` (`berger_crossing_A`, `berger_crossing_S`; receipt-checked, see §1.6 note on 8e-16) | as above | — | COMPUTED |
| Competitor table (§4.2): weights 0.62/0.66/0.73, slopes −1.561/−0.623/+1.684 vs 9.741/9.620/10.179; λ_min(B₄,₀) − λ₄ has negative first differences on 241-point grids at 47 (t,y) pairs | double precision | `scripts/ricci_su2_twist_anatomy.py` | `twist_anatomy.json` | as above | — | COMPUTED |
| Prop. 4.1, edge polynomial: P = det(s²(B₄,₀ − λ₄I)) ∈ ℤ[x,y,s], 473 terms, degrees (12,14,28); P = −16384·P₁P₂, P₁ (5,6,12; 61 terms), P₂ (7,8,16; 113 terms), irreducible over ℚ, even in y | exact symbolic determinant over ℚ(√2+√3)[x,y,s], exact factorization; Lean `edgeDet_factor`; second builder recovers the same edge factor exactly | `scripts/ricci_su2_edge_polynomial.py`, `scripts/edge_wp1_block_polynomials.py`; Lean `Helike/Edge.lean` | `research/su2-floor-family-20261119/edge_polynomial.txt`, `edge_polynomial.srepr`; `research/edge-theorem-20261121/wp1_block_polynomials.json`, `wp1_block_polynomials.txt` | `python scripts/ricci_su2_edge_polynomial.py out/edge_polynomial.txt` (writes `.srepr` beside it); `OMP_NUM_THREADS=1 python scripts/edge_wp1_block_polynomials.py research/edge-theorem-20261121/wp1_competitor_map.json out/wp1_block_polynomials [j:q …]` | not recorded (Lean: 231 s for this theorem) | PROVED |
| P₂ vanishes on all 77 grid edges where B₄,₀ is the competitor (nearest root within 2.4e-13); P₁ residual ≥ 2.6e-3 | double precision | `scripts/edge_wp1_block_polynomials.py` | `research/edge-theorem-20261121/wp1_poly_check_final.json` | — | — | COMPUTED |
| "The edge is not the largest real root of P₂" (withdrawn earlier statement) | found by Test A's known-answer gate A-code-2 | `scripts/edge_wp4_heldout.py` | `wp4_heldout.json` (`A_code_2`), prereg `wp4_prereg_A_heldout.md` | — | — | COMPUTED |
| Prop. 4.2, Berger restriction: P₂((2−s²)/s, s, s) = −32(s−1)(s+1)Q₁₆(s) | Lean (`field_simp`, `ring`); the Python script only checks that Q₁₆ divides the restriction | Lean `Helike/Edge.lean` (`edgeFactor_berger`); `scripts/ricci_su2_edge_polynomial.py` | Lean build; `edge_polynomial.txt` | §1.5 | — | PROVED |
| Fig. 2 (`fig_berger`): 121 values of t in [2.55, 2.85] | double precision | `scripts/edge_paper_figures.py` | `research/edge-paper-figures-20261122/berger_path.json` | §1.1 | — | COMPUTED |
| §4.5, other competitors: 132-point grid, 8 blocks set the edge; all 132 grid edges are roots of the right factor to 2.4e-13; single sign change on a 401-point scan; φ_A(−3) ≥ 1.43; Table 3 (competitor factors) | double precision map; factors exact | `scripts/edge_wp1_competitor_map.py`, `scripts/edge_wp1_block_polynomials.py` | `research/edge-theorem-20261121/wp1_competitor_map.json`, `wp1_competitor_map.md`, `wp1_block_polynomials.txt` | `OMP_NUM_THREADS=1 python scripts/edge_wp1_competitor_map.py out/wp1_competitor_map.json` | 5,163 s (receipt), about 1.4 h | map COMPUTED; polynomials PROVED |
| §4.6, sixteen Lean theorems | Lean 4 v4.36.0 + Mathlib; no `sorry`; axioms propext, Classical.choice, Quot.sound | `docs/papers/berger-spectrum/lean/Helike/{Edge,Price}.lean` **on branch `feat/helike-lean` only** | `lean/README.md`, `lean/RESULTS.md`, `lean/Axioms.lean` (same branch) | §1.5 | full build: min (Edge factorization 231 s) | PROVED (polynomial identities) |

#### The region is bounded, and x>0 loses (§5)

| Claim | Method | Script | Receipt | Command | Time | Status |
| --- | --- | --- | --- | --- | --- | --- |
| Thm. 5.1, for x>0 the twist is not the floor of H+GG* in every sector: Rayleigh quotients of e₋ (type (1,−1)) and e₀ (type (1,0)) minus λ in closed form; threshold on l | exact Rayleigh quotients from Q for j = 0..6 (both sides are polynomials of degree ≤ 2 in l, so 7 values prove it); independently from the rows through identity (5.1), all nine types | `scripts/ricci_su2_direct_rayleigh.py`, `scripts/edge_wp2_symbolic.py` | `research/edge-theorem-20261122-proofs/direct_rayleigh.json` (`all_exact_checks_pass: true`), `research/edge-theorem-20261121/wp2_symbolic.txt` | `python scripts/ricci_su2_direct_rayleigh.py out/direct_rayleigh.json 6`; `python scripts/edge_wp2_symbolic.py out/wp2_symbolic.txt` | s | PROVED (for H+GG*; gauge-orthogonal version COMPUTED only) |
| Cor. 5.2, all-sector H+GG* edge ≤ min(x*₁₂, 0) | from Thm. 5.1 | — | — | — | — | follows |
| Positive edges are cut-off artefacts: at all 58 grid points with x*₁₂>0 a sector j>12 undercuts; at x = −0.01, −0.001 nothing undercuts at 57 points (min margin 0.029); gauge-orthogonal margin −0.581 at j=120, (0.1,√10,10) | double precision, two builders | `scripts/edge_wp1_competitor_map.py` (tail runs) | `research/edge-theorem-20261121/wp1_tail_check.json` (1,310 s), `wp1_tail_negx.json`, `wp1_fullsector_check.json`, `workflow_reports.json`, `wp2_large_sector_bound.md` §0 | — | min–h | COMPUTED |
| Prop. 5.3, gauge-orthogonal lower bound r_lo,hi = −(√t + 2/√t) ∓ √(5t+3y²+4) from v = θ̂⁰⊗τ₀ | exact (G*v = 0; Rayleigh quotient 4t+2y²) | `scripts/ricci_su2_direct_rayleigh.py`, `scripts/edge_wp4_design_checks.py`, `scripts/ricci_su2_lower_edge_closed_form.py` | `direct_rayleigh.json`, `wp4_design_checks.json`, `lower_edge_closed_form.json` (`theta0_tau0_RQ_equals_4t+2y^2`, `Gstar_theta0_tau0_zero`) | as above | s | PROVED (both spectra) |
| Prop. 5.4, sector-0 block of charge ±1: cubic P_lo; region of H+GG* lies in x ≥ r₁ | exact 9×9 sector-0 matrix from Q by a SymPy port of the general builder | `scripts/ricci_su2_lower_edge_closed_form.py` | `research/edge-theorem-20261122-proofs/lower_edge_closed_form.json` | `python scripts/ricci_su2_lower_edge_closed_form.py out/lower_edge_closed_form.json` | 4.4 s on re-run | PROVED (H+GG* only). *Re-run: identical.* |
| r₁ = x_lo at the 48 Test A points to 8.2e-12; trace of B−λ₀I ≥ 43.06 there | double precision | same | same (`A_max_abs_dev` = 8.21e-12, `A_min_trace_at_smallest_root` = 43.058; receipt-checked) | same | — | COMPUTED |
| Cor. 5.5, region bounded: max(r_lo, r₁) ≤ x ≤ min(x*₁₂, 0) | from 5.2–5.4 | — | — | — | — | follows |
| Fig. 3 (`fig_region`), region is one interval in j ≤ 12 at 300 (t,y) pairs; 47 positive x*₁₂ at t=8; upper edges agree with the competitor map at 60 shared points to 2.7e-15 | double precision | `scripts/edge_paper_figures.py` | `research/edge-paper-figures-20261122/region_j12.json` | §1.1 | — | COMPUTED |
| Thm. 5.6, the edge on two slices t=3, y∈{√3, 3/2}, j ≤ 12: m(x) > 0 on [−3, x*), simple zero at x*, negative on (x*, r_next); every other block above λ_j on [−3, x*+0.02]; x* = −0.3821684822419167 and −0.37158172515020627 | exact blocks; exact factorization on the slice (3⁹D = P); rational root isolation (widths < 4e-31); interval Cholesky at 96 bits (mpmath); Rellich analytic branches | `scripts/edge_wp3_slice_iff.py` | `research/edge-theorem-20261121/wp3_slice_iff.json`, `wp3_slice_iff.md`, `wp3_slice_iff_log.txt` | `python scripts/edge_wp3_slice_iff.py out/wp3` | about 25 s (paper); 18 s on re-run | PROVED(T). *Re-run: identical except the last digits of the double-precision check m(x*+1e-3).* x*, r_next, and the enclosures of Table 4 receipt-checked. |
| Thm. 5.7, large sectors on slabs: on each box of Table 5, every sector j ≥ 40 (and j ≥ 20 on [−3,−0.6]×[0.5,3]×[3,10]) | interval LDLᵀ with bisection on near-end blocks scaled by 1/l in U = 1/l; Gershgorin with the exact quantity G on bulk rows; mpmath intervals at 80 bits | `scripts/edge_wp2_interval_proof.py` | `research/edge-theorem-20261121/wp2_interval_proof_*.json` (13 boxes, each with `.log.txt`; all `all_proved: true`); rerun `wp2_verify_rerun_x-0.6_L20.json` (125.7 s) | `python scripts/edge_wp2_interval_proof.py out/box.json L0 N1 XA XMIN XMAX YMIN YMAX TMIN TMAX`, e.g. `… 20 8 20 -3 -0.6 0.5 3 3 10` (L0=20: N1=8, XA=20; L0=10: N1=XA=10) | 19.1 s to 4,445.8 s per box (receipt seconds match Table 5) | PROVED(T) |
| Lemma 5.8, exact Gershgorin quantity: G = D − λ − (exact radius) for all nine types; FA, FB of the proof code return G on symbolic arguments | exact SymPy identities; cross-check on 23,031 rows to 1.0e-12 | `scripts/ricci_su2_gershgorin_lemma_symbolic.py` | `research/edge-theorem-20261122-proofs/gershgorin_lemma.json` (`all_proved: true`) | `python scripts/ricci_su2_gershgorin_lemma_symbolic.py out/gershgorin_lemma.json` | 34.9 s (paper); 12.3 s on re-run | PROVED. *Re-run: identical.* |
| Conj. 5.9 (all-sector edge = min(x*₁₂, 0) for t ≥ 3) fails at and near t = 2: j=200 undercuts at t=2; j=16, q=8 at (2.01, 0.25); j=58 at (2.001, 0.25) | double precision, two builders | `scripts/edge_wp1_competitor_map.py` | `wp1_tail_check.json`, `workflow_reports.json` | — | — | COMPUTED |
| Near-top families D₊ (q=l+1), D₀ (q=l) with l symbolic; leading coefficients c₊, c₀ vanish at s=√2; Table 8 (family edge falls like 1/j at (12,2)) | exact algebra; family edges double precision | `scripts/edge_wp1_block_polynomials.py` | `research/edge-theorem-20261121/wp1_top_family.txt`, `wp1_top_family.json`, `wp1_family_members.json`, `wp1_competitor_map.md` §4 | — | — | algebra PROVED; edges COMPUTED |
| Table 7 (edge vs cut-off J at (12, 2.75)) | double precision | `scripts/edge_wp1_competitor_map.py` | `wp1_competitor_map.md` §4 | — | — | COMPUTED |

#### The floor on a domain (§6)

| Claim | Method | Script | Receipt | Command | Time | Status |
| --- | --- | --- | --- | --- | --- | --- |
| Thm. 6.1, on D = [−3,−1.5]×[0.5,3]×[3,10] the twist is the floor of H+GG* in every sector, multiplicity exactly 2, and the lowest gauge-orthogonal eigenvalue (also Thm. 7.3 of paper 2, v3) | (a) reduction to charge blocks (Lemma 2.3); (b) j ≥ 80: G > 0 for l ≥ 40 by interval bisection + Lemma 5.8; (c) j < 80: mean-value interval certificate on boxes (interval LDLᵀ at the centre with outward rounding + a posteriori residual, variation bounded by Σ sup‖∂ᵢA‖_F × half-widths) | (b) `scripts/ricci_su2_family_large_j.py`; (c) `scripts/ricci_su2_family_block_cert.py` with `scripts/ricci_su2_outward.py` | (b) `research/su2-floor-family-20261119/family_large_j.json`; (c) `research/su2-floor-family-20261119/family_block_cert_j80.json` | (b) `python scripts/ricci_su2_family_large_j.py out/family_large_j.json 40 20 -3 -1.5 0.5 3 3 10`; (c) `python scripts/ricci_su2_family_block_cert.py out/cert.json 0 80 -3 -1.5 0.5 3 3 10 [NPROC]` (the receipt records `jmin 0, jmax 80`; it holds rows j = 0..79) | (b) 13.5 s (5.9 s on re-run); (c) **229,134.7 s ≈ 63.6 CPU-hours** | PROVED. *(b) re-run: identical.* (c) receipt-checked: 80 rows, 3,400 blocks, 1,640,935 boxes, 0 failures, min per-box slack 1.2220726e-6 at j=1, `all_certified: true`, `point_check` 1.7e-13; Table 6 rows (j = 0, 1, 2, 3, 36) match. |
| Rounding bound for (c): the last scalar operations of each box (round-to-nearest) cannot change a verdict: error ≤ κ(1+u)M* = 1.63e-15 vs smallest slack 1.22e-6 (ratio 7.48e8) | a priori floating-point error analysis; M* an interval bound | `scripts/ricci_su2_family_cert_rounding_bound.py` (reads the certificate receipt) | `research/edge-theorem-20261122-proofs/rounding_bound.json`, `rounding_bound.md` | `python scripts/ricci_su2_family_cert_rounding_bound.py out/rounding_bound.json 60` | 15.8 s on re-run | PROVED. *Re-run: identical.* |
| Appendix B.3 details: region A min 1.0678e-4 at type (0,1), ξ=0, 1,360 pieces; region B min 0.7226 (type (0,0)) to 1.5918 (type (−1,1)) | interval | as (b) | `family_large_j.json` (receipt-checked) | — | — | PROVED |
| Rem. 6.3, D lies inside the region: min φ_A = 2.35 on an 11×15 grid, min φ_S = 2.69 on a 3×3 grid | double precision | `scripts/ricci_su2_family_large_j.py`, `scripts/ricci_su2_exact_edge.py` | `family_large_j.json` | — | — | COMPUTED |
| Rem. 6.2 (why D stops at x = −1.5): sampled plain-Gershgorin cutoffs l ≈ 640, 160, 40 | double precision | not named | `research/edge-theorem-20261121/wp2_bounds.json` covers the weighted-Gershgorin cutoffs of App. B.2 | — | — | COMPUTED |

#### What decides the edge (§7), earlier pre-registered tests

| Claim | Script | Receipt | Prereg / note | Status |
| --- | --- | --- | --- | --- |
| Test 2 (symmetry; failed): twist the floor of no sector on the squashed meron at 2,600 (t,j); two SU(3) families, none of 9 sectors at any of 200 points | `scripts/ricci_su2_test2_family.py`, `scripts/ricci_su3_test2_family.py`, `scripts/ricci_su2_gyromagnetic_split.py` | `research/su2-floor-family-20261119/test2_meron.json`, `test2_berger_control.json`, `su3_test2_family2a.json`, `su3_test2_family2b.json`, `su3_test2_controls.json`, `gyromagnetic_split.txt` | `docs/research/skeleton-tests-preregistration-2026-11-20.md`, `docs/research/skeleton-test2-family1-results-2026-11-20.md` | COMPUTED |
| Test 4 (alignment; failed) | `scripts/ricci_alignment_test.py` | `alignment_test_4a.json`, `alignment_test_4b.json`, `alignment_test_controls.json` (all in F) | `docs/research/alignment-test-preregistration-2026-11-21.md` | COMPUTED |
| Test 5 (alignment + curvature term; failed): 72 points, 35/5/32 | `scripts/ricci_stretch_test.py` | `research/su2-floor-family-20261119/stretch_test.json` | `docs/research/stretch-test-preregistration-2026-11-21.md`, `stretch-test-results-2026-11-21.md` | COMPUTED |
| Test 6 (edge model c₁/t + c₂/t²; E1 and E2 failed) | `scripts/ricci_su2_edge_test.py` (`fit OUT_PRED.json`, then `test PRED.json OUT.json`) | `edge_test_predictions.json`, `edge_test_results.json` (F) | `docs/research/edge-test-preregistration-2026-11-21.md`, `edge-test-results-2026-11-21.md` | COMPUTED |
| Exploratory map: 3,294 all-nine cells have x ≤ 0 (3,286 have x < 0) | `scripts/ricci_su2_twist_family_map.py OUT.json [NPROC]` | `research/su2-floor-family-20261119/twist_family_map.json` | not pre-registered | COMPUTED |

Run times for these tests are not recorded in their receipts; the code-README of paper 2 gives "minutes each" for
Test 2.

#### Pre-registered tests A–D and F (§8; all COMPUTED)

| Test | Script | Receipt | Pre-registration (fixed before scored data) | Results note | Receipt-checked headline |
| --- | --- | --- | --- | --- | --- |
| A, held-out points (48) | `scripts/edge_wp4_points.py` (draws the points), `scripts/edge_wp4_heldout.py` | `research/edge-theorem-20261121/wp4_heldout.json`, `wp4_heldout_log.txt` | `wp4_prereg_A_heldout.md`, `wp4_prereg_A_points.json`; protocol string "@ 232ab8d + Amendment 1 @ 9098bd9" | `docs/research/edge-prereg-A-results-2026-11-22.md` | A1 failed at 4 of 38 (`A1_failures` has 4 entries: three (1,−1.5), one (12,−7) at t=11); A2, A3, A4 held (empty failure lists); A-code-3 1.04e-18 |
| B, general circle-symmetric fields (200 draws, 193 scored) | `scripts/edge_wp4_general_fields.py` | `wp4_general_fields.json`, `wp4_general_fields_log.txt` | `wp4_prereg_B_general_fields.md`; protocol "@ 232ab8d + Amendment B1" | `docs/research/edge-prereg-B-results-2026-11-22.md` | B-S failures 129; ρ_s quantiles 0.0024, 0.43, 0.67, 0.78, 0.88; neutral fraction 0.316; `B_II_code` true; B-half failures none |
| C, SU(3) with free fibre field (20 shapes) | `scripts/edge_wp4_su3_free_fibre.py` | `wp4_su3_free_fibre.json`, `wp4_su3_free_fibre_log.txt` | `wp4_prereg_C_su3.md`; protocol "@ 232ab8d + Amendment C1 @ 4287efc + Amendment C2" | `docs/research/edge-prereg-C-results-2026-11-22.md` | `C_P1_weak` false, `C_P1_strict` false |
| C diagnostic (post hoc, not part of the protocol) | `scripts/edge_wp4_su3_diagnostic.py` | `wp4_su3_diagnostic_posthoc.json`, `_log.txt` | none (post hoc) | as above | — |
| D, SU(3) along her own charge circle | `scripts/edge_wp5_design_checks.py`, `scripts/edge_wp5_su3_own_circle.py` | `research/edge-theorem-20261122-d/wp5_design_checks.json`, `wp5_su3_own_circle.json`, `_log.txt` | `research/edge-theorem-20261122-d/wp5_prereg_D_su3_own_circle.md` | `docs/research/edge-prereg-D-results-2026-11-22.md` (interim: `edge-prereg-D-interim-2026-11-22.md`) | D-P1 weak and strict true, not truncation-sensitive; D-P2 15 scorable, pass; D-P3 true; D-P4 false (15 failures) |
| F, which circle (SU(4), Sp(2)) | `scripts/edge_wp7_principal_circle.py`, `scripts/edge_wp7_groups.py`; post-hoc far-wall diagnostic `scripts/edge_wp8_farwall_diagnostic.py` | `research/edge-theorem-20261122-f/wp7_principal_circle.json` (the first, faulty receipt kept as `wp7_principal_circle_first_receipt.json`; log `wp7F.log`); diagnostic `research/edge-theorem-20261122-g/farwall_diagnostic.json` | `wp7_prereg_F_principal_circle.md`, `wp7_prereg_F_amendment1_peak.md`, `wp7_prereg_F_amendment2_wall.md` | `docs/research/edge-prereg-F-results-2026-11-22.md` | H_θ supported in both groups; H_P, H_reg, H_C not; per-cell verdicts as in the text |

Commands: each script takes `OUT.json` as its only argument, run with `OMP_NUM_THREADS=1`; e.g.
`OMP_NUM_THREADS=1 python scripts/edge_wp4_heldout.py out/wp4_heldout.json`. Run times are not recorded in the
receipts (Test C's receipt records 19.6 s per x point); class: minutes to hours.

Fig. 4 (`fig_prereg`) and Fig. 5 (`fig_su3`) are drawn by `scripts/edge_paper_figures.py` from these receipts and
`research/edge-paper-figures-20261122/su2_margins_t4.json`; Fig. 5(b) is post hoc.

Test G (G₂, Sp(3)) is described in the paper as "running". Its registration and partial receipts are in
`research/edge-theorem-20261122-g/` (`wp8_prereg_G_two_new_groups.md`, `wp8G_g2.json`, `wp8G_g2.log`) with the
interim note `docs/research/edge-prereg-G-interim-2026-11-23.md`; no result of Test G is claimed.

### 1.4 Dependency chain

```
Standard facts (Peter–Weyl matrix elements; H is the operator of Q; ρ_a* = −ρ_a)
  └─ Lemma 2.2  Weitzenböck identity, whole family           [weitzenbock_family.json, PROVED]
       └─ Lemma 2.3  charge blocks, closed-form rows          [same + charge_selection_symbolic.txt, PROVED]
            ├─ Prop. 3.1 twist exact (counting) ─ Prop. 3.2 closed form ─ Cor. 3.3, Prop. 3.4
            ├─ "code rows = proved rows"                     [rows_transcription.json, PROVED]
            │     covers: symbolic_rows (exact_edge.py), domain-certificate rows, wp1 and wp3 rebuilt rows, B₄,₀−λ₄
            ├─ condition (T): wp2 near-end matrices, wp3 slice construction   [double precision only]
            │
            ├─ Prop. 4.1 edge polynomial  ← exact det + factorization; Lean edgeDet_factor (identity only)
            │     └─ Prop. 4.2 Berger restriction ← Lean edgeFactor_berger
            │           └─ (computed) Berger path crosses the edge at t_A, t*
            ├─ Thm. 5.1 x>0 loses ← direct Rayleigh quotients from Q (does NOT need Lemma 2.3); identity (5.1)
            ├─ Prop. 5.3 r_lo ← direct from Q;  Prop. 5.4 r₁ ← exact sector-0 matrix from Q
            ├─ Thm. 5.6 two slices  ← Prop. 4.1 on the slice + exact blocks + interval Cholesky + (T)  → PROVED(T)
            ├─ Lemma 5.8 exact Gershgorin G ← rows (under the proved transcription of symbolic_rows)
            │     └─ Thm. 5.7 slabs ← Lemma 5.8 (bulk) + interval LDLᵀ on near-end matrices + (T)  → PROVED(T)
            └─ Thm. 6.1 floor on D
                  (a) reduction: Lemma 2.3 + Prop. 3.2 + antiunitary map
                  (b) j ≥ 80: Lemma 5.8 + interval bisection of G   [family_large_j.json]
                  (c) j < 80: interval certificate (rows proved = Lemma 2.3 via rows_transcription)
                        + outward-rounded LDLᵀ (ricci_su2_outward.py)
                        + rounding bound for the final round-to-nearest steps   [rounding_bound.json]
```

So Theorem 6.1 rests on: Lemma 2.2, Lemma 2.3, the transcription check (`rows_transcription.json`), Lemma 5.8, the
two interval certificates (`family_large_j.json`, `family_block_cert_j80.json`) and the rounding bound. It does not
rest on (T): the domain certificate's rows are among those proved equal to Lemma 2.3. Theorems 5.6 and 5.7 do rest on
(T), which is why they carry PROVED(T).

### 1.5 Lean

The paper cites `docs/papers/berger-spectrum/lean/Helike/Edge.lean` and `Price.lean`. **They are not on the review
branch.** They are on `feat/helike-lean` (head `bbe3478`, 21 Nov 2026), which is not merged into
`feat/reading-the-curve-update`. Files there: `lean/Helike.lean`, `lean/Helike/Edge.lean`, `lean/Helike/Price.lean`,
`lean/Axioms.lean`, `lean/README.md`, `lean/RESULTS.md`, `lakefile.toml`, `lake-manifest.json`,
`lean-toolchain` (`leanprover/lean4:v4.36.0`).

```bash
git checkout feat/helike-lean -- docs/papers/berger-spectrum/lean     # or: git worktree add ../helike feat/helike-lean
cd docs/papers/berger-spectrum/lean
lake exe cache get && lake build            # 18,256 jobs on a full rebuild; Helike.Edge about 231 s
lake env lean Axioms.lean                   # lists the axioms of each theorem
```

The paper states (Code and data availability) that the tagged tree must contain these files. Lean checks only the
polynomial identities and the two brackets; it does not check that `edgeDet` is the determinant, that P₂'s root is the
edge, or that `price` is an eigenvalue (§4.6, "What Lean does not check").

### 1.6 Caveats and disclosed amendments to check

1. **(T) is not symbolic.** Theorems 5.6 and 5.7 depend on the near-end matrices of `edge_wp2_interval_proof.py` and
   the slice construction of `edge_wp3_slice_iff.py` agreeing with Lemma 2.3, checked to 4.5e-16 and 2.8e-14 in double
   precision only. Open problem 2 states this.
2. **Slab distances are grid-measured.** The distances δ in Table 5 come from the worst grid point of each slab; the
   gap to the true edge between grid points is not proved. Sectors j < 40 on the slab boxes are not certified, and
   x ∈ (−0.02, 0) is not covered.
3. **Theorem 5.1 is for H+GG* only.** The test states are not gauge-orthogonal (‖G*e₋‖² = y²); the gauge-orthogonal
   version is computed, not proved. Likewise Prop. 5.4 is for H+GG* only.
4. **One-interval structure is computed, not proved.**
5. **The domain certificate's per-box minima are box slack, not spectral margins** (1.22e-6 vs an actual margin of
   about 2.35). The receipt stores only the minimum per sector, so a directed recheck of individual boxes requires
   re-running the 63.6 CPU-hour certificate. The last scalar operations are round-to-nearest; the rounding bound covers
   them.
6. **Prop. 2.1's receipt checks the k = −1 case only at z = 0**, with the opposite sign convention; the isometry group
   for t ≠ 1 is assumed, not re-proved.
7. **Withdrawn statement.** "The edge is the largest real root of P₂" was withdrawn after Test A's known-answer gate
   A-code-2 fired (Amendment 1).
8. **Amendments.** Test A: Amendment 1 (predicted root selection), registered at `9098bd9`. Test B: Amendment B1
   (competitor accepted up to rounding ties), after gate B-II-code stopped the run before any B-II point. Test C:
   Amendments C1 (made after a smoke test showed a gate would fire) and C2 (tied doublets). Test D: design revised after
   an adversarial review, before any data. Test F: Amendment 1 (peak test) is **not blind** on three printed points
   that had already been seen; Amendment 2 (reading at a degenerate wall) carries a disclosed implementation
   correction, made after the whole first receipt had been seen, which changed no verdict (the first receipt is kept).
   See the results notes in `docs/research/edge-prereg-{A,B,C,D,F}-…md`.
9. **Test F is in the text but not in the paper's receipts table (§10).** **[FIXED 23 Nov: Tests E and F added to the table.]** Its script and receipt are listed in §1.3
   above.
10. **Placeholders.** Tag and Zenodo DOI are not yet filled; the date line says "(draft)".  **[FIXED 23 Nov, before posting.]**
11. **"Each script writes its receipt by default at the path above"** (§10, *Reproducing*). This is not accurate for  **[FIXED 23 Nov, before posting.]**
    most scripts: they require the output path as an argument (only `ricci_su2_weitzenbock_family_symbolic.py` and
    `ricci_su2_lower_edge_closed_form.py` have default output paths).
12. **"Within 8e-16 of t_A".** The receipt's `diff` field is 8.88e-16, measured against the double nearest t_A
    (2.7311222782381135). Against the higher-precision root in `tstar_exact.txt` (2.7311222782381136231) the
    difference is 7.8e-16. The statement is true against the exact root; a reader comparing with the receipt field will
    see 8.9e-16.

---

## 2. *The Floor Theorem*, version 3

### 2.1 Build

- **Clean v3 (`paper_v3.pdf`)**: `paper.tex` line 27 currently reads `\redlinetrue`. Set it to `\redlinefalse`, then

  ```bash
  cd docs/papers/berger-spectrum
  pdflatex paper.tex && pdflatex paper.tex      # 20 pages, date line "November 2026 (version 3)"
  ```

  The committed `paper_v3.pdf` was added in commit `6d50499` (23 Nov 2026); the build step itself is not scripted in the
  repository.
- **Redline (`paper_v3_draft_redline.pdf`)**: build `paper.tex` as committed (`\redlinetrue`): insertions blue,
  deletions red and struck out (needs `ulem`). The date line then reads "version ~~2~~3, draft".
- **Published v2 (`paper.pdf`, 15 pages)**: committed at `5715737`; its source is at tag `floor-theorem-v2`
  (`git show floor-theorem-v2:docs/papers/berger-spectrum/paper.tex`).
- Packages: `../dgstyle.tex` (as in §1.1), plus `amsmath`, `amsthm`, `amssymb`, `graphicx`, `booktabs`, `url`,
  `hyperref`, `ulem`.
- Figures:

  | Figure | File | Script | Inputs |
  | --- | --- | --- | --- |
  | 1, spectrum ladder | `figures/fig_spectrum_ladder.pdf` | `docs/papers/two-ring-pg/make_figure_spectrum_ladder.py` (writes into `docs/papers/two-ring-pg/figures/`; `scripts/make_zenodo_floor_theorem.sh` copies it into `berger-spectrum/figures/`) | `research/su2-left-invariant-sector-20261119/gauge_check.json`; lattice eigenvalues `research/su2-pg-stability-20261117/L{0,1}_*.eigs.npy`, `research/su2-spectrum-n3-20261119/L2_*.eigs.npy` |
  | 2, three pieces (new in v3) | `figures/fig_three_pieces.pdf` | `docs/papers/berger-spectrum/make_figure_three_pieces.py` | closed forms; checks the identity symbolically |
  | 3, floor family | `figures/fig_floor_family.pdf` | `docs/papers/berger-spectrum/make_figure_floor_family.py` | closed forms |
  | 4, twist map | `figures/fig_twist_map.pdf` | `python scripts/ricci_su2_twist_family_figures.py research/su2-floor-family-20261119/twist_family_map.json OUTDIR docs/papers/berger-spectrum/figures/fig_twist_map.pdf` | `twist_family_map.json` |

- Warnings to expect: `Label 'nielsenolesen1978' multiply defined` (the reference is entered twice in the  **[FIXED 23 Nov, before posting.]**
  bibliography; the clean v3 PDF prints it twice). See §2.5.
- Zenodo: `docs/papers/berger-spectrum/zenodo/{UPLOAD.md, code-README.md, description.html, metadata.json,
  requirements.txt}`; packaging script `scripts/make_zenodo_floor_theorem.sh`, which rebuilds the PDF and checks that
  the proof scripts, rerun from the code zip in an empty folder, reproduce their receipts.

Numbering below is that of v3 (from `paper.aux`). Sections 1–6 and 8–9 keep v2's numbering; Section 7 is new.

### 2.2 Claims, methods, scripts, receipts, commands

Receipt directories: `research/su2-left-invariant-sector-20261119/` (L) and `research/su2-floor-family-20261119/` (F).
Commands are those of `zenodo/code-README.md`, which I checked against the scripts' usage lines.

| Claim | Method | Script | Receipt | Command | Time | Paper's status |
| --- | --- | --- | --- | --- | --- | --- |
| Prop. 3.1, left-invariant sector at t=3: spectrum of H, gauge compression (8, 12, 12), gauge-orthogonal spectrum −14/3 (×2), 16−2√73, 22/3 (×2), 16+2√73 | exact symbolic (entries in ℚ(√3)) | `scripts/ricci_su2_left_invariant_sector.py`, `scripts/ricci_su2_sector_minimality.py` | L/`sector.json`, L/`minimality.json` (`spin0_slice_exact`) | `python scripts/ricci_su2_left_invariant_sector.py out/sector.json`; `python scripts/ricci_su2_sector_minimality.py out/minimality.json 120` | 2.7 s on re-run; minimality 137 s (receipt) | exact. *sector.json re-run: identical except last digits of the lattice comparison.* |
| Prop. 4.1, the floor formula (j(j+2)−14)/3 is an exact gauge-orthogonal eigenvalue in every sector | exact symbolic in j | `scripts/ricci_su2_sector_formula.py` | L/`sector_formula.txt` | `python scripts/ricci_su2_sector_formula.py > out/sector_formula.txt` | s | exact |
| Lemma 5.1, Weitzenböck identity at t=3 in U(su(2)); R₀ eigenvalues; ω² = 76/3 | exact, PBW normal order | `scripts/ricci_su2_gap_certificates.py` | L/`gap_certificates.json` (`certificate_1`) | `python scripts/ricci_su2_gap_certificates.py out/gap_certificates.json` | 321.9 s (receipt) | exact |
| Thm. 5.2, the Floor Theorem: (1) tail bound; (2) λ(j) is the floor for j=0,12,20,24 with gap 1/3 − 1e-53; (3) Poincaré-sphere spectrum below 142 | (1) reverse triangle inequality + Lemma 5.1; (2) certified inertia: mpmath intervals at 200 bits, realification, LDLᵀ without pivoting, a posteriori residual, Sylvester + Weyl | `scripts/ricci_su2_gap_certificates.py`, `scripts/ricci_su2_outward.py` (adversarial tests `scripts/ricci_su2_outward_test.py`, output `F/outward_test.txt`) | L/`gap_certificates.json` (`certificate_2`; every pivot, ρ, residual, ε) | as above | as above | certified. Table 1 receipt-checked (j=0: ρ 3.0e-58, residual 5.5e-59, ε 3.5e-58, 4 negative pivots, min pivot 0.679; j=30 margin 0.446). |
| Cor. 5.4, floor gap 168/3 = 56 | algebra on Prop. 4.1 + Thm. 5.2 | — | — | — | — | exact |
| Remark after Table 1: double-precision H+GG* for j ≤ 120 | double precision | `scripts/ricci_su2_sector_minimality.py` | L/`minimality.json` | as above | 137 s | numerical, not relied on |
| Prop. 6.1, the floor law λ_t(j) for every t>0 | exact symbolic in j and t | `scripts/ricci_su2_sector_formula_squashed.py` | F/`formula_squashed.txt` | `cd scripts && python ricci_su2_sector_formula_squashed.py` | about 30 s | exact |
| Prop. 6.2, Weitzenböck for every t; X₊b = 0, X₀b = iκb, R₀β = (12−8t)β; three-piece decomposition | exact symbolic in t | `scripts/ricci_su2_weitzenbock_symbolic_t.py`, `scripts/ricci_su2_landau_symbolic.py` | F/`weitzenbock_symbolic_t.json`, F/`landau_symbolic.txt` | `python scripts/ricci_su2_weitzenbock_symbolic_t.py out/weitzenbock_t.json`; `python scripts/ricci_su2_landau_symbolic.py out/landau_symbolic.txt` | min; s | exact |
| Fig. 2 (new in v3), the three pieces | closed forms, identity checked symbolically | `docs/papers/berger-spectrum/make_figure_three_pieces.py` | — | `python docs/papers/berger-spectrum/make_figure_three_pieces.py` | s | exact |
| Thm. 6.4, Floor Theorem at (4,1,1); j = 0,12,20,24,30,32; spectrum below 136 | as Thm. 5.2 at t=4 | `scripts/ricci_su2_gap_certificates_t.py` | F/`gap_certificates_t4.json` | `python scripts/ricci_su2_gap_certificates_t.py 4 out/t4.json 136 0,12,20,24,30,32 36` | 741.3 s (receipt) | certified |
| Lemma 6.5, charge blocks at the Berger field (t symbolic); Rem. 6.6 (why the formula is exact) | exact symbolic | `scripts/ricci_su2_charge_selection_symbolic.py`; rows `scripts/ricci_su2_block_rows_symbolic.py`; validation `scripts/ricci_su2_charge_block_direct.py` | F/`charge_selection_symbolic.txt`, F/`block_rows_symbolic.json`; validation prints only | `python scripts/ricci_su2_charge_selection_symbolic.py out/charge_selection_symbolic.txt`; `python scripts/ricci_su2_block_rows_symbolic.py out/block_rows_symbolic.json` | a few min each | exact |
| Thm. 6.7, the floor law on 2.75 ≤ t ≤ 10, every sector | j ≤ 99: block interval certificates per t-slice (t and λ_t(j) entered as intervals; outward rounding); j ≥ 100: closed-form Gershgorin G proved positive by interval bisection; independent first certificate on full sector matrices of 15 sectors | `scripts/ricci_su2_block_interval_cert.py`, `scripts/ricci_su2_floor_large_j.py`, `scripts/ricci_su2_floor_interval_cert.py` | F/`block_cert_j40.json`, F/`block_cert_j41_99.json`, F/`floor_large_j.json`, F/`interval_cert_2.75_10.json` | `python scripts/ricci_su2_block_interval_cert.py out/b40.json 2.75 10 40 8`; `… out/b99.json 2.75 10 99 8 41`; `python scripts/ricci_su2_floor_large_j.py out/large_j.json 50 20 2.75 10`; `python scripts/ricci_su2_floor_interval_cert.py out/range.json 2.75 10 0,1,2,3,4,5,6,7,8,9,10,11,12,20,24 6` | j ≤ 40: 24,268 CPU-s ≈ 6.7 CPU-h; 41–99: 54,199 CPU-s ≈ 15.1 CPU-h (sums of per-sector times in the receipts; about 0.8 h and 1.9 h wall on 8 workers); j ≥ 100: 0.6 s (1.4 s on re-run); full-matrix certificate 8,904 s | certified. Receipt-checked: 943 blocks / 12,596 slices / min margin 1.74e-5; 4,307 / 36,166 / 2.86e-3; 373 + 9 boxes, G/l ≥ 5.99e-4, G/(lx) ≥ 0.859; 15 sectors, 1,091 slices in total, min δ−ε 0.00178. *floor_large_j re-run: identical.* |
| Rem. 6.8, below the range: t* ≈ 2.6980 (bisection), t_A = 2.7311222782… exact root of the 16th-degree polynomial; no sector j ≤ 40 below λ_t between t_A and 2.75 | t*: double precision; t_A: exact symbolic determinant; scan: double precision | `scripts/ricci_su2_floor_family_scan.py`, `scripts/ricci_su2_tstar_exact.py`, `scripts/ricci_su2_tA_scan.py` | F/`scan.json`, F/`tstar_exact.txt`, F/`tA_scan.txt` | `python scripts/ricci_su2_floor_family_scan.py out/scan.json 40`; `python scripts/ricci_su2_tstar_exact.py out/tstar_exact.txt`; `python scripts/ricci_su2_tA_scan.py out/tA_scan.txt` | min; **h** (symbolic determinants); about 10 min | t_A exact; rest numerical |
| Cor. 6.9 (floor gap (240−24t)/t; zeros t_j; softest sector), Rem. 6.10 (Casimir point) | algebra on Prop. 6.1 | — (the cutoff is also Lean `floorLaw_cutoff`, branch `feat/helike-lean`) | — | — | — | exact where Thm. 6.4/6.7 apply |
| Fig. 3, floor family | closed forms | `docs/papers/berger-spectrum/make_figure_floor_family.py` | — | `python docs/papers/berger-spectrum/make_figure_floor_family.py` | s | exact |
| "Further observations": flow, resting fields, symmetry does not make the floor (meron is the floor of no sector; R₀ split, checked to 4e-15) | double precision | `scripts/ricci_su2_soft_mode_flow.py`, `scripts/ricci_su2_resting_fields.py`, `scripts/ricci_su2_resting_fields_general.py`, `scripts/ricci_su2_meron_family_soft.py`, `scripts/ricci_su2_meron_index_below1.py`, `scripts/ricci_su2_test2_family.py`, `scripts/ricci_su2_gyromagnetic_split.py`, `scripts/ricci_su2_meron_ceiling_check.py` | F/`soft_mode_flow.json`, F/`resting_fields.json`, F/`resting_fields_general.json`, F/`meron_family_soft.json`, F/`meron_index_below1.json`, F/`test2_meron.json`, F/`test2_berger_control.json`, F/`gyromagnetic_split.txt`, F/`meron_ceiling_check.txt` | per code-README | min each | "double precision; not certified" |
| **§7 (new in v3)** Prop. 7.1, twist across the family, closed form | as paper 1, Prop. 3.2 | `scripts/ricci_su2_twist_family_closed_form.py` | F/`twist_family_closed_form.txt` | §1.3 | — | exact |
| Family generality (commutant) | as paper 1, Prop. 2.1 | `scripts/edge_wp4_design_checks.py` | `research/edge-theorem-20261121/wp4_design_checks.json` | §1.3 | — | exact (receipt not named in this paper) |
| Prop. 7.2, the edge polynomial: P = P₁P₂ over ℚ(√2,√3); P₂ degrees (7,8,16); on the Berger path P₂ contains the t_A polynomial | exact determinant, factorization, polynomial division | `scripts/ricci_su2_edge_polynomial.py` | F/`edge_polynomial.txt`, `.srepr` | §1.3 | — | exact |
| "The edge, numerically": x* in j ≤ 12; H+GG* edges −0.68, −0.38, −0.22 and gauge-orthogonal −0.66, −0.36, −0.19 at t=2,3,4 (y=√t); x>0 loses (sector 120 falls 0.58 below); conjecture fails near t=2 (sector 16); region bounded below; Berger crossings at t_A (to 8e-16) and t* (to 1e-12) | double precision, except the x>0 argument (symbolic leading term) | `scripts/ricci_su2_exact_edge.py`, `scripts/edge_wp2_symbolic.py`, `scripts/edge_wp4_design_checks.py`; directory `research/edge-theorem-20261121/` | F/`exact_edge.json`, `research/edge-theorem-20261121/wp2_symbolic.txt`, `wp1_tail_check.json`, `workflow_reports.json` | §1.3 | — | numerical (stated) |
| Fig. 4, twist map (sectors j ≤ 8) | double precision | `scripts/ricci_su2_twist_family_map.py`, `scripts/ricci_su2_twist_family_figures.py` | F/`twist_family_map.json` | `python scripts/ricci_su2_twist_family_map.py out/twist_family_map.json [NPROC]` | not recorded | numerical |
| Thm. 7.3, the floor on a domain D | identical proof and receipts to paper 1, Thm. 6.1 | `scripts/ricci_su2_family_large_j.py`, `scripts/ricci_su2_gershgorin_lemma_symbolic.py`, `scripts/ricci_su2_family_block_cert.py`, `scripts/ricci_su2_weitzenbock_family_symbolic.py`, `scripts/ricci_su2_rows_transcription_check.py` | F/`family_large_j.json`, F/`family_block_cert_j80.json`, `research/edge-theorem-20261122-proofs/{gershgorin_lemma.json, rounding_bound.md, weitzenbock_family.json, rows_transcription.json}` | §1.3 | 63.6 CPU-h for the j < 80 part | certified |
| "What does not decide it": Tests 2, 4, 5 | as paper 1 §7 | as §1.3 | as §1.3 | — | — | numerical, pre-registered |
| §8 Checks, round sphere: lowest gauge-orthogonal eigenvalues −2, 1, 6, 7, 10 for j = 0..4 (= Kumar–Lechtenfeld–Picanço Costa); index 1; abelian control = coexact 1-form spectrum | double precision | the paper names no script for the j = 0..4 slice values. The abelian round-sphere control is `scripts/ricci_su2_spin_sector.py` (L/`spin_sectors.json`, key `control_round_abelian`) | **SOURCE NOT FOUND** for the list −2, 1, 6, 7, 10 as a receipt entry | — | — | numerical |
| Table 2, lattice vs exact eigenvalues | lattice eigenvalues from *Reading the Curve*; eigenvector identification | `scripts/ricci_su2_sector_eigvec_check.py` | `research/su2-pg-stability-20261117/`, `research/su2-spectrum-n3-20261119/`, L/`eigvec_{L0,L1}_{native,pg}.json` | recomputing needs the meshes and lattice forces of *Reading the Curve* (doi:10.5281/zenodo.24328085) | — | measured |

### 2.3 Dependency chain

```
Prop. 3.1 (exact j=0 sector)            Prop. 4.1 (exact eigenvector, every j)
            \                                 /
             Lemma 5.1 (Weitzenböck at t=3, exact in U(su(2)))
                    ├─ Thm. 5.2(1) tail bound
                    ├─ Thm. 5.2(2) certified inertia (gap_certificates.json + outward.py)
                    └─ Thm. 5.2(3) = (1) + (2) + Prop. 3.1  →  Cor. 5.4

Prop. 6.1 (floor law, exact)  →  Prop. 6.2 (Weitzenböck for all t; Landau level)  →  Cor. 6.9, Rem. 6.10
Thm. 6.4 = Thm. 5.2 machinery at t=4
Lemma 6.5 (charge blocks, exact) → Rem. 6.6
      └─ Thm. 6.7: block interval certificates (j ≤ 99) + closed-form Gershgorin bisection (j ≥ 100)
                   [independent check: full-matrix certificate, 15 sectors]
Rem. 6.8: t_A exact (tstar_exact.txt); t* and the scans numerical

§7 (v3): Prop. 7.1, Prop. 7.2 exact; edge numerics; Thm. 7.3 = paper 1 Thm. 6.1 (same chain as §1.4)
```

### 2.4 What changed from v2 (the redline)

All v3 changes are marked in `paper_v3_draft_redline.pdf`:

- Abstract: one inserted paragraph on circle-symmetric fields and the edge.
- Introduction: one new bullet ("Where the twist is the floor") and a pointer to §7.
- Remark after Prop. 6.2: Nielsen–Olesen resemblance, "resemblance only".
- Fig. 2 (`fig_three_pieces`): new.
- New §7 "Where the twist is the floor": Props. 7.1, 7.2, the numerical edge, Fig. 4 caption sentence on x>0
  artefacts, Thm. 7.3, "What does not decide it".
- In §7: "to 9×10⁻¹⁶" changed to "to 8×10⁻¹⁶"; "smallest certified margin" changed to "per-box slack … not a
  spectral margin"; the agreement statement changed from "with the general sector builder to 1.7e-13" to "interval
  blocks vs double-precision closed-form blocks (a check of the interval evaluation)", plus the rows proved exactly.
- Discussion: open question on the edge as a theorem.
- Code availability: one bullet for §7's scripts.
- Bibliography: Avramidi–Collopy, Nielsen–Olesen, Savvidy added.

### 2.5 Caveats to check

1. **Duplicate bibliography entry.** `nielsenolesen1978` appears twice (once inserted for v3, once already present);  **[FIXED 23 Nov, before posting.]**
   LaTeX warns "multiply defined", and the clean v3 PDF prints the reference twice.
2. **Code availability points at tag `floor-theorem-v2`**, "the scripts are on that tag". The §7 scripts added in v3  **[FIXED 23 Nov, before posting.]**
   (and `ricci_su2_gershgorin_lemma_symbolic.py`, `ricci_su2_weitzenbock_family_symbolic.py`,
   `ricci_su2_rows_transcription_check.py`, `edge_wp*`) are not on that tag; a v3 tag is needed.
3. **zenodo/code-README.md numbering is off by one** for two rows, in both v2 and v3: it calls the floor law on
   [2.75, 10] "Theorem 6.6" (it is Thm. 6.7; 6.6 is the Remark "Why the floor formula is exact") and "Below the range"  **[FIXED 23 Nov, before posting.]**
   "Remark 6.7" (it is Rem. 6.8).
4. **zenodo/code-README.md run times do not match the receipts** in places: it says `ricci_su2_gap_certificates.py`
   takes "about 20 s" (receipt 321.9 s) and `ricci_su2_gap_certificates_t.py` "about 1 min" (receipt 741.3 s); it  **[FIXED 23 Nov, before posting.]**
   calls the block certificates "about 0.8 CPU-hours" and "about 2.1 CPU-hours", which are close to wall time on 8
   workers, while the summed per-sector CPU time in the receipts is 6.7 h and 15.1 h. Machine-dependent, but the
   "CPU-hours" label is wrong.
5. **"1,091 slices tile [2.75, 10] exactly in each of the fifteen sectors"** (proof of Thm. 6.7). The receipt has  **[FIXED 23 Nov, before posting.]**
   1,091 slices *in total* over the fifteen sectors (38 for j=0, 138 for j=1, …), each sector tiled exactly. The
   sentence can be read as 1,091 per sector.
6. **Thm. 7.3 (domain)** carries every caveat of paper 1 §1.6 items 5 and 6; it does not depend on (T).
7. **§7's numerical edge paragraph** says x* is a root of P₂ "to 10⁻⁸"; paper 1 reports 2.4e-13 on the grid edges.
   Not a contradiction, but the two papers state different precisions for the same check.
8. **Round-sphere check**: see the SOURCE NOT FOUND entry in §2.2.

---

## 3. *Three Ways Across a Turn*, version 2

### 3.1 Build

```bash
python docs/papers/chord-arc-tangent/verify.py > out/verify_output.txt      # 68/68 PASS; about 12 s
GIGI_RICCI_DATA_ROOT=<checkout with output/> python docs/papers/chord-arc-tangent/face_angles.py   # writes face_angles.json
GIGI_RICCI_DATA_ROOT=<checkout with output/> python docs/papers/chord-arc-tangent/make_figures.py   # figures/*.pdf
cd docs/papers/chord-arc-tangent && pdflatex paper.tex && pdflatex paper.tex                     # 19 pages
```

- Packages: `../dgstyle.tex` plus `geometry`, `graphicx`, `float`, `amsmath`, `amsthm`, `booktabs`, `tikz`,
  `hyperref`, `url`, `listings`, `array`. Figs. 1 and 7 are drawn inline in TikZ; the other six figures
  (`fig_sphere_3d`, `fig_lengths`, `fig_energy`, `fig_cayley_3d`, `fig_time`, `fig_face_angles`) come from
  `make_figures.py`.
- Only `fig_face_angles` needs the frozen meshes; `make_figures.py` asserts that its numbers match Table 2.
- The Russian edition `ru/` (`paper_ru.tex`, `part1–5.tex`, `paper_ru.pdf`) is unchanged since `1c4c893` and is not
  part of this repost.

### 3.2 Claims, methods, scripts, receipts

The paper's own list (§"What is claimed") says every identity is checked by `verify.py`. Receipt:
`docs/papers/chord-arc-tangent/verify_output.txt`. *Re-run: 68/68 PASS, output identical except that the committed
file prints numpy scalars as `np.float64(…)` (numpy 2 repr).*

| Claim | Method | Script / receipt | Time |
| --- | --- | --- | --- |
| Prop. 1, C < A < T on (0,π); A−C = θ³/24, T−A = θ³/12; (2C+T)/3 = θ + θ⁵/320; Snellius–Huygens inequality | symbolic series (SymPy) + dense grid | `verify.py` → `verify_output.txt` | s |
| §3: U−1 = 2i sin(θ/2)e^{iθ/2}; S + d² = 1, d = C/2, 1−S = C²/4; Cayley U = (1+iτ)/(1−iτ) | symbolic + grid to 1e-12 | `verify.py` | s |
| Prop. 2, (2C²+T²)/3 = θ² + 7θ⁶/720 + θ⁸/1008 + …; θ⁴ cancels | symbolic series | `verify.py` | s |
| §5 SU(2): \|U−1\| = 2 sin(a/2), log U = a n, Cayley with \|v\| = tan(a/2), on 2,000 random quaternions (chord/arc vs closed forms to 1e-14; Cayley to 1e-10) | double precision | `verify.py` | s |
| §6 time stepping: explicit/implicit Euler off the circle, Crank–Nicolson on it with phase lag s³/12; mirror logarithmic spirals; golden step s* ≈ 0.6515 | symbolic + numeric | `verify.py` | s |
| §7 twins, far side, Bertrand: θ⁵/2880 and the 9:1 blend; sign flip; continuation through the rim; Bertrand 1/3, 1/2, 1/4 (2 million chords each) | symbolic + Monte Carlo | `verify.py` | s |
| §7 Haar answer to Bertrand on SU(2): 1/3 − √3/(4π) ≈ 0.19550; simulation 0.19545 | exact integral + Monte Carlo (4 million) | `scripts/haar_sameness.py` (prints only; no receipt file) | 6 s on re-run (prints 0.195449 and the exact value) |
| §8 ticket game: the six one-ticket angles, Thales twin, break-even S = d² = 1/2; Table 1 and Fig. 7 | symbolic + grid | `verify.py` | s |
| Table 2, face angles on the SU(2) Berger meshes n = 1, 2, 3, 5 (480 / 3,840 / 12,960 / 60,000 faces; max a = 0.0937, …) | measured from frozen engine output | `docs/papers/chord-arc-tangent/face_angles.py` → `face_angles.json` (receipt-checked against Table 2) | s, but needs inputs not in the repository (see 3.4) |
| Fig. 8, face-angle distributions | as above | `make_figures.py` (asserts agreement with `face_angles.json`) | s |
| §9 (v2 text): with the arc in the force's recovery instead of the chord, the SU(2) force changes by 0.090%, 0.0084%, 0.0021% of its norm at n = 1,2,3, and its error (4.7%–36%) by at most 0.011 points | double precision, on the two-ring force | `scripts/ricci_su2_force_log_vs_defect.py` (named in *Reading the Curve*, `docs/papers/two-ring-pg/paper.tex`, not in this paper) | **SOURCE NOT FOUND**: no receipt for this script in `research/` |
| §10: the Cayley step's gap from the exponential falls as \|v\|³/12, observed time order 1.99, eight steps reproduce RKMK4 within 1.5% | double precision | `scripts/ricci_su2_pg_flow_cayley.py`; results `docs/RICCI_SU2_PG_FLOW_CAYLEY_RESULTS.md` (C-cay → 1/12; C-order 1.993, 1.998; N = 8 gaps 0.47% and 1.15% at n=2); receipts `research/su2-cayley-20261118/` | not re-run (flow runs) |
| Table 3, where each idea lives in GIGI (paths at commit `442c13d` of `feat/reading-the-curve`) | description of code | `src/gauge/plaquette.rs`, `src/dials.rs`, `src/gauge/ricci_simplicial.rs`, `src/gauge/ricci_link.rs`, `src/gauge/lie_exp.rs`, `src/gauge/symplectic_flow.rs`, `examples/ricci_operator_probe.rs`, `scripts/ricci_su2_pg_flow_cayley.py`, `scripts/haar_sameness.py` (all present; commit `442c13d` exists) | — |
| §11 Yang–Mills reading: sectional curvature ¼\|[X,Y]\|² (Milnor), centre {±1} | cited / standard | — | — |

### 3.3 What version 2 changes

Version 2 is the text of commit `f31870e` (19 Nov 2026, 21:28), against `1c4c893` (19 Nov, 11:17, the last commit
before it). The repository does not record which commit's PDF was uploaded on 19 November; the comparison below is
`git diff 1c4c893 f31870e -- docs/papers/chord-arc-tangent/paper.tex`.

1. Abstract and §4: the arc's square is the continuum energy "to leading order" (and "to leading order in its size;
   the holonomy alone does not fix the curvature inside a face").
2. §5: "The chord and arc agree to 1e-14" became "the computed chord and arc agree with their closed-form values to
   1e-14".
3. §8: the two-circle count stated for 0 < θ < π, with the endpoint cases (θ = π, θ = 0); "for any two points short of
   opposite".
4. §9: the force was measured directly (energy gap does not bound the force gap; the per-face derivative ratio is
   sin a/a), with the numbers in the table above; the error range is now "between 4.7% and 36%" (v1 said "2.5% to 36%").

### 3.4 Caveats to check

1. **The R6 force numbers (0.090%, 0.0084%, 0.0021%; ≤ 0.011 points) have no receipt in the repository.** The script
   exists; its output file does not.
2. **The frozen mesh inputs are not in the repository.** `face_angles.py` reads
   `output/ricci-shape-regular-20261117/berger/level{0..3}_berger_q6.{input,output}.json` under
   `GIGI_RICCI_DATA_ROOT`; only `diagnostic.json` of that directory is tracked. The table can be checked against
   `face_angles.json`, but not recomputed from this tree. (They belong to the code and data of *Reading the Curve*.)
3. **The Zenodo description is stale.** `zenodo/description.html` still says "the 2.5–36% force errors"; the v2 text  **[FIXED 23 Nov, before posting.]**
   says 4.7%–36%. `zenodo/METADATA.md` still describes the v1 file set (including `paper_ru.pdf` and a
   `perelman_ru.pdf` kept outside the repository).
4. **The Russian edition is not updated** (`ru/part5.tex` still says 2.5%–36% and lacks the R6 measurement).
5. **The title page carries no version mark** ("November 2026"); v2 is distinguishable only by its text.
6. **The Haar result** has no receipt file; `scripts/haar_sameness.py` prints to the terminal.
7. **Prior art.** The energy blend is the one statement the paper does not find in the sources it checked; the
   search is listed as open.

---

## 4. All SOURCE NOT FOUND items

| Paper | Claim | What is missing |
| --- | --- | --- |
| 1 | Lean theorems (16), Prop. 4.2, parts of Prop. 3.4 | `docs/papers/berger-spectrum/lean/` is not on `feat/reading-the-curve-update`; it exists on `feat/helike-lean` (`bbe3478`) |
| 2 | §8 round-sphere slice values −2, 1, 6, 7, 10 (j = 0..4) | no script named; no receipt entry located |
| 3 | §9 force with the arc in the recovery (0.090%, 0.0084%, 0.0021%; ≤ 0.011 points) | receipt of `scripts/ricci_su2_force_log_vs_defect.py` |
| 3 | Table 2 recomputation | frozen inputs `output/ricci-shape-regular-20261117/berger/level*_berger_q6.*.json` |
| 1 | multiplicity-two and mirror-copy checks after Prop. 3.1 | no script or receipt named in the paper |

## 5. Re-runs done for this file (23 Nov 2026)

| Script | Result vs committed receipt | Time |
| --- | --- | --- |
| `ricci_su2_weitzenbock_family_symbolic.py` | identical | 7.9 s |
| `ricci_su2_rows_transcription_check.py` | identical | 9.3 s |
| `ricci_su2_gershgorin_lemma_symbolic.py` | identical | 12.3 s |
| `ricci_su2_direct_rayleigh.py` | identical except last digits of a numpy cross-check | 3.6 s |
| `ricci_su2_lower_edge_closed_form.py` | identical | 4.4 s |
| `ricci_su2_family_cert_rounding_bound.py` | identical | 15.8 s |
| `ricci_su2_family_large_j.py … 40 20 -3 -1.5 0.5 3 3 10` | identical | 5.9 s |
| `ricci_su2_floor_large_j.py … 50 20 2.75 10` | identical | 1.4 s |
| `edge_wp3_slice_iff.py` | identical except last digits of the double-precision m(x*+1e-3) | 18 s |
| `ricci_su2_left_invariant_sector.py` | identical except last digits of the lattice comparison | 2.7 s |
| `docs/papers/chord-arc-tangent/verify.py` | 68/68 PASS; identical except numpy repr | 12 s |
| `scripts/haar_sameness.py` | prints 0.195449 (MC) and 0.19550 (exact) | 6 s |

Not re-run (hours): the domain certificate for j < 80 (63.6 CPU-h), the block certificates of paper 2 (≈ 22 CPU-h),
`ricci_su2_tstar_exact.py`, the competitor map (≈ 1.4 h), the slab proofs, and the pre-registered tests.
