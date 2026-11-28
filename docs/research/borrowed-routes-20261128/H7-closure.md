# H7, every compact simple group: the vacua of the unsourced room, closed

28 November 2026 (overnight). Proof note. **I edit no existing file. Line 24 stays [X = H7].**

**Status.**

- **Theorem A: CLOSED BY CITATION PLUS DEDUCTION** for every compact simple 𝔤, every t > 0 and every g > 0, in the
  unsourced (s = 1) reduced model. The one cited theorem with real content is Whitehead's first lemma, and its
  hypotheses match exactly. Everything else is proved here or is elementary linear algebra.
- **Corollary B: [Cd] on the single named input (S)**, the same input as in my SU(2) small-coupling deduction.
- **One correction to the research note** (`H7-borrowed-routes.md` §5, Corollary B): the limit
  E_{1,0} − E_{0,0} → ω_C(t) holds for t ≥ 1/3 only. For t < 1/3 the origin's X₁-breathing level 2√t lies lower.
  The carrier ordering itself (charge 0 lowest) holds for every t > 0. Details in §6 and §8.

---

## 1. Setting

I fix a compact simple real Lie algebra 𝔤 of dimension n with a positive Ad-invariant inner product ⟨·,·⟩. I write G
for a compact connected Lie group with Lie algebra 𝔤; Ad(G) is the same for every choice. I fix t > 0 and g > 0.

- **Configuration.** q = (X₁, X₂, X₃) ∈ 𝔤³ = ℝ^{3n}, with the product inner product and Lebesgue measure.
- **Frequencies.** ω = (ω₁, ω₂, ω₃) = (√t, 1/√t, 1/√t) and S = ω₁ + ω₂ + ω₃ = √t + 2/√t.
- **Cubic.** 𝒞(q) = ⟨X₁, [X₂, X₃]⟩.
- **Superpotential.** W(q) = ½ Σ_k ω_k ‖X_k‖² − g 𝒞(q).
- **Potential at s = 1.** V = ½‖∇W‖². This is the unsourced reduced potential: H7 Step 3 (9) with src = 0, and
  `h8-vacuum-floor-proof-2026-11-27.md` §1.
- **Operator.** H_g = −½Δ + V, the Friedrichs realisation. By the vacuum-floor note §2 it has compact resolvent for
  every t > 0, using V ≥ (κ/2)‖q‖² − 27S⁴c_𝔤⁴/(32g²) with κ = min(t, 1/t).
- **Symmetries.**
  - Colour: Ad(G) acts diagonally on 𝔤³.
  - Charge: R_θ rotates (X₂, X₃) ↦ (cos θ X₂ − sin θ X₃, sin θ X₂ + cos θ X₃).
  - Both preserve W, because ω₂ = ω₃ and 𝒞 is frame-SO(3) invariant. They commute with each other and with H_g.
- **Spectral notation.** E_{k,m} is the k-th eigenvalue (k = 0, 1, …, with multiplicity) of H_g on colour-singlet
  functions of charge m.
- **Brackets.** Y_k = ½ ε_klm [X_l, X_m], so Y₁ = [X₂, X₃] cyclically.
- **Basis of su(2).** e₁, e₂, e₃ with [e₁, e₂] = e₃ cyclically.
- **Spin decomposition.** For a Lie-algebra homomorphism φ: su(2) → 𝔤, I write Z_k = φ(e_k). Then ad∘φ is a real
  representation of su(2) on 𝔤. Its complexification decomposes as 𝔤_ℂ = ⊕_l m_l(φ)·V_l (spins l ∈ ½ℤ_{≥0}).

**Lemma 0 (gradient and Hessian).**

∇_k W = ω_k X_k − g Y_k,

(Hess W · δ)_k = ω_k δ_k − g ε_klm [X_l, δ_m].

*Proof.* ∂/∂X₁ of 𝒞 is [X₂, X₃]. For X₂, Ad-invariance gives ⟨X₁, [δ, X₃]⟩ = ⟨[X₃, X₁], δ⟩, so the derivative is
[X₃, X₁]. X₃ is the same, cyclically. Then δY_k = ε_klm [X_l, δ_m]. ∎

Two consequences of Lemma 0:

- 𝒞 is linear in each X_k, so the diagonal blocks of Hess 𝒞 vanish. Hence **tr Hess W = nS at every q**, and
  ΔW = nS. This is the constant in the vacuum-floor note §3.
- At a zero of ∇W, ∂_a∂_b V = Σ_i ∂_a∂_i W ∂_b∂_i W + Σ_i ∂_i W ∂_a∂_b∂_i W. The second sum vanishes, so
  **Hess V = (Hess W)²**.

---

## 2. Statement

**Theorem A (vacua of the unsourced reduced model).** For every compact simple 𝔤 with a positive Ad-invariant inner
product, every t > 0 and every g > 0, put

a = (a₁, a₂, a₃) = (1/(g√t), 1/g, 1/g), i.e. a_k = √(ω_l ω_m)/g for {k, l, m} = {1, 2, 3}.

**(A1) Classification.** V(q) = 0 if and only if X_k = a_k φ(e_k) (k = 1, 2, 3) for some Lie-algebra homomorphism
φ: su(2) → 𝔤. The map φ ↦ q_φ is a bijection from Hom(su(2), 𝔤) onto the zero set 𝒵. φ = 0 gives q = 0, and the
long-root φ gives my q_MC^ι (H7 Step 3 (8)).

