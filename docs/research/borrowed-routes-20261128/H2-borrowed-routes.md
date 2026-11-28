# H2 (M3 uniform dressing) and KC1: borrowed routes

*28 November 2026 (night shift). A research note, not a register change. Nothing here edits CLOSED-SET.md or the paper.
Status tags as in the register: [PC] proved and cited, [D] closed by deduction, [elem] elementary argument given here,
[Cd] conditional, [X] open.*

---

## 1. The target

**H2** (register line 16, spectral form in `closed-set-20261127/H2-M3-uniform-dressing.md` §1.2). At s = 1, on the
chart U around q = 0, t in a compact T not containing 1, small g(R): the window levels of the full Hamiltonian lie
within C g²(R) of those of the reduced model, uniformly in Λ_cut (H2a). With o(g²) error against H⁽²⁾_eff (H2b).

**KC1** (H2 §6, the first rung). Fix t. For |F| ≤ f₀, with the heat-trace spectral regularization of line 14,

  2 m_ik(F) = −g² (b₀/8π²) log(ΛR) δ_ik + 2 m^fin_ik(F) + O((ΛR)⁻¹),  m = m^NA + m^C,

where m^NA is the second-order non-adiabatic (Born–Oppenheimer) mass and m^C the Coulomb term of the fast Gauss law.
The sign and normalization are the ones that make the kinetic coefficient run like the potential's
(line 14: E₀ = −(b₀/8π²) log(Λ_cut R) S_cl + finite).

**Why KC1 is necessary for H2 and not a choice of coordinates [elem].** In the geometric coordinate a (the connection
component; line 8 has F = 2gq), the one-loop slow Hamiltonian has the form
(g_E²/2) π_a² + S(a)/(2 g_B²). The potential S(a) has quadratic, cubic and quartic parts, so the dilation a → λa does
not keep this form. Then g_E and g_B are each fixed by the spectrum. Line 14 makes g_B finite. KC1 is exactly the
statement that g_E is finite too, i.e. that the speed of light c = g_E/g_B has no log. If KC1 failed, the levels would
carry g² log(ΛR) and H2 would be false, as H2 §5 item 3 says. The finite part of c is the c₀ of the H2 Flag.

The question I asked: can KC1 be deduced from known results, namely the four-dimensional covariance of the one-loop
log divergence (which forces E² and B² to run together) plus locality of heat-kernel coefficients on the Berger sphere?
**Answer: yes, in effective-action form (§5), closable tonight. In Hamiltonian/Coulomb-chart form it needs two more
steps (§4), and I name both precisely.**

---

## 2. Established work I can borrow (all citations checked against INSPIRE, arXiv or publisher records)

### 2.1 Heat-kernel coefficient a₄ and the Yang–Mills operator: the locality engine

**D. V. Vassilevich, "Heat kernel expansion: user's manual", Phys. Rept. 388 (2003) 279–360, arXiv:hep-th/0306138.**

- Laplace-type operator, eq. (2.2): D = −(g^{μν}∇_μ∇_ν + E), with ∇ the Levi-Civita connection plus a bundle connection
  ω, whose curvature is Ω (eq. (2.10)).
- Heat trace, eqs. (2.21)–(2.22): Tr(f e^{−tD}) ≅ Σ_k t^{(k−n)/2} a_k(f,D), with a_k = (4π)^{−n/2} ∫ √g tr_V(f b_k).
  Eq. (2.27): a_n(f,D) = ζ(0,f,D). In n = 4, a₄ is the coefficient of the log divergence (eq. (1.21) and §2.3).
- **Eq. (4.28)**, closed manifold, any Riemannian metric:
  a₄(f,D) = (4π)^{−n/2} (1/360) ∫ √g tr_V{ f (60 E;kk + 60 R E + 180 E² + 12 R;kk + 5 R² − 2 Ric² + 2 Riem² + 30 Ω_ij Ω_ij) }.
- **Yang–Mills in background Feynman gauge**, eqs. (3.43)–(3.53), on a general Riemannian manifold:
  connection (3.45); endomorphism (3.46): E = −Ric ⊗ 1 + 2 F(B)·c (c = adjoint structure constants);
  curvature (3.47): Ω = Riem ⊗ 1 + F(B)·c ⊗ 1; ghost operator (3.49) with Ω_gh = F·c and E_gh = 0;
  a_k^tot = a_k(D_vector) − 2 a_k(D_ghost) (3.53). The text says only a_n^tot carries the one-loop divergence in a
  gauge-invariant regularization such as ζ.
