# Lehmann–Goerisch for S_C: registration of the frame, with the base problem as a gate

25 November 2026. **REGISTRATION of method and decision rules, before any implementation or computation.** It
follows the [S_C design](helike-sc-proof-design-2026-11-25.md), whose Route 1 (crude IMS) failed its feasibility
estimate. It registers what is solid now. It registers the base problem of the homotopy as an explicit gate with
feasibility criteria, because that is where this route can fail. Nothing here has been computed.

## 1. Reduction: S_C follows from a count in the full singlet operator (proved)

Let H be the colour-singlet operator (all charges), with discrete spectrum by Theorem A, and let
ρ_C = E₀ᶜ + 2ω_H − ω_min/4 as frozen.

**Lemma R.** If H has at most four eigenvalues below ρ_C (that is, λ₅(H) ≥ ρ_C), then S_C holds.

**Proof.** The spectrum of H is the union of its charge sectors, with multiplicity. Below ρ_C there are already, by
results proved on the controls:
- at least one eigenvalue of charge +2 and one of charge −2: the Weinstein windows of the projected Helike states
  lie below ρ_C, E_H + R_H < ρ_C;
- at least two of charge 0: Lemma 3.

That makes four, from three different sectors. If H has at most four in total, charge 0 has at most two below ρ_C,
which is S_C. ∎

The check E_H + R_H < ρ_C uses existing enclosures and is part of the evaluation. This reduction removes every
charge projector from the Lehmann step: the trial functions are cut states ηψ_k, colour singlets of no fixed
charge.

## 2. The Lehmann–Goerisch inequality (to be verified against the literature before implementation)

Let H be self-adjoint and bounded below, with discrete spectrum λ₁ ≤ λ₂ ≤ … below the essential spectrum. Let
u₁, …, u_n ∈ D(H) be linearly independent, and suppose ρ ≤ λ_{N+1}. Form

    A₀ = (⟨u_i, u_j⟩),   A₁ = (⟨u_i, Hu_j⟩),   A₂ = (⟨Hu_i, Hu_j⟩),

and the eigenproblem (A₁ − ρA₀)x = τ(A₂ − 2ρA₁ + ρ²A₀)x, with the right-hand matrix positive definite. Let
τ₁ ≤ τ₂ ≤ … be its eigenvalues. Then, for every i with τ_i < 0 and i ≤ N,

    λ_{N+1−i} ≥ ρ + 1/τ_i.

This is Lehmann's theorem, in the form used by Goerisch, Behnke and Plum. The exact index convention and its
hypotheses will be checked line by line against Behnke–Goerisch (1994) and Plum (1991) before any code is written,
and recorded in an amendment if they differ. Goerisch's extension, which replaces A₂ by an auxiliary inner product,
is not needed here: A₂ is computable (§3).

**Use.** Take N + 1 = 12. The target is λ₅ ≥ ρ_C. The base must supply ρ_b ≤ λ₁₂. The trial space must contain the
ground state, the Helike pair, the competitor and the two-quantum cluster near E₀ + 2.17–2.21, about seven states.
That way the bound on λ₅ is close to λ₅ itself.

## 3. Interval matrices for cut trial functions (registered formulas)

Let u_k = ηψ_k with ψ_k a saved state, and work with the shifted operator H − σ (any exact σ) to keep entries
small. Write R_k = (H − σ)ψ_k, and use the existing upper bounds:
- t_k ≥ ‖1_ext ψ_k‖, the Arb exterior mass, which bounds all of ψ_k outside the plateau ball;
- ℓ_k ≥ ‖[H, η]ψ_k‖;
- r_k(σ) ≥ ‖R_k‖, from the interval residual with centre σ.

Then:
- **A₀:** ⟨u_i, u_j⟩ = ⟨ψ_i, ψ_j⟩ − ⟨ψ_i, (1−η²)ψ_j⟩. The first term is exact (Hermite orthonormality). The second
  is at most t_i t_j.
- **A₁:** ⟨u_i, (H−σ)u_j⟩ = ⟨ψ_i, R_j⟩ − ⟨ψ_i, (1−η²)R_j⟩ + ⟨ηψ_i, [H,η]ψ_j⟩. The first term is the exact linear
  cross moment (polarization of the linear moment code). The second is at most t_i r_j, the third at most ℓ_j.
- **A₂:** ⟨(H−σ)u_i, (H−σ)u_j⟩ = ⟨R_i, R_j⟩ − ⟨R_i, (1−η²)R_j⟩ + (terms in [H,η]ψ). The first term is the exact
  centred cross moment (polarization of the residual code). The commutator terms are bounded by r_i ℓ_j + ℓ_i r_j +
  ℓ_i ℓ_j.
- **One new enclosure is required for A₂:** τ_k ≥ ‖1_ext R_k‖, the exterior norm of the residual, giving
  |⟨R_i, (1−η²)R_j⟩| ≤ τ_i τ_j. It is an Arb majorant of the same type as `metric_tail`, with second derivatives and
  the potential's Laurent terms added. Without it, A₂ would carry errors of order r_i r_j, which is too crude.

The positive-definiteness of A₂ − 2ρA₁ + ρ²A₀ is checked on the interval matrices. The eigenproblem is solved with
verified interval linear algebra. A rigorous enclosure of the negative τ_i is required, for example by inclusion
over the interval matrix pencil. Floating eigenvalues are not enough.

