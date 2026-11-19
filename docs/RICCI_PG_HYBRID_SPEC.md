# The hybrid three-ring Petrov–Galerkin force: BW05 at n = 1, 2, 3, 5, stability and numerical range (spec)

2026-11-19. Written before any gated run. Only the n = 1 smoke test below has been run; its numbers are
controls, not gates.

- **Scripts:** `scripts/ricci_pg_hybrid.py` (the fit) and `scripts/ricci_pg_hybrid_gate.py` (the gate).
- **Receipt:** `research/ricci-pg-hybrid-20261119/diagnostic.json`.
- **Kind:** a research run, not an engine run.
- **Setting:** the Abelian control of *Reading the Curve* (round S³/2I, A = θ¹, target velocity −4a).

## Where this comes from

`docs/research/v2-script-review-2026-11-19.md` on `feat/variational-force` probed three ways of reading a
cell's neighbourhood. One was a probe of the base V2 would freeze (C = 0, no variational change).

The *hybrid* reading:

- fits closed linear and closed quadratic fields (26 coefficients) over three rings, with the cell's own
  faces exact;
- keeps the linear part;
- adds back the minimum-norm closed linear field that carries the quadratic part's own-face flux.

Put into the published Petrov–Galerkin force in place of the two-ring linear fit, it gave:

| n | published two-ring PG | hybrid three-ring PG |
| --- | --- | --- |
| 1 | 6.22% | 5.47% |
| 2 | 3.83% | 0.85% |
| 3 | 2.79% | 0.401% |

The two orders between consecutive levels are 2.85 and 1.89; the published force's are 0.73 and 0.80.

That probe was exploratory:

- dense assembly, n ≤ 3 only, q6 only;
- no gate function;
- no stability.

This spec puts the hybrid force through **every check the published force passed**, unchanged, so it
can enter the paper or be dropped on the same terms.

## What changes, and what does not

**Changes:** only the per-cell map from neighbourhood face fluxes to the 11 closed linear
coefficients, `fit_cell`:

    M = M₂₆[:11] + pinv(A_lin) · A_quad · M₂₆[11:]

where:

- M₂₆ is the equality-constrained least-squares fit of the 26 closed linear and quadratic fields over
  three rings (own faces exact, the rest unweighted);
- A_lin and A_quad are the own-face flux rows;
- quadratic fluxes use the edge-midpoint rule, which is exact for quadratics.

**Unchanged:**

- the trial space (11 closed linear fields), the Whitney test functions and the force assembly
  (`pg.build`);
- the transport and neighbourhood construction (`ricci_pg_tworing.neighbourhood_rings`);
- the mass, the data, the BW05 measurement, the `numerical_gate` function and h_max;
- the condition cap (10¹²) and the own-face residual check (10⁻¹²).

**Properties that hold by construction** (proof in the review note):

- own fluxes are exact for all data;
- closed linear data are exact;
- the force is still not variational.

## Gates (fixed now)

| gate | condition | the published force's value |
| --- | --- | --- |
| **H-BW05** | `numerical_gate` passes on n = 1, 2, 3, 5 (q4 and q6) | pass |
| **H-beat** | relative force error below the published two-ring force's at every level | 6.22, 3.83, 2.79, 1.45% |
| **H-falls** | the n = 5 error is below the n = 3 error (0.401%) | — |
| **H-S3** | at n = 1, 2, 3, the small-eigenvalue count equals the Whitney gauge dimension (21, 163, 545), and every physical eigenvalue has a strictly positive real part | stable, min Re λ 4.001 / 3.994 / 3.997 |
| **H-NR** | at n = 1, 2, the smallest eigenvalue μ of the symmetric part against the mass, on the physical space, is positive (dissipative flow) | μ = 3.998 / 3.969 |

**Controls:**

- **C-repro.** At n = 1, 2, 3 (q6), the relative error equals the probe receipts
  (`research/variational-baseprobe-20261119/L{0,1,2}_three_bases.json`, `rings3_hybrid.pg_error`:
  0.0547407, 0.00847656, 0.00400658) to 10⁻⁶ relative.
  - The smoke test gives 0.0547406691481317 at n = 1, against the receipt's 0.05474066914813318.
- **C-own.** The own-face residual check passes in every cell (the script raises otherwise).
- **C-quad.** The q4/q6 velocity difference is reported. It is part of `numerical_gate`.

**Reported, not gated:** the curvature relative error, the maximum condition number of the 26-column
flux map, the observed orders, the largest imaginary part, the largest eigenvalue of the symmetric
part, and seconds.

## If a gate fails

- **H-BW05, H-S3 or H-NR fails:** the hybrid force does not replace anything. The paper keeps the
  two-ring force as its result and mentions the hybrid only as a lead, with the failing gate stated.
- **H-beat or H-falls fails (and the rest pass):** the hybrid is reported as an alternative with its
  measured numbers, without a claim of higher order.

## How it enters *Reading the Curve* (if every gate passes)

- **The two-ring force stays the paper's force for SU(2).** The covariant recovery, the non-Abelian
  force, its stability at n ≤ 3 and the flow in time were all built and measured with the two-ring fit.
  None of that is re-run here.
- **The hybrid enters as a result of the Abelian control.** It is a second-order reading:
  - one section, with its force table, stability numbers and the reason it works (from the count: two
    rings hold exactly as many clues as a second-order fit has unknowns, so a third ring is needed);
  - one line in the abstract;
  - one item in the contributions.
- **Its SU(2) version is stated as open work,** not claimed.
- **No version label appears in the paper text.**

## Not done here

- SU(2), and any flow.
- n > 5.
- The variational question: this force is a Petrov–Galerkin force, as the published one is.
