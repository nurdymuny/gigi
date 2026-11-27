# The Lowest Note: code and data

This archive holds the code and receipts behind every **computed** statement in *The Lowest Note: A Geometric
Theory of the Yang–Mills Mass Gap*. Proved statements are proved in the text. Conjectures have no code, because
nothing here computes a conjecture into truth.

Paths are relative to the archive root, which mirrors the repository (github.com/nurdymuny/gigi, tag `lowest-note-v1`).

## Requirements
- Python 3.11 with `numpy` and `scipy` (`requirements.txt`).
- For the spectra only: Rust (stable, 2021 edition) to build `helike-lab`.
- For the GIGI demonstrations only: a running `gigi-stream` server on 127.0.0.1 (see the repository). Their results
  are included, so they need not be rerun.

## Statement → code → receipt

| Statement in the document | Reproduce with | Receipt |
|---|---|---|
| The dressing's divergence is asymptotic freedom (b₀ = 22/3); the potential part widens Helike's lead | `helike zero-point` (Rust) → `scripts/helike_m3_zero_point.py`, `scripts/helike_m3_zeta.py`, `scripts/helike_m3_deep_analysis.py` | `research/m3-zero-point-20261127/` (spectra gzipped; `background_deep.json`; `README.md`) |
| How large the kinetic term must be to reverse the order; the dressed switch | `python scripts/lowest_note_threshold.py OUT.json` | `research/lowest-note-threshold-20261127/receipt.json` |
| The bundle in GIGI: area law, gauge invariance, centre vortex, topology on the line | `scripts/gigi_lattice_gauge_demo.py`, `scripts/gigi_grid_demos.py` | `research/gigi-lattice-gauge-20261127/`, `research/gigi-grid-demos-20261127/` |
| The fast fibre in GIGI: round-sphere law, flat-point equality, b₀, the background | `scripts/gigi_m3_demo.py` | `research/gigi-m3-20261127/result.json` |
| The Helike doublet figure (introduction) | `scripts/build_lowest_note_doublet.py`; checked by `scripts/check_lowest_note_doublet.py` (run inside the git repository) | `docs/papers/ym-theory-statement/figures/helike-doublet.json` |
| The cubic-quartic bound for every compact simple group (reduced discreteness) | proof: `docs/research/h7-cubic-quartic-proof-2026-11-27.md`; checks: `scripts/check_h7_cubic_quartic.py` | `research/h7-cubic-quartic-20261127/` |
| Review and algebra checks (harmonic switch, Hessians, anomaly normalization, receipts) | `python scripts/check_lowest_note_review.py OUT` (run inside the git repository: it reads the paper and stamps its receipt with the revision) | `research/lowest-note-review-20261127/` |

The register of statements, its hypotheses and the referee's coherence report are in
`docs/research/closed-set-20261127/`. The mathematics of the bundle framing and the M3 computation is in
`docs/research/helike-bundle-framing-math-2026-11-27.md` (§14). The registered O(g²) coefficients used by the
threshold come from `docs/research/helike-flux-background-registration-2026-11-24.md`.

## Rerunning

**The threshold** (seconds, no solver):
```
python scripts/lowest_note_threshold.py /tmp/threshold.json
```
It must reproduce `research/lowest-note-threshold-20261127/receipt.json`.

**The background dressing from the saved spectra** (about two minutes):
```
gunzip -k research/m3-zero-point-20261127/zp_t3.0_j200_*.json.gz
PYTHONPATH=scripts python scripts/helike_m3_deep_analysis.py research/m3-zero-point-20261127 /tmp/background.json
```
It must reproduce `research/m3-zero-point-20261127/background_deep.json`.

**The spectra themselves** (Rust; jmax 200 takes tens of minutes):
```
cd helike-lab && cargo build --release -p helike-cli
./target/release/helike --out /tmp/zp.json zero-point --t 1 --lams 0,1 --jmax 12
```
At t = 1 and λ = 0 the slice spectrum is (k+1)², with multiplicity 6k(k+2) over sectors; λ = 1 gives the same
multiset with sectors shifted by two. That is an exact check of the build.

## What is deliberately kept
`research/m3-zero-point-20261127/` also holds two superseded analyses, labelled as such: a jmax 130 run that was not
converged at the strong background field, and a first regulator fit that used a poor asymptotic window. They are
kept because failures are part of the record.

## License
See `LICENSE`.
