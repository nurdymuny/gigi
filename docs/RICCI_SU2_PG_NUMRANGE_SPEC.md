# S2c: numerical range of the SU(2) two-ring force (spec)

2026-11-18. Fixed **before** measurement. This is idea 1 of
`docs/research/small-edges-2026-11-18.md`, which has its provenance (Bee's Crouzeix
ledger and protocol, the HHE/Crouzeix check on the Ricci control, and
Crouzeix–Palencia 2017).

## In plain English

S2b showed the two-ring force has no runaway eigenvalues. But the operator is
lopsided (non-normal), and for a lopsided operator, eigenvalues say where the motion
ends up, not how much it can swell on the way. The numerical range is a region in the
complex plane that also sees the lopsidedness. Its left edge says whether any swell can
happen at all. Its whole shape tells us, through Crouzeix–Palencia, how large the time
steps can be. This spec measures it, before any time integration is written.

## Technical

The flow is B ξ′ = −K ξ (S2b's K and B, same level, trial and ε). With B = L Lᵀ
(Cholesky) and y = Lᵀξ, it becomes y′ = −Ĝ y, where Ĝ = L⁻¹ K L⁻ᵀ, and ‖y‖₂ = ‖ξ‖_B.

| Quantity | Definition |
| --- | --- |
| **W-left** | min Re W(Ĝ) = λ_min(S), with S = ½(Ĝ + Ĝᵀ). Also reported: the number of negative eigenvalues of S. |
| **Numerical radius** | w(Ĝ) = max over θ of λ_max(H(θ)), where H(θ) = ½(e^{−iθ}Ĝ + e^{iθ}Ĝᵀ); 180 angles over [0, π), using both λ_max and λ_min. |
| **Boundary points** | z(θ) = x*Ĝx, with x the extreme eigenvector of H(θ). An inner polygon of W. |
| **η** | ‖ĜĜᵀ − ĜᵀĜ‖₂ / ‖Ĝ‖₂² (Bee's C3 ledger, Lemma 1: 0 ≤ η ≤ 1) |
| **F1** | ‖Ĝ‖₂ / w(Ĝ) (Berger–Pearcy: 1 ≤ F1 ≤ 2) |
| **Exact-flow envelope** | ‖e^{−tĜ}‖ ≤ e^{mt}, with m = −W-left (numerical abscissa, constant 1) |
| **Implicit Euler** | the pole −1/h stays outside W iff h < 1/m (when m > 0) |
| **Explicit Euler** | h_EE = the largest h with max over boundary points of \|1 − h z\| ≤ e^{hm}. That is, the step at which the stepper amplifies no more than the exact flow's own envelope allows. Crouzeix–Palencia then bounds k steps by (1+√2)e^{khm}. |

## Gates

- **G-W (fixed now).** The two-ring operator's W-left is at least twice the native
  operator's W-left at the same level (both are expected to be negative, since the Berger
  field is moving), **and** it is at least −10. Failing either is reported as a finding
  against the force: a swell the eigenvalues hid.
- **Removal (one ring).** The one-ring operator must fail G-W. If it passes, the gate
  is not measuring roughness, and that is reported.
- **Reported, not gated:** w, F1, η, the boundary polygon, 1/m, and h_EE.

## Controls

- **C-eig.** The eigenvalues of Ĝ reproduce S2b's saved `*.eigs.npy` for the same level
  and trial. Every saved eigenvalue has a new one within 1e-8 × max|λ|, and vice versa.
- **C-chol.** B is symmetric positive definite (Cholesky succeeds), and
  ‖LLᵀ − B‖ / ‖B‖ < 1e-13.
- **C-W.** Every boundary point lies within the supporting lines (a consistency check
  of the θ sweep), and every eigenvalue of Ĝ has real part ≥ W-left − 1e-9 × ‖Ĝ‖.

## Scope

n = 1 and n = 2 (426 and 3,372 directions), trials native, pg and pg_onering. The
linearisation is at one field, so the nonlinear flow is not covered. Research script,
not an engine verb. The K matrices are saved locally under `outputs/` (gitignored,
since they are too large to commit) so the time-integration rung can reuse them.
