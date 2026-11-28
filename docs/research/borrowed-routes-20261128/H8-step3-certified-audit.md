# H8 Step 3 certificate: independent audit

28 November 2026. I audited `scripts/h8_step3_certify.py` (sha256 `70d66ef1…6851`), its receipt
`research/h8-step3-certified-20261128/receipt.json`, the note `H8-step3-certified.md`, the lemma
`H8-step3-closure.md` and its referee report `H8-step3-referee.md`. I set out to break it. My checks were: a line-by-line
read of the code, an independent floating re-implementation, a 40-digit check of the point entries, a full re-run,
eight mutation runs, and a check of the logic from the lemma to the conclusion. I edited nothing except this file.
All my scratch code is in `/tmp/h8audit/` and `/tmp/h8mut/`, outside the repo.

**Verdict: certified at all five couplings** (g² = 0.002, 0.004, 0.03, 0.05, 0.07; α = 1; U = 274/(30√3)). Every
quantity that enters the verdict is a rigorous Arb enclosure. The cover is the whole group. The integrands match
the lemma, and the logic is valid. The certificate inherits the conditions the note already states: the closure
lemma (through it, Theorem A and the symmetry reduction), Arb, and this script, which has now been read by a second
party. I found no soundness defect. I found four small reporting inaccuracies in the note (§6). None of them
changes a verdict.

## 1. Code read, line by line: is every input to the verdict a ball?

**Constants and the coupling.** π, √3, √11, u, ω, ω_C, r, U = 274/(30√3) and (2π)^{9/2} are all `arb` expressions.
g² enters as `arb("0.002")` etc., and g as its ball square root. α = `arb(1)`, exact. Box endpoints are dyadic
floats, so `arb(float)` is exact. Floats appear only in heap scores, the stopping test, the bisection bracket for
μ*, and the reported statistics. None of those enters a verdict: μ* is re-tested in balls at the final k
(`best = negdef_at(mustar)`), and it is reported only if that test passes.

**A.** A is J + 2ω_C e eᵀ + α Σ k kᵀ/|k|², built from exact ball entries. It equals |J| + αP_ker exactly by the
analytic eigenstructure in closure §4.1, which the script confirms: nine residuals contain 0, and A > 0 holds by
nine positive leading minors. My finite-difference |J| + P_ker (numpy eigh) agrees with the script's A to 2×10⁻¹².

**Taylor-model arithmetic.** Semantics: for each point δ of the box, some coefficient choice inside the balls gives
a polynomial within r of the true value (Frobenius norm). I checked every operation against these semantics:
- sum, scaling and constant multiplication (remainder × ‖K‖_F upper bound);
- `tmul`: the truncated product, plus the dropped degree > n terms bounded by Σ_{d1+d2>n} n_X[d1]·n_Y[d2], plus
  X̄r_Y + r_XȲ + r_Xr_Y. Correct by submultiplicativity of the Frobenius norm;
- trace (√dim·r, Cauchy–Schwarz), entry and transpose;
- `tmat3`: √(Σr_ij²);
- `kron_I3`: √3·r, because ‖I₃⊗E‖_F = √3‖E‖_F;
- `ttrig`: exact Taylor coefficients at the ball midpoint, Lagrange remainder ρ^{n+1}/(n+1)!;
- `texp`: e^{F₀}·Horner(Σ G^k/k!), tail Ḡ^{n+1}e^{Ḡ}/(n+1)!;
- `tinv_logdet`:
  - Z₀ = P₀⁻¹ is Arb's enclosure, valid for every matrix in the ball. Y = Z₀(X − P₀), and Ȳ < 1 is required in
    balls, otherwise the box fails.
  - Neumann tail: Ȳ^{n+1}/(1 − Ȳ).
  - log det: det P₀ > 0 is certified first, and det(I + Y) > 0 follows on the connected set ‖Y‖_F < 1. The Horner
    form of Σ(−1)^{k+1}Y^k/k is right (I traced the signs). Tail: Σ_{k>n} 3Ȳ^k/k ≤ 3Ȳ^{n+1}/((n+1)(1 − Ȳ)). Correct.

Every bound is monotone in its nonnegative inputs, and those inputs are taken at their upper ends, so the
remainders are valid.

**Box integration.** The integration domain is the exact box [πs_l, πs_h] × [πt_l, πt_h]. The ball centre
PI·arb(s_m) contains the exact midpoint.
- Even monomials: c·ρ_β^pρ_γ^q/((p+1)(q+1)). This is valid even though the coefficient choice depends on δ,
  because x^py^q ≥ 0 when p and q are both even.
- Odd monomials: the midpoint integrates to 0, and the δ-dependent part is bounded by rad(c)·sup|x^py^q|. This
  is the subtle case, and it is handled correctly.
