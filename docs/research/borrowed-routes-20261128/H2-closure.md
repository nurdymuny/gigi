# KC1-ζ: the kinetic and potential logs are one log (effective-action form)

*28 November 2026 (night shift). A proof note, not a register change. It does not edit CLOSED-SET.md, the paper, or any
existing note. Research context: `H2-borrowed-routes.md` (same folder), §§2.1, 2.2, 4, 5. Tags as in the register:
[PC] proved and cited, [elem] elementary argument given in full here, [Cd] conditional, [X] open.*

**Verdict.** The route works for the statement as I give it in §2. Every step is either cited with matching hypotheses or
proved here. The result is about the one-loop **background-Feynman-gauge effective action with a four-dimensional
proper-time (or ζ) regulator**. It is not yet KC1 as worded in H2 §6, which is about the Coulomb-chart masses
m^NA + m^C under my spatial cutoff. Step (e) of the proposed route, which used R2 to make that identification, does
**not** go through as stated; §6 says exactly why and what is still needed (R3 and R4).

---

## 1. Setting and conventions

**Manifold.** Fix t > 0, R > 0, β > 0. Let θ¹, θ², θ³ be the left-invariant Maurer–Cartan 1-forms on SU(2) ≅ S³, with
dual left-invariant frame e₁, e₂, e₃, and let h_t = t θ¹⊗θ¹ + θ²⊗θ² + θ³⊗θ³ be the Berger metric (t, 1, 1) of Part IV.
Set

  M = S³ × S¹_β,  g = dτ² + R² h_t,  τ ∈ ℝ/βℤ.

M is a smooth, compact, connected 4-manifold without boundary, and g is a smooth Riemannian metric. Write
V_t = Vol(S³, h_t), and h_t^{kl} for the inverse of h_t in the frame θ (so h_t^{kl} = diag(1/t, 1, 1)).

**Gauge algebra.** su(2) with basis T_a, [T_a, T_b] = ε_{abc} T_c. The structure constants c^a_{bc} = ε_{abc} are totally
antisymmetric, and Vassilevich's Killing form (his eq. (4.32)) is K_{δγ} = c^δ_{αβ} c^γ_{αβ} = 2δ_{δγ}. For a connection
A = A^a T_a, F = dA + ½[A, A] = F^a T_a, and I write

  S₀[A] = ¼ ∫_M √g F^a_{μν} F^a_{μν} d⁴x  (indices raised with g).

The classical Euclidean action is S₀[A]/g₀², with g₀ the bare coupling.

**Homogeneous paths.** A *homogeneous path* is a smooth map q : ℝ/βℤ → ℝ^{3×3}, τ ↦ (q_{ka}(τ)). It defines the smooth
connection on the trivial bundle M × su(2)

  A_q = q_{ka}(τ) θᵏ T_a,  with no dτ component (temporal gauge).

Every quantity below depends on q only through A_q. The relation between this q and the q of Part IV / register line 8
(where F = 2gq and energies are in reduced units) is a fixed linear rescaling q ↦ c·g·q with c independent of q and τ.
The statements below are invariant under any such linear change; §4 Step 7 says what it gives in Part IV's coordinates.

**Peter–Weyl sectors.** SU(2)_L acts on M by left translation on the S³ factor. Left translations are isometries of
h_t (h_t is left-invariant), so of g, and they fix A_q (each θᵏ is left-invariant and q depends only on τ). Pull-back
therefore gives a unitary action of SU(2)_L on L² sections of every bundle built from TM and su(2). In the frame
(dτ, θ¹, θ², θ³) ⊗ (T_a), a section is a tuple of functions on M, and the action is the left-regular representation on
each coefficient. I define

  H₀ = the SU(2)_L-invariant sections = the sections whose frame coefficients are constant on S³ (sector j = 0),
  H_{≥1} = H₀^⊥ (sectors j ≥ 1),

and P₀ the orthogonal projection onto H₀ (average over SU(2)_L with Haar measure). This is the slow/fast split of
register line 13: slow = homogeneous, fast = Peter–Weyl sectors j ≥ 1.

