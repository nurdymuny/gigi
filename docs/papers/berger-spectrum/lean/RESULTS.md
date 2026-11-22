# HELIKE Lean check: results

2026-11-21. Lean 4 v4.36.0, Mathlib v4.36.0 (prebuilt cache), Windows 11.

## Verdict

**All 12 theorems in `Helike/Price.lean` compile on the first attempt.**
- No `sorry` anywhere.
- Every theorem depends only on Lean's three standard axioms: `propext`, `Classical.choice` and `Quot.sound`.
- `lake build` reports `Build completed successfully (18254 jobs)`. The one warning suggests the newer `module`
  header; it does not affect any proof.

Before any Lean was written, the same identities were checked symbolically in sympy, and every difference simplified
to 0.

## The theorems

| theorem | statement | axioms |
| --- | --- | --- |
| `price_anatomy` | λ = κ² + (2j + t + y²) + ((4 − 2t) − 2B), with κ = (t−2)/√t − x − j/√t and B = y² − 2√t·x | standard 3 |
| `floorLaw_paper_form` | [j² + (8−2t)j − 2t² − 4t + 16]/t = [j(j+2) − 2(t−3)j − 2(t−2)(t+4)]/t | standard 3 |
| `kappa_berger` | on the Berger field x = (2−t)/√t: κ = (2t − 4 − j)/√t | standard 3 |
| `fieldCurv_berger` | on the Berger field: B = 3t − 4 | standard 3 |
| `price_berger` | her price on the Berger field (x = (2−t)/√t, y = √t) equals the floor law | standard 3 |
| `floorLaw_three` | at t = 3: λ = (j(j+2) − 14)/3 | standard 3 |
| `floorLaw_three_sector0` | λ₃(0) = −14/3 | standard 3 |
| `floorLaw_three_sector12` | λ₃(12) = 154/3 | standard 3 |
| `floorLaw_vertex_form` | λ_t(j) = (j − (t−4))²/t + (4 − 3t) | standard 3 |
| `floorLaw_ge_vertex` | for t > 0, λ_t(j) ≥ 4 − 3t for every j | standard 3 |
| `vertex_on_line` | λ_t(t − 4) = −3(t − 4) − 8: the vertices lie on λ = −3j − 8 | standard 3 |
| `floorLaw_cutoff` | for t ≥ 4/3, λ_t(t − 4 + √(t(3t − 4))) = 0 | standard 3 |

How the stretch is written: t = s² with s ≠ 0 (s > 0 where an inequality needs it). That keeps every identity
rational, and only the cutoff uses `Real.sqrt`.

## Scope

| status | claim |
| --- | --- |
| **checked by Lean** | the algebra of her price: the three-piece anatomy, the Berger specialization, the vertex line, the cutoff, the t = 3 values |
| **not checked by Lean** | that this formula is an eigenvalue of the Yang–Mills Hessian on the Berger sphere |
| **not checked by Lean** | that she is the floor (the lowest mode in every sector); that is the Floor paper's computed certification, doi:10.5281/zenodo.24412603 |
| **not checked by Lean** | the edge (the crossing with the neutral competitor) and hypotheses H1–H3 |

## Natural next files

1. The edge polynomial P = P₁P₂: its factorization, and the t_A polynomial dividing P₂. This is exact algebra, the
   same kind of work as above.
2. The eigenvalue derivation itself: her mode is an eigenvector of the Hessian's restriction to the lowest Landau
   level. This needs the operator in coordinates, so it is a larger step.
3. The certification, with rigorous interval bounds. This is the largest step, and a research project in its own
   right.

## Reproduce

```
cd docs/papers/berger-spectrum/lean
lake exe cache get
lake build
lake env lean Axioms.lean
```

## Edge polynomial (`Helike/Edge.lean`), 2026-11-21

All four theorems compile, with no `sorry`.
- Every theorem, together with the original twelve, depends only on `propext`, `Classical.choice` and `Quot.sound`.
- Rebuilt independently on macOS: Lean v4.36.0, Mathlib v4.36.0 prebuilt cache. `lake build`: `Build completed
  successfully (18256 jobs)`. Helike.Edge took 231 s; `ring` multiplies out the degree-28 factorization.
- The 473-term determinant is entered in ten parts of at most 48 terms (`edgeDetPart1` … `edgeDetPart10`), because a
  single expression of that size times out in elaboration. It is the same polynomial; the parts are checked equal to
  the receipt by sympy before the Lean was written.
- The file sets `maxHeartbeats 20000000`. That changes no axiom and no proof obligation.

| theorem | statement | axioms |
| --- | --- | --- |
| `edgeDet_factor` | edgeDet = −16384 · P₁ · P₂ | standard 3 |
| `edgeFactor_berger` | P₂((2 − s²)/s, s, s) = −32 (s − 1)(s + 1) Q₁₆(s), for s ≠ 0 | standard 3 |
| `tA_poly_root` | ∃ s ∈ [1.65261, 1.65262], Q₁₆(s) = 0 | standard 3 |
| `berger_crosses_edge` | ∃ s there with P₂ = 0 on the Berger path and s² ∈ [2.73111, 2.73116] | standard 3 |

