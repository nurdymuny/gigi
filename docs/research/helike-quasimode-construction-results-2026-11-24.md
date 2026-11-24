# Quasimode construction: four-control audit and the remaining certificate gate

24 November 2026. **COMPUTED numerical diagnostics; NOT CERTIFIED.**
The result is `UNRESOLVED_INTEGRATION`. The full grid and LocalFock release remain held.

## What this adds

The earlier Mq=6 to 8 controls measured stability inside the finite trial spaces. This audit evaluates
the full differential Hamiltonian on the saved trial functions, including the part that can leave those
spaces. It also measures departure from the intended SO(2) charge. These are different questions from
whether two finite matrices give similar eigenvalues.

The cutoff, projector convention and numerical integration orders were frozen in
[the construction registration](helike-quasimode-construction-registration-2026-11-24.md) before the run.
Source `e21087a144b5ca9dc7fb8ecad3acdebce9e3f699` was committed and pushed first. No Hamiltonian,
coupling, cutoff placement, integration order or acceptance threshold was changed after measurement.

All four controls completed: s=1.15, t=3, g^2=0.004 and 0.002, each at Mq=6 and 8. Each control uses the
ground state, the two complex Helike partners and the competitor. Tensor integration uses 10^6 and 12^6
nodes. The two complex combinations are fixed at the first order and reused at the second.

## Results, with their limits

These are order-12 estimates of ||(H-E)psi|| for the normalized, uncut trial function, **not rigorous
upper bounds**, not eigenvalue errors, and not certified physical gaps. The two Helike partners give the
same numbers at the displayed precision. Their E is the midpoint of the selected pair's Ritz energies,
not an assumed exact eigenvalue or projected Rayleigh quotient.

| g^2 | Mq | Ground residual | Helike +/- residual | Competitor residual | State checks agreeing across orders |
| --- | --- | --- | --- | --- | --- |
| 0.004 | 6 | 0.0151472742 | 0.0487248630 | 0.0801232973 | 2 of 4 |
| 0.004 | 8 | 0.0064683924 | 0.0088214519 | 0.0176954480 | 0 of 4 |
| 0.002 | 6 | 0.0053945134 | 0.0238940775 | 0.0390186205 | 2 of 4 |
| 0.002 | 8 | 0.0021918497 | 0.0028098270 | 0.0055258600 | 0 of 4 |

The finer basis improves all these residual estimates. The Helike charge-defect estimates also fall:
0.0082334774 to 0.0011798503 at g^2=0.004, and 0.0039326495 to 0.0003644585 at g^2=0.002.
That is useful numerical evidence about the trial functions, not a proof of exact charge purity.

**12 of 16 state/control comparisons fail the registered integration-agreement check.** That check
requires every reported integral quantity to agree within 1e-9*(1+abs(fine)). The largest squared-residual
difference is 2.0329583147306622e-5, for the g^2=0.004, Mq=8 competitor. Passing that comparison would
still not provide an error enclosure. Its present failure is not waived.

All quadrature estimates of removed mass and cutoff-commutator norm are zero at the sampled nodes.
This does **not** prove zero tails, zero shell leakage, or satisfaction of the 1e-10 leakage gate.
The grid can miss the small regions that carry those integrals. Both shell contributions are present in
the evaluator and are tested on separate, nonzero manufactured fixtures.

## Independent operator checks

The reduced six-coordinate calculation was compared with direct nine-coordinate Cartesian automatic
differentiation at five fixed points per control, for four states, both with and without the cutoff.
**All 160 comparisons passed.** The largest absolute difference was 2.3592239273284576e-16, under the
unchanged 1e-9*(1+abs(Hpsi)) rule.

This checks the differential action at the selected points. It does not bound the error of a six-dimensional
integral. Some physical shell amplitudes underflow; the separate nonzero shell fixtures, rather than those
tiny physical amplitudes alone, test the commutator terms.

## Exact charge projection: the mathematical construction

The continuous SO(2) projector P_m is not replaced by a finite angular sum. For integer charges,

    ||(I-P_m)f||^2 <= ||(L-m)f||^2.