## 4. The base problem: the gate

Lehmann needs a rigorous ρ_b ≤ λ₁₂(H) with ρ_b clearly above λ₅. Here that means about E₀ + 2.2 to 2.4. That is the
hard part. Candidates, with order-of-magnitude feasibility stated in advance:

| Candidate | Idea | Expected status |
| --- | --- | --- |
| B1: global minorant | H ≥ −½Δ + ½κ\|q\|² − S⁴/(2g²) (Theorem A), an explicit oscillator | **Infeasible.** Starts about 8.7e3 below the well at g² = 0.004; the homotopy would have to climb thousands of units |
| B2: Neumann boxes | Domain decomposition with a constant potential minorant per box (Plum) | **Infeasible.** In 6 to 9 dimensions the base has of order 10⁴–10⁵ states below the target |
| B3: Neumann bracketing in a tube, polynomial model inside | H ≥ H_in^N ⊕ H_out^N. Outside, V − V_bg ≥ ρ_C-level, which needs a tube radius of about 4 and an Arb global bound. Inside, a polynomial anharmonic model in flat transverse coordinates, with exact Hermite moments, plus an orbit-curvature correction | **Open.** No localization error; but the inside model's Neumann boundary, the curvature constant and the non-monotone cubic all need work |
| B4: keep S_C as a hypothesis | The ordering stays "conditional on S_C" | Always available |

**Gate G-base.** Before any Lehmann code is written, a floating-point feasibility estimate of B3 at the controls
must show:
- a tube radius at which the outside bound clears ρ_C + 0.05;
- an inside lower bound for λ₁₂ of H_in^N, from a homotopy over the polynomial model with an explicit base, above
  E₀ + 2.2.

If either fails, this route stops and B4 stands. That outcome is reported as a result, not a defect.

## 5. Decision rule (once G-base passes and the code exists)

Per control:
- **S_C PROVED** if all three hold:
  - the windows check of Lemma R passes;
  - the enclosed base gives ρ_b ≤ λ₁₂;
  - the Lehmann bound on λ₅ is strictly above ρ_C, in interval arithmetic.
- **UNRESOLVED** otherwise.

There is no "S_C false" outcome from this rule. A failure to prove is not a disproof. When S_C is PROVED at a
control, the ordering there is unconditional up to the stated arithmetic.

## 6. Contract and order of work

1. Verify the statement of §2 against Behnke–Goerisch and Plum, and amend if needed.
2. Run the G-base feasibility estimate for B3, then report pass or stop.
3. Only if it passes:
   - register the base's constants and enclosures (Arb global bound outside the tube; inside model and homotopy
     steps);
   - implement τ_k;
   - implement the cross-moment polarizations and the verified pencil solver.

   Each step gets RED tests and mutants as before. The mutants must include: dropping t_i t_j, r-corrections or τ_i
   τ_j; using floating eigenvalues; skipping positive-definiteness; claiming PROVED without Lemma R's windows; and
   using ρ_b without its enclosure.
4. Evaluate at the four controls, and only there.

## 7. Honest expectation

The reduction (§1) and the matrix enclosures (§3) are solid and mostly reuse existing code. The base problem (§4) is
the crux. Standard global bases are ruled out by the depth of the well (about 1/g²) and the dimension (6 to 9). B3
is a real research step, not an implementation detail. It is quite possible that S_C stays a stated hypothesis at
these couplings. If so, the ordering result keeps its current form: proved conditional on S_C, with S_C supported by
a large numerical margin and a global-minimum search.

References: N. J. Lehmann (1949/50); F. Goerisch and H. Haunhorst (1985); H. Behnke and F. Goerisch, *Inclusions
for eigenvalues of selfadjoint problems* (1994); M. Plum, *Bounds for eigenvalues of second-order elliptic
differential operators*, ZAMP 42 (1991). Statement details are to be confirmed per §6.1.

---

## 8. Amendment, 25 November: statement verified; G-base estimate run

This section follows steps 6.1 and 6.2. It changes the registration in three places. It is written before any
Lehmann code exists.

### 8.1 The Lehmann statement (step 6.1): verified, with one index correction

**Source.** Behnke, Mertins, Plum and Wieners, *Eigenvalue inclusions via domain decomposition*, Proc. R. Soc.
Lond. A 456 (2000) 2717–2730, Theorem 2.1. I read it as restated in T. Vejchodský, *Three methods for two-sided
bounds of eigenvalues – a comparison* (arXiv:1602.08961), Theorem 1 and equation (9). That is Goerisch's general
form, with a shift γ and a flux σ. Taking the auxiliary space to be L² itself, with σ replaced by Hu (allowed
because Hu ∈ L² for our trial functions), gives exactly the statement of §2.

**Independent proof.** Here is the argument for our form, so that the paper does not rest on the literature alone.
Suppose ρ ∉ σ(H) and the spectrum below ρ is discrete, which holds by Theorem A. Put w = (H − ρ)u with u ∈ U. Then

    ⟨w, (H−ρ)⁻¹w⟩ / ⟨w, w⟩ = ⟨u, (H−ρ)u⟩ / ‖(H−ρ)u‖² = xᵀ(A₁−ρA₀)x / xᵀ(A₂−2ρA₁+ρ²A₀)x,

