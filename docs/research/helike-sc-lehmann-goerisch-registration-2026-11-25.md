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
