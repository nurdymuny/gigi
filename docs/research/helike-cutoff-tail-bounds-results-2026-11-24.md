# Rigorous cutoff and tail bounds on the frozen controls

24 November 2026. Same model, same states, same thresholds; full grid held.

## Result in plain English

We can now bound what the cutoff removes without relying on a quadrature grid
to happen to visit the tiny tails. The bound covers the entire infinite exterior
and includes the derivative cost of trimming the state, with outward-rounded
arithmetic. These are bounds for the saved finite trial functions, not exact
eigenstates or a certificate for the full spectrum.

All sixteen states have valid, positive tail and cutoff-commutator upper bounds.
The eight states at g^2=0.002 pass the unchanged 1e-10 leakage-norm threshold.
The eight at g^2=0.004 do not yet have sufficiently small upper bounds to pass.
**This is an unresolved bound at the stronger coupling, not a measured physical
leakage failure.** An upper bound above the threshold does not imply that the
actual quantity is above it.

The all-controls CUTOFF_TAIL_BOUNDS gate stays open. Residual/charge certification,
projection and spectral separation remain separate obligations. No grid release.

## Provenance and scope

- Registration: [fixed analytic bounds](helike-cutoff-tail-bounds-registration-2026-11-24.md).
- Initial frozen implementation: `517ab5ad`; pushed before any control attempt.
- [Provenance amendment](helike-cutoff-provenance-amendment-2026-11-24.md) pins the
  previously reviewed zero-extension guard, without changing mathematics.
- Measured source: `c31aab8ed623761c42ef6ad62af85dae120fa7c0`, pushed before the run.
- Saved-state parent: `controls-e21087a1`, normalized receipt SHA-256
  `e922d6bfffea50907d1c28d7cf7cdae622bb8eda0da5e4f97fdc1c13807045ab`.
- Bound receipt: [receipt.json](../../research/cutoff-tail-bounds-20261124/controls-c31aab8e/receipt.json),
  normalized SHA-256 `93e7096f8051cb89c59b8eafe74db26d196d0d48fee78941058baaa8c4958ae8`.

The four model/chart modules match the parent source hashes exactly. The cutoff
evaluator matches the reviewed zero-extension hash. Each derived trial archive
retains the exact binary64 coefficient product, chart, frequencies, basis and
original spectral centers. Its byte hash is recorded. The original state files
are unchanged. The cutoff, eigenvalue model and all scientific thresholds are
unchanged; no eigensolver ran here.

Arithmetic: python-flint 0.9.0, Arb at 128-bit midpoint precision. Authoritative
endpoints are dyadic integer/exponent pairs. Decimal tables below are deliberately
rounded **up**. The small printed uncertainty on an endpoint's decimal display
is not the uncertainty in the unknown physical norm: that norm is enclosed
between zero and the saved upper endpoint.

## The measured bounds

Here `tail` means ||(1-eta)psi||/||psi||, and `leakage` means
||[H,eta]psi||/||psi||. Every number is an upper bound, not a point estimate.
Both Helike partners are retained independently in the receipt; their bounds
coincide, so one table row lists them together.

| g^2 | Mq | State | Tail norm upper | Leakage norm upper | <=1e-10 established? |
| --- | --- | --- | --- | --- | --- |
| 0.004 | 6 | Ground | 5.199252e-15 | 2.921986e-9 | No |
| 0.004 | 6 | Helike + and - | 8.670177e-15 | 4.868752e-9 | No |
| 0.004 | 6 | Competitor | 5.079392e-15 | 2.853159e-9 | No |
| 0.004 | 8 | Ground | 4.235081e-14 | 2.388757e-8 | No |
| 0.004 | 8 | Helike + and - | 4.809084e-13 | 2.713423e-7 | No |
| 0.004 | 8 | Competitor | 3.048094e-13 | 1.720321e-7 | No |
| 0.002 | 6 | Ground | 1.510019e-34 | 1.363270e-27 | Yes |
| 0.002 | 6 | Helike + and - | 1.842970e-34 | 1.663057e-27 | Yes |
| 0.002 | 6 | Competitor | 1.085277e-34 | 9.794884e-28 | Yes |
| 0.002 | 8 | Ground | 1.241959e-33 | 1.123416e-26 | Yes |
| 0.002 | 8 | Helike + and - | 1.975901e-32 | 1.787667e-25 | Yes |
| 0.002 | 8 | Competitor | 1.269150e-32 | 1.148433e-25 | Yes |

All tail-squared enclosure widths are below 2.313e-25. All leakage-squared
enclosure widths satisfy the separate 1e-12 integration-width budget, but eight
do not meet the stronger 1e-20 squared-leakage threshold. Norms are enclosed from
Hermite orthogonality, rather than assumed to equal one. Normalized cut-state
squared norms are bounded from below by one minus the exterior mass bound.

## Why the method is rigorous, and where it is conservative

1. Prove a whole ball in standardized Gaussian coordinates lies in eta=1. The
   fixed geometry-only radius rule gives 9.75 for g^2=0.004 and 13.75 for
   g^2=0.002, at both Mq values. This uses no state coefficients or gate outcomes.