For normalized psi, an invariant cutoff eta, and genuine upper bounds t, c, r, ell on removed norm,
charge defect, full residual and cutoff commutator, respectively, the registered construction gives

    q_lower^2 = (1-t)^2 - c^2 > 0,
    ||(H-E) P_m eta psi|| / ||P_m eta psi|| <= (r+ell)/q_lower.

This uses commutation of the Hamiltonian with the continuous symmetry and of eta with the projector.
The scalar inequality is available and tested. Its inputs are not enclosed by this numerical audit, so
the stored `projection_estimates` explicitly say `bound_inputs_enclosed: false`. No exact projected
wavefunction, spectral enclosure, level identification or ordering is claimed from those estimates.

## Adversarial integration review: a specification gap

The registration's finite box covers the support of eta psi and its cutoff commutator. It does **not**
cover the uncut function. In particular, ||(1-eta)psi|| includes everything outside that box, and the
uncut residual and charge norms also have contributions outside it. Compact-box integration alone cannot
supply all the global inputs to the displayed shortcut. This was identified after the audit; it is not
silently treated as a completed pre-registered method.

The next implementation needs a separate amendment before measurement. Two mathematically valid routes
are available, but neither has been implemented or selected by new results here:

- Retain the registered shortcut and enclose its whole-domain norms, including rigorous exterior tails.
- Work directly with f=eta psi. If n_lower bounds ||f|| from below, c_upper bounds ||(L-m)f|| from above,
  and r_upper bounds ||(H-E)f|| from above, then a positive denominator gives
  r_upper/sqrt(n_lower^2-c_upper^2) as an upper bound for the normalized projected residual. Those three
  integrands are compactly supported. This avoids assuming unmeasured uncut tails are zero.

Either route still needs actual outward-rounded integration, treatment of the original model constants,
and manufactured-integral tests that reject insufficient enclosures. The fixed 128-bit / 100,000-box
proposal is not evidence that the target widths are achievable. Exhaustion must remain a refusal.
The existing model and scientific thresholds are not candidates for adjustment.

## TDD and a post-audit correction

Before the physical audit: 12 focused construction tests passed, eight deliberate mechanism-removal
mutants were caught by assertions, and 27 convergence-verifier regression tests passed. The initial
behavioral failures and the missing full-operator failure are retained with their logs.

After the audit, a new RED fixture exposed a boundary-handling omission in the pointwise oracle wrappers:
they could enter the positive chart using qq^T even for a negative-determinant input. The minimal correction
returns the specified zero extension off the positive support, including singular matrices, before entering
that chart. The uncut oracle explicitly refuses nonpositive determinant. This changes neither the positive
chart operator nor the frozen integration calculation.

After that correction: **13 focused tests pass and all nine deliberate mutants are caught by assertions**.
All 160 saved positive-sheet point checks were replayed with the correction and still agree. The full
quadrature was not rerun; the original receipt and source snapshots are preserved unchanged. This distinction
is recorded rather than assigning post-fix provenance to the earlier run.

No full Rust suite and no full physical grid were run. These are focused research-script gates, not a claim
that every repository target has passed.

## Evidence and next decision

- [Evidence directory and reproduction commands](../../research/quasimode-construction-20261124/README.md)
- [Frozen-source control receipt](../../research/quasimode-construction-20261124/controls-e21087a1/receipt.json)
- [Live stage log](../../research/quasimode-construction-20261124/controls-e21087a1/events.jsonl)
- [Saved-state integrity and point replay](../../research/quasimode-construction-20261124/replay-e21087a1.json)
- [Post-fix mutation evidence](../../research/quasimode-construction-20261124/mutations-zero-extension.json)

The receipt, snapshots and four state files have verified SHA-256 identities. Text digests use the existing
LF-normalizing `sha()` contract; NPZ digests use raw bytes. All 42 files then present in the evidence directory
were also compared byte-for-byte with the Git staging area after disabling text conversion for this archive.
The run completed in about
4 minutes 33 seconds, within the registered per-control budget. Process exit 0 means the numerical audit
completed; the physical status remains `UNRESOLVED_INTEGRATION`, `certified: false`, `full_grid_released: false`.

**Next question:** can we rigorously enclose the norm, charge defect and full residual of the cut-off
functions at the existing accuracy and resource limits? Establish that on known integrals and shell fixtures
before another physical run. Separation/target identification and the prior method-amendment gate also
remain open. The full grid is not released by this work.
