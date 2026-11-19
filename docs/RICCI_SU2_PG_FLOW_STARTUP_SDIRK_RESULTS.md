# S3e: a second-order L-stable start-up for the Cayley step (results)

2026-11-19. Spec: `docs/RICCI_SU2_PG_FLOW_STARTUP_SDIRK_SPEC.md` (e1d8e3b, before measurement).
Receipts: `research/su2-startup-sdirk-20261119/`. Research script, **not an engine run**.

## In plain English

S3d's damping start-up was only first order and made things worse, so it could not say whether the
Cayley step's N = 4 failure came from undamped stiff modes. S3e's start-up damps stiff modes just as
completely but is second order. With it, the first-step error at N = 4 halves for both forces
(native 22.4% → 8.9%, two-ring 13.1% → 6.1%), and nothing changes at N = 8. **Undamped stiff modes
are the cause of the N = 4 failure.** The native force's later alternation is not fully removed. For
the programme nothing changes: N = 8 Cayley steps already pass every gate without a start-up; a
second-order damping start-up is the remedy if fewer steps are ever wanted.

## Gap to RKMK4 at t = 0.005, 0.01, 0.015, 0.02 (%)

| run | SDIRK2 start-up | backward-Euler start-up (S3d) | none (S3b) |
| --- | --- | --- | --- |
| n=1 native N=4 | 4.06, 1.80, 1.18, 0.87 | 10.43, 1.79, 0.73, 0.13 | 8.24, 2.22, 1.45, 1.01 |
| n=1 native N=8 | 1.11, 0.52, 0.32, 0.24 | 2.81, 0.83, 0.39, 0.18 | 1.43, 0.59, 0.35, 0.26 |
| n=1 two-ring N=4 | 1.99, 1.03, 0.68, 0.47 | 5.21, 0.19, 1.15, 1.54 | 4.29, 1.40, 0.87, 0.58 |
| n=1 two-ring N=8 | 0.57, 0.30, 0.19, 0.13 | 1.26, 0.01, 0.29, 0.39 | 0.75, 0.35, 0.21, 0.15 |
| n=2 native N=4 | **8.93**, 1.48, 2.42, 1.72 | 12.46, 5.01, 0.46, 0.39 | 22.41, 0.44, 4.09, 1.62 |
| n=2 native N=8 | 0.77, 0.53, 0.48, 0.47 | 2.60, 2.61, 2.25, 1.77 | 0.02, 0.35, 0.43, 0.47 |
| n=2 two-ring N=4 | **6.12**, 1.97, 2.10, 1.80 | 67.35, 25.60, 13.49, 9.31 | 13.12, 2.20, 2.75, 2.18 |
| n=2 two-ring N=8 | 1.05, 0.61, 0.54, 0.48 | 1.03, 0.74, 0.39, 0.05 | 1.15, 0.68, 0.60, 0.54 |

Every step of every run converged.

## Predictions

| prediction | measured | verdict |
| --- | --- | --- |
| P-first: n = 2, N = 4 first gap below half of S3b | native 8.93 < 11.2; two-ring 6.12 < 6.6 | **PASS** (both) |
| P-order: below S3d's backward-Euler start-up | native 8.93 < 12.46; two-ring 6.12 < 67.35 | **PASS** (both) |
| P-settle: native n = 2, N = 4 gap at 0.015 ≤ gap at 0.01 | 2.42 > 1.48 | **FAIL** |
| P-harmless: N = 8 E(0.02) within E/20 of S3b | 0.02, 0.01, 0.00, 0.05% | **PASS** |

By S3e's own decision rule (P-first and P-order hold), undamped stiff modes are the cause and a
second-order damping start-up is the remedy. S3d's contrary reading came from its start-up's
first-order error.