**Operators.** D_v is Vassilevich's background-Feynman-gauge Yang–Mills operator (his eqs. (3.43)–(3.47), ξ = 1) on
su(2)-valued 1-forms on M, with background B = A_q. D_gh is his ghost operator (eq. (3.49)) on su(2)-valued functions.
Both are quoted exactly in §3.

**The regulated fast effective action.** For Λ > 1/R, define

  W_Λ[q] = −½ ∫_{Λ⁻²}^{R²} (ds/s) [ Tr_{H_{≥1}} e^{−s D_v} − 2 Tr_{H_{≥1}} e^{−s D_gh} ].

This is the proper-time form of ½ log det(D_v|_{≥1}) − log det(D_gh|_{≥1}), that is, of −log of the background-gauge
Gaussian integral over the fast fluctuations and fast ghosts (Vassilevich (3.51): Z_F = det(D)^{−1/2} det(D^{[gh]})).
The upper limit R² is an infrared cut. It makes W_Λ finite for every q, also when D_v|_{≥1} has zero or negative
modes, and it changes W_Λ only by a q-dependent finite amount that does not depend on Λ. Only the ultraviolet end
matters below.

---

## 2. Statement

**Theorem (KC1-ζ).** For every t > 0, every R > 0, every β > 0 and every homogeneous path q ∈ C^∞(ℝ/βℤ; ℝ^{3×3}):

(a) As Λ → ∞,

  W_Λ[q] = α₄ (ΛR)⁴ + α₂ (ΛR)² + α₁ (ΛR) − (b₀/8π²) log(ΛR) · S₀[A_q] + W_fin[q] + O((ΛR)⁻¹),  b₀ = 22/3,

where α₄, α₂, α₁ depend on (t, R, β) but **not on q**, W_fin[q] is finite, and the O((ΛR)⁻¹) term may depend on q.
In particular the only q-dependent divergence of the fast background-gauge effective action is the logarithm, and its
coefficient is exactly −(b₀/8π²) times the coupling-free classical action of the path.

(b) Writing out S₀ for a homogeneous path,

  S₀[A_q] = ∫₀^β dτ [ ½ G_{(ka)(lb)} q̇_{ka} q̇_{lb} + U_{t,R}(q(τ)) ],  G_{(ka)(lb)} = R V_t h_t^{kl} δ_{ab},

with U_{t,R}(q) = ¼ ∫_{S³} √g F^a_{ij} F^a_{ij} the magnetic energy of the homogeneous field q (a polynomial in q).
Hence the log-divergent part of W_Λ is

  −(b₀/8π²) log(ΛR) ∫ dτ [ ½ G q̇ q̇ + U(q) ],

with **one** coefficient for the kinetic and the potential term, no dependence on q or F, and no other velocity
structure (no q̈, no q̇⁴, no q-dependent kinetic metric) in the log.

(c) (Part IV coordinates.) If q is rescaled so that S₀[A_q]/g₀² = ∫ dτ (½|q̇|² + V(q)), as Part IV's reduced model
states (its Definition p4def:bo: the kinetic term ½|p_q|² is flat), then the log part of the one-loop Euclidean
effective Lagrangian is

  −g₀² (b₀/8π²) log(ΛR) [ ½ δ_{ik} q̇_i q̇_k + V(q) ],

that is, in the notation L = ½(δ_{ik} + 2m_{ik}) q̇_i q̇_k + …, the log part of the velocity coefficient is
2m^{log}_{ik} = −g₀² (b₀/8π²) log(ΛR) δ_{ik}, the same factor that multiplies V. With
1/g₀²(Λ) = 1/g²(R) + (b₀/8π²) log(ΛR), the sum (classical + one-loop) of both terms is finite with the single
renormalized coupling g(R).

The theorem holds at t = 1 as well, and for every F: no smallness of |F| and no positivity of D_v is used.

---

## 3. What I borrow (verified)

### 3.1 Vassilevich, the heat-kernel manual

**D. V. Vassilevich, "Heat kernel expansion: user's manual", Phys. Rept. 388 (2003) 279–360, arXiv:hep-th/0306138.**
Equations read on ar5iv (arXiv HTML) on 27–28 Nov 2026.

- **Standing hypothesis (§2.1, §4.1).** "Let us consider a smooth compact Riemannian manifold M without boundary." "Let D
  be an operator of Laplace type on V, and let f be a smooth function on M."
