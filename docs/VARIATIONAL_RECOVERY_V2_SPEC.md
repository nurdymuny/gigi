# V2: three rings, the change kept off the smooth field to second order (spec)

2026-11-19. Written before any V2 code ran. It follows from the count in
`docs/research/variational-count-2026-11-19.md`. Research script `scripts/ricci_variational_v2.py`
(to be written from `scripts/ricci_variational_v1.py`), **not an engine run**. This is the Abelian
control of *Reading the Curve*: round S³/2I, A = θ¹, target velocity −4a. It uses the same frozen
inputs, mass, clock, BW05 measure, smooth fields 𝒮 and hold-out as V1.

**Name.** The V1 spec reserved "V2" for V1 at n = 3, 5. That label is superseded: this is V2. A
refinement to n = 3 follows only if V2 passes here.

## In plain English

V1 failed because it had nowhere honest to put its change. At two rings on the coarsest mesh, the
fit's spare clues are exactly the curved, second-order part of the true field. To satisfy the
column condition, V1 had to read that curvature as signal, and the error exploded (432%).

V2 makes two changes and nothing else:
1. **It reads three rings**, giving 50–64 clues instead of 26–29.
2. **It forbids the change to touch any field that is smooth to second order** (linear or
   quadratic), so the change can only act on what lies beyond.

If the count is the whole story, the column condition can now be met without stealing the field's
own curvature, and the error from reading the data stays at a few percent.

## The construction

Per cell K, as in V1, with these differences:

- **Base fit.** M_K is the published closed-linear fit (equality-constrained least squares, own
  faces exact: `tw.fit_cell_rings`), built over **three** rings instead of two. The base estimator
  is otherwise unchanged; only the ring count differs.
- **The change.** M′_K = M_K + Z_K C_K N₂,Kᵀ, where:
  - Z_K is as in V1: own fluxes are kept exactly.
  - N₂,K is an orthonormal basis of the left null space of **[X_lin | X_quad]**, the face fluxes of
    the 11 closed linear fields and the 15 closed quadratic fields. These are divergence-free
    homogeneous quadratic vector fields in the cell's orthonormal chart, with fluxes computed by
    the edge-midpoint rule (exact for quadratics), as in `scripts/ricci_variational_quadcount.py`.
  - So M′_K X_lin = I and **M′_K X_quad = M_K X_quad**. The change cannot alter how the recovery
    reads any field that is smooth to second order.
- **Unchanged from V1:** the energy, the force K′ = Σ_K w_K T′_Kᵀ G_K T′_K (symmetric by
  construction), the column condition on every edge for the 23 enforced generators, and the
  minimum-norm objective.
  - The objective is deliberately V1's (minimise Σ‖C_K‖²). The count says the failure was where
    the change could go, not what it minimised, so only that is changed.

## Gates (fixed now)

- **P-recon (the count's prediction).** The reconstruction term of Proposition 1 is at most **10%**
  at n = 1 and at n = 2. V1 gave 432% and 61.6%; the published two-ring fit with C = 0 gives 4.7% and
  3.7%.
- **V2-acc.** The BW05 relative error is below Whitney's: 29.8% at n = 1 and 26.0% at n = 2.
- **V2-stable.** As V1: S3 at n = 1, 2, no negative eigenvalue of K′ on the physical space, and a
  small-eigenvalue count equal to the gauge dimension (21, 163).
- **Mechanism removal (fixed now).** The same script with N₂ replaced by V1's N (left null of X_lin
  only) at three rings: the third ring without the quadratic exclusion.
  - The prediction is that its reconstruction term is at least **3×** V2's at n = 2.
  - If it is not, the third ring alone does the work, not the exclusion. That is reported as such,
    and the count's explanation is then incomplete.

## Controls

- **C-repro.** With rings = 2 and C = 0, the code reproduces the two-ring variational force
  (45.4%, 276.3%) to 10⁻⁶ relative. With rings = 3 and C = 0 the error is reported. This is the
  "before" for V2.
- **C-copies.** No cell's three-ring neighbourhood carries two copies of the same edge
  (`cells_seeing_two_copies_of_an_edge` = 0).
  - On the 120-cell quotient at n = 1, three rings reach 30 cells, a quarter of the manifold, so
    this can fail there.
  - If it fails at n = 1, n = 1 is reported as inadmissible for three rings, and n = 2 alone
    decides the gates.