- **Eq. (4.34)** (stated there for flat space): a₄^tot = (11/96π²) ∫ d⁴x √g F^δ_{ρν} F^γ_{ρν} K_{δγ}. The text says it
  reproduces the structure of the classical action and gives the Yang–Mills β-function coefficient.

*Hypotheses:* smooth compact Riemannian manifold without boundary (or a smearing function f of compact support); smooth
background; Laplace-type operator. *Arithmetic check (mine):* from (4.28) the F-dependent part is
(180·(−4) + 30·4)/360 = −5/3 for the vector and −2·(30/360) = −1/6 for the ghosts, total −11/6 in units of
(4π)⁻² ∫ F·F·K. For su(2), |K| = 2, and the log coefficient per ∫F^aF^a/4 is 11/12π² = (22/3)/8π² = b₀/8π². ✓

### 2.2 The same identity already computed on the round S³ (van den Heuvel)

**B. van den Heuvel, "Glueballs on the three-sphere", Nucl. Phys. B488 (1997) 282–334, arXiv:hep-lat/9608101.**
(Companion: Phys. Lett. B368 (1996) 124, arXiv:hep-lat/9509019. Reviewed in P. van Baal, "QCD in a finite volume",
arXiv:hep-ph/0008206, §7, eqs. (86)–(88).)

- Setting: SU(2) on the **round** S³ (t = 1), 18 slow modes (c, d) (left- and right-invariant), background-gauge
  condition on the high modes, Euclidean path integral, **dimensional regularization by attaching an ε-dimensional torus
  to space** (his eq. 92ff).
- Potential, eqs. (90)–(91): the pole is absorbed by 1/g_R² = 1/g₀² + 11/(12π² ε), "the standard result for SU(2) …
  the renormalization is an ultra-violet effect and does not depend on the global properties of the space".
- **Kinetic term, §3.4, eq. (117):** the one-loop contribution to ċ·ċ + ḋ·ḋ contains (11/12)/ε plus finite constants,
  and "the divergency will be absorbed by the same coupling constant renormalization of eq. (91) that also made the
  potential part finite". Eq. (118)–(119): the scheme is chosen so that the renormalized kinetic term is exactly
  classical (2π²/2g_R²)(ċ² + ḋ²); this fixes the finite part κ₀ = 0.0566264741…
- His diagrams: ghost loop, Q₀ (time-component) loop, vector loop, and a mixed Q₀–Q_i diagram with two insertions of
  W₀ᵢ ∝ ad(Ḃᵢ). In Hamiltonian language the Q₀ and mixed pieces are where the Coulomb/Gauss term m^C lives.
- *Limitations he states:* terms Bⁿ Ḃ² are neglected (so the kinetic log is computed at B = 0 only); the adiabatic
  approximation on S³ is "not controlled by the coupling constant" (van Baal review).

This is KC1's log-matching **at t = 1, F = 0, in a covariant (dimensional) scheme, in the Lagrangian formalism.**

### 2.3 The torus template (Lüscher; Koller–van Baal)

**M. Lüscher, "Some analytic results concerning the mass spectrum of Yang–Mills gauge theories on a torus",
Nucl. Phys. B219 (1983) 233–261.** **J. Koller and P. van Baal, "A non-perturbative analysis in finite volume gauge
theory", Nucl. Phys. B302 (1988) 1–64.**

The effective Hamiltonian, as reproduced in van Baal's review (eq. (9), with values below it), is
L H_eff = −g²/(2(1 + α₁g²)) Σ ∂²/∂(c_i^a)² + (1/4)(1/g² + α₂) Σ (f_ij^a)² + …, with g = g(L) the renormalized coupling
at μ = 1/L and **finite** α₁ = +2.1810429·10⁻², α₂ = +7.5714590·10⁻³. Both the kinetic and the magnetic coefficient are
made finite by **one** coupling renormalization: the torus KC holds at one loop. Integrating out the non-zero momenta
uses Bloch degenerate perturbation theory, which the review calls a rigorous framework (C. Bloch, Nucl. Phys. 6 (1958)
329).

*Hypotheses:* T³, zero-momentum modes **exactly degenerate** with the vacuum at g = 0 (the potential is quartic), a fixed
order in g^{2/3}. van den Heuvel notes explicitly that on S³ the ground state at g = 0 is non-degenerate, so the Bloch
route does not transfer. My Feshbach step (H2 §4, Step 3) is the non-degenerate replacement.

