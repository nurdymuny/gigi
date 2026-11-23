# Map: where everything is (23 November 2026)

This map is for reviewing the work from another machine. Every path below is either a **git path** (repository,
branch, path from the repository root) or a **Drive path** (folder, then file name). Nothing here lives only on
my laptop.

## The two places

| place | what is there |
| --- | --- |
| **Git:** `github.com/nurdymuny/gigi`, branch **`feat/reading-the-curve-update`** | every paper's source, every script, every receipt, every research note, the pre-registrations, this package's three documents |
| **Git:** `github.com/nurdymuny/gigi`, branch **`feat/helike-lean`** | the Lean proofs (`docs/papers/berger-spectrum/lean/`) |
| **Drive:** folder **"Papers — review and posting package (23 Nov 2026)"** (My Drive, top level) | the PDFs to post, their LaTeX sources as zips, and these three documents as Google Docs |
| **Drive:** the HELIKE book folder (the one with "READ ME FIRST — HELIKE handoff (2026-11-22, v5.9)") | the book (not in git): v5.9 PDFs, source zip, handoff notes |
| **Git:** `github.com/nurdymuny/chihiro-plasma`, branch **`feat/helike-direction`** | the plasma direction notes and their diagnostics (separate project) |

To get the git side: `git clone https://github.com/nurdymuny/gigi.git && cd gigi && git checkout feat/reading-the-curve-update`.
The commit the zips and PDFs were built from is in §6.

## 1. The three documents of this package

| document | git path | Drive |
| --- | --- | --- |
| **This map** | `docs/review-package-2026-11-23/MAP.md` | package folder, "MAP — where everything is" |
| **How the results were derived** (per paper: claim → method → script → receipt → command) | `docs/review-package-2026-11-23/DERIVATIONS_README.md` | package folder, "DERIVATIONS — how each result was derived" |
| **The research summary** (how it was found, day by day, with every failure and amendment) | `docs/review-package-2026-11-23/RESEARCH_SUMMARY.md` | package folder, "RESEARCH SUMMARY — how we found and derived it" |

## 2. Papers: what to post, what not yet

| paper | status | PDF (Drive, package folder) | source (Drive zip; git path) | already on Zenodo |
| --- | --- | --- | --- | --- |
| **By Enough, But Not Too Much** (the edge paper) | **new: post v1** | `By_Enough_But_Not_Too_Much_v1.pdf` | `by-enough_v1_source.zip`; git `docs/papers/by-enough/` (figures from `scripts/edge_paper_figures.py`) | not yet |
| **The Floor Theorem for the Yang–Mills Hessian at the Berger Field** | **repost as v3** | `Floor_Theorem_v3.pdf` (clean), `Floor_Theorem_v3_redline_vs_v2.pdf` (changes marked) | `floor-theorem_v3_source.zip`; git `docs/papers/berger-spectrum/paper.tex` (clean build: set `\redlinefalse`; committed build `paper_v3.pdf`; the published v2 stays as `paper.pdf`) | v1 and v2: concept DOI 10.5281/zenodo.24386613 (v2 = 24412603, 21 Nov) |
| **Three Ways Across a Turn: Chord, Arc and Tangent** | **repost as v2** (corrections made after the 19 Nov posting; English only, the Russian edition is unchanged) | `Three_Ways_Across_a_Turn_v2.pdf` | `three-ways_v2_source.zip`; git `docs/papers/chord-arc-tangent/` | v1: 10.5281/zenodo.24375338 (concept 24375337), 19 Nov |
| Reading the Curve, Not Counting the Points | **no change**: v3 on Zenodo (24386756, 20 Nov) matches git | — | git `docs/papers/two-ring-pg/` | concept 10.5281/zenodo.24328085 |
| The Davis Manifold, second edition | **not ready**: draft waiting for its adversarial review | — | git `docs/papers/davis-manifold/the_davis_manifold_v2.tex` | first edition: 10.5281/zenodo.19428406 lineage |
| The Lift Unification, second edition | **not ready**: draft waiting for its adversarial review. Also: its acknowledgment section needs my decision before any repost | — | git `docs/papers/lift-unification/lift_unification_v2.tex` | first edition on Zenodo |

## 3. Results, notes and receipts (git, branch `feat/reading-the-curve-update`)

