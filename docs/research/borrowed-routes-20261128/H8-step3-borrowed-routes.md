# Certified bounds for the reduced matrix model: what I can borrow (H1, H8 Steps 3–4)

28 November 2026. **Research note. Nothing here changes a status in the register.** Every number marked
*floating* is an ordinary double-precision estimate, not an enclosure.

Revised 28 Nov after the referee pass: §4a no longer calls the quotient "a ratio of positive integrals" (N_H changes sign, so ∫(h − U)N₁ involves cancellation).

## 1. The precise targets

The operator is the one in the H8 file, unchanged: on H = L²(ℝ^{3×3})^{SO(3)} (colour singlets, ψ(qOᵀ) = ψ(q)),

  H_g = −½Δ_q + V₀,  V₀ = ½ Σ_{k,c} (ω_k q_{kc} − g cof(q)_{kc})² = ½|∇W|²,  W = ½Σ_k ω_k‖q_k‖² − g det q,

with t = 3, ω = (√3, 1/√3, 1/√3), S = 5/√3, the natural (Friedrichs) realization, compact resolvent (Theorem A,
line 9/19), and the Berger circle charge m (rows 2,3 rotated). Write E_{n,m} for the n-th eigenvalue (from 1, with
multiplicity) of H_g on charge m. The registered couplings are g² ∈ {0.002, 0.004, 0.03, 0.05, 0.07}.

- **Step 3 (H8, upper bound).** At s = 1, prove E_{2,0} ≤ U_g with U_g < 5.2731 = (225 + 49)/(30√3) at every
  registered g². Why that threshold: the proved charge tail gives E_{1,m} ≥ (225 + m²)/(30√3), so U_g < 5.2731
  excludes every |m| ≥ 7.
- **Step 4 (H8, lower bounds).** At s = 1, prove E_{1,m} > U_g for 1 ≤ |m| ≤ 6. (By charge reversal, m and −m are
  the same.)
- **H1 = S_C (lower bound with counting).** At the sourced controls (t = 3, s = 1.15, g² ∈ {0.002, 0.004}), prove
  λ₅(H) ≥ ρ_C on the whole singlet space. By Lemma R this gives "at most two charge-0 levels below ρ_C".

Harmonic orientation (from the H8 file, absolute energies): 3S/2 = 4.3301. E_{2,0} → 3S/2 + ω_C = 5.1224 (the
ground state of the q_MC well). The binding competitor in Step 4 is charge ±2 at the origin, at 3S/2 + 2/√3 = 5.4848.
The other charges sit at harmonic levels 6.06 (|m| = 3), 6.64 (|m| = 1, 4), 7.22 (5) and 7.79 (6). So Step 3 has a
margin of about 0.15 against 5.2731, and Step 4 has about 0.2 to 0.36, depending on U_g.

## 2. Established approaches, with verified citations

### 2a. The matrix bootstrap (positivity plus stationarity)

**Han, Hartnoll and Kruthoff,** "Bootstrapping Matrix Quantum Mechanics", *Phys. Rev. Lett.* **125**, 041601 (2020),
arXiv:2004.10212. I read the paper's text. What I borrow:
- *Stationarity.* "If the state ρ commutes with the Hamiltonian then ⟨[H,O]⟩ = 0, ∀O" (their eq. 9). For a single
  particle there is the stronger ⟨OH⟩ = E⟨O⟩.
- *Positivity.* ⟨O†O⟩ ≥ 0, so the moment matrix M_ij = ⟨O_i†O_j⟩ is positive semidefinite (their eq. 7 and
  supplement).
- *Gauge constraint.* ⟨tr G O⟩ = 0 for the SU(N) generator G.
- *Symmetry sector.* In their two-matrix model they impose SO(2) ≅ U(1) invariance by keeping only words of net charge
  zero (supplement: the basis A, B, C, D with eigenvalues −1, 1, 1, −1, and n(A) − n(B) − n(C) + n(D) = 0).