### 2.4 Hamiltonian (spatial-cutoff) lattice: the speed of light does not run

**A. Hasenfratz and P. Hasenfratz, "The scales of Euclidean and Hamiltonian lattice QCD", Nucl. Phys. B193 (1981) 210.**
The abstract says they compute the ratio of the Hamiltonian (continuous time, spatial lattice) and Euclidean Λ-parameters
with the background field method: 0.84 for SU(2), 0.91 for SU(3).
**F. Karsch, "SU(N) gauge theory couplings on asymmetric lattices", Nucl. Phys. B205 (1982) 285–300.** The abstract
says that with background fields at weak coupling he computes how the space-like and time-like couplings change with
the anisotropy ξ, that the two derivatives sum to the one-loop coefficient b₀, and that his result agrees with
Hasenfratz–Hasenfratz as ξ → ∞.

*What I borrow:* a **finite** ratio Λ_H/Λ_E, and finite ∂g_σ⁻²/∂ξ and ∂g_τ⁻²/∂ξ, mean the electric (time-like) and
magnetic (space-like) couplings carry the **same log** under a regulator that cuts only space and keeps time
continuous. Their difference is a finite renormalization: the lattice c₀. *Hypotheses:* flat space, hypercubic lattice,
one loop, temporal gauge with the Gauss law imposed on states (Kogut–Susskind). This is the closest established
analogue of my H2 regulator, which cuts only space spectrally.

### 2.5 Background-field gauge invariance

**L. F. Abbott, "The background field method beyond one loop", Nucl. Phys. B185 (1981) 189–203.** In background-field
gauge the effective action Γ[B] is a gauge-invariant functional of B, so its local divergences are gauge-invariant local
functionals of B. The background field is renormalized only through g, so the geometric coordinate (my a; my q at fixed
renormalized g) is not renormalized. *Hypotheses:* covariant background gauge (Feynman gauge at one loop is enough here),
gauge-invariant regularization.

### 2.6 Adiabatic theory (structure, not uniformity)

**S. Teufel, *Adiabatic Perturbation Theory in Quantum Dynamics*, Lecture Notes in Mathematics 1821, Springer (2003),
doi:10.1007/b13355.** **G. Panati, H. Spohn and S. Teufel, "Space-adiabatic perturbation theory", Adv. Theor. Math.
Phys. 7 (2003) 145–204, arXiv:math-ph/0201055.** These give second-order effective Hamiltonians, including the
non-adiabatic mass correction, for a fast Hamiltonian with an isolated band and a **uniform gap**. *Hypotheses:* the
fast operator family is smooth in the slow variable in operator (or symbol) norm. They are written for finitely many,
or norm-controlled, fast degrees of freedom. They do **not** cover a fast fibre whose ground states at different q are
unitarily inequivalent as Λ → ∞ (Shale; H2 §4 Step 3). So they give the *form* of H⁽²⁾_eff at fixed Λ, not
uniformity.

### 2.7 Coulomb-gauge pieces (context; nothing borrowed at equation level)

- **N. H. Christ and T. D. Lee, "Operator ordering and Feynman rules in gauge theories", Phys. Rev. D22 (1980) 939.**
  The Coulomb Hamiltonian's ordering terms are O(ħ²) beyond the classical term, so they enter at two loops. They do not
  touch KC1.
- **I. B. Khriplovich, "Green's functions in theories with non-abelian gauge group", Sov. J. Nucl. Phys. 10 (1970) 235
  [Yad. Fiz. 10 (1969) 409].** This is the standard source for splitting b₀ in Coulomb gauge into an anti-screening
  Coulomb part and a screening transverse part. I have verified the bibliographic data but not read the equations, so I
  use it only as a reason to expect m^C to outweigh m^NA (the sign in KC1). I quote no numbers from it.

---

## 3. How each aligns with my framing

