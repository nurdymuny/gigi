# Larger-colour-space transport: evidence

27 November 2026. Baseline `e213f9fd`.

## Result and scope

For the unchanged unsourced reduced model, each injective su(2) embedding
with measured pullback metric I > 0 transports the proved gradient path.
The action from 0 to its specified Ad(G) endpoint orbit is exactly
I/(2 sqrt(t)) in scaled coordinates, with division by g^2 in original
coordinates. The lower bound is ambient: leaving the embedded algebra
cannot lower the action to that orbit.

The [written proof](../../docs/research/h8-group-transport-proof-2026-11-27.md)
uses the earlier global H8 path theorem and the cited rank-one-subalgebra
construction. The [Lean coverage](../../formal/h7-cubic-quartic/H8Group-README.md)
is conditional transfer algebra and real scalar consequences, not a
formalization of Lie-group structure or the global path infimum.

## Exact examples

All use B(A,B) = -2 tr(AB), with anti-Hermitian generators. These entries
are checked by exact matrix operations, not fitted or simulated curves.

| Fixture | Ambient real configuration dimension | Measured metric I |
| --- | ---: | ---: |
| Fundamental block in SU(3) | 24 | 1 |
| Irreducible spin-1 in SU(3) | 24 | 4 |
| Two fundamental blocks in SU(4) | 45 | 2 |
| Conjugated fundamental block in SU(3) | 24 | 1 |

The all-nine-coordinate restriction checks W, the full gradient and V.
The independent ambient differentiation tests all 24 coordinates in each
of the three SU(3) fixtures: 72 exact derivative comparisons. The measured
metric is I(dx^2 + 2dy^2). The velocity is unchanged, while the action has
factor I. These are genuinely different checks.

## Gate history

- `red/`: ten exact tests pass; the absent written proof fails; all nine
  absent Lean declarations fail. The original stub is retained. The receipt
  remains passed=false with expected_red_observed=true.
- `release-01/`: all eleven checks and all nine Lean lemmas pass, but one
  verification example's square-root simplification is incomplete. The
  receipt is failed. The example was completed using the square-root
  identity; neither its statement nor a theorem assumption was weakened.
- `release-02/`: all eleven checks, nine Lean lemmas and four verification
  examples pass. All nine dependency audits use only standard foundations.
  Both poisoned dependency reports and all four broken sources are rejected.

The broken sources remove bracket preservation, omit I, remove the planar
factor two, or halve the endpoint action. Matrix-level controls also reject
non-homomorphic generators, reversed orientation of just one generator,
wrong metric declarations and an artificial inverse-I velocity.

Focused H8 and H7 regression runs have their own directories and receipts:
`h8-regression/` passes fourteen checks, fourteen H8 Lean targets, the eight
H7 dependency audits and four H8 broken-source rejections;
`h7-algebra-regression/` passes twelve checks. `reuse-rejected.log` records
the refusal to reuse the passing output directory; its receipt hash is
unchanged across that attempted rerun.
No engine test suite, eigensolve, numerical trajectory or GPU run is needed.
The paper source/PDF and the earlier H7/H8 theorem implementations are unchanged.

## Reproduce

From the repository root, with the existing pinned Lean cache and SymPy:

```text
python scripts/check_h8_group_transport.py <fresh-directory>
python scripts/check_h8_interwell.py <another-fresh-directory>
python scripts/check_h7_cubic_quartic.py <third-fresh-directory>
```

Each receipt retains commands, exit codes, tool versions, dependency
revisions and LF-normalized source hashes. Its source revision is the
pre-commit baseline; the hashes identify the checked working-tree files.
Output directories cannot be reused, and failed receipts are retained.
Deliberately broken Lean files are evidence, not build targets.

## Adversarial reading

The group map need not be injective: its Lie-algebra differential must be.
The metric factor is measured, not assumed to be an integer. The gradient
is derived in all ambient directions, not projected first. The lower bound
applies to the entire target orbit. Endpoint connectors handle infinite
flow time. Extra wells need not be classified for this endpoint statement.
Other embeddings can have different actions, and equal-action paths are
not excluded. No carrier ordering, quantum splitting or continuum claim
is promoted. H7 and H8 remain open.
