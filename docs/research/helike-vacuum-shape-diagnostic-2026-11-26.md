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

## Evidence added the same night (exploratory; decides nothing)

**1. Compactness.** Script: `research/dressed-chart-20261126/vacuum-shape/compactness.py`; output: `compactness_k6.json`.
The converged dressed Mq = 10 states at the smoke point were projected onto a Hermite basis in dressed √G coordinates,
and the weight lying beyond total degree k was measured. The projection used the exact half-density Jacobian, and
Parseval holds: every tail is nonnegative and decreasing.

| state | beyond degree 4: chart → √G | beyond degree 6: chart → √G |
|---|---|---|
| ground | 1.3·10⁻³ → 3.6·10⁻⁵ | 3.4·10⁻⁴ → 1.8·10⁻⁵ |
| Helike | 6.9·10⁻³ → 1.9·10⁻⁴ | 1.4·10⁻³ → 1.0·10⁻⁴ |
| competitor | 2.1·10⁻² → 6.4·10⁻⁴ | 4.6·10⁻³ → 4.2·10⁻⁴ |

One Gaussian in √G carries 99.7% of the vacuum; in the chart it carries 94.4%.

**2. A √G Rayleigh–Ritz prototype.** Script: `sqrtg_ritz.py`.
- **Coordinates.** P is any symmetric matrix, and G = P² ≥ 0 automatically. P with flipped eigenvalue signs is the
  other sheet of the fold, so the basis covers the fold with no wall.
- **Basis and frame.** A Hermite basis in a frame of P, made self-consistent from the prototype's own ground state.
- **Measure.** The orbit volume ∏|λᵢ+λⱼ| enters as a weight, and the problem is solved as a generalized
  eigenproblem.
- **Operator.** The cometric is J A(x) Jᵀ with J = dP/dx, and the potential is the same Laurent V. Quadrature is
  Gauss–Hermite order 10.

| point | basis | size | E₀ | Helike gap | competitor gap |
|---|---|---|---|---|---|
| smoke, s = 1.18, g² = 0.03 | log chart, bare, Mq 10 | 8,008 | −2.794782 | 1.057131 | 1.096694 |
| | log chart, bare, Mq 12 | 18,564 | −2.795147 | 1.056067 | 1.090392 |
| | log chart, Mq 12 + 4 enrichment directions | 18,568 | −2.795330 | 1.055780 | 1.088060 |
| | **√G, Mq 4** | **210** | −2.795301 | 1.057388 | 1.090948 |
| | **√G, Mq 6** | **924** | **−2.7954691** | **1.0556820** | **1.0854191** |
| control, s = 1.15, g² = 0.004 | log chart, converged, Mq 10 | 8,008 | | 1.0879687 | 1.0955590 |
| | **√G, Mq 4** | **210** | −42.121004 | 1.0879994 | 1.0956439 |

**Reading.**
- **Variational order.** Every Ritz energy is an upper bound in its realization. The √G ground energy at 924
  functions is below every log-chart value up to 18,568 functions, and it lies on the log chart's own convergence
  trend. So there is no sign of a realization mismatch: the face is far from the vacuum at these couplings.
- **Parity.** At the g² = 0.004 control, √G at Mq = 4 reproduces the converged log-chart gaps to 3·10⁻⁵ (Helike) and
  8·10⁻⁵ (competitor).
- **Symmetry.** Helike's pair is degenerate to 4·10⁻¹¹ with no symmetrization, because the rotation acts linearly on
  P, P → R P Rᵀ.
- **Not yet shown.** At the smoke point the self-consistent step from Mq 4 to 6 is 0.0014 for Helike and 0.0041 for
  the competitor. Whether the step from Mq 6 to 8 is ≤ 0.003 is still running. So are the order-12 quadrature seam and
  Mq 6 at the control.

**3. Control parity at Mq = 6 and the quadrature seam.**
- **Parity.** At s = 1.15, g² = 0.004, √G at Mq = 6 (924 functions) gives Helike 1.0879712 and competitor 1.0955577.
  The converged log-chart values at Mq = 10 are 1.0879687 and 1.0955590, so the differences are 2.5·10⁻⁶ and
  1.3·10⁻⁶.
- **Seam.** At the smoke point, Mq = 4 at quadrature order 12 agrees with order 10 to 10⁻¹⁵, in E₀ and in every gap.

**4. Why the seam is exact: in the unfolded chart the operator is polynomial.** Along random lines in ζ through the
vacuum, on the sheet where the signs of P's eigenvalues are constant, these are exact polynomials. "Exact" means a
fitted polynomial matches to 10⁻¹¹ relative.

| piece | degree |
|---|---|
| the orbit-volume weight ρ = ∏(λᵢ+λⱼ) | 3 |
| the potential V (√det G = \|det P\|) | 4 |
| the weighted cometric ρ·g | 6–8 |

Every matrix element is a Gaussian times a polynomial. So Gauss–Hermite quadrature of order about Mq + 5 is exact
there, as the log chart's Kronecker moments are exact. The exception is the face, where an eigenvalue sign changes:
the absolute values break polynomiality there, and that region carries negligible weight at these couplings. This is
observed numerically. A symbolic proof of the cometric's polynomial form is still owed.

**What this suggests for a solver.**
- **The construction.** Port the unfolded chart to Rust on the existing slab-streaming machinery (`SlabGrid`):
  forward transform to the nodes, multiply by ρ, V and ρ·g at the exact order, transform back. The operator is applied
  matrix-free, as the bare `ConeOperator::new` quadrature path already does.
- **Cost.** The cost scales like N·q⁶ per application, not N²·q⁶.
- **A cheaper ladder.** The ladder could be Mq ∈ {4, 6, 8}: at the controls, Mq = 6 already matches the log chart's
  converged gaps to 3·10⁻⁶.

**5. Convergence at grid coupling (smoke point, Mq = 8).** The Mq = 8 run (3,003 functions) starts from the converged
Mq = 6 frame; the frame then moved by 3.7·10⁻⁵.

| √G | size | E₀ | Helike gap | competitor gap | step from the previous rung, Helike / competitor |
|---|---|---|---|---|---|
| Mq 4 | 210 | −2.7953006 | 1.0573881 | 1.0909481 | — |
| Mq 6 | 924 | −2.7954691 | 1.0556820 | 1.0854191 | 0.0017 / 0.0055 |
| Mq 8 | 3,003 | −2.7954777 | 1.0554540 | 1.0850236 | **0.00023 / 0.00040** |

- **The step passes, by a wide margin.** The competitor's step from Mq 6 to 8 is 0.0004, against the grid's 0.003
  rule. In the log chart the step was 0.0091 (bare) and 0.0053 (dressed, §5.10) from Mq 8 to 10.
- **The convergence is geometric:** the step falls by about 10× per rung.
- **The log chart overstated the competitor's gap.** At Mq = 10 it was too high by about 0.012 (1.0967 bare, against
  about 1.0850).
- **The sign at s = 1.18.** Converged, the competitor is 0.030 above Helike there. That is consistent with a switch
  below s = 1.18; the prediction is s* = 1.1674 at g² = 0.03. These are observations only, and no switch is located
  from one point.
