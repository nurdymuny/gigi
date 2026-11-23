# S2b: stability of the SU(2) two-ring force (spec)

2026-11-17. Fixed **before** measurement. Follows S2a
(`docs/RICCI_SU2_PG_FORCE_RESULTS.md`): the two-ring PG force passes BW05 on
Berger, and so does the one-ring force. In the Abelian paper, stability is what
separated them. This spec asks the same question for SU(2).

## What is measured

The semi-discrete flow is `B ξ' = −a(U)`, with `ξ` the right-trivialised link
variations, `a` the force, and `B` the native SU(2) tangent mass. At the Berger
field `U*` the linearised operator is `K = ∂a/∂ξ`, and the stability object is the
generalized spectrum of `(K, B)`, as in the paper's S3.

Each column of `K` comes from a central difference of `a` in one link direction:
`U_g → U_g exp(±ε τ_k)` with `ε = 1e-6`, the same convention as the native
derivative. Only the cells whose neighbourhood reads link `g` are recomputed for
that column, so every other cell's contribution is exactly zero. The full
gradient assembled cell by cell must equal S2a's gradient (control C-grad).

Because the background is not a critical point of the energy, gauge directions
are not exact zero modes: equivariance gives `K·(gauge direction) = [generator, a]`,
which is of the order of the force. The paper's "modulus below one equals the gauge
dimension" count is therefore **reported, not gated**.

## Gates (thresholds fixed now)

- **G-rough (the instability signature).** No eigenvalue of the two-ring operator
  has `Re λ < −10`. The continuum scale here is of order 4. The Abelian one-ring
  instability sat at −1.35×10⁴ and −2.42×10⁴, so −10 separates the two regimes by
  more than three orders of magnitude.
- **G-native (comparison, reported).** The same counts and extremes for the native
  Whitney operator.
- **Mechanism removal: one ring.** The one-ring operator is expected to violate
  G-rough, as in the Abelian case. If it does not, the paper's Abelian story does
  not transfer, and that is reported as such.

## Scope

- **Levels:** n = 1 and n = 2. Dense eigenproblems of size 426 and 3,372.
  n = 3 (11,358) is deferred.
- **Not measured here:** the numerical range (symmetric part), the flow, and
  time integration.
- Research script, not an engine verb.

## Controls

- **C-grad.** The cellwise-assembled gradient equals S2a's `run(level, order, "pg")`
  gradient to 1e-12 relative. The native trial likewise reproduces the native
  gradient.
- **C-col.** One Jacobian column computed by full-gradient central differences
  equals the local computation to 1e-8.
- **C-mass.** `B` is the native `blocks` matrix, symmetric to 1e-12 with a positive
  diagonal, through `test_ricci_operator_native.sparse_matrix`.
