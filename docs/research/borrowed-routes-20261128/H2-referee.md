# Referee report on `H2-closure.md` (KC1-ζ)

*28 November 2026 (night shift). Independent, skeptical referee pass. I did not edit the note, the register or the
paper. Context read: `H2-closure.md`, `H2-borrowed-routes.md`, `closed-set-20261127/CLOSED-SET.md` (lines 8, 13–16),
`closed-set-20261127/H2-M3-uniform-dressing.md` (§1, Step 4, KC₁ statement), and the frame convention of
`docs/papers/berger-spectrum/paper.tex` §Setting.*

**Verdict: holds with fixes.** I tried to break the deduction at every step and could not. The theorem as stated in
§2 (a one-loop, background-Feynman-gauge, 4D proper-time statement for periodic homogeneous paths on S³_t × S¹_β)
is correct. The citations are real and say what the note claims. The note's own scope statement (§6) is honest: it
does **not** claim KC1 as worded in H2 §6, and it correctly withdraws step (e) of the proposed route. The fixes below
are about presentation and bookkeeping, not about whether the result is true.

---

## 1. Citations (checked tonight)

**Vassilevich, "Heat kernel expansion: user's manual", arXiv:hep-th/0306138 (Phys. Rept. 388 (2003) 279).** I read it
on ar5iv.
- §3.4: the quadratic action (3.43) contains the term A_ν A_μ R^{μν} and √g, so it is written for a curved metric.
  The quoted (3.44)–(3.47), (3.49) and (3.51) match the source character for character. The ghost operator "is just
  the ordinary scalar Laplacian" with connection (3.49): confirmed. "The case κ≠1 yields a non-minimal operator":
  confirmed. "Only this coefficient contains information on the one-loop divergences": confirmed.
- (3.53) is printed as a_k^tot = a_k(D^χ) − 2a_k(D^{[gh]}_χ), where D^χ means the operator with the gauge-fixing term
  added. The note drops the χ. That is a notation slip only.
- §4.1: the odd-coefficient statement and (4.26)–(4.28) match.
- §4.2.1 is titled "Yang–Mills theory in flat space". (4.30)–(4.34) and the "11/3" sentence match. The note says
  correctly that these equations are flat-space statements and re-proves them on curved M.
- (2.27): a_n(f,D) = ζ(0,f,D). Confirmed.

**van den Heuvel, "Glueballs on the three-sphere", Nucl. Phys. B488 (1997) 282–334, arXiv:hep-lat/9608101,
doi:10.1016/S0550-3213(97)00016-3.** I read the arXiv PDF. Every quote checks out:
- "We will neglect higher order time derivatives of B and also terms of the form BⁿḂ²".
- Background gauge on the high modes, "choosing the Feynman gauge ξ = 1".
- "a laplacian term for an ε-dimensional torus of size L that we attached to our space S³".
- Eq. (91) and the sentence "the renormalization is an ultra-violet effect and does not depend on the global
  properties of the space".
- Eq. (117) contains 11/(12ε) on ċċ + ḋḋ, followed by "The divergency will be absorbed by the same coupling constant
  renormalization of eq. (91)".
- Eqs. (118)–(119), and κ₀ = 0.0566264741439181 in Table 1.
- The arithmetic of the cross-check works: (2π²/2)·11/(12π²ε) = 11/(12ε).

**Context citations (not proof steps).** Hasenfratz–Hasenfratz, Nucl. Phys. B193 (1981) 210, doi
10.1016/0550-3213(81)90526-5, and Karsch, Nucl. Phys. B205 (1982) 285–300, doi 10.1016/0550-3213(82)90390-X, both
exist. I could not read either abstract tonight. So the specific numbers that `H2-borrowed-routes.md` §2.4 attributes
to them (0.84 and 0.91, "the two derivatives sum to b₀") are not confirmed by me. They must stay "context only", as
the closure already treats them.

---

## 2. Step-by-step check

**Step 1 (Laplace type on a closed M).** Valid. M = S³ × S¹_β with dτ² + R²h_t is smooth, compact and has no boundary.
A_q is smooth. ξ = 1 gives minimal operators. Vassilevich's (3.43) is derived on a general metric and places no
condition on B (no field equation, no gauge choice for the background). Taking the background in temporal gauge is
harmless, because a₄^tot is gauge invariant in B.

**Step 2 (block diagonal).** Valid. Left translations are isometries of the left-invariant h_t. They fix every θᵏ and
dτ, so they fix A_q. They act by the left-regular representation on frame coefficients. D_v, D_gh and the background
gauge-fixing term are all natural in (g, B), so U_h commutes with them. One labelling remark: the register (line 13,
"multiplicity j + 1") uses j = 2·spin. "j ≥ 1" therefore includes spin ½. Since H_{≥1} is *defined* as H₀^⊥, nothing
breaks.