- Remainder: ±r·vol.

A failed box (singular P₀, Ȳ ≥ 1, non-finite value) is kept in the heap with score ∞. Any failed box at the end
gives NOT CERTIFIED.

**Cover completeness.** The initial grid is 8×8 on (s, t) ∈ [0,1]², and refinement is exact bisection, so the boxes
always partition [0, π]_β × [0, π]_γ. The boundary β ∈ {0, π} is inside closed boxes, and sin β is just a factor,
with no singularity. No tail bound is used. The rotation ball is not used at all, so its boundary does not arise.

**Haar density and normalisation.** F₁ = N₁ sin β/(4π) on [0, π] × [0, π], doubled for γ < 0. Proper Euler angles
of type XYX have Haar density sin β/(8π²). Integrating out φ₁ − φ₂ gives 2π/(8π²) = 1/(4π), and the total measure
is 2·2π/(4π) = 1. Correct.

**Euler reduction (note §2.4, beyond the closure note).** I re-derived both identities:
- N(R₁ORᵀ₁) = N(O), by substituting x = RyRᵀ and using φ∘L_φ = φ;
- N(Oᵀ) = N(O), by substituting x = yOᵀ (for N_H this also uses the symmetry of H on Schwartz functions);
- conjugation by R₁(π) = diag(1, −1, −1) sends R₂(β) to R₂(−β). Chaining these gives γ → −γ.

The script's own spot checks use only small angles (scaled by g). So I checked the identities in floating point at
twelve random large-angle points (β up to 2.5, |γ| up to 4.3, logN as low as −776). All four forms agree to 9
digits. More decisively, my independent integral over the full 3-D rotation-vector ball with the Weyl density uses
no reduction at all, and it lands inside every ball (§3).

**Pencil test.** The verdict is M11 > 0 ∧ det M > 0 ∧ a11 < 0 ∧ a22 < 0 ∧ det(K − UM) > 0, each an Arb strict
comparison. a22 is the direct enclosure of ∫(h − U)F₁, which is algebraically K22 − U·M22. a22 < 0 is redundant
under Sylvester but harmless.

**Minor code remarks (not soundness).**
- The module docstring still says "midpoint Taylor polynomial (degree 2, point balls)". The code runs degree-4
  Taylor models.
- `ttr` defaults to dim = 9 when the constant coefficient is None. That is correct for every call here, but fragile.

## 2. Do the integrands match the lemma?

I wrote `/tmp/h8audit/indep_float.py`, which shares no code with the script:
- J by finite differences of ∇W, then |J| and P_ker by eigh;
- the Gaussian pair from the unshifted definitions (c = Σ(A₁a₁ + A₂a₂), κ = a₁ᵀA₁a₁ + a₂ᵀA₂a₂ − cᵀCc);
- E V₀ by the moment identity for each gradient component, written as a quadratic form α_i + β_i·z + zᵀΓ_iz:
  E f_i² = α_i² + β_iᵀΣβ_i + 2α_i trΓ_iΣ + (trΓ_iΣ)² + 2tr(Γ_iΣ)². This is Isserlis written differently from the
  script's COF/TQ tensors;
- rotations by Rodrigues' formula.

**Point values of log N₁(O) and h(O)** against `Model.NO_point`, at θ = 0, 0.05, 0.6 and 2.9 (random axes) and
g² = 0.002 and 0.07:
- They agree to 10⁻¹² in both log N₁ and h, including log N₁ = −778.03 and h = −342.59 at θ = 2.9, g² = 0.002.
- h changes sign as the referee said (h = −22.2 at θ = 0.6, g² = 0.002), so the direct enclosure of ∫(h − U)N₁
  matters.

**40-digit check of the point entries** (mpmath, |J| from an exact second difference and mp eigendecomposition):
M12 and K12 at g² = 0.002, 0.03 and 0.07 lie inside the receipt balls, with relative differences of 4×10⁻¹⁷ to
8×10⁻¹⁷ against ball radii of about 10⁻¹⁵. M11 and K11 overlap their closed forms in the receipt.

## 3. Independent SO(3) integrals versus the receipt balls

I used the full rotation-vector ball and the Weyl density (1 − cos θ)/(4π²) dθ d²n, with no symmetry reduction:
Gauss–Legendre on 10 θ panels and a product grid on the sphere. The Haar total is 1 to 13 digits. The two grids,
(10, 20, 40) and (16, 32, 64), agree to 11 or more digits at every coupling.

