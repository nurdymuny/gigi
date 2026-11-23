# RF2F result: verified NO-GO for the first variational recovery

Date: 2026-11-17. Source base HEAD:
`0a64e2f848693221bf25f330ea4fb19041249106`, with a dirty working tree.
Exact mathematical source hashes are retained in the experiment manifest.
No production Rust operator, mapping, mass, clock or BW05 threshold changed.
No commit or deployment. Full regressions deferred as requested.

## What was built

The pre-run contract is `docs/RICCI_VARIATIONAL_RECOVERY_SPEC.md`.
The implementation uses closed degree-one polynomial two-forms in radial
cell charts, fitted to actual neighboring face fluxes with the four central
fluxes exact. A global BDM1 variational projection makes all three shared
face moments agree, preserving normal traces and closedness. Auxiliary
variables are eliminated; the reduced energy on ORIGINAL edge data is

    E(a) = 1/2 (R D a)^T H (R D a),
    velocity = -M1^-1 D^T R^T H R D a.

The transpose recovery includes the auxiliary solve derivative. It is not
the old codifferential applied to a recovered field. The fitting objective
and the physical energy are distinguished explicitly. No target field,
Maurer-Cartan basis, continuum velocity, regularization or spectral cutoff
enters the reconstruction. The same round Abelian controls are evaluated
only after construction.

Every patch selected radius one by the predeclared rank/conditioning rule.
There were no refused or skipped cells. The largest free-fit condition number
was 23.5242 on the coarsest mesh and 20.7437 on the finest, below the fixed
1e6 refusal limit. Global moment counts are 720, 5760, 19440 and 90000;
auxiliary unknown counts are 480, 3840, 12960 and 60000.

## Full-error result

The same theta_1 (pronounced thay-tuh one) control, q6 quadrature:

| Subdivision n | h maximum | Original relative error | Recovered relative error | Full mass error |
|---|---:|---:|---:|---:|
| 1 | 0.3881395154 | 29.8111% | 115.6035% | 4.5736281876 |
| 2 | 0.2016619534 | 25.9850% | 125.4204% | 5.0022754544 |
| 3 | 0.1354592602 | 25.8603% | 104.0716% | 4.1574490002 |
| 5 | 0.0815947846 | 26.1324% | 80.9109% | 3.2349125829 |

Observed orders: -0.1368202801, 0.4648933648, 0.4949634925.
Both final orders fail >=0.5; the final relative error fails <5% by a large
margin. This candidate is worse than the original at these resolutions.
There is no passing Berger result: Berger was NOT RUN, because the Abelian
gate did not unlock it. General correspondence remains NOT ESTABLISHED.

Finest exact residual decomposition (the original full mass norm):

- Full error: 3.2349125829.
- Reconstructed-action/independent-load error: 3.2348960786.
- Mass/interpolation error: 0.002115869153.
- Cross inner product: 0.00005115134074; vector recombination error <1.9e-13.
- q4/q6 velocity difference: 9.058640024e-9.
- Propagated solve-residual correction estimate: 6.580716857e-12.

This establishes that neither the measured quadrature difference nor the
linear solves explain this candidate's failure. It does not prove that no
other variational reconstruction can work.

The energy density improves to 1.9999905795 against the continuum value 2.
Its relative energy error is about 0.0004710%, despite 80.9109% flow error.
This is a concrete counterexample to accepting energy accuracy as evidence
of a repaired full generator. The other two independent round Maurer-Cartan
controls have the same full relative error to numerical precision.
A nonconstant degree-12 deck-invariant scalar potential gives an exact
gradient control: finest face-flux residual <1.9e-14 and energy <4.1e-24.
That checks a nonhomogeneous zero-curvature field, not a fourth nonzero-flow
convergence example.

## Evidence and gate strength

Evidence directory: `output/ricci-variational-20261117/abelian/`.

- Behavioral RED retained before implementation, followed by GREEN.
- 18 final focused tests pass; no skips. Receipt:
  `focused-final/FOCUSED_TESTS.json`.
- Independent triangle quadrature validates all 12 BDM moments; all 11 closed
  affine polynomial basis directions are reproduced. Independent dense KKT
  elimination and actual-mesh finite differences validate the reduced adjoint.
- Five in-memory mechanism-removal mutants produce behavioral failures:
  missing recovery adjoint, missing auxiliary solve, wrong flux normalization,
  removed exact central constraint, wrong face-vertex permutation.
- Eight quadrature/mesh cases replayed from original geometry. All stored
  sparse matrices and arrays match the fresh replay exactly. This includes
  24 smooth-control evaluations and eight exact-gradient checks.
- Six evidence attacks refused: invented error, missing level, forged numeric
  PASS, forged Berger PASS, wrong artifact digest and nonfinite data. Every
  refusal removes both stale PASS and stale verified receipts.
- Initial eight-case experiment: 51.75 seconds on this machine; this is a
  local run duration, not a product performance claim.

`VERIFIED_DIAGNOSTIC.json` says `evidence_valid: true` and
`abelian_numeric_gate: FAIL`. These are intentionally different statements.
No release PASS file is created. Diagnostic SHA256:
`9d402dc2cc5d745c78f845f060c92ba71ae39497a4aa53a12e8619b104ed413f`.

## Reproduction

From the GIGI root in PowerShell, using a FRESH experiment directory:

```powershell
$env:OPENBLAS_NUM_THREADS='1'
$env:OMP_NUM_THREADS='1'
python e2e/probes/ricci_variational_experiment.py output/ricci-variational-replay/abelian
python e2e/probes/ricci_variational_evidence.py output/ricci-variational-replay/abelian
python scripts/ricci_variational_focused_gate.py output/ricci-variational-replay/abelian/focused
python scripts/ricci_variational_evidence_attacks.py output/ricci-variational-replay/abelian
```

The frozen RF2B/C/D sources and retained evidence must remain available and
unchanged. This is an isolated numerical experiment, not a live HTTP feature.

## Next mathematical issue

The remaining requirement is the full dual-mass action/load consistency
bound, not a more accurate energy value or a larger polynomial fit alone.
Any next candidate needs a fresh spec explaining why its adjoint controls
mesh-scale variations under the ORIGINAL mass and original input links.
No parameter or patch-radius search was conducted after this failure.
Russian Geom Work is to review these results before any chapter claim.
