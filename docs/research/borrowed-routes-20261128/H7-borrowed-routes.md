# H7, every compact simple group: what I can borrow, and what it closes

28 November 2026 (written overnight). Research note. **I edit no existing file. Line 24 stays [X = H7].** Below is
one sub-statement I can close by citation plus deduction, and one conditional corollary. Both are reduced-model
statements.

Revised 28 Nov after the referee pass: §5(5) no longer attributes "exactly one spin-1 copy" to Borel–de Siebenthal (it is the elementary root-string fact proved in `H7-closure.md` Step 7), and Corollary B's limit is flagged as min(ω_C, 2√t), so ω_C only for t ≥ 1/3.

---

## 1. The precise target

H7 (line 24; `closed-set-20261127/H7-every-group.md` §1) is the whole theory for every compact simple 𝔤, read
through the dictionary D_G. What is already closed for every 𝔤 (I do not redo it):

- lines 1–3 [PC] and the electric note 2C_min(G)g²/a;
- the cubic-quartic bound |C| ≤ c_𝔤 Q^{3/4} (Austing–Wheater) and the compact resolvent of the reduced model;
- the strict vacuum floor E₀ > nS/2;
- the origin splitting (charge ±2 pair exact at 2/√t, charge 0 moves by −2κ(1+1/t)g²);
- path transport through su(2) embeddings;
- the horizontal barrier and charge tail.

**Open pieces I work on here.**

- **(T1)** H7 Step 4 says plainly: "The 2h^∨ − 4 doublet modes (j = ½) need their own diagonalisation, which I have
  not done."
- **(T2)** H7 §5 failure mode 2: centraliser or doublet modes could reorder the carriers at s = 1, so that H8(G)
  "fails as worded".
- **(T3)** Bee's small-coupling deduction of H8 is written for SU(2) only
  (`h8-small-coupling-switch-deduction-2026-11-27.md`). Its input (M) uses "the zeros of V⁰ are exactly
  {0} ∪ q_MC·SO(3)". For general 𝔤 that list is wrong: every su(2) embedding gives a vacuum. The input
  therefore has to be re-proved before the deduction can be transported.

Notation (`h8-vacuum-floor-proof-2026-11-27.md` §1):

- q = (X₁, X₂, X₃) ∈ 𝔤³, with ω = (√t, 1/√t, 1/√t) and S = √t + 2/√t;
- C(q) = ⟨X₁, [X₂, X₃]⟩ and W = ½ Σ_k ω_k‖X_k‖² − g C;
- at s = 1, V = ½‖∇W‖², with ∇_k W = ω_k X_k − g Y_k and Y_k = ½ ε_klm [X_l, X_m].

The superpotential identity itself holds verbatim for every 𝔤. It is already used by the vacuum-floor proof, so
it is not a new closure.

---

## 2. What I found in the literature (every citation checked tonight)

### A. Dasgupta, Sheikh-Jabbari, Van Raamsdonk: the spectrum around every su(2)-representation vacuum

*K. Dasgupta, M. M. Sheikh-Jabbari, M. Van Raamsdonk, "Matrix perturbation theory for M-theory on a PP-wave",
JHEP 0205 (2002) 056, arXiv:hep-th/0205185.* I read it from the arXiv PDF.

**What it says.**

- §4.2: the bosonic potential of the BMN matrix model is a sum of squares. Its SO(3) part is
  ½(μ/3R)²·Tr(X^i + iε_ijk X^j X^k ·(3R/μ))², written there as "(μ/3R) X^i + iε_ijk X^j X^k" squared.
- Its supersymmetric vacua are X^i = (μ/3R) J^i, "where J^i form a representation of the SU(2) algebra". So "the
  classical supersymmetric solutions are labeled by the N dimensional representations of SU(2)".
- §5.2 (Bosons in the SO(3) directions): with V_Y = ½(1/3)²(Y^i + iε_ijk[J^j, Y^k])², the eigenvalue problem
  Y^i + iε_ijk[J^j, Y^k] = λY^i reduces, for each spin-j matrix harmonic, to **λ(λ + j)(λ − (j + 1)) = 0**.