- *Excited states.* For the anharmonic oscillator, "the region of allowed values splits into a discrete set of
  islands", and these converge to the spectrum as K → ∞ (their ref. [20]: if the constraints hold for all O and
  ⟨OH⟩ = E⟨O⟩, then ρ is an eigenstate). "Higher energy states require more constraints."
- *Rigorous bounds there.* Their rigorous upper and lower bounds in the two-matrix case come from a Born–Oppenheimer
  wavefunction, not from the bootstrap itself.

**Lin and Zheng,** "High-Precision Bootstrap of Multimatrix Quantum Mechanics", *Phys. Rev. Lett.* **136**, 181603
(2026), arXiv:2507.21007. This covers models obtained by dimensionally reducing Yang–Mills, at **large N and in the
ground state only**.

**Lawrence,** "Bootstrapping Lattice Vacua", arXiv:2111.13007 (2021). "Requiring expectation values of manifestly
positive semi-definite operators to be non-negative" gives a lower bound on the ground-state energy.

**Laliberte and McPeak,** "Bootstrapping supersymmetric (matrix) quantum mechanics", arXiv:2510.01356 (2025). This
covers N = 1 SUSY QM with cubic superpotential and the Marinari–Parisi model, with ground-state bounds that are tight
at weak coupling against instanton results. It does not treat excited states or sectors.

**Keren,** "Stationarity is not enough: tightness of the quantum mechanical bootstrap and the copositive cone",
arXiv:2608.07047 (2026). It proves that in two or more variables, stationarity plus positivity can admit moment
vectors of no state. Adding the eigenstate constraint ⟨OH⟩ = E⟨O⟩ removed every violation found numerically, but that
is a conjecture, not a theorem. *For me this concerns sharpness, not soundness:* a true eigenstate always gives a
feasible moment vector, so infeasibility is still a valid exclusion.

**Exact certificates.** Peyrl and Parrilo, "Computing sum of squares decompositions with rational coefficients",
*Theoret. Comput. Sci.* **409** (2008) 269–281. Their rounding and projection step turns a numerical SOS/SDP
solution with a strictly feasible margin into an exact rational certificate.

### 2b. Lehmann–Goerisch, Temple and Kato, with Plum's homotopy

- **Temple,** "The theory of Rayleigh's principle as applied to continuous systems", *Proc. Roy. Soc. London A* **119**
  (1928) 276–293.
- **Kato,** "On the upper and lower bounds of eigenvalues", *J. Phys. Soc. Japan* **4** (1949) 334–339.
- **Lehmann,** "Beiträge zur numerischen Lösung linearer Eigenwertprobleme I, II", *ZAMM* **29** (1949) 341–356 and
  **30** (1950).
- **Goerisch and Haunhorst,** *ZAMM* **65** (1985) 129–135.
- **Behnke and Goerisch,** "Inclusions for eigenvalues of selfadjoint problems", in *Topics in Validated Computations*,
  North-Holland 1994, 277–322.
- **Plum,** "Bounds for eigenvalues of second-order elliptic differential operators", *ZAMP* **42** (1991) 848–863
  (homotopy method; the ODE version is *ZAMP* **41** (1990) 205–226).
- **Behnke, Mertins, Plum and Wieners,** "Eigenvalue inclusions via domain decomposition", *Proc. R. Soc. Lond. A*
  **456** (2000) 2717–2730.
- **Book:** Nakao, Plum and Watanabe, *Numerical Verification Methods and Computer-Assisted Proofs for PDEs*, Springer
  Ser. Comput. Math. 53 (2019).

