# Registration: Helike's flux edge in the sourced condensate

24 November 2026. **REGISTERED. Frozen with the commit that adds this document and the runner. Not yet run.** Design and
derivations: [the flux-background design](helike-flux-background-design-2026-11-24.md).

## Question

Around a held background with flux along her circle, does the lowest quantum vibration switch to Helike's channel
where the helicity² − flux mechanism, corrected to O(g²), says it should?

## Model (frozen)

H_src = −½Δ_q + ½|ω∘q − g cof q|² − ½a|q₀|² − ½b(|q₁|² + |q₂|²), with ω = (√t, 1/√t, 1/√t). The Maurer–Cartan-line
background is q_bg = diag(2s/√t, 2s, 2s)/(2g). The sources are a = (1 − s)(t − 2s) and b = (1 − s)[(1 − s)/t − s]. The
window is s < (t + 2)/(t + 1). Colour singlets only, on the sheet det q > 0.

## Method (frozen; changed from the design, and why)

The design's orbit-envelope basis needs a Haar series of order K ≈ 60–100 at |q_bg| ≈ 4.5–6.7. That expands into
hundreds of millions of G-monomials, so it is infeasible on any machine. It is replaced by **local Fock
diagonalisation in the reduced G-space** (`scripts/helike_condensate_fock.py`):
- **Exact reduction:** colour singlets reduce to G = qqᵀ, with H_s = −½Σ∂_n B_np(z)∂_p + V(z) + U(z) in the normal
  coordinates of the harmonic problem. B = I + (linear in z) is exact.
- **Taylor expansion:** V and the measure's curvature U are expanded to order P.
- **Spectrum:** diagonalised in the 6-mode oscillator basis with at most Mq total quanta.

This method is convergence-controlled, not residual-certified.

**Validation before registration:**
- In the harmonic limit, every single- and two-quantum level is exact.
- At small g, (gap − ω)/g² extrapolates to the exact O(g²) coefficients: −0.525 against −0.530 (Helike), and −2.737
  against −2.738 (competitor), at t = 3, s = 1.15.
- Levels are identified by single-quantum weight (0.98 or more there).
- Tests: `scripts/test_helike_condensate_fock.py` (3 passing). Mutation check
  `scripts/check_helike_condensate_fock_mutations.py`: 3 of 3 caught (kinetic cubic dropped, kinetic cubic flipped,
  Taylor truncated at order 2).
- The curvature term U is constant at O(g²) and cancels in gaps, so it is deliberately not a mutant.

## Design (frozen, as in the runner `scripts/run_helike_flux_background.py`)

| Block | t | g² | s grid |
| --- | --- | --- | --- |
| Primary | 3 | 0.03, 0.05, 0.07 | 1.12, 1.15, 1.165, 1.18, 1.195, 1.22, 1.24 |
| Secondary | 4 | 0.03, 0.05 | 1.09, 1.12, 1.14, 1.16, 1.18 |

- **Convergence ladder:** (P, Mq) = (6, 6), (6, 8), (8, 8). A point is converged if both identified gaps change by at
  most 0.003 on the last step.
- **Identification:** Helike's level is the state of maximal weight on her |Q| = 2 single-quantum doublet; the
  competitor's is the state of maximal weight on the low Q = 0 mode. Both weights must be at least 0.9.

## Predictions (registered before any registered-order computation)

The switch is where the competitor's gap rises above Helike's, by linear interpolation across the s grid. Predicted
from the exact O(g²) theory:

| t | g² = 0.03 | 0.05 | 0.07 |
| --- | --- | --- | --- |
| 3 | 1.1674 | 1.1809 | 1.1931 |
| 4 | 1.1260 | 1.1413 | — |

**Tolerance:** |measured − predicted| ≤ 0.015 in s. That is about the grid spacing; the O(g⁴) coefficients measured at
small g largely cancel between the two levels.

**Secondary (reported, not decisive):** near the switch, Helike's gap shifts from its harmonic value by much less than
the competitor's: |c_H| ≈ 0.5 against |c_C| ≈ 2–3 times g².

## Decision rule

- **Per (t, g²):**
  - UNRESOLVED if any point fails convergence or identification;
  - NOT_SUPPORTED if converged with no switch, or a switch outside tolerance;
  - SUPPORTED if converged with a switch inside tolerance.
- **Overall (primary block):**
  - SUPPORTED if all three are SUPPORTED;
  - UNRESOLVED if any is UNRESOLVED;
  - NOT_SUPPORTED if all three are NOT_SUPPORTED;
  - MIXED otherwise.

## Exposure (disclosed)

- Before this registration I computed:
  - the harmonic and O(g²) predictions (theory);
  - two validation solves at g² = 0.002 and 0.004, t = 3, s = 1.15 (not registered couplings);
  - a software smoke run of the runner at t = 3, g² = 0.05, s ∈ {1.15, 1.20}, at low order (P = Mq = 4), with the
    decision line hidden. Those two points are registered points.
- I did not inspect the smoke run's gaps or decision. Its receipt stays in the job's scratch directory, outside the
  repository.

## Scope

This is a controlled quantum test of the helicity² − flux mechanism around a held background, in one-grain
homogeneous Yang–Mills on a Berger sphere. The sources are external. It says nothing about the unsourced vacuum or
the continuum. Approximation: Taylor order P, basis Mq and the d > 0 sheet; tunnelling to the origin region is
neglected, which the depths c/g² = 3.9–7.4 justify.

## Run

```
python -m unittest scripts/test_helike_condensate_fock.py
python scripts/check_helike_condensate_fock_mutations.py OUT/mutations.json
python scripts/run_helike_flux_background.py OUT
```

The run is CPU-heavy at (8, 8): the sparse space has quanta up to 18, and sympy Taylor expansion to order 8 in six
variables is slow. It is meant for the other machine.