- The λ = 0 states are Y^i = [J^i, Y_jm], "fluctuations in the directions of the gauge orbit". The masses are
  M = λμ/3.
- §5.3 treats a general reducible representation by the same method, block by block.

**Hypotheses.** Gauge algebra u(N), acting on N×N matrices. A round sphere, so a single mass μ for all three SO(3)
directions. The extra SO(6) scalars and the fermions decouple at quadratic order.

**How it aligns with my framing.** At t = 1 (round S³) my unsourced reduced model is exactly the SO(3) bosonic
sector of this model, with my W in the role of their superpotential:

- V = ½|∇W|² is their perfect square;
- my vacua (zeros of ∇W) are their su(2)-representation vacua;
- my λ-line is their membrane radius.

So their spectrum formula *is* my Hessian of W at a vacuum, at t = 1.

**What transfers.** The eigenvalue problem is posed separately on each spin-j isotypic component. So it does not
care whether the spin-j pieces come from Mat_N under conjugation or from 𝔤 under ad∘φ. It transfers verbatim to
every compact simple 𝔤 once 𝔤 is decomposed under the embedding.

**What does not transfer.** The Berger squashing t ≠ 1 breaks the frame SO(3) to O(2), so their formula fails for
t ≠ 1. That needs a new step, which I give in §5: a congruence plus Sylvester's law of inertia.

### B. Kim, Klose, Plefka: homogeneous modes on S³ give this matrix model

*N. Kim, T. Klose, J. Plefka, "Plane-wave matrix theory from N = 4 super Yang–Mills on R×S³", Nucl. Phys. B 671
(2003) 359–382, arXiv:hep-th/0306054.*

**What it says.** Plane-wave matrix theory "arises from a consistent truncation" of N = 4 SYM on a round R×S³ to
its lowest modes.

**How it aligns.** This is my small-room move: keep the homogeneous modes on S³, get a matrix model with a
Chern–Simons-type superpotential. Someone else made it in the supersymmetric theory, independently of me. It
supports the believability of the slow-mode reduction itself.

**What does not transfer.** The setting is supersymmetric, round, and U(N) with scalars and fermions. Their
statements about anomalous dimensions are not statements about pure Yang–Mills spectra. I borrow only the fact
that the reduction exists and its shape.

### C. Whitehead's first lemma: why every vacuum orbit is non-degenerate

*N. Jacobson, Lie Algebras (Interscience 1962; Dover reprint 1979), Ch. III §7, Lemma 3.* This chapter and lemma
are the attribution given by the standard reference page for the lemma, which states it as follows.

**Statement.** Let 𝔰 be a finite-dimensional semisimple Lie algebra over a field of characteristic 0, and M a
finite-dimensional 𝔰-module. If f: 𝔰 → M is linear with f([x, y]) = x·f(y) − y·f(x), then there is v ∈ M with
f(x) = x·v for all x. Equivalently, H¹(𝔰, M) = 0.

**Hypotheses.** 𝔰 = su(2) (real, semisimple, characteristic 0) and M = 𝔤 via ad∘φ (finite-dimensional). These
match exactly, and no transfer step is needed.

The same rigidity idea is behind Nijenhuis and Richardson's deformation theory of homomorphisms (Bull. Amer. Math.
Soc., 1967). I only confirmed the existence and subject of that note, not its exact page range, so I use Whitehead
alone.

### D. Witten: supersymmetry and Morse theory (an idea, not a theorem I need)

*E. Witten, "Supersymmetry and Morse theory", J. Differential Geom. 17 (1982) 661–692, doi:10.4310/jdg/1214437492.*

**The idea I borrow.** For H = ½(−Δ + |∇W|²), the low states sit at the critical points of W, organised by the
Morse index of W there. The harmonic ground energy at a critical point is ½Σ|μ_i| over the Hessian eigenvalues of
W. The constant ½ΔW = ½ tr Hess W is what the e^{−W} factorization subtracts.

**How it aligns.** This is exactly the structure of Bee's floor. ΔW = nS is constant, because the cubic C is
harmonic: its pure second derivatives vanish (vacuum-floor proof §3). The offset of a well above the floor is
therefore ½(Σ|μ| − Σμ) = **Σ_{μ<0} |μ|, the sum of the negative Hessian eigenvalues of W**.

