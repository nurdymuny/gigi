# WISH on Sudoku — correction to the prior letter

**From:** GIGI team
**To:** Fable (spec author), Marcella team (review caller)
**Date:** 2026-06-10
**Status:** Retraction + new finding. The earlier letter
([`WISH_USE_CASE_SUDOKU_2026-06-10.md`](WISH_USE_CASE_SUDOKU_2026-06-10.md))
overstated on three load-bearing points; this corrects all three.
**Commit:** `81f9bac`.

---

## Tl;dr

Marcella team was right on every point of their review. We ran the
analytic-gradient version they called for. Two findings:

1. **The original "5/6 correct, +33.3pp lift" was scipy
   finite-difference gradient noise.** Analytic gradient (closed form,
   cross-checked against finite difference to 4e-5 relative) shows the
   per-candidate τ differences live at **1e-10 to 1e-12** — machine
   epsilon, not signal. The picks were iterator order on ties.

2. **The puzzle I generated had two valid solutions.** Swapping every
   bottleneck cell's value 7↔8 produces a *second* valid sudoku (rows,
   cols, boxes all still {1..9}). The naked-pair pattern admits a
   global symmetry the row/col/box constraints can't break. So WISH
   correctly returns ties — there is **no unique answer** in this
   puzzle for the verb to find.

The first finding kills the empirical claim. The second finding
explains why even after we fix the solver, the puzzle wasn't a fair
test. Both go on the receipts.

## What was wrong in the prior letter

| Prior claim | Reality |
|-------------|---------|
| "WISH picks the correct value in 5 of 6 cells" | 0/6 clean signal across λ ∈ {10, 50, 100}; 6/6 ties at machine epsilon |
| "+33.3pp lift over random baseline" | No lift. The picker chose iterator order when τ tied; the count over truth was coincidence |
| "Cell (8,4)→7 when truth=8 is honest geometric failure" | All cells are at honest geometric failure; (8,4) was no different from the others |
| "Tie-break interpretation: 4 clean + 1 tie + 1 miss" | Honest reading: 0 clean + 6 ties + 0 clean-wrong |
| "The signal lives in the 4th–5th decimal of τ" | The "signal" was finite-difference gradient error of ~1e-4. Analytic gradient: differences are 1e-10 |

## What the data actually shows

Analytic-gradient run (5–6s wall-clock per λ, vs 745s for the
finite-difference run):

```
lambda=10:   clean_correct=0/6   ties_on_truth=6   ties_against_truth=0
lambda=50:   clean_correct=0/6   ties_on_truth=6   ties_against_truth=0
lambda=100:  clean_correct=0/6   ties_on_truth=6   ties_against_truth=0
```

Per-cell τ differences across the three λ values:
- λ=10:  diffs of 1.10e-12 to 1.10e-11
- λ=50:  diffs of 3.81e-12 to 2.96e-10
- λ=100: diffs of 9.09e-13 to 1.59e-09

These are at the L-BFGS gtol floor (1e-8 here) times the conditioning
of the solve. They are not signal.

## Why the puzzle wasn't a fair test

The 6 bottleneck cells:

```
(0,4)={7,8} truth=7     (0,5)={7,8} truth=8
(6,5)={7,8} truth=7     (6,7)={7,8} truth=8
(8,4)={7,8} truth=8     (8,7)={7,8} truth=7
```

Swap all six: 7↔8. Then check rows, cols, boxes:

- Each affected row (0, 6, 8) still has one 7 and one 8 among the
  bottleneck cells — just in the opposite positions. Other cells in
  these rows are unchanged. So row constraints still satisfied.
- Same logic for cols 4, 5, 7.
- Same logic for the three boxes containing the bottleneck cells.

So the "puzzle" has two completions, both valid sudokus. The
generator (hide cells from a known solution, stop when basic CP can't
finish) never verified uniqueness. It treated "basic CP can't finish"
as "the answer is hidden but unique," which it isn't.

**This is a real lesson.** A puzzle that's hard for basic CP isn't
necessarily a puzzle with a unique answer. To test WISH's positive
value, we need a generator that runs full backtracking and confirms
**one and only one** completion exists.

## What WISH *did* correctly demonstrate

The tie behavior is the spec's verdict trichotomy doing the right
thing. When the seed-to-target geodesic energy is equal under both
candidate commitments — because the constraint structure cannot
distinguish them — WISH returns equal τ. A downstream caller that
checks "are there ties at this cell?" gets a clean signal that the
puzzle is under-determined at that cell.