so the τ_i are the Ritz values of the bounded self-adjoint operator (H − ρ)⁻¹ on the space (H − ρ)U. The denominator
is positive definite exactly when the (H − ρ)u_i are independent. The negative eigenvalues of (H − ρ)⁻¹ below its
other spectrum are μ_k = 1/(λ_{M+1−k} − ρ), where M is the number of eigenvalues below ρ. Min–max gives τ_i ≥ μ_i.
If τ_i < 0, there are therefore at least i eigenvalues below ρ, and 1/τ_i ≤ λ_{M+1−i} − ρ. If ρ ≤ λ_{N+1}, then
M ≤ N, so λ_{N+1−i} ≥ λ_{M+1−i} ≥ ρ + 1/τ_i. If ρ is itself an eigenvalue, apply this at ρ − ε and let ε → 0; the
pencil is continuous in ρ while its right-hand side stays positive definite. The cited theorem also allows
ρ ≤ λ_{N+1}. ∎

Every hypothesis holds for cut trial states. ηψ is smooth with compact support, so it lies in D(H). It also lies
in the domain of any Neumann restriction H_in^N whose region contains supp η, which matters for §8.3.

**Index correction.** The harmonic singlet levels relative to E₀, the same at both controls, are:

    0 | 1.0901, 1.0901, 1.1067 | 2.1802 ×3, 2.1968 ×2, 2.2133 | 2.4297 ×2 | 2.6777 …

The two-quantum cluster is **six** levels (λ₅–λ₁₀), not seven, and λ₁₁ = λ₁₂ is the degenerate stiff-mode pair.
**Amended: N + 1 = 11.** The base must supply ρ_b ≤ λ₁₁, with ρ_b between λ₁₀ and λ₁₁, registered as ρ_b = E₀ +
2.32. The trial space is the 10 states λ₁–λ₁₀. The base step then has a margin of about 0.11 on each side. That is
tighter than S_C's own margin of 0.27, and is stated here as a risk.

### 8.2 Correction to the G-base outside criterion

Bracketing gives λ_k(H) ≥ min(λ_k(H_in^N), inf H_out^N). For the base we need λ₁₁(H) ≥ ρ_b, so **the outside region
must clear ρ_b, not ρ_C + 0.05**, as §4 wrongly said. **Amended accordingly.** Clearing ρ_C + 0.05 is enough only
for a direct count at ρ_C.

### 8.3 G-base estimate (step 6.2), floating point, not a bound

This uses `scripts/estimate_helike_sc_gbase.py` (sha256 7a9c2110…); the logs are in `research/sc-gbase-20261125/`.
It ran on CPU only, at the background controls s = 1.15 and t = 3.

**Checks along the way:**
- C₃ is 0.0750 at g² = 0.004 and 0.0530 at g² = 0.002, matching Route 1.
- The quartic remainder is exactly (g²/2)|cof Nz|² ≥ 0. Its sampled minimum is 0 to rounding.
- The singlet kinetic term on a transverse slice is at least the flat one: the inverse metric's zz block is a
  Schur-complement inverse, and so is ≥ I.
- Orbit-curvature terms on the tube are V_J ≈ −1×10⁻³ and Robin β ≈ 0.04–0.06. Both are negligible.

**Outside.** m(r) is the minimum of V − V_bg on the shell at distance r from the orbit. In every sample the
minimiser's distance to the orbit was exactly r.

| r | 3 | 4 | 5 | 6 | 8 | 10 | 13 | 17 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| g² = 0.004 | 4.56 | 7.58 | 11.06 | 14.84 | 22.78 | 30.46 | 40.11 | 47.70 |
| g² = 0.002 | 4.83 | 8.20 | 12.22 | 16.78 | 27.03 | 38.14 | 54.88 | 74.57 |

Here ρ_b − V_bg is 7.73 at both controls, and ρ_C + 0.05 is 7.36–7.37. The tube radius needed to clear ρ_b is
about 4.1 at g² = 0.004 and below 4 at g² = 0.002. Beyond r = 17, the Route 1 global search found nothing below
V_bg + 20 except the background orbit. In the product tube of §8.4, the weak directions are all soft: the soft-block
minima equal the shell minima above, and the stiff block clears ρ_b from L_v ≈ 1.7. **Outside condition: PASS at
both controls.**

**Inside, 6-ball tube.** The only rigorously explicit base on a 6-ball is radial:
½ω²_min r² − C₃r³ + min V_J, with Neumann and Robin conditions. It is far too weak.

| | L = 4 | L = 5 | L = 6 |
| --- | --- | --- | --- |
| g² = 0.004: base λ₁₁ / levels below ρ_b | 4.26 / 1464 | 4.44 / 2541 | 4.44 / 4213 |
| g² = 0.002: base λ₁₁ / levels below ρ_b | 4.75 / 923 | 4.80 / 483 | 4.80 / 462 |

The homotopy would have to start from hundreds to thousands of levels. **The 6-ball version of B3 fails.**

### 8.4 Variant B3′: a product tube with a separable Neumann base

