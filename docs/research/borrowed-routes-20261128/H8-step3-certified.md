# H8 Step 3: the certified pencil

28 November 2026. This note records the ball-arithmetic run that `H8-step3-closure.md` §7 asked for, with the
referee's fixes applied (`H8-step3-referee.md` §4): α = 1 at every coupling, target U = 274/(30√3) = 5.273132…,
the cancellation in ∫(h − U)N₁ enclosed directly, and the whole of SO(3) covered at every coupling, with no tail
bound anywhere.

- Script: `scripts/h8_step3_certify.py` (python-flint 0.9.0, Arb balls at 64 bits).
- Receipt: `research/h8-step3-certified-20261128/receipt.json` (per-coupling balls for M11, M12, M22, K11, K12,
  K22, the pencil test, the certified upper end of μ₂, runtimes and the script's sha256).

**Result.** All five registered couplings certify. At each one M > 0 and K − U M < 0 hold as strict
ball inequalities, so μ₂ < 274/(30√3). The certified upper ends are μ* = 5.12221, 5.12196, 5.11796, 5.11395 and
5.11399 at g² = 0.002, 0.004, 0.03, 0.05 and 0.07. The margin to U is at least 0.1509, at g² = 0.002. The whole
run took 33 minutes, niced, alongside the long job.

## 1. What is computed

The trial space is span{φ₀, ψ₁} at t = 3, s = 1, α = 1, exactly as in the closure note §2:
A = |J| + P_ker J, a = diag(u, 1, 1)/g, φ = exp(−½(q − a)ᵀA(q − a)), ψ₁ = ∫φ(qOᵀ)dO. The matrices are

- M11 = ⟨φ₀,φ₀⟩, M12 = ⟨φ₀,φ⟩, M22 = ∫N₁ dO;
- K11 = ⟨φ₀,Hφ₀⟩, K12 = ⟨φ₀,Hφ⟩, K22 = ∫N_H dO.

I write 1 for φ₀ and 2 for ψ₁ (the closure note uses 0 and 1). The test is the square-root-free form (5) of the
lemma: M11 > 0, det M > 0, K11 − U·M11 < 0, K22 − U·M22 < 0 and det(K − U M) > 0, all certified as strict
inequalities between balls. With M > 0 this is Sylvester's criterion for K − U M < 0, which is μ₂ < U.

For the certified upper end of μ₂ I run the same test at μ = k/10⁵ and bisect on the integer k. The receipt
records the smallest k on the grid that the test passes, together with the balls of K11 − μ*M11,
K22 − μ*M22 and det(K − μ*M) at that k. The lower end I report is R(ψ₁) = K22/M22, since μ₂ is the largest
Rayleigh quotient on the span.

## 2. Soundness, step by step

Every number in the script is an Arb ball (`arb` or `arb_mat`), and every operation is a ball operation. No
float enters except the dyadic box endpoints, which are exact binary numbers.

**2.1 Constants.** π, √3, √11, u = 1/√3, ω = (√3, u, u), ω_C = (√11 − √3)/2, r = (√3 + √11)/4, U = 274/(30√3)
and (2π)^{9/2} are all ball expressions. The coupling enters as `arb("0.002")` and so on, which is the ball around
the decimal (radius about 10⁻²⁰), and g is its ball square root.

**2.2 The Hessian and A.** J = Ω − D cof(diag a′) is built from the exact entries of the closure note §4.1. A is
assembled as J + 2ω_C e eᵀ + α Σ_k k kᵀ/|k|², with e the competitor eigenvector and k the three colour-tangent
vectors. This equals |J| + αP_ker J exactly when e and the k's carry the eigenstructure of J, and the script
checks that in balls. All nine eigenvector residuals Jv − λv contain 0, with radii ≤ 4×10⁻¹⁸, and the nine
vectors are mutually orthogonal. In any case the lemma only needs A > 0 and [A, L_φ] = 0. A > 0 is certified by
Sylvester (all nine leading minors are balls with positive lower ends; the smallest is 2.11). [A, L_φ] = 0 holds
exactly by the closure note §4.5, and the ball check at φ = 0.7 contains 0. As a further check, ½ tr|J| is the
ball 5.12241401031545…, which overlaps 3S/2 + ω_C.

**2.3 The pair formula.** All six entries come from one routine, the closure note §6.2 written in the shifted
variable x = q − a. Since W is cubic and ∇W(a) = 0, the shift gives ∇W(a + x) = Jx − g cof(x) exactly. With
C = A₁ + A₂, Σ = C⁻¹, d = a₁ − a₂, w = c − a₂ = ΣA₁d and m = c − a:

- log⟨G₁,G₂⟩ = log (2π)^{9/2} − ½ log det C − ½κ, with κ = (A₁d)ᵀ(d − w) = dᵀA₁ΣA₂d;
- kinetic = ½[tr A₁ − tr(A₁ΣA₁)] + ½(w − d)ᵀA₁A₂w, which is ½tr(A₁ΣA₂) + ½(c − a₁)ᵀA₁A₂(c − a₂);
- potential = ½[|r|² − 2g r·E cof(z) + tr(GᵀGΣ) + g²E|cof z|²], with r = Jm − g cof(m) and
  G = J − g Dcof(m). Here E cof(z) = COF·vec Σ, and E|cof z|² = vec Σᵀ T vec Σ, where T is the Isserlis
  tensor. The cross term E[(Gz)·cof(z)] is a third moment and vanishes.

This is the referee's fix 6, the explicit non-central expansion. It is the same algebra as `E2/E3/E4` in the
floating receipt, rearranged so that every piece is a matrix product. At points the ball routine agrees with the
pure-Python floating `pair()` to 10⁻¹² in both log N and h, including at a far rotation where log N = −311.9.
M11 overlaps the closed form π^{9/2}3^{3/4}, and K11/M11 overlaps 5√3/2 + 15g²/4, at every coupling (receipt
`closed_form_checks`).

**2.4 Reduction of the SO(3) integrals to two angles (exact).** I use proper Euler angles
O = R₁(φ₁)R₂(β)R₁(φ₂), with R₁ the rotation of coordinates 2 and 3 (the circle of the closure note) and R₂ the
rotation of coordinates 1 and 3. Haar measure is sin β dβ dφ₁ dφ₂/(8π²) on [0,π] × [0,2π)², and this map is a
bijection off a null set. Two exact symmetries of N₁ and N_H do the reduction.

- **Circle conjugation.** N(R₁ O R₁ᵀ) = N(O). The closure note §4.5 gives φ(L_φ q) = φ(q) with L_φ q = R q Rᵀ.
  Substituting y = R x Rᵀ in ⟨φ, φ(·Oᵀ)⟩ gives N(ROR^T) = N(O). The same holds for N_H, because H commutes with
  L_φ. Hence N(R₁(φ₁)R₂(β)R₁(φ₂)) = N(R₁(φ₁+φ₂)R₂(β)), and integrating out φ₁ − φ₂ gives

    ∫N dO = ∫₀^π ∫_{−π}^{π} N(R₁(γ)R₂(β)) sin β/(4π) dγ dβ.

  This covers all of SO(3), so nothing is left as a tail.
- **Reflection γ → −γ.** N(Oᵀ) = N(O): for N₁ this is the symmetry of the real inner product after the
  substitution y = xOᵀ, and for N_H it also uses the symmetry of H on Schwartz functions. Conjugation by
  R₁(π) sends R₂(β) to R₂(−β). Chaining these, N(R₁(γ)R₂(β)) = N(R₁(γ)R₂(−β)) = N(R₂(β)R₁(−γ)) =
  N(R₁(−γ)R₂(β)), the last step by circle conjugation. So I integrate over γ ∈ [0, π] and double.

The script also checks both identities in balls at three random points per coupling (receipt
`conjugation_reduction_checks`: the 3-angle value, the reduced value and the reflected value overlap).

This replaces the closure note's rotation-vector ball, which has the same Haar measure (the Weyl density). The
Euler form is a compact rectangle with a polynomial density, and the circle symmetry removes one dimension
exactly. It covers the whole group at every coupling, which is the referee's requirement at g² ≥ 0.03, and I
use it at 0.002 and 0.004 as well, so no tail bound is used anywhere.

**2.5 Enclosure on a box: Taylor models.** On a box β ∈ πs_m ± ρ_β, γ ∈ πt_m ± ρ_γ, I propagate each
intermediate quantity X as a Taylor model (P, r). P is a polynomial of total degree ≤ 4 in (δβ, δγ) with ball
coefficients (scalar, vector or 9×9) computed at the box centre. r is a bound with ‖X(δ) − P(δ)‖_F ≤ r for every
δ in the box. The operations and their remainders are:

- sum: r₁ + r₂;
- constant × model: ‖K‖_F r;
- product (matrix or scalar): the exact product truncated to degree 4, plus a bound for the dropped
  degree > 4 terms from the coefficient norms times ρ^k, plus X̄ r_Y + r_X Ȳ + r_X r_Y, where X̄ bounds the
  polynomial part over the box. All of this uses submultiplicativity of the Frobenius norm;
- trace: |tr E| ≤ 3‖E‖_F; entry: |E_ij| ≤ ‖E‖_F; transpose and reshape preserve the norm;
- sin and cos of the angles: the exact Taylor polynomial plus the Lagrange remainder ρ⁵/5!;
- inverse and log det of C: X = P₀(I + Y) with Y = P₀⁻¹(X − P₀). The script requires Ȳ < 1 (certified, else the
  box is marked failed). The Neumann series is taken to degree 4 with tail Ȳ⁵/(1 − Ȳ), and
  log det(I + Y) = Σ(−1)^{k+1} tr Y^k/k is taken to degree 4 with tail 3Ȳ⁵/(5(1 − Ȳ)). P₀⁻¹ and det P₀ are Arb's
  rigorous matrix inverse and determinant, and det P₀ > 0 is certified before the log;
- exp: e^{F₀} times the degree-4 Horner series of the non-constant part G, with tail Ḡ⁵ e^{Ḡ}/5!.

The coefficients are balls, so the semantics are: for every δ there is a choice of coefficients inside the balls
whose polynomial is within r of the true value. Every operation above preserves this.

**2.6 Integrating a box.** On the symmetric box, the mid-polynomial monomials with an odd exponent integrate to
exactly 0. Their coefficient radii are added back as ±rad(c)·ρ^{p}ρ^{q} per unit area. The even monomials give
c·ρ_β^pρ_γ^q/((p+1)(q+1)). The remainder contributes ±r. Multiplying by the box area 4ρ_βρ_γ gives a ball that
contains the exact integral over the box. The integrands are F₁ = N₁ sin β/(4π), F_G = (h − U)F₁ and F_H = hF₁.
F_G is enclosed directly as one Taylor model, so the cancellation in K22 − U·M22 that the referee flagged
(h changes sign) is handled inside the model, not by subtracting two wide balls. The receipt also gives
K22 − U·M22 from the separate balls, and the direct one is the narrower.

**2.7 Cover and refinement.** The starting grid is 8 × 8 boxes on (s, t) ∈ [0,1]², with β = πs and γ = πt. The
box with the largest contribution to the total width is bisected along its longer side. Refinement stops when the
summed width falls below a tolerance relative to M22 (3×10⁻⁴), or at a time cap. The boxes always partition the
full rectangle, and a box that fails (Ȳ ≥ 1 or a non-finite value) keeps the coupling uncertified. No box
failed in the final run. The tolerance and the cap affect only the width of the enclosure, never its validity.

**2.8 The pencil.** detM, K11 − U·M11, K12 − U·M12, K22 − U·M22 and det(K − U M) are ball expressions in the
six enclosures. The verdict is the conjunction of five strict ball comparisons (`arb > 0` is true only if the
whole ball is positive).

## 3. Results

Script sha256 `70d66ef1510e061a958a0319dade6c26efbe6b798398e0e510d4332cbbfe6851`, total runtime 1996 s.
The structure checks all pass: the J eigenvector residuals contain 0, the nine eigenvectors are orthogonal, A > 0
is certified, and [A, L_φ] contains 0.

**The six entries (balls, mid ± radius).**

| g² | M11 | M12 | M22 | K11 | K12 | K22 |
|---|---|---|---|---|---|---|
| 0.002 | 393.564002063437 ± 2.4e−15 | 3.35149608263983e−91 ± 6e−106 | 0.0012265 ± 5.5e−8 | 1707.13384902551 ± 2.2e−14 | −1.86073797652783e−89 ± 3e−104 | 0.0062821 ± 7.1e−8 |
| 0.004 | 393.564002063437 ± 2.4e−15 | 5.95739811561704e−45 ± 9e−60 | 0.0034725 ± 6.4e−8 | 1710.08557904099 ± 4.3e−14 | −1.52145921584926e−43 ± 6e−58 | 0.017786 ± 2.2e−7 |
| 0.03 | 393.564002063437 ± 2.4e−15 | 7.21493036127228e−5 ± 8e−21 | 0.072321 ± 6.8e−7 | 1748.45806924218 ± 9.4e−15 | 3.56390521926637e−5 ± 6e−20 | 0.37013 ± 5.4e−6 |
| 0.05 | 393.564002063437 ± 2.4e−15 | 0.0211294940884492 ± 4e−18 | 0.15739 ± 4.3e−6 | 1777.97536939693 ± 3.2e−14 | 0.0450840267325688 ± 5e−17 | 0.8048 ± 4.9e−5 |
| 0.07 | 393.564002063437 ± 2.4e−15 | 0.241005966136567 ± 2.4e−17 | 0.263875 ± 9.0e−7 | 1807.49266955169 ± 4.4e−14 | 0.689245555186948 ± 4.2e−16 | 1.3480 ± 2.0e−5 |

The SO(3) entries M22 and K22 carry relative widths of 3×10⁻⁶ to 1.2×10⁻⁵. Everything else is at working
precision.

**The pencil test at U = 274/(30√3) = 5.27313245859858… (every entry certified).**

| g² | det M (lower end) | K11 − U·M11 (upper end) | K22 − U·M22, direct | K22 − U·M22, from separate balls | det(K − U M) (lower end) | verdict |
|---|---|---|---|---|---|---|
| 0.002 | 0.48268 | −368.18 | −1.8520e−4 ± 4.9e−8 | ± 2.7e−7 | 0.068152 | negative definite |
| 0.004 | 1.36664 | −365.23 | −5.2524e−4 ± 1.3e−7 | ± 5.0e−7 | 0.19174 | negative definite |
| 0.03 | 28.4627 | −326.86 | −0.011226 ± 3.3e−6 | ± 3.3e−5 | 3.6683 | negative definite |
| 0.05 | 61.9409 | −297.34 | −0.025080 ± 9.3e−6 | ± 3.4e−5 | 7.4502 | negative definite |
| 0.07 | 103.793 | −267.82 | −0.043451 ± 1.7e−5 | ± 7.0e−5 | 11.294 | negative definite |

**μ₂ itself.**

| g² | R(ψ₁) = K22/M22 (ball) | certified μ₂ ∈ | μ* test: det(K − μ*M) lower end | U − μ* | boxes / seconds |
|---|---|---|---|---|---|
| 0.002 | [5.122071, 5.122263] | [5.122071, 5.12221] | 7.8e−7 | 0.15092 | 1063 / 254 |
| 0.004 | [5.121802, 5.122023] | [5.121802, 5.12196] | 9.6e−6 | 0.15117 | 1063 / 340 |
| 0.03 | [5.117808, 5.118001] | [5.117808, 5.11796] | 1.7e−4 | 0.15517 | 1163 / 372 |
| 0.05 | [5.113689, 5.113867] | [5.113689, 5.11395] | 1.8e−4 | 0.15918 | 1513 / 422 |
| 0.07 | [5.108386, 5.108548] | [5.108386, 5.11399] | 5.0e−4 | 0.15914 | 2113 / 608 |

The upper ends μ* are k/10⁵, the smallest grid point at which the square-root-free test passes. At g² = 0.07 the
refinement stopped at the 600 s time cap rather than at the tolerance. That costs width, not validity: the
μ* = 5.11399 certificate is still a strict ball inequality. The lower ends are R(ψ₁). At g² ≥ 0.05 the overlap
pushes μ₂ visibly above R(ψ₁), so the lower ends there are loose by design, and only the upper ends matter for
Step 3 and Step 4.

Consistency with the floating receipt (the closure note §7 table, which is not an enclosure): the ball midpoints give μ₂ = 5.12217, 5.12191,
5.11790, 5.11389 and 5.11392, and R(ψ₁) = 5.12217, 5.12191, 5.11790, 5.11378 and 5.10847, against the floating
5.12217, 5.12191, 5.11791, 5.11389 and 5.11394 (μ₂) and 5.12217, 5.12191, 5.11791, 5.11378 and 5.10849 (R(ψ₁)).
The normalised overlaps are 4.8×10⁻⁹¹, 5.1×10⁻⁴⁵, 1.35×10⁻⁵, 2.68×10⁻³ and 2.36×10⁻². Each certified μ* sits
4×10⁻⁵ to 6×10⁻⁵ above the floating μ₂. The ball code and the floating receipt agree everywhere they overlap, so
the floating table was right.

## 4. What is and is not established

**Established, rigorously, modulo the soundness of Arb and of the script (which is in the repo and hashed in the
receipt):** at t = 3, s = 1, α = 1 and each registered g² ∈ {0.002, 0.004, 0.03, 0.05, 0.07}, the 2×2 pencil
(K, M) on span{φ₀, ψ₁} has M > 0 and K − U M < 0 with U = 274/(30√3), and in fact μ₂ < μ*, with μ* as in the
table. With the closure note's lemma (proved, conditional on the register's Theorem A and the symmetry
reduction), this gives **E_{2,0}(g) < μ* < 274/(30√3) at each registered coupling. That is Step 3 of H8.**