| g² | M22 (mine) | K22 (mine) | inside receipt balls? | position (mine − mid)/rad | μ₂ (mine) | receipt [R(ψ₁), μ*] |
|---|---|---|---|---|---|---|
| 0.002 | 0.00122645757093 | 0.00628212008759 | yes, yes, and K22−UM22 yes | < 0.001 | 5.1221667 | [5.122071, 5.12221] |
| 0.004 | 0.0034725232565 | 0.0177859595934 | yes, yes, yes | < 0.001 | 5.1219123 | [5.121802, 5.12196] |
| 0.03 | 0.0723208988647 | 0.370131462342 | yes, yes, yes | < 0.001 | 5.1179046 | [5.117809, 5.11796] |
| 0.05 | 0.157386498032 | 0.804839596351 | yes, yes, yes | < 0.001 | 5.1138853 | [5.113690, 5.11395] |
| 0.07 | 0.263874977612 | 1.34799651622 | yes, yes, yes | < 0.001 | 5.1139178 | [5.108386, 5.11399] |

So the ball midpoints are right to about 11 digits, and the radii are honest over-estimates. A float Euler-grid
integral at g² = 0.07 gives M22 = 0.26387497761, agreeing with the rotation-ball value to 5×10⁻¹⁰. That is a
second confirmation of the reduction and the density.

## 4. Re-run

I ran `scripts/h8_step3_certify.py /tmp/audit_receipt.json` with the stated PYTHONPATH and interpreter, niced, on a
machine at load about 40. It took 2035 s. **all_certified: true.**
- The script sha and all structure checks are identical.
- At g² = 0.002, 0.004, 0.03 and 0.05 every ball (M11, M12, M22, K11, K12, K22, and the direct K22 − U·M22) is
  **bit-identical** to the receipt, with the same box counts, the same μ* and the same det(K − UM) lower ends.
- At g² = 0.07 refinement stops at the 600 s time cap, so the box count depends on machine load: 2063 here against
  2113 in the receipt. The M22, K22 and K22 − UM22 balls differ slightly (rad M22 9.5×10⁻⁷ against 8.7×10⁻⁷).
  The verdict, μ* = 5.11399 and det(K − UM) ≥ 11.294 reproduce, and the point entries are bit-identical. This
  coupling is reproducible in verdict, **not bit-for-bit**. Raising `--time-cap` or capping by boxes would make it
  deterministic.

## 5. Mutation tests

Each copy in `/tmp/h8mut/` changes one or three lines and was run at g² = 0.07 with `--time-cap 90`. The control is
the unmodified script under the same cap.

| mutation | verdict | enclosure vs truth (§3) | caught by |
|---|---|---|---|
| m0 control (unmodified, 90 s) | CERTIFIED, μ* = 5.17816 (coarse, 263 boxes) | M22, K22 contain truth | (none needed) |
| m1 drop Haar sin β | **NOT CERTIFIED** (det(K−UM) ∈ [−42.0, −21.6]) | M22 ≈ 1.017, wrong | verdict |
| m4 U = 5.10 (below μ₂) | **NOT CERTIFIED** (det straddles 0) | (unchanged) | verdict |
| m3 drop the ×2 for γ < 0 | CERTIFIED, μ* = 5.18323 | M22 ≈ 0.132: **outside** | independent integral only |
| m5 sign of the −2g r·E cof term | CERTIFIED, μ* = 5.20512 | K12 = 0.72417 against true 0.68925: **outside** | independent point check only |
| m6 all remainders set to 0 | CERTIFIED, radii ~10⁻¹⁶ | M22 = 0.26387258 ± 9×10⁻¹⁷ against true 0.26387498: **outside** | independent integral only |
| m7 drop the strip β ∈ [0, π/8] (identity) | CERTIFIED, μ* = 4.74164 | M22 and μ₂ grossly wrong | independent integral only |
| m2 cover only γ ∈ [0, π/2] | CERTIFIED, μ* = 5.13462 | still contains truth | **not caught** |

What this shows:
- **The remainders are load-bearing.** Without them (m6) the "enclosure" is falsely tight and misses the true
  integral by about 2500 of its own radii. With them, the true value sits at the centre of every ball.
- **The verdict at U is a weak discriminator.** Several gross errors (m3, m5, m7) still pass, because the pencil
  test at U has a margin of about 0.15 and is nearly invariant to rescaling M22 and K22 together. Only a missing
  Haar factor or a U below μ₂ flips it. So the evidential weight of "CERTIFIED" rests entirely on the enclosures
  being correct. That is why §1–§3 matter, and they pass.
- **Far-region cover cannot be witnessed numerically.** The half γ ∈ [π/2, π] carries only 3.7×10⁻⁷ of M22 and
  −8.0×10⁻⁷ of K22 at g² = 0.07 (my float integral), which is below even the final enclosure width
  (rad M22 ≈ 8.7×10⁻⁷). Dropping it (m2) is invisible. Completeness of the cover therefore rests on the code read
  (§1: an exact dyadic partition of [0,1]², with failed boxes forcing NOT CERTIFIED), and that holds.