Cut the tube as a product of balls, B_u(L_u) × B_v(L_v), in the soft (u) and stiff (v) normal blocks. Then
½zᵀKz − C₃|z|³ ≥ ½ω²_s,red|u|² + ½ω²_t,red|v|², with ω²_red = ω²_min,block − 2C₃√(L_u² + L_v²). That is a sum of two
isotropic 3-d Neumann oscillators on balls, solvable by radial equations, so it is a valid explicit base.

| (L_u, L_v) | g² = 0.004: base λ₁₁ / levels below ρ_b | g² = 0.002: base λ₁₁ / levels below ρ_b |
| --- | --- | --- |
| (4.25, 2) | 6.45 / 57 | 6.95 / 23 |
| (4.5, 2) | 6.36 / 68 | 6.92 / 23 |
| (5, 2) | 6.13 / 87 | 6.84 / 32 |
| (4.5, 2.5) | 6.24 / 68 | 6.99 / 23 |

ρ_b − V_bg is 7.73 throughout.

**Reading.** The explicit base sits 0.8–1.6 below ρ_b. It has 23–87 levels below ρ_b, against the 10 we need. The
homotopy has to climb from there to the true polynomial potential, undoing the cubic reduction. That is a homotopy of
tens of levels, which is the scale of published Plum–Behnke computations, not thousands.

There is a usable structure. The base eigenfunctions (radial × spherical harmonics in each block) satisfy the
Neumann condition. They lie in D(H_s) for every step s, because the perturbation is a bounded polynomial on a bounded
domain. They therefore form one fixed Galerkin and Lehmann basis for the whole homotopy, and every matrix entry is a
polynomial integral against them. Late steps can switch to cut Hermite states (§8.1).

A wall-clearance choice is needed. With L_u ≈ 4.1–4.25, the soft wall's minimum is about 7.6–8.2, close to ρ_b, and
the Neumann problem could carry wall states below ρ_b. **Amended: L_u = 5, L_v = 2.** At L_u = 5 the wall minimum is
11–12, which keeps the base at 87 levels (g² = 0.004) and 32 levels (g² = 0.002).

### 8.5 Gate verdict

- **(i) Outside: PASS** at both controls, with the corrected criterion of §8.2 and with margin at L_u = 5.
- **(ii) Inside: NOT DEMONSTRATED.**
  - The 6-ball base fails (§8.3).
  - The product base B3′ is explicit and of tractable size (§8.4).
  - The registered criterion, an inside lower bound on λ₁₁ above ρ_b *from a homotopy*, can only be estimated by
    running that homotopy in floating point. That has not been done.

By the rule of §4, nothing has failed, so the route does not stop. Nothing has passed outright either. **Amended
order of work:** before any rigorous code, a floating-point Lehmann homotopy dry run on B3′ at L_u = 5, L_v = 2, at
both controls.
- **It passes** if the last step gives λ₁₁ of the inside model at least ρ_b, using at most 40 homotopy steps and at
  most 2,000 basis functions.
- **Otherwise** B4 stands: S_C stays a stated hypothesis.

The honest expectation from §7 is unchanged, with two updates. The base problem is now a concrete computation rather
than an open question. Its margin of 0.11 is the tightest number in the chain.

---

## 9. Amendment, 25 November: the dry run's method, registered before it is run

This section fixes how the floating-point dry run of §8.5 is done and judged. It is written before the run. The
script is `scripts/dryrun_helike_sc_lehmann_homotopy.py`.

### 9.1 The homotopy

On the product tube B_u(5) × B_v(2), with the Neumann condition and the Robin term −β/2 on each block's sphere
(as in the G-base script):

    H_s = −½Δ + W₀ + v_J + s·D,    D = P − W₀,    0 ≤ s ≤ 1.

- P(z) = V(x₀ + Nz) − V_bg is the exact quartic on the normal slice, fitted by least squares; the fit error is reported.
- W₀ = ½ω²_s,red|u|² + ½ω²_t,red|v|² is the B3′ base; v_J is the sampled minimum of the Jacobian potential.
- C₃ is the larger of the G-base script's value and a maximisation of the fitted cubic form.
- D ≥ 0 on the tube makes every eigenvalue non-decreasing in s. Its sampled minimum is reported. A value below −10⁻⁹
  means the monotonicity argument fails at this C₃, and the dry run is then reported as FAIL (model), not run.

### 9.2 Basis and matrices

- The basis is the products of exact block eigenfunctions of H₀, the lowest at most 2,000, cut at a level boundary.
  Each block uses a radial Galerkin in r^l P_k(2r/L − 1), k < 44, times real spherical harmonics.
- These functions satisfy the Robin condition, so they lie in D(H_s) for every s.
- A₂ uses H_sφ_I = E_Iφ_I + sDφ_I with the exact moments of the degree-8 polynomial D², not products of truncated
  matrices.
- Checks reported: the base count below ρ_b reproduces §8.4 (87 and 32); block orthonormality; and radial convergence
  (k < 44 against k < 56).

### 9.3 The step rule

- **Step 0.** The exact base spectrum supplies lower bounds ℓ_j = E_j at s = 0.
- **Next pair.** After each step, the next pair is (N′, ρ′) = (j − 1, ℓ_j), with ℓ_{N+1} = ρ, for the candidate
  j ≥ 11 that lets the next step go furthest in s.
