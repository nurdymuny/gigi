# H8 Step 3: the projected-Gaussian min–max lemma (proof note)

Revised 28 Nov after the referee pass: target written as 274/(30√3) = 5.273132 (not 5.27317); the finite-difference receipt is cross-referenced to §7, not §8; the "equivalent" remark is no longer presented as correcting the research note, which already used sufficiency language; §7 now says h = N_H/N₁ changes sign (cancellation widens enclosures), recommends certifying at α = 1 everywhere (α = 4 only after its own convergence check), and asks for the certified upper end of μ₂ to be recorded; the explicit moment expansion of E_{N(c,Σ)}V₀ is pointed to the receipt's E2/E3/E4 (still to be written out in the note). No change to the mathematics; no status upgraded.

28 November 2026. **Proof note for one lemma. It changes no status in the register by itself.** Step 3 stays open
until the interval computation in §7 is run. Every number marked *floating* is an ordinary double-precision
estimate, not an enclosure.

The route works. While writing it out I found one point to make explicit, one gap that needed a proof, and one
plan for the computation that has to change:

- **Sufficient, not equivalent.** Min–max gives E_{2,0} ≤ μ₂, so certifying μ₂ < 5.2731 is **sufficient** for
  Step 3. It is not necessary: a bad trial space can fail even though Step 3 is true. (The research note already uses
  sufficiency language; this is a clarification, not a correction of a claim it made.)
- **Gap filled.** "φ₀ and ψ₁ are linearly independent (different centres)" is not a proof, because ψ₁ is an
  average of Gaussians and is not centred anywhere. §5 proves it for every g > 0 and α > 0 by comparing decay
  along the competitor mode.
- **Plan changed.** The crude Gaussian tail bound in SO(3) is effective only at the two smallest couplings. At
  g² ≥ 0.03 the integrand is not small anywhere on SO(3), so the certified integral must cover the whole
  rotation ball. That is still a compact 3-dimensional box cover (§7).

## 1. Setting (unchanged from the H8 file)

Fix t = 3, so u = 1/√3, ω = (√3, u, u) and S = 5/√3. Coordinates are q ∈ ℝ^{3×3} ≅ ℝ⁹ (row k is the Berger frame
direction, column c is colour) with the Frobenius inner product. At s = 1 the sources vanish and

  W(q) = ½ Σ_k ω_k ‖q_k‖² − g det q,  V₀ = ½|∇W|² = ½ Σ_{k,c} (ω_k q_{kc} − g cof(q)_{kc})²,

  T = −½Δ + V₀ on C_c^∞.

The register's realization H_g is the closure from C_c^∞ (H8 file §1) or the Friedrichs extension (line 9). The
argument below works for either, because I only use that H_g is a closed extension of T.

- **Colour.** O ∈ SO(3) acts by q ↦ qOᵀ. The singlet space is 𝓗 = {ψ ∈ L²(ℝ⁹) : ψ(qOᵀ) = ψ(q) ∀O}.
- **Circle.** (U_φψ)(q) = ψ(R_φᵀ q R_φ), with R_φ = diag(1, R₂(φ)) the rotation of rows 2 and 3. 𝓗₀ ⊂ 𝓗 is
  the charge-0 subspace, U_φψ = ψ for all φ.

**Taken from the register.**
- **(T-A)** Line 9/19, Theorem A at s = 1: H_g is self-adjoint and bounded below, with compact resolvent.
- **(T-S)** Line 17 and H8 §1: H_g commutes with colour and with U_φ, and preserves each 𝓗_m.

So H_g|_{𝓗₀} is self-adjoint on 𝓗₀ with compact resolvent: the restriction of a compact operator to a closed
reducing subspace is compact. E_{n,0} is its n-th eigenvalue, counted with multiplicity from n = 1, exactly as in
H8 §1.

Notation. M_O is the 9×9 orthogonal matrix of q ↦ qOᵀ (M_O = 1₃ ⊗ O in row-major vec), and M_Oᵀ is q ↦ qO.
L_φ is the orthogonal map q ↦ R_φ q R_φᵀ, with matrix R_φ ⊗ R_φ. Ω is the 9×9 diagonal matrix carrying ω_k on
row k, so φ₀ below is exp(−½ qᵀΩq). dO is normalised Haar measure, which is bi-invariant and invariant under
conjugation.