**Step 3 (the j = 0 block has no s⁰ term).** Valid. On invariant sections every e_k acts as 0, so ∇_i acts as the
constant-in-space matrix ω_i(τ). On the product, Γ^ρ_{0ν} = 0 and B₀ = 0. So D₀ = −∂_τ² + (matrix) on L²(S¹; ℂ^N).
That is Laplace type on a closed 1-manifold, and its heat trace has only half-odd powers of s. Hence no log comes
from j = 0. b₀ = (4π)^{−1/2}Nβ, with N = 12 and N = 3, is right.

**Step 4 (a₄ on curved M).** Valid. I redid it independently with a small numeric check: random antisymmetric F^γ_{μν},
random symmetric Ric, Riem built as a Kulkarni–Nomizu product (so it has the full algebraic symmetries), and su(2)
structure constants. Results: the F-part of tr E is 0, the E² cross term is 0, the Ω cross term is 0 (to 10⁻¹³),
tr(E_F²)/FFK = 4, Σ tr(Ω^FΩ^F)/FFK = −4, ghost/FFK = −1, and the total (180·4 + 30·(−4) + 60)/360 = 11/6. Then
11/6·(4π)⁻² = 11/96π². With K = 2δ and S₀ = ¼∫F², this gives a₄^tot ⊃ (11/12π²)S₀ = (b₀/8π²)S₀, with b₀ = 22/3.
Correct. The structural reason is simple: every cross term carries tr(ad) = 0, or the trace R^σ_{σμν} = 0.

**Step 5 (regulated fast action).** The signs check out: ½ log det D_v − log det D_gh = −½∫ds/s[…]. The a₄ term gives
−a₄ log(ΛR). The subtracted s^{−1/2} b₀ term gives +b₀(Λ − 1/R), and the b₂ term gives b₂(R − 1/Λ). The q-independence
of α₄, α₂ and α₁ is right. Two blemishes:
- (i) The display in (a) has **no q-independent log term**. The text "moves −c_R log(ΛR) into the α-terms", but the α's
  multiply powers of ΛR. As written, (a) is literally false whenever c_R ≠ 0. Fix: add α₀ log(ΛR) with α₀
  q-independent.
- (ii) The ζ remark ("for positive D … ζ(0) = a₄ − 0 − dim ker … by (2.27) applied blockwise") is muddled. Positive
  means the kernel is trivial. (2.27) is stated for a differential operator on M, and D|_{H_{≥1}} is not one. The
  correct justification is the Mellin transform of the Step 5 asymptotic, which needs D_{v,≥1} > 0 and
  D_{gh,≥1} > 0. Positivity fails for large F, where Nielsen–Olesen-type negative modes appear. So the ζ form holds
  only where positivity does. The proper-time form with the IR cut is unaffected.

**Step 6 (kinetic + potential).** Valid. In temporal gauge, F = q̇ dτ∧θ + F_sp. Also ¼|F|² = ½F_{0i}F_{0i} + ¼F_{ij}F_{ij},
√g = R³√h and g^{kl} = R⁻²h^{kl}, which gives G = R V_t h_t^{kl} δ_ab. a₄ is exactly ∫F² with no derivatives of F, so
"no other velocity structure in the log" is right.

**Step 7 (Part IV coordinates).** The result is right, but the bookkeeping needs a fix:
- §1 says the map to Part IV's q is "a fixed linear rescaling q ↦ c·g·q". In the note's coframe θᵏ (Maurer–Cartan,
  non-orthonormal), the kinetic metric is diag(1/t, 1, 1) ⊗ δ, which is **anisotropic** for t ≠ 1. The Berger paper
  (`berger-spectrum/paper.tex` §Setting) writes A = Σ M_i θ̂ⁱ in the **orthonormal** coframe, and line 13 has M = 2gq.
  In those coordinates the classical kinetic term is isotropic, which agrees with Part IV's flat ½|p|². So the map is
  linear but anisotropic, roughly diag(√t, 1, 1) in the long-axis slot, up to the frame-bracket factor 2. The log
  statement is invariant under any constant linear map, so the conclusion survives. The sentence in §1 is wrong as
  written.
- The note attributes "F = 2gq" to register line 8. It is line 13 ("background coordinates M = 2gq"). The same
  mis-cite appears in `H2-borrowed-routes.md` §1.
- Step 7 says "line 13 states that the homogeneous sector of the Yang–Mills Hamiltonian is H_red". Line 13 says the
  homogeneous *classical restriction* gives H_red, and adds explicitly: "This is not a direct sum of independent
  sectors of the interacting quantum Hamiltonian." Only the classical restriction is used, so reword to match.

