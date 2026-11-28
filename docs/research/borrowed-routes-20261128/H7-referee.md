# H7 closure: referee report

28 November 2026 (overnight). I wrote this as a skeptical referee of my own note `H7-closure.md`, with
`H7-borrowed-routes.md` and `closed-set-20261127/CLOSED-SET.md` as context. **I edited no existing file.**

**Verdict: holds with fixes.**

- **Theorem A** (vacua of the unsourced reduced model, every compact simple 𝔤, every t > 0, g > 0) survives every
  attack I tried. It earns **[D] in the reduced model at s = 1**. Exactly one cited theorem carries content
  (Whitehead's first lemma), and its hypotheses match exactly.
- **Corollary B** is correctly labelled **[Cd on (S)]**. Given (S), the step from levels to eigenvalues is valid. I
  found nothing that upgrades it, and nothing that breaks it.
- The note's correction of the research note for t < 1/3 is **right**: I confirmed it independently.
- The fixes below are wording and completeness repairs. None of them changes a status.
- **H7 (line 24) stays [X].**

---

## 1. Did the borrowed theorems' hypotheses really hold?

| Borrowed item | Its hypotheses (as I verified them) | Here | Transfer step needed? |
|---|---|---|---|
| Whitehead's first lemma (MIT 18.745 Lec. 18, Thm 18.4) | 𝔤 semisimple, characteristic 0, V a finite-dimensional representation | su(2) over ℝ; V = 𝔤 via ad∘φ, which is a representation because φ is a homomorphism | **None.** The cocycle convention a([x,y]) = x∘a(y) − y∘a(x) (MIT p. 1) matches Step 2's kernel equations exactly. I checked the (e₂, e₃) case term by term. |
| Sylvester's law of inertia | A Hermitian, S nonsingular | A = 1 − L restricted to ℂ³⊗U, which is Hermitian (L is symmetric, as I re-derived); S = D, which is real diagonal positive | **None.** |
| DSV §5.2–5.3 | u(N), round sphere (t = 1), irreducible block first, then reducible block by block | Used only as a precedent at t = 1 | Correctly **not** used as a logical input. The proof re-derives the spectrum from S·J for every t. |
| Simon 1983, Thm 1.1 | (A1) h, g C^∞; (A2) g bounded below, h ≥ 0; (A3) finitely many zeros of h, with h bounded away from 0 outside a ball; (A4) positive definite Hessian at each zero; full L²(ℝ^ν), flat Laplacian | Zeros are positive-dimensional orbits; the statement is needed on colour-singlet and charge sectors | **Does not apply**, as the note says. Simon writes: "Condition (A4) is critical for the results to have the form they will; we hope to study the case where h vanishes on a manifold in a later paper in this series" (p. 296 of the scan). So keeping (S) as a named input is honest. |
| Witten 1982 | the supersymmetric operator on forms | used only as the idea that ½ΔW is what the factorisation subtracts | Cited for the idea only. Correct. |

**Regimes.**

- *Coupling.* Theorem A holds for every g > 0, because it is classical. Corollary B is a g → 0 statement, so it is
  semiclassical. The rescaling p = gq, ħ = g² puts it in Simon's form −Δ + λ²h with λ = g⁻² and h = 2V¹. I checked
  that the harmonic levels come out O(1) and do not depend on g, as the note says.
- *Perturbative or not.* Corollary B is leading-order harmonic only. It has no rate and no explicit g₀. The note
  says so in §8.
- *Finite or infinite volume.* Everything lives in the homogeneous-mode reduced model on S³_t. No statement about
  the physical room is made, and none is needed for the claims as written.

---

## 2. Are the citations real, and do they say what is claimed?

All checked tonight against primary sources where I could reach them.

| Citation | Verified | Notes |
|---|---|---|
| MIT OCW 18.745 (Fall 2020) Lecture 18, Theorem 18.4 | **Yes, verbatim.** "Theorem 18.4. (Whitehead) If g is semisimple in characteristic zero then for every finite dimensional representation V of g, H¹(g, V) = 0." The 1-cocycle and 1-coboundary definitions also match the quoted text. | — |
| Jacobson, *Lie Algebras*, Ch. III §7, Lemma 3 | Location confirmed only via the Wikipedia page ("Jacobson 1979, Ch. III, § 7, Lemma 3") | The note discloses that I did not see Jacobson's page. Fine. |
| Dasgupta–Sheikh-Jabbari–Van Raamsdonk, JHEP 0205 (2002) 056, hep-th/0205185 | **Yes.** I read the arXiv v4 PDF. §4.2: "J^i form a representation of the SU(2) algebra"; "labeled by the N dimensional representations of SU(2)". §5.2: "λ(λ + j)(λ − (j + 1)) = 0 so we may have λ = 0, λ = −j or λ = j + 1"; the λ = 0 modes "correspond to fluctuations in the directions of the gauge orbit and not to physical zero-modes". §5.3: spins ½\|N_k − N_l\| ≤ j ≤ ½(N_k + N_l) − 1, and "the diagonalization proceeds exactly as above". | The quotes are accurate. Small slip, in the research note only: it writes "M = λμ/3", while DSV write M² = (λ/3)² in their units. This is immaterial. |
| Simon, Ann. IHP Phys. Théor. 38 (1983) 295–308 | **Yes.** I read the numdam PDF. The hypotheses (A1)–(A4), Theorem 1.1 and the manifold-scope quote are as stated. | §6 of Simon sketches a manifold version (Laplace–Beltrami operators on forms, still with **point** wells). The note could mention it, since it handles the non-flat metric that slice coordinates produce. It does not handle orbit wells. |
| Simon erratum, ibid. 40 (1984) 224 | **Yes.** "Theorem 5.1 is incorrect as stated … one must assume that degenerate states in the same well have the same parity." | The note does not use Theorem 5.1. Correct. |
| Witten, J. Differential Geom. 17 (1982) 661–692, doi:10.4310/jdg/1214437492 | **Yes** (Princeton record, MaRDI) | — |
| Horn–Johnson, *Matrix Analysis* (1985), Thm 4.5.8 | Confirmed only second-hand: several papers cite "Theorem 4.5.8, p. 223" for Sylvester's law | Disclosed in the note. The numbering is for the 1st edition. If the 2nd edition (2013) is cited, the number should be checked. |

**Research-note citations (spot checks, not used by the closure's proofs).**

- Kim–Klose–Plefka, hep-th/0306054. The abstract says plane-wave matrix theory "arises through a consistent
  truncation to the lowest lying modes" of N = 4 SYM on round S³. Confirmed.
- Lucini–Teper, JHEP 0106:050, hep-lat/0103027. The value 3.37(15) appears in the paper. Confirmed.
- Aharony–Marsano–Minwalla–Papadodimas–Van Raamsdonk, ATMP 8 (2004) 603–696, hep-th/0310285. Confirmed.
- I did not re-check the G₂, Athenodorou–Teper, Lucini–Teper–Wenger, Lüscher or van Baal entries. They are
  evidence only.

**No invented citations found.**

---

## 3. Is every step valid? (step-by-step audit)

I re-derived each step by hand and ran two small independent scripts (§5).

- **Lemma 0.**
  - The gradient and Hessian are correct, including the Ad-invariance sign ⟨X₁,[δ,X₃]⟩ = ⟨δ,[X₃,X₁]⟩.
  - tr Hess W = nS is correct, because the frame-diagonal blocks of Hess 𝒞 vanish.
  - Hess V = (Hess W)² at a zero is correct.
- **Step 1 (A1).** The identity g a_l a_m = ω_k a_k checks. The bijection with Hom(su(2), 𝔤) is correct. My
  basis-free check: e_k = −iσ_k/2 satisfies [e₁, e₂] = e₃. q_MC^ι = (1/g)·diag(1/√t, 1, 1) in H7 Step 3 matches a.
- **Step 2 (A3).**
  - The kernel equals the 1-cocycles. Whitehead then gives coboundaries, and these are tangent to the orbit.
  - The converse inclusion is correct.
  - The local-structure argument (slice, inverse function theorem, submersion of G × Σ_ε at (e, q)) is correct. It
    uses ran Hess W = T^⊥, which holds because Hess W is symmetric.
- **Step 3 (A2).**
  - The boundedness bound is correct as written: Σ (2j+1)·j(j+1)/3 ≤ n·(n²−1)/12, since j(j+1) ≤ (n²−1)/4.
  - Compact, with each orbit open in 𝒵, gives finitely many orbits. Correct.
- **Step 4 (A4).**
  - Hess W = D(1 − L)D: I re-derived it entrywise.
  - L is symmetric: re-derived.
  - 1 − L = 1 + S·J: the sign is pinned by the explicit spin-1 block, where L has eigenvalues {2, −1} on
    span{f₁, f₂}, consistent with 1 + S·J ∈ {l+1, 0, −l} and not with the opposite sign.
  - The S·J eigenvalue arithmetic is correct. The Sylvester transfer is correct.
- **Step 5 (offset).** Off = ½Σ_{μ≠0}|μ| − ½ tr Hess W = Σ_{μ<0}|μ|. Correct.
- **Step 6.** The 2×2 block [[√t, −√2], [−√2, 0]] is correct, and its negative eigenvalue is −ω_C. The spin-½
  spectrum {0², (3/(2√t))², ((2t+1)/(2√t))²} is confirmed numerically for t ∈ {0.1, 1/3, 1, 3, 10}.
  **Wording gap: fix F1 below.**
- **Step 7 (A5).**
  - The root-system Cauchy–Schwarz argument (|⟨α, θ^∨⟩| ≤ 1 for α ≠ ±θ, θ long) is correct, and gives m₁ = 1,
    m_{l≥3/2} = 0.
  - The lower bound Off(φ) ≥ ω_C, from φ(su(2)) ≅ spin 1, is correct.
- **Step 8.** The hypotheses of (S) are supplied by A2, A3 and the vacuum-floor growth bound (an in-house closed
  input).
- **Step 9 (charge on a well).**
  - R_θ q_φ = Ad(exp(−θZ₁)) q_φ: I checked the rotation directions.
  - exp(2πZ₁) maps to the central −1 of SU(2), so it lies in K. Only integer charges then appear on K-invariant
    functions.
  - ρ_θ normalises K, so it preserves K-invariant functions. The note does not say this, but it is immediate.
- **Step 10 (harmonic levels).**
  - Origin degree-2 invariants: Λ²(𝔤)^G = 0 and S²(𝔤)^G is one-dimensional for simple 𝔤, so the listed four
    families are complete. Three or more quanta lie at ≥ 3·min(√t, 1/√t). Correct.
  - Inequalities (i)–(iii) are correct. (iii) has its unique solution at t = 1/3, confirmed symbolically.
  - Well levels: the ground state is K- and ρ-invariant; every excited level is above Off. Correct.
- **Step 11 (counting).** This is valid. It is the cleanest part of the note: it needs (S) only on the full singlet
  sector and on the charge-0 sector, so it never needs uniformity over the infinitely many charge sectors m ≠ 0.

**The t < 1/3 correction is correct.** At t = 0.1, ω_C = 1.2649 while 2√t = 0.6325. So the origin's X₁-breathing
level is the second charge-0 level there. The research note's "→ ω_C(t) for every t" is wrong for t < 1/3. The
H8(G) ordering itself (charge 0 below every charged level) survives for every t > 0, conditional on (S). My control
t = 3 is unaffected.

**Consistency with H7 Step 4(c).** The centraliser formula ω₂² − b = s[(t+2) − s(t+1)]/t gives 1/t at s = 1. That is
frequency 1/√t, exactly Theorem A's spin-0 block. At the long-root well for SU(3), the charged centraliser singlet
sits at ω_C + 1/√3 = 1.3697 at t = 3. That is above the charged origin threshold 2/√3 = 1.1547, consistent with
"1.15470 or above".

---

## 4. Fixes (none changes a status)

- **F1 (Step 6, wording).** f₁ = (Z₁, 0, 0)/ζ, f₂ = (0, Z₂, Z₃)/(√2ζ) live only in the particular spin-1 copy
  φ(su(2))_ℂ. For a general φ with m₁ > 1, the formula Off = Σ m_l ε_l needs every spin-1 copy to give the same ε₁.
  - That holds, but the note should say why. Each irreducible summand U ≅ V_l carries an invariant Hermitian form,
    unique up to scale (Schur). So an isometric intertwiner U → V_l conjugates the block D(1 − L)D on ℂ³⊗U to the
    model block, and ε_l depends only on l and t.
  - In Step 6, replace Z_k by u_k = Ψ(Z_k) for an equivariant isometry Ψ (up to scale) onto U.
  - A5 is unaffected, since the long-root well has only the copy φ(su(2)).
- **F2 (Step 9, one line).** Add that ρ_θ normalises K, because ρ_θ fixes q and commutes with Ad. Hence it acts on
  K-invariant functions on N_φ.
- **F3 (§8, uniqueness of the competitor's well).** The missing argument is short. I record it as a suggestion; I
  have not written it out to proof standard.
  - Off(φ) = ω_C forces m₁ = 1 and m_l = 0 for every l ≥ 3/2, so ad h has spectrum in {0, ±1, ±2} with
    dim 𝔤₂ = 1.
  - Conjugate h into the closed dominant Weyl chamber. Then α(h) ≤ θ(h) ≤ 2 for every root α, and 𝔤₂ ≠ 0 forces
    θ(h) = 2. Since 𝔤₂ is one-dimensional, 𝔤₂ = 𝔤_θ and e ∈ 𝔤_θ.
  - Kostant's conjugacy of sl₂-triples with the same e then identifies φ_ℂ with the root triple up to G_ℂ.
  - One more step is needed to pass from G_ℂ-conjugacy to G-conjugacy of compact homomorphisms. It needs a precise
    citation (e.g. Collingwood–McGovern, *Nilpotent Orbits in Semisimple Lie Algebras*, Ch. 3). I have not verified
    a theorem number, so the note should keep "not claimed" until it is checked.
- **F4 (a borrowable source for (S) that the note misses).** B. Helffer and J. Sjöstrand, "Puits multiples en
  mécanique semi-classique VI. (Cas des puits sous-variétés)", Ann. IHP Phys. Théor. 46 (1987) 353–372 (numdam
  AIHPA_1987__46_4_353_0; MR912154).
  - I read the abstract and introduction. It treats −h²Δ + V₀ + hV₁ on a compact Riemannian manifold or on ℝⁿ.
    The wells are compact connected submanifolds Γ_i on which V₀ − E₀ vanishes exactly to order 2.
  - It studies the "uniformly degenerate" case, where the sub-principal potential (the transverse zero-point energy)
    is constant along Γ. That is exactly my situation, since Ad(G) acts transitively on each orbit well. The
    authors mention examples with a compact group action.
  - Theorem 1.1 there builds a WKB eigenvalue E₀ + E₁h + E₂h² + O(h³) for the Dirichlet problem in a tubular
    neighbourhood. Part 2 proves it is the only eigenvalue there.
  - **It does not by itself give (S).** It is about the bottom level of each well on full L², not about the excited
    harmonic tower in colour-singlet and charge sectors.
  - It is a closer precedent than Simon for "orbit wells", and worth citing as the partial borrowed route for (S).
    (S) still needs the symmetry-reduction step: G-invariant functions near G/K are K-invariant functions on the
    normal slice, and the along-orbit operator acts on them as a constant. Plus IMS in the sectors.
- **F5 (title).** "…the vacua of the unsourced room, closed" could be misread as H7 closed. Something like "H7,
  every compact simple group: vacua of the unsourced reduced model (Theorem A [D]; Corollary B [Cd on (S)])" is
  safer. The body already says line 24 stays [X].
- **F6 (research note only).** §5(5) attributes "exactly one spin-1 copy" to Borel–de Siebenthal. That is a
  misattribution; it is the elementary root-string fact re-proved in the closure's Step 7. The research note's
  Corollary B limit is wrong for t < 1/3; the closure already corrects it.

---

## 5. Independent checks I ran (small, under a second of CPU each)

- **Universal blocks.** D(1 − L)D on ℂ³⊗V_l, built from explicit spin matrices, for l ∈ {0, ½, 1, 3/2, 2} and
  t ∈ {0.1, 1/3, 1, 3, 10}:
  - inertia (3,0,0), (4,2,0), (5,3,1), (6,4,2), (7,5,3) exactly;
  - tr = (2l+1)S to 10⁻¹⁴;
  - ε₁ = ω_C to 10⁻⁶;
  - spin-½ eigenvalues 3/(2√t) and (2t+1)/(2√t), each twice;
  - ε_{3/2}(3) = 2.431281 and ε₂(3) = 4.932883, matching the research note.
- **Threshold.** The symbolic solve of ω_C = 2√t gives t = 1/3.
- **Independent su(3) check.**
  - Setup: my own basis −iλ_a/2 with ⟨A, B⟩ = −2tr(AB). The Hessian of **W** was taken by finite differences of W
    itself, with no use of Lemma 0.
  - Long root at (t, g) = (0.2, 1.3) and (3, 0.7): ∇W ≈ 10⁻¹⁰, dim ker = 7, one negative eigenvalue, Off = 1.208175
    and 0.792287 = ω_C(t), tr − 8S ≈ 10⁻⁸.
  - Principal (spin-1, so(3) ⊂ su(3)) at t = 3: dim ker = 8, four negative eigenvalues, Off = 5.725170 = ε₁ + ε₂.

---

## 6. Honest status

| Item | Status the deduction earns |
|---|---|
| A1 classification by Hom(su(2), 𝔤) | **[D]**, elementary |
| A2 finiteness, compact zero set | **[D]**, elementary |
| A3 Morse–Bott, ker = orbit tangent | **[D] by citation**: Whitehead's first lemma, hypotheses exact |
| A4 inertia per spin for every t; Off = Σ_{μ<0}\|μ\|; ε₁ = ω_C | **[D]**, with fix F1 (a wording repair) |
| A5 long root: index 1, Off = ω_C; Off ≥ ω_C for every φ ≠ 0; doublet frequencies | **[D]** |
| Corollary B: H8(G) at small g, s = 1, every t > 0; limit min(ω_C, 2√t) | **[Cd on (S)]**. (S) is not covered by Simon (point wells, full space). Helffer–Sjöstrand VI is a closer partial precedent (submanifold wells, uniformly degenerate case), but sectors and excited levels still need the reduction step. |
| H7 (line 24) | **stays [X]** |

All of this is reduced-model, s = 1 only. Nothing transfers to the sourced controls (s = 1.15, where the centraliser
is soft), to the physical room (H2, H2b), or uniformly in 𝔤.

Sources checked tonight:

- [MIT OCW 18.745 Lecture 18](https://ocw.mit.edu/courses/18-745-lie-groups-and-lie-algebras-i-fall-2020/mit18_745_f20_lec18.pdf)
- [DSV, arXiv:hep-th/0205185](https://arxiv.org/abs/hep-th/0205185)
- [Simon 1983, numdam](https://numdam.org/item/AIHPA_1983__38_3_295_0)
- [Simon erratum 1984](https://www.numdam.org/item/AIHPA_1984__40_2_224_0/)
- [Helffer–Sjöstrand VI, numdam](https://www.numdam.org/item/AIHPA_1987__46_4_353_0)
- [Whitehead's lemma page](https://en.wikipedia.org/wiki/Whitehead%27s_lemma_(Lie_algebra))
- [Witten 1982, Princeton record](https://collaborate.princeton.edu/en/publications/supersymmetry-and-morse-theory/)
- [Kim–Klose–Plefka, hep-th/0306054](https://arxiv.org/abs/hep-th/0306054)
- [Lucini–Teper, hep-lat/0103027](https://arxiv.org/abs/hep-lat/0103027)
- [AMMPV, hep-th/0310285](https://arxiv.org/abs/hep-th/0310285)
