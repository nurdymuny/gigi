# S2d: the linearised SU(2) force at n = 3: spectrum and numerical range (spec)

2026-11-19. Written before any measurement in this file. Research script
`scripts/ricci_su2_pg_spectrum.py`, **not an engine run**. It extends S2b
(`docs/RICCI_SU2_PG_STABILITY_SPEC.md`) and S2c (`docs/RICCI_SU2_PG_NUMRANGE_SPEC.md`), which
were measured at n = 1, 2 only. *Reading the Curve* lists that as a limitation ("stability and
the numerical range are measured at n ≤ 2").

## In plain English

S2b asked whether the two-ring SU(2) force has runaway ("rough") modes, and S2c whether its
lopsided spectrum hides transient growth. Both said no at the two coarsest meshes, and both
deferred the third mesh because its matrices are large (11,358 unknowns). This spec runs the
same two tests at n = 3 with the thresholds unchanged, computing only the quantities those
tests gate: the eigenvalues, and the left edge of the numerical range. The boundary polygon
and the other quantities S2c reported but did not gate are not recomputed.

## What is computed

For each force (native, two-ring `pg`, one-ring `pg_onering`) at the Berger field, q6:

- **K**, the linearised force, one column per right-trivialised link direction by central
  differences (S2b's `Setup.column`, step 10⁻⁶), assembled in parallel (S3b's `buildK`). Since
  the exact defect derivative (`docs/RICCI_SU2_EXACT_DEFECT_DERIVATIVE_SPEC.md`) each column is
  free of the 10⁻¹⁰ gradient noise S2b's columns carried.
- **B**, the native tangent mass (`test_ricci_operator_native.sparse_matrix`), B = LLᵀ.
- **Ĝ = L⁻¹KL⁻ᵀ** (dense). Its eigenvalues λ (the generalized spectrum of (K, B)), and the
  smallest eigenvalue of its symmetric part, W-left = min Re W(Ĝ).

## Gates (thresholds unchanged from S2b and S2c)

- **G-rough (S2b).** No eigenvalue of the two-ring operator has Re λ < −10.
- **G-W (S2c).** The two-ring W-left is at least twice the native W-left at the same level, and
  at least −10.
- **Removal (one ring).** Expected to fail G-rough. If it passes, that is reported.
- **Reported, not gated:** counts with |λ| < 1 and Re λ < 0, max |Im λ|, the five most negative
  real parts, and the number of negative eigenvalues of the symmetric part.

## Controls

- **C-repro (fixed now).** Before n = 3 counts, the same script at n = 1 and n = 2 reproduces the
  published values for native and two-ring: the rough count exactly, and min Re λ and W-left to
  10⁻³ relative (S2b `research/su2-pg-stability-20261117/L{0,1}_{native,pg}.txt`, S2c
  `research/su2-numrange-20261118/L{0,1}_{native,pg}.json`). The tolerance allows for the
  removal of S2b's finite-difference gradient noise, which entered K at about 10⁻⁷ relative.
- **C-col (S2b).** At each level and force, one Jacobian column computed from two full
  gradients agrees with the cell-local column to 10⁻⁸ relative.
- **C-chol (S2c).** ‖LLᵀ − B‖/‖B‖ < 10⁻¹³.
- **C-W (S2c).** Every eigenvalue satisfies Re λ ≥ W-left − 10⁻⁹‖Ĝ‖.

## Not measured

The numerical-range boundary polygon, the numerical radius, step-size bounds and η at n = 3;
n = 5; the flow.