- **Laplace form (§2.1).** For D = −(g^{μν}∂_μ∂_ν + a^σ∂_σ + b) with scalar principal part, "There is a unique connection
  on V and a unique endomorphism E of V … so that D = −(g^{μν}∇_μ∇_ν + E)" (eq. (2.2)), with
  ω_δ = ½ g_{νδ}(a^ν + g^{μσ}Γ^ν_{μσ} I_V), E = b − g^{νμ}(∂_μω_ν + ω_νω_μ − ω_σΓ^σ_{νμ}), and curvature
  Ω_{μν} = ∂_μω_ν − ∂_νω_μ + ω_μω_ν − ω_νω_μ (eq. (2.10)).
- **Expansion (eqs. (2.21)–(2.22)).** Tr_{L²}(f e^{−sD}) ≅ Σ_{k≥0} s^{(k−n)/2} a_k(f, D), with
  a_k(f, D) = (4π)^{−n/2} ∫_M dⁿx √g b_k(x, x) f(x). Here ≅ is an asymptotic expansion as s → 0⁺: for every N the
  difference between the trace and the sum up to k = N is O(s^{(N+1−n)/2}).
- **ζ link (eq. (2.27)).** a_n(f, D) = ζ(0, f, D).
- **Manifolds without boundary (§4.1).** "Coefficients with odd index k vanish, a_{2j+1}(f, D) = 0", and
  (4.26) a₀(f, D) = (4π)^{−n/2} ∫ √g tr_V{f},
  (4.27) a₂(f, D) = (4π)^{−n/2} 6⁻¹ ∫ √g tr_V{f(6E + R)},
  (4.28) a₄(f, D) = (4π)^{−n/2} 360⁻¹ ∫ √g tr_V{f(60E_{;kk} + 60RE + 180E² + 12R_{;kk} + 5R² − 2R_{ij}R_{ij}
  + 2R_{ijkl}R_{ijkl} + 30Ω_{ij}Ω_{ij})}.