**What it means here.**

- The competitor at s = 1 is a Morse-index direction of the Chern–Simons superpotential.
- Witten's paper is about the supersymmetric operator on forms. I use only the harmonic-oscillator arithmetic,
  which is elementary, so the citation is for the idea.

### E. Simon 1983: the semiclassical input

*B. Simon, "Semiclassical analysis of low lying eigenvalues. I. Non-degenerate minima: asymptotic expansions",
Ann. Inst. H. Poincaré Phys. Théor. 38 (1983) 295–308.*

**Hypotheses.** The potential h ≥ 0 has finitely many minima, each of them a non-degenerate point.

**Correction.** An erratum, AIHPA 40 (1984) 224, says Theorem 5.1 needs an extra same-parity assumption in the
degenerate case. I use only the leading-order harmonic limit, not Theorem 5.1.

**What does not transfer.** My minima are group orbits, not points. The orbit case is input (S) of Bee's SU(2)
deduction (IMS localization in slice coordinates), and it stays a cited-but-not-re-proved input here too.

### F. Lattice evidence for every group (evidence, not proof)

- **SU(N), the scalar is lightest.** *B. Lucini, M. Teper, "SU(N) gauge theories in four dimensions: exploring the
  approach to N = ∞", JHEP 0106 (2001) 050, arXiv:hep-lat/0103027.* For N = 2, 3, 4, 5 the lightest computed
  glueball is the 0⁺⁺ scalar, below the 2⁺⁺. The one exception is one undersized SU(4) lattice, which they exclude.
  The N = ∞ extrapolation is m_{0⁺⁺}/√σ = 3.37(15). This supports H8(G): the charge-0 carrier is lightest.
- **Larger N.** *A. Athenodorou, M. Teper, "SU(N) gauge theories in 3+1 dimensions: glueball spectrum, string
  tensions and topology", JHEP 12 (2021) 082, arXiv:2106.00364*, covers N = 2 to 12, continuum-extrapolated. I did
  not confirm the ordering statement from the abstract, so I cite it only for coverage.
  *B. Lucini, M. Teper, U. Wenger, JHEP 0406 (2004) 012, arXiv:hep-lat/0404008* computes the lightest glueball
  masses for N = 2 to 8 with improved operators.
- **G₂, no centre.**
  - *K. Holland, P. Minkowski, M. Pepe, U.-J. Wiese, "Exceptional confinement in G(2) gauge theory", Nucl. Phys. B
    668 (2003) 207–236, arXiv:hep-lat/0302023*: "G(2) theories confine even without a center", while the 7-string
    breaks by gluon screening.
  - *M. Pepe, U.-J. Wiese, "Exceptional deconfinement in G(2) gauge theory", Nucl. Phys. B 768 (2007) 21–37,
    arXiv:hep-lat/0610076*: a first-order deconfinement transition without centre symmetry.
  - **How it aligns.** My route to a gap never uses the centre (H7 §5 item 4). This matches what is seen for G₂.
- **Not found.** I did not find a dedicated pure-G₂ glueball spectrum paper. Wellegehausen's G₂ work
  (arXiv:1312.5579) is G₂-QCD with quarks, not pure gauge, so I do not cite it for glueballs.

### G. Small-volume effective Hamiltonians (method aligns, setting does not)

- *M. Lüscher, "Some analytic results concerning the mass spectrum of Yang–Mills gauge theories on a torus", Nucl.
  Phys. B 219 (1983) 233–261.* This is the Born–Oppenheimer effective Hamiltonian for zero-momentum modes.
- *P. van Baal, "QCD in a finite volume", arXiv:hep-ph/0008206* (Ioffe Festschrift, ed. M. Shifman). This review
  includes the spherical volume.

**How it aligns.** The method is the template for H2/M3 (layer 8 of the framing note).

