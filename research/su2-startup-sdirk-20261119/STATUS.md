# S3e status: partial, 7 of 8 runs (2026-11-19)

Spec `docs/RICCI_SU2_PG_FLOW_STARTUP_SDIRK_SPEC.md` (e1d8e3b, before measurement). Gap to RKMK4 (%)
at t = 0.005, 0.01, 0.015, 0.02, with the SDIRK2 start-up:

| run | SDIRK2 start-up | backward-Euler start-up (S3d) | no start-up (S3b) |
| --- | --- | --- | --- |
| n=1 native N=4 | 4.06, 1.80, 1.18, 0.87 | 10.43, 1.79, 0.73, 0.13 | 8.24, 2.22, 1.45, 1.01 |
| n=1 native N=8 | 1.11, 0.52, 0.32, 0.24 | 2.81, 0.83, 0.39, 0.18 | 1.43, 0.59, 0.35, 0.26 |
| n=1 two-ring N=4 | 1.99, 1.03, 0.68, 0.47 | 5.21, 0.19, 1.15, 1.54 | 4.29, 1.40, 0.87, 0.58 |
| n=1 two-ring N=8 | 0.57, 0.30, 0.19, 0.13 | 1.26, 0.01, 0.29, 0.39 | 0.75, 0.35, 0.21, 0.15 |
| n=2 native N=4 | **8.93**, 1.48, 2.42, 1.72 | 12.46, 5.01, 0.46, 0.39 | 22.41, 0.44, 4.09, 1.62 |
| n=2 native N=8 | 0.77, 0.53, 0.48, 0.47 | 2.60, 2.61, 2.25, 1.77 | 0.02, 0.35, 0.43, 0.47 |
| n=2 two-ring N=4 | **6.12**, 1.97, 2.10, 1.80 | 67.35, 25.60, 13.49, 9.31 | 13.12, 2.20, 2.75, 2.18 |
| n=2 two-ring N=8 | running | 1.03, 0.74, 0.39, 0.05 | 1.15, 0.68, 0.60, 0.54 |

Interim (P-harmless needs the last run, n=2 two-ring N=8):
- P-first: native 8.93 < 11.2 PASS; two-ring 6.12 < 6.6 PASS (narrowly).
- P-order: native 8.93 < 12.46 PASS; two-ring 6.12 < 67.35 PASS.
- P-settle: native 2.42 at 0.015 > 1.48 at 0.01: FAIL.
- Reading so far: with a second-order L-stable start-up the N = 4 first-step gap halves for both
  forces, which supports undamped stiff modes as the cause once the start-up's own error is removed.