- **Admissibility.** A step to s′ is admissible while the Ritz value satisfies μ_{j−1}(H_s′) ≤ ℓ_j − Δ.
  - Ritz values are tracked on 401 equally spaced s and up to index 200.
  - **Δ = 0.02.**
- **The step.** The step goes to the furthest admissible grid point. It applies Lehmann at (s′, ρ′, N′), with the N′
  lowest Ritz vectors as trial functions. Positive definiteness of A₂ − 2ρA₁ + ρ²A₀ is checked by Cholesky.

### 9.4 Verdict, per control

- **PASS** as soon as a step gives ℓ₁₁ ≥ ρ_b. Monotonicity then gives λ₁₁(H₁) ≥ ρ_b.
- **FAIL** in any of these cases:
  - 40 steps are used;
  - no candidate advances;
  - s = 1 is reached without ℓ₁₁ ≥ ρ_b.
- **Diagnostic, not a verdict.** If the Ritz value μ₁₁(H₁) is below ρ_b, then λ₁₁ of the inside model is below ρ_b.
  B3′ at this tube then cannot pass by any homotopy. This is reported.
- **Sensitivity.** Runs at Δ = 0.01 and 0.05 are reported as sensitivity only. The verdict is the Δ = 0.02 run.

A PASS is floating-point feasibility only. The rigorous version still needs everything in §6.3, plus enclosed C₃,
β and v_J.

### 9.5 Correction after a crashed first run (no verdict reached)

The first run at g² = 0.004 stopped at step 1 on an indexing error in a log line, so no verdict was reached. The run
showed a defect in the rule of §9.3.

**The defect.** Choosing the candidate that reaches furthest in s picked j = 11 at step 0 (N′ = 10). N′ < 11 can never
yield ℓ₁₁, and the index only decreases, so the rule could steer the homotopy into a dead end that has nothing to do
with whether the base problem is feasible.

**The floor.** §8.1 already records λ₁₁ = λ₁₂ as a degenerate pair. The run's Ritz values at s = 1 confirm it
(μ₁₁ = μ₁₂ = E₀ + 2.3528, μ₁₃ = E₀ + 2.6278). So a useful final ρ must sit above λ₁₂, which needs N′ ≥ 12.

**Amended:** candidates are restricted to j ≥ 13. Nothing else changes: Δ = 0.02, the same verdict rule, and the same
limits of 40 steps and 2,000 functions.

**Seen before this amendment** (reported, not a verdict):
- the model checks: quartic fit 7.7 × 10⁻¹³; minimum of D on the tube 2.5 × 10⁻¹⁴;
- the basis: 1,986 functions;
- the s = 1 Ritz value μ₁₁ − ρ_b = +0.033 at g² = 0.004. This is the whole margin the final Lehmann bound has to fit inside.

---

## 10. Results, 25 November: dry run FAIL at both controls under the registered rule

**Sources.**
- Script `scripts/dryrun_helike_sc_lehmann_homotopy.py` (sha256 aa87ad32…, at commit 81b46182).
- Logs in `research/sc-lehmann-dryrun-20261125/`.
- **The g² = 0.004 verdict run** was made at 6ef30399. The later basis-cut fix does not touch that control: its basis
  is the same 1,986 functions, and the sensitivity runs reproduce it.
- **Run order.** Run 1 crashed (§9.5). Run 2 gave the g² = 0.004 verdict, then stopped at an assertion that the
  g² = 0.002 basis was incomplete, before any step. The block cut was widened, and that control was rerun.

### 10.1 Model checks (both controls)

| | g² = 0.004 | g² = 0.002 |
| --- | --- | --- |
| quartic fit, max error on fresh points | 7.7e-13 | (same order) |
| C₃ (script = fitted cubic) | 0.07496 | 0.0530 |
| min of D on the tube | 2.5e-14 (origin) | 1.7e-14 |
| basis (cut, levels of H₀ below ρ_b) | 1,986 (ρ_b + 4.70; 87) | 1,938 (ρ_b + 6.26; 32) |
| radial convergence, block orthonormality | 1.2e-4, 3.6e-10 | 2.8e-4, 8.3e-10 |
| Ritz at s = 1 minus E₀, μ₁ … μ₁₃ | −0.011 · 1.077 ×2, 1.086 · 2.156 … 2.176 (six) · **2.353 ×2** · 2.628 | −0.011 · 1.078 ×2, 1.091 · 2.163 … 2.190 (six) · **2.356 ×2** · 2.626 |
| **μ₁₁(H₁) − ρ_b** | **+0.033** | **+0.036** |

The inside model's levels sit about 0.08 below the harmonic levels of §8.1, because of the Neumann/Robin walls and
v_J. So the whole margin for λ₁₁ ≥ ρ_b is about 0.03, not 0.11 as §8.1 estimated. The inside model passes
its necessary condition, but narrowly.

### 10.2 Homotopy

| Δ | g² = 0.004 | g² = 0.002 |
| --- | --- | --- |
| **0.02 (verdict)** | **FAIL: stuck at s = 0.1475 after 2 steps** (N 23 → 20, ρ − E₀ 1.29 → 1.23) | **FAIL: stuck at s = 0.610 after 3 steps** (N 23 → 13 → 12, ρ − E₀ 2.22 → 2.17 → 2.04) |
| 0.01 (sensitivity) | stuck at s = 0.155 after 2 steps | stuck at s = 0.590 after 2 steps |
| 0.05 (sensitivity) | stuck at s = 0.130 after 2 steps | stuck at s = 0.598 after 3 steps |