**Scope.** Lean checks that the edge factor, restricted to the Berger path, *is* the t_A polynomial, and that t_A
lies where the Floor paper says. It does not check that `edgeDet` is the determinant of the Hessian's block (the
computer-algebra step), that P₂'s root is the edge (an eigenvalue comparison), or that she is the floor.


## The fall, the cut, the overload, the hands, the shadow (2026-11-22)

Five new files, 27 theorems. All compile, with no `sorry`. Together with the earlier sixteen, all **43 theorems**
depend only on `propext`, `Classical.choice` and `Quot.sound`.
- Windows 11, Lean v4.36.0, Mathlib v4.36.0 prebuilt cache.
- `lake build`: `Build completed successfully (18266 jobs)`.
- Four files compiled on the first attempt. `Cut.lean` needed one fix: `simp` had normalised the derivative's form.

They formalise the exact algebra behind Chapter 4 §4.3 (the fall and the stress tests) and Chapter 5 (the two hands)
of the book. The research notes are `docs/research/the-basin-2026-11-22.md`,
`docs/research/stress-tests-results-2026-11-22.md` and `docs/research/the-two-hands-2026-11-22.md` on
feat/reading-the-curve-update.

The family is `M₀ = xτ₀, M₁ = yτ₁, M₂ = yτ₂`, with `B = y² − 2√t·x`, `c = y(x − 2/√t)` and energy `E = ½(B² + 2c²)`.

| file | theorem | statement |
| --- | --- | --- |
| `Fall` | `crossCurv_berger` | on the Berger field `c = −t` |
| `Fall` | `energy_berger` | the Berger field's energy is `½((3t − 4)² + 2t²)` (34.71 at t = 3.55) |
| `Fall` | `flat_zero`, `flat_theta` | `A = 0` and `A = θ` (x = 2/√t, y = 2) have `B = c = 0` |
| `Fall` | `energy_zero`, `energy_theta` | both flat fields have energy 0 |
| `Fall` | `price_zero`, `price_theta` | her price is `(j + 2)²/t` at `A = 0` and `(j + 4)²/t` at `A = θ` |
| `Fall` | `price_zero_pos`, `price_theta_pos` | both are positive for every family `j ≥ 0`: at the bottom she is a wall |
| `Shadow` | `price_theta_eq_shift` | her price at `A = θ` in family `j` equals her price at `A = 0` in family `j + 2` |
| `Shadow` | `shadow_price` | her price at `A = 0` in family `j − 2` is `j²/t` |
| `Cut` | `hasDerivAt_energy_y` | `∂E/∂y = 2y(B + (x − 2/√t)²)` |
| `Cut` | `cut_line_invariant` | that slope is zero on `y = 0`: a cut field never leaves the cut line |
| `Cut` | `energy_cut`, `hasDerivAt_energy_cut` | on the cut line `E = 2t·x²` and `∂E/∂x = 4t·x` |
| `Cut` | `cut_fall_solves` | `x(τ) = x₀·e^{−4tτ}` solves the flow `dx/dτ = −∂E/∂x` |
| `Cut` | `cut_fall_tendsto_zero` | and tends to 0: the cut field falls to `A = 0` |
| `Cut` | `price_cut_mono` | on `y = 0`, from the Berger value of `x` upward, her price in every family `j ≥ 0` only rises (stress test S1c) |
| `Overload` | `overload_excess` | under the field scaled by `k`, price − (2√t·k·x − k²y²) = (k·x + (j + 2)/√t)² |
| `Overload` | `overload_lower` | so in every family her price is at least `2√t·k·x − k²y²` |
| `Overload` | `overload_attained` | with equality in the family `j + 2 = −k·x·√t`, where her stride cancels the belt |
| `Overload` | `overload_near` | within half a family of that, her price is within `1/(4t)` of the bound (so price/k² → −y²) |
| `Hands` | `rowEntry_cross` | in a row `(f − c_l·x − 2l/√t)²` the part bilinear in `x` and `l` is `4c_l·x·l/√t` |
| `Hands` | `her_minus_sister` | hers (`c_l = 1`) minus her sister's (`c_l = −1`) is `8xl/√t` |
| `Hands` | `price_cross` | in her price the part bilinear in `x` and `j` is `2xj/√t` |
| `Hands` | `price_cross_matches_row` | at `j = 2l` the two agree |

**Scope.**
- **Checked by Lean:** the algebra and calculus above.
- **Not checked by Lean:**
  - that the fall stays inside the circle-symmetric family (a symmetry argument);
  - that `x₀·e^{−4tτ}` is the *only* solution (uniqueness for the ODE);
  - which basin a general field lands in;
  - whether she is the floor anywhere, including in the current sweep and the stress tests;
  - that the `j²/t` state at `A = θ` is the lowest wall;
  - that the rows are the operator's rows (the edge paper's symbolic Weitzenböck identity);
  - the Poincaré-sphere holonomy.