## 6. Inaccuracies in the note (reporting only; no verdict changes)

1. **Radii quoted from printed strings.** The M22/K22 radius columns in note §3 and the "from separate balls"
   column use Arb's printed strings, whose radius absorbs the rounding of the midpoint. Examples: M22 at
   g² = 0.002 is quoted ±5.5×10⁻⁸ against a true rad of 1.29×10⁻⁸, and "separate" at 0.03 is quoted ±3.3×10⁻⁵
   against a true 7.0×10⁻⁶. The "direct" column uses the true rad. So the comparison mixes the two.
2. **"2 to 6 times narrower" should read 1.3 to 3.0 times.** On true radii the direct K22 − U·M22 is narrower by
   2.45, 3.03, 2.15, 1.51 and 1.29 at the five couplings.
3. **The relative radii of M22 and K22 are 3.3×10⁻⁶ to 1.25×10⁻⁵** on true rad. That is consistent with the
   note's "3×10⁻⁶ to 1.2×10⁻⁵" only because the string radii happen to land nearby; the note should quote `rad`.
4. **The script docstring** (degree 2) disagrees with the code (degree 4). See §1.

## 7. Logic: from the pencil to E_{2,0} < U

- **The pencil.** M > 0 is certified: M11 > 0 and det M > 0. Negative definiteness of the 2×2 symmetric K − UM
  follows from (K − UM)₁₁ < 0 and det(K − UM) > 0 (Sylvester applied to U M − K). So xᵀKx < U·xᵀMx for all x ≠ 0,
  which means every Rayleigh quotient on V = span{φ₀, ψ₁} is below U, and μ₂ = max R < U. The same holds with U
  replaced by μ*.
- **The entries are the right ones.**
  - M22 = ⟨ψ₁,ψ₁⟩ = ∫N₁ dO and K22 = ⟨ψ₁,Hψ₁⟩ = ∫N_H dO, by closure §6.1 (refereed).
  - M12 and K12 collapse to ⟨φ₀, φ⟩ and ⟨φ₀, Hφ⟩ by colour invariance of φ₀ and H.
  - The code computes exactly these, with B_O = M_OᵀAM_O and b_O = M_Oᵀa = aO, where M_O = I₃⊗O is the
    matrix of q ↦ qOᵀ. I checked this index by index.
- **Min–max.** By lemma item 4 (closure §6.4):
  - φ₀, ψ₁ ∈ 𝓢 ∩ 𝓗₀ ⊂ D(H_g) (§4.3).
  - dim V = 2 because M > 0, and it is certified here, not only proved in §5.
  - So μ_2(H_g|𝓗₀) ≤ μ₂.
  - Compact resolvent (T-A) puts Reed–Simon XIII.1 in alternative (1), so μ_2 = E_{2,0}.
  - Hence E_{2,0} ≤ μ₂ < μ* < U.

  Valid.
- **The target.** The charge-tail bound gives inf spec on charge m ≥ (225 + m²)/(30√3), and |m| = 7 gives exactly
  274/(30√3). So U = 274/(30√3), strict, is the right threshold for excluding |m| ≥ 7. The certified
  μ* ≤ 5.12221 also meets the register's looser wording "a certified U < 5.2731".
- **Conditions.** These are already stated in the notes, and I did not re-audit them: Theorem A at s = 1 (compact
  resolvent), the symmetry reduction (T-S), Reed–Simon through Fan's restatement, and the soundness of
  Arb/python-flint 0.9.0.

## 8. Per-coupling verdicts

| g² | verdict | reasons |
|---|---|---|
| 0.002 | **certified** | Re-run bit-identical. Independent M22, K22 and point entries inside the balls. det(K − UM) ≥ 0.06815, det M ≥ 0.4827. μ* = 5.12221. |
| 0.004 | **certified** | Re-run bit-identical. Independent values inside the balls. det(K − UM) ≥ 0.1917. μ* = 5.12196. |
| 0.03 | **certified** | Re-run bit-identical. Independent values inside the balls. det(K − UM) ≥ 3.668. μ* = 5.11796. |
| 0.05 | **certified** | Re-run bit-identical. Independent values inside the balls. det(K − UM) ≥ 7.450. μ* = 5.11395. |
| 0.07 | **certified** (the enclosure is not bit-reproducible) | Time-capped. The re-run gives a slightly different but valid enclosure (2063 boxes) with the same verdict, the same μ* = 5.11399 and det(K − UM) ≥ 11.294. Independent values inside both the original and the re-run balls. |

Common to all five: the certificate is conditional on the closure lemma (Theorem A, the symmetry reduction), on
Arb, and on the script. Nothing is established between the couplings, at α ≠ 1, for Step 4, or as a lower bound on
E_{2,0}.