The Lehmann step itself behaves well:
- the right-hand matrix was positive definite at every step;
- every trial state gave a negative τ;
- ℓ₁₁ sat 10⁻⁴ to 4 × 10⁻² below μ₁₁, with residuals of 5 × 10⁻³ to 5 × 10⁻².

**Verdict, by §8.5 and §9.4: FAIL at both controls. B4 stands. S_C stays a stated hypothesis**, and the ordering
stays proved conditional on S_C.

### 10.3 What failed: the step rule, not (yet shown) the base

The steps fail the same way at every Δ, and the cause is structural:
- The new ρ′ = ℓ_j needs a Lehmann bound at index j. That exists only while N ≥ j.
- The greedy "reach the largest s" rule drops the index at once (87 → 23 at step 1), and then to 13 and 12.
- Once N = 12, the only candidate left is j = 13 with ρ′ = ρ. ρ is then frozen, and the homotopy stops as soon as
  μ₁₂ climbs to ρ − Δ.
- At g² = 0.004 a dense band near E₀ + 1.2 leaves no clean gap above any index, so the rule stalls even earlier.

A homotopy that reaches s = 1 has to keep N well above 12 until late, giving up reach per step for ρ that keeps rising.
The registered rule never makes that trade. So this FAIL settles the rule as registered. It does not show that no
admissible path exists for B3′.

**Not done here; would need its own registration before any run:**
1. A planner over the Ritz curves (dynamic programming over (s, N)) that decides whether any admissible index path to
   ℓ₁₁ ≥ ρ_b exists, using μ as a proxy for ℓ and Δ as the margin. If none exists even with exact proxies, B3′ is
   ruled out, not just this rule.
2. A larger margin at the end. The 0.03 margin of §10.1 is the binding number. It could be widened by a larger tube
   (smaller wall effect) or by a sharper base than the block-isotropic reduction, for example keeping the
   anisotropic w² in the soft block.

---

## 11. Registration, 25 November: exhaustive path search, before it is run

This follows item 1 of §10.3. Script: `scripts/plan_helike_sc_lehmann_path.py`. The model and basis are imported
unchanged from the dry-run script.

### 11.1 What is searched

The step rule is replaced by a search over every index path on an 801-point s-grid.
- **State (N, k).** The last Lehmann step was at s_k with N trial states, and ρ = ℓ_{N+1}(s_k). At k = 0, ρ is the
  exact base level E_{N+1}.
- **Move.** From (N, k), a step to k′ > k is admissible when μ_N(s_k′) ≤ ρ − Δ.
  - It yields ℓ₁ … ℓ_N.
  - The next state is (N′, k′) for any 12 ≤ N′ ≤ N − 1.
  - Keeping ρ (j = N + 1) is dominated and is left out. N therefore decreases strictly.
- **Proxy.** ℓ_i(s_k) = μ_i(s_k) − LOSS.
- **Goal.** A step at k′ with μ₁₁(s_k′) − LOSS ≥ ρ_b, within 40 steps.
- **Method.** Dynamic programming in decreasing N, minimising the number of steps.

**Configurations (Δ, LOSS).**
- (0, 0) is the most optimistic proxy.
- (0.02, 0) adds the registered margin.
- (0.02, 0.01) adds a Lehmann loss of the size seen in §10.2.

### 11.2 Validation

Every path found is executed with real Lehmann steps:
- the real ℓ_{N′+1} is the next ρ;
- admissibility is checked against the real ρ;
- the path is judged by the real ℓ₁₁ ≥ ρ_b.

### 11.3 Verdict, per control

- **PASS** if any configuration's path validates to a real ℓ₁₁ ≥ ρ_b within 40 steps (on the same basis of at most
  2,000 functions). This would reverse §10 for that control: B3′ is feasible in floating point, and the rigorous
  build of §6.3 becomes the next step.
- **RULED OUT (under the proxy)** if (0, 0) finds no path. Then no index path exists on this grid even with exact
  Ritz proxies, and B3′ at L_u = 5, L_v = 2 is set aside. This is a floating-point statement about the proxy, not a
  proof of impossibility. Ritz values are upper bounds, so the stand-in bounds are optimistic, but admissibility uses
  them too.
- **UNRESOLVED** otherwise: paths exist under the proxy but none validates.

### 11.4 Results: UNRESOLVED at both controls

Run at commit ac5f1a09. Logs are in `research/sc-lehmann-pathsearch-20261125/`.

- **Paths exist under the proxy at every configuration, and they are short.**
  - At g² = 0.002: 4 steps, for example N 92 → 47 → 32 → 13, with s at 0.11, 0.37, 0.58 and 0.96.
  - At g² = 0.004: 7 steps, for example N 186 → 115 → 86 → 47 → 23 → 13 → 12.
  - **B3′ is not ruled out.** μ₁₁ first reaches ρ_b at s = 0.958 (g² = 0.002) and 0.971 (g² = 0.004).
