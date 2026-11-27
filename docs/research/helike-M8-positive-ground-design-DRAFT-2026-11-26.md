<!-- Produced by an adversarial-review-and-design workflow on 26 November 2026 (Claude, at Bee's request to press forward). DRAFT design only: nothing here is registered. Claims are labelled VERIFIED / derived / recalled / UNVERIFIED as in the memo. -->
# DRAFT design memo: a ground state for M8 that is positive by construction

26 November 2026. **Design only.** I changed no repository file, ran no solver, and computed no M8 quantity. I ran two small scratch checks; their script is `/Users/bdavis/.claude/jobs/c8f0f39d/tmp/wf/psd/wcheck.py`, and both are marked VERIFIED where they appear below. Everything else is derived on paper, recalled from the literature, or proposed. Each claim is labelled.

**One correction to the computed task text.** It writes V_eff = ½ g∇S·∇S − ½Δ_g S + V. For ψ = e^{−S} the sign is the other way round, and the Laplacian must carry the measure:

  E_loc := (Hψ₀)/ψ₀ = V − ½ g^{ij}∂ᵢS∂ⱼS + ½ Δ_μ S,  with Δ_μ S = ρ⁻¹∂ᵢ(ρ g^{ij}∂ⱼS) and dμ = ρ dz.

Check in one dimension with S = wx²/2: E_loc = ½w²x² − ½w²x² + ½w = w/2. The task's sign gives w²x² − w/2, which is not constant. A registered mutant below catches this sign error.

---

## 0. Bottom line

- **Where the skew goes.** At strong coupling the vacuum is lopsided. In the Ritz ground, the lopsidedness sits in a polynomial amplitude, and a polynomial that bends a Gaussian has to change sign somewhere. I propose moving it into the exponent: ψ₀ = e^{−S}.
- **Primary family (B, chart-native).** χ₀ = gauss(z)·e^{−Q(x)}. Q is a charge-0 polynomial in the Hermite variables x = √w z, with a guaranteed positive quartic guard (§2.1). It keeps the Ritz space's own Gaussian envelope, so the solver's excited states stay compatible in the tails, and every M8 integrand becomes free of division and of exponentials.
- **Geometric family (A, cone-native), a cross-check and the Q4 reading.** ψ₀ = det(q)·e^{−P(G)}, with P an SO(2)-invariant polynomial in G. This is the q-space picture with the Dirichlet face built in.
- **How S is set.** By minimising the energy over its parameters (the variational principle). The residual ‖(H − E)ψ₀‖ is the spread of E_loc and is reported. Weinstein, Temple and Eckart supply the floating comparisons. A test at the controls checks that e^{−S} belongs to the same operator realization as the Ritz space.
- **What it buys for M8.** f is defined everywhere. The 1/ψ₀² spike mechanism behind amendment 5 disappears. R gets a clean reading as the excited state's average of E_loc − E₀. Q4 has a closed form, and a structural split, described in §4.4.

---

## 1. The picture first

- **The tube.** The field is a 3×3 matrix q ∈ ℝ⁹. Colour O(3) acts on its right index, and G = qqᵀ folds ℝ⁹ onto the cone of positive Gram matrices. The condensate is one point q_bg ~ 1/g, so its colour orbit is a 3-dimensional ring of radius ~1/g in ℝ⁹. The vacuum is a tube around that ring. The six Gram coordinates are the tube's cross-section.
- **The fold is exact (VERIFIED).**
  - The reduced measure dμ = det(G)^{−1/2} dG is the pushforward of flat Lebesgue measure on q.
  - The density w = dμ/dvol_g of amendment A1.1 is exactly the O(3)-orbit volume: w = 8·∏_{i<j}(λᵢ + λⱼ)^{1/2}, where λ are the eigenvalues of G. My check reproduced it at five random q to relative ~10⁻¹⁴.
  - The kinetic coefficient A_ij = ∇_q Gᵢ·∇_q Gⱼ is, by construction, the inverse metric of the quotient of flat ℝ⁹ by O(3). In the literature this is the Bures–Wasserstein geometry; that name is a label I am recalling.
  - So (H, dμ) is exactly −½Δ_q + V restricted to colour-invariant functions. The note's "folded drum" is literally flat space folded by colour.
- **Why the vacuum is lopsided.**
  - The stiffness A grows linearly with G: on the outer side of the tube (larger G) the vacuum is wider, and on the inner side it is narrower.
  - A one-dimensional analogue, −½ a x ∂² + ½k(x − x₀)², has a Gamma-like ground state, skewed toward large x. Its log is a cubic in (x − x₀), not a quadratic. (Derived; this is the basis of prediction P5.)
  - In the log-Cholesky chart z, the large-G side is an exponential wall, because V ~ g²G² ~ e^{4u}. The small-G side is a face at infinite chart distance.
  - So the true vacuum in z is a skewed drop. A polynomial times a Gaussian can only skew a Gaussian by adding odd polynomial terms, and those go negative on one side. Truncated expansions of a decaying exponential change sign in the same way that Taylor polynomials of e^{−x} acquire real roots.
  - **Conjecture (not shown):** the nodal surface the probe found is this truncation effect, and it should move outward as Mq grows.
- **The face of the cone.**
  - The face det G = 0 is the hypersurface det q = 0 inside ℝ⁹, which separates the two sheets.
  - The Ritz basis is Gaussian in u = log r. It vanishes faster than any power there, so its form closure is the **Dirichlet** realization, H¹₀. (The face has positive capacity: in the normal singular value σ the form is ½∫|∂_σψ|²dσ, flat and one-dimensional.)
  - The Dirichlet ground state vanishes linearly in distance, ψ₀ ∝ det q × (positive), i.e. a factor (det G)^{1/2}.
  - **det q is harmonic in ℝ⁹** (it is multilinear, so Δ_q det q = 0). That makes the face factor free in the local energy: for ψ = det q·e^{−P(G)},

      E_loc = V − ½A(∂P,∂P) + ½A:∂²P + 5 tr(∂P/∂G),

    where 5 = 3 (from b = Lap_q G = (6,6,6,0,0,0), halved) + 2 (from the cross term ∇log det q·∇P = 2 tr P_G).
  - The cross term was VERIFIED by finite differences in q-space: 5.518068 against 5.518068, a relative difference of 6·10⁻⁸, limited by the finite differences.
  - Harmonic limit: S = ½tr(ΩG) − ½log det G gives E_loc ≡ 2.5 tr Ω, which is the Dirichlet (one-quantum-per-row) ground. Derived, and consistent with counting.
  - The real q-space vacuum need not vanish at the face. The Dirichlet choice is the solver's unresolved `boundary_status`. M8 must compare like with like, so any e^{−S} must sit in the same realization (§3.4).

---

## 2. The representation

### 2.1 Family B: chart-native (primary for M8)

In the solver's half-density picture (χ = √ρ ψ, x = √w z):

  χ₀(z) = N · gauss(x) · e^{−Q(x)}, equivalently S(z) = ½zᵀWz + ½ℓ·z + Q(√w z) + const.

So "p₀ → e^{−Q}": the polynomial factor of the Ritz ground is replaced by a positive exponential with a polynomial exponent.

**The families.** All monomials are charge-0 at leading order. The blocks are H (charge 2, h = x_H1 + i x_H2), S (charge 1, s), and C and R (charge 0).

- **F₂** (7 terms): x_C, x_R, x_C², x_C x_R, x_R², |h|², |s|².
- **F₃** = F₂ plus the cubics and a quartic guard (21 terms):
  - cubics: x_C³, x_C²x_R, x_C x_R², x_R³, x_C|h|², x_R|h|², x_C|s|², x_R|s|², Re s²h̄, Im s²h̄;
  - guard: x_C⁴, x_R⁴, |h|⁴, |s|⁴, with coefficients ≥ γ_min.
- **F₄** = F₃ plus all remaining charge-0 quartics (about 35 terms). The quartic part is written as mᵀΛm over the 21 quadratic monomials m. The guard certificate is Λ ⪰ 0 plus a diagonal-quartic part ≥ γ_min. This is a sufficient sum-of-squares condition for Q to be bounded below.
- The relative doublet phase decides whether Re s²h̄ or Im s²h̄ is reflection-even. Both are kept; prediction: one fits to zero.

**Automatic, with a two-line proof.**
- ψ₀ > 0 everywhere. With the guard, Q ≥ Q_min > −∞.
- **Normalisable.** ∫χ₀²dz = ∫gauss²e^{−2Q} < ∞.
- **Finite energy and finite variance.** g^{ij}(z), its derivatives and V(z) are finite sums of exp(k·u)·poly(u₃,u₄,u₅). Since u is linear in z, a Gaussian times e^{−2Q} dominates every term by completing the square. The same argument handles all integration-by-parts boundary terms at |z| → ∞.
- **Cone boundary.** The face sits at u_i → −∞, i.e. |z| → ∞. ψ₀ vanishes there faster than any power of det q, so it meets the Dirichlet condition of the Ritz realization. Physically it is too steep: Gaussian in log r rather than linear in det q. The cost is about the face weight, which is reported.

**Still to be shown (UNVERIFIED).**
- **(a) Closure.** χ₀ lies in the closure of the Hermite span in the form norm of the Ritz H. I expect this, because χ₀ has Gaussian or faster decay while the form weights grow only like exp(c|z|). It is not proved. The projection seam in §3.4 checks it numerically.
- **(b) Charge purity.** The charge rotation acts nonlinearly in the Cholesky chart, so Q is only approximately invariant. The leak is measured exactly: Lψ₀ = −(LS)ψ₀, so ‖Lψ₀‖² = ⟨(LS)²⟩_{ψ₀²}. The existing Ritz ground has the same approximate invariance; N2's charge compression is evidence of that.

### 2.2 Family A: cone-native (cross-check, face physics, Q4)

  ψ₀ = (det G)^{1/2} · e^{−P(y)},  with y = M⁻¹(G − G₀) the linear-Gram normal coordinates.

P uses the same invariant monomial list as F₂ to F₄, but in y (exactly invariant under SO(2)₁₂ in G). Optionally it adds γ√det G, which matches the √det term of V.

- **Normalisable** iff the leading form of P is positive on the closed PSD cone minus 0. For degree 2 it suffices that the quadratic form be positive definite. Otherwise it is a floating sampled check on the compact base {tr G = 1}.
- **Integrable at the face.** ψ₀²dμ ~ det^{1/2}·det^{−1/2}dG.
- **Weakness.** Tails: P of degree ≥ 2 in G decays like e^{−cG²}, while the WKB tail is ~gG^{3/2}, and the Ritz excited states decay only like a Gaussian in log G. So f = ψ/ψ₀ grows super-exponentially, and spurious tail energy appears in D, with an opposite R (§5, risk 7).
- **Quadrature.** It needs its own frame (the dressed chart from A's own moments). The Gauss–Hermite ratio e^{−2S_A}ρ_z/gauss grows on the face side.
- **Role.** Exploratory and gluing only. In the bulk (vacuum weight > 10⁻⁶), S_A and S_B must agree up to a constant: two routes to one vacuum.

---

## 3. Determining S as the ground state of the same operator

### 3.1 The objective (energy, decided)

Use the existing first-order kinetic form, with no Laplacian and no exponentials of growing sign. At the Gauss–Hermite nodes x_n with weights W_n:

  m_n = W_n e^{−2(Q_n − Q_min)},  q̂_n = √w⊙(−∂ₓQ − x) − ½ℓ  (= −∂_z S),
  ε_n = ½ q̂ₙᵀ G_n q̂_n + V_n,
  E[Q] = Σ m_n ε_n / Σ m_n.

The gradient over the coefficient c_k of basis function φ_k is closed form:

  ∂E/∂c_k = Σ m[ −q̂ᵀG(√w⊙∂ₓφ_k) − 2(ε − E)φ_k ] / Σm.

- **Method.** L-BFGS, or the linear method, which is a Ritz problem in span{ψ₀, φ_kψ₀} iterated to self-consistency. Its fixed point is Galerkin stationarity: ⟨φ_kψ₀, (H − E)ψ₀⟩ = 0 for every k.
- **Starts.**
  - (i) Q = 0, the bare Ritz envelope.
  - (ii) A weighted least-squares fit of −log p₀^Ritz on nodes where the vacuum weight exceeds 10⁻⁶. That is the region amendment 5 found stable, where p₀ > 0. This is an initializer only, never the answer.
- **Why energy and not variance.** The energy is an upper bound, and the realization test relies on that ordering. The variance is the diagnostic.

### 3.2 The residual (reported)

Second-order local energy, closed form in the chart:

  E_loc = V − ½g∂S∂S + ½[g:∂²S + (∂ᵢg^{ij} + g^{ij}ℓᵢ)∂ⱼS],

with ∂_zS = Wz + ½ℓ + √w⊙∂ₓQ and ∂²_zS = W + diag(√w)∂²ₓQ diag(√w). ∂ᵢg^{ij} comes from analytic derivatives of the Laurent monomials in `_symbolic_metric`, so no finite differences are needed.

  σ² := ‖(H − E)ψ₀‖²/‖ψ₀‖² = Σm(E_loc − E)²/Σm.

**Ground IBP seam.** Σmε = Σm E_loc in the continuum. Their quadrature difference tests boundary terms and the derivative code.

### 3.3 Comparisons (floating unless stated)

- **Weinstein (rigorous given exact integrals).** The spectrum meets [E − σ, E + σ].
- **Variational ordering.** E[S] ≥ E₀^true, and E_Ritz(Mq) ≥ E₀^true, for the same realization. Neither dominates the other in general. Where the Ritz is converged (the controls), E[S] < E_R would be a bug or a realization leak (§3.4).
- **Temple.** E₀^true ≥ E − σ²/(ε₁ − E), but only with a lower bound ε₁ ≤ E₁^true. The Ritz E₁ is an upper bound, so this stays floating unless a registered lower bound exists. The repo has Temple code (`helike-lab/crates/helike-quantum/src/temple.rs`); I did not check what lower bound it uses.
- **Eckart.** 1 − |⟨ψ₀^S, ψ₀^true⟩|² ≤ (E[S] − E₀)/(E₁ − E₀). It is reported with Ritz estimates, so it is floating.

### 3.4 Same realization: tests that e^{−S} is in the Ritz world

- **Projection seam.** Project ψ₀^S onto the Hermite space at Mq = 4 and 6 now, and at 8 and 10 when the CPU allows. Two things must converge: the L² deficit must go to 0, and the Rayleigh quotient of the projection must approach E[S]. A plateau above E[S] means e^{−S} is outside the closure.
- **Realization test (controls only, where the Ritz is converged).** Require E[S] ≥ E_R(Mq_max) − max(10⁻⁹Δ, 10·|E_R(10) − E_R(8)|).
- Family A with the face factor removed (β = 0, Neumann-like) is the mutant this test is built for. It is only catchable on a synthetic case with O(1) face weight; at the registered points the face weight is tiny.

### 3.5 How close is close enough for M8's residual term

With ψ normalised and E₀ = E[S]:

  R = ⟨f²ψ₀, (H − E₀)ψ₀⟩ = ⟨ψ, (E_loc − E₀)ψ⟩,  so  |R| ≤ σ_ψ := ⟨(E_loc − E₀)²⟩_{|ψ|²}^{1/2}.

R is the tilt of the local-energy landscape between where the excitation lives and where the vacuum lives. Its ground-weighted mean is 0 by definition.

The share error is at most about |R|/D. So:
- **Keep** the registered flag: |R|/Δ > 10⁻² means ground-state limited.
- **Add:** a verdict whose deciding share lies within |R|/Δ of its threshold (0.5, or J's 0.2) is OPEN.
- **Target:** |R|/Δ ≤ 10⁻³, which matches the share seam tolerance.

The bound σ_ψ is preferred to the Cauchy–Schwarz form ‖f‖²_{L⁴(ψ₀²)}σ. With a quartic Q, f = p e^{Q} need not be in L⁴, so that form can be vacuous.

---

## 4. What changes in M8

### 4.1 f, D and R become free of division and of exponentials

f = p·e^{Q}, and ∇_z f = √w e^{Q}(dp + p∂ₓQ). Write v̂ = √w⊙(dp + p∂ₓQ). Then:

  D = ½ ΣW v̂ᵀ G v̂,
  R = ΣW[ ½(p²q̂ + 2p v̂)ᵀ G q̂ + (V − E₀)p² ].

Since q_a = v̂ + p q̂, the sum D + R equals ½ΣW q_aᵀGq_a + ΣW(V − E₀)p² **node by node**. So (★) becomes a pointwise algebraic identity in this form.

- **Q0a keeps its registered role.** It checks that the quadrature reproduces the solver's ⟨ψ,(H − E₀)ψ⟩, with the same 10⁻⁴ tolerance.
- **The integration-by-parts content moves to a new seam (R-IBP).** R in first-order form must equal ΣW p²(E_loc − E₀) in second-order form.
- **The amendment-5 mechanism disappears.** There is no 1/ψ₀², so the ragged-edge spike is gone. I would keep the ε = 10⁻¹² share region as registered for continuity, and report full-domain shares beside it.
- **E₀ = E[S] in (★).** Report the grid's Δ = E_k − E_R0 beside E_k − E[S]. Their difference is E_R0 − E[S].
- **Orthogonality.** The constant part of f carries no gradient, so the shares do not depend on the overlap ⟨ψ_C, ψ₀^S⟩. That overlap is reported. For Helike, orthogonality holds up to the charge leak.
- **Overflow.** Only e^{−2(Q − Q_min)} ≤ 1 ever appears. Q_min comes from the guard certificate plus a local minimisation.

### 4.2 Q2 must be amended (a registration change)

The solver's §5.3 dressed overlap uses ψ₀^Ritz, so the 10⁻⁶ gluing tolerance no longer applies. I would split it:
- **Q2a (internal, decided).** ⟨f, ȳ⟩_{ψ₀²} = ΣW p ȳ e^{−Q} computed two ways, to 10⁻¹⁰.
- **Q2b (reported).** Against the solver's dressed overlap, with 1 − |⟨ψ₀^S, ψ₀^R⟩|² beside it.

### 4.3 Q3

Q3 is unchanged in form, using the trial span {ȳψ₀^S} and the same residual correction. It contains no 1/ψ₀.

### 4.4 Q4 in closed form

With wψ₀² = χ₀²(det g_ij)^{−1/2} and −log χ₀² = zᵀWz + 2Q + const:

  Ric_g − Hess_g log(wψ₀²) = Ric_g + Hess_g(zᵀWz + 2Q) + ½ Hess_g log det(g_ij).

The ℓ·z terms cancel exactly. Hess_g = ∂² − Γ∂, with Γ from analytic Laurent derivatives. This replaces A1.1's finite differences.

**Structural reading (exploratory, partly recalled).**
- In G coordinates the operator is the colour quotient of flat ℝ⁹, with w the orbit volume (VERIFIED, §1).
- Lott (2003, Bakry–Émery tensor and Riemannian submersions), as I recall it: if the total space has Ric ≥ 0, the base with fibre-volume measure has Ric_g − Hess_g log w ⪰ 0 on the free stratum. **RECALLED, UNVERIFIED.** It must be checked numerically at the nodes before anyone uses it.
- If it holds, then BE ⪰ 2Hess_g S.
- Horizontal geodesics project to geodesics, so Hess_g S(X,X) equals the Euclidean Hessian of S(qqᵀ) along horizontal lifts. (Standard.)
- **Face term (derived here).** For horizontal H (qᵀH symmetric): Hess_q(−log det q)(H,H) = ‖N^{−1/2}(qᵀH)N^{−1/2}‖² ≥ 0, with N = qᵀq. So the Dirichlet factor only adds curvature.
- The curvature bound would then reduce to the vacuum's convexity in horizontal field directions.
- This stays floating and decides nothing (A1.2).

---

## 5. Risks, and the seam or test that catches each

| # | Risk | Caught by |
|---|---|---|
| 1 | S family too narrow | σ/Δ and R share reported; the family seam (F₃ vs F₄ shares ≤ 10⁻³) fails, so OPEN, with one pair and no ladder |
| 2 | e^{−S} in a different realization (face, Neumann) | Realization test at the controls; projection seam; β = 0 synthetic mutant |
| 3 | Quadrature error | Order seam per the amendment-4 rule, on E, σ and shares; ground IBP seam; R-IBP seam |
| 4 | exp overflow or underflow | Guard certificate (Q bounded below); log-domain evaluation with the Q_min shift; chart \|u\| ≤ 300 refusal kept; synthetic with Q_min = −800 must give finite values |
| 5 | Optimizer local minimum | Two starts must agree in E (≤ 10⁻¹⁰Δ) and in bulk S up to a constant (weighted rms ≤ 10⁻⁶); stationarity ‖∇_cE‖ ≤ 10⁻⁹ |
| 6 | S inherits Ritz artefacts | The fit is an initializer only; mutant "fit without the variational step" fails stationarity or gives higher E |
| 7 | Tail mismatch (f growing where Ritz ψ is heavy) | Family B shares the Ritz envelope by design; for A, the R share and the S_A–S_B bulk gluing |
| 8 | Charge leak in the chart | ⟨(LS)²⟩ gated (tolerance in §6 is a guess) |
| 9 | Shares move because the denominator changed, not the physics | Control gluing: S-ground shares vs the registered Ritz-ground control shares agree to 10⁻³ |
| 10 | Overclaiming | e^{−S} is not the exact ground. Shares mean "the Dirichlet energy in the e^{−2S} geometry plus R". Verdicts within \|R\|/Δ of a threshold are OPEN |
| 11 | Sign or measure errors in E_loc | Harmonic synthetic (E_loc constant); mutants (½\|∇S\|² sign flipped; Δ_μ → Δ_g) |

---

## 6. DRAFT registration text (not registered; Bee decides)

> ### Amendment 6 (DRAFT, not registered): a ground state that is positive by construction
>
> **Why.** At s = 1.18, t = 3, g² = 0.03 the saved Ritz ground changes sign inside the physical cone, with a witness root where min eig G = 46.6. f = ψ/ψ₀ is singular there. I replace the denominator, not the excited states and not the operator.
>
> **The representation.** χ₀ = N·gauss(x)·e^{−Q(x)}, where gauss is the solver's own envelope. Q is in the nested families F₃ ⊂ F₄ of charge-0 monomials in the H, C, S, R blocks. Q carries a quartic guard that certifies it is bounded below (Λ ⪰ 0, diagonal ≥ γ_min = 10⁻⁶). A certificate failure is a refusal. The excited states are the solver's states, unchanged.
>
> **Fixing Q.** Q minimises E[Q] = ⟨ψ₀, Hψ₀⟩/⟨ψ₀, ψ₀⟩ with the solver's metric, measure and Laurent potential, at the fine quadrature order of the amendment-4 rule. Two starts: Q = 0, and a fit of −log p₀^Ritz on nodes with vacuum weight > 10⁻⁶. Convergence: ‖∇_cE‖ ≤ 10⁻⁹. Both starts must agree to 10⁻¹⁰Δ in E, and to weighted rms 10⁻⁶ in bulk S up to a constant. Otherwise the point is OPEN. The decided family is F₄.
>
> **Reported at every point:** E[S]; E_R0; σ = ‖(H − E)ψ₀‖; the Weinstein interval; Eckart's floating bound; 1 − |⟨ψ₀^S, ψ₀^R⟩|²; the charge leak ⟨(LS)²⟩; the face weight; the excluded fraction; full-domain shares.
>
> **Seams (each OPEN on failure; one pair, no ladder):**
> - order seam (n_c, n_f): E changes ≤ 10⁻⁸Δ, σ ≤ 10⁻³ relative, shares ≤ 10⁻³;
> - ground IBP: |Σmε − Σm E_loc| ≤ 10⁻⁸Δ;
> - R-IBP: |R₁ − R₂| ≤ max(10⁻³|R|, 10⁻⁸Δ);
> - family seam F₃ vs F₄: shares ≤ 10⁻³;
> - Q0a: 10⁻⁴, unchanged;
> - Q2a: 10⁻¹⁰;
> - charge leak: ⟨(LS)²⟩ ≤ 10⁻⁴ (a guess, not derived).
>
> **Realization test (controls).** E[S] ≥ E_R(10) − max(10⁻⁹Δ, 10|E_R(10) − E_R(8)|). The projection seam at Mq = 4, 6, 8, 10: the L² deficit and the Rayleigh-quotient gap to E[S] both decrease, with the last step within 10× of the previous one.
>
> **Verdict guard.** A deciding share within |R|/Δ of its threshold makes the point OPEN. The residual-share flag stays at 10⁻².
>
> **Predictions at the four controls:**
> - P1: all seams pass;
> - P2: σ/Δ ≤ 10⁻³ and the residual share ≤ 10⁻³;
> - P3: own shares agree with the registered control shares (0.99562 to 0.99969) to 10⁻³;
> - P4: E[S] − E_R ∈ [−10⁻⁹Δ, +10⁻⁵Δ];
> - P5 (geometric, derived from A ∝ G): the fitted cubic skews the vacuum toward larger tr G, so the third moment of tr G under ψ₀² is positive;
> - P6: one of Re s²h̄ or Im s²h̄ has a coefficient below 10⁻⁸;
> - P7: charge leak ≤ 10⁻⁶.
>
> **At the strong witness (saved Mq = 12; no new eigensolve):**
> - positivity holds by construction (not a prediction);
> - Q0a ≤ 10⁻⁴;
> - no prediction for the shares or the verdict;
> - conjecture: the Ritz nodal surface lies where ψ₀^S carries per-node weight ≤ 10⁻⁹ of the global norm, and it moves outward with Mq (cheap check at Mq ≤ 6).
>
> This does not repair the unchanged 0.95 reference requirement.
>
> **Synthetic tests:**
> - harmonic in the chart: Q* = 0, σ = 0, E = Σw/2;
> - a planted lopsided vacuum: pick S* ∈ F₃ and build V = E + ½|∇S*|² − ½Δ_μS* (the inverse ground-state transformation). The optimizer recovers S* to 10⁻⁸ with σ ≤ 10⁻¹⁰, and a small-Mq Ritz solve of the same V shows the sign-change mechanism;
> - q-space Dirichlet harmonic (family A): S = ½trΩG − ½log det G gives E_loc ≡ 2.5 trΩ;
> - a one- or two-dimensional anharmonic case with a fine-grid exact ground: the Weinstein interval contains it;
> - closed-form w: the chart's w equals 8∏(λᵢ + λⱼ)^{1/2}.
>
> **Mutants that must be caught:**
> - the sign of ½|∇S|² flipped in E_loc;
> - Δ_μ replaced by Δ_g (the measure dropped from the Laplacian);
> - the face factor removed in family A on the O(1)-face synthetic (β = 0);
> - |ψ₀^Ritz| substituted for e^{−S};
> - S from the fit without the variational step;
> - the guard removed;
> - the Q_min shift removed (overflow);
> - e^{−2Q} dropped from the normalisation;
> - different E₀ in D and R;
> - Q2b used as a gate.
>
> **Scope.** The sourced homogeneous reduction, in the Ritz (Dirichlet) realization only. Q4 stays floating and decides nothing.

---

## 7. Status of claims

- **VERIFIED** (scratch, `/Users/bdavis/.claude/jobs/c8f0f39d/tmp/wf/psd/wcheck.py`):
  - w = det G^{−1/2}·det A^{1/2} = 8∏_{i<j}(λᵢ + λⱼ)^{1/2}, the orbit volume, at five random q to about 10⁻¹⁴;
  - the det q cross term in E_loc equals 2 tr(∂P/∂G), by finite differences.
- **Derived on paper, not run:**
  - the local-energy formulas;
  - the pointwise form of (★);
  - |R| ≤ σ_ψ;
  - the harmonic Dirichlet energy 2.5 trΩ;
  - horizontal convexity of −log det q;
  - the cancellation of ℓ in the Q4 formula;
  - the capacity argument (the Ritz closure is Dirichlet at the face).
- **Recalled, unverified:** Lott's submersion inequality; the "Bures–Wasserstein" label; the Weinstein, Temple and Eckart statements. These three are standard.
- **Conjectures and guesses:** the truncation origin of the nodal surface and its outward drift with Mq; all numeric predictions in §6 except where marked derived; the charge-leak tolerance.