- **C-quad.** On every cell, ‖N₂ᵀX_quad‖/‖X_quad‖ ≤ 10⁻¹⁰ and ‖N₂ᵀX_lin‖/‖X_lin‖ ≤ 10⁻¹⁰.
- **C-exact.** After the change, every cell reproduces:
  - its own fluxes to 10⁻¹⁰;
  - every closed linear field to 10⁻¹⁰;
  - M′X_quad = MX_quad to 10⁻¹⁰.
- **C-count.** The number of live change directions per cell, rank(N₂ᵀD_K), is reported. The count
  predicts 24 at n = 1 and 38 at n = 2. The same number at two rings is reported too, predicted 0
  and 3.
- **C-solve.** The column condition holds after the solve, to 10⁻⁸ of its initial size.
  - Primary solve: V1's (dense eigh of AAᵀ, relative cut 10⁻¹²).
  - If its residual exceeds 10⁻⁸, the same system is solved again by `scipy.sparse.linalg.lsqr`
    (atol = btol = 10⁻¹⁴, iter_lim 10⁵). The smaller residual is used, and both are reported.
  - V1 failed this at n = 2 (6.8 × 10⁻⁵). If both solves fail, the gates are still reported, but
    the run is marked unconverged.
- **C-span.** As V1, reported only. V1's part (b) failed for a known reason: any generic smooth
  basis nearly spans the control locally.

## Reported, not gated

- ‖C_K‖, median and max.
- The amplification ‖M′_K‖₂ against ‖M_K‖₂ (median and max), at both two rings and three rings.
- The test-variation term.
- The curvature relative error.
- trace K′.
- Seconds per stage.

## Cost (measured, not gated)

The change has 8 × (faces − 26) unknowns per cell: about 430 at n = 1 and 590 at n = 2. V1 had 232
and 264. The constraint rows are the same per edge (19 median), but each edge is read by more cells.
V1 took 4,670 s at n = 2 on this machine; V2 is expected to take longer, but not predicted.

## Not done here

n = 3 and 5, SU(2), the flow, and any objective other than minimum norm.

## Amendment 1 (19 November 2026, before any V2 run)

From the base-fit probe in `docs/research/v2-review-2026-11-19.md` (C = 0 only; no V2 change has been
computed). Nothing above is relaxed.

- **Second arm, V2q.** Identical to V2 except the base: fit the 26 closed linear and quadratic
  coefficients by the same equality-constrained least squares over three rings, and keep the linear
  part (the first 11 coefficients, at the chart origin). Then M X_quad = 0, and the change, kept off
  linear and quadratic fields, leaves the quadratic response at zero. Same gates, controls and
  predictions as V2; P-recon, V2-acc and V2-stable are judged for each arm separately.
- **Solve.** `lsqr` on A (atol = btol = 10⁻¹⁴, iter_lim 10⁵) is primary; the dense eigh of AAᵀ is the
  cross-check. Both residuals are reported; C-solve uses the smaller.
- **Reported, not gated:** the three-ring Petrov–Galerkin force error for both bases.

## Amendment 2 (19 November 2026, before any V2 run)

From a check of the V2q base (`fit_maps` in `scripts/ricci_variational_baseprobe.py`, three rings,
every 20th to 50th cell). Nothing above is relaxed.

- **V2q gives up the cell's own fluxes, by design.** Keeping only the linear part of the 26-field fit
  drops the quadratic part's flux through the cell's own faces. On fields smooth to second order, the
  kept field misreproduces the cell's own fluxes by 4.8% (median, relative, n = 1 and n = 2). The
  linear base does so to 4 × 10⁻¹⁶. With a linear output, no base can both reproduce its own fluxes
  and read quadratic fields as zero: a quadratic field has nonzero own fluxes. For V2q, **C-exact's
  own-flux clause is replaced by:** the change does not alter the own fluxes (‖A(M′ − M)‖ ≤ 10⁻¹⁰),
  and the base's own-flux defect is reported. If V2q passes and V2 does not, this departure from
  *Reading the Curve*'s "own faces exact" is a design change. Its effect on the consistency argument
  is stated before any claim.
