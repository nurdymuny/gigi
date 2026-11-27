# H8: the inter-well geometry in Lean

Companion to [The curved path reaches the other well](../../docs/research/h8-interwell-proof-2026-11-27.md).
This uses the existing pinned Lean 4.36.0 / Mathlib project; the H7 source
is unchanged. The `H8Interwell` library is a second build target.

The written proposition constructs a negative-gradient orbit from
diag(u,1,1) to 0 and determines the ambient zero-energy Agmon distance as
u/2, with u = 1/sqrt(t). Lean checks the following local ingredients.

| Declaration | Checked content |
| --- | --- |
| `metric_pairing` | Pairing the weighted negative gradient with a tangent |
| `dissipation` | W decreases at rate f^2 + 2 h^2 |
| `path_derivative` | Actual chain rule along any differentiable solution of the local ODE |
| `rectangle_boundaries` | The four boundary signs |
| `interior_descent` | y decreases in the open rectangle |
| `equilibria` | Only (0,0) and (u,1) when y >= 0 and u > 0 |
| `saddle_height` | Endpoint difference u/2 |
| `unstable_root` | The positive root and its characteristic equation |
| `unstable_direction` | The inward eigenvector relation |
| `rectangle_ceiling` | max(x0,u y0^2) < u for interior data |
| `decay_direction` | The differential inequality used for y's decay |
| `weighted_cauchy` | The two-dimensional weighted Cauchy inequality |
| `straight_excess` | Squared straight-path excess (u^2-1)^2/18 |
| `coupling_scaling` | Original q-space superpotential scales by 1/g^2 |

## Explicit formal boundary

The full nine-dimensional restriction is checked independently with exact
SymPy differentiation. The unstable-manifold theorem, forward invariance
as a global ODE assertion, continuation, endpoint convergence, full-space
path inequality and infimum over finite paths are in the written proof,
not formalized end to end here. `path_derivative` takes the ODE derivative
relations as hypotheses; it does not assert that a global orbit exists.
No custom axiom asserts the missing global construction.

Thus the appropriate description is **a written proof with machine-checked
local geometry and calculus**, not a fully Lean-verified heteroclinic or
a Lean proof of H8. Neither this nor the written result gives the quantum
splitting, a prefactor or the finite-coupling carrier ordering.

## Reproduce

From this project directory, with the pinned dependencies installed:

```text
lake exe cache get Mathlib.Analysis.Calculus.Deriv.Pow Mathlib.Analysis.Calculus.Deriv.Mul Mathlib.Analysis.Calculus.Deriv.Add
lake build
lake env lean H8Declarations.lean
lake env lean H8Verification.lean
lake env lean Verification.lean
```

From the repository root:

```text
python scripts/check_h8_interwell.py <fresh-output-directory>
```

The wrapper runs exact algebra and document-scope checks, prints theorem
signatures, audits all fourteen declarations, checks the eight H7 targets,
and compiles deliberately broken source variants. It also tests the axiom
auditor with poisoned output. No source admission, custom axiom or native
evaluation shortcut is permitted. Allowed foundational dependencies are
`propext`, `Classical.choice` and `Quot.sound`.

The four broken variants change the metric coefficient in the Cauchy
inequality, reverse the repeated-coordinate flow, halve the endpoint
action, or remove the y < 1 domain condition. They must be rejected by
Lean for proof/type errors, not missing imports. Exact examples additionally
check a failed ceiling outside the rectangle, the doubled gradient error,
and both vanishing and positive straight-path excess.

Evidence and source hashes: [h8-interwell-20261127](../../research/h8-interwell-20261127/).
