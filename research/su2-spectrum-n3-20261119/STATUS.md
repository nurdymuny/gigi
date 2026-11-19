# S2d status: partial (2026-11-19)

Spec `docs/RICCI_SU2_PG_SPECTRUM_N3_SPEC.md` (4938db6, before measurement). Script `scripts/ricci_su2_pg_spectrum.py`.

Finished receipts: L0_native, L0_pg, L1_native, L1_pg (the C-repro runs), L2_native, L2_pg.
Running: L2_pg_onering (the removal).

| run | rough (Re λ < −10) | min Re λ | W-left | max \|Im λ\| | C-col |
| --- | --- | --- | --- | --- | --- |
| n=1 native | 0 | −4.47513 | −4.63383 | 0.80 | 1.5e−12 |
| n=1 two-ring | 0 | −4.39425 | −4.56277 | 103.9 | 5.8e−12 |
| n=2 native | 0 | −4.65046 | −4.68206 | 0.63 | 3.6e−13 |
| n=2 two-ring | 0 | −4.63199 | −4.66440 | 150.3 | 1.3e−12 |
| n=3 native | 0 | −4.66124 | −4.67629 | 0.64 | 2.1e−13 |
| n=3 two-ring | 0 | −4.65076 | −4.66561 | 417.96 | 5.4e−13 |

C-repro (n = 1, 2 against S2b/S2c, tolerance 10⁻³ relative): min Re λ and W-left agree to about
4×10⁻⁵ (the removed finite-difference noise), rough counts identical: pass. 
**n = 3 gates: G-rough PASS** (no eigenvalue with Re λ < −10 among 11,358). **G-W PASS**: two-ring
W-left −4.6656 ≥ 2 × native (−9.3526) and ≥ −10. The removal (one ring) is still running.
Trend, reported: the two-ring operator's max |Im λ| grows with refinement (104, 150, 418 at
n = 1, 2, 3) while its numerical range stays at the native's; non-normality grows, transient growth
does not. C-chol 1.8e−16, C-W 0.
