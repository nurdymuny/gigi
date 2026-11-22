# Pre-registration B: random circle-symmetric fields, and whether any lie beyond the (x, y) ansatz (WP4, 21 November 2026)

Written before any floor or edge is computed for this test.

The only computations behind it are the design checks D1 in `scripts/edge_wp4_design_checks.py`, with the receipt
`wp4_design_checks.json`. They establish the class of fields. They compute no floor and no edge.

**Revision after review (21 November, before any B-II point is computed).** Changes:
- B-half now scans from the closed-form lower bound and requires the two sign changes of an interval (the lower edge
  is real and proved);
- B-II is described honestly, as a random (ρ, t) scan of the reduced family;
- a minimum number of scored points is set;
- H2's neutrality clause (N) is recorded as **already counted against** by existing data (below), so B-N is
  descriptive only;
- B-S's threshold is labelled post hoc, and the twist slope it uses is specified;
- the fitting procedure is named;
- half-integer weights are covered.

**Labels.**
- **PROVED:** exact symbolic, or rigorous interval.
- **COMPUTED:** double precision.
- **CONJECTURE:** stated, not shown.

## 1. The stated check: is the ansatz general up to gauge?

**The class.**
- Fields: left-invariant SU(2) fields A = Σ m_i θ^i on (t, 1, 1), written as a 3 × 3 matrix M[i, a] with frame index i
  and Lie index a.
- Symmetry: invariant under the right circle about e₀, lifted to the bundle by a constant gauge circle of weight k
  about a Lie axis.
- No generality is lost by fixing that Lie axis to e₀, because a constant gauge rotation conjugates any axis to e₀.
- Infinitesimally the condition is L M − k M L = 0, with L the so(2) generator on indices 1 and 2.

**D1a, the commutant (PROVED, sympy `solve`, receipt key `D1_commutant_by_weight_k`):**

| k | dimension | general invariant M |
| --- | --- | --- |
| ±2 | 1 | m₀ = x e₀, m₁ = m₂ = 0 |
| 0 | 3 | m₀ = any Lie vector, m₁ = m₂ = 0 |
| +1 | 3 | m₀ = x e₀, m₁ = y e₁ − z e₂, m₂ = z e₁ + y e₂ |
| −1 | 3 | m₀ = x e₀, m₁ = −y e₁ + z e₂, m₂ = z e₁ + y e₂ |

For |k| ≥ 3 the weights cannot match, so the same reasoning gives the k = ±2 answer. Half-integer k arises if the
circle is taken in SU(2) with period 4π. It also gives only the weight-0 match, so it reduces to the y = 0 ansatz, and
the conclusion stands.

**D1b, the gauge reductions (PROVED: explicit constant rotations, checked on the fields to ≤ 2.3 × 10⁻¹⁶):**
- **k = +1:** a rotation by α = arg(y + iz) about Lie axis e₀ maps (x, y, z) to (x, √(y² + z²), 0).
- **k = −1:** a rotation by π about Lie axis e₁ maps the (x, y) member to the ansatz at (−x, y).
- **k = 0:** a rotation maps m₀ to |m₀| e₀, which is the ansatz with y = 0.
- **|k| = 2:** already the ansatz with y = 0.

**D1c, the spectral code control (COMPUTED):**
- What was compared: full H + GG\* sector spectra, j ≤ 4, at 6 random (x, y, z, t).
- Maximum relative difference:
  - z removal: 1.95 × 10⁻¹⁵;
  - k = −1 versus x → −x: 0.0;
  - a random constant gauge frame: 2.23 × 10⁻¹⁵.

**Conclusion (PROVED, within the stated class):** every left-invariant, circle-symmetric SU(2) field on (t, 1, 1) is
gauge-equivalent to the ansatz m₀ = x e₀, m₁ = y e₁, m₂ = y e₂ with y ≥ 0. **No such field lies beyond the ansatz.**