**Not established:**

- Nothing between the registered couplings. The certificate is per coupling and g enters as a decimal ball.
- Nothing about α ≠ 1. The α = 4 row of the closure note stays floating and unchecked, as the referee asked.
- Step 4 (the lower bounds E_{1,m} > E_{2,0} for 1 ≤ |m| ≤ 6) is untouched. The useful output for it is μ*,
  which leaves about 0.15 more room than the bare U-test.
- No lower bound on E_{2,0} and no identification of the state. R(ψ₁) is a lower end for μ₂, not for E_{2,0}.
- The conditional inputs are unchanged: Theorem A at s = 1 (compact resolvent), the symmetry reduction, and
  Reed–Simon XIII.1 through Fan's restatement. The reduced model only, as in the closure note §8.
- Trust base. Arb's ball arithmetic and matrix inverse/determinant enclosures, and the correctness of my
  Taylor-model code. I checked the code against the floating receipt at points and against the closed forms
  for M11 and K11, but it has not been audited by a second person. The Euler-angle reduction (§2.4) is a short
  proof I wrote for this note, beyond what the closure note proves.

## Status line

Step 3 of H8 is certified at all five registered couplings at α = 1 (U = 274/(30√3), margin ≥ 0.1509), with certified μ₂ upper ends
5.12221, 5.12196, 5.11796, 5.11395 and 5.11399. The certificate is conditional on the closure note's lemma (and
through it on Theorem A and the symmetry reduction), on Arb, and on this script. Step 4 is still open, and H8_reg
needs both.