- **No path validates.** Every failure is an inadmissible step after 2–5 real steps (real μ_N against ρ − Δ):

  | control | (Δ, LOSS) | real steps | fails at s | real μ_N | ρ − Δ |
  | --- | --- | --- | --- | --- | --- |
  | g² = 0.002 | (0, 0) | 2 | 0.566 | 8.528 | 8.199 |
  | g² = 0.002 | (0.02, 0) | 2 | 0.579 | 8.5463 | 8.5352 |
  | g² = 0.002 | (0.02, 0.01) | 2 | 0.593 | 8.5664 | 8.5663 |
  | g² = 0.004 | (0, 0) | 3 | 0.410 | 8.176 | 7.547 |
  | g² = 0.004 | (0.02, 0) | 2 | 0.170 | 8.2050 | 8.2024 |
  | g² = 0.004 | (0.02, 0.01) | 5 | 0.809 | 7.767 | 7.605 |

- **Why they fail.**
  - The fewest-steps plan sets each next step at the edge of admissibility.
  - The real bound for the top index in the trial space falls below the proxy. Lehmann is weakest just below ρ,
    and the residuals grow to 0.07 by s ≈ 0.6.
  - So the next step becomes inadmissible.
- **What still holds.** The Lehmann bounds at low indices stay tight: ℓ₁₁ is within 5 × 10⁻³ of μ₁₁ at every real
  step.

## 12. Registration, 25 November: receding-horizon execution, before it is run

Script: `scripts/replan_helike_sc_lehmann_path.py`. It uses the same cached operators as §11.

- **The loop.** After each real Lehmann step:
  - the new real states are {(N′, k′): real ℓ_{N′+1}} for 12 ≤ N′ ≤ N − 1;
  - the §11 dynamic program is re-solved from those real states;
  - only its first move is executed.
- **Fixed settings.** Δ = 0.02, an 801-point grid, 40 real steps in total, and a basis of at most 2,000 functions.
- **Planning loss.** The proxy loss for planned (not yet executed) steps is LOSS ∈ {0.01, 0.03, 0.06}. A larger loss
  plans with more slack.
- **Verdict, per control.**
  - **PASS** if any LOSS reaches a real ℓ₁₁ ≥ ρ_b. B3′ is then feasible in floating point, §10's FAIL is superseded
    for that control, and the rigorous build of §6.3 is next.
  - **FAIL** if every LOSS ends with no plan from its real state or with 40 steps used. Item 2 of §10.3, widening the
    margin, is then the remaining route; otherwise B4 stands.

### 12.1 Results: FAIL at both controls

Run at commit 677acc39. Logs are in `research/sc-lehmann-replan-20261125/`.

| control | LOSS 0.01 | LOSS 0.03 | LOSS 0.06 |
| --- | --- | --- | --- |
| g² = 0.002 | 4 real steps to s = 0.758, ℓ₁₁ = ρ_b − 0.173; no plan | 4 real steps to **s = 0.994**, ℓ₁₁ = ρ_b − 0.150; no plan | no plan at s = 0 |
| g² = 0.004 | 5 real steps to s = 0.606, ℓ₁₁ = ρ_b − 0.434; no plan | 4 real steps to s = 0.368, ℓ₁₁ = ρ_b − 0.739; no plan | no plan at s = 0 |

**Verdict by §12: FAIL at both controls.**

**The binding constraint is the trial-function quality near s = 1, not the index path.**
- At g² = 0.002 with LOSS 0.03, replanning reached s = 0.994 with N = 13.
  - There μ₁₁ = ρ_b + 0.031, but the real ℓ₁₁ = ρ_b − 0.150. Lehmann lost 0.18 at the last step.
- The trial residuals ‖(H − μ)u‖ grow steadily along the homotopy, from 1 × 10⁻² near s = 0.05 to 1.2 × 10⁻¹ at s ≈ 1.
  - D multiplies the base functions out of the span of the 2,000 functions.
  - The quartic and cubic terms couple to levels far above the cut.
- With a margin of 0.03, the final Lehmann step needs residuals of order 10⁻² or below at s ≈ 1. Within this basis
  that is about an order of magnitude out of reach.

**What this settles.** B3′ with a basis of at most 2,000 product base eigenfunctions does not reach λ₁₁ ≥ ρ_b in
floating point at either control. The index path is not the obstacle (§11.4). The obstacle is the approximation
quality of the final trial space against a margin of 0.03.

**Remaining options. Each changes a registered parameter and needs its own registration.**
1. **Better final trial functions.** Keep the base basis for the early steps. For the last one or two steps, use a
   trial space built for H₁: for example, Hermite functions of the true (anisotropic) frequencies, cut to the tube.
   They stay in D(H₁), and A₂ stays exact through the D² moments.
2. **Raise the basis ceiling** above 2,000. This is a change to §8.5's criterion.
3. **Widen the margin** (§10.3, item 2), with a larger tube or a sharper base.

Until one of these passes, B4 stands: S_C is a stated hypothesis, and the ordering is proved conditional on it.

## 13. Registration, 25 November: block-adapted trial functions (option 1 of §12.1), before the homotopy is run

Script: `scripts/vscf_helike_sc_lehmann.py`.

### 13.1 Trial functions

