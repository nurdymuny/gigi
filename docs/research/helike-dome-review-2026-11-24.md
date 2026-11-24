# Review of the dome result: why the truncation gate failed, and where the dome belongs

24 November 2026. **POST HOC REVIEW. It does not change the registered verdict, which stays
`UNRESOLVED_TRUNCATION`.** The measurements below use only overlaps of trial functions: no Hamiltonian, no
residual, no hypothesis outcome. Code: `scripts/ym_helike_dome_truncation.py`. It is a separate module, so the
registered sources stay byte-identical.

## The registered record is sound

[The results](helike-dome-envelope-results-2026-11-24.md) report every comparison without selection. They keep
the secondary K-sensitivity failure as a gate failure instead of relabelling the outcome MIXED, and they separate
finite-space improvement from per-function efficiency. I agree with the verdict.

## Why K = 30 → 36 could flip a verdict

The total truncation error of a dome function is tiny. A per-term bound on the Haar series (the dropped terms are
homogeneous, so the triangle inequality applies) gives a relative norm change of at most 6×10⁻⁸ at K = 30 for
g² = 1, β = 2, and 1×10⁻⁹ for g² = 2, β = 3. Judged by its total norm, K = 30 looks converged.

But a dome function lies almost entirely inside the origin basis. What it adds is the small component outside
that span, and the truncated terms (degree 31 and up) live mostly out there. Measured against that component,
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

At N = 18, K = 30 changes 14% of everything the dome adds. A verdict comparing that addition with ordinary
expansion can then depend on K, and it did. The right truncation criterion is the outside-origin ratio at the
largest cutoff used, not the total-norm ratio. A future protocol should freeze K by that ratio (for example, below
10⁻⁶ at the top cutoff) before any comparison is computed.

## Where the dome belongs

The weak-coupling note ([helike-second-vacuum-weak-coupling](helike-second-vacuum-weak-coupling-2026-11-24.md))
shows why the controls were the hard case.
- At g² = 1 and 2 the second vacuum sits at |q| = √3 and √1.5. That is well inside the origin basis's reach, and
  the barriers are 3/32 and 3/64. By N = 18 a dome function's new content is 10⁻¹⁵ of its norm².
- At weak coupling the second vacuum moves out to √3/g, and the barrier 3/(32g²) separates the vacua.
- An exact local count at the second vacuum predicts the scalar gap → 1 there. The saved scan already shows the
  scalar's ⟨det q⟩ at 85% of the orbit value at g² = 0.5.

That is the regime in which a seated envelope should carry the state instead of adding a sliver to it.

## Recommendations

1. Keep the dome verdict as registered.
2. For any further dome comparison, freeze K by the outside-origin truncation ratio at the top cutoff.
3. Make the next dome test the weak-coupling one, with the matched widths from Hess W: 1 for breathing, 2 for shape,
   none along the orbit. Register it with held-out couplings and the predicted limits.
4. Check the literature on glueballs on the three-sphere with a sphaleron barrier (van Baal and collaborators)
   before any novelty claim.