## Audit (28 November, independent)

An independent audit (`H8-step3-certified-audit.md`) found no soundness defect.
- **Re-run:** the first four couplings reproduce bit for bit. At g² = 0.07 refinement stops at the 600 s cap, so the box
  count depends on machine load. The verdict, μ* = 5.11399 and det(K − UM) ≥ 11.294 reproduce, and the auditor's
  independent values lie inside both runs' balls.
- **Independent check:** a separate floating implementation (no shared code) agrees to 10⁻¹². A 40-digit check places
  M12 and K12 inside the receipt balls. A full-rotation-ball integral without the symmetry reduction confirms the
  reduction.
- **Mutation tests:** dropping the Haar factor, or setting U = 5.10, flips the verdict. Other planted errors are caught
  by the independent cross-check. The pass/fail test alone is therefore weak, and the certificate's weight rests on the
  enclosures, which the code reading confirms.
- **Two reporting corrections:**
  - The width comparison should be read on true Arb radii: enclosing K22 − U·M22 directly is 1.3–3.0 times narrower
    than subtracting the separate balls.
  - The script's docstring describes the midpoint polynomial as degree 2. The Taylor models in the code are degree 4.
    The script is left unchanged so that its recorded sha256 stays valid.

**Conclusion:** E_{2,0} < 274/(30√3) at t = 3, s = 1, at all five registered couplings. This is conditional on the
closure lemma (Theorem A and the symmetry reduction), on Arb, and on this script. With the charge barrier it excludes
every |m| ≥ 7. H8 at these couplings still needs Step 4 (certified lower bounds in charges 1–6); physical H8 still
needs H2b.
