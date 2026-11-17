# RF2G: final bounded attempt, verified NO-GO

2026-11-17. The repair is parked after this single candidate. No production
changes, full-suite runs, commit, deployment, Berger run or chapter claim.

Research synthesis and pre-run math/engineering contract:
`docs/RICCI_FINAL_BOUNDED_ATTEMPT.md`. This reviews the continuum result,
lattice/mesh failures, operator diagnosis, double-cover/HHE controls and the
failed fitted variational recovery, alongside three primary research papers.

## Candidate and outcome

Minimize actual BDM1 curvature energy subject to every ORIGINAL face flux,
rather than fitting a patch target and then differentiating another energy.
The auxiliary variables are eliminated by a Schur complement. The original
edge inputs, incidence, mass, physical clock and BW05 criteria remain fixed.
There is no fitted coefficient, patch search, spectral filter or oracle field.

Reuse of hash-verified RF2F matrices kept the eight-case experiment to 5.13
seconds, excluding imports/startup, tests and replay. This is a local duration,
not a product benchmark. The experiment had a predeclared 180-second cap.

| Refinement n | Full relative error | Full mass error |
|---|---:|---:|
| 1 | 33.4651864% | 1.3239853392 |
| 2 | 23.7021842% | 0.9453390988 |
| 3 | 21.5955404% | 0.8626978018 |
| 5 | 20.5839286% | 0.8229696380 |

Measured-h orders: 0.5144660098, 0.2298924685, 0.0930057808.
The finest error is above 5%; both finest orders are below 0.5. **FAIL.**
This improves on the original 26.1324% finest Abelian error and the fitted
recovery's 80.9109%, but neither improvement is an acceptance result.

Finest residual decomposition:
- Action/load error: 0.8230720500.
- Mass/interpolation error: 0.002115869153.
- Cross inner product: -0.00008652573079.
- Vector recombination error: 1.83e-13 or less.
- q4/q6 difference: 1.778889966e-9.
- Solve correction plus independent-implementation difference: 4.266121063e-12.
- Largest changed-face-flux residual: 8.68e-19 or less.

The measured numerical errors do not explain the discrepancy. The slowing
rates provide no evidence of the prescribed convergence, but do not prove
that asymptotic convergence is impossible on all finer meshes.

## Verification

Evidence: `output/ricci-final-20261117/`.

- Four focused tests: RED before implementation, then GREEN. Independent
  dense KKT solution, full energy adjoint, stationary minimum, original flux,
  energy finite difference, zero curvature and nonfinite refusal.
- Removing the minimization makes the behavioral tests fail.
- All eight cases replayed and compared against retained numbers.
- An algebraically separate existing recovery implementation with zero target
  agrees; the finest velocity difference is 1.64e-13 or less.
- Forged PASS, missing level and nonfinite evidence refused; stale verified
  and PASS files removed before refusal.
- Source/matrix hashes checked before and after execution. No old evidence
  or mathematical operator source changed.

`VERIFIED.json` records `evidence_valid: true`, `abelian_gate: FAIL`,
`berger: NOT_RUN`, `correspondence: NOT_ESTABLISHED`, `release_gate: OPEN`.
Diagnostic SHA256:
`f41afff646bd4165527700bf316f8ca3c2239305f755009d73c6241cac7c1968`.

Reproduce in a fresh directory, with the retained source matrices available:

```powershell
$env:OPENBLAS_NUM_THREADS='1'
$env:OMP_NUM_THREADS='1'
python scripts/ricci_final_attempt.py test output/ricci-final-replay --label GREEN
python scripts/ricci_final_attempt.py run output/ricci-final-replay
python scripts/ricci_final_attempt.py verify output/ricci-final-replay
python scripts/ricci_final_controls.py output/ricci-final-replay
```

## What would justify reopening this

Russian Geom Work's independent review agrees that exact flux constraints,
field recovery and energy accuracy do not control the energy derivative
against every original Whitney variation. It recommends separating field
reconstruction error from test-field/adjoint mismatch and requiring a full
dual-mass consistency estimate before another construction. The reviewer
does not currently offer a proven unfiltered repair under the frozen rules.

This review is mathematical advice, not an independent replay of our files.
No additional diagnostic is being run in this bounded attempt. A future
proof or a clearly labeled change of research question could justify new
work; a positive-time flow test or projected initial condition would not
retroactively pass the present exact-input instantaneous-generator gate.

The supported homogeneous continuum correspondence remains a separate result.
This failure neither disproves it nor establishes general impossibility for
GIGI or for all possible lattice discretizations.