**What does not transfer.** On T³ the classical vacuum valley is the flat abelian torus of constant Cartan modes.
That is a continuum of degenerate vacua for every G. On S³ the vacua are finitely many isolated orbits (Theorem A
below). So the torus SU(3) results (Weisz–Ziemann, Vohwinkel) cannot be imported as spectra. I could not verify
the Vohwinkel bibliographic data tonight, so I give no citation for it.

### H. Free Yang–Mills on S³ for any gauge group

*O. Aharony, J. Marsano, S. Minwalla, K. Papadodimas, M. Van Raamsdonk, "The Hagedorn/deconfinement phase
transition in weakly coupled large N gauge theories", Adv. Theor. Math. Phys. 8 (2004) 603–696,
arXiv:hep-th/0310285.*

**What it says.** At zero coupling on S³ the partition function reduces to an integral over a single holonomy, so
the singlet spectrum is a group-theory count.

**How it aligns.** This is the origin-well counting of H7 Step 6. The adjoint has no invariant vector, so the
lowest singlets have two quanta, with the same degree-2 invariants ⟨X_k, X_l⟩ for every 𝔤.

**What does not transfer.** They treat the round sphere and do not see the second family of wells. The
two-well (two-vacuum) structure is mine.

---

## 3. Alignment summary

| Piece of my picture | Transfers to every 𝔤 verbatim? | Borrowed support |
|---|---|---|
| V = ½\|∇W\|², W = ½Σω‖X‖² − gC | yes (already used) | DSV §4.2 perfect square; KKP truncation |
| Vacua = zeros of ∇W | **no: one vacuum orbit per conjugacy class of su(2) → 𝔤**, not just 0 and q_MC | DSV §4.2 (vacua = su(2) representations) |
| Vacuum orbits non-degenerate | yes, new proof | Whitehead's lemma |
| Hessian spectrum at a vacuum, t = 1 | yes, per spin component | DSV §5.2, λ ∈ {0, −j, j+1} |
| Same, t ≠ 1 | needs a new step (done below) | Sylvester inertia (elementary) |
| Competitor = MC-well ground, offset ω_C | **yes, exactly, for every 𝔤** (below) | Witten's Morse-index idea |
| Helike = charge-2 Hessian eigenvector | yes (H7 Step 4b, already) | — |
| Two-carrier story at the sourced controls s = 1.15 | **no** (H7 Step 5: centraliser at 0.392) | — |
| Centre / Polyakov-loop intuition | not used, which is good for G₂, F₄, E₈ | Holland et al.; Pepe–Wiese |
| Lowest glueball = 0⁺⁺ | evidence | Lucini–Teper (N = 2..5, N → ∞) |

---

## 4. A reduced route for H7 using the borrowed pieces

1. **Lines 1–6 and the reduced model's well-posedness (already closed for every 𝔤).**
2. **Classical vacuum structure for every 𝔤 (Theorem A, closed here).** Finitely many vacuum orbits, one per
   conjugacy class of su(2) → 𝔤. Each is Morse–Bott non-degenerate, with an explicit Morse index of W and an
   explicit zero-point offset.
3. **H8(G) at small coupling, every t > 0 (Corollary B, conditional on (S) only).** This removes failure mode 2 of
   H7 §5 at small g in the reduced model. Bee's SU(2) small-coupling result becomes a statement for every 𝔤 with
   the same named input.
4. **H1(G) and the switch.** These do not transfer: at the controls the centraliser is soft. The route to H3 starts
   at s = 1 (line 20), so step 3 is the piece the route actually needs.
5. **H2(G), H3(G), H4′(G), H6(G).** These remain the same open problems as for SU(2), with b₀ → 11h^∨/3. The
   borrowed work offers no shortcut for them: no continuum result is known for any exceptional group.

   The extra label C for Out(𝔤) ≠ 1 (line 20) is still needed. Corollary B is insensitive to it: the competitor is
   the MC-well ground, a Gaussian, so it is invariant under every symmetry fixing the orbit. That includes C
   whenever C preserves the long-root class, because C is then realised by an automorphism mapping the class to
   itself.

---

## 5. The closable sub-statement

### Theorem A (vacua of the unsourced reduced model, every 𝔤, every t > 0)