- **Mechanism removal for V2q.** The same as for V2 (N replaced by V1's N, the left null of X_lin
  only), on the quadratic-aware base. The prediction and threshold are the same: its reconstruction
  term is at least 3× V2q's at n = 2.
- **Solve (refines Amendment 1).** `lsqr` from x₀ = 0, so that it returns the minimum-norm solution
  that V1's objective asks for. It stops when the C-solve residual reaches 10⁻⁹ relative, or at
  iter_lim 10⁵; atol and btol are not the stopping rule. Its iteration count is reported.
- **Not added, recorded for later.** A 26-dimensional trial space, i.e. keeping the whole fitted
  field rather than its linear part, would reproduce the cell's own fluxes *and* read quadratic
  fields without bias. It changes the energy, the force assembly and the Petrov–Galerkin test
  pairing, so it is a separate design. It is considered only if V2q passes and the own-flux
  departure turns out to matter.

## Amendment 3 (19 November 2026, before any V2 run)

From `docs/research/v2-hybrid-base-2026-11-19.md` (base probe, C = 0 only). Nothing above is relaxed.

- **Third arm, V2h: the hybrid base.** Fit the 26 closed linear and quadratic coefficients over three
  rings (own faces exact), keep the linear part, and add the minimum-norm closed linear field carrying the
  quadratic part's own-face flux: M = M₂₆[:11] + pinv(A_lin) A_quad M₂₆[11:]. It reproduces the cell's own
  fluxes and every closed linear field exactly, so **V2's C-exact clause applies unchanged** (no Amendment 2
  replacement). The change is kept off linear and quadratic fields exactly as in V2 (N₂), so
  M′X_quad = M X_quad: the quadratic response stays the cell's own-face representative.
- **Why.** At n = 2, with C = 0, the hybrid base has the smallest reconstruction term (0.92%; V2q 1.29%,
  V2 4.78%), the smallest test-variation term (111%; 182% to 249%) and the lowest amplification (16.2), with
  own fluxes exact to 5×10⁻¹⁶.
- **Same gates, controls and removal arm** as V2 (P-recon ≤ 10%, V2-acc below Whitney, V2-stable; removal:
  N₂ replaced by V1's N on the hybrid base, reconstruction term at least 3× V2h's at n = 2). The three arms are
  judged separately; none is preferred in advance.
- **Reported, not gated:** the three-ring Petrov–Galerkin force with the hybrid base (0.85% at n = 2 with
  C = 0), as a lead for *Reading the Curve*.

## Amendment 4 (19 November 2026, after the first n = 1 arm stalled; no V2 result has been read)

**Written before any V2 result exists.** The first n = 1 arm (linear, N2) spent over 45 minutes in
`lsqr` without finishing, and its receipt was never written. Nothing is relaxed in the gates, and
nothing changes in what is solved for: still the minimum-norm least-squares C.

- **Why it stalled.** Amendment 2 stops `lsqr` only on ‖r‖ ≤ 10⁻⁹‖b‖ (atol = 0). That test can fire
  only if the column condition is exactly consistent. V1 at n = 2 already showed it may not be
  (6.8 × 10⁻⁵). For an inconsistent system the criterion is never met, so `lsqr` runs its full
  10⁵ iterations: hours per arm at n = 1, and far longer at n = 2.
- **Change.** `lsqr` (from x₀ = 0, so it still converges to the minimum-norm least-squares solution)
  stops on either of two tests:
  - ‖r‖ ≤ 10⁻⁹‖b‖ (btol = 10⁻⁹): the constraints are met.
  - ‖Aᵀr‖ ≤ 10⁻¹²‖A‖‖r‖ (atol = 10⁻¹²): the least-squares optimum is reached and no smaller residual
    exists.

  iter_lim becomes 2 × 10⁴.
- **Reported in every receipt:** which test stopped the solve (istop), the iteration count, and both
  residuals. The dense eigh of AAᵀ stays the cross-check.
- **C-solve is unchanged.** The column condition must hold to 10⁻⁸ of its initial size. If the
  least-squares optimum stops short of that, C-solve fails and is reported as such: the column
  condition is then inconsistent at this locality, which the V1 spec already names as a finding.
- **Arms run as separate processes**, one per arm, so that one slow solve does not delay the others.