| Source | Aligns | Does not align / the transfer step needed |
|---|---|---|
| Vassilevich a₄ (2.1) | Local and covariant on **any** Riemannian 4-manifold, so on S³_t × ℝ for every t, including the Berger sphere. It is exactly "locality of the heat-kernel coefficients". Here locality does the work that Lorentz invariance does in flat space: a₄ is built from the 4-metric, which treats τ and space alike pointwise, even though S³_t × ℝ has no boosts. | It is a ζ/proper-time statement about a 4D operator in covariant background gauge. My line 14 uses the 3D heat trace of K(F) (a spatial cutoff) in the Coulomb chart. |
| van den Heuvel (2.2) | The same programme as mine: S³, slow modes, a one-loop effective Hamiltonian, a kinetic term renormalized by the same g. It is the S³ predecessor of my M3 and should be cited as such. | t = 1 only (my T excludes it; his 18-mode space is what my §5 item 1 says is needed there). Kinetic log at B = 0 only. Dim reg, Lagrangian. |
| Lüscher / Koller–van Baal (2.3) | Same shape: kinetic and magnetic coefficients finite after one g(L). | Torus, degenerate zero modes, Bloch theory. Not transferable as a method; only as evidence. |
| Hasenfratz², Karsch (2.4) | **Spatial-only cutoff, continuous time, Gauss law on states**: my Hamiltonian setting. Equal logs, finite c₀. | Flat lattice, not S³_t; not my spectral χ(K(F)) regulator. |
| Abbott (2.5) | Fixes the slow coordinate as geometric and unrenormalized. This matches my bundle reading, where q is a base coordinate and not a renormalized field. | Covariant gauge, not my Coulomb chart. |
| Teufel, PST (2.6) | Give the second-order form of H⁽²⁾_eff, with m^NA as a mass tensor. | Uniform gap and norm control. They cannot handle Λ → ∞ (Shale). |
| Christ–Lee, Khriplovich (2.7) | Coulomb-chart Hamiltonian; sign intuition. | Nothing at equation level. |

**Honest negative.** None of these gives a cutoff-uniform spectral theorem with infinitely many fast modes and marginal
couplings. That remains the open content of H2 (Step 5 of the H2 file: the Bałaban-type multiscale step). The borrowed
work closes the **one-loop algebra** (KC1), not the uniformity.

---

## 4. A reduced route for KC1 (and its place in H2)

**R1 [D, from 2.1 + elementary steps; §5]. KC1-ζ.** The log coefficient of the one-loop background-field effective
action Γ₁[q(·)] on S³_t × ℝ is −(b₀/8π²) S_YM[A_{q(·)}]. Here S_YM is the full classical action of the homogeneous
path, kinetic term included. Hence the q̇² log coefficient equals the potential's and is F-independent.

**R2 [elem, proved below]. Mass = second derivative coefficient.** For the Gaussian fast system at fixed Λ, the
Born–Oppenheimer mass m^NA_ik is exactly the q̇_i q̇_k coefficient (Euclidean, low-frequency) of the one-loop
fast effective action. Proof: with C_ik(τ) = ⟨∂_iH_f(τ)∂_kH_f(0)⟩_conn = Σ_n ⟨0|∂_iH_f|n⟩⟨n|∂_kH_f|0⟩ e^{−Δ_n|τ|}, the
second-order term of the Euclidean effective action is −½∫∫ δq_i(τ) C_ik(τ−τ′) δq_k(τ′). Its Fourier kernel is
Σ_n (·)(2/Δ_n)(1 − p²/Δ_n² + …), so the p² term is Σ_n ⟨0|∂_iH|n⟩⟨n|∂_kH|0⟩/Δ_n³ · q̇_iq̇_k. Since
⟨n|∂_k0⟩ = ⟨n|∂_kH|0⟩/(E₀ − E_n), this equals Re⟨∂_iΩ|Π̄(H_f − e₀)⁻¹Π̄|∂_kΩ⟩ = m^NA_ik. Check on one oscillator
with frequency ω(q): BO gives m = ω′²/(16ω³). The derivative expansion of ½Tr log(−∂² + ω(τ)²), using
∫dk/2π [(k²+ω²)((k+p)²+ω²)]⁻¹ = 1/(ω(p² + 4ω²)), gives V̇²/(64V^{5/2}) = ω̇²/(16ω³). ✓ The Lagrangian
½(1 + 2m)q̇² goes with the Hamiltonian ½p(1 − 2m)p, as in H⁽²⁾_eff.

**R3 [X, gauge transfer]. Coulomb chart ↔ background Feynman gauge at one loop.** Claim: the sum m^NA + m^C in the
Coulomb chart equals the q̇² coefficient of Γ₁ in background Feynman gauge, up to finite terms. Supporting facts:
- (a) The linear source terms vanish in both formalisms on fast modes [elem]. In van den Heuvel's eq. (116) the sources
  are J_ν = (D_μF_μν)(B). For a homogeneous path, J_i ∝ q̈ + ∇V and J₀ = [B_i, Ḃ_i] are SU(2)_L-invariant, hence purely
  j = 0, and the fast fluctuations (j ≥ 1) do not see them. This is the time-dependent version of my "no slow–fast
  linear coupling".
