# S_C stage R2: interval Lehmann steps on the folded path (registration of design, before code)

26 November 2026. **REGISTRATION of design.** Nothing in this note has been computed. It is the last piece before
S_C can be stated as proved at the controls.

## What is already in place

- **Base.** B3′ with the ray-aware fold: (0.36, 0.17) at g² = 0.002 and (0.45, 0.25) at g² = 0.004. D ≥ 0 is
  proved (R1.8.4), so H_s = H₀ + sD increases in s. The base spectrum is enclosed by R1.4 (re-run at the folded
  constants: R2.0 below).
- **Outside.** inf V ≥ ρ_C off the tube, proved at both controls (outside-bound registration).
- **Floating path.** 2–3 real steps per control, with ℓ₅ − ρ_C = +0.21 to +0.24 at the last step (R1.9.2).
- **Route.** Lemma R, and λ₅(H) ≥ min(ℓ₅(H_s), ρ_C) for any step (R1.8.5).

## The trial functions: cut Hermite states (chosen over Kummer moments)

The floating VSCF trial functions are combinations of exact Robin eigenfunctions of the base: Kummer radials times
harmonics. Enclosing their matrices needs about 10⁵ rigorous radial moments ∫ r^{2+k} R_a R_b dr. A timing test of
Arb's rigorous integrator (`acb.integral`) took about 13 s per integral, so that route is weeks of CPU. It is not
taken.

**Chosen:** u = ηψ, with ψ a polynomial times a Gaussian in z (Hermite functions with per-block frequencies, so
VSCF-like coefficients fit) and η a smooth cutoff that is ≡ 1 well inside the tube and ≡ 0 near its wall.
- u vanishes near the wall, so u lies in the domain of every H_s (Neumann/Robin) and of H₁. No boundary condition
  needs checking.
- H_s u = η H_s ψ − ½[Δ, η]ψ. Polynomial moments of ψ against D and D² are exact (Kronecker or Gaussian moments, as
  in the exact-moment Rust solver).
- The cut corrections use the existing Arb enclosures:
  - the tail t_k ≥ ‖1_ext ψ_k‖;
  - the leakage ℓ_k ≥ ‖[H, η]ψ_k‖;
  - and τ_k ≥ ‖1_ext (H_s − σ)ψ_k‖ (registered in §3 of the Lehmann registration).
- A₀, A₁ and A₂ follow by the formulas of that §3, with H replaced by H_s.

## Steps

0. **R2.0.** Re-run R1.4 (`helike_sc_r1_base_enclosed.py`) at the folded constants. Every enclosed level must contain
   the floating level. The first step's ρ is an enclosed base level, taken at its lower endpoint.
1. **R2.1.** Interval A₀, A₁ and A₂ for cut Hermite trial states at each floating step's (s, N, ρ).
   - Tests: the exact parts reproduce the floating matrices to 10⁻¹⁰.
   - Mutants: drop t·t, drop ℓ, drop τ, and use the uncut moments.
2. **R2.2.** A verified pencil.
   - Positive definiteness of A₂ − 2ρA₁ + ρ²A₀ by interval Cholesky.
   - Enclosures of the negative τ_i: verified eigenvalue bounds for the symmetric definite pencil, via interval
     Cholesky of B and Gershgorin or Weyl on L⁻¹(A₁ − ρA₀)L⁻ᵀ, with the floating eigenvectors as preconditioner.
   - Then ℓ_k = ρ + 1/τ_i, rounded down.
3. **R2.3.** Chain the steps: each step's ρ is the previous step's interval ℓ_{N′+1}, rounded down.

## Decision rule, per control

- **S_C PROVED** if all three hold:
  - the Lemma R windows check passes;
  - some step's interval ℓ₅ ≥ ρ_C, using the **frozen** ρ_C of the Temple ordering receipt;
  - the outside bound holds at that frozen ρ_C (rerun the outside code with it).
- **UNRESOLVED** otherwise. There is no "S_C false" outcome.

## Risks, stated in advance

- **Cutting costs residual.** η must be ≡ 1 where the states live (|u| ≲ 3.5, |v| ≲ 1.5). The tube gives room, but the
  leakage enters A₁ and A₂ linearly, against a margin of 0.21 on ℓ₅. The leakage gate measured leakage of order
  10⁻³–10⁻² on comparable states, so the margin should hold.
- **The middle steps** (s around 0.3–0.6, N = 23–92) need Hermite trial spaces good enough for many states. Fewer,
  larger steps are possible if the planner is re-run with the cut-Hermite trial quality.
- **Interval widening** in a 2,000-function pencil. The verified-eigenvalue route must use the floating eigenvectors as
  a preconditioner, or the Gershgorin discs will be too wide.

## R2.0 result, 26 November: done

This used `scripts/helike_sc_r1_base_enclosed.py --fold=a,b`, with ω²_red at the lower endpoint of w²_min − a (and
likewise for the stiff block), the enclosed β, and v_J. Log: `research/sc-r1-20261125/r2_0_folded_base_enclosed.txt`.

| | g² = 0.002, fold (0.36, 0.17) | g² = 0.004, fold (0.45, 0.25) |
|---|---|---|
| enclosed product levels below ρ_b (none ambiguous) | 23 | 23 |
| base λ₁ / λ₁₁ | 4.94169 / 7.26443 | 4.83880 / 7.14161 |
| floating Galerkin count, max diff | 23, 1.4·10⁻⁹ | 23, 1.1·10⁻⁹ |
| max enclosure radius | 9.1·10⁻¹⁴ | 9.1·10⁻¹⁴ |

For comparison, the uniform base had 38 and 92 levels, with λ₁₁ of 6.83 and 6.09 (R1.8.3).
