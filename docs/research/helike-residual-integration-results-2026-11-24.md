# What caused the twelve residual-integration disagreements

24 November 2026. Numerical investigation on the same four saved controls.
**Numerical residual diagnosis PASS on all four controls. Physical certificate OPEN; full grid held.**

## The question and the controls

The preceding audit found twelve failed 10-versus-12 quadrature comparisons. Every failure was in
the full residual squared and its identical sampled cutoff residual. The stored vectors, centers,
complex charge combinations and operator are fixed. No eigensolver is rerun here.

Sources: [integration registration](helike-residual-integration-registration-2026-11-24.md),
[centering amendment](helike-residual-centering-amendment-2026-11-24.md), and the
[original audit](helike-quasimode-construction-results-2026-11-24.md).
The source commits were pushed before their respective runs: `f2ae42a3` for diagnosis and `d0297072`
for the centered correction. The original model, scientific thresholds and cutoff choices are unchanged.

## Finding 1: the residual was underintegrated

The full differential operator changes the function being integrated. Being able to integrate the
original trial norm does not establish that the same rule integrates the full residual squared.
The log-coordinate metric and potential also introduce exponentials.

A planted example makes the mechanism explicit. The normalized Hermite polynomial h_10 has squared
Gaussian norm 1. The ten-point Gauss-Hermite rule samples it at its own zeros and returns zero in exact
arithmetic, about 2.7e-31 in this implementation. The gate requires the algebraic integral to recover 1.
This is not a tolerance dispute. It is a rule being applied outside its exactness class.

For the physical controls, an independently assembled finite exponential-polynomial representation
replays the original tensor residuals. The representation also agrees with the Cartesian differential
operator at fixed points. It therefore lets us vary integration while keeping the function unchanged.
This is stronger evidence than comparing two newly fitted spectra or two similar-looking curves.

## The integration identity

Write psi=fP in the original chart, with f the Gaussian divided by the square root of the density.
The unchanged Laurent operator yields a finite sum

    (H-E)psi / f = sum_a exp(alpha_a.z) Q_a(z).

For beta=alpha_a+alpha_b, completing the square translates by mu_k=beta_k/(2w_k):

    <exp(alpha_a.z)Q_a, exp(alpha_b.z)Q_b>_Gaussian
      = exp(sum_k beta_k^2/(4w_k)) <Q_a(z+mu), Q_b(z+mu)>_Gaussian.

The remaining polynomials have finite degree. Their translated Hermite coefficients give the inner
product through orthogonality. The second implementation obtains the translation matrix by
degree-sufficient polynomial quadrature after removing the exponential. It does not repeat the original
unshifted 10/12 rule. The identity is exact in exact arithmetic; floating-point evaluation is not a
rigorous error enclosure.

## Finding 2: agreement could conceal cancellation

The first algebraic implementation summed large signed pair contributions, up to an absolute total of
about 6.4 million, to recover residual squares near 1e-5. Its two implementations could agree and still
lose digits. We did not accept that agreement as the end of the investigation.

For F(z)=10^4*(exp(10^-6*z)-1), the exact Gaussian norm formula gives approximately
0.00005000000000004375. Both first implementations returned 0.00004999339580535889: they agreed with
each other but missed the answer by about 6.6e-9. That fixture failed before repair and passes afterward.
The first run is retained, explicitly superseded as accuracy evidence; its old agreement flag is not a
current approval of those moment values.

The smallest repair preserves the identity but changes the accumulation order. Compute the common
polynomial sum before squaring. For each translated pair, evaluate the translation increments directly
and use `expm1` for the exponential increment. No large translated coefficient array is subtracted from
its rounded copy. The independent quadrature route uses a differenced Hermite recurrence for the same
reason. No new precision, fitted quadrature order, omitted coefficient or relaxed threshold is introduced.

## Results

The final source is `d02970726220fd161fbdf98047610fae18a0cb47`. All four controls completed under the
900-second per-control budget, from 22:33:21 to 22:41:30 UTC. The replay verified all 20 source snapshots
and the original saved-state identities. No eigensolves, refits or full-grid cases were added.

| Check | Outcome |
| --- | --- |
| Two centered integration paths, 16 state/control cases | All pass; largest absolute difference 1.674345211089001e-11 |
| Original 10/12 tensor values, 32 comparisons | All reproduced; largest difference 1.885210737517795e-15 |
| Expanded residual versus Cartesian operator, 80 point checks | All pass; largest difference 9.811964015940403e-16 |
| Existing 1e-9*(1+abs(reference)) comparison rule | Unchanged |

Numerical uncut residual norms from the centered moment calculation (not error enclosures):

| g^2 | Mq | Ground | Helike +/- | Competitor |
| --- | --- | --- | --- | --- |
| 0.004 | 6 | 0.01514727 | 0.04872486 | 0.08012330 |
| 0.004 | 8 | 0.00646840 | 0.00882145 | 0.01769661 |
| 0.002 | 6 | 0.00539451 | 0.02389408 | 0.03901862 |
| 0.002 | 8 | 0.00219187 | 0.00280982 | 0.00552598 |

The data trace all twelve original disagreements to integration of the same unchanged residual. In all
twelve cases the old ten-node value differs from the centered value beyond the existing comparison limit.
The twelve-node value also misses that limit for both Mq=8 competitors: squared-residual differences
4.0968523966843605e-8 at g^2=0.004 and 1.318931116314195e-9 at g^2=0.002. Dropping the coarse comparison
and trusting the old fine rule would therefore not have been a sufficient correction.

The centered accumulation reduces the absolute sum of the accumulated contributions from about
1.6-6.4 million to about 22,000-74,000. This is a conditioning diagnostic, not a rigorous rounding bound.
The falsely agreeing manufactured example and the tiny-translation example are now protected by tests.

This closes the numerical **uncut residual comparison** issue on these controls. It does not retroactively
change the old receipt or certify its sampled cutoff residual. The physical outcome remains
`UNRESOLVED_INTEGRATION`, with `certified: false` and `full_grid_released: false`.

## Gates and scope

The final focused construction suite has 16 tests. Eleven deliberate mechanism removals are caught by
assertions, including restoration of unstable subtraction, the uncentered pair calculation, omitted
complex conjugation, omitted cross terms, omitted density drift, coefficient truncation and false
certification. Initial behavioral failures, the failed first centering repair and subsequent green logs
are retained. A final combined run passed all **56 focused tests**: 16 new, 13 original construction and
27 convergence-verifier tests.
No full repository suite was run or claimed.

This correction concerns the **uncut numerical residual integral**. The smooth cutoff is not a finite
exponential-polynomial. Zero leakage at sampled nodes is still not a shell bound. Rigorous integration
of the cut-off state, normalization and charge defects, spectral separation, target identification and
the earlier method amendment remain open. No physical gap certificate or full-grid release follows.

Evidence and commands: [residual integration archive](../../research/residual-integration-20261124/README.md).
The [verification summary](../../research/residual-integration-20261124/verification-centered.json) contains
all sixteen state results, both old-rule errors, counts and hashes. The [complete receipt](../../research/residual-integration-20261124/controls-d0297072/receipt.json)
contains the intermediate moments and numerical diagnostics. Text digests follow the repository's
LF-normalized SHA-256 convention; the original NPZ state digests are over raw bytes.
