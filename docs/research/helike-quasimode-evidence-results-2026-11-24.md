# Quasimode review: evidence repaired, larger run still held

24 November 2026. FOCUSED SOFTWARE AND MATHEMATICAL CONTROLS, not a new physical-spectrum result.

I have corrected the four review findings before spending time on another large calculation. The physical
Hamiltonian, domain, controls and registered thresholds are unchanged. This checkpoint is for review on the
other machine; it does not launch the Mq=8 controls or release the registered grid.

## What changed

1. **A convergence label now needs evidence.** The checker refuses missing/extra cases and rungs, nonfinite
   measurements, invalid diagnostics, changed parameters, altered snapshots, false provenance, and inconsistent
   decisions or changes. It recomputes the decision rather than trusting its label. Complete valid evidence of
   nonconvergence is kept distinct from invalid evidence. Verification replaces an old PASS before reading the
   receipt, so an interruption cannot preserve that earlier PASS as the current attempt's result.
2. **Charge projection is no longer called exact without justification.** Finite polynomial degree in the log
   chart is not finite angular bandwidth. Mode 34 masquerades as mode 2 on both 16- and 32-point grids. The spec
   now requires algebraically charged physical functions or bounded projection error, including the relevant
   operator norm. A numerical charge diagnostic does not establish an exact sector.
3. **The residual and certification contract is precise.** Complex absolute squares replace plain squares;
   compute the full residual directly rather than subtracting nearly equal norms. Temple's denominator must
   be positive and separation must be stated. Numerical quadrature agreement is not an interval enclosure.
   Normalization, cutoff-shell, projection, quadrature and eigensolve errors must be bounded. The old 0.9
   grid identification requirement remains in place, with charge checks additional rather than substituting
   for it. The outer cutoff and integration design remain visibly unregistered.
4. **The orientation prediction is narrowed, not erased.** The original universal prediction is retained with
   its counterexample. For H = I - R/2 with R exchanging two states, the orientation-odd part is zero, yet
   keeping one state raises the ground energy from 0.5 to 1. Missing even couplings matter. The next hypothesis
   concerns declared omitted blocks of the actual two-grain operator, not all orientation truncations.

## What the controls demonstrate

- The original rejection subset failed in **17 assertions across seven tests**, before implementation. The
  first broader RED run also exposed the unhandled interruption; that original log is preserved, not rewritten.
- An additional RED check exposed an overflowing integer that escaped the release helper. It is now refused
  without clearing the hold.
- The final focused selection contains **96 tests**: 27 evidence tests, eight independent mathematical
  controls, 25 flux-review regressions, 18 positive-cone controls/replays and 18 boundary controls/replays.
- **15/15 mechanism-removal checks** fail assertions when safeguards are weakened; none is credited for a
  test error. The removed mechanisms include coverage, finite values, fixed thresholds, provenance, integrity,
  decision recomputation, stale-PASS invalidation and release validation.
- The complex-vector control has squared norm 1 but plain square sum 0. The off-space residual control has
  true squared residual 0.25 while the compressed matrix reports 0. These expose two distinct ways to lose
  mathematical information before reporting a plausible answer.
- On the two-state reflection control, the exact Schur correction at E=0.5 is -0.5. This explains the retained
  energy error; evaluating at the already-known E is not a new prediction. The singular-resolvent control refuses.

## What a PASS does and does not mean

The convergence verifier checks **receipt consistency only, not matrix replay, authenticity or physical
certification**. Hashes are integrity checks, not signatures. It cannot detect an author fabricating every
measurement consistently; physical evidence still needs saved matrices/states and an independent operator
check. Verification requires the recorded source checkout and its snapshots, and consumers revalidate rather
than trusting a stored verification label. Concurrent writers to an output directory are not supported.

A self-consistent NOT_CONVERGED receipt can pass verification: this means its negative conclusion is supported
by its recorded measurements, not that convergence passed. Dirty-source evidence cannot satisfy the release
helper. A bare certificate status cannot satisfy anything: no physical-certificate verifier exists yet, so
that condition stays unmet. The LocalFock method-amendment hold remains unconditional.

No new Mq=8 spectrum, physical quasimode certificate, switch result, deployment or full Rust regression is
claimed here. The previous Mq=4 to 6 gap change of 0.0034559546 still exceeds 0.003 and stays on record.

## Files and reproduction

- [Corrected quasimode spec](helike-quasimode-certification-spec-2026-11-24.md)
- [Orientation prediction and correction](helike-reflections-and-intentions-2026-11-24.md)
- [Gate amendment, written before implementation](helike-quasimode-evidence-amendment-2026-11-24.md)
- [Commands, raw RED/GREEN logs and mutation traces](../../research/quasimode-evidence-20261124/README.md)

The model and registered design files have an empty diff against f5c93266. The runner's original 0.003 and
0.95 constants are unchanged; the original grid's 0.9 and 0.015 requirements are unchanged. The source-hash
manifest accompanies the logs so this review checkpoint can be reproduced without relying on memory.