The exact statement I borrow is the abstract Lehmann form, which is the one my R2 registration already uses. Let H
be self-adjoint and bounded below, and v₁, …, v_n ∈ D(H). Set A₀ = (⟨v_i,v_j⟩), A₁ = (⟨Hv_i,v_j⟩) and
A₂ = (⟨Hv_i,Hv_j⟩). Take ρ ∈ ℝ with A₂ − 2ρA₁ + ρ²A₀ > 0, and let τ₁ ≤ … ≤ τ_n be the eigenvalues of
(A₁ − ρA₀)x = τ(A₂ − 2ρA₁ + ρ²A₀)x. Then for each i with τ_i < 0, the interval [ρ + 1/τ_i, ρ) contains at least i
eigenvalues of H. Applied with ρ ≤ λ_{s+1}, this gives λ_{s+1−i} ≥ ρ + 1/τ_i.

I checked the published Laplacian version, BMPW 2000 Thm 2.1, as reproduced in Vejchodský, arXiv:1602.08961, Thm 1.
That version has matrices M = (∇ũ_i,∇ũ_j) + (γ−ρ)(ũ_i,ũ_j) and N, which adds the flux σ̃: "for all i such that μ_i < 0,
the interval [ρ − γ − ρ/(1−μ_i), ρ − γ) contains at least i eigenvalues". The hypothesis that matters is the same:
**an a priori lower bound ρ ≤ λ_{s+1} is needed.** Plum's homotopy exists to supply it, starting from a base problem
with a known spectrum.

### 2c. Rayleigh–Ritz plus symmetry projection (for upper bounds)

- *Min–max* (Reed–Simon IV, §XIII.1). For any 2-dimensional subspace of the form domain of H|_{H₀}, E_{2,0} is at
  most the larger Ritz value.
- *Peierls and Yoccoz,* "The collective model of nuclear motion", *Proc. Phys. Soc. A* **70** (1957) 381–387. A state
  that breaks a symmetry is averaged over the group to give a trial state with good quantum numbers. This is the
  angular-momentum projection of nuclear physics. I checked the bibliographic record and the described method, not the
  full text.

### 2d. The SU(2) constant-field literature (for comparison, not certification)

- **Pavel,** "SU(2) Yang–Mills quantum mechanics of spatially constant fields", *Phys. Lett. B* **648** (2007) 97–106,
  arXiv:hep-th/0701283. Symmetric gauge (six physical fields in a positive symmetric 3×3 matrix), an analytic
  oscillator basis, variational. No rigorous error bounds are claimed.
- **Lüscher,** "Some analytic results concerning the mass spectrum of Yang–Mills gauge theories on a torus", *Nucl.
  Phys. B* **219** (1983) 233–261. Effective Hamiltonian for constant modes, expanded perturbatively.
- **van Baal and van den Heuvel,** "Zooming-in on the SU(2) fundamental domain", *Nucl. Phys. B* **417** (1994)
  215–237, arXiv:hep-lat/9310005. S³, an 18-dimensional space containing the tunnelling path and the sphaleron.

## 3. Alignment with my framing