| what | git path |
| --- | --- |
| Index of research notes (newest last) | `docs/research/README.md` |
| Index of receipt folders | `research/README.md` |
| Live status (what is running, what landed, what needs review) | `docs/research/STATUS-live.md` |
| The floor and its edge | `docs/research/exact-edge-2026-11-21.md`, `docs/research/the-twist-a-note-2026-11-21.md`, `docs/research/twist-family-map-2026-11-21.md`, `docs/research/the-basin-2026-11-22.md` |
| The theorem on the box D (proof pieces) | `scripts/ricci_su2_weitzenbock_family_symbolic.py`, `scripts/ricci_su2_gershgorin_lemma_symbolic.py`, `scripts/ricci_su2_rows_transcription_check.py`, `scripts/ricci_su2_family_cert_rounding_bound.py`; receipts `research/edge-theorem-20261122-proofs/` |
| Pre-registered tests A–F: registrations and receipts | A, B, C: `research/edge-theorem-20261121/` (`wp4_prereg_*`, `wp4_heldout.json`, `wp4_su3_free_fibre.json`, …); D, E, F: `research/edge-theorem-20261122-d/`, `-e/`, `-f/` (each holds the registration, its amendments and its receipt) |
| Pre-registered tests A–F: results notes | `docs/research/edge-prereg-{A,B,C,D,E,F}-results-2026-11-22.md` |
| Test G (running): registration | `research/edge-theorem-20261122-g/wp8_prereg_G_two_new_groups.md` (registered at commit 333d97c, after review; the review and the fixes are its §7) |
| Test G: interim note and G₂ receipt | `docs/research/edge-prereg-G-interim-2026-11-23.md`; `research/edge-theorem-20261122-g/wp8G_g2.json` and `.log` |
| Test G: code | `scripts/edge_wp8_two_new_groups.py` (driver), `scripts/edge_wp8_fast.py` (block and matrix-free evaluators), `scripts/edge_wp7_principal_circle.py` and `scripts/edge_wp7_groups.py` (Test F's code, which G's gates reproduce) |
| The far-wall diagnostic (post hoc, disclosed) | `scripts/edge_wp8_farwall_diagnostic.py`; `research/edge-theorem-20261122-g/farwall_diagnostic.json` |
| The two hands, the arc, the deep dive | `docs/research/the-two-hands-2026-11-22.md`, `docs/research/the-arc-2026-11-22.md`, `docs/research/arc-deep-dive-2026-11-22.md` (+ `research/arc-deepdive-20261122/`) |
| Edge-paper figures and its review | `scripts/edge_paper_figures.py`, `research/edge-paper-figures-20261122/`; the math review `research/edge-paper-review-20261122/` |
| Working scratch scripts (not cited as sources) | `scripts/scratch/2026-11/` (its README says what each is) |

## 4. Lean (git, branch `feat/helike-lean`)

| what | git path |
| --- | --- |
| Project root (build with `lake build`) | `docs/papers/berger-spectrum/lean/` (`lakefile.toml`, `lean-toolchain`, `README.md`) |
| The floor and edge algebra | `docs/papers/berger-spectrum/lean/Helike/` |
| Reviews of other papers' Lean checks | `.../lean/DavisManifold/`, `.../lean/SpectralBranch/`, `.../lean/NonDecoupling/`, `.../lean/Lift/` |
| The plasma Lean (Rayleigh anatomy, plateau) | `.../lean/Chihiro/` |

## 5. The book (Drive only)

- **Drive:** the HELIKE folder. Start with `READ ME FIRST — HELIKE handoff (2026-11-22, v5.9)`, then
  `HELIKE_whole_draft_with_gap_map_v5.9.pdf`. The source is `HELIKE_source_2026-11-22_v5.9.zip`.
- Older versions stay in the same folder; their handoff notes are titled "OUTDATED — see vN".

## 6. How to check that Drive and git agree

- Each source zip in the Drive package was made with `git archive` from commit **8b7371c** on
  `feat/reading-the-curve-update`, so its paths are the repository's paths.
- To compare, unzip and run `git diff 8b7371c -- <path>` against a checkout.
- PDFs were built from the same commit:
  - the edge paper: `pdflatex` twice in `docs/papers/by-enough/`;
  - the Floor v3: the clean build with `\redlinefalse`;
  - Three Ways: the committed `paper.pdf`.