At each step s, the trial functions are Ritz vectors of H_s in a product basis of eigenfunctions of the real block
operators:
- h_u(s) = h⁰_u + s(P(u, 0) − ½ω²_s,red|u|²), and h_v(s) likewise.
- Each is diagonalised in the full span of its block's base eigenfunctions: 683 soft (E_u < 13.03) and 78 stiff at
  g² = 0.002, sized the same way at g² = 0.004.
- The Galerkin trial space is the lowest 2,000 block products. The registered ceiling now applies to this space.
- The representation span is the full tensor product of the two blocks: 53,274 functions at g² = 0.002. **This is a
  deliberate change to §8.5's "at most 2,000 basis functions", stated here.**

### 13.2 Why the bounds stay valid

Every trial function is a finite combination of products of Robin base eigenfunctions, so it lies in D(H_s).

A₀, A₁ and A₂ are exact polynomial moments, contracted as coefficient tensors:
- A₂ = ⟨H₀u, H₀w⟩ + s(⟨H₀u, Dw⟩ + ⟨Du, H₀w⟩) + s²⟨u, D²w⟩;
- D² is the degree-8 polynomial, taken over all 165 of its u-monomial groups.

A₀ is computed, not assumed to be the identity.

### 13.3 Driver and verdict

The §12 receding-horizon driver is unchanged:
- planning on the cached isotropic Ritz curves;
- Δ = 0.02;
- LOSS ∈ {0.01, 0.03, 0.06};
- 40 real steps.

Only the real Lehmann step is replaced.

- **PASS** per control if any LOSS reaches a real ℓ₁₁ ≥ ρ_b. That supersedes §10 and §12 for that control, and makes
  B3′ feasible in floating point.
- **FAIL** otherwise.

### 13.4 Seen before this registration: component check only (g² = 0.002, s = 1)

These come from a check run with 13 trial vectors and no homotopy.

- **Ritz values minus E₀:** −0.0106, 1.0778 ×2, 1.0905, 2.1620 … 2.1888, 2.3555 ×2, 2.6241. Each is at or below its
  §10.1 value.
- **Residuals:** 3.8 × 10⁻³ for λ₁–λ₄, 4.4–4.7 × 10⁻³ for the six-level cluster, 1.1 × 10⁻² for λ₁₁ = λ₁₂, and
  1.5 × 10⁻² for λ₁₃.
- **Comparison:** the isotropic basis gave 0.12 at the same point, so this is about ten times smaller.

### 13.5 Results: PASS at both controls (floating point)

Run at commit 16c4d71c. Logs are in `research/sc-lehmann-vscf-20261125/`.

| control | LOSS | real steps | last s | real ℓ₁₁ − ρ_b | residuals | verdict |
| --- | --- | --- | --- | --- | --- | --- |
| g² = 0.002 | 0.01 | 4 (N 92 → 47 → 32 → 13) | 0.969 | **+0.0092** | ≤ 1.4e-2 | PASS |
| g² = 0.002 | 0.03 | 4 (N 47 → 32 → 23 → 13) | 0.994 | **+0.0299** | ≤ 1.5e-2 | PASS |
| g² = 0.002 | 0.06 | — | — | — | — | no plan at s = 0 |
| g² = 0.004 | 0.01 | 7 (N 186 → … → 13 → 12) | 0.980 | −0.0007 | ≤ 1.8e-2 | ran out of indices |
| g² = 0.004 | 0.03 | 9 (N 186 → 115 → 86 → 47 → 32 → 29 → 23 → 13 → 12) | 0.9975 | **+0.0240** | ≤ 1.7e-2 | PASS |
| g² = 0.004 | 0.06 | — | — | — | — | no plan at s = 0 |

**Verdict by §13.3: PASS at both controls.**

The ordering of eigenvalues gives the inside-model statement λ₁₁(H_in^N) ≥ λ₁₁(H_s) ≥ ℓ₁₁ ≥ ρ_b, with floating-point
margins of +0.030 at g² = 0.002 and +0.024 at g² = 0.004.

The real Lehmann bounds stay within 10⁻² of the Ritz values at the last step: ℓ₁₁ is 4 × 10⁻⁴ below μ₁₁ at g² = 0.002
and 5 × 10⁻³ below at g² = 0.004. §12 lost 0.18 at the same point.

**What this settles.**
- **Gate G-base of §4 and §8.5 passes in floating point at both controls.**
  - (i) Outside: as in §8.3.
  - (ii) Inside: a homotopy from the explicit B3′ base reaches λ₁₁ ≥ ρ_b within 40 steps, with at most 2,000 trial
    functions. The representation span is about 54,000 product functions; that change is disclosed in §13.1.
- **Floating point only, not a bound.** The rigorous version still needs every item of §6.3:
  - Arb enclosures of C₃, β and v_J;
  - rigorous radial eigenpairs (an enclosed base spectrum);
  - interval A₀, A₁ and A₂;
  - a verified pencil solver;
  - and then the Lehmann step on the full singlet H with cut Hermite states for λ₅ ≥ ρ_C (Lemma R), which this base
    feeds.
- **The margins are thin**, 0.024–0.030. Interval widening in the rigorous build must stay well below that.
- **S_C is still a stated hypothesis.** The ordering stays proved conditional on S_C until the rigorous chain closes.
  What has changed is that the route is now known to be feasible.
