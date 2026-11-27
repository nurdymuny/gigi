# Strict vacuum floor: formal boundary

The written theorem is in
[`h8-vacuum-floor-proof-2026-11-27.md`](../../docs/research/h8-vacuum-floor-proof-2026-11-27.md).
This uses the existing pinned Lean 4.36.0 / Mathlib project.

Seven declarations in `H8VacuumFloor.lean` check:

1. The scalar differential-jet expansion with the correct constant-shift sign.
2. A finite sum of real squares can vanish only when every entry vanishes.
3. The exact cone-tail remainder identity.
4. The corresponding tail inequality under the declared radial threshold.
5. Strictness from a lower bound, an attained bottom, equality implying a
   kernel state, and exclusion of admissible kernel states.
6. Transfer of a strict lower bound to a higher sector bottom.
7. The local harmonic-Gaussian eigenvalue identity.

The jets are independent scalar variables: this file is not a formal
integration-by-parts or Sobolev-domain theorem. The analytic inputs to
item 5 are explicit hypotheses, not newly declared axioms. Their proofs
are in the research note. No Lean claim is made for the global
Lie-algebra inequality, compact embedding, distributional constant
theorem, or divergence on a spherical cone.

`H8VacuumDeclarations.lean` checks that every target exists.
`H8VacuumVerification.lean` reports its foundational dependencies and
checks counterexamples to two tempting overclaims. The Python driver
rejects unknown dependencies and forbidden proof shortcuts, then compiles
four deliberately broken variants. Failed imports do not count as valid
negative tests. Existing H7/H8 files are not weakened or reaxiomatized.

Run from the repository root with a fresh destination:

```text
python scripts/check_h8_vacuum_floor.py <new-output-directory>
```

This is bounded machine verification accompanying a written proof, not
an end-to-end formalization of the spectral theorem.