## 2. Statement

**Lemma (projected-Gaussian min–max).** Fix t = 3, s = 1, any g > 0 and any α > 0. Let

- a = q_MC = diag(u, 1, 1)/g,
- J = Hess W(a),
- A = |J| + α P_{ker J}, where |J| is the spectral absolute value and P_{ker J} the orthogonal projection onto
  ker J,
- φ₀(q) = exp(−½ Σ_k ω_k ‖q_k‖²),
- φ(q) = exp(−½ (q − a)ᵀ A (q − a)),
- ψ₁(q) = ∫_{SO(3)} φ(qOᵀ) dO, a pointwise Lebesgue integral of a continuous function over a compact group.

Then:

1. ψ₁ ∈ 𝓢(ℝ⁹) ∩ 𝓗₀, and φ₀ ∈ 𝓢(ℝ⁹) ∩ 𝓗₀. Both lie in D(H_g), where H_g acts on them as the differential
   expression T.
2. Put N₁(O) = ⟨φ, φ(·Oᵀ)⟩ and N_H(O) = ⟨φ, H_g φ(·Oᵀ)⟩. These are continuous, explicit elementary functions on
   SO(3), given in closed form in §6. The Gram and energy matrices of (φ₀, ψ₁) are
   - 𝐌 = [[⟨φ₀,φ₀⟩, ⟨φ₀,φ⟩], [⟨φ₀,φ⟩, ∫N₁ dO]],
   - 𝐊 = [[⟨φ₀,H_gφ₀⟩, ⟨φ₀,H_gφ⟩], [⟨φ₀,H_gφ⟩, ∫N_H dO]].
3. φ₀ and ψ₁ are linearly independent, so 𝐌 is positive definite.
4. E_{2,0}(g) ≤ μ₂, the larger root of det(𝐊 − μ𝐌) = 0.
5. Equivalently, for any real U: if k₁₁ − U m₁₁ < 0 and (k₀₀ − U m₀₀)(k₁₁ − U m₁₁) > (k₀₁ − U m₀₁)², then
   E_{2,0}(g) < U.

**Corollary (what Step 3 needs).** If, at each registered g² ∈ {0.002, 0.004, 0.03, 0.05, 0.07}, an
interval-arithmetic evaluation verifies (5) with U = 5.2731 for some α > 0 (α may depend on g), then Step 3 holds:
E_{2,0} < 5.2731 at those couplings. U = 5.2731 is just below 274/(30√3) = 5.273132….

Form (5) avoids square roots. 𝐊 − U𝐌 is negative definite exactly when its (1,1) entry is negative and its
determinant is positive, and that holds exactly when every Rayleigh quotient on the span is below U.

## 3. What I borrow, with exact hypotheses

**(B1) Min–max principle.** M. Reed and B. Simon, *Methods of Modern Mathematical Physics IV: Analysis of
Operators*, Academic Press (1978), Theorem XIII.1, §XIII.1.

I checked the statement through a verbatim restatement in H. Fan, "Schrödinger equations, deformation theory and
tt*-geometry", arXiv:1107.1290, Thm 2.1, not the book's own pages. The statement as restated there:

> Let H be a self adjoint operator of a Hilbert space that is bounded below, i.e. H ≥ CI for some C. Define
> μ_n(H) = sup_{φ₁,…,φ_{n−1}} U_H(φ₁,…,φ_{n−1}), where U_H(φ₁,…,φ_m) = inf{(ψ,Hψ) : ψ ∈ D(H), ‖ψ‖ = 1,
> ψ ∈ [φ₁,…,φ_m]^⊥}. Then for each n, one of the following must hold: (1) there are n eigenvalues (counting
> multiplicity) below the bottom of the essential spectrum and μ_n(H) is the n-th eigenvalue (counting
> multiplicity); (2) μ_n is the bottom of the essential spectrum, … and there are at most n − 1 eigenvalues
> (counting multiplicity) below μ_n.

- Hypothesis: H is self-adjoint and bounded below.
- Use: H = H_g|_{𝓗₀} on the Hilbert space 𝓗₀, which meets the hypothesis by (T-A) and (T-S).
- The same source restates Reed–Simon IV, Theorem XIII.64: a self-adjoint H bounded below has compact resolvent if
  and only if μ_n(H) → ∞, if and only if there is an orthonormal eigenbasis with eigenvalues increasing to ∞. With
  compact resolvent the essential spectrum is empty, so alternative (1) holds for every n.