Fix a compact simple 𝔤 with a positive Ad-invariant inner product, t > 0 and g > 0. Write a_k = √(ω_l ω_m)/g for
{k, l, m} = {1, 2, 3}, so (a₁, a₂, a₃) = (1/(g√t), 1/g, 1/g).

1. **(Classification.)** V(q) = 0 iff X_k = a_k Z_k (k = 1, 2, 3), where Z_k = φ(e_k) for a Lie-algebra
   homomorphism φ: su(2) → 𝔤 and [e₁, e₂] = e₃ cyclically. Here φ = 0 gives q = 0, and the long-root φ gives Bee's
   q_MC.
2. **(Finiteness.)** The zero set is a finite union of Ad(G)-orbits.
3. **(Morse–Bott.)** At every zero, ker Hess V = ker Hess W = the tangent space of the orbit.
4. **(Index and offset.)** Decompose 𝔤_ℂ under ad∘φ into spins l, with multiplicities m_l(φ). Then Hess W has
   - 2l+3 positive, 2l+1 zero and 2l−1 negative eigenvalues on each spin-l copy with l ≥ 1;
   - 4 positive and 2 zero on each spin-½ copy;
   - 3 positive on each spin-0 copy;

   **for every t > 0**. The zero-point offset of the well above the floor nS/2 is

   Off(φ, t) = Σ_l m_l(φ) ε_l(t),  ε_l(t) = Σ_{μ<0} |μ| on one spin-l copy.

   Here ε₀ = ε_½ ≡ 0, ε_l > 0 for l ≥ 1, and ε₁(t) = ω_C(t) = ½(√(t+8) − √t).
5. **(Long-root well.)** For the long-root (Dynkin-index-1) embedding, 𝔤_ℂ = 𝟑 ⊕ (2h^∨−4)·𝟐 ⊕ 𝔨_ℂ.
   - The well has Morse index 1.
   - Off = ω_C(t) exactly, **the same as SU(2), for every 𝔤**.
   - Every nonzero φ has Off(φ, t) ≥ ω_C(t).
   - Per complex doublet, the doublet normal frequencies are 3/(2√t) (twice) and (2t+1)/(2√t) (twice).

   This closes the diagonalisation in (T1) at s = 1.

#### Proof

**(1) Classification.** At s = 1, V = 0 iff ∇W = 0, i.e. ω_k X_k = g[X_l, X_m] cyclically. Put X_k = a_k Z_k.
Since g a_l a_m = ω_k a_k (because a_l² a_m² g² = ω_k² ω_l ω_m / g² = ω_k² a_k²), the equation becomes
[Z_l, Z_m] = Z_k. Those are the su(2) relations, so φ(e_k) := Z_k is a homomorphism. Conversely every
homomorphism gives a zero. (This is the computation of Theorem 9 in the framing note, now done without assuming q
is diagonal.)

**(3) Morse–Bott.** At a zero, ∂_a∂_b V = Σ_i ∂_a∂_i W ∂_b∂_i W, so Hess V = (Hess W)² and the two kernels
agree. Linearize ∇W at X = aZ with δX_k = a_k δZ_k:

δ(∇_k W) = ω_k a_k (δZ_k − ε_klm [Z_l, δZ_m]).

Vanishing for all k says exactly that c(e_k) := δZ_k satisfies c([x, y]) = [φx, c y] − [φy, c x], i.e. c is a
1-cocycle of su(2) with values in (𝔤, ad∘φ). By Whitehead's lemma (§2C), c(x) = [φx, Λ] for some Λ ∈ 𝔤. So
δX_k = [X_k, Λ], which is tangent to the Ad(G)-orbit. The converse holds by G-invariance.

**(2) Finiteness.**

- *Each orbit is isolated in the zero set.* By (3), the Hessian of V is positive definite on a normal slice along
  the compact orbit. So V ≥ c·dist² near the orbit, and no other zero comes close.
- *The zero set is bounded.* The inner product is a multiple of −tr_ρ for the faithful representation ρ = ad (the
  invariant inner product on a simple algebra is unique up to scale). So ‖Z_k‖² is a multiple of the sum of
  j(j+1)(2j+1)/3 over the constituents of ρ∘φ, which is bounded in terms of dim 𝔤.