**(A2) Finiteness.** 𝒵 is compact and is a finite union of Ad(G)-orbits.

**(A3) Morse–Bott.** At every q ∈ 𝒵:

ker Hess V(q) = ker Hess W(q) = T_q(Ad(G)·q),

and 𝒵 coincides with the orbit Ad(G)·q in a neighbourhood of that orbit.

**(A4) Index and offset.** Let q = q_φ. Hess W(q) does not depend on g. On C³ ⊗ U, for each irreducible summand
U ≅ V_l of 𝔤_ℂ, for every t > 0, it has the following inertia (#positive, #zero, #negative):

| spin | inertia |
|---|---|
| l = 0 | (3, 0, 0) |
| l = ½ | (4, 2, 0) |
| l ≥ 1 | (2l + 3, 2l + 1, 2l − 1) |

Define the well's zero-point offset as

Off(φ, t) := ½ Σ_{μ ≠ 0} |μ| − nS/2,

with μ running over the eigenvalues of Hess W(q_φ). The normal frequencies of V at the well are exactly the |μ|,
μ ≠ 0, so Off is the harmonic ground energy of the well measured from the floor nS/2. Then

Off(φ, t) = Σ_{μ < 0} |μ| = Σ_l m_l(φ) ε_l(t),

where ε_l(t) ≥ 0 is the sum of |negative eigenvalues| on one spin-l copy. Moreover:

- ε₀ = ε_½ = 0;
- ε_l > 0 for l ≥ 1;
- **ε₁(t) = ω_C(t) := ½(√(t+8) − √t)** exactly.

**(A5) Long-root well and lower bound.**

- For every nonzero φ, Off(φ, t) ≥ ω_C(t).
- For the long-root homomorphism φ_θ (§5, Step 7), m₁ = 1 and m_l = 0 for l ≥ 3/2. So the long-root well has Morse
  index 1 (as a critical orbit of W) and Off(φ_θ, t) = ω_C(t), **for every 𝔤 and every t > 0**.
- On each complex doublet, Hess W has eigenvalues 0 (twice), 3/(2√t) (twice) and (2t+1)/(2√t) (twice). So the
  doublet normal frequencies are 3/(2√t) and (2t+1)/(2√t), each twice.

**Corollary B (H8(G) at small coupling, s = 1; conditional on (S)).** Assume input (S) of §4. Then for every compact
simple 𝔤 and every t > 0 there is g₀(𝔤, t) > 0 such that, for all 0 < g < g₀,

E_{1,0} < inf_{m ≠ 0} E_{0,m}.

Moreover, as g → 0:

- E_{0,0} − nS/2 → 0;
- E_{1,0} − E_{0,0} → ν₁(t) := min(ω_C(t), 2√t);
- ν₁(t) = ω_C(t) if and only if t ≥ 1/3. For t > 1/3 the level ω_C is supplied by the ground state of the long-root
  well: the competitor, for every 𝔤.

---

## 3. Borrowed results, with their exact hypotheses

### B1. Whitehead's first lemma (the one cited theorem with real content)

**Statement (MIT OCW 18.745, Lie Groups and Lie Algebras I, Fall 2020, Lecture 18 notes, Theorem 18.4).** "If g is
semisimple in characteristic zero then for every finite dimensional representation V of g, H¹(g, V) = 0."

The same notes define a 1-cocycle as a linear a: g → E with "a([x, y]) = x∘a(y) − y∘a(x)", and a 1-coboundary as
a(x) = x·v.

**Classical source.** N. Jacobson, *Lie Algebras* (Interscience 1962; Dover reprint 1979), Ch. III §7, Lemma 3. This
is the location the standard reference page gives for the proof. I did not see Jacobson's own page, so for the
wording I rely on the MIT statement above.

**Hypotheses, checked against my use.**

| Hypothesis | My case |
|---|---|
| semisimple Lie algebra | su(2), a real semisimple Lie algebra |
| characteristic 0 | the field is ℝ |
| finite-dimensional module | 𝔤 with x·v = [φ(x), v], i.e. ad∘φ, a representation because φ is a homomorphism |

They match exactly. No transfer step is needed.

### B2. Sylvester's law of inertia

**Statement.** If A is Hermitian and S is nonsingular, then S*AS has the same numbers of positive, zero and negative
eigenvalues as A. (J. J. Sylvester, 1852; stated as Theorem 4.5.8 of R. A. Horn and C. R. Johnson, *Matrix
Analysis*, Cambridge University Press, 1985. I confirmed the theorem number through papers that cite it, not from
the book itself.)

**Hypotheses.** A finite-dimensional Hermitian A and an invertible S. Both hold in Step 4.

### B3. Dasgupta–Sheikh-Jabbari–Van Raamsdonk: the precedent at t = 1 (corroboration, not a logical input)

K. Dasgupta, M. M. Sheikh-Jabbari, M. Van Raamsdonk, "Matrix perturbation theory for M-theory on a PP-wave", JHEP
0205 (2002) 056, arXiv:hep-th/0205185. I read §§4.2, 5.2 and 5.3 in the arXiv PDF.

- **§4.2.** The bosonic potential is a sum of squares. Its SO(3) term vanishes iff X^i = (μ/3R) J^i, "where J^i form a
  representation of the SU(2) algebra, [J^i, J^j] = iε^{ijk}J^k". So "the classical supersymmetric solutions are
  labeled by the N dimensional representations of SU(2)".
- **§5.2** (irreducible vacuum, "Bosons in the SO(3) directions").
  - Quadratic potential: V_Y = ½(⅓)²(Y^i + iε^{ijk}[J^j, Y^k])².
  - Eigenvalue problem: Y^i + iε^{ijk}[J^j, Y^k] = λY^i, solved on each spin-j matrix harmonic.
  - Result: "λ(λ + j)(λ − (j + 1)) = 0, so we may have λ = 0, λ = −j or λ = j + 1".
  - Zero modes: for λ = 0 the eigenstates are Y^i = [J^i, Y_jm]. They "correspond to fluctuations in the directions
    of the gauge orbit and not to physical zero-modes".
  - Multiplicities, from their explicit eigenvectors: λ = −j has 2j − 1 states (−j < m < j), and λ = j + 1 has
    2j + 3.
- **§5.3** (general reducible representation): the blocks decompose into spins ½|N_k − N_l| ≤ j ≤ ½(N_k + N_l) − 1,
  which includes half-integers, and "the diagonalization proceeds exactly as above".

**Hypotheses.** The gauge algebra is u(N), acting on Hermitian N×N matrices. The SO(3) mass is single, i.e. the
sphere is round.

**Alignment.** Put Z_k = −iJ_k. Then my operator 1 − L of Step 4 is literally Y ↦ Y + iε[J, Y]. So at t = 1, where
D = 1, my Hess W at q_φ is their SO(3) mass operator (up to their overall factor ⅓). Their eigenvalues
{j + 1, 0, −j} are my {l + 1, 0, −l}.

**What does not transfer by itself.**

- **𝔤 ≠ u(N).** Their computation only uses the su(2) angular-momentum algebra on each spin component, so it applies
  verbatim to any spin-l copy in 𝔤_ℂ. Nonetheless I re-derive the spectrum in Step 4 from S·J, so the proof does not
  lean on their matrix setting.
- **t ≠ 1.** Their formula is false for t ≠ 1. What survives is the inertia, by B2.

### B4. Simon 1983: what it covers, and why (S) stays a named input

B. Simon, "Semiclassical analysis of low lying eigenvalues. I. Non-degenerate minima: asymptotic expansions", Ann.
Inst. H. Poincaré Sect. A 38 (1983) 295–308 (numdam AIHPA_1983__38_3_295_0). Erratum: ibid. 40 (1984) 224, which
concerns Theorem 5.1 in the degenerate case. I do not use Theorem 5.1.

- **Operator.** H(λ) = −Δ + λ²h + λg on L²(ℝ^ν).
- **Hypotheses.**
  - (A1) h, g ∈ C^∞.
  - (A2) g is bounded below and h ≥ 0.
  - (A3) h has finitely many zeros x^{(a)}, and is bounded away from 0 for |x| > R. This is my reading of the scanned
    text.
  - (A4) At each zero the Hessian of h is strictly positive definite.
- **Theorem 1.1.** For each fixed n and λ large, E_n(λ)/λ → e_n, the n-th eigenvalue of ⊕_a K_a, where K_a are the
  harmonic approximations at the zeros.
- **Scope.** Simon himself says (A4) "is critical for the results to have the form they will; we hope to study the
  case where h vanishes on a manifold in a later paper in this series."

**Mismatch with my case.** My zeros are group orbits, not points, and I need the statement on the colour-singlet and
charge-m subspaces, not on all of L². Neither is in the paper. So, as in my SU(2) deduction, the orbit-and-sector
version is the named input (S), not a citation.

### B5. Witten 1982 (the idea, not a theorem I use)

E. Witten, "Supersymmetry and Morse theory", J. Differential Geom. 17 (1982) 661–692, doi:10.4310/jdg/1214437492.

**The idea I borrow.** For −½Δ + ½|∇W|² the low levels sit at the critical points of W and are organised by the
Morse index of W there. The constant ½ΔW is what the factorisation subtracts. In my room ΔW = nS is constant, so the
offset of a well is controlled by the negative eigenvalues of Hess W: that is (A4). I use only harmonic-oscillator
arithmetic, so Witten is cited for the idea.

### B6. Standard facts used without separate citation

- Complete reducibility of finite-dimensional complex su(2)-representations, and their classification by spin l with
  dim V_l = 2l + 1 and Σ_m m² = l(l+1)(2l+1)/3.
- Addition of angular momentum: V₁ ⊗ V_l = V_{l+1} ⊕ V_l ⊕ V_{l−1}, the last term absent for l < 1.
- The Killing form B of a compact semisimple 𝔤 is negative definite.
- Orbits of a compact group acting linearly are compact embedded submanifolds.
- The root system of a complex simple Lie algebra is reduced, i.e. the only roots proportional to θ are ±θ.
- Weyl's compact real form: 𝔤 is spanned by i𝔥_ℝ and e_α − e_{−α}, i(e_α + e_{−α}) for a Chevalley basis.

### B7. From my own closed notes

- Vacuum-floor note §§1–3: compact resolvent, the growth bound, and ΔW = nS.
- H7 §3: the long-root decomposition 𝔤_ℂ = 𝟑 ⊕ (2h^∨ − 4)·𝟐 ⊕ 𝔨_ℂ. I use the doublet count only for bookkeeping;
  the part the proofs need, m₁ = 1 and m_l = 0 for l ≥ 3/2, is re-proved in Step 7.

---

## 4. The named input (S)

**(S) Semiclassical harmonic limit for orbit wells, per symmetry sector.** Let O₀ = {0}, O₁, …, O_r be the vacuum
orbits of Theorem A. For each i:

- pick q_i ∈ O_i, with stabiliser K_i ⊂ G and normal space N_i = T_{q_i}O_i^⊥;
- let h_i = ½(−Δ_y + ⟨y, Hess V(q_i) y⟩) on L²(N_i), and keep its K_i-invariant eigenfunctions;
- in the charge-m version, keep those of charge m, for the circle action defined in Step 9.

Then, for each fixed k:

- the k-th colour-singlet eigenvalue of H_g tends, as g → 0, to the k-th element (with multiplicity) of the union
  over i of these harmonic levels;
- likewise in each charge-m sector.

**Status.** For a single isolated non-degenerate point, without symmetry sectors, this is Simon's Theorem 1.1 after
the rescaling q = p/g, ħ = g². For orbits it is the same IMS localisation in slice coordinates. Near O_i the
G-invariant L² inner product becomes ∫_{N_i} |f(y)|² J_i(y) dy with J_i smooth and positive near 0 (J_i(0) = vol
O_i). **I do not re-prove (S). It is the same input as in `h8-small-coupling-switch-deduction-2026-11-27.md`.**

---

## 5. Proof of Theorem A

### Step 1 (A1): classification

V = ½‖∇W‖² ≥ 0, so V(q) = 0 iff ∇W(q) = 0. By Lemma 0 that means ω_k X_k = g[X_l, X_m] for each cyclic (k, l, m).

Substitute X_k = a_k Z_k. Since

g a_l a_m = g · √(ω_m ω_k) · √(ω_k ω_l) / g² = ω_k √(ω_l ω_m) / g = ω_k a_k,

and every a_k > 0, the equations become [Z_l, Z_m] = Z_k cyclically. These are exactly the conditions for
e_k ↦ Z_k to define a Lie-algebra homomorphism su(2) → 𝔤. Conversely, every homomorphism gives a solution. The map
φ ↦ q_φ is injective, since a_k ≠ 0. ∎

### Step 2 (A3): kernel equals orbit tangent (Whitehead)

Fix q = q_φ and write Z_k = φ(e_k). For a variation δ = (δ₁, δ₂, δ₃), put δ_k = a_k c_k. Lemma 0 and
g a_l a_m = ω_k a_k give

(Hess W · δ)_k = ω_k a_k (c_k − ε_klm [Z_l, c_m]).

So δ ∈ ker Hess W iff, cyclically,

c₁ = [Z₂, c₃] − [Z₃, c₂],  c₂ = [Z₃, c₁] − [Z₁, c₃],  c₃ = [Z₁, c₂] − [Z₂, c₁].

Define the linear map c: su(2) → 𝔤 by c(e_k) = c_k. The 1-cocycle identity

c([x, y]) = [φx, c(y)] − [φy, c(x)]

is bilinear and antisymmetric in (x, y). It therefore holds iff it holds on the pairs (e₂, e₃), (e₃, e₁), (e₁, e₂),
which are exactly the three equations above. So ker Hess W ≅ Z¹(su(2), 𝔤_{ad∘φ}).

By B1, every such c is a coboundary: c(x) = [φx, Λ] for some Λ ∈ 𝔤. Then

δ_k = a_k [Z_k, Λ] = [X_k, Λ] = −(d/ds)|₀ Ad(exp sΛ) X_k,

which lies in T_q(Ad(G)·q).

Conversely, ∇W vanishes on all of Ad(G)·q, since W is Ad-invariant and 𝒵 is Ad-invariant. Differentiating along the
orbit gives T_q(orbit) ⊂ ker Hess W.

Since Hess V = (Hess W)² at q (Lemma 0) and Hess W is symmetric, ker Hess V = ker Hess W.

**Local structure of 𝒵 near the orbit.** Write F = ∇W, so 𝒵 = F⁻¹(0). Put T = T_q(orbit) and P for the orthogonal
projection onto T^⊥ = ran Hess W(q).

- On the affine slice Σ = q + T^⊥, the map P∘F|_Σ has derivative Hess W(q)|_{T^⊥}: T^⊥ → T^⊥ at q, which is
  invertible.
- By the inverse function theorem there is ε > 0 such that q is the only zero of F in Σ_ε = Σ ∩ B(q, ε).
- The map μ: G × Σ_ε → ℝ^{3n}, (h, s) ↦ Ad(h)s, has derivative at (e, q) with image T + T^⊥ = ℝ^{3n}. It is a
  submersion there, so its image contains an open neighbourhood U of q.
- Hence 𝒰 = Ad(G)·U is an open neighbourhood of the orbit. Any zero z ∈ 𝒰 can be written z = Ad(h)s with s ∈ Σ_ε.
  Since 𝒵 is Ad-invariant, s is a zero, so s = q and z lies on the orbit.

So 𝒵 ∩ 𝒰 = Ad(G)·q. ∎

### Step 3 (A2): finiteness

- **𝒵 is closed**, being the zero set of a polynomial map.
- **𝒵 is bounded.** At q_φ, ‖X_k‖ = a_k ‖Z_k‖. By B6, −B is positive definite, so ⟨·,·⟩ ≤ C_𝔤 (−B) for some
  C_𝔤 < ∞ by equivalence of norms. Next, −B(Z_k, Z_k) = −tr((ad Z_k)²) = tr(J_k²) with J_k = i ad Z_k on 𝔤_ℂ. Since
  [J₁, J₂] = iJ₃ cyclically, the J_k are angular-momentum operators. Hence
  tr(J_k²) = Σ_{constituents V_j of ad∘φ} j(j+1)(2j+1)/3. With Σ(2j+1) = n and j ≤ (n−1)/2, this is at most
  n(n²−1)/12.
- **Conclusion.** 𝒵 is compact. By Step 2 each orbit is open in 𝒵, and distinct orbits are disjoint. A compact space
  partitioned into disjoint open sets has finitely many of them. ∎

(Remark, not used: this is Richardson-type rigidity. By Jacobson–Morozov and Kostant the orbits correspond to the
nilpotent orbits of 𝔤_ℂ. For su(N) these are the partitions of N, which is DSV's count.)

### Step 4 (A4): inertia, for every t (Sylvester plus angular momentum)

From Step 2, with D = diag(√ω₁, √ω₂, √ω₃) ⊗ 1_𝔤 and (Lδ)_k = ε_klm [Z_l, δ_m]:

(Hess W · δ)_k = ω_k δ_k − Σ_{l,m} ε_klm √(ω_k ω_m) [Z_l, δ_m],

because g a_l = √(ω_k ω_m) for distinct k, l, m. Hence

**Hess W(q_φ) = D(1 − L)D.**

- **L does not depend on t or g.** It involves only φ, so neither does the inertia below. Hess W itself depends on t
  through D, and not on g.
- **L is symmetric.** Using antisymmetry of ad Z_l,
  ⟨δ', Lδ⟩ = Σ ε_klm ⟨δ'_k, [Z_l, δ_m]⟩ = −Σ ε_klm ⟨[Z_l, δ'_k], δ_m⟩ = Σ ε_mlk ⟨δ_m, [Z_l, δ'_k]⟩ = ⟨δ, Lδ'⟩.
- **Complexify.** Extend ⟨·,·⟩ to a Hermitian form on 𝔤_ℂ. Then ad Z_l is anti-Hermitian, and 𝔤_ℂ splits as an
  orthogonal sum of irreducible ad∘φ-summands U. Both D and L preserve each C³ ⊗ U, and the complexified Hessian
  has the same eigenvalues, with the same multiplicities, as the real one.
- **Spin form of L.** Put J_l = i ad Z_l, which is Hermitian with [J₁, J₂] = iJ₃, and frame spin-1 matrices
  (S_l)_{km} = −iε_lkm, with [S₁, S₂] = iS₃. Then (E_l)_{km} := ε_klm = −i(S_l)_{km} and ad Z_l = −iJ_l, so
  L = Σ_l E_l ⊗ ad Z_l = −S·J and **1 − L = 1 + S·J**.
- **Spectrum of 1 − L on a spin-l copy.** With K the total spin, S·J = ½(K(K+1) − 2 − l(l+1)). So 1 + S·J equals
  - l + 1 on K = l + 1, with multiplicity 2l + 3;
  - 0 on K = l, with multiplicity 2l + 1;
  - −l on K = l − 1, with multiplicity 2l − 1.

  The K = l − 1 term is absent for l < 1, and the K = l term is absent for l = 0. This gives the inertia table of
  (A4) for 1 − L. At t = 1 it is DSV's {j + 1, 0, −j} (B3).
- **Transfer to Hess W.** On C³ ⊗ U, Hess W = D*(1 − L)D with D positive definite, hence invertible. By B2 it has the
  same inertia as 1 − L, for every t > 0. ∎

### Step 5 (A4): the offset formula

- By Lemma 0, the eigenvalues of Hess V = (Hess W)² are μ², so the normal frequencies are |μ| for μ ≠ 0. The zero
  eigenvalues are exactly the orbit directions (Step 2).
- The harmonic ground energy of the transverse oscillator is ½Σ_{μ≠0}|μ|.
- Since tr Hess W = nS (Lemma 0), Off = ½Σ(|μ| − μ) = Σ_{μ<0}|μ|. The terms with μ = 0 contribute 0 to both sums.
- The count is additive over the copies of 𝔤_ℂ, so Off = Σ_l m_l ε_l.
- By Step 4, spins 0 and ½ have no negative eigenvalues, so ε₀ = ε_½ = 0. For l ≥ 1 there are 2l − 1 ≥ 1 of them,
  so ε_l > 0. ∎

### Step 6 (A4, A5): the spin-1 and spin-½ blocks explicitly

**Spin 1.** In a spin-1 copy, Ad-invariance and the su(2) symmetry make ‖Z₁‖ = ‖Z₂‖ = ‖Z₃‖ =: ζ. Take

f₁ = (Z₁, 0, 0)/ζ,  f₂ = (0, Z₂, Z₃)/(√2 ζ).

Directly from (Lδ)_k = ε_klm [Z_l, δ_m] and [Z_l, Z_m] = Z_k:

- Lf₁ = (0, [Z₃, Z₁], −[Z₂, Z₁])/ζ = (0, Z₂, Z₃)/ζ = √2 f₂;
- Lf₂ = ([Z₂, Z₃] − [Z₃, Z₂], −[Z₁, Z₃], [Z₁, Z₂])/(√2 ζ) = √2 f₁ + f₂.

D acts by t^{1/4} on f₁ and by t^{−1/4} on f₂. So span{f₁, f₂} is invariant and

Hess W|_{span{f₁,f₂}} = [[√t, −√2], [−√2, 0]],  with eigenvalues (√t ± √(t+8))/2.

The negative one is −ω_C(t). By Step 4 a spin-1 copy has exactly 2l − 1 = 1 negative eigenvalue, so
ε₁(t) = ω_C(t). ∎

The full spin-1 characteristic polynomial, computed symbolically with r = √t, is

λ³(rλ − 2)²(rλ − r² − 1)²(λ² − rλ − 2).

Its eigenvalues are 0 (×3), 2/√t (×2), √t + 1/√t (×2) and (√t ± √(t+8))/2. These are the SU(2) q_MC frequencies
(competitor, charge ±2 and charge ±1), as they must be.

**Spin ½.** The symbolic characteristic polynomial of D(1 − L)D on C³ ⊗ V_½, with r = √t, is (up to a positive
constant)

λ²(2rλ − 3)²(2rλ − 2r² − 1)².

This gives the doublet eigenvalues stated in (A5). Two consistency checks:

- the trace is 2·3/(2r) + 2(2r² + 1)/(2r) = 2(r + 2/r) = 2S, as Lemma 0 requires;
- at t = 1 all four nonzero eigenvalues equal 3/2 = l + 1, as Step 4 requires.

### Step 7 (A5): the long-root homomorphism

Let θ be a long root of 𝔤_ℂ (with respect to a Cartan subalgebra 𝔥 ⊂ 𝔤_ℂ coming from a maximal torus of G), and let
(e_θ, h_θ, f_θ) be its sl₂-triple. Its compact real form gives a homomorphism φ_θ: su(2) → 𝔤 (B6). This is the
Dynkin-index-1 embedding ι of H7, and all long roots are W-conjugate, so the class does not depend on the choice.

The spins of ad∘φ_θ are half the eigenvalues of ad h_θ. On g_α the eigenvalue is ⟨α, θ^∨⟩ = 2(α, θ)/(θ, θ). For
α ≠ ±θ:

- α is not proportional to θ, since the root system is reduced;
- so by strict Cauchy–Schwarz, |(α, θ)| < |α||θ| ≤ |θ|², because θ is long;
- hence |⟨α, θ^∨⟩| < 2, so ⟨α, θ^∨⟩ ∈ {−1, 0, 1}.

On 𝔥 the eigenvalue is 0. So the eigenvalue 2 occurs only on g_θ, which is one-dimensional, and no eigenvalue
exceeds 2. Therefore m_l = 0 for l ≥ 3/2, and m₁ = dim(eigenspace 2) = 1. By Steps 4–6:

- the Morse index of W at the long-root well is Σ_{l≥1} m_l (2l − 1) = 1;
- Off(φ_θ, t) = m₁ ε₁ = ω_C(t).

**The lower bound.** Let φ ≠ 0. Since su(2) is simple, φ is injective, and φ(su(2)) ⊂ 𝔤 is an ad∘φ-submodule
isomorphic to the adjoint of su(2), i.e. spin 1. So m₁(φ) ≥ 1, and with every ε_l ≥ 0 this gives
Off(φ, t) ≥ ε₁(t) = ω_C(t). ∎

This completes Theorem A.

---

## 6. Proof of Corollary B (given (S))

### Step 8: the hypotheses of (S) hold

- **Finitely many orbit wells, each transversally non-degenerate.** This is Theorem A2–A3.
- **Growth at infinity.** In p = gq, V(q) = g⁻²V¹(p) with V¹ = ½|∇W¹|² and W¹(p) = ½Σω_k‖P_k‖² − 𝒞(p). V¹ is
  independent of g and satisfies V¹ ≥ (κ/2)|p|² − const (vacuum-floor note §2), so it grows at infinity.
- **The Hessian does not depend on g** (Step 4). So the harmonic levels below do not depend on g.

### Step 9: charge on a well

Fix q_φ. Ad(exp θZ₁) fixes Z₁ and rotates (Z₂, Z₃) by θ. Since a₂ = a₃, R_θ q_φ = Ad(exp(−θZ₁)) q_φ. So
ρ_θ := Ad(exp θZ₁) ∘ R_θ fixes q_φ. It is orthogonal and preserves V, so it commutes with Hess V(q_φ) and preserves
the normal space.

- **On colour-singlet functions ρ_θ acts as R_θ**, which is the charge.
- **It is a circle action there.** exp(2πZ₁) is central in the image of SU(2), so it lies in the stabiliser K. It
  therefore acts trivially on K-invariant functions, so only integer charges occur.

### Step 10: the harmonic levels, measured from the floor nS/2

**The origin O₀.** Here K = G and the oscillator has frequency ω_k on each of the n colour directions in frame row k.

- Charge: X₁ quanta carry charge 0, and the circular combinations of X₂, X₃ carry ±1.
- A single quantum lies in ℂ³ ⊗ 𝔤_ℂ, which has no G-invariant vector because 𝔤 is simple. So every excited singlet
  has at least 2 quanta.
- Two quanta: ‖X₁‖² at 2√t (charge 0); ‖X₂‖² + ‖X₃‖² at 2/√t (charge 0); ‖X₂‖² − ‖X₃‖² ± 2i⟨X₂, X₃⟩ at 2/√t
  (charge ±2); ⟨X₁, X₂ ± iX₃⟩ at √t + 1/√t (charge ±1).

Hence:

- the origin's lowest excited charge-0 level is 2 min(√t, 1/√t);
- every charged origin level is at least 1/√t + min(√t, 1/√t). That is ≥ 2 for t ≤ 1 and ≥ 2/√t for t ≥ 1.

**A well O_φ with φ ≠ 0.**

- **Ground state.** The transverse Gaussian, at offset Off(φ, t) ≥ ω_C(t), is non-degenerate because all normal
  frequencies are positive. It is invariant under every orthogonal map of N_φ that commutes with Hess V. That
  includes K and ρ_θ, so it is a colour singlet of charge 0.
- **Every other level** carries at least one quantum, so it lies at ≥ Off(φ, t) + min normal frequency > ω_C(t). In
  particular every charged level of the well is > ω_C(t).

**Elementary inequalities** (ω_C = 4/(√(t+8) + √t)):

- (i) ω_C(t) < 2/√t for all t > 0, because √(t+8) > √t.
- (ii) ω_C(t) ≤ √2 < 2.
- (iii) ω_C(t) ≤ 2√t iff √(t² + 8t) + t ≥ 2 iff t ≥ 1/3, with equality only at t = 1/3.

**Consequences.**

- The only level at offset 0 is the origin's ground state.
- The second charge-0 level is ν₁ = min(ω_C, 2√t, 2/√t) = min(ω_C, 2√t), by (i).
- Every charged level, from any well, is > ω_C ≥ ν₁: by (i) and (ii) at the origin, and by the strict bound above at
  the other wells. There are finitely many wells, and each has a discrete set of levels, so
  ν_* := inf{charged levels} > ν₁.

### Step 11: from levels to eigenvalues

Fix c with ν₁ < c < ν_*, not equal to any level. Let N_c be the number of levels (all charges, with multiplicity)
below c. Every one of them is charge 0, and N_c ≥ 2: the origin's ground state plus the level ν₁.

- By (S) on the whole singlet sector, applied to k = N_c − 1 and k = N_c: for small g exactly N_c singlet eigenvalues
  of H_g − nS/2 lie below c.
- By (S) on the charge-0 sector: for small g, N_c of them are charge 0.
- H_g is reduced by the orthogonal charge decomposition of the singlet space. So no eigenvalue of charge m ≠ 0 lies
  below nS/2 + c, i.e. inf_{m≠0} E_{0,m} ≥ nS/2 + c.
- By (S) in charge 0, E_{0,0} − nS/2 → 0 and E_{1,0} − nS/2 → ν₁ < c.

Choose g₀ so that all four statements hold for g < g₀. Then E_{1,0} < nS/2 + c ≤ inf_{m≠0} E_{0,m}. The limit
E_{1,0} − E_{0,0} → ν₁ = min(ω_C, 2√t), and by (iii) it equals ω_C iff t ≥ 1/3. For t > 1/3 the level ω_C is the
long-root well's Gaussian ground state (A5). ∎

At my control t = 3 this reproduces the SU(2) deduction's numbers, now for every 𝔤: ν₁ = ω_C = 0.79229, and the
charged threshold is 2/√3 = 1.15470 or above. The extra colour modes (doublets, centraliser, other embeddings) only
add levels above ω_C.

---

## 7. Checks run (record only, not part of the proof)

These were small scripts, run with python3.11 and numpy/sympy, under a minute of CPU in total.

- **Universal blocks.** l ∈ {0, ½, 1, 3/2, 2, 5/2} at t ∈ {0.1, 0.3, 1, 3, 10}:
  - inertia exactly as in (A4);
  - tr = (2l+1)S to 10⁻¹²;
  - spectrum of 1 − L = {l + 1, 0, −l};
  - ε₁ = ω_C to 10⁻⁶;
  - at t = 1, ε_l = l(2l − 1).
- **Symbolic characteristic polynomials.** For spin ½ and spin 1, as in Step 6.
- **Independent su(3) check.** Real basis −iλ_a/2, inner product −2tr; Hess W by finite differences of ∇W, and
  Hess V by second differences of V.
  - **Long-root φ**, at (t, g) ∈ {(3, 0.7), (0.2, 1.3), (1, 0.5)}:
    - V(q_φ) ≤ 5·10⁻³²;
    - dim ker Hess W = 7 = orbit dimension;
    - one negative eigenvalue;
    - Off = 0.792287, 1.208175, 1.000000 = ω_C(t);
    - tr Hess W − 8S below 3·10⁻¹¹;
    - ‖Hess V − (Hess W)²‖_max below 10⁻¹⁰.
  - **Principal (spin-1) φ:**
    - dim ker = 8;
    - four negative eigenvalues (1 + 3, from 𝔰𝔲(3)_ℂ = V₁ ⊕ V₂);
    - Off = ε₁ + ε₂ = 5.725170 at t = 3, and 7 = 1 + 6 at t = 1.
- **Threshold.** The symbolic solve of ω_C = 2√t gives t = 1/3.

---

## 8. What this does not give

- **Not the physical room.** Everything is in the homogeneous-mode reduced model. Passing to Yang–Mills on S³_t
  needs H2 (M3 uniform dressing) and H2b, unchanged from SU(2).
- **Not the registered couplings.** g₀(𝔤, t) is not explicit. There are no enclosures and no rate.
- **Not uniform in 𝔤.** The input (S) and g₀ both depend on 𝔤. H7 asks for no uniformity, but large algebras (E₈)
  may have tiny g₀.
- **Not the sourced controls, and not the switch.** At s = 1.15 the centraliser modes are soft (H7 Step 4c, frequency
  about 0.392), so the two-carrier story there is not universal. Clause 2 of my SU(2) small-coupling note does not
  transport. Theorem A concerns s = 1 only.
- **Not (S) itself.** The orbit-and-sector semiclassical limit stays a named input. Simon (B4) covers point wells on
  the full space; he explicitly leaves the manifold case to later work.
- **Not the eigenvectors.** Corollary B is about eigenvalues. It does not prove that the E_{1,0} eigenfunction
  localises in the long-root well. That needs the IMS localisation of (S) with error bounds, or Simon's part II
  (tunnelling) adapted to orbits.
- **Not uniqueness of the competitor's well.** For t > 1/3, if some other conjugacy class of homomorphisms also had
  Off = ω_C (i.e. m₁ = 1 and no spin ≥ 3/2), its Gaussian would share the level ν₁, still in charge 0. I expect only
  the long-root class qualifies: m₁ = 1 forces a single root β with β(h) = 2, so e ∈ g_β, and Kostant's conjugacy of
  sl₂-triples would then identify φ with a root su(2). But I have not written that out, so I do not claim it.
- **Not C-parity.** For Out(𝔤) ≠ 1, H3(G) must track C (H7 §2). The long-root Gaussian is C-even: C maps the
  long-root class to itself, and the combined map fixing q_θ commutes with Hess V. Turning that into "E_{1,0} is
  C = +" needs (S) in C-sectors plus the uniqueness above. Not claimed.
- **The correction for small t.** For 0 < t < 1/3 the second charge-0 level in the limit is the origin's ‖X₁‖²
  breathing mode at 2√t, not the competitor. The research note's "E_{1,0} − E_{0,0} → ω_C(t) for every t", and the
  phrase "the lowest excitations sit at 2/√t and √t + 1/√t", are right only for t ≥ 1/3 and for t > 1 respectively.
  H7 Step 6's "lowest singlets are two quanta in k = 2, 3 at 2/√t" is likewise a t > 1 statement; the
  origin-splitting proof is stated for t > 1, so it is consistent. The H8(G) ordering (charge 0 below every charged
  level) holds for every t > 0. My controls (t = 3) are unaffected.
- **Nothing about H1(G), H3(G), H4′(G) or H6(G).** H7 stays [X].

---

## 9. Summary of standing

| Item | Standing |
|---|---|
| A1 classification of the vacua by Hom(su(2), 𝔤) | proved here (elementary) |
| A2 finitely many orbits, compact zero set | proved here (inverse function theorem + norm bound) |
| A3 Morse–Bott, ker = orbit tangent | **closed by citation**: Whitehead's first lemma (B1), hypotheses exact |
| A4 inertia per spin for all t, offset = Σ\|μ<0\|, ε₁ = ω_C | proved here (Sylvester B2 + angular momentum); DSV (B3) is the t = 1 precedent |
| A5 long root: index 1, Off = ω_C; Off ≥ ω_C for all φ ≠ 0; doublet frequencies | proved here (root-system Cauchy–Schwarz + symbolic blocks) |
| Corollary B: H8(G) at small g, s = 1, every t > 0; limit min(ω_C, 2√t) | **[Cd] on (S)** |

Sources consulted tonight:

- [DSV, arXiv:hep-th/0205185](https://arxiv.org/abs/hep-th/0205185) (PDF read, §§4.2, 5.2, 5.3)
- [Simon 1983, numdam](https://numdam.org/item/AIHPA_1983__38_3_295_0) (PDF read, §1)
- [Simon erratum 1984](https://www.numdam.org/item/AIHPA_1984__40_2_224_0/)
- [MIT OCW 18.745 Lecture 18](https://ocw.mit.edu/courses/18-745-lie-groups-and-lie-algebras-i-fall-2020/mit18_745_f20_lec18.pdf) (PDF read, Theorem 18.4)
- [Whitehead's lemma reference page](https://en.wikipedia.org/wiki/Whitehead%27s_lemma_(Lie_algebra)) (for the Jacobson location)
- [Witten 1982, Princeton record](https://collaborate.princeton.edu/en/publications/supersymmetry-and-morse-theory/)
- Horn–Johnson Theorem 4.5.8, confirmed only through papers that cite it