2. Bound each saved Hermite polynomial and its physical derivatives outside the
   ball. Keep the Gaussian derivative, density derivative and inverse chart.
3. Integrate the resulting radial majorants to infinity using the finite
   upper-incomplete-gamma identity. No infinite exterior is omitted.
4. Keep all Laurent terms of the nonconstant cometric. The symbolic Cartesian
   Gram-map identity is checked independently of the bound calculation.
5. Bound both cutoff shells and the mixed Laplacian term. Use
   Geta*sqrt(gradient_tail_squared) + Leta*sqrt(exterior_mass)/2, normalized by a
   rigorous positive lower bound for the original state norm.

The incomplete-gamma expression is a standard exact identity; outward ball
arithmetic encloses its evaluation. The mathematical enclosure follows from the
inequalities in the registration, not from agreement of two sampled integrals.
See [DLMF 8.4.8](https://dlmf.nist.gov/8.4.E8) and
[Arb directed bounds](https://python-flint.readthedocs.io/en/latest/arb.html).

Conservatism is explicit: entrywise plateau bounds discard correlations between
coordinates; coefficient majorants discard polynomial cancellation; absolute
metric sums discard signs; Young's inequality enlarges exponential tails; and
global shell derivative bounds are applied over the full exterior of the ball.
The higher Mq majorant can therefore be worse even when the state's actual
residual improves. This table does not establish physical deterioration at Mq=8.

For the g^2=0.004, Mq=8 Helike states, the two contributions to the reported bound
are approximately 2.40622e-7 (gradient term) and 3.07203e-8 (Laplacian term).
Both individually exceed 1e-10. Merely improving the gradient estimate cannot
close this bound while leaving the current Laplacian estimate intact.

One justified next hypothesis is to separate the determinant transition shell
from the outer-norm shell and use each shell's own localization bound. That would
avoid combining the largest inner-shell derivative constant with all outer-tail
mass. It needs a new pre-registered enclosure argument and tests; **it was not
implemented or tuned after this result**. A tighter bound is not guaranteed to pass.

## Strict TDD and replay evidence

| Check | Result |
| --- | --- |
| Initial mathematical RED | 12 assertion failures before implementations |
| Completion/stale-PASS RED | 2 additional assertion failures before runner guards |
| Historical provenance RED | Reviewed zero-extension version rejected before hash correction |
| New focused suite, final | 19 passed |
| Existing quasimode suite | 13 passed |
| Existing centered-moment suite | 16 passed |
| Existing cone-convergence suite | 27 passed |
| Total focused tests | 75 passed; no full Rust suite run |
| Deliberate mechanism removal | 21/21 caught by assertions, zero execution-error catches |
| Exact serialized bound replay | 16/16 equal after loading the saved derived trial archives |
| Physical grid | Held throughout |

Tests include high-precision independent incomplete gamma, explicit complex
Hermite polynomials, a positive tail below binary64's representable range,
directed serialization, non-unit normalization, invalid plateau refusal,
independent symbolic physical metric, and nonzero Cartesian cutoff-shell checks.
They also reject changed model hashes, incomplete control/state coverage, forged
acceptance flags and a stale overall PASS after an exception.

The first attempted run refused before measuring a single control due to the
historical source-guard mismatch. It remains archived, not overwritten. The
subsequent run completed all controls in approximately 2.08 seconds of summed
per-control work, excluding startup and source checks. This is an execution
observation, not a performance comparison or scalability claim.

Evidence folder: [cutoff-tail-bounds-20261124](../../research/cutoff-tail-bounds-20261124/).
Live checkpoints: [events.jsonl](../../research/cutoff-tail-bounds-20261124/controls-c31aab8e/events.jsonl).
Exact replay and contribution diagnosis:
[replay-and-diagnosis.log](../../research/cutoff-tail-bounds-20261124/replay-and-diagnosis.log).

## Reproduction

Use source c31aab8ed623761c42ef6ad62af85dae120fa7c0 or its unchanged retained source
snapshot, the original hash-verified state archive, and the recorded Python
environment. Install the pinned additional dependency:

```powershell
python -m pip install python-flint==0.9.0
python -m unittest discover -s scripts -p test_helike_cutoff_bounds.py -v
python scripts/check_helike_cutoff_mutations.py NEW-mutations.json
python -u scripts/run_helike_cutoff_bounds.py research/NEW-cutoff-replay
```

The output directory must be new. The runner logs each state as it lands and
refuses dirty registered source. Exit code **3** means a completed bound run
whose all-controls cutoff gate is still open, as here; **2** means a computation
or provenance refusal. Neither is permission to relax a threshold. A restart or
cache never promotes this evidence into a complete spectral certificate.

Final receipt status:

```text
execution: COMPLETE_CUTOFF_TAIL_BOUNDS
cutoff_tail_gate_passed: false
status: UNRESOLVED_INTEGRATION
certified: false
full_grid_released: false
```