- *Conclusion.* The zero set is compact and each orbit is open in it, so there are finitely many orbits.

I do not need Dynkin's or Kostant's classification. It only adds that the orbits correspond to nilpotent orbits of
𝔤_ℂ. For SU(N) they correspond to the partitions of N, matching DSV's count.

**(4) Index and offset.** Let D = diag(√ω_k) act on the frame index, and define (Lδ)_k = ε_klm [Z_l, δ_m]. L is
symmetric for the product inner product, since ad Z_l is antisymmetric and ε is antisymmetric in (k, m). The
linearization above gives

Hess W = D(1 − L)D,

and L does not depend on t. By Sylvester's law of inertia, Hess W has the inertia of 1 − L for every t > 0. Both
operators preserve each isotypic component, acting as (one copy) ⊗ 1 on the multiplicity space. On a spin-l copy,
with Z_l = −iJ_l and frame spin-1 matrices (S_l)_km = iε_klm, one gets L = −S·J. So the eigenvalues of 1 − L are
1 + S·J:

- l + 1 for total spin J = l+1, with multiplicity 2l+3;
- 0 for J = l, with multiplicity 2l+1;
- −l for J = l−1, with multiplicity 2l−1.

This is exactly DSV's λ ∈ {j+1, 0, −j} (§2A), now on 𝔤 instead of Mat_N.

For the offset:

- In harmonic order the well's ground energy is ½Σ|μ| over its normal modes.
- tr Hess W = tr(D²(1 − L)) = nS, because L has zero frame-diagonal blocks. That is the floor E_* = nS/2.
- So the offset is ½(Σ|μ| − Σμ) = Σ_{μ<0} |μ|.
- Spin 0 and spin ½ have no negative eigenvalues, so ε₀ = ε_½ = 0, and ε_l > 0 for l ≥ 1.

**ε₁ = ω_C.** In the spin-1 copy, take the frame-diagonal directions f₁ = (Z₁, 0, 0) and
f₂ = (0, Z₂, Z₃)/√2. There L = [[0, √2], [√2, 1]], so

Hess W = [[√t, −√2], [−√2, 0]],

with eigenvalues (√t ± √(t+8))/2. The single negative one is −ω_C. The 2l−1 = 1 count says it is the only one.

**(5) Long-root well.**

- The long-root grading has exactly one spin-1 copy, namely ι(su(2)) (H7 §3; this is the elementary root-string fact |⟨α, θ^∨⟩| ≤ 1 for α ≠ ±θ, re-proved in `H7-closure.md` Step 7, not Borel–de Siebenthal). The rest is
  doublets and singlets, so Off = ε₁ = ω_C and the Morse index is 1.
- Every nonzero φ has its own image as a spin-1 copy, so Off(φ) ≥ ε₁.
- The doublet characteristic polynomial, computed symbolically with u = √t, factors as
  λ²(2uλ − 3)²(2uλ − 2u² − 1)². This gives the stated frequencies. ∎

#### Checks run tonight

These are small scripts, run for the record only; they are not part of the proof.

- **Universal spin-l blocks**, for l ∈ {0, ½, 1, 3/2, 2} and t ∈ {0.3, 1, 3, 10}:
  - the inertia is as stated;
  - tr = (2l+1)S;
  - ε₁ = ω_C to 10⁻⁶;
  - ε_{3/2}(3) = 2.4313 and ε₂(3) = 4.9329.
- **Independent finite-difference Hessian of the full V on su(3)** at the long-root q_MC (t = 3, g = 0.7):
  - V(q_MC) = 2·10⁻³²;
  - 7 zero modes, matching orbit dimension 8 − 1;
  - frequencies {0.5774², 0.7923, 0.8660⁴, 1.1547², 1.7321, 2.0207⁴, 2.3094², 2.5243};
  - offset 0.792287 against ω_C = 0.792287.
- **The SU(3) principal (spin-1) embedding** has index 1 + 3 = 4 and offset ε₁ + ε₂ = 5.725 at t = 3.

### Corollary B (H8(G) at small coupling, at s = 1, for every 𝔤 and every t > 0; conditional on (S))

