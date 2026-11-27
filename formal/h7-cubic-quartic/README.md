# H7 cubic-quartic deduction in Lean

This project formalizes the **real-algebraic deduction**, not the whole
Yang--Mills model. The global Austing--Wheater inequality is a hypothesis
of the theorem. There is no custom axiom asserting it.

## What the checker verifies

For every configuration, cubic and quartic homogeneity and a global bound
give the exact real-number implication

$$|C(q)|\le A Q(q)+B\quad\Longrightarrow\quad
|C(q)|\le(A+B)Q(q)^{3/4},\qquad Q(q)\ge0.$$

The source handles Q = 0 separately. It then verifies the potential estimate

$$V(q)\ge\frac\kappa2\|q\|^2-
\frac{27S^4c^4}{32g^2}$$

from explicit cubic and quadratic bounds, including the original open
parameter window giving positive kappa. These are universal implications
over real numbers, not checks of finitely many triples.

| Declaration | Mathematical content |
| --- | --- |
| `H7.zero_of_scaled_bound` | Uniform boundedness under cubic scaling forces the cubic to vanish when the quartic is zero |
| `H7.cubic_quartic_bound` | Global inhomogeneous bound + cubic/quartic homogeneity + Q nonnegative imply the 3/4 inequality for every configuration |
| `H7.positive_constant_bound` | Positive global-bound constants give a positive constant in that conclusion |
| `H7.coercivity_factorization` | Exact 27/32 polynomial identity |
| `H7.scaled_coercivity` | Scalar bound with the correct g dependence and real fractional power |
| `H7.potential_lower_bound` | Combine the cubic and quadratic bounds into the reduced-potential estimate |
| `H7.kappa_pos` | Strict positivity on the original open source window |
| `H7.quadratic_coefficients` | The same kappa is the minimum of the existing quadratic coefficients |

The abstract configuration space needs only a scalar action: the proof
does not smuggle in a Lie-algebra conclusion through an unnecessary
structural assumption. Cubic and quartic homogeneity and the global bound
remain visible in the theorem signature.

## What is not formalized here

- Austing--Wheater's equation (2.2), including its global root-stratum argument.
- The concrete Lie-algebra functions, invariant norm and Hermitian trace dictionary.
- The Friedrichs realization, compact form embedding and compact resolvent.
- A quantum gap, a carrier ordering, or any continuum/infinite-volume limit.

The [written proof](../../docs/research/h7-cubic-quartic-proof-2026-11-27.md)
supplies the Lie-algebra translation and compact-resolvent argument using
the cited inputs. Neither that note nor this project establishes the gap,
carrier ordering or continuum limit. This build is not end-to-end formal
verification of Statement 47 or H7. It is a second, kernel-checked
verification of the central algebra.

## Reproduce

Lean: `leanprover/lean4:v4.36.0`.
Mathlib: `fc4f59e577feb1a4956671bc04ac18aaaf8cd842` (v4.36.0).
The transitive revisions are fixed by `lake-manifest.json`.

From this directory:

```text
lake update
lake exe cache get Mathlib.Analysis.SpecialFunctions.Pow.Real Mathlib.Tactic.FieldSimp Mathlib.Tactic.Linarith Mathlib.Tactic.NormNum Mathlib.Tactic.Positivity Mathlib.Tactic.Ring
lake exe cache get Mathlib.Analysis.Calculus.Deriv.Pow Mathlib.Analysis.Calculus.Deriv.Mul Mathlib.Analysis.Calculus.Deriv.Add
lake build
lake env lean Verification.lean
```

`lake update` may fetch the full Mathlib cache through its update hook.
Set `MATHLIB_NO_CACHE_ON_UPDATE=1` first to use only the targeted cache
command above. This is dependency setup, not a physics computation.

From the repository root, the audit wrapper additionally runs deliberate
source mutations and retains the compiler diagnostics:

```text
python scripts/check_h7_lean.py <new-output-directory>
```

The wrapper refuses existing output directories. It records failure unless
the build, signatures, foundational-axiom audit and all expected compiler
rejections pass. Its axiom allowlist contains only `propext`,
`Classical.choice` and `Quot.sound`; these are standard foundations, not
the published inequality. No admission or native-evaluation proof shortcut
is accepted. Lean's kernel checks the proof terms produced by the tactics.

## Status

**Passed, 27 November 2026.** All eight target declarations compile, as do
the two fourth-root helper lemmas. The axiom reports for the targets list
only `propext`, `Classical.choice`, and `Quot.sound`.

Four exact examples also compile: the too-small 27/64 constant fails at
z = 3/2; 27/32 gives equality there; kappa vanishes at the source-window
boundary t = 3, s = 5/4; and a pointwise bound alone is insufficient.

Four deliberately broken source variants are compiler-rejected: exponent
2/3, a removed global-bound hypothesis, removed cubic homogeneity, and
27/64 in the factorization. Two poisoned axiom reports are also rejected
by the auditor. This tests the checker's refusal behavior; it does not
claim every possible proof mutation has been tested.

The initial absent-theorem failures and final compiler evidence are in
[`research/h7-lean-20261127/`](../../research/h7-lean-20261127/).
The source revision in each receipt names the baseline; its individual
file hashes identify the uncommitted proof source checked in that run.

The companion [H8 inter-well library](H8-README.md) shares these pinned
dependencies and is now a second default build target. Its local algebra
and calculus have a separate audit; the original H7 theorem source is
unchanged.
