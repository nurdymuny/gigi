# S2d status: partial (2026-11-19)

Spec `docs/RICCI_SU2_PG_SPECTRUM_N3_SPEC.md` (4938db6, before measurement). Script `scripts/ricci_su2_pg_spectrum.py`.

Finished receipts: L0_native, L0_pg, L1_native, L1_pg (the C-repro runs), L2_native.
Running: L2_pg (n = 3, two-ring: building K, then the dense eigensolve), then L2_pg_onering.

| run | rough (Re λ < −10) | min Re λ | W-left | max \|Im λ\| | C-col |
| --- | --- | --- | --- | --- | --- |
| n=1 native | 0 | −4.47513 | −4.63383 | 0.80 | 1.5e−12 |
| n=1 two-ring | 0 | −4.39425 | −4.56277 | 103.9 | 5.8e−12 |
| n=2 native | 0 | −4.65046 | −4.68206 | 0.63 | 3.6e−13 |
| n=2 two-ring | 0 | −4.63199 | −4.66440 | 150.3 | 1.3e−12 |
| n=3 native | 0 | −4.66124 | −4.67629 | 0.64 | 2.1e−13 |

C-repro (n = 1, 2 against S2b/S2c, tolerance 10⁻³ relative): min Re λ and W-left agree to about
4×10⁻⁵ (the removed finite-difference noise), rough counts identical: pass. The n = 3 gates (G-rough,
G-W) wait for L2_pg; the one-ring removal for L2_pg_onering.
