# `poincare_to_sharding` validation suite

Six TDD gates that validate the math claims in
[`../poincare_to_sharding.md`](../poincare_to_sharding.md) and
[`../SHARDING_SPEC.md`](../SHARDING_SPEC.md).

## Running

All six tests:

```bash
python theory/poincare_to_sharding/validation/run_all.py
```

Single test:

```bash
python theory/poincare_to_sharding/validation/t1_mayer_vietoris_betti.py
```

Each returns exit code 0 if green, 1 if red.

## The six gates

| File | Validates | Theory §  | Wall-clock |
|---|---|---|---|
| `t1_mayer_vietoris_betti.py`       | Sharded BETTI exact via M-V    | §3.1 | ~4.5 s |
| `t2_cocycle_bound.py`              | Cocycle bound 0 / first-order   | §3.2 | ~1.3 s |
| `t3_sharded_curvature.py`          | Sharded CURVATURE via sheafify  | §3.3 | ~1.3 s |
| `t4_sharded_holonomy.py`           | Sharded HOLONOMY w/ gauge       | §3.4 | ~0.3 s |
| `t5_cauchy_interlacing_lambda1.py` | Honest λ₁ bounds (NOT universal)| §3.5 | ~1.7 s |
| `t6_clean_finger_move.py`          | Resolver terminates in N/2      | §3.6 | ~0.7 s |

Total: ~10 seconds.

## Design philosophy

Every test follows the same shape:

1. **Module docstring** declares: claim under test, reference paper /
   theorem, independent ground truth, test design, pass criterion, and
   **circular-logic guards**.

2. **Ground truth path** computed via a textbook method that does not
   depend on the claim (SymPy chain homology, closed-form curvature,
   independent eigendecomposition, etc.).

3. **Claim-under-test path** computes the same quantity via the recipe
   we want to ship. Receives only per-shard / per-chart data; no global
   shortcuts.

4. **Assertion**: truth == claim (or |truth − claim| < tol for numerical
   tests).

5. **Substantive non-triviality witness**: each test demonstrates that
   the cover/atlas/sharding is doing real work — e.g., per-chart Bettis
   differ from global Bettis (T1), per-chart metric values differ at
   shared points (T3), per-chart connections are gauge-inequivalent
   (T4). Prevents tautological passes.

## On red-first

Two of the six gates were RED on first run and required correction:

- **T2**: my naive 3σ extreme-value cap was too tight for max-of-600
  Gaussians. Fixed by using `sqrt(2 ln(N))` extreme-value scaling. The
  *substantive* slope check (first-order in ε) passed from the start.
- **T5**: my claim "min(per-shard λ₁) upper-bounds global λ₁" was
  **wrong for expanders**. Caught immediately by the random regular
  graph cases. Fixed by reformulating to honestly disclose that the
  naive bound is non-universal; the universal Weyl bound and the
  natural-clustering bound are both tested explicitly.
- **T6**: my first analog required `downstream ∩ unresolved = ∅` as a
  precondition. Spurious blocking. Fixed by re-reading Davis Thm 5.3:
  the Clean Finger Move's path-avoidance is about the *chosen path*,
  not the dependency graph. The engineering analog has FREEDOM to
  choose local-support resolution.

These red-then-green cycles are the point of the TDD discipline: claims
in the theory doc are gated on math we actually validated, not math we
intuitively expected to hold.

## Circular-logic guards (the discipline)

Each test has explicit guards documented in its docstring. The recurring
ones:

- **Truth must be independent of claim.** If both paths compute via the
  same routine, equality is tautological. We use textbook methods for
  truth, custom assembly for claim.
- **Substantive non-triviality.** Per-chart / per-shard data must
  actually differ from global data; otherwise the test doesn't
  exercise the claim. Explicit asserts in each test.
- **No silent failures.** Algorithms must report blocked / refused
  states explicitly. Pass criterion uses independent observables
  (residual count, eigenvalue ratio) not the algorithm's own
  termination flag.
- **Match expectation, not "always pass".** T5 has cases that are
  EXPECTED to fail (expander naive bound). Test passes when behavior
  matches expectation, not when bound holds universally.

## Adding a new gate

1. Create `tN_short_name.py` following the docstring template.
2. Declare the claim, ground truth source, test design, pass criterion,
   and circular-logic guards in the module docstring.
3. Implement two independent paths (truth + claim) and assert agreement.
4. Add a substantive non-triviality witness.
5. Add to `run_all.py`'s TESTS list.
6. Update this README table.
7. Reference from `poincare_to_sharding.md` §3.
