# Stable moment centering: post-diagnosis amendment

24 November 2026. Registered after the f2ae42a3 diagnostic run and before the centered implementation.
No physics, state, threshold, cutoff, integration domain or full-grid policy changes.

The first replay reproduces all original tensor residuals, but its pairwise completed-square sum has
absolute contributions up to about 6.4 million for residual squares near 1e-5. Two moment implementations
agree within 1e-9 yet can share cancellation error. Their agreement flag is therefore insufficient to
establish corrected residual accuracy. Preserve that run; do not silently endorse its moment values.

A planted counterexample proves the issue without physical data. For w=1 and
F(z)=10^4*(exp(10^-6*z)-1), the Gaussian norm squared is

    10^8 * [exp(10^-12) - 2 exp(10^-12/4) + 1]
      = 0.0000500000000000437500000000161458...

Both old moment paths return 0.00004999339580535889. Their equality is not accuracy.
This becomes a RED fixture with absolute tolerance 1e-12 (a manufactured arithmetic test, not a change
to any physical acceptance threshold). A separate tiny-translation fixture must catch subtraction
of a large constant after adding a small translation increment.

## Minimal algebraic correction

For a pair A,B and their common translation mu, compute dA=A(z+mu)-A(z), dB=B(z+mu)-B(z) DIRECTLY from
the nonconstant Taylor terms, never by subtracting two rounded large arrays. The independent route uses
the strictly degree-lowering entries of its polynomial quadrature translation matrix; its diagonal is
exactly 1 by the translation identity, not an estimated quadrature value.

The tiny-translation gate additionally exposed cancellation inside the quadrature translation matrix.
Evaluate h_n(x+s)-h_n(x) by its differenced Hermite recurrence before quadrature, not by subtracting two
sampled Hermite arrays. This keeps the polynomial quadrature oracle independent of the Taylor-coefficient
implementation while preserving tiny increments. The first failed repair log is retained.

Let c=<A,dB>+<dA,B>+<dA,dB>, b=<A,B>, and e=sum(beta^2/(4w)). Then

    exp(e)<A(z+mu),B(z+mu)> = b + c + expm1(e)*(b+c).

Sum every b term as ||sum_a Q_a||^2, combining coefficients BEFORE squaring. Sum only the corrections
c+expm1(e)*(b+c) pairwise. Use compensated coefficient summation and scalar summation. This is the same
finite algebraic integral, with no new quadrature nodes, no polynomial truncation and no altered tolerance.

Require the planted cancellation and tiny-translation tests to fail before repair and pass after. Mutants
restoring naive translation subtraction or the old uncentered sum must be caught, in addition to the
previous eight mechanisms. Repeat both moment paths and the old-rule replay on the same four saved controls
after a new source commit. All logs and the earlier run remain accessible.

The cancellation-scale diagnostic is still not a rigorous error bound. Numerical agreement must never
be promoted to an interval certificate. The smooth cutoff and exterior-tail questions remain open.
