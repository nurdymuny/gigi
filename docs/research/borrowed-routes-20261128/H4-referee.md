# Referee pass on `H4-closure.md` (C₀: no intrinsic speed of light, perturbative)

*28 November 2026 (overnight). Skeptical referee pass. I don't edit `H4-closure.md`, `CLOSED-SET.md` or the paper here.
I tried to refute the deduction. I checked every quote against the arXiv v4 full text of Hollands (downloaded and
grepped), and checked the metadata of every citation on arXiv.*

**Verdict: holds with fixes.** Clauses (a)–(c) survive within their stated scope: Lorentzian ℝ × S³_{R,t}, renormalized
perturbation theory, Hollands' gauge-fixed framework. Clause (d), already marked [Cd], contains one invalid inference:
the R- and t-independence of the regulator's c₀ does not follow from Step 10. The headline status needs one caveat
that is currently only in §7. I found no fatal error.

---

## 1. Citations: all real, all say what is claimed

| Citation | Metadata check | Quote check (v4 text) |
|---|---|---|
| Hollands, *Renormalized Quantum Yang-Mills Fields in Curved Spacetime*, Rev. Math. Phys. 20 (2008) 1033–1172, arXiv:0705.3340, DOI 10.1142/S0129055X08003420 | confirmed on arXiv (v4 dated 14 Mar 2018) | abstract quote verbatim; "toplogically trivial, i.e., of the form M × G" (§2.2, typo is Hollands'); "standard flat connection in the bundle M × G"; §4.1.2 "For simplicity, we again assume that M has compact Cauchy-surfaces"; T1 (eq. 136), T2 "homogeneously up to logarithms" (eq. 140), T4, T5, T11; §3.6: "the Dn depend only polynomially on the Riemann curvature tensor, the mass parameter m², and the field φ", eqs. (254), (257); Theorem 2 (eq. 263); Lemma 1 (eq. 47); Theorem 1 "simple or semi-simple compact Lie group with no abelian factors" (eqs. 63–64) and Remark 1; Lemma 11 (eq. 389); the remark after Theorem 3 ("must be exactly the same as in Minkowski spacetime!"); §3.5 "completely analogous to the scalar case"; §4.3 "the analog of conditions T1–T11"; the dimension/ghost tables (A 1, C 0, C̄ 2, B 2; A‡ 3, C‡ 4, C̄‡ 2, B‡ 2). **All verified.** |
| Hollands–Wald, CMP 223 (2001) 289–326, gr-qc/0103074, DOI 10.1007/s002200100540 | confirmed | (uniqueness; cited via Hollands) |
| Hollands–Wald, CMP 231 (2002) 309–345, gr-qc/0111108, DOI 10.1007/s00220-002-0719-y | confirmed | (existence; cited via Hollands) |
| Barnich–Brandt–Henneaux, Phys. Rept. 338 (2000) 439–569, hep-th/0002245, DOI 10.1016/S0370-1573(00)00049-1 | confirmed | (source of Hollands' Thm 1) |

One point in favour of the note that it doesn't use: Hollands himself applies Theorem 2 to the Yang–Mills
time-ordered products in §4.4 ("Recalling that by thm. 2, the changes in the time-ordered products are parametrized by
local, covariant maps Dn", just before eq. 390). So the YM version of Theorem 2 is not only "completely analogous". The
author also uses it in his own YM proof. That strengthens the caveat in §2, but it is still not a separately written proof.

---

## 2. Hypotheses: do they hold on M_{R,t}?

- **Global hyperbolicity, compact Cauchy surface (Step 1).** Correct. An ultrastatic metric −dτ² + R²h_t with
  (S³, h_t) compact is globally hyperbolic. The curve argument is fine.
- **Trivial bundle (Step 2).** Correct: [S³, BG] = π₃(BG) = π₂(G) = 0 for connected G. Hollands assumes M × G on
  every spacetime of the category, not just on M_{R,t}. That is his standing assumption, so nothing breaks.
- **Gauge group.** Compact connected simple, so no abelian factors. Matches Thm 1.
- **Coupling regime.** Formal power series around A = 0. Stated honestly (§7).
- **Signature.** Lorentzian. H4′ and the register's "S³ × ℝ" are Euclidean. §7 says so, but §8's suggested register
  text drops the word "Lorentzian" (see Fix 5).
- **Gauge fixing.** Theorem 2 compares two prescriptions *for the same auxiliary theory*: Lorentz gauge G = ∇^μA_μ, the
  standard flat background connection, the same field content. So C₀ is a statement inside that framework. Gauge-fixing
  independence is a separate issue (Hollands §5.2) and is not needed for c₀, but the setting should keep saying so.

---

## 3. Step-by-step check

**Step 4 (dimension exactly 4).** Valid, but the stated reason is incomplete. The construction does contain a
dimensionful constant: the length scale of the Hadamard parametrix (this is why T2 holds only "up to logarithms"). What
forbids, for example, ℓ² · Ric^{μν} tr F_{μρ}F_ν{}^ρ in D is T2 applied to *both* T and T̂. The difference D is
polynomial in curvature, so it cannot carry logarithms, so it must scale exactly. Hence (257). "Pure YM has no m²" is
true but is not the whole reason. *(Nit; Fix 4.)*

**Step 5 (cocycle).** Valid in substance. Going from an integrated identity "in the region f = 1" to ŝω = dη at the level
of densities needs the usual argument: (S₀+F, S₀+F) is supported in supp df modulo d-exact terms, and f is varied. This
is standard and is the same pattern Hollands uses around eqs. (389)–(391).

**Step 6 (enumeration).** The conclusion is correct. One bullet is muddled. Representatives (63) contribute **nothing** at
ghost number 0 in form degree 4. The total degree (form + ghost) of q_r is 2m(r) − 1 = 3 for the quadratic Casimir, and
any extra factor r_t or f_s raises it by at least 2. So no product reaches total degree exactly 4. tr F∧F comes from (47)
with Θ = tr F∧F, not from (63). The sentence "the only survivor is the ghost-0 top component dq = tr F∧F" should go.
Harmless. *(Nit; Fix 3.)* The other parts check out: a linear ad-invariant polynomial vanishes on semisimple 𝔤;
dimension-0 tensors are g and ε; the SO(1,3) invariant-theory step gives tr F∧⋆F and tr F∧F; tr F∧F = d CS(A) with
CS local, so it is trivial in H⁰(ŝ|d, P⁴).

**Step 7 (X has no curvature factor).** I tried to break it using the fact that C has dimension 0. Ghost −1 with field
dimension ≤ 2: C's add +1 ghost at no dimension cost, so you cannot reach ghost −1 with them. The only candidates are a
single C̄ or a single B‡, which are adjoint and so not singlets. The step holds.

**Step 8 (the correction to the route).** Correct, and a good catch. I checked by hand: on round ℝ × S³_R,
Ric = (2/R²)(g + u♭⊗u♭) and Scal = 6/R², so u_μu_ν = (3/Scal)Ric_{μν} − g_{μν}. The exclusion really does come from
polynomiality (T4/T5 on the *whole category*, which includes spacetimes where Scal = 0, so a 1/Scal coefficient is not
smooth there) together with exact dimension 4. The note should say explicitly that the 1/Scal term is ruled out by
smoothness across the category, not by anything on M_{R,t} itself. *(Nit.)*

**Step 9 (all orders).**
- The "a · tr F∧⋆F = finite renormalization of g" step is fine:
  (1+a)S_YM[A, λ] = S_YM[√(1+a)A, λ/√(1+a)], with the compensating gauge-fixing and ghost changes ŝ-exact.
- **Overclaim.** "ŝX with X linear in antifields is the BV form of a field redefinition." Nothing shows that X is linear
  in antifields, or that it contains antifields at all. X can contain gauge-fermion-type terms such as
  tr C̄ d⋆A (ghost −1, dimension 2 + 2 = 4, a colour singlet). Then ŝX is a *change of gauge fixing / BV canonical
  transformation*, not just a field redefinition. This doesn't touch c₀, because ŝ-exact terms are quotiented out by
  definition and don't change the physical cohomology. But Theorem C₀(b)'s last sentence ("a finite renormalization of
  g, a field redefinition and a c-number shift") is too narrow as written. *(Fix 2.)*
- The induction is my own deduction in Hollands' pattern, as §7 says. I accept it as [D].

**Step 10 (R, t independence for covariant schemes).** Valid. D is a single natural transformation on the category.

**Step 11 / clause (d).** **Here the deduction fails, in two places.**
1. (HR) as stated says the regulated series on M_{R,t} converges to "time-ordered products satisfying T1–T12". T1 is a
   condition *across* spacetimes, and Theorem 2's uniqueness needs both prescriptions defined on the whole category. A
   regulator defined only on the static sphere family can't satisfy T1 by itself. (HR) has to be restated: "on each
   M_{R,t} the limit coincides with the restriction of one fixed locally covariant prescription T*".
2. Even with the corrected (HR), "by Step 10 it does not depend on (R, t)" is a **non sequitur**. Step 10 is about the
   difference between two *covariant* prescriptions, which is a natural transformation. The bare c₀(ε; R, t) a
   non-covariant regulator needs is not a natural transformation. Matching fixes it separately on each (R, t), and
   nothing cited forbids it from depending on R (through ε²/R² heat-kernel terms) or on ε (scheme logarithms). Getting
   "one c₀ for every room" needs an extra hypothesis about the regulator, for example: its local UV structure is the flat
   one up to O(ε² · Riem) corrections that vanish as ε → 0 order by order. The formula c₀ = 1 + Σ_k c₀^{(k)} g₀^{2k} with
   *constant* coefficients is likewise an extra assumption. In the flat Hamiltonian lattice the speed-of-light
   renormalization is finite (Hasenfratz–Hasenfratz, as described in the route note), but that has not been shown for
   H2's spectral cutoff. *(Fix 1. This is the most important fix.)*

**§5 corollary.** The identification "g ↦ μ²g is motion along R at fixed t" is correct (τ′ = μτ). Citing the remark
after Theorem 3 is fair, and the note calls it a remark. Overreach: "That is the perturbative content behind register
line 14's −(b₀/8π²) log(Λ_cut R)". Line 14 is about a *non-covariant cutoff*. Hollands' flow is the flow inside a
covariant scheme, and only the scheme-independent leading coefficients transfer without the matching of clause (d).
*(Fix 6.)*

---

## 4. Is the claimed status honest?

Mostly. Two adjustments:
- The headline "[PC + D]" should carry its caveat in the same line: *modulo the vector/ghost analogue of Hollands–Wald
  uniqueness (Theorem 2), which Hollands asserts by analogy (§3.5, §4.3) and uses in §4.4, but does not prove
  separately.*
- (c)'s phrase "every locally covariant renormalized theory has c₀ ≡ 1" is close to empty, since a covariant scheme has
  no bare c₀ to compare. The real content is: **the marginal freedom of covariant schemes on Lorentzian ℝ × S³_{R,t}
  is the single coupling g (plus ŝ-exact changes and c-numbers); there is no second, anisotropic marginal parameter.**
  I'd state it that way.

**Honest status:** C₀(a)–(c) closed in renormalized perturbation theory on Lorentzian ℝ × S³_{R,t}, in Hollands'
framework (Lorentz gauge, trivial bundle, flat background connection), [PC + D] modulo the YM analogue of Hollands'
Theorem 2. C₀(d) is [Cd] on a corrected (HR), and its R/t-independence clause needs a further regulator-locality
hypothesis. None of this touches the Euclidean, non-perturbative H4′ or the two-parameter tuning that register line 23
states for the H2 regulator.

---

## 5. Fixes, in priority order

1. **Clause (d)/Step 11:** restate (HR) as "the limit equals, on each M_{R,t}, the restriction of a single locally
   covariant prescription". Remove "by Step 10 it does not depend on (R, t)". Add an explicit regulator-locality
   hypothesis (flat UV structure up to O(ε²Riem)) if R-independence of the bare c₀ is wanted. Drop the claim that the
   coefficients are constant, or label it as part of the hypothesis.
2. **(b), last sentence and Step 9:** "field redefinition" → "BV canonical transformation (field redefinition and/or
   change of gauge fermion)". Drop "X linear in antifields".
3. **Step 6:** representatives (63) give nothing at ghost 0 in form degree 4. tr F∧F comes from (47).
4. **Step 4:** credit T2 on both prescriptions for exact homogeneity. The Hadamard scale is a dimensionful constant of
   the construction.
5. **§8 suggested register text:** write "Lorentzian ℝ × S³_{R,t}" (the register's S³ × ℝ is Euclidean). Add "in
   Hollands' gauge-fixed framework". Qualify the replacement of the paper's §8 conjecture as perturbative: "in
   perturbation theory, c₀ is regulator data: covariant schemes have no c₀".
6. **§5:** drop "that is the perturbative content behind line 14", or restrict it to the universal leading β
   coefficients.
7. **Outcome line:** move the Theorem-2-for-YM caveat up next to "[PC + D]".

---

## 6. A Euclidean lead (not checked for hypotheses)

The note honestly leaves the Euclidean side open. One lead I could confirm exists, though I have **not** checked its
hypotheses or statements: K. J. Costello, "Renormalisation and the Batalin-Vilkovisky formalism", arXiv:0706.1533. Its
abstract says it renormalises theories on compact manifolds in the BV framework, including Yang–Mills "in dimension 4
only". If its counterterms are local and covariant in the Riemannian metric on S³ × S¹ (compact), the same
polynomiality-plus-dimension argument might give a Euclidean C₀. That would need reading, and S³ × ℝ is non-compact. I
cite it only as a place to look.
