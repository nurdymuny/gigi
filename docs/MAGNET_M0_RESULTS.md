# Magnet M0: results (19 November 2026)

**Specification:** [`MAGNET_M0_SPEC.md`](MAGNET_M0_SPEC.md), including Amendment 1.
**Script:** `scripts/ricci_magnet_m0.py`. Research code, not an engine run.
**Receipts:** `research/magnet-m0-20261119/spawn/L1.json` (n = 2) and `L2.json` (n = 3), from the spawn rerun with
8 workers and BLAS threads limited. `L0.json` (n = 1) and the serial `L1.json` are kept for comparison.

The reading is a k = 1 MLS 1-form with Wendland weights and a reach fixed by the vote rule; the median reach is
12 rings.

The decision levels are n = 2 and n = 3; n = 1 is reported only. All controls pass: C-poly, C-orient and C-deriv
are 0. C-copies passes at n = 2. At n = 3 no cell reaches two copies of an edge (C_copies_cells counts zero
violating cells).

## Gates

| Gate | n = 2 | n = 3 | Rule | Verdict |
| --- | --- | --- | --- | --- |
| **P-A**: poke term, unseated arm | 51.15% | 51.06% | n = 3 no larger than n = 2 | **pass** (flat, not falling) |
| **P-A-removal**: frozen-centre (seamed) arm | 147.2% | 170.3% | must grow, or P-A is inconclusive | grows: **P-A is conclusive** |
| **P-wobble**: eigenvalues below 1, unseated | 360 | 904 | at least one at n = 2 | the wobble exists |
| **P-wobble**: eigenvalues below 1, seated | 0 | 0 | none at n = 2, 3 | **pass** |
| **P-seam**: RMS wall jump, relative | 0.034% | 0.0094% | ≤ 1%, and smaller at n = 3 | **pass** |

## Reported, not gated

| | n = 2 | n = 3 |
| --- | --- | --- |
| BW05 relative error, unseated | 50.5% | 50.1% |
| BW05 relative error, seated | 45.8% | 47.7% |
| BW05 relative error, frozen | 145.8% | 169.5% |
| Reconstruction term (k = 1; not required to converge) | 7.65% | 6.46% |
| Smallest physical eigenvalue, seated (continuum: 4) | 3.899 | 3.955 |
| Negative physical eigenvalues, any arm | 0 | 0 |
| Asymmetry of K (variational by construction) | 3e-16 | 6e-16 |

Whitney's BW05 error at n = 2 is 26.0%.

## What it means

1. **Lemma A's mechanism is confirmed.**
   - With seams (the frozen arm) the poke term grows: 147% → 170%.
   - The same reading without seams stops growing: 51.15% → 51.06%.
   - Seams are what drove the 1/h growth of every earlier variational force (276% → 330% for the seamed two-ring).
2. **The seat spring works.** It removes all 360 and 904 wobble modes. The smallest physical eigenvalue then
   approaches the continuum value 4 (3.899, 3.955). No arm has a negative physical eigenvalue.
3. **The force is exactly variational:** K is symmetric to 6e-16.
4. **It is not yet accurate.** The poke term is flat near 51%, not falling, so BW05 sits near 48–50%. That is
   worse than Whitney's 26%. By the spec's decision rule (both gates pass), the next step is k = 2.
   - Whether a higher-order reading lowers a flat poke term is the open question. The T1 follow-up (non-variational
     part of the two-ring force: smooth-in, rough-out, flat near 33%) suggests the per-edge BW05 measure, which
     counts rough output in full, may stay O(1) for any energy gradient.
   - That is the case for also scoring k = 2 on a weak-consistency criterion (force error tested against smooth
     fields) and on the flow trajectory, fixed before it runs.
