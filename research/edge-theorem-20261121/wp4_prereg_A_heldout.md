# Pre-registration A: the edge at held-out family points (WP4, 21 November 2026)

Written before any edge, spectrum or polynomial value has been computed at the points below. The points were drawn by
`scripts/edge_wp4_points.py` and written to `wp4_prereg_A_points.json`. That script evaluates nothing; it only checks
that no point lies within 10⁻³ of a (t, y) pair used before (50 pairs, from `exact_edge.json` and `twist_anatomy.json`,
plus the stretches t ∈ {1, 2, 2.5, 3, 4, 5, 6, 8, 10, 12}).

The design checks behind §2 are in `scripts/edge_wp4_design_checks.py`, with the receipt `wp4_design_checks.json`
(check D2). They are structural: they compute no floor and no edge.

**Labels.**
- **PROVED:** exact symbolic, or rigorous interval.
- **COMPUTED:** double precision.
- **CONJECTURE:** stated, not shown.

## 1. What is under test

H1 of `docs/research/what-she-is-a-law-of-2026-11-21.md`, in its "edge" clause:

> For y ≳ 1 the twist is the floor of every sector exactly when x < x\*(t, y), where x\* is the largest real root in x
> of P₂(x, y, √t).

The test also checks three things around that clause:
- the class of the competitor;
- sectors above 12;
- the claim that her region is a half-line.

## 2. Already decided before the test: her region is bounded below (PROVED, structural)