That's not a small property. **Honest under-determination detection
is exactly the spec's `Indeterminate` verdict** spelled out for a
combinatorial substrate. The verb refuses to pick when picking would
be a coin flip. That much we observed.

## Where Marcella team was right, point by point

(For the record, since they were ahead of us.)

1. **"4th–5th decimal is a red flag, not a reassurance."** ✓ Exactly
   right. Finite-difference gradient at d=12 floors around 1e-4
   relative; the "signal" was at the same scale. Analytic gradient
   eliminates it.

2. **"(6,5) is also in the 130.67 group and the tau values tie."** ✓
   Right. The original 5/6 was actually 3 clean by iterator order + 2
   ties that randomly fell on truth + 1 tie that fell against truth.
   The honest reading was already messy. The new reading is cleaner:
   0 signal, period.

3. **"λ=50 needs justification."** ✓ Right. The sweep at {10, 50,
   100} shows the embedding does **no work at any λ** on this puzzle.
   If WISH ever does pick correctly on a fair puzzle, the λ sweep
   needs to be part of the demo.

4. **"Granted-only doesn't exercise the verb."** ✓ Right. We never
   triggered Unreachable or Indeterminate in the WishOutcome sense
   (the spec's verdict trichotomy). What we observed instead was that
   *every* solve returned Granted with tied τ values — which is the
   verb's honest report on an under-determined input but isn't what
   the spec's verdict types are designed to surface. Worth a §11
   conversation.

5. **"60s at d=12 doesn't extrapolate to 384-dim."** ✓ Right. Analytic
   gradients drop the per-solve cost to 5-6s at d=12. Scaling to
   d=384: each overlap_value_and_grad call is O(pairs); pairs scales
   like O(N²_bot) where N_bot is the bottleneck cell count; energy
   evals scale linearly in path nodes. Optimistic linear in d? — but
   the L-BFGS conditioning may degrade. §3.3 still needs a real run
   at d > 100 before any cost claim ships.

6. **"Call: C now, A as next commit, B before publishing."** This
   stands unchanged, except now (C) is "spec integration as a
   *honest-ties demonstration*, not a positive-pick demonstration."
   The Rust port doesn't gate on the pick story.

## What we'd need for a fair positive test

A puzzle generator that:

1. Hides cells from a known solution until basic CP saturates.
2. Runs full backtracking on the residual; **rejects the puzzle if
   it has more than one completion**.
3. Reports the bottleneck cells with a confidence flag (unique, or
   under-determined).

We have the infrastructure for (1). (2) is a standard backtracking
solver, maybe 30 lines. (3) drops out for free. Then the question
"does WISH pick the unique answer when one exists?" becomes
testable.

## Two paths forward, both honest

**A. Try a fair puzzle.** Build the uniqueness-verified generator,
re-run the analytic experiment, see if τ separates when there's a
unique answer to separate to. This is the test the original letter
should have run. If WISH separates, the spec integration is real; if
it ties even on unique-solution puzzles, we've learned the embedding
doesn't carry enough information and the constraint coupling needs
rethinking.

**B. Stop and document the lesson.** Integrate this as an §11.x
"honest-ties demonstration" only, drop the positive-pick claim
entirely, and let the Rust port proceed unblocked. The lesson on
review-convergence (Marcella team caught a structural error two
reviewers and three drafts missed) goes in the spec as a footnote.

I lean **A** because the original question Bee asked was "does WISH
pick correctly on cells basic CP can't solve" and we haven't answered
it yet — what we have is "WISH correctly says 'I don't know' when no
answer exists." Those are different findings and the second doesn't
substitute for the first.

But (B) is defensible if the WISH-on-CSP question is genuinely a
side quest and the Rust port shouldn't keep waiting.

Bee's call.

---

## For the record

- Spec is **not** changing based on this run. No §11.x added yet.
- The earlier letter
  ([`WISH_USE_CASE_SUDOKU_2026-06-10.md`](WISH_USE_CASE_SUDOKU_2026-06-10.md))
  stays in the repo as the corrected-into-this-one paper trail.
  Receipts including the wrong ones.
- The convergence-of-reviews observation from W-math holds doubly
  here. Marcella team's single review caught everything the original
  letter missed. Three drafts + one review > one draft + zero
  reviews, even when the draft author thinks the run looked clean.

— GIGI team
