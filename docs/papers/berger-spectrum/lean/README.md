# HELIKE in Lean

These are the exact algebraic statements behind *HELIKE*, Chapter 4, Section 4.1, checked by the Lean 4 proof
assistant against Mathlib v4.36.0 (Lean v4.36.0).

There is no `sorry` anywhere. Every theorem depends only on Lean's three standard axioms: `propext`,
`Classical.choice` and `Quot.sound`.

## What is checked (`Helike/Price.lean`)

| theorem | statement |
| --- | --- |
| `price_anatomy` | her price splits exactly into κ² + (2j + t + y²) + ((4 − 2t) − 2B), for every x, y, j and every t > 0 |
| `floorLaw_paper_form` | the floor law equals the form printed in the Floor paper |
| `kappa_berger`, `fieldCurv_berger` | on the Berger field: κ = (2t − 4 − j)/√t and B = 3t − 4 |
| `price_berger` | her price on the Berger field *is* the floor law |
| `floorLaw_three`, `_sector0`, `_sector12` | at t = 3: (j(j+2) − 14)/3, −14/3 in sector 0, 154/3 in sector 12 |
| `floorLaw_vertex_form` | each parabola has its bottom at j = t − 4, with height 4 − 3t |
| `floorLaw_ge_vertex` | no sector is cheaper than 4 − 3t |
| `vertex_on_line` | every vertex lies on λ = −3j − 8 |
| `floorLaw_cutoff` | for t ≥ 4/3 her price crosses zero at j = t − 4 + √(t(3t − 4)) |

Here t = s² with s ≠ 0, so the statements need no square roots, except the cutoff.

## What is checked (`Helike/Edge.lean`): the edge polynomial

| theorem | statement |
| --- | --- |
| `edgeDet_factor` | the determinant numerator of the competing block B₄,₀ (473 terms) equals −16384 · P₁ · P₂ |
| `edgeFactor_berger` | on the Berger path x = (2 − s²)/s, y = s: P₂ = −32 (s − 1)(s + 1) · Q₁₆(s), with Q₁₆ the t_A polynomial |
| `tA_poly_root` | Q₁₆ has a root s with 1.65261 ≤ s ≤ 1.65262, by the intermediate value theorem |
| `berger_crosses_edge` | there, P₂ vanishes on the Berger path, and t = s² ∈ [2.73111, 2.73116] |

The polynomials are entered verbatim from the computer-algebra receipt (`research/su2-floor-family-20261119/edge_polynomial.srepr`).
That they are det(B₄,₀ − λ₄ I), and that P₂'s root is the edge, are not checked in Lean.

## What is checked: the fall, the cut, the overload, the hands, the shadow (2026-11-22)

Five more files (`Helike/Fall.lean`, `Shadow.lean`, `Cut.lean`, `Overload.lean`, `Hands.lean`), 27 theorems:
- the flat fields and her prices at the bottom of the fall, `(j + 2)²/t` and `(j + 4)²/t`;
- the wrap that moves her two families;
- the cut: the cut line is invariant, the fall is `x₀·e^{−4tτ}` to `A = 0`, and her price only rises;
- the overload bound, reached where her stride cancels the belt;
- the cross terms of the two hands.

Full table and scope in `RESULTS.md`. In total, 43 theorems, all on the standard three axioms.

## What is *not* checked here

These files check the **algebra**: the formula for her price, and what follows from that formula. They do not check
that this formula is the eigenvalue of the Yang–Mills Hessian, and they do not check that she is the floor (the
cheapest mode in every sector). That is the Floor paper's computed certification (doi:10.5281/zenodo.24412603), and
formalizing it is a separate and much larger project.

## Reproduce

```
lake exe cache get    # download Mathlib's prebuilt files
lake build            # check every proof
lake env lean Axioms.lean   # list the axioms each theorem depends on
```
