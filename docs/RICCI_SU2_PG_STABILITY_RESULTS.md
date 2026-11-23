# S2b: stability of the SU(2) two-ring force: results

2026-11-17. Spec, fixed before measurement: `docs/RICCI_SU2_PG_STABILITY_SPEC.md`.
Research script `scripts/ricci_su2_pg_stability.py`, **not an engine run**.
Receipts, including every eigenvalue: `research/su2-pg-stability-20261117/`.

The object is the generalized spectrum of `(K, B)`, where `K` is the linearised
force at the Berger field (one column per right-trivialised link direction, by
central differences, ε = 1e-6) and `B` is the native SU(2) tangent mass.

## Controls

| control | n = 1 | n = 2 |
| --- | --- | --- |
| C-grad: the assembled gradient against S2a (two-ring) | 1.7×10⁻¹⁴ | 3.0×10⁻¹⁴ |
| C-grad: the assembled gradient against native (native trial) | 4.5×10⁻¹⁰ | 4.8×10⁻¹⁰ |
| C-col: a cell-local column against a full-gradient column (two-ring) | 6.7×10⁻¹² | 1.6×10⁻¹² |
| B symmetry | 8×10⁻¹⁷ | 6×10⁻¹⁷ |

## Result

| operator | n | DOF | rough (Re λ < −10) | most negative Re λ | negative physical | modulus < 1 | max |Im λ| |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| native SU(2) Whitney | 1 | 426 | 0 | −4.48 | 6 | 62 | 0.80 |
| **two-ring PG** | 1 | 426 | **0** | −4.39 | 6 | 62 | 104 |
| removal: one ring | 1 | 426 | **207** | −8,621 | 216 | 62 | 345 |
| native SU(2) Whitney | 2 | 3,372 | 0 | −4.65 | 9 | 483 | 0.63 |
| **two-ring PG** | 2 | 3,372 | **0** | −4.63 | 9 | 481 | 150 |
| removal: one ring | 2 | 3,372 | **1,451** | −19,829 | 1,483 | 483 | 663 |

**G-rough: PASS for the two-ring force at n = 1 and n = 2.** No eigenvalue has a
real part below −10. The one-ring force fails it badly, as the spec required,
and gets worse under refinement. That is the same signature as the Abelian
one-ring (−1.35×10⁴ and −2.42×10⁴ in the paper).

**The few negative eigenvalues belong to the field, not to the method.** The
Berger field is not at rest; the flow moves it, so its linearisation has some
directions of growth. The two-ring operator's most negative real parts match the
native operator's almost exactly:

| n = 2 | five most negative real parts (modulus ≥ 1) |
| --- | --- |
| native | −4.650, −4.650, −2.504, −2.504, −1.366 |
| two-ring PG | −4.632, −4.632, −2.503, −2.503, −1.390 |

So the two-ring force adds no instability of its own. The native one adds none
either, but it is 25% inaccurate (S2a).

## Honest observations

- **The operator is non-normal.** The two-ring operator has eigenvalues with
  imaginary parts up to 104 (n = 1) and 150 (n = 2), where the native operator's
  stay below 1. That is the price of a non-symmetric (non-reciprocal) force. The
  eigenvalues show no rough growth, but they cannot rule out transient non-normal
  growth; the numerical-range certificate (the paper's dissipativity proposition)
  would, and it was not computed here.
- **The small-modulus count differs slightly.** At n = 2 it is 481 for two-ring and
  483 for native. Gauge directions are not exact zero modes at a non-stationary
  field, so this count was reported, not gated (spec).
- **Not measured:** n ≥ 3 (11,358 DOF dense), the symmetric part (numerical
  range), and the flow and time integration.

## Where this leaves the ladder

The SU(2) two-ring Petrov–Galerkin force is now **accurate (S2a: BW05 on Berger)
and stable (S2b: no rough modes at n = 1, 2)**. The native SU(2) Whitney force is
stable but stuck at 25%. One ring is accurate but unstable. That is the paper's
Abelian picture, reproduced non-Abelian.

Remaining:
- the flow and time integration against the oracle's exact trajectories;
- other metrics;
- the engine verb;
- reciprocity (the open problem).
