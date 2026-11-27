# Formal boundary: the Wick-triangle cancellation

The five declarations in `H6WickTriangle.lean` prove that a product of
three matrices anticommuting with a common involution has zero trace.
They use Mathlib's actual finite-matrix trace and cyclicity, not an
unproved trace axiom. The dimension is arbitrary and the scalars are real.

The physical instantiation is the Hodge involution on Euclidean two-forms
and the three separated-point Maxwell covariance blocks. Its derivation,
including all normalizations and the connected Wick factor 64 n, is in
[`h6-wick-triangle-proof-2026-11-27.md`](../../docs/research/h6-wick-triangle-proof-2026-11-27.md).
Exact Python tests independently verify that instantiation and contraction
counting. Lean does not construct a continuum quantum field theory or
formalize those Wick combinatorics in this bounded project.

The existing Lean 4.36.0 and Mathlib revision remain pinned. The checker
requires all five declarations, audits their foundational dependencies,
rejects proof shortcuts, and compiles deliberately broken variants with
a missing exchange, missing involution, or incorrect conjugation sign.

```text
python scripts/check_h6_wick_triangle.py <fresh-output-directory>
```

The original prediction is refuted, not proved. The four-point positive
control supports the already proposed alternative diagnostic only under
H6's matching assumptions; H6 remains open.
