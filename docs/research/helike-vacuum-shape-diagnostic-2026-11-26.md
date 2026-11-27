# Where the vacuum's shape lives: a diagnostic (26 November; observation only, decides nothing)

**The question.** After §5.10 the competitor's identity is clean in the dressed frame: w̃ = 0.957 at the smoke point,
against 0.914 bare. But its basis convergence improved only by 42%, a step of 0.0053 against the 0.003 rule. The
dressed chart adapts a Gaussian's centre and width. What it cannot adapt is shape. So I measured where the vacuum's
non-Gaussian shape lives.

**The measurement.** The script is `research/dressed-chart-20261126/vacuum-shape/skew_diag.py`. It reads the saved
Mq = 10 exports at s = 1.18, t = 3, g² = 0.03, with no new solve. It takes the vacuum density ψ₀², uses Gauss–Hermite
order 12, and keeps nodes above 10⁻¹⁴ of the total weight. It reports Mardia's multivariate skewness b₁ and excess
kurtosis in four coordinate systems for the same vacuum. Gaussian means b₁ = 0 and excess kurtosis 0.

| coordinates | bare chart: b₁ / excess kurtosis | dressed chart: b₁ / excess kurtosis |
|---|---|---|
| Gram-linear y, where the identity is measured | 0.956 / 1.32 | 0.954 / 1.32 |
| the solver's chart z, log-Cholesky, where the basis lives | 0.581 / 1.81 | 0.595 / 1.91 |
| linear Cholesky factor (r, a, b, c) | 0.161 / 0.43 | 0.164 / 0.47 |
| **√G, the polar part of q = √G·O** | **0.062 / 0.10** | **0.064 / 0.13** |

**Reading.**
- **The skew belongs to the vacuum, not to the frame.** The two columns agree, so dressing cannot remove it. That
  matches the missed convergence prediction.
- **The vacuum is nearly Gaussian in √G.** It is 10× less skewed and about 15× less heavy-tailed there than in the
  chart the basis is built in.
- **Why this is the geometry.** The reduced field space is flat ℝ⁹ (the matrix q) folded by colour O(3). The
  condensate is a ring, a colour orbit of radius about 1/g, and the vacuum is a tube around that ring.
  - √G is the tube's cross-section: the polar decomposition q = √G·O separates the colour orbit (O) from the
    directions across the tube.
  - In those directions the vacuum is a nearly Gaussian blob.
  - The log-Cholesky chart puts an exponential wall on the outer side, and the Gram coordinates square the radius.
    Both bend the blob.
- **How earlier choices fit the same picture.**
  - The original Cartesian q-space basis failed to converge (report page 04), because the vacuum is a ring there, not
    a point.
  - The Gram-linear continuation left the cone (report page 12).
  - The log-Cholesky chart fixed positivity and paid for it in skew.
  - √G is the chart between them: across the tube, not along it.

**What this does not establish.**
- That a basis Gaussian in √G converges faster. That is a prediction, below.
- That its operator can be built with the present exact-moment machinery. It cannot be as it stands: the cometric in
  √G coordinates is rational, not Laurent-exponential.
- Anything about the face det G = 0. That is far from the vacuum at these couplings, but the face must still be
  handled, as the fold of page 12.

## Draft §5.11: the unfolded chart (DRAFT for Bee; not registered)

**Proposal.** Build the trial space in the tube's cross-section coordinates P = √G (six entries of a symmetric
positive matrix), centred and scaled by the dressed construction of §5.10, which carries over unchanged.
- **Basis:** Hermite functions in a linear frame of P.
- **Domain:** each basis function is multiplied by the existing smooth cutoff that vanishes before rank loss, as the
  quasimodes already are (report page 12). So every trial function is in the physical operator's domain.
- **Operator:** −½Δ on colour-invariant functions of q, written in P. Its cometric and measure are rational in the
  eigenvalues λ of P; the measure is ∏(λᵢ+λⱼ) dP, the colour-orbit volume, which the design memo verified in G form.
  The matrix elements are by quadrature, with a registered order seam, not by exact moments.
- **What stays:** the ladder, the convergence tolerance, the identification (dressed w̃, §5.10), the selection fix and
  the switch rule.

**Predictions to register if adopted.** At the smoke point:
- the competitor's step from Mq 8 to 10 is ≤ 0.003;
- the dressed w̃ stays ≥ 0.95;
- the vacuum's Mardia skewness in the trial frame is ≤ 0.1.

**Cost and risk.**
- A new operator path, in Python first and then in Rust, without exact moments.
- Quadrature seams on rational integrands.
- A parity control: at g² = 0.002 and 0.004 the new chart must reproduce the converged log-Cholesky gaps to 10⁻⁴.
- If the √G basis does not beat the log chart at equal dimension, the shape argument is wrong, or incomplete, and
  that is the falsifier.