**§5 cross-check.** Valid as a cross-check. Two wording fixes:
- "t = 1, F = 0" should read "t = 1, B = 0 (spatial F = 0)". The kinetic term needs F_{0i} = Ḃ ≠ 0.
- "My theorem extends his kinetic statement to every t and every F" should say "its **log part**". He also computes
  finite parts at B = 0, which this theorem does not.

**Domain claim ("for every F, no positivity used").** True for W_Λ *as defined* (proper time with the IR cut at R²).
Calling W_Λ "−log of the background-gauge Gaussian integral" (§1) is only literal where D_{v,≥1} and D_{gh,≥1} are
positive. Elsewhere W_Λ is a regularized definition. Say so in §1.

---

## 3. Is the status honest?

Mostly yes. To be precise about what was gained:
- **KC1-ζ is close to a standard fact.** In background-field gauge the one-loop divergence of Γ[B] is a gauge-invariant
  local functional of B (Abbott). On any closed 4-manifold that forces it to be ∝ ∫F² plus pure curvature, because
  terms linear in F vanish for a semisimple algebra. The genuinely new steps are small: the j = 0 subtraction (Step 3)
  and the homogeneous-path reading (Steps 6–7). They are correct and worth writing down. But the note should not be
  read as a substantial reduction of KC1.
- **The real content of KC1 is R3 + R4, and both are open.** R3 is the Coulomb-chart m^NA + m^C ↔ background-gauge q̇²
  coefficient. R4 is the log under the spatial cutoff χ(K(F)/Λ²R²). The closure says this clearly in §6. I agree that
  step (e) of the borrowed-routes plan fails as stated: R2 concerns the transverse Gaussian fibre, not W_Λ.
  - R3 is not a formality. Vassilevich notes that a_n^tot is gauge-independent only on shell. The off-shell q̇²
    coefficient is exactly the kind of quantity that can differ between gauges by EOM-proportional terms (the
    closure's §6 items 1 and 6).
- **Tag.** The note proposes [D] for KC1-ζ. The register defines [PC] as "proved and cited", and that fits better: one
  cited theorem with matching hypotheses, plus elementary steps. Either tag is defensible. The register is Bee's call.
- **H2 itself is untouched**, and the note says so (§6 item 8).

R2 (in the borrowed-routes note, used here only negatively): I re-derived the single-oscillator check. ⟨2|x²|0⟩ =
√2/(2ω) gives m = ω′²/(16ω³), and V̇²/(64V^{5/2}) = ω̇²/(16ω³) with V = ω². ✓

---

## 4. Required fixes (none is fatal)

1. Add the q-independent α₀ log(ΛR) to (a), or say explicitly that only q-dependent divergences are displayed.
2. Correct §1's "scalar rescaling" to "constant (anisotropic) linear map". Cite line 13 (M = 2gq) instead of line 8,
   and note the orthonormal coframe of the Berger paper.
3. Reword Step 7's paraphrase of line 13 (classical restriction only).
4. In §1, state that W_Λ is −log of a Gaussian integral only where both fast operators are positive. Otherwise it is
   the proper-time definition.
5. Replace the ζ remark in Step 5 with "by Mellin transform of the Step 5 asymptotic, where D_{≥1} > 0".
6. §5: write "B = 0" for "F = 0", and "log part" in the extension claim.
7. Notation: write D^χ in the (3.53) quote. Optionally state the j = 2·spin convention.
8. Optionally record KC1-ζ as [PC] rather than [D]. Keep KC1 at [Cd on R3, R4] and H2 at [X].

**Honest status.** KC1-ζ is proved: the one-loop background-Feynman-gauge proper-time/ζ log for periodic homogeneous
paths on S³_t × S¹_β, every t > 0. It rests on Vassilevich with matching hypotheses, with (4.30)–(4.34) correctly
re-proved on curved M, plus elementary steps. KC1 as worded in H2 §6 is conditional on R3 and R4, both open. H2 stays
open.

## Sources checked
- D. V. Vassilevich, "Heat kernel expansion: user's manual", Phys. Rept. 388 (2003) 279–360, arXiv:hep-th/0306138
  (ar5iv: §3.4, §4.1, §4.2.1, eq. (2.27)).
- B. van den Heuvel, "Glueballs on the three-sphere", Nucl. Phys. B488 (1997) 282–334, arXiv:hep-lat/9608101,
  doi:10.1016/S0550-3213(97)00016-3 (arXiv PDF: §3.1–3.5, eqs. (80), (89)–(91), (116)–(119), Table 1).
- Bibliographic records only: A. Hasenfratz and P. Hasenfratz, Nucl. Phys. B193 (1981) 210–220,
  doi:10.1016/0550-3213(81)90526-5; F. Karsch, Nucl. Phys. B205 (1982) 285–300, doi:10.1016/0550-3213(82)90390-X.