- **Yang–Mills operator (§3.4, on M with metric g; the quadratic action (3.43) contains the Ricci term, so it is the
  curved-space operator).** Background split A → B + A, gauge condition (3.44) ∇^μA^α_μ = 0 with ∇ containing both
  the metric and the background gauge part, gauge-fixing term of weight κ = 1 (Feynman gauge; "The case κ ≠ 1 yields a
  non-minimal operator"):
  (3.45) (ω_μ)^{αρ}_{νβ} = B^γ_μ c^α_{γβ} δ^ρ_ν − Γ^ρ_{μν} δ^α_β,
  (3.46) (E)^{αρ}_{νβ} = −R^ρ_ν δ^α_β + 2F(B)^{γρ}_ν c^γ_{βα},
  (3.47) (Ω_{μν})^{ασ}_{ρβ} = R^σ_{ρμν} δ^α_β + F(B)^δ_{μν} c^α_{δβ} δ^σ_ρ,
  (3.49) ghost connection (ω^{[gh]}_μ)^α_γ = B^β_μ c^α_{βγ}, the ghost operator being "just the ordinary scalar
  Laplacian" with this connection, so E^{[gh]} = 0 (stated at (4.33)),
  (3.51) Z_F(B) = det(D)^{−1/2} det(D^{[gh]}),
  (3.53) a^{tot}_k = a_k(D) − 2a_k(D^{[gh]}); "only this coefficient [a_n^tot] contains information on the one-loop
  divergences".
- **Flat-space evaluation (§4.2.1, "pure Yang–Mills theory in flat space").** (4.30) tr_V(E²) = 4F^δ_{ρν}F^γ_{ρν}K_{δγ};
  (4.31) tr_V(Ω_{ij}Ω_{ij}) = −4F^δ_{ρν}F^γ_{ρν}K_{δγ}; (4.32) K_{δγ} = c^δ_{αβ}c^γ_{αβ}; (4.33)
  tr_V(Ω^{[gh]}_{ij}Ω^{[gh]}_{ij}) = −F^δ_{ρν}F^γ_{ρν}K_{δγ}; (4.34) a₄^{[tot]} = (11/96π²) ∫ d⁴x √g F^δ_{ρν}F^γ_{ρν}K_{δγ},
  "We also recover the coefficient 11/3 which is familiar from computations of the Yang–Mills beta functions."

**Hypotheses check.** M = S³ × S¹_β with g = dτ² + R²h_t is smooth, compact, without boundary; A_q is smooth; D_v and
D_gh are Laplace type (Feynman gauge). So §2.1, (2.21)–(2.22), (4.26)–(4.28) and the odd-vanishing statement apply
verbatim, with f ≡ 1. **(4.30)–(4.34) are stated in flat space**, so they do not transfer by citation: Step 4 below
proves that on a curved M the F-dependent part of a₄^tot is still exactly (4.34). That is the one place where the
borrowed result needs an extra step, and the step is elementary.

### 3.2 Van den Heuvel: the same log on the round S³ (cross-check only)

**B. van den Heuvel, "Glueballs on the three-sphere", Nucl. Phys. B488 (1997) 282–334, arXiv:hep-lat/9608101,
doi:10.1016/S0550-3213(97)00016-3.** Read from the arXiv PDF.

- Setting: SU(2) on the round S³ (my t = 1), slow space the 18 left- and right-invariant modes (c, d), "We will impose
  the background gauge condition on the high-energy modes", Feynman gauge ξ = 1, dimensional regularization by "a
  laplacian term for an ε-dimensional torus of size L that we attached to our space S³".
- Eq. (91): 1/g_R² = 1/g₀² + 11/(12π²ε), "the standard result for SU(2) gauge theory in 1 + 3 dimensions … the
  renormalization is an ultra-violet effect and does not depend on the global properties of the space".
- Eq. (117): the one-loop contribution to the kinetic term ċᵃᵢċᵃᵢ + ḋᵃᵢḋᵃᵢ contains 11/(12ε) plus finite constants, and
  "The divergency will be absorbed by the same coupling constant renormalization of eq. (91) that also made the
  potential part finite." Eqs. (118)–(119): the scheme in which the renormalized kinetic term is exactly
  (2π²/2g_R²)(ċ² + ḋ²) fixes the finite part, κ₀ = 0.0566264741439181.
- His stated approximation: "We will neglect higher order time derivatives of B and also terms of the form BⁿḂ²". So his
  kinetic log is at B = 0 only.

I use this only as an independent check of the special case t = 1, F = 0 (§5). It is not a step of the proof.

---

## 4. Proof

**Step 1. The operators are Laplace type on a closed manifold.** [PC] By §1, M is smooth, compact and without boundary,
g is smooth, and B = A_q is a smooth connection (q is smooth in τ and θᵏ are smooth global forms). Vassilevich's
(3.43)–(3.47) are derived for a general Riemannian metric (the Ricci term in (3.43) comes from commuting two covariant
derivatives), with no field equation imposed on B. So D_v is Laplace type with the E and Ω of (3.46)–(3.47), and D_gh
is Laplace type with E^{[gh]} = 0 and Ω^{[gh]}_{μν} = F^δ_{μν} c_δ (adjoint action). The background is in temporal
gauge (B₀ = 0); nothing in §3.4 of Vassilevich restricts the gauge of B. As an independent check of (3.46)–(3.47): on
adjoint-valued 1-forms, the Weitzenböck formula gives (d_B d_B^* + d_B^* d_B) = ∇^*∇ + Ric + 2 ad F (up to the sign
convention of ad F), which is −(∇² + E) with E = −Ric + 2F·c, and the bundle curvature of the Levi-Civita ⊗ adjoint
connection is Riem ⊗ 1 + 1 ⊗ ad F. The Feynman-gauge operator is exactly this Hodge-type Laplacian.

**Step 2. The operators are block-diagonal in j.** [elem] D_v and D_gh are built naturally from g and B. Left translation
L_h (h ∈ SU(2)) is an isometry of g and L_h^*A_q = A_q, so the pull-back U_h commutes with D_v and D_gh (naturality of
∇, Ric, Riem, F under isometries preserving B). Hence P₀ = ∫ U_h dh commutes with both operators and with their heat
semigroups, and

  Tr_{L²} e^{−sD} = Tr_{H₀} e^{−sD₀} + Tr_{H_{≥1}} e^{−sD_{≥1}},  D₀ := D|_{H₀}, D_{≥1} := D|_{H_{≥1}},

for D ∈ {D_v, D_gh} and every s > 0; all three traces are finite because e^{−sD} is trace class on a closed manifold.

**Step 3. The j = 0 block has no s⁰ term and no q-dependent divergence.** [elem + PC]
H₀ consists of sections whose coefficients in the frame (dτ, θᵏ) ⊗ (T_a) depend on τ only. The map
u ↦ (coefficients) identifies H₀ isometrically (up to the constant factor R³V_t in the inner product, which does not
change the operator) with L²(ℝ/βℤ; ℂ^N), N = 12 for D_v (4 form components × 3 colours) and N = 3 for D_gh. On such
sections every frame derivative e_k acts as zero, so D₀ is a second-order operator in τ alone. Its principal symbol is
g^{00}ξ₀² = ξ₀², a scalar times the identity. By Vassilevich's Laplace-form lemma (§2.1) D₀ is a Laplace-type operator
on the closed 1-manifold ℝ/βℤ with the flat metric dτ². (Explicitly, since B₀ = 0 and Γ^ρ_{0ν} = 0 on the product,
D₀ = −∂_τ² + M(τ) with M a smooth matrix function, but only the Laplace type is used.) Apply (2.21) with n = 1 and the
odd-vanishing statement:

  Tr_{H₀} e^{−sD₀} ≅ s^{−1/2} (b₀ + s b₂ + s² b₄ + …),  b₀ = (4π)^{−1/2} N β  (by (4.26)).

There is no s⁰ term. The leading coefficient b₀ does not depend on q. (b₂ does depend on q through tr M, but it comes
with s^{+1/2} and gives no divergence; see Step 5.)

**Step 4. The four-dimensional coefficients on curved M.** [elem, with (4.28) and (3.46)–(3.47) cited] Take n = 4.

*Odd coefficients.* a₁ = a₃ = 0 (closed M).

*a₀.* tr_V{1} = 12 for the vector, 3 for the ghost: independent of q.

*a₂.* By (4.27) it is (4π)⁻² 6⁻¹ ∫ √g tr_V(6E + R). From (3.46),
tr_V E = −R^ρ_ρ δ^α_α + 2F^{γρ}_ρ c^γ_{αα} = −3R + 0, because F^{γρ}_ρ = 0 (F is antisymmetric) and c^γ_{αα} = 0 (c is
antisymmetric). For the ghost, E = 0. So a₂^tot does not depend on q.

*a₄.* Write E = E_R + E_F with E_R = −Ric ⊗ 1 and E_F = 2F·c, and Ω_{ij} = Ω^R_{ij} + Ω^F_{ij} with Ω^R = Riem ⊗ 1 and
Ω^F = 1 ⊗ F_{ij}·c. Go through the terms of (4.28) with f ≡ 1:

- E_{;kk} and R_{;kk} are divergences; their integrals over the closed M vanish.
- tr(RE) = R tr E = −3R², independent of q (computed above).
- tr(E²) = tr(E_R²) + 2 tr(E_R E_F) + tr(E_F²). The cross term is
  tr(E_R E_F) = −2 R^ρ_ν F^{γν}_ρ c^γ_{αα} = 0, because c^γ_{αα} = 0 (and also because R^ρ_ν F^{γν}_ρ = 0, symmetric
  against antisymmetric). tr(E_R²) is pure curvature. tr(E_F²) is the flat-space quantity (4.30): it is a pointwise
  algebraic trace over (vector index, colour index), whose value 4F^δ_{ρν}F^γ_{ρν}K_{δγ} does not use flatness. (On a
  curved M, indices are contracted with g, which is all (4.30) requires.)
- tr(Ω_{ij}Ω_{ij}) = tr(Ω^RΩ^R) + 2 tr(Ω^RΩ^F) + tr(Ω^FΩ^F). The cross term factorizes:
  tr(Ω^R_{ij}Ω^F_{ij}) = R^σ_{σij} · F^δ_{ij} c^α_{δα} = 0 · 0. tr(Ω^FΩ^F) is (4.31), again a pointwise trace.
- R², Ric², Riem² do not involve q.
- Ghost: Ω^{[gh]} = F·c, E^{[gh]} = 0, so the only q-dependent part of a₄(D_gh) is 30 tr(Ω^{[gh]}Ω^{[gh]}), which is (4.33).

So the q-dependent part of a₄^tot = a₄(D_v) − 2a₄(D_gh) is

  (4π)⁻² 360⁻¹ [ 180·4 + 30·(−4) − 2·30·(−1) ] ∫ √g F^δ F^γ K_{δγ}
  = (4π)⁻² (5/3 + 1/6) ∫ √g FFK = (11/96π²) ∫_M √g F^δ_{ρν} F^γ_{ρν} K_{δγ},

which is (4.34), now proved on the curved M. With K = 2δ for su(2):

  a₄^tot = (11/48π²) ∫_M √g F^a_{μν} F^a_{μν} + c_R = (11/12π²) S₀[A_q] + c_R = (b₀/8π²) S₀[A_q] + c_R,

with c_R a pure-curvature constant (depends on t, R, β, not on q), since b₀/8π² = (22/3)/(8π²) = 11/(12π²).

*Numerical check of the algebra (mine, small).* With random antisymmetric F^γ_{μν}, symmetric Ric and a random tensor
with the antisymmetries of Riem, I built (3.46) and (3.47) as explicit 12 × 12 matrices for su(2) and checked:
tr E has no F part; tr(E²) and Σ tr(Ω_{ij}Ω_{ij}) split exactly into an F-only and a curvature-only part (no cross term);
tr(E_F²)/FFK = 4, Σ tr(Ω^FΩ^F)/FFK = −4, ghost/FFK = −1; the combination gives 11/6 = 16π² · 11/96π². (Python, a few
lines; no heavy computation.)

**Step 5. The regulated fast action.** [elem] By Step 2,

  Tr_{H_{≥1}} e^{−sD_v} − 2 Tr_{H_{≥1}} e^{−sD_gh}
  = [Tr e^{−sD_v} − 2 Tr e^{−sD_gh}] − [Tr_{H₀} e^{−sD_{v,0}} − 2 Tr_{H₀} e^{−sD_{gh,0}}].

By Steps 3 and 4 and the asymptotic form of (2.21) (truncated at a fixed finite order), as s → 0⁺ this equals

  s⁻² a₀^tot + s⁻¹ a₂^tot + a₄^tot − s^{−1/2} b₀^tot − s^{1/2} b₂^tot + O(s),

where b^tot = b(D_{v,0}) − 2b(D_{gh,0}), b₀^tot = (4π)^{−1/2}(12 − 6)β, and a₀^tot, a₂^tot, b₀^tot do not depend on q.
(The O(s) remainder collects a₆ s and b₄ s^{3/2}.) Insert into W_Λ and integrate from Λ⁻² to R²:

- s⁻² a₀^tot → −¼ a₀^tot (Λ⁴ − R⁻⁴), s⁻¹ a₂^tot → −½ a₂^tot (Λ² − R⁻²), s^{−1/2} b₀^tot → +b₀^tot (Λ − R⁻¹):
  these give α₄(ΛR)⁴ + α₂(ΛR)² + α₁(ΛR) + const, with q-independent α's;
- a₄^tot → −½ a₄^tot · log(R²Λ²) = −a₄^tot log(ΛR) = −(b₀/8π²) log(ΛR) S₀[A_q] − c_R log(ΛR);
- s^{1/2} b₂^tot → +½ b₂^tot ∫_{Λ⁻²}^{R²} s^{−1/2} ds = b₂^tot (R − Λ⁻¹): finite, plus a q-dependent O((ΛR)⁻¹);
- the O(s) remainder is integrable at s = 0; its integral from Λ⁻² equals its integral from 0 plus O(Λ⁻²).

Moving the q-independent −c_R log(ΛR) into the α-terms (it is a field-independent vacuum-energy log), this is (a).
The ζ form is the same statement: for positive D on H_{≥1}, ζ_{tot,≥1}(0) = a₄^tot − 0 − dim ker = (b₀/8π²)S₀[A_q] +
const by (2.27) applied blockwise, so the μ-dependence −ζ_{tot,≥1}(0) log(μR) carries the same coefficient.

**Step 6. Kinetic and potential parts of S₀[A_q].** [elem] In temporal gauge, with A_q = q_{ka}(τ)θᵏT_a,

  F = q̇_{ka} dτ∧θᵏ T_a + F_sp(q(τ)),  F_sp(q) = (q_{ka} dθᵏ + ½ ε_{abc} q_{kb} q_{lc} θᵏ∧θˡ) T_a,

where F_sp has no dτ leg. The components F_{0i} and F_{ij} are orthogonal in |F|², so
¼|F|² = ½ g^{ij}F^a_{0i}F^a_{0j} + ¼ F^a_{ij}F^a_{ij}. In the frame θ, g^{kl} = R⁻² h_t^{kl} on the S³ factor and
√g d⁴x = R³ dτ dvol_{h_t}. Since q̇ is constant on S³,

  ∫_{S³} R³ dvol_{h_t} ⋅ ½ R⁻² h_t^{kl} q̇_{ka} q̇_{la} = ½ R V_t h_t^{kl} q̇_{ka} q̇_{la},

and the spatial part integrates to U_{t,R}(q(τ)), a polynomial in q that does not involve q̇. This proves the formula
for S₀[A_q] in (b), with constant G. Substituting it into (a) gives the log part
−(b₀/8π²) log(ΛR) ∫ dτ [½ G q̇q̇ + U(q)]: the same coefficient on both terms, independent of q and F. Because a₄^tot is
exactly quadratic in F with no derivatives of F, the log contains no other velocity structure. This is (b).

**Step 7. Part IV coordinates and renormalization.** [elem, given Part IV's normalization] Part IV's reduced model has
kinetic term ½|p_q|² (flat) and potential V, and line 13 states that the homogeneous sector of the Yang–Mills
Hamiltonian is H_red. The kinetic metric G is constant and V is the restriction of the magnetic energy, so after a
constant linear change of variables (absorbing R, V_t, h_t^{kl}, the factor g₀, and the common reduced-unit factor ½)
S₀[A_q]/g₀² = ∫(½|q̇|² + V). Then (a) and (b) read W_Λ ⊃ −g₀²(b₀/8π²) log(ΛR) ∫(½|q̇|² + V), which is (c). The
classical plus one-loop Euclidean Lagrangian is
(1/g₀² − (b₀/8π²) log(ΛR)) · g₀²(½|q̇|² + V) + finite, and with 1/g₀²(Λ) = 1/g²(R) + (b₀/8π²) log(ΛR) the bracket is
1/g²(R) for both the kinetic and the potential term. ∎

---

## 5. Cross-check against van den Heuvel

At t = 1 and F = 0 (his B = 0), the theorem predicts the same divergence coefficient for the kinetic and the potential
term. Van den Heuvel finds, in dimensional regularization, the pole 11/(12π²ε) in 1/g² from the potential (eq. (91))
and a kinetic pole absorbed "by the same coupling constant renormalization of eq. (91)" (eq. (117)). In dimensional
regularization the 1/ε pole plays the role of my log(ΛR), and 11/12π² = b₀/8π² for SU(2). So his result is the
t = 1, F = 0 case of (b). Two differences, neither affecting the log: his slow space is 18-dimensional (left- and
right-invariant modes) where mine is the 9 left-invariant ones, and his scheme is dimensional, not proper-time. By the
argument of Step 3, removing any finite-rank set of spatial modes that the operator preserves changes no log, so the
agreement is expected. My theorem extends his kinetic statement to every t and every F (he neglected BⁿḂ² terms).

---

## 6. What this does not give

1. **Not the Coulomb-chart masses (R3, open).** KC1 in H2 §6 is a statement about m = m^NA + m^C: the Born–Oppenheimer
   mass of the transverse fast oscillators plus the Coulomb term from the fast Gauss law, in the Coulomb chart of the
   Hamiltonian. The theorem above is about W_Λ, the background-Feynman-gauge determinant ratio, which contains the time
   component a₀, the mixed a₀–aᵢ terms through F_{0i} = q̇, and the ghosts. **Step (e) of the proposed route does not
   work as stated.** R2 (H2-borrowed-routes §4) proves that m^NA equals the low-frequency q̇² coefficient of the
   Euclidean effective action *of the transverse Gaussian fast fibre H_f(q)*. It says nothing about the background-gauge
   W_Λ. To pass from W_Λ to m^NA + m^C I still need R3: that the two gauge fixings of the same fast Gaussian integral
   give the same q̇² coefficient up to finite terms, including that the Coulomb-chart slow coordinate picks up no log
   field renormalization (in background gauge the background is renormalized only through g, by Abbott's theorem; in
   Coulomb gauge that is not automatic).
2. **Not my spatial cutoff (R4, open).** The regulator here is four-dimensional proper time (equivalently ζ). H2's
   admissible regulator is χ(K(F)/Λ²R²) on the spatial fast spectrum, with time continuous. The log coefficient is
   expected to be regulator-independent (Hasenfratz–Hasenfratz and Karsch on the flat lattice; line 14's numerical b₀
   with my regulator for the potential half), but that transfer is not proved here.
3. **Not the power divergences under my regulator.** In this scheme all power divergences (α₄, α₂, α₁) are
   q-independent. That is a property of the covariant proper-time regulator and of tr E having no F part. It does not
   prove the analogous statement for Φ and ⟨Ω|W₄|Ω⟩ under the spatial cutoff (H2 §4 Step 3).
4. **Not the finite parts.** W_fin[q], and hence m^fin and the finite speed-of-light renormalization c₀, are not
   computed, and their analyticity in F is not proved. The F-dependent finite kinetic term (van den Heuvel's neglected
   BⁿḂ² terms) is what line 15's sign needs; it remains a finite one-loop sum on the Berger spectrum.
5. **Not the O((ΛR)⁻¹) remainder of KC1.** The O((ΛR)⁻¹) here comes from b₂^tot of the j = 0 block and is
   q-dependent. KC1's remainder under the spatial cutoff is a separate question.
6. **Only Feynman gauge, only one loop.** I use ξ = 1, where both operators are Laplace type. I do not claim the
   off-shell q̇² log is ξ-independent, and I make no statement beyond one loop.
7. **Only S³ × S¹_β with f ≡ 1.** On S³ × ℝ the same statement should hold for the local density by locality of heat
   coefficients, but I have not written that argument with a cited theorem, so it is not part of the claim.
8. **Not H2.** Nothing here touches uniformity in Λ, large fields, or the multiscale step (H2 §4 Steps 1, 5). KC1-ζ is
   the one-loop algebraic input that says the electric and magnetic couplings carry one log. The uniform spectral
   theorem with infinitely many fast modes remains the open content of H2.

**Status I would record (for Bee to decide; I have not edited the register).** KC1-ζ as stated in §2: [D], from
Vassilevich (cited, hypotheses matched; the flat-space step (4.30)–(4.34) re-proved on curved M in Step 4) plus
elementary Steps 2, 3, 5–7. KC1 as worded in H2 §6: [Cd] on R3 and R4.

---

## Sources

- D. V. Vassilevich, "Heat kernel expansion: user's manual", Phys. Rept. 388 (2003) 279–360, arXiv:hep-th/0306138.
  Read on ar5iv: §2.1 (Laplace form, (2.2), (2.10)), (2.21)–(2.22), (2.27), §3.4 ((3.43)–(3.47), (3.49), (3.51),
  (3.53)), §4.1 (hypotheses, odd coefficients vanish, (4.26)–(4.28)), §4.2.1 ((4.30)–(4.34)).
- B. van den Heuvel, "Glueballs on the three-sphere", Nucl. Phys. B488 (1997) 282–334, arXiv:hep-lat/9608101,
  doi:10.1016/S0550-3213(97)00016-3. Read from the arXiv PDF: §3.1–3.2 (approximations, background and Feynman gauge),
  the ε-torus regularization, eqs. (90)–(91), (116)–(119).
- Context only, not used as proof steps: L. F. Abbott, Nucl. Phys. B185 (1981) 189; A. Hasenfratz and P. Hasenfratz,
  Nucl. Phys. B193 (1981) 210; F. Karsch, Nucl. Phys. B205 (1982) 285 (as checked in H2-borrowed-routes.md §2).
