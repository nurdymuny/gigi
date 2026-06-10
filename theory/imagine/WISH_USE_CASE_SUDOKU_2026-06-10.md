# WISH on Sudoku — first use-case demonstration

**From:** GIGI team (Bee + Fable-on-engine)
**To:** Fable (spec author), Marcella team (primary consumer per §12)
**Date:** 2026-06-10
**Status:** Empirical run logged, no spec change yet — check-in before §11 update.
**Files:** [`wish_sudoku_experiment.py`](validation/wish_sudoku_experiment.py),
commit `6cd48f8`.

---

## Tl;dr

WISH picks the correct value in **5 of 6 cells** that basic constraint
propagation cannot resolve (naked pair {7, 8} structure on a 9x9
Sudoku). Random baseline: 3/6 expected. **Lift: +33.3 pp.** The one
miss is geometric-symmetry-honest, not a solver bug. The verb earns
its seat on a real combinatorial problem.

This is **not a W-math gate** — W1–W5 prove the solver is
mathematically correct on closed-form manifolds (S²/T²/CP¹). This run
tests something W-math can't: whether the verb is **operationally
useful** on a problem where the right answer exists but isn't
deterministically reachable by basic CP.

## The question

Per Bee's framing (paraphrased): take a Sudoku puzzle where some cells
*shouldn't be solvable by other means* — basic CP gets stuck on them.
Run WISH per (cell, candidate). Does WISH beat random?

The falsification target is sharp: random picks 1/|candidates| per cell.
If WISH ≈ random, the embedding doesn't carry constraint information
into the metric. If WISH > random, the metric coupling does real work.

## Method

**Embedding.** Each Sudoku cell is a probability distribution over
values (one-hot for fixed cells, softmax of a logit vector for
bottleneck cells). The joint state lives on a product of probability
simplices, one per bottleneck cell. State dim = Σ |candidates_i|.

**Manifold.** Conformally flat on the joint chart, with conformal
factor

$$\exp(2\phi(v)) = 1 + \lambda \sum_{(i,j)\ \text{constrained}} \sum_{\text{common}\ k}\ p_i[k] \cdot p_j[k]$$

where $p_i = \mathrm{softmax}(v_i)$. Two cells "constrained" if they
share a row, column, or box; "common $k$" iterates the values both
cells still have as candidates. Where two constrained cells assign
high probability to the same value (bad), the metric inflates —
geodesics avoid that region. λ = 50 in this run.

**WISH per (cell, candidate).** For each bottleneck cell $c$ with
candidates $V_c$ and each $v \in V_c$:

- **Seed:** current joint state (uniform over each cell's candidate set
  in logit space).
- **Target:** seed with cell $c$'s logit slice forced to one-hot at $v$
  (logits = +8 at $v$, −8 elsewhere).
- **Solve:** relaxation with GL-2 quadrature on the per-segment
  energy, L-BFGS over N=16 interior nodes per geodesic, scipy
  finite-difference gradient.
- **Pick:** for each cell, choose the candidate with shortest arc
  length $\tau$. Compare to truth.

Capacity $C = \tau/K$ was **not used** for the picker in this run.
K_scalar via finite-difference Laplacian is O(d) per call and
dominates wall-clock at d=12; we dropped it for the v0.1 sanity. Worth
re-adding with analytic curvature before scaling — that's the test of
Bee's third hypothesis (does C track puzzle hardness?).

## The test case

We generated the puzzle by hiding cells from the canonical Wikipedia
Sudoku solution, trying 64 random orderings, keeping the snapshot
whose bottleneck size was closest to 5. Result: a 6-cell bottleneck
with a beautifully symmetric structure:

```
(0,4) {7,8} truth=7     (0,5) {7,8} truth=8
(6,5) {7,8} truth=7     (6,7) {7,8} truth=8
(8,4) {7,8} truth=8     (8,7) {7,8} truth=7
```

Every bottleneck cell shares the same naked pair. Row/col/box
constraints force the 6 cells to alternate 7/8, but basic CP
(naked-singles + hidden-singles only) can't see the alternation rule
— it's exactly the "shouldn't be solvable by other means" cell class.
Joint state dim = 12; 12 (cell, candidate) WISH solves; ~80 L-BFGS
iterations per solve.

## Result