| Borrowed piece | Aligns | Fails or needs transfer |
|---|---|---|
| Bootstrap / SOS (2a) | Very well. My existing proofs are already SOS certificates in the Weyl algebra: H = ½Σa_i*a_i + 3S/2 with a_i = ∂_i + ∂_iW (the vacuum floor), and the charge tail is a twisted square plus the horizontal barrier. The bootstrap dual is exactly "H − E* = Σ B_k†B_k + (terms vanishing on the sector)". The sector terms are what I already use: colour generators G_a annihilate singlets, so B G_a + G_a†B† drops out, and (L − m)C + C†(L − m) drops out on charge m. HHK's U(1) word-counting is literally my charge selection. My superpotential is Laliberte–McPeak's SUSY structure. | (i) The literature is large-N or ground state; N = 2 at finite N with charge sectors is new work, though easy in principle (9 commuting coordinates, not trace words). (ii) The bootstrap gives *lower* bounds on the **lowest** level of a sector and exclusion windows. It cannot give an *upper* bound on E_{2,0}, because exclusion is not existence, and it cannot *count* (λ₅) without a finite-rank extension (§4c). (iii) It works at weak coupling only after rescaling: the q_MC well sits at \|q\| ≈ 1.53/g (34 at g² = 0.002), so monomial moment matrices in q are badly conditioned. I must use p = gq (ħ = g²) and moments centred on both wells. (iv) Multivariate sharpness is not guaranteed (Keren). |
| Lehmann–Goerisch + Plum (2b) | Already my S_C route (R1 base, outside bound, R2 interval Lehmann). Domain decomposition (BMPW 2000) is my B3 tube bracketing. Works in any symmetry sector, provided the trial functions and base live in that sector. | Plum's and BMPW's theorems are stated for bounded domains and elliptic operators. My unbounded ℝ⁹ needs Neumann bracketing plus an outside potential bound, as done for S_C. This is an extra step I have already carried out once, not automatic. The Lehmann form needs v_i ∈ D(H), not only the form domain. |
| Ritz + Peierls–Yoccoz projection (2c) | Exactly my geometry. The q_MC well is a "deformed intrinsic state". Colour averaging over SO(3) makes it a singlet, and choosing the Gaussian invariant under the stabiliser circle makes it charge 0. The deformed vacuum lives on an orbit, and the orbit is what I average over. | Gives only upper bounds. |
| Pavel, Lüscher, van Baal (2d) | Pavel's symmetric gauge q = P·O is my signed-polar chart (line 7). His oscillator basis is a ready trial basis. | Different potential: flat constant fields have no quadratic or cubic Chern–Simons terms. Variational or perturbative, with no certified bounds. van Baal keeps left and right modes, so his two vacua are gauge-equivalent and mine are not. Useful only as cross-checks of method. |

## 4. The reduced route

### 4a. Step 3, via projected-Gaussian Ritz. This is the cheapest piece, and I think it is closable this week.

Let φ₀(q) = exp(−½Σ_k ω_k‖q_k‖²). It is colour invariant, charge 0 and Schwartz. Let a = q_MC = diag(u,1,1)/g, and
let A = |J| on the six physical directions and α·1 on the three gauge directions, where J = Hess W(a). A commutes with
the stabiliser circle q ↦ RqRᵀ, because W is separately invariant under q ↦ Rq (ω₂ = ω₃) and under colour, and a is
fixed. Put φ(q) = exp(−½(q−a)ᵀA(q−a)) and

  ψ₁ = ∫_{SO(3)} φ(·Oᵀ) dO   (normalised Haar measure).

Then ψ₁ is a colour singlet. It is charge 0, since ψ₁(Rq) = ψ₁(RqRᵀ) = ψ₁(q). It is Schwartz, so it lies in D(H).
Haar invariance and the colour invariance of H give closed forms:

  ⟨ψ₁,ψ₁⟩ = ∫ dO N₁(O),  ⟨ψ₁,Hψ₁⟩ = ∫ dO N_H(O),  ⟨φ₀,ψ₁⟩ = ⟨φ₀,φ⟩,  ⟨φ₀,Hψ₁⟩ = ⟨φ₀,Hφ⟩.

Here N₁(O) = ⟨φ, φ(·Oᵀ)⟩ and N_H(O) = ⟨φ, Hφ(·Oᵀ)⟩ are products of two 9-dimensional Gaussians. Since H is −½Δ
plus a quartic polynomial, they are *explicit elementary functions of O*: an exponential of a quadratic form times
Gaussian moments of degree at most 4. Min–max on span{φ₀, ψ₁} gives **E_{2,0} ≤ μ₂**, the larger root of the 2×2
pencil. Only the two ψ₁-diagonal entries need a 3-dimensional integral, and every integrand is analytic and
explicit.

**Floating evidence (receipt: `h8_step3_projected_gaussian_floating.py`; α = 1; exact 3-point Gauss–Hermite for the
degree-4 moments; grid quadrature on a 5σ ball in rotation-vector coordinates with the Haar density
(2 − 2cos θ)/θ²; converged in the grid to about 10⁻³):**

