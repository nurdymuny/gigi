# S3d: is the Cayley step's N = 4 failure undamped stiff modes? (results)

2026-11-19. Spec: `docs/RICCI_SU2_PG_FLOW_STARTUP_SPEC.md` (9b44b89, before measurement). Receipts:
`research/su2-startup-20261119/L{0,1}_{native,pg}_N{4,8}_be2.json`; the runs without start-up are
S3b's receipts. Research script, **not an engine run**.

## In plain English

Partly. Replacing the first Cayley step by two damping (backward-Euler) half-steps removes the
native force's sign-flipping alternation, which is what undamped stiff modes would cause. But it
does not remove the N = 4 failure, and for the two-ring force it makes the first step much worse
(67% off instead of 13%). The stiff-mode explanation accounts for the alternation, not for the
failure. N = 8 Cayley steps remain the right choice; the start-up is not adopted.

## Gap to S3's RKMK4 reference at t = 0.005, 0.01, 0.015, 0.02 (%)

| run | with start-up | without (S3b) | E(0.02) change |
| --- | --- | --- | --- |
| n=1 native N=4 | 10.43, 1.79, 0.73, 0.13 | 8.24, 2.22, 1.45, 1.01 | 1.13% |
| n=1 native N=8 | 2.81, 0.83, 0.39, 0.18 | 1.43, 0.59, 0.35, 0.26 | 0.44% |
| n=1 two-ring N=4 | 5.21, 0.19, 1.15, 1.54 | 4.29, 1.40, 0.87, 0.58 | 0.95% |
| n=1 two-ring N=8 | 1.26, 0.01, 0.29, 0.39 | 0.75, 0.35, 0.21, 0.15 | 0.25% |
| n=2 native N=4 | 12.46, 5.01, 0.46, 0.39 | 22.41, 0.44, 4.09, 1.62 | 1.97% |
| n=2 native N=8 | 2.60, 2.61, 2.25, 1.77 | 0.02, 0.35, 0.43, 0.47 | 2.23% |
| n=2 two-ring N=4 | **67.35, 25.60, 13.49, 9.31** | 13.12, 2.20, 2.75, 2.18 | 6.97% |
| n=2 two-ring N=8 | 1.03, 0.74, 0.39, 0.05 | 1.15, 0.68, 0.60, 0.54 | 0.48% |

Every step of every run converged.

## Predictions

| prediction | measured | verdict |
| --- | --- | --- |
| P-first: first gap at n = 2, N = 4 below half of S3b's | native 12.46 (bound 11.2); two-ring 67.35 (bound 6.6) | **FAIL** (both) |
| P-settle: native n = 2, N = 4 gap at 0.015 ≤ gap at 0.01 | 0.46 ≤ 5.01 | **PASS** |
| P-harmless: N = 8 E(0.02) moves by less than E/20 | 0.44, 0.25, 2.23, 0.48% | **PASS** |

## Reading

- The native force's alternation (22.4, 0.4, 4.1, 1.6%) is gone with a damping first step (12.5, 5.0,
  0.46, 0.39%): consistent with Crank–Nicolson flipping the sign of stiff components.
- The size of the first-step error is not explained. A backward-Euler half-step is first order: its
  own local error is O(h²) and, at h = 0.005, it is larger than the stiff-mode effect it removes,
  dramatically so for the two-ring force, whose operator is strongly non-normal (|Im λ| up to 150).
- An L-stable *second-order* start-up (TR-BDF2 or a two-stage SDIRK) would separate the two effects;
  it is not run here. For the programme the conclusion does not change: N = 8 Cayley steps pass every
  gate at n = 1, 2, 3 without any start-up.