**A by-product (PROVED):** at y = 0, (x, 0) and (−x, 0) are gauge-equivalent. So the spectrum is even in x there, and at
y = 0 both the twist and her mirror (the top state of m + c_f − c_l) are exact eigenvectors.

**What the check does not cover**, stated so that nobody reads more into it:
1. **Fields that are not left-invariant but are circle-invariant.** These mix Peter–Weyl sectors. No builder for them
   exists in this repository.
2. **Lifts of the circle by non-constant gauge transformations.**
3. **Circles other than the right circle about e₀.** I take the isometry group of (t, 1, 1), t ≠ 1, to have identity
   component SU(2)_L × U(1)_R. I did not prove that here; it is the standard description of the Berger sphere. Left
   translations already preserve every left-invariant field.

## 2. Consequence for the design

Inside the left-invariant class, "random fields beyond the ansatz" do not exist. So prereg B tests two different
things:
- **B-I:** that the whole class (non-reduced and randomly gauge-framed) behaves as its reduction. This is a code
  control. It cannot fail except through a code error, and it is labelled that way.
- **B-II:** the claims about the whole class that are not automatic. These are H2's kill criteria, at random points
  over a much wider box than any scan so far.

Class 1 above (sector-mixing fields) is the genuine "beyond", and it is left undesigned. As Test 3B in
`skeleton-tests-preregistration-2026-11-20.md` already says, every prediction for it must be one a non-skeleton
operator would fail. I have none yet.

## 3. B-I: the class equals its reduction (code control, not evidence)

**Draws.**
- 200 random members, numpy `default_rng(1122)`.
- k ∈ {+1, −1} with equal probability.
- x ~ U[−10, 10], y ~ U[0, 6], z ~ U[−6, 6], t ~ U[0.5, 15].
- A random constant gauge frame R from QR of a Gaussian matrix, with det R = +1.
- The field R·M is fed to `T2.hessian_and_gauge` in sectors j ≤ 8 **without** reduction.
- The charge operator is **fitted** from the field by `T2.fit_signs` (`scripts/ricci_su3_test2_family.py`): the
  signs of the frame, gauge and V parts are chosen so that it commutes with H and GG\*, with tolerance
  ‖[Q, ·]‖ ≤ 10⁻¹⁰ relative. Exactly one sign choice must pass, or the run stops. It is not assumed.

**Gates (all stop the run on failure):**

| gate | what must hold |
| --- | --- |
| **B-sym** | ‖[Q, H]‖ ≤ 10⁻¹⁰‖H‖, and the top-charge eigenspace has dimension 1 in every sector (2 at y = z = 0, where both circles act) |
| **B-exact** | the top state b is an exact eigenvector (residual ≤ 10⁻⁹‖H‖), and ‖G\*b‖ ≤ 10⁻⁹‖G‖ |
| **B-red** | ⟨b, Hb⟩ equals the closed form λ(x_red, ρ, t, j). Here ρ = √(y² + z²), and x_red = x for k = +1 or −x for k = −1 (D1b). Tolerance 10⁻¹⁰ relative |
| **B-spec** | the full H + GG\* and slice spectra equal those of the reduced field (x_red, ρ, 0) to 10⁻¹² relative |
| **B-even** | at 10 extra draws with y = z = 0, the spectrum at x equals the spectrum at −x to 10⁻¹² |

**Verdict:** all hold → "the ansatz is the whole left-invariant circle-symmetric class" (PROVED in §1) is confirmed by
code. Any failure is a code or convention error and stops the run before B-II is read.

## 4. B-II: a random (ρ, t) scan of the reduced family (the part that can fail)

**What B-II is.** The x and k of each draw are discarded and the edge is located over x for each (ρ, t). So B-II is a
200-point random scan of the reduced (x, y, t) family, over a much wider box than any scan so far. It is not separate
evidence about "general" fields; §1 shows there are none beyond the ansatz in this class.