**(B2) Group-averaging projection, idea only.** R. E. Peierls and J. Yoccoz, "The Collective Model of Nuclear
Motion", *Proc. Phys. Soc. A* **70** (1957) 381–387, DOI 10.1088/0370-1298/70/5/309. I verified the Crossref
record (authors, title, volume, issue 5, pages, May 1957), not the full text.

I borrow the idea that a symmetry-breaking "intrinsic" trial state averaged over the group gives a trial state
with good quantum numbers. No theorem of theirs is used. Every property of ψ₁ is proved in §4.

**(B3) Isserlis' theorem.** L. Isserlis, "On a formula for the product-moment coefficient of any order of a normal
frequency distribution in any number of variables", *Biometrika* **12** (1918) 134–139, DOI
10.1093/biomet/12.1-2.134. I verified the Crossref record and the Zenodo scan's stated aim.

- Statement: for a centred Gaussian vector z with covariance Σ, E[z_i z_j z_k z_l] = Σ_ijΣ_kl + Σ_ikΣ_jl +
  Σ_ilΣ_jk, and odd moments vanish.
- Hypotheses: Gaussian with finite covariance. These hold here because every product of two of my Gaussians is a
  nondegenerate Gaussian (§6).

The only other ingredients are standard and need no citation: dominated convergence (differentiation under the
integral sign), Fubini–Tonelli, Green's formula for Schwartz functions, and Haar measure on a compact group.

## 4. Proof of (1): regularity, domain, symmetry

**4.1 The Hessian, exactly.** ∇W(a) = 0: Ωa = diag(1, u, u)/g = g cof(a). Since det is cubic, J = Ω − g D²det(a)
does not depend on g. In row-major coordinates:

- J_{(kc),(kc)} = ω_k;
- for k ≠ l with m the third index, J_{(kk),(ll)} = −a'_m and J_{(kl),(lk)} = +a'_m, where a' = (u, 1, 1);
- all other entries are 0.

Its spectrum:

| Block | Eigenvalues |
|---|---|
| (q₁₁, q₂₂, q₃₃) | 2u on (0,1,−1); μ₊ = (√3+√11)/2 and λ₋ = −ω_C on span{(1,0,0), (0,1,1)}, with ω_C = (√11−√3)/2 = 0.79229 |
| (q₁₂, q₂₁) and (q₁₃, q₃₁) | 4/√3 and 0 |
| (q₂₃, q₃₂) | 2u and 0 |