Assume (S), Bee's semiclassical input: Simon's harmonic limit for wells that are group orbits, applied per symmetry
sector on colour singlets. Then for every compact simple 𝔤 and every t > 0 there is g₀(𝔤, t) > 0 such that for
0 < g < g₀:

E_{1,0} < inf_{m≠0} E_{0,m}.

That is, the lowest excitation of the unsourced reduced model is charge 0. Moreover E_{1,0} − E_{0,0} → ω_C(t),
carried by the ground state of the long-root well: the competitor, for every 𝔤. *(Referee pass, 28 Nov: this limit
holds only for t ≥ 1/3. In general the limit is min(ω_C(t), 2√t); for t < 1/3 the origin's X₁-breathing level 2√t is
lower. See `H7-closure.md` §6 and §8. The charge-0-lowest ordering holds for every t > 0.)*

#### Proof

(S) applies:

- by Theorem A(2, 3), the zero set consists of finitely many orbits, each transversally non-degenerate;
- V ≥ (κ/2)|q|² − C grows at infinity, by the cubic-quartic closure;
- charge, the rotation of (X₂, X₃), commutes with H and maps each vacuum orbit to itself, since frame rotations of
  a homomorphism are conjugate under exp(θZ₁).

The harmonic limits, measured from E_{0,0}:

- **The origin well is the unique global ground.** Its offset is 0, and Off(φ) ≥ ω_C > 0 for every φ ≠ 0
  (Theorem A4).
- **The origin's excitations.** The adjoint has no invariants, so the lowest singlets have two quanta. Charge 0 and
  ±2 sit at 2/√t, charge ±1 at √t + 1/√t (H7 Step 6, independent of 𝔤).
- **The ground state of each well φ ≠ 0** is the transverse Gaussian. It is invariant under the stabiliser and under
  the charge rotation, both of which act orthogonally and commute with the Hessian. So it is a charge-0 singlet at
  Off(φ) ≥ ω_C, with equality for the long-root class.
- **Every charged state of a well φ ≠ 0** lies at Off(φ) + (a positive harmonic excitation), which is > ω_C.

Hence E_{1,0} − E_{0,0} → min(ω_C, 2/√t) = ω_C, because ω_C < 2/√t for every t > 0: that inequality is
√(t² + 8t) < t + 4. Also inf_{m≠0} E_{0,m} − E_{0,0} tends to the minimum of finitely many numbers, each > ω_C,
using ω_C < 4/√8 < 2 ≤ √t + 1/√t for the origin's charge-±1 level. The strict gap between the two limits gives g₀. ∎

#### What Corollary B does and does not say

- **It closes (T2) and (T3) at small coupling in the reduced model.** The extra colour modes cannot reorder the s = 1
  carriers there: centraliser and doublet modes have zero offset, and the ground states of other embeddings cost at
  least ω_C.
- **It extends Bee's SU(2) clause 1 of the small-coupling deduction to every 𝔤 and every t > 0**, with the same
  single input (S).
- **It does not give the switch.** Clause 2 of the SU(2) note fails for 𝔤 ≠ su(2): at s = 1.15 the centraliser
  modes are soft (H7 Step 4c). The switch is not part of the s = 1 route.
- **It does not give the registered coupling, the physical room (H2b), continuum statements, or any uniformity in
  𝔤.** g₀ depends on 𝔤 and t.
- **Status.** Theorem A is closed by citation (Whitehead) plus deduction. Corollary B is [Cd] on (S). H7 stays [X].

---

## 6. Honest negatives

- The torus small-volume programme (Lüscher; van Baal; the SU(3) torus spectroscopy) does not transfer as
  spectra, because its vacuum valley is flat. Only the Born–Oppenheimer method transfers.
- Nothing I found gives a rigorous continuum or gap result for any group beyond lines 1–3. H2–H6 for general G are
  exactly as open as for SU(2).
- DSV is the closest match, but only at t = 1. The Berger extension (Sylvester congruence) is mine and is
  elementary.
- Lattice glueball ordering is evidence for H8(G) at physical coupling, not proof.