| g² | R(ψ₁) | R(φ₀) | relative overlap | margin to 5.2731 |
|---|---|---|---|---|
| 0.002 | 5.1222 | 4.3376 | ~10⁻⁹³ | 0.151 |
| 0.03 | 5.1181 | — | — | 0.155 |
| 0.07 | 5.1094 | 4.5926 | ~2·10⁻³ (unaveraged scale) | 0.164 |

So μ₂ ≈ R(ψ₁), with margin 0.15 to 0.16 at every coupling checked. That is about 100 times the quadrature error.
A single Gaussian already suffices; no polynomial correction is needed. (Without colour averaging, the Rayleigh
quotient is 5.9, since the gauge width costs energy. **The projection is essential**, which is the Peierls–Yoccoz
point.)

**What certification needs.** This is light, CPU-only work.
1. Interval or Arb evaluation of the closed-form N₁ and N_H on boxes of a 3-dimensional cover of the ball |θ| ≤ θ₀,
   with Taylor or monotonicity bounds per box. This is a few thousand boxes, each evaluating a 9×9 Gaussian formula.
2. A tail bound off the ball. There, N₁(O) ≤ exp(−c θ₀²/g²) and |N_H| ≤ poly · N₁, with c taken from the proved
   positive definiteness of the exponent's quadratic form. One also needs to check that q_MC Oᵀ is near q_MC only
   near O = 1, which holds because q_MC is invertible.
3. Interval 2×2 pencil.

The decision is the sign of ∫(h − U)N₁. *(Referee pass, 28 Nov: this is not a ratio of positive integrals. N₁ > 0,
but N_H is not positive, h = N_H/N₁ changes sign, and the integral involves cancellation that widens interval
enclosures; see `H8-step3-closure.md` §7.)* The
remaining couplings 0.004 and 0.05 interpolate the table but must be run.

### 4b. Step 4, lower bounds in charges 1–6. The bootstrap can do it in principle, and I would run it second.

*Option B (bootstrap/SOS, new but aligned).* Per charge m, search for an exact identity on C_c^∞:

  H − E*_m = Σ_k B_k†B_k + Σ_a (C_a G_a + G_a†C_a†) + ((L − m)D + D†(L − m)),

with B_k, C_a and D polynomial in (p = gq, ∂_p) and E*_m > U_g. C_c^∞ ∩ H_m is a form core, because averaging over
colour and the circle preserves C_c^∞. So the identity implies E_{1,m} ≥ E*_m: this is the bootstrap's dual. Peyrl–
Parrilo rounding makes the certificate exact.

At g = 0 such a certificate exists at degree 2, and it is sharp. Per colour c, the row-(2,3) oscillator has
N_c − L_c = 2n_{−,c}. So H₀ − (3S/2 + u|m|) = √t N₁ + 2u Σ_c n_{∓,c} + u(±L − |m|), which gives 3S/2 + 2/√3 at
|m| = 2. At g > 0 the natural seeds are the deformed annihilators a_i. My computation gives the exact identity

  ½Σ_c (a_{+c}*a_{+c} − a_{−c}*a_{−c}) = u·(charge) − i g Y,  Y = Σ_c (cof_{2c}∂_{3c} − cof_{3c}∂_{2c}),

where Y is divergence-free, so iY is symmetric. The obstacle is that −igY is first order and unbounded. It must be
absorbed by higher-degree squares, which is where the SDP earns its keep. Whether a low-degree certificate reaches
E*₂ > U_g at g² = 0.07 is **unknown**. Keren's result is a warning that it may need high degree.