- (b) Both are the same regulated Gaussian integral over the fast transverse fields and the Gauss-law multiplier,
  computed in two gauge fixings (Faddeev–Popov).
- (c) Possible off-shell gauge dependence is proportional to the slow equations of motion. A log-divergent term of the
  form log·X(q)·EOM would be a log field redefinition of the slow coordinate. In geometric coordinates that is not
  harmless (§1). The question is whether the Coulomb-chart slow coordinate picks up a log renormalization; in Coulomb
  gauge Z_A is not tied to Z_g in general.
- (d) What would close it: compute the q̇² log in the Coulomb chart directly, or show that the Feshbach J (H2 §1.3,
  undressed q) agrees with the background-field Ā = slow part of A.

**R4 [X, regulator transfer; sketch]. Log universality for the spatial cutoff χ(K(F)/Λ²R²).** Write
m_ik = ½∫₀^∞ τ² C_ik(τ) dτ (from R2). The log comes from the short-time singularity C_ik(τ) ~ c_ik τ⁻³, which is local
(a Hadamard/heat-kernel statement for the fast fibre) and regulator-free. A spatial cutoff that only smooths C below
τ ~ 1/Λ gives m_ik(Λ) = (c_ik/2) log(ΛR) + finite + o(1), with the same c_ik as the ζ scheme. Equivalently: a
log-divergent (degree −4) integrand gives the same log coefficient on every scaled cutoff region, including the
cylinder {|k| < Λ, k₀ free}. This is the mechanism behind Hasenfratz–Hasenfratz/Karsch (finite c₀). The instantaneous
m^C needs the analogous statement for a static spatial sum. Line 14 has already checked the potential half
numerically (b₀ to 10⁻⁴ with exactly this regulator).

**Then KC1 = R1 + R2 + R3 + R4.** In H2, KC1 is Step 4; it feeds Step 3 (H⁽²⁾_eff finite with one coupling) and is the
first rung of Step 5's multiscale estimate (δc_m summable). Steps 1 (large fields) and 5 (uniformity) get **no**
borrowed shortcut from this literature beyond the Bałaban/Magnen–Rivasseau–Sénéor templates already listed in the H2
file.

**Spin-off for H8/line 15.** R1 plus van den Heuvel's scheme (renormalized kinetic term exactly classical, eq. (118))
gives a **concrete convention for the finite part of c₀**: choose the scheme in which the F = 0 kinetic correction is
zero. Then the O(g²) kinetic shift −m^ren_uu ω_k in H2 Interface reduces to its F-dependent part, which is finite and
scheme-free. Computing that part (Bⁿ Ḃ² terms, which van den Heuvel neglected) is what settles the sign at line 15.
This is a finite one-loop sum on the known Berger spectrum: a computation, not a proof, and the CPU is busy tonight.

---

## 5. The most closable sub-statement now

**KC1-ζ (effective-action form, Berger sphere, all t, all F).** Let t > 0, let q(τ) be a smooth path of homogeneous
fields, and let A_q = q_{ka}(τ) θᵏ T_a in temporal gauge on M = S³_t × S¹_β (or on S³_t × ℝ with a compactly supported
smearing f(τ)). Let D_v and D_gh be the background-Feynman-gauge vector and ghost operators of Vassilevich (3.45)–(3.49)
on M, restricted to Peter–Weyl sectors j ≥ 1. Then

  a₄^tot(f; j ≥ 1) = a₄^tot(f) = (11/96π²) ∫_M f √g F^δ_{μν} F^γ_{μν} K_{δγ} + (A-independent curvature terms).

Consequently the log-divergent part of Γ₁[q(·)] is −(b₀/8π²) log(ΛR) · S_YM[A_q], with b₀ = 22/3. In particular,
because F_{0i} = ∂_τ A_i in temporal gauge, the log coefficient of the q̇² term is −(b₀/8π²) times the classical
kinetic form: δ_ik in line-8 normalization, since H_red = ½|p|² is the homogeneous classical restriction (line 13). It
does not depend on q. This is KC1's log identity in ζ/covariant form.

**Deduction.**
1. *Laplace type on M.* Vassilevich (3.43)–(3.47) hold on any Riemannian 4-manifold. S³_t × S¹_β with the Berger
   metric is one, and A_q is smooth. [PC]