**Predictions under test** (H2 of `what-she-is-a-law-of-2026-11-21.md`, made precise now):
- **(N) neutrality:** wherever the upper edge is set by a competitor, that competitor carries total charge q = 0.
- **(S) blindness:** its slope in x is small next to hers. The ratio is ρ_s = |∂_x λ_comp| / |∂_x λ_twist| at the edge.
  - The prediction is **ρ_s < 0.5** at every edge.
  - This threshold is fixed now. The three computed ratios in `twist_anatomy.json` (§3 of that note) are 1.56/9.74,
    0.62/9.62 and 1.68/10.18.

**Known before the test, recorded so it is not presented as a discovery.** At y = 0.5, `exact_edge.json` lists the H +
GG\* competitor blocks as (3, ½) at t = 2, (2, ±1) at t ≥ 2.5. Their total charge is not zero. **So (N) as stated in H2
is already false at y = 0.5 (COMPUTED, `exact_edge.json`).**

**It is also already false at y ≥ 1.1 (COMPUTED, `wp1_competitor_map.json`).** At 14 grid points with y ≥ 1.1 and
upper edge x* ≤ 0, the competitor is non-neutral: (1, ±3/2) at y ≥ 2 for t = 2, 3, 4, and (2, ±2) at t = 2, y ≥ 2.75.

- So **H2 (N) is counted against by data in hand.** B-N is kept as a **description**, the fraction of scored points
  with a neutral competitor, with no gate.
- Restricting (N) to a sub-box chosen after seeing these failures would be a forking path, so it is not done.

**Draws (B-II).**
- The 200 B-I members, reduced to (x_red, ρ, t).
- For each (ρ, t), the upper edge x\* is located on x ∈ [−10, 10] with the exact-edge machinery:
  - `Blocks.phi_A`, j ≤ 12;
  - the last sign change from ≥ 0 to < 0 on a 0.01 grid, then brentq to 10⁻¹³.
- Points with no sign change are reported as "no edge in range" and not scored.
- The competitor (j, q) is read at x\* + 10⁻⁴.
- Slopes:
  - ∂_x λ_comp by Hellmann–Feynman on the competitor block (∂B/∂x from the symbolic rows);
  - ∂_x λ_twist = 2(x + (j + 2)/√t) + 2√t, exact. Here j is **the competitor's sector**, and the twist being compared
    is the one in that sector.
- Edges are found in the reduced coordinates. That is legitimate because B-spec has passed.

**Gates and verdict:**

| gate | rule | if it fails |
| --- | --- | --- |
| **B-II-code** | at the 27 rows of `exact_edge.json`, the same routine reproduces `edge_A` to ≤ 10⁻⁹ and the competitor block exactly | stop |
| **B-count** | at least 60 scored points (an upper edge found in range) | below 60: B-S and B-half are reported as **inconclusive** |
| **B-N** | descriptive only (H2 (N) is already counted against; see above) | — |
| **B-S** | every scored point has ρ_s < 0.5 | **counts against H2 (S)**, "nearly blind". The threshold 0.5 was chosen after seeing three ratios (0.16, 0.064, 0.165), so it is a loose, post-hoc bar, and a pass is weak evidence |
| **B-half** | on the 0.01 grid over [r_lo(ρ, t) − 0.5, x\* + 0.5], with r_lo the closed-form lower bound (prereg A §2), φ_A has exactly two sign changes: up at the lower edge, down at the upper edge | counts against H1's no-hole clause (holes narrower than 0.01 cannot be seen) |

- **Supported:** B-S and B-half hold at every scored point, with B-count met. That supports H2's "nearly blind"
  clause (weakly; see above) and H1's interval shape across the scan (COMPUTED). It is still not a proof.
- **Descriptive:**
  - the fraction of draws with an edge in range;
  - the distribution of ρ_s;
  - q_comp on ρ < 1.1;
  - the lower edge, where the scan reaches it.

## 5. Runtime rules

- One process, OMP_NUM_THREADS=1.
- Script: `scripts/edge_wp<N>_general_fields.py`.
- Receipt: `research/edge-theorem-20261121/wp<N>_general_fields.json`.
