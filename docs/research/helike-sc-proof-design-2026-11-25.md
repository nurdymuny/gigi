# Proving S_C: design for a rigorous lower bound on the third charge-0 level

25 November 2026. **DESIGN, not a registration and not a result.** S_C is the one hypothesis behind the
[Kato–Temple ordering](helike-temple-ordering-results-2026-11-25.md):

> *In the colour-singlet, charge-0 sector, at most two eigenvalues lie below ρ_C = E₀ᶜ + 2ω_H − ω_min/4.*

Proving it means a rigorous lower bound λ₃⁽⁰⁾ ≥ ρ_C. Theorem A (discrete spectrum) is already proved, so the third
eigenvalue exists. This note fixes the route, the constants that must be enclosed, and the order of work. No
computation has been done for it.

## Route 1 (primary): IMS localization with a harmonic comparison

This is the standard semiclassical harmonic-approximation argument, made explicit with enclosed constants. Weak
coupling is the semiclassical regime here: the well depth scales like 1/g², and its width stays O(1).

1. **Partition.** Take J_in² + J_out² = 1 with smooth cutoffs that are functions of colour and SO(2) invariants
   (for example the distance of G = qqᵀ from G₀ in the chart). They then preserve every sector. J_in is supported
   in a tube of radius L around the background's colour orbit. The IMS formula gives
       H ≥ J_in H J_in + J_out H J_out − ½(|∇J_in|² + |∇J_out|²),
   with the last term at most c_J/L².
2. **Outside the tube.** V ≥ V_bg + D_L, where D_L is the minimum of V − V_bg off the tube, on both determinant
   sheets. V is a quartic polynomial, so D_L is a polynomial optimisation over an unbounded set, made compact by
   Lemma A3's coercivity. D_L must be enclosed rigorously: Arb branch-and-bound on the compact region, plus A3
   beyond it. The out-of-tube contribution then lies above V_bg + D_L − c_J/L². That is required to exceed ρ_C.
3. **Inside the tube.** Exactly, with z the transverse normal coordinates,
       V = V_bg + ½zᵀKz + P₃(z) + P₄(z),
   where P₃ and P₄ are the explicit cubic and quartic parts of the polynomial. On the tube,
   |P₃| ≤ C₃|z|³ ≤ C₃L|z|², and P₄ ≥ −C₄|z|⁴ similarly. So V ≥ V_bg + ½zᵀ(K − 2(C₃L + C₄L²))z. In the singlet sector
   the colour-orbit directions are a compact rotor, whose J = 0 level is the singlet's, plus curvature corrections
   of order 1/|q_bg|², which is O(g²). The inside operator is then bounded below by a harmonic oscillator with
   reduced frequencies, minus c_J/L² and the curvature constant. Its charge-0 levels are explicit: 0, ω_C, 2ω_H,
   2ω_C, … (scaled).
4. **Count.** By min–max applied to the IMS bound, the number of eigenvalues of H in charge 0 below ρ_C is at most
   the number for the comparison operator. If the reduced third charge-0 harmonic level, minus all errors, stays
   above ρ_C, then S_C holds.

**What must be enclosed** (all CPU work, interval or Arb, no GPU):
- K and its eigenvalues;
- the operator norms C₃ and C₄ of the exact cubic and quartic tensors at q_bg;
- the partition constant c_J;
- the curvature constant of the orbit reduction;
- D_L.

**The budget.** The slack available is the harmonic third level minus ρ_C, which is ω_min/4 ≈ 0.27. The errors
scale like C₃L + C₄L² (frequency reduction), c_J/L² (localization) and g² (curvature). At g² = 0.004 they must
total less than about 0.27. The optimal L balances the first two terms. Whether the explicit constants fit is the
first thing to compute: a floating-point feasibility estimate before any enclosure.

## Route 2 (fallback): Lehmann–Goerisch with a homotopy base problem

If Route 1's constants are too large, use Route 1's comparison operator, which has an exactly known spectrum, as the
base problem of a homotopy (Plum; Behnke–Goerisch). Deform it to H in steps. At each step, Lehmann–Goerisch bounds
from the existing trial spaces give lower bounds for the first few levels, using the previous step's bound as ρ.
This is sharper and more expensive. It reuses the interval moment machinery for the mass, ⟨Hφ, Hφ⟩ and cross terms.

## Order of work

1. A floating-point feasibility estimate of Route 1 at the four controls: C₃, C₄, D_L(L), the optimal L and the
   resulting margin. This is light, and only an estimate.
2. If the margin is positive with room, register Route 1: constants, enclosure methods, decision and predictions.
   Then implement the enclosures (Arb for D_L; intervals for C₃, C₄ and K), with RED tests and mutants as before.
3. If not, register Route 2 instead.
4. Only after S_C is proved at a control may "conditional on S_C" be dropped there. At the grid couplings, S_C must
   be proved per point, or stay stated.

## Risks, stated in advance

- **The rotor.** The colour-orbit reduction's curvature constant may not be small enough at the coarser couplings.
  At the controls it is O(g²) ≈ 0.004, small against a 0.27 slack.
- **The depth.** D_L near the rank-two fold, and on the mirror sheet, must be computed and not assumed. The signed
  cubic separates the sheets, which helps.
- **Route 1 may simply fail** at g² = 0.004 because of large C₃. Then Route 2 is the path, and the timeline grows.