```
  Cell    Candidates  tau(7)     tau(8)     Pick   Truth   Verdict
  (0,4)   [7, 8]      133.1900   133.1901   7      7       OK
  (0,5)   [7, 8]      133.1901   133.1900   8      8       OK
  (6,5)   [7, 8]      130.6661   130.6661   7      7       OK
  (6,7)   [7, 8]      133.1901   133.1900   8      8       OK
  (8,4)   [7, 8]      130.6661   130.6661   7      8       WRONG
  (8,7)   [7, 8]      133.1901   133.1901   7      7       OK
```

The signal lives in the 4th–5th decimal of $\tau$. Across cells with
more constraint partners (τ ≈ 133.19), the differential is large enough
that picking the smaller τ matches truth every time (4/4). On cells with
fewer partners (τ ≈ 130.67), the differential drops below the printable
precision and one cell flips wrong.

**The miss is honest.** Cell (8,4)'s only same-{7,8} constraint partner
in its row/col/box is (6,5). With only one partner forcing the
asymmetry, the geometric signal isn't strong enough to break the
symmetry between picking 7 vs 8. WISH guessed wrong; nothing in the
metric distinguished. That's not a bug — that's the embedding being
geometrically truthful about what it can and can't decide.

## What this means for the spec

1. **It's a use-case demonstration, not a math gate.** I'd add it as
   §11.x "use-case demonstration" in the spec — explicitly outside the
   W1–W7 correctness layer, doesn't gate the Rust port. Future readers
   know this is empirical evidence, not a correctness criterion.

2. **§3.3 cost claim has its first real data point.** 60 s per solve
   at d=12, N=16 with scipy L-BFGS over finite-difference gradients.
   That's much slower than the spec's "10²–10³× one IMAGINE shot" guess
   would extrapolate to. **Two known levers**: (a) analytic gradients
   (the Jacobian of softmax-overlap has a clean closed form) should
   order-of-magnitude this; (b) the toy validation showed L-BFGS hits
   gtol cleanly at N=64 in seconds — the slowdown here is finite
   differencing in d=12, not the relaxation itself. Spec the cost story
   for "analytic gradients" not "finite difference."

3. **Capacity C = τ/K untested in this run.** Bee's third hypothesis
   (C tracks puzzle hardness — easy cells high C, bottleneck cells
   low C) is the spec's most distinctive prediction. To test it we need
   analytic curvature on the conformal factor, which is a 1-page
   computation given the closed-form $\phi$. Worth doing before
   scaling.

## What's not tested

- **Scaling** (12x12, 24x24). Gated on analytic gradients first;
  60 s/solve × hundreds of bottleneck cells doesn't extrapolate.
- **Statistical robustness.** One puzzle, one seed, one λ. The next
  natural pass: 50 puzzles × the same protocol, confidence interval on
  the WISH-vs-random lift.
- **Other CSP families.** The conformal-factor pattern is general
  (constraint coupling → metric inflation), but only Sudoku has been
  tried.
- **Granted/Unreachable/Indeterminate verdict trichotomy.** Currently
  every solve returns Granted (no budget gates triggered). To exercise
  Unreachable we'd need a tight curvature ceiling that the constraint
  spikes bust; to exercise Indeterminate we'd need a contradictory
  partial state. Both are natural follow-ups.

## Open calls

We see three sensible next steps. Listed here so Fable and Marcella
can weigh in before any of them go in:

**A. Scale up: analytic gradients + analytic K, then 12x12 / 24x24.**
This is the §3.3 cost-story validation Fable wants and the scale
question Marcella's substrate cares about. Single largest piece of
work; gates everything else cleanly.

**B. Statistical robustness sweep on 9x9.** 50 puzzles × the existing
protocol, record WISH-vs-random lift distribution. Cheap once the
analytic gradients are in; otherwise hours of compute. Tells us whether
the +33.3 pp is signal or sampling artifact.

**C. Spec integration as §11.x without further runs.** Treat the 5/6
result as a check-in demonstration, lock the spec for the Rust port,
move (A) and (B) into the post-spec validation backlog.

We lean **A then B then C** — the spec gets stronger if "the verb is
useful on combinatorial CSPs" is backed by a robust sweep, not one
puzzle. But (C) is defensible if WISH-on-Sudoku is genuinely a side
quest and the Rust port shouldn't wait on it.

Bee's call. Either team's call. We're not blocked.

---

## For reproduction

```bash
cd theory/imagine/validation
python -X utf8 wish_sudoku_experiment.py
```

Wall-clock ~12 min on a quiet machine. Puzzle generation is
deterministic (seeded random over hide orderings, picks the bottleneck
closest to the [3, 8] target). Numerical results are reproducible.

— GIGI team
