# RF2H results: what worked, what didn't, and what's invalid

Claude, 2026-11-17. Spec: `RICCI_ADJOINT_CONSISTENT_RECOVERY_SPEC.md`. Branch
`claude/rf2h-adjoint-consistent-recovery`. **BW05 is NOT passed; no gate closure is claimed.** Only
levels 0 and 1 (n = 1, 2) were measured. Levels 2–3 were not run because the global constraint
solve does not scale (see §5). Abelian only; Berger was not run.

## 1. What was confirmed

**The two-term structure of the force error is real and measurable.** At levels 0–1, the action
error splits cleanly into:
- **(A)** recovery error tested against the test fields;
- **(B)** test-variation mismatch.

Checks on that split:
- the BDM-interpolation remainder is 8e-5 at level 0 and 2e-5 at level 1;
- the load convention is confirmed: the true field tested against Whitney equals `−load` to 8e-5.

**(Adj) works.** Imposing the test-variation condition makes (B) small and decreasing: 7.9% then
4.3% in the run below. The constraint system is consistent: (Acc) and (Adj) are satisfied to
1e-14 or better, with zero rank deficiency, on both levels.

**The quadratic recovery energy `½‖(j+zΦ)Da‖²_H` is structurally unstable.** Its recovery error is
paired with Φ-amplified test fields. Level 0 gives 501× relative error, of which 501 goes through
Φᵀ.

**The linearised energy fixes that amplification.**
`S = jᵀHj + jᵀHzΦ + (jᵀHzΦ)ᵀ`, with (Adj) taken against the Whitney field `jΦ_F`, tests the
recovery error only against the original Whitney fields. Level 0 drops from 501 to 0.44 with the
same construction. **Caveat: S is indefinite.** Generalized eigenvalues against the Whitney
stiffness reach −27 at level 0, so it is not yet a bounded-below energy. A VEM-style stabilisation
that vanishes on locally affine fluxes would fix this in principle; it was not built.

## 2. Invalid result, discarded

The first linearised run gave 44.1% at level 0 and 32.9% at level 1. **These numbers are not
valid.** The local "affine" directions were chosen by minimum norm among the 26 restricted closed
affine 2-forms of R⁴. The Abelian control's curvature dθ is itself a constant ambient 2-form
restricted to S³. So that choice can quietly favour the control's own form, which is fitting the
analytic control in effect and is forbidden by the contract.

With the local fields forced to be chart-affine to second order (`CLEAN_JETS = True`, now the
default):

| Level (h) | Whitney | RF2H-L | (A) | (B) | Recovery error, H-relative |
|---|---|---|---|---|---|
| 0 (0.388) | 29.7% | 3872% | 3873% | 86% | 418% |
| 1 (0.202) | 26.0% | 184% | 184% | 18% | 10.6% |

This variant isn't a faithful affine recovery either. Clearing the second jet with ambient forms
leaves very large third-order content (coefficients about 350), so the exactness constraints act on
non-affine fields. **Neither run is an honest measurement of an affine-exact recovery.** That needs
true chart-polynomial fields computed by quadrature in each chart (§4).

## 3. The quantitative lesson

Term (A) behaves like `c · (recovery error) / h`, with c of about 3 here. To reach under 5% at
level 3 (h = 0.082), the H-relative recovery error must be about 0.1% or less. An affine-exact
(second-order) recovery at the observed constants won't get there. The requirement is
**quadratic-exact recovery** (third order, 26 closed quadratic fields per face, radius-2 stencils),
plus (Adj).

## 4. Recommended next steps (not started)

1. **Chart-polynomial fields:** closed affine (11) and quadratic (26) vector proxies in each gnomonic
   chart, with moments by quadrature on the projective image. No ambient family at all, so the
   control's form is never privileged.
2. **Quadratic (Acc) with radius-2 stencils; affine or quadratic (Adj)**, using the linearised
   energy plus an affine-invisible stabilisation for positivity.
3. **A scalable solver.** The global (Adj) coupling makes the reduced normal matrix dense: 12% at
   level 1 (12,364 rows, 19M nonzeros). Level 1 needs a dense Cholesky; level 3 (193k rows) is
   infeasible this way. Options are overlapping-patch Schwarz iteration, or reformulating (Adj)
   edge by edge with an exactly solvable local structure.
4. Only then the full BW05 eight-case run, with mechanism removals.

## 5. Artifacts

- Code:
  - `e2e/probes/ricci_rf2h.py` (construction, solvers, energies);
  - `ricci_rf2h_experiment.py` (BW05 harness on the frozen RF2F artifacts);
  - `test_ricci_rf2h.py`.
- Tests: 5 geometry tests pass (closedness on cell images, Stokes, deck transport of moments, deck
  invariance, neighbour-image consistency). RED was recorded before implementation.
- Frozen inputs (RF2F `h/m/d/arrays`, shape-regular payloads) are read-only and hash-checked by the
  harness. No prior receipt was modified.