The kernel is exactly the colour tangent space T_a = {a'X/g : X antisymmetric}. The competitor eigenvector is

  v_C ∝ (1, r, r) in (q₁₁, q₂₂, q₃₃), r = (√3+√11)/4.

Check: row 1 gives √3 − 2r = λ₋; row 2 gives −1 = λ₋ r, since (3 − 11)/8 = −1.

So A has eigenvalues 4/√3 (×2), 2/√3 (×2), μ₊, ω_C and α (×3). A > 0 with λ_min(A) = min(α, ω_C). Also
½ tr|J| = 3S/2 + ω_C = 5.1224, the harmonic value. All of this was checked against finite differences of ∇W
(error 3×10⁻¹⁰) in the receipt script (described in §7).

**4.2 Schwartz class.** Write φ_O(q) := φ(qOᵀ). Since qOᵀ − a = (q − aO)Oᵀ,

  φ_O(q) = exp(−½ (q − aO)ᵀ M_OᵀAM_O (q − aO)) ≤ exp(−½ λ_min(A) (|q| − |a|)₊²),

because |aO| = |a|. For every multi-index β, ∂^β φ_O = P_{β,O}·φ_O, with P_{β,O} a polynomial of degree |β| in q
whose coefficients are polynomials in the entries of O, A and a, hence bounded on the compact SO(3). The map
(q, O) ↦ ∂^β φ_O(q) is continuous. Dominated convergence therefore lets me differentiate under the integral sign
to every order, with

  |q^γ ∂^β ψ₁(q)| ≤ sup_O |q^γ P_{β,O}(q)| · exp(−½ λ_min (|q|−|a|)₊²) ≤ C_{γβ}.

So ψ₁ ∈ 𝓢(ℝ⁹). φ₀ is a centred Gaussian with Ω > 0, so it is Schwartz too.

**4.3 Domain.** Let f ∈ 𝓢(ℝ⁹) ∩ 𝓗₀. Take χ ∈ C_c^∞ radial with χ = 1 near 0, and χ_n(q) = χ(|q|/n). Colour and
circle act orthogonally on ℝ⁹, so χ_n f ∈ C_c^∞ is still a charge-0 singlet. Now

  T(χ_n f) = χ_n Tf − ∇χ_n·∇f − ½(Δχ_n) f.

Tf ∈ L² because V₀ is a polynomial and f is Schwartz, and ‖∇χ_n‖_∞ = O(1/n), ‖Δχ_n‖_∞ = O(1/n²). So
χ_n f → f and T(χ_n f) → Tf in L². Hence f ∈ D(T̄), and T̄ f = Tf.

Both realizations in the register are closed extensions of T: the closure equals T̄, and the Friedrichs extension
contains T̄. So f ∈ D(H_g) with H_g f = Tf. The same holds whether the core is all of C_c^∞ or its singlet
charge-0 part, since χ_n f belongs to both.

**4.4 Singlet.** ψ₁(qPᵀ) = ∫ φ(q(OP)ᵀ) dO = ψ₁(q), by right invariance under O ↦ OP. φ₀(qPᵀ) = φ₀(q), because
colour does not change row norms.

**4.5 Charge 0.**
- W∘L_φ = W. Indeed L_φ(q) = R(qRᵀ): the colour rotation qRᵀ keeps row norms and det, and the row rotation R mixes
  rows 2 and 3, whose weights are equal (ω₂ = ω₃). It also keeps det, since det R = 1.
- L_φ a = a, since R diag(u,1,1) Rᵀ = diag(u, R₂R₂ᵀ).
- Differentiating W∘L_φ = W twice at the fixed point a gives L_φᵀ J L_φ = J. So the orthogonal L_φ commutes with
  J, hence with |J| and with P_{ker J}, hence with A.
- Therefore φ(L_φ q) = exp(−½ (L_φ(q − a))ᵀ A L_φ(q − a)) = φ(q), and φ(Rᵀ y R) = φ(y) for every y.

Fix q and put y = q R Oᵀ Rᵀ = q(ROR^T)ᵀ. Then Rᵀ y R = Rᵀ q R Oᵀ, so

  (U_φψ₁)(q) = ∫ φ(Rᵀ q R Oᵀ) dO = ∫ φ(q(RORᵀ)ᵀ) dO = ψ₁(q),

by conjugation invariance of Haar measure. φ₀ is U_φ-invariant by the same row-weight argument. So
φ₀, ψ₁ ∈ 𝓗₀. ∎(1)

## 5. Proof of (3): linear independence

ψ₁ > 0 everywhere, so ψ₁ ≠ 0. I show ψ₁/φ₀ is unbounded, so ψ₁ is not a multiple of φ₀.

Take e = v_C/|v_C| (§4.1). Since e ⊥ ker J, eᵀAe = ω_C. Exactly,

  eᵀΩe = (√3 + 2u r²)/(1 + 2r²) = (19 + √33)/(√3(11 + √33)) = 0.85319.

The strict inequality eᵀΩe > ω_C has a rational proof:
- ω_C < 0.8, because √11 < 1.6 + √3 ⟺ 5.44 < 3.2√3 ⟺ √3 > 1.7.
- eᵀΩe > 0.85, because 19 + √33 > 24.7445 (5.7445² = 32.99928), while 0.85(11√3 + 3√11) < 0.85(19.0531 +
  9.9501) = 24.6527 (using 1.7321² > 3 and 3.3167² > 11).

Choose δ > 0 with (√ω_C + δ)² < eᵀΩe. Let N_δ = {O : |M_O e − e|_A < δ}, where |x|_A = (xᵀAx)^{1/2}. It is an open
neighbourhood of 1, so its Haar measure h_δ is positive. For O ∈ N_δ and s > 0,

  |s M_O e − a|_A ≤ s(√ω_C + δ) + |a|_A,

so

  ψ₁(se)/φ₀(se) ≥ h_δ exp(½ s² eᵀΩe − ½ (s(√ω_C + δ) + |a|_A)²) → ∞ as s → ∞.

Hence φ₀ and ψ₁ are linearly independent for every g > 0 and α > 0, and 𝐌 > 0 (strict Cauchy–Schwarz). ∎(3)

(Geometrically: along the soft competitor direction, the orbit-averaged Gaussian is wider than the origin's
Gaussian.)

## 6. Proof of (2): the matrix entries and closed forms

**6.1 Collapsing the averages.** By 4.2, Hψ₁ = ∫ (H_gφ_O) dO pointwise (differentiation under the integral
sign). Also H_gφ_O = (H_gφ)_O, because Δ commutes with the orthogonal change of variables M_O, and V₀(qOᵀ) = V₀(q):
W is colour invariant, so ∇W(qOᵀ) = ∇W(q)Oᵀ. All integrands below are bounded by a polynomial times
exp(−c(|q| − |a|)₊²), uniformly in O, so Fubini applies.

- ⟨φ₀, ψ₁⟩ = ∫dO ∫ φ₀(q) φ(qOᵀ) dq = ∫dO ∫ φ₀(yO) φ(y) dy = ⟨φ₀, φ⟩, by the substitution y = qOᵀ (Jacobian 1)
  and colour invariance of φ₀. Likewise ⟨φ₀, Hψ₁⟩ = ⟨φ₀, Hφ⟩.
- ⟨φ_{O₁}, (Hφ)_{O₂}⟩ = ∫ φ(y) (Hφ)(y O₁O₂ᵀ) dy = N_H(O₂O₁ᵀ), by the substitution y = qO₁ᵀ. Then

    ⟨ψ₁, Hψ₁⟩ = ∫∫ N_H(O₂O₁ᵀ) dO₁dO₂ = ∫ N_H dO,

  by right invariance in O₂ for fixed O₁. The same computation with H removed gives ⟨ψ₁, ψ₁⟩ = ∫ N₁ dO.

**6.2 The Gaussian pair formula.** For G_i = exp(−½ (q − a_i)ᵀA_i(q − a_i)) with A_i > 0, put

  C = A₁ + A₂,  Σ = C⁻¹,  c = Σ(A₁a₁ + A₂a₂),
  κ = a₁ᵀA₁a₁ + a₂ᵀA₂a₂ − cᵀCc = (a₁ − a₂)ᵀ(A₁⁻¹ + A₂⁻¹)⁻¹(a₁ − a₂) ≥ 0.

Then

  ⟨G₁, G₂⟩ = (2π)^{9/2} (det C)^{−1/2} e^{−κ/2},
  ⟨G₁, H_g G₂⟩ = ⟨G₁, G₂⟩ · [ ½ tr(A₁A₂Σ) + ½ (c − a₁)ᵀA₁A₂(c − a₂) + E_{N(c,Σ)} V₀ ].

The kinetic term comes from Green's formula, ⟨G₁, −½ΔG₂⟩ = ½⟨∇G₁, ∇G₂⟩, with ∇G_i = −A_i(q − a_i)G_i. I then
write q − a_i = (c − a_i) + z with z ~ N(0, Σ). For the potential term, V₀ is a polynomial of degree 4. Substitute
q = c + z and apply (B3) to its moments of order ≤ 4. Each of the 9 summands (ω_k q_kc − g cof_kc)² expands into
2nd-, 3rd- and 4th-order moments of affine functions of z. *(For an auditable interval code this explicit
non-central expansion should be written out here; until it is, the receipt's `E2`/`E3`/`E4` functions in
`h8_step3_pencil_floating_purepy.py` are the formulas, and the referee confirmed they are the right ones.)*

**6.3 The four entries.**

| Entry | A₁, a₁ | A₂, a₂ |
|---|---|---|
| m₀₀, k₀₀ | Ω, 0 | Ω, 0 |
| m₀₁, k₀₁ | Ω, 0 | A, a |
| N₁(O), N_H(O) | A, a | B_O = M_OᵀAM_O, b_O = aO |

The first row is closed in elementary terms:

  m₀₀ = π^{9/2} 3^{3/4},  k₀₀/m₀₀ = 5√3/2 + 15g²/4 = 3S/2 + 15g²/4.

The cross term E[q·cof] is odd and vanishes. Each E[cof_kc²] = 2 var_{k₁} var_{k₂}, with var_k = 1/(2ω_k). These
sum to 7.5.

In the third row, B_O and b_O are polynomial in the entries of O, and det C(O) ≥ (2λ_min)⁹ > 0. So N₁ and N_H are
products of (det C)^{−1/2}, the exponential of a rational function, and a rational function of the entries of O.
They are real-analytic on SO(3). This proves (2).

**6.4 Bound.** Every nonzero vector of V = span{φ₀, ψ₁} has Rayleigh quotient at most μ₂ := max_{x≠0} xᵀ𝐊x/xᵀ𝐌x,
the larger generalised eigenvalue. Let φ₁ ∈ 𝓗₀ be arbitrary. Since dim V = 2, there is a unit ψ ∈ V ∩ {φ₁}^⊥.
By 4.3 it lies in D(H_g|_{𝓗₀}), so U_H(φ₁) ≤ ⟨ψ, Hψ⟩ ≤ μ₂. Taking the supremum over φ₁, μ_2(H_g|_{𝓗₀}) ≤ μ₂.

By (T-A) the essential spectrum of H_g|_{𝓗₀} is empty, so (B1) alternative (1) gives μ_2(H_g|_{𝓗₀}) = E_{2,0}.
Hence E_{2,0} ≤ μ₂. Item (5) is the statement "𝐊 − U𝐌 is negative definite", which is equivalent to μ₂ < U. ∎

## 7. What closing Step 3 now needs (not done here)

An interval or ball-arithmetic evaluation at each of the five couplings:

1. **The cross entries.** m₀₀ and k₀₀ are closed form. m₀₁ and k₀₁ are one 9×9 Gaussian evaluation each, with no
   integral.
2. **The SO(3) integrals.** ∫N₁ and ∫(h − U)N₁, where h = N_H/N₁ is the bracket in 6.2, in rotation-vector
   coordinates θ on the closed ball |θ| ≤ π. Note that h is **not** positive: it changes sign (the referee's sampling
   found h ∈ [−0.52, 5.87] at g² = 0.002, α = 1, and [−10.3, 8.1] at g² = 0.07, α = 4). So ∫(h − U)N₁ involves
   cancellation, which widens interval enclosures, worst at large α. There dO = (2 − 2cos|θ|)/(8π²|θ|²) d³θ, the Weyl density
   (2/π)sin²(θ/2) on the angle with a uniform axis. It integrates to 1, which I checked. On each box, enclose the
   integrand with interval or Taylor bounds; it is analytic.
3. **Tail bound, small couplings only.** From 6.2, κ(O) ≥ (λ_min/2)|a − aO|². Also |a'(1 − O)|²_F ≥ u²·8 sin²(θ/2)
   and det C ≥ (2λ_min)⁹, so

     N₁(O) ≤ (π/λ_min)^{9/2} exp(−(2/3) λ_min sin²(θ/2)/g²),

   and |N_H| ≤ sup|h| · N₁. This makes the region θ ≥ 1 negligible at g² = 0.002 and 0.004. At g² ≥ 0.03 it does
   not: at g² = 0.07 the factor at θ = π is only e^{−7.5} against a prefactor of about 500. **There I cover the
   whole ball**, which is still compact and 3-dimensional, instead of using the tail bound. This corrects the
   research note's plan of "box cover near O = 1 plus a Gaussian tail" at the larger couplings.
4. **The pencil test.** Check (5) with U = 5.2731 in interval arithmetic. Also record the certified upper end of μ₂
   itself at each coupling (≈ 5.11 to 5.12 plus the enclosure width), not only pass/fail at 5.2731: Step 4 needs
   E_{1,m} > E_{2,0}, and the enclosure leaves it about 0.36 of harmonic room at |m| = 2 instead of about 0.21.

Because α is free (any α > 0 gives a valid upper bound), it may be chosen per coupling to widen the margin.

**Floating evidence.** These are not enclosures. Receipt: `h8_step3_pencil_floating_purepy.py` in this folder. It
is pure Python, independent of the earlier numpy script, with the analytic |J|, Isserlis moments and the full 2×2
pencil, plus trapezoid quadrature on ±7σ in θ. The grid is converged to about 10⁻⁴ (nq = 15, 21, 27 agree at
g² = 0.07).

| g² | α | R(φ₀) | R(ψ₁) | normalised overlap | μ₂ | 5.2731 − μ₂ |
|---|---|---|---|---|---|---|
| 0.002 | 1 | 4.3376 | 5.12217 | 5×10⁻⁹¹ | 5.12217 | 0.1509 |
| 0.004 | 1 | 4.3451 | 5.12191 | 5×10⁻⁴⁵ | 5.12191 | 0.1512 |
| 0.03 | 1 | 4.4426 | 5.11791 | 1.4×10⁻⁵ | 5.11791 | 0.1552 |
| 0.05 | 1 | 4.5176 | 5.11378 | 2.7×10⁻³ | 5.11389 | 0.1592 |
| 0.07 | 1 | 4.5926 | 5.10849 | 2.4×10⁻² | 5.11394 | 0.1592 |
| 0.07 | 0.5 | | 5.2431 | | 5.2454 | 0.028 |
| 0.07 | 4 | | 5.0255 | | 5.0386 | 0.2345 |
| 0.002 | 4 | | 5.12005 | | 5.12005 | 0.1530 |

How these line up with what I already had:
- The R(φ₀) column matches the closed form 3S/2 + 15g²/4.
- R(ψ₁) at α = 1 matches the earlier numpy receipt to about 10⁻³ (5.1222, 5.1181, 5.1094).
- At g² = 0.07 the pencil raises μ₂ above R(ψ₁) by 0.0055; the normalised overlap is 0.024, not 2×10⁻³. Since
  μ₂ ≥ R(ψ₁) always, the decision has to be made on μ₂, as in (5), not on R(ψ₁).
- α matters at the larger couplings. With α = 0.5 the margin collapses to 0.028; with α ≥ 1 it is 0.15 to 0.23.
  A small α means a wide Gaussian in the gauge directions. That Gaussian follows the tangent line rather than the
  curved orbit, and averaging then smears the transverse profile. I would certify with **α = 1 at all five
  couplings**, where the floating margin is already 0.151 to 0.159. The α = 4 rows have no convergence check (the
  nq = 15, 21, 27 check is for g² = 0.07, α = 1 only), and an independent referee estimate at g² = 0.07, α = 4 is lower
  by about 0.12, so α = 4 is not supported until it has its own convergence check.

## 8. What this does not give

- **No certified number.** Step 3 is still open until §7 runs in interval arithmetic. The table is floating. No
  charge is removed by the charge tail until a certified U exists.
- **Not an equivalence.** μ₂ < 5.2731 is sufficient for E_{2,0} < 5.2731, not necessary.
- **Only an upper bound.** The lemma says nothing about which state sits at E_{2,0}, about E_{1,0}, or about
  whether the second charge-0 level is the q_MC well's ground state. It gives no lower bound on anything.
- **Not Step 4.** The lower bounds E_{1,m} > U for 1 ≤ |m| ≤ 6, with |m| = 2 the tight case, are untouched. H8_reg
  needs Step 3 and Step 4 together.
- **Not H1.** S_C at the sourced controls (s = 1.15) is a counting lower bound (R2, Lehmann). An upper bound of
  this kind can only feed Lemma R's upper-bound side.
- **Reduced model only**, at t = 3 and s = 1. Nothing about the physical H8 (H2b, dressing by the fast modes), the
  continuum, or other groups. The lemma holds for every g > 0, but certification is per coupling and gives
  nothing uniform in g.
- **Borrowing.** Peierls–Yoccoz supplies only the idea; every property of ψ₁ is proved here. The min–max
  statement was checked through a verbatim secondary restatement (Fan, arXiv:1107.1290), not Reed–Simon's own
  pages. The two inputs I take from the register, (T-A) compact resolvent at s = 1 and (T-S) the symmetry
  reduction, are cited from lines 9/19 and 17 and the H8 file, not re-proved here.

## Status line

Proved, conditional only on the register's Theorem A (compact resolvent) and the symmetry reduction: for t = 3,
s = 1, every g > 0 and every α > 0, E_{2,0}(g) ≤ μ₂(𝐊, 𝐌), with every entry in closed form except two analytic
integrals over SO(3). Step 3 is reduced to a finite interval computation; it is not yet closed.
