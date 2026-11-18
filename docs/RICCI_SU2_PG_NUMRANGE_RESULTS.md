# S2c: numerical range of the SU(2) two-ring force (results)

2026-11-18. The spec was fixed before measurement: `docs/RICCI_SU2_PG_NUMRANGE_SPEC.md`.
Research script `scripts/ricci_su2_pg_numrange.py`, **not an engine run**. Receipts are in
`research/su2-numrange-20261118/` (one JSON per level and trial, including the boundary
polygon of the numerical range). The provenance is idea 1 of
`docs/research/small-edges-2026-11-18.md`: Bee's Crouzeix work, the HHE/Crouzeix check on
the Ricci control, and Crouzeix–Palencia (2017).

## In plain English

S2b showed that the two-ring SU(2) force has no runaway eigenvalues, but it is lopsided
(non-normal). For a lopsided operator, eigenvalues alone cannot rule out a brief swell before
things settle. The numerical range can: its left edge bounds how fast anything can grow.

The two-ring force's left edge sits at −4.66 at n = 2, almost exactly where its slowest
eigenvalue is (−4.63), and almost exactly where the native force's left edge is (−4.68). So
**there is no hidden swell**. The only growth is the field's own motion, the same for both
forces. The one-ring force's left edge is at −19,993, so its swell is enormous, as the
removal requires. This holds at both levels measured.

## Controls

| Control | n = 1 | n = 2 | Threshold (spec) |
| --- | --- | --- | --- |
| C-chol: ‖LLᵀ − B‖/‖B‖ | 1.4×10⁻¹⁶ | 1.7×10⁻¹⁶ | 10⁻¹³ |
| C-eig: eigenvalues of Ĝ against S2b's saved ones (worst, relative to max\|λ\|) | ≤ 1.5×10⁻¹⁴ | ≤ 6.0×10⁻¹⁴ | 10⁻⁸ |
| C-W: boundary points inside every supporting line (absolute excess) | ≤ 1.1×10⁻¹¹ | ≤ 3.3×10⁻¹¹ | consistency |
| C-W: every eigenvalue right of the left edge | 0 violations | 0 violations | none |

The C-W excess is relative to numerical radii of 1,652 to 19,993. All controls pass for all
three operators at both levels.

## Result

Ĝ = L⁻¹KL⁻ᵀ is the linearised force whitened by the native tangent mass (B = LLᵀ). Its
numerical range is W(Ĝ), and the **left edge** is min Re W(Ĝ) = λ_min(½(Ĝ + Ĝᵀ)).

| Operator | n | DOF | **left edge of W** | slowest eigenvalue (min Re λ) | numerical radius w | η (non-normality) | F1 = ‖Ĝ‖/w |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| native SU(2) Whitney | 1 | 426 | −4.634 | −4.475 | 1,913 | 0.0014 | 1.000 |
| **two-ring PG** | 1 | 426 | **−4.563** | −4.394 | 1,652 | 0.134 | 1.007 |
| removal: one ring | 1 | 426 | −8,774 | −8,621 | 8,774 | 0.116 | 1.005 |
| native SU(2) Whitney | 2 | 3,372 | −4.682 | −4.650 | 7,339 | 0.0003 | 1.000 |
| **two-ring PG** | 2 | 3,372 | **−4.664** | −4.632 | 7,027 | 0.142 | 1.003 |
| removal: one ring | 2 | 3,372 | −19,993 | −19,829 | 19,993 | 0.176 | 1.008 |

**G-W (fixed before measurement): PASS at n = 1 and n = 2.** The two-ring left edge must be at
least twice the native one, and at least −10.
- n = 1: −4.563 against a bound of −9.27.
- n = 2: −4.664 against −9.36.

**Removal: one ring fails G-W at both levels, as required** (−8,774 and −19,993). The gate
therefore measures what it was built to measure.

## What it means for time stepping

- **Exact flow.** ‖e^(−tĜ)‖ ≤ e^(mt) with m = −(left edge), the numerical-abscissa bound
  with constant 1. Over the S3 window [0, 0.02], the two-ring bound is e^(4.664 × 0.02) =
  1.098: at most about 10% growth, the same as native (1.098). For one ring it is
  e^(19,993 × 0.02), which is astronomical.
- **Implicit Euler.** Its pole −1/h stays outside W while h < 1/m. That allows h < 0.214 for
  two rings and native at n = 2, but only h < 5.0×10⁻⁵ for one ring.
- **Explicit Euler** (the step at which the stepper amplifies no more than the exact flow's
  own envelope, from the boundary polygon): 2.85×10⁻⁴ for two rings and 2.73×10⁻⁴ for native,
  at n = 2. That agrees with the RK4 step limit used for S3 (about 2.8/w = 3.8×10⁻⁴), so the
  S3 step choice at n = 2 (h = 3.1×10⁻⁴ and smaller) is consistent with this measurement.
- **Crouzeix–Palencia** then bounds k steps of any rational stepper r by (1+√2) sup over W of
  |r(hz)|^k, with no symmetry needed.

## Honest observations

- **Non-normality is real, but harmless here.** η for the two-ring operator is 0.13–0.14,
  about 100–500 times the native value. But F1 = ‖Ĝ‖/w stays within 1% of 1, and the left
  edge lies within 0.04 of the slowest eigenvalue at both levels. The lopsidedness sits in
  the fast, strongly damped part of the spectrum, not at the left edge where growth would
  come from.
- **The symmetric part is indefinite for every operator** (242 negative directions for two
  rings and 248 for native, at n = 2). That is the moving Berger field, not the method: the
  field is not at rest, so some directions grow. The paper's dissipativity proposition
  (μ > 0) therefore does not apply at this field, and the honest bound is the e^(mt) envelope
  above.
- **The explicit-Euler step for one ring is not meaningful** (about 10⁶). Its envelope e^(hm)
  with m ≈ 20,000 is so permissive that any step passes the definition. This is a defect of
  the definition when m is large, noted in the WIP commit. It does not affect the gate, which
  uses the left edge.
- **Not measured:** n ≥ 3, other metrics, and the nonlinear flow. This is a linearisation at
  one field.

## Where this leaves the ladder

The SU(2) two-ring force is now:
- **accurate** (S2a: BW05 on Berger, 2.51% at n = 5);
- **stable** (S2b: no rough modes at n ≤ 2);
- **free of hidden transient growth** (S2c: left edge matching native at n ≤ 2).

Time integration (S3, the BW07 replica) is running at n = 2. Its step sizes are consistent
with the limits measured here.
