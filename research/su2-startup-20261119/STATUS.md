# S3d status: complete, 8 of 8 runs (2026-11-19); results in docs/RICCI_SU2_PG_FLOW_STARTUP_RESULTS.md

Spec `docs/RICCI_SU2_PG_FLOW_STARTUP_SPEC.md` (9b44b89, before measurement). Gap of E(t) to S3's
RKMK4 reference at t = 0.005, 0.01, 0.015, 0.02 (%), with the backward-Euler start-up, and S3b's
values without it (from `docs/RICCI_SU2_PG_FLOW_CAYLEY_RESULTS.md`):

| run | with start-up | without (S3b) |
| --- | --- | --- |
| n=1 native N=4 | 10.43, 1.79, 0.73, 0.13 | 8.24 worst |
| n=1 native N=8 | 2.81, 0.83, 0.39, 0.18 | 1.43 worst |
| n=1 two-ring N=4 | 5.21, 0.19, 1.15, 1.54 | 4.29 worst |
| n=1 two-ring N=8 | 1.26, 0.01, 0.29, 0.39 | 0.75 worst |
| n=2 native N=4 | 12.46, 5.01, 0.46, 0.39 | 22.41, 0.44, 4.09, 1.62 |
| n=2 native N=8 | 2.60, 2.61, 2.25, 1.77 | 0.02, 0.35, 0.43, 0.47 |
| n=2 two-ring N=4 | **67.35, 25.60, 13.49, 9.31** | 13.12, 2.20, 2.75, 2.18 |
| n=2 two-ring N=8 | 1.03, 0.74, 0.39, 0.05 | 1.15, 0.68, 0.60, 0.54 |

Interim reading (not the verdict; L1_pg_N8 missing):
- P-first (n=2, N=4 first gap below half of S3b): native 12.46 vs bound 11.2 → fail (narrowly);
  two-ring 67.35 vs bound 6.6 → fail badly. The start-up makes the two-ring force much *worse*.
- P-settle (native n=2 N=4 gap at 0.015 ≤ gap at 0.01): 0.46 ≤ 5.01 → pass; the alternation is gone.
- P-harmless (N=8 E(0.02) within E/20 of S3b): n=1 native 0.4%, n=1 two-ring 0.25%, n=2 native
  2.2% → pass so far.
- So the stiff-mode hypothesis explains the native alternation but not the N = 4 failure; for the
  two-ring force a first-order start-up step is far worse than the Cayley step. A first-order
  backward-Euler half-step has its own O(h²) error, which at h = 0.005 appears to dominate.