H1 and the sentence in `she-is-the-law-2026-11-21.md` ("…turns against her charge **by more than** the shape of space
allows") both say her region is a half-line in x. **That is false for every (y, t).** The argument has four steps.

1. **Her value grows like x².** Her eigenvalue in sector 0 is λ₀(x) = (x + 2/√t)² + 2√t·x − y². This is exact (the
   closed form).
2. **The rows are at most linear in x.** In the closed-form rows of `scripts/ricci_su2_exact_edge.py`, check D2 found:
   - w_a = −c_l·x + f_a(t) for every charge state;
   - every entry of C has degree ≤ 1 in x;
   - the ladder couplings Ω± do not involve x.
3. **A competing state that grows only linearly.** Take the state v = θ⁰ ⊗ τ₀ in sector j = 0 (m = 0, c_f = c_l = 0).
   - Its row has w = f(t), so its Rayleigh quotient ⟨v, (H + GG\*)v⟩ = f(t)² + C_vv(x, y, t) is **affine in x**.
   - G\*v = −[m₀, τ₀] = 0 for every (x, y, t) (D2, symbolic). So v also lies in the gauge slice, and ⟨v, Hv⟩ is the
     same affine function.
4. **Conclusion.** λ₀(x) − ⟨v, Hv⟩ → +∞ as x → ±∞. So for each (y, t) there is X(y, t) such that she is not the floor
   of sector 0 when |x| > X. This holds for both definitions: H + GG\* and the gauge-orthogonal spectrum.

**Consequences, fixed now:**
- H1's "half-line" must become "interval [x_lo, x\*]". Her region was observed as a half-line only because every scan so
  far was cut off at x = −3.
- The sentence's "by more than" clause needs an upper limit. This is a correction, not a test outcome.
- **Risk to the running certificate.** I have not computed where x_lo is. If x_lo > −3 anywhere in y ∈ [0.5, 3],
  t ∈ [3, 10], then D is not inside her region, and the small-j certificate on D must fail in sector 0. The large-j
  half (j ≥ 80) is unaffected, because the argument uses sector 0. Arm A4 measures x_lo, so this risk is settled by the
  test and not here.

## 3. Points (fixed; `wp4_prereg_A_points.json`, seed 20261121)

| class | how drawn | count |
| --- | --- | --- |
| **P2 class** | t ~ U[2.2, 11.5], y ~ U[1.1, 3.2] | 30 |
| **small-y class** | t ~ U[2.2, 11.5], y ~ U[0.3, 0.7] | 10 |
| **fixed grid** | t ∈ {2.2, 3.5, 7.0, 11.0} × y ∈ {1.2, 2.5}, treated as P2 class | 8 |

None of these has been computed at.

## 4. Predictions (from the exact edge equation, made before measurement)

- **Pred-A1 (P2 class and grid).**
  - Predicted edge: x\*_pred(t, y) = the largest real root in x ∈ [−3, 3] of P₂(x, y, √t).
  - P₂ is recovered as the factor of degree (7, 8, 16), with 113 terms, of the polynomial in
    `research/su2-floor-family-20261119/edge_polynomial.srepr`, by `sp.factor_list`, as in
    `scripts/ricci_su2_edge_polynomial.py`.
  - Roots come from `sp.Poly(P₂(x, y₀, s₀), x).nroots(n=30)` at the rational-rounded (y, s) to 30 digits.
  - The competitor block just above the edge is (j, q) = (4, 0).
- **Pred-A2 (small-y class).** The competitor block just above the H + GG\* edge has j ∈ {2, 3}. This is read from the
  pattern at y = 0.5 in `exact_edge.json`, where the competitors are (3, ½), (2, ±1). No closed form is predicted for
  this class.
- **Pred-A3 (all sectors).** At x = x\*_pred − 0.02 (P2) or x = x\*_meas − 0.02 (small-y), no sector j ∈ [13, 60] has a
  non-twist block eigenvalue below λ_j.
- **Pred-A4 (interval, no holes).** On a scan x ∈ [−40, x\* + 0.5] at step 0.01, the set where she is the floor of
  sectors j ≤ 12 is a single interval [x_lo, x\*] with x_lo > −40. Its existence is PROVED (§2). What can fail is "no
  holes", and x_lo inside the scan.

## 5. Measurement (an independent code path)

- **Upper edge.** x\*_meas is a root of φ_full(x) = min over j ≤ 12 of [the lowest eigenvalue of the full sector
  matrix H + GG\* from `T2.hessian_and_gauge`, with the twist value λ_j removed once, − λ_j]. It is bracketed by the
  last sign change from ≥ 0 to < 0 on a 0.01 grid in [−3, 3], then refined by brentq to xtol 10⁻¹³.
  - The twist value is removed by taking out one eigenvalue within 10⁻¹⁰ of λ_j. If none is found, the run stops,
    because the closed form would be wrong.
  - The competitor (j, q) at x\*_meas + 10⁻⁴ is read from `Blocks.phi_A` and reported. It is not used to measure.
- **Gauge-orthogonal edge.** x\*_S is found the same way with `phi_S`. It is reported with no gate, since P₂ is the
  H + GG\* polynomial.
- **Lower edge.** x_lo is the smallest x of the floor interval on the A4 grid, refined by brentq.

## 6. Gates

| gate | what must hold | if it fails |
| --- | --- | --- |
| **A-code-1** | at every point and j ≤ 12 at x = x\*_pred, `Blocks.sector_spectrum` equals the full eigvalsh to ≤ 10⁻¹² relative | stop: code |
| **A-code-2** | the recovered P₂ has degrees (7, 8, 16) and 113 terms. At the 18 rows of `exact_edge.json` with height 1.5 or √t, the largest real root of P₂ in [−3, 3] equals `edge_A` to ≤ 10⁻⁹ (known-answer control) | stop: P₂ not recovered |
| **A-code-3** | the Berger path: P₂((2 − s²)/s, s, s) vanishes at s = √t_A, t_A = 2.7311222782381136 (`ricci_su2_exact_edge.py`), to ≤ 10⁻⁸ relative | stop |
| **A-ctrl-neg** | discrimination control: the largest real root of the **other** factor P₁ (degree (5, 6, 12)) misses x\*_meas by more than 10⁻⁶ at ≥ 80% of the P2-class points | if it does not, A1 cannot tell P₂ from P₁ there, and A1 is reported as **inconclusive** |
| **A-bound** | x_lo ≥ r(y, t), the lower root of λ₀(x) = ⟨v, Hv⟩(x) (§2; v = θ⁰ ⊗ τ₀; computed in closed form from the rows at run time) | stop: the scan or the argument is wrong |
| **A1** | at every P2-class and grid point, \|x\*_meas − x\*_pred\| ≤ 10⁻⁹, **and** the competitor is (4, 0) | see verdict |
| **A2** | at every small-y point, the competitor has j ∈ {2, 3} | see verdict |
| **A3** | no undercut for j ∈ [13, 60] at any point | see verdict |
| **A4** | a single interval at every point, with x_lo > −40 | see verdict |

## 7. Verdict rule (fixed now)

- **A1 holds at all 38 points:** "the edge is the largest real root of P₂ for y ≳ 1" is supported at held-out points
  (COMPUTED). It becomes a candidate for an interval proof.
- **A1 fails at any point:** counts against that clause. Each failure is reported with its (t, y), the measured
  competitor, and whether x\*_pred is a root of P₂ that is not the edge (another eigenvalue of B₄,₀ meeting λ₄) or the
  competitor is a different block.
- **A2 fails:** the small-y class is not governed by sectors 2–3. That counts against the class description only, not
  H1.
- **A3 fails:** **counts against H1** (a sector above 12 undercuts her inside the claimed region).
- **A4: a hole** counts against H1 even in its corrected interval form. An interval reaching x = −40 is reported as
  "x_lo below the scan". It is not a failure, because §2 proves x_lo is finite.
- **Descriptive, with no gate:**
  - x_lo(t, y) and the block that sets it;
  - x\*_S − x\*_A;
  - the error of the Test 6 model x\* = c₁/t + c₂/t² (coefficients from `edge_test_predictions.json`) at the same
    points, as a baseline.

## 8. Runtime rules

- One process, OMP_NUM_THREADS=1.
- Script: `scripts/edge_wp<N>_heldout.py`.
- Receipt: `research/edge-theorem-20261121/wp<N>_heldout.json`.
- The points file is not edited after this note.