2. *Cross terms vanish.* Expand (4.28) with E = −Ric⊗1 + 2F·c and Ω = Riem⊗1 + F·c⊗1. Every term linear in F carries
   tr_color(c^γ) = c^γ_{αα} = 0 (the adjoint generators are traceless), and the term linear in Ω also carries the trace
   R^σ_{σμν} = 0. So tr(RE), tr(E²) and tr(ΩΩ) split into a pure-curvature part plus the flat-space F² part. E;kk and
   R;kk are total derivatives (with f, they give boundary-free terms that vanish for f ≡ 1 on S¹_β). So the F-dependent
   part of a₄^tot is (4.34) verbatim, now on the curved M. The arithmetic is checked in §2.1. [elem]
3. *Removing j = 0 adds no log.* For homogeneous backgrounds the fluctuation operators commute with SU(2)_L (line 13),
   so they are block-diagonal in j. The j = 0 block acts on finitely many spatial modes. It is a matrix 1D operator
   −∂_τ² + (first order) + M(τ), whose heat trace is t^{−1/2}Σ_k c_k t^k: there is no t⁰ term, so it does not
   contribute to ζ(0) or to the log. Hence a₄^tot(j ≥ 1) = a₄^tot. (van den Heuvel handles this with an explicit
   projector.) [elem]
4. *Evaluate.* For A_q in temporal gauge, F_{0i} = ∂_τ A_i = q̇_{ka} θᵏ_i T_a and F_ij is the line-8 field strength.
   ∫_{S³_t} F·F splits into the kinetic part (proportional to |q̇|² in line-8 normalization) and 4V/(…) in the same
   normalization that gives S_cl. The log coefficient multiplying both is the single number b₀/8π² (§2.1 check). [elem]
5. *Cross-check.* At t = 1, F = 0, this is van den Heuvel eq. (117) against eq. (91): the same 11/(12π²ε) pole in
   kinetic and potential. [PC]

**What KC1-ζ does not give, stated exactly.** (i) The Coulomb-chart decomposition m^NA + m^C and its identification
with the background-gauge q̇² coefficient (R3). (ii) The same log under my spatial heat-trace cutoff (R4). (iii) The
finite part m^fin and its analyticity, and the O((ΛR)⁻¹) remainder. (iv) Anything about uniformity (H2 proper). With
R2 proved, KC1 as worded in H2 §6 is **conditional on R3 and R4**. Both are one-loop, finite-dimensional-in-spirit
statements. Neither needs the Bałaban step.

**Domain note.** KC1-ζ holds for every t, including t = 1, and for every F where the operators are smooth. At the
reducible copy q_MC the ghost operator has zero modes, and ζ(0) then needs the usual zero-mode subtraction. I restrict
to |F| ≤ f₀ as in H2, to be safe.

---

## Sources

- Vassilevich, Phys. Rept. 388 (2003) 279, arXiv:hep-th/0306138, https://arxiv.org/abs/hep-th/0306138 (eqs. 2.2, 2.10,
  2.21–2.27, 3.43–3.53, 4.28, 4.34 read on ar5iv).
- van den Heuvel, Nucl. Phys. B488 (1997) 282, arXiv:hep-lat/9608101 (eqs. 90–91, 116–119 read from the arXiv PDF);
  Phys. Lett. B368 (1996) 124, arXiv:hep-lat/9509019.
- van Baal, "QCD in a finite volume", arXiv:hep-ph/0008206 (eqs. 9, 86–88 and coefficient tables read from the PDF).
- Lüscher, Nucl. Phys. B219 (1983) 233 (INSPIRE 183485). Koller and van Baal, Nucl. Phys. B302 (1988) 1 (INSPIRE).
- Hasenfratz and Hasenfratz, Nucl. Phys. B193 (1981) 210 (INSPIRE abstract). Karsch, Nucl. Phys. B205 (1982) 285
  (INSPIRE abstract).
- Abbott, Nucl. Phys. B185 (1981) 189 (INSPIRE).
- Teufel, LNM 1821 (2003), doi:10.1007/b13355. Panati, Spohn and Teufel, Adv. Theor. Math. Phys. 7 (2003) 145,
  arXiv:math-ph/0201055.
- Christ and Lee, Phys. Rev. D22 (1980) 939 (INSPIRE). Khriplovich, Sov. J. Nucl. Phys. 10 (1970) 235 (INSPIRE;
  equations not read).
