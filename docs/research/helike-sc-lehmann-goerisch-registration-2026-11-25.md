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
