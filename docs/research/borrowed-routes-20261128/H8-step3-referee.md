# H8 Step 3 closure: referee report

28 November 2026. I refereed `H8-step3-closure.md` (the projected-Gaussian min–max lemma) against
`H8-step3-borrowed-routes.md` and the register (`closed-set-20261127/CLOSED-SET.md`, lines 9, 17, 19 and the H8
row). I tried to break it. **Verdict: the lemma holds. The fixes below are small, and none of them touches the
logic.** Its status line is honest: the lemma is proved conditional on Theorem A and the symmetry reduction, and
Step 3 is *not* closed until the interval computation runs. Nothing in the register should change because of this
note.

## 1. What I checked, step by step

**Setting and inputs (T-A), (T-S).** Line 9 gives Theorem A [P] for 0 < s < (t+2)/(t+1). At t = 3 that window is
s < 5/4, so s = 1 is inside it, for every g > 0, with compact resolvent. Line 17 and H8 §1 give the commuting colour
and circle actions. The note uses only that H_g is a closed extension of T = −½Δ + V₀ on C_c^∞. That covers both
realizations the register mentions. (They coincide anyway: V₀ ≥ 0 is a polynomial, so T is essentially self-adjoint
on C_c^∞ by Kato's inequality, Reed–Simon II Thm X.28. The note does not need this.) The restriction to the reducing
subspace 𝓗₀ keeps compact resolvent. Correct.

**§4.1 Hessian.** I recomputed J by finite differences of ∇W at g = 0.3. This is independent of the note's analytic
matrix. ∇W(a) = 4×10⁻¹⁶, and the spectrum is {−0.792287, 0, 0, 0, 1.154701 (×2), 2.309401 (×2), 2.524338}. That is
exactly the table: λ₋ = −ω_C, the triple kernel, 2u (×2), 4/√3 (×2), μ₊. The kernel is the colour tangent space,
with |JK| = 4×10⁻¹⁶. ½tr|J| = 5.1224140 = 3S/2 + ω_C. I also checked the identity by hand: μ₊ + ω_C = √3 + 2ω_C gives
½tr|J| = 15/(2√3) + ω_C. The competitor eigenvector check, the relation 2r² − √3r − 1 = 0, is right.

**§4.2 Schwartz class.** The pointwise bound φ_O(q) ≤ exp(−½λ_min(|q|−|a|)₊²) uses |aO| = |a| and B_O ≥ λ_min.
Derivatives are polynomial × Gaussian, uniformly in O on a compact group, so dominated convergence applies. Correct.

**§4.3 Domain.** The cutoff argument is standard and correct. χ_n is radial, so it preserves singlet and charge 0.
Also ‖∇χ_n‖_∞ = O(1/n), Tf ∈ L² because V₀ is polynomial, and the Friedrichs extension contains T̄.

**§4.4–4.5 Singlet and charge 0.** I traced the substitution y = qRO^TR^T. It gives ∫φ(q(ROR^T)^T)dO, and
conjugation invariance of Haar measure finishes it. W∘L_φ = W uses ω₂ = ω₃ and det R = 1. L_φa = a, so L_φ commutes
with J, hence with |J| and P_ker, hence with A. Correct. The research note's version ("on singlets this equals
q ↦ Rq") was also fine. The closure's version is more careful.

**§5 Linear independence.** This is the real new content, and it is correct.
- e^T A e = ω_C, because e is a J-eigenvector orthogonal to ker J.
- I evaluated e^TΩe = 0.8531892 two ways, and it matches (19+√33)/(√3(11+√33)).
- The rational certificate is valid. ω_C < 0.8 ⟺ √3 > 1.7. And 19 + √33 > 24.7445 > 24.6527 >
  0.85(11√3 + 3√11), with 5.7445² = 32.99928 < 33, 1.7321² = 3.00017 > 3 and 3.3167² = 11.0005 > 11.
- The ratio lower bound h_δ exp(½s²e^TΩe − ½(s(√ω_C+δ)+|a|_A)²) → ∞ follows from the triangle inequality in |·|_A
  and positivity of Haar measure on open sets.
- ψ₁ > 0 rules out c = 0 as well.

So the research note's "different centres" gap is genuinely closed.

**§6.1 Collapsing the averages.** H_gφ_O = (H_gφ)_O uses cof(qO^T) = cof(q)O^T for O ∈ SO(3). Correct. The
substitution gives N_H(O₂O₁^T), which I re-derived: N_H(O) uses (Hφ)(yO^T), and O^T = O₁O₂^T. Fubini is justified by
the uniform Gaussian bound. Correct.

**§6.2 Pair formula.** The kinetic term is ½tr(A₁A₂Σ) + ½(c−a₁)^TA₁A₂(c−a₂), from Green's formula and q − a_i =
(c − a_i) + z. The normalization (2π)^{9/2}(det C)^{−1/2}e^{−κ/2} and the κ identity are standard. Correct.

**§6.3 Closed forms.** I checked by hand that m₀₀ = π^{9/2}3^{3/4} (det Ω = 3^{−3/2}). I also checked
k₀₀/m₀₀ = 3S/2 + 15g²/4: the kinetic part ¼trΩ = 3S/4, the potential part 3S/4, and ½g²·7.5 from
E[cof²] = 2var var, with row sums 4.5 + 1.5 + 1.5. A 4×10⁵-sample Monte Carlo of E V₀ at g² = 0.07 gives 2.4249
against the closed form 2.4276, which is within sampling error. The real-analyticity of N₁ and N_H uses
det C ≥ (2λ_min)⁹. Correct.

**§6.4 Min–max step.** dim V = 2 gives a unit ψ ∈ V ∩ {φ₁}^⊥ in D(H_g) ∩ 𝓗₀. So μ_2(H_g|𝓗₀) ≤ μ₂. Compact
resolvent puts alternative (1) in force, so μ_2 = E_{2,0}. Item (5): with 𝐌 > 0, 𝐊 − U𝐌 < 0 ⟺ both generalized
eigenvalues are < U ⟺ (1,1) entry < 0 and det > 0 (Sylvester, 2×2). Correct.

**§7 Tail bound.** κ ≥ (λ_min/2)|a − aO|², from (A^{-1} + B_O^{-1})^{-1} ≥ λ_min/2. And
‖1 − O‖_F² = 4 − 4cos θ = 8 sin²(θ/2), with the row weight ≥ u². The prefactor (2π)^{9/2}(2λ_min)^{−9/2} =
(π/λ_min)^{9/2}. Correct. The numbers e^{−7.5} and ≈ 490 at g² = 0.07, θ = π also check. The Haar density
(2 − 2cos θ)/(8π²θ²) integrates to 1 (I checked numerically).

## 2. Citations (verified)

- **Reed–Simon IV, Thms XIII.1 and XIII.64.** The note relies on Fan's restatement. I fetched arXiv:1107.1290
  (Huijun Fan, "Schrödinger equations, deformation theory and tt*-geometry") and extracted the text. §2.1.1 there
  reads "We write down the Theorem XIII.1 of [RS]: Theorem 2.1. Let H be a self adjoint operator of a Hilbert space
  that is bounded below …", with exactly the two alternatives quoted in the note. §2.1.2 restates XIII.64 as
  Theorem 2.2, equivalences (1)–(6), including "(6) µn(H) → ∞". The quotation in the note is faithful. The
  hypotheses (self-adjoint, bounded below) match H_g|𝓗₀. Caveat, which the note already states: this is a secondary
  restatement, not Reed–Simon's own pages.
- **Peierls and Yoccoz, Proc. Phys. Soc. A 70 (1957) 381–387, DOI 10.1088/0370-1298/70/5/309.** Confirmed: issue 5,
  1 May 1957, IOP. The abstract describes treating shell-model wavefunctions as variational trial functions for
  collective motion. It is used for the idea only, and every property is proved in the note. Correct and honest.
- **Isserlis, Biometrika 12 (1918) 134–139, DOI 10.1093/biomet/12.1-2.134.** Confirmed. Its hypotheses (a centred
  Gaussian with finite covariance) hold, because C = A₁ + A₂ > 0. The receipt uses the non-central expansion, which
  follows by writing q = c + z. That is standard, but it is not spelled out (see fix 6).

No invented citation and no misquoted theorem. No transfer problem either: everything borrowed is abstract Hilbert
space or Gaussian algebra, with no geometry, coupling or volume hypotheses to transfer.

## 3. Independent floating cross-check

I wrote a separate re-implementation, kept outside the repo, that does not share code with the receipt. J comes from
finite differences, E V₀ from a degree-5 Gaussian cubature with 163 points (exactness checked on every monomial of
degree ≤ 5) instead of the Isserlis code, and the SO(3) integral from Haar importance sampling instead of a grid.
With 3000 samples:

| g² | α | R(ψ₁) mine | R(ψ₁) note | μ₂ mine | μ₂ note | overlap mine / note | ESS |
|---|---|---|---|---|---|---|---|
| 0.07 | 1 | 5.106 | 5.1085 | 5.1118 | 5.11394 | 0.0238 / 0.024 | 2726 |
| 0.002 | 1 | 5.113 | 5.1222 | 5.113 | 5.12217 | 4.8e−91 / 5e−91 | 2615 |
| 0.07 | 4 | 4.906 | 5.0255 | 4.923 | 5.0386 | 0.041 / — | 714 |

At α = 1 the agreement is within my Monte Carlo noise, about 0.03. At α = 4 my estimate is lower by 0.12, with a
much smaller effective sample size. I read that as noise, not a contradiction, but it is not a confirmation either.

## 4. Fixable issues

1. **Misattributed "weakening".** The closure says the research note claimed "Step 3 is *equivalent* to certifying
   μ₂ < 5.2731". The research note never says "equivalent" (I grepped it). It already uses sufficiency language ("a
   certified enclosure … showing μ₂ < 5.2731"). The non-equivalence remark is true, but it should not be presented as
   correcting a claim that was not made.
2. **Arithmetic slip.** 274/(30√3) = 5.273132…, not 5.27317. This is harmless, since 5.2731 is still below it.
3. **Cross-reference.** §4.1 says the finite-difference check is in "the receipt script (§8)". The receipt is
   described in §7.
4. **Uncorrected claim about positivity.** The research note says "the quotient is a ratio of positive integrals …
   robust". N_H is not positive: h = N_H/N₁ ranges over [−0.52, 5.87] at g² = 0.002, α = 1, and over [−10.3, 8.1] at
   g² = 0.07, α = 4 (my sampling). The closure's §7 correctly works with ∫(h − U)N₁. It should say explicitly that h
   changes sign and that the integral involves cancellation, which widens interval enclosures. The cancellation is
   worst at α = 4, which is exactly where §7 suggests certifying.
5. **α = 4 is not yet trustworthy as the certification choice.** The convergence check (nq = 15, 21, 27) is reported
   for g² = 0.07 at α = 1 only. The α = 4 rows have no convergence check, and my independent estimate differs by
   0.12. At α = 1 the floating margin is already 0.151 to 0.159 at all five couplings, so I would certify at α = 1
   everywhere, and use α = 4 only after its own convergence check.
6. **E_{N(c,Σ)}V₀ is not written out.** It is "apply (B3)". For an interval code that someone else must audit, the
   explicit non-central 2nd-, 3rd- and 4th-moment expansion of each (ω_k q_kc − g cof_kc)² should be in the note.
   The receipt's `E2`/`E3`/`E4` are the right formulas.
7. **Downstream advice for Step 4.** The corollary tests U = 5.2731. Step 4 needs E_{1,m} > E_{2,0}, so the more
   useful output is the certified upper end of μ₂ itself (≈ 5.11 to 5.12 plus enclosure width) at each coupling.
   That leaves Step 4 about 0.36 of harmonic room at |m| = 2, instead of about 0.21. Record the enclosure, not only
   the pass/fail at 5.2731.

## 5. Fatal issues

None found. Every step is either proved in the note or rests on an abstract theorem whose hypotheses (self-adjoint,
bounded below, compact resolvent; Gaussian with positive covariance) hold exactly here.

## 6. Honest status

The lemma E_{2,0}(g) ≤ μ₂(𝐊, 𝐌), for t = 3, s = 1, every g > 0 and every α > 0, is **proved, conditional on the
register's Theorem A (line 9) and the symmetry reduction (line 17 and H8 §1)**. Both are registered [P]. I would tag
the lemma [D] (closed by deduction) as a proof note, with no register status change.

**Step 3 remains open.** It reduces to a finite interval computation at five couplings. The floating margin of about
0.15 at α = 1 is real by two independent implementations. It is not an enclosure, and it removes no charge from the
tail.

Sources:
- [arXiv:1107.1290 (Fan)](https://arxiv.org/abs/1107.1290)
- [Peierls–Yoccoz record (Scilit)](https://scilit.com/publications/ca187edcedd7cda1ac25c53b4b43136e)
- [Isserlis 1918 scan (Zenodo)](https://zenodo.org/records/1431593/files/article.pdf)