*Option LG (proven machinery).* Reuse the S_C stack at s = 1, in charge-m subspaces: Neumann bracketing into an origin
tube, an orbit tube, and the outside (needing V ≥ U_g + margin, proved ray by ray as in the S_C outside bound). In the
orbit tube the charge-m levels are at least ω_C + 2/√3 ≈ 6.28 for |m| ≥ 1, so crude IMS or harmonic comparison
suffices. In the origin tube, |m| = 2 is the only tight case (≈ 5.48 against U_g ≈ 5.15 to 5.27), and an LG homotopy
from the harmonic base handles it. The reduced origin splitting (charge ±2 unshifted at O(g²)) says the margin does
not erode.

### 4c. H1 / S_C

Keep the registered R2 route (Lehmann–Goerisch on the folded homotopy, floating margin +0.21 to +0.24). The bootstrap
does **not** replace it: a lower bound on λ₅ is a counting statement. The SOS analogue, "H − ρ + Σ_{k≤4} c_k|f_k⟩⟨f_k|
⪰ 0", is a finite-rank Temple/Lehmann certificate. It is the same mathematics as LG, with no saving. The new piece
from 4a that helps H1: **projected-Gaussian trial functions** give upper bounds on the four lower levels that Lemma R
counts (E_H + R_H < ρ_C), with closed-form matrix elements and without Arb radial integrals. The earlier registration
found those integrals cost weeks of CPU.

## 5. The most closable sub-statement now

**Lemma (projected-Gaussian min–max; closable by deduction now).** Take t = 3, s = 1, any g > 0, any α > 0, and φ₀,
φ, ψ₁ as in §4a. Then ψ₁ ∈ D(H_g) ∩ H₀ (singlet, charge 0). The Gram and energy matrices of {φ₀, ψ₁} are

  M = [[⟨φ₀,φ₀⟩, ⟨φ₀,φ⟩], [⟨φ₀,φ⟩, ∫N₁]],  K = [[⟨φ₀,Hφ₀⟩, ⟨φ₀,Hφ⟩], [⟨φ₀,Hφ⟩, ∫N_H]],

with N₁ and N_H explicit elementary functions on SO(3). Hence E_{2,0}(g) ≤ μ₂(K, M), the larger generalised
eigenvalue.

*Deduction.*
1. Invariance. φ(·Oᵀ) is Schwartz and depends continuously on O, so the Bochner integral ψ₁ is Schwartz-class
   (differentiate under the integral; SO(3) is compact). It is colour invariant by Haar invariance. It has charge 0
   because A and a are fixed by q ↦ RqRᵀ, and on singlets that action equals q ↦ Rq.
2. The matrix entries. H commutes with the colour action. So ⟨φ(·O₁ᵀ), Hφ(·O₂ᵀ)⟩ = N_H(O₂O₁ᵀ), and Fubini plus Haar
   invariance gives ∫∫ = ∫N_H. For the φ₀ entries, φ₀ and Hφ₀ are invariant, so the average drops out.
3. Bound. φ₀ and ψ₁ are linearly independent (different centres). Then min–max (Reed–Simon IV §XIII.1) on H_g|_{H₀},
   which is self-adjoint with compact resolvent by Theorem A, gives E_{2,0} ≤ μ₂.
4. Closed form. N₁ and N_H are integrals of a polynomial of degree at most 4 against the product Gaussian with
   precision C = A + MᵀAM and the standard mean. Isserlis' theorem evaluates them.

**What closes Step 3 after the lemma:** a certified enclosure of the two SO(3) integrals at the five couplings,
showing μ₂ < 5.2731. Floating values are 5.109 to 5.122. This is a light interval computation, minutes of CPU, not a
research problem. With Step 3 closed, H8_reg reduces to Step 4, a finite list of six charges with |m| = 2 the only
tight one.

## Status line

Nothing is newly proved in the register's sense. The lemma in §5 is a deduction I am confident of, and the
certification of Step 3 is a short computation away. Step 4 and H1 have honest routes (SOS bootstrap, aligned but
unproven at the needed degree; or LG, proven machinery), with no shortcut.
