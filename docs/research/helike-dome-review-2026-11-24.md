# Review of the dome result: a projected-tail diagnosis and a candidate regime

24 November 2026. **POST HOC REVIEW. It does not change the registered verdict, which stays
`UNRESOLVED_TRUNCATION`.** The measurements below use only overlaps of trial functions: no Hamiltonian, no
residual, no hypothesis outcome. Code: `scripts/ym_helike_dome_truncation.py`. It is a separate module, so the
registered sources stay byte-identical.

## The registered record is sound

[The results](helike-dome-envelope-results-2026-11-24.md) report every comparison without selection. They keep
the secondary K-sensitivity failure as a gate failure instead of relabelling the outcome MIXED, and they separate
finite-space improvement from per-function efficiency. I agree with the verdict.

## Why K = 30 → 36 could flip a verdict

The total-norm change can look tiny while the part outside the origin basis changes appreciably. The diagnostic
below compares two finite Haar polynomials; it does not bound the infinite series. The earlier draft quoted
per-term upper bounds without a retained derivation or certificate. Those bounds are withdrawn pending that
calculation. Small total-norm change alone cannot close the truncation gate.

But a dome function lies almost entirely inside the origin basis. What it adds is the small component outside
that span. A tail can therefore matter disproportionately outside it. Measured against that component,
for m = 1 with the selected widths and the reference K_ref = 48:

| g² | N | Outside fraction of the dome function (norm²) | K = 24 | K = 30 | K = 36 | K = 42 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 12 | 4.2×10⁻⁹ | 6.5×10⁻² | 7.9×10⁻⁴ | 4.0×10⁻⁶ | 1.2×10⁻⁸ |
| 1 | 18 | 2.9×10⁻¹⁵ | 1.2 | **1.4×10⁻¹** | 2.0×10⁻³ | 9.7×10⁻⁶ |
| 2 | 12 | 9.0×10⁻¹⁰ | 3.6×10⁻³ | 2.5×10⁻⁵ | 6.2×10⁻⁸ | 8.0×10⁻¹¹ |
| 2 | 18 | 1.3×10⁻¹⁴ | 3.0×10⁻² | 3.0×10⁻⁴ | 3.6×10⁻⁶ | 1.1×10⁻⁸ |

This matches the gate table in the results. At g² = 2 the K = 30 outside ratio at N = 18 is 3×10⁻⁴, and no
verdict changed with K. At g² = 1 it is 14%, and the one reversal happened there.

The last four columns are ‖P⊥(Φ_K_ref − Φ_K)‖ / ‖P⊥Φ_K_ref‖.

At g² = 1, N = 18, the K = 30 change is about 14% of the finite-reference exterior norm of this one function.
That is a plausible sensitivity mechanism, not an attribution of the entire residual discrepancy. The five-function
enriched span can contain nearly dependent combinations, and L² agreement alone does not control Hamiltonian
residuals. A future gate needs reference-order stability, a joint-span check and Hamiltonian graph-norm control,
not just this single-function ratio. The [bounded review protocol](dome-local-review-protocol-2026-11-24.md)
repeats these measurements with raw-overlap replay, a higher reference and higher precision; it does not silently
turn a finite reference into an infinite-tail certificate.

## Where the dome belongs

The weak-coupling note ([helike-second-vacuum-weak-coupling](helike-second-vacuum-weak-coupling-2026-11-24.md))
suggests why the controls were a hard case.
- At g² = 1 and 2 the second vacuum sits at |q| = √3 and √1.5. That is well inside the origin basis's reach, and
  the barriers are 3/32 and 3/64. At N = 18 the quoted exterior fractions for m = 1 are approximately
  2.9×10⁻¹⁵ and 1.3×10⁻¹⁴ of the finite-reference function's norm², respectively.
- At weak coupling the second vacuum moves out to √3/g, and the barrier 3/(32g²) separates the vacua.
- An exact local count gives a candidate energy difference of 1; identifying it with the global scalar gap still
  requires a spectral calculation. The saved scan shows the scalar's ⟨det q⟩ at about 85% of the orbit value at
  g² = 0.5. That moment ratio is not an 85% localization probability.

That is a candidate regime in which a seated envelope may carry more of the state instead of adding a sliver.

## Recommendations

1. Keep the dome verdict as registered.
2. Before another physical comparison, freeze a truncation rule covering exterior norm, joint span, Hamiltonian
   graph norm and the infinite tail; the present diagnostic covers only the first and finite-reference sensitivity.
3. First define a globally normalizable envelope with the normal harmonic inverse-width coefficients from Hess W:
   1 for breathing and 2 for shape. Zero tangent stiffness does not license a degenerate ambient Gaussian.
   Preserve the orbit symmetry and all kinetic derivatives. Then register held-out couplings and acceptance rules.
4. Check the literature on glueballs on the three-sphere with a sphaleron barrier (van Baal and collaborators)
   before any novelty claim.
