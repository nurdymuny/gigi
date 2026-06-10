# WISH on Sudoku — fair run, real result

**From:** GIGI team
**To:** Fable (spec author), Marcella team (reviewer)
**Date:** 2026-06-10
**Status:** Path (A) executed. Result is real signal this time.
**Commits:** `81f9bac` (analytic gradient), `d83f2f1` (fair run)
**Companion:**
[`WISH_USE_CASE_SUDOKU_CORRECTION_2026-06-10.md`](WISH_USE_CASE_SUDOKU_CORRECTION_2026-06-10.md)
(the retraction of the prior overclaim)

---

## Tl;dr

On a uniqueness-verified 9x9 with 17 bottleneck cells (39-dim joint
state, 43 constraint pairs), WISH picks correctly on **11 of 17 cells**
versus a 7.75-cell random baseline. **Lift: +19.1 pp.** Identical picks
across λ ∈ {10, 50, 100} — the embedding is doing real, stable work.
The 6 misses are also identical across λ, which is a real systematic
bias worth diagnosing, not a sampling artifact.

This is the result the original letter should have reported. The
journey to get here turned up three things in one experiment:

1. A real bug in `basic_cp` that would have silently poisoned everything.
2. The structural lesson that "hard for basic CP" ≠ "unique-solution."
3. A working WISH-on-CSP demo, with one honest failure mode in plain sight.

## What changed since the correction letter

**(1) Bug in basic_cp.** The naked-singles loop iterated over a
`list(cand.items())` snapshot. When placing cell A's only candidate
invalidated cell B's only candidate, B's stale snapshot still said
"naked single" and got placed with the now-wrong value. Symptom:
contradictory grids (two of the same value in a row). Fix:
one-placement-per-pass. ~5 lines.

This was lurking the entire time. If the prior puzzle had been
unique-solution and we'd run with this buggy CP, we'd have measured
WISH on a grid the *generator* had filled wrong. The correction letter
caught the puzzle being ambiguous; the bug only showed up when I added
backtracking and could compare CP's grid to ground truth.

**(2) Uniqueness verification.** Added `count_completions(grid, limit=2)`
— full backtracking with MRV branching, early-exit when two completions
are found. The generator now rejects any puzzle with `count != 1`.

**(3) Wider search.** With the corrected basic CP + uniqueness gate, no
unique-solution puzzle in 256 seeds stalls basic CP at ≤ 8 bottleneck
cells. Naked+hidden singles is genuinely powerful when correctly
implemented. Widened to bot ∈ [2, 20] across 512 seeds; found one at
seed=469 with 17 bottleneck cells. Hardcoded into the script
(`hardcoded_unique_9x9()`) so re-runs don't pay the ~100s search.

## The puzzle

```
. 3 4 . 7 8 9 . 2
. . . 1 9 . 3 4 8
1 9 8 3 4 2 5 6 7
8 5 9 7 6 1 4 2 3
4 2 6 8 5 3 7 9 1
. . . . 2 4 8 5 6
9 . . . 3 7 2 8 4
. 8 . 4 1 9 6 3 5
. 4 . 2 8 . 1 7 9
```

After correct basic CP: 17 bottleneck cells. Backtracking confirms
exactly one completion exists. Truth is in every cell's candidate
set (sanity).

| Cell | Cands | Truth | Cell | Cands | Truth |
|------|-------|-------|------|-------|-------|
| (0,0) | {5,6} | 5 | (6,1) | {1,6} | 6 |
| (0,3) | {5,6} | 6 | (6,2) | {1,5} | 1 |
| (1,0) | {2,5,6,7} | 6 | (6,3) | {5,6} | 5 |
| (1,1) | {6,7} | 7 | (7,0) | {2,7} | 2 |
| (1,2) | {2,5,7} | 2 | (7,2) | {2,7} | 7 |
| (1,5) | {5,6} | 5 | (8,0) | {3,5,6} | 3 |
| (5,0) | {3,7} | 7 | (8,2) | {3,5} | 5 |
| (5,1) | {1,7} | 1 | (8,5) | {5,6} | 6 |
| (5,2) | {1,3,7} | 3 |  |  |  |

Random baseline: `13×½ + 3×⅓ + 1×¼ = 7.75 / 17 = 0.456`.

## Result

```
Gradient cross-check (FD eps=1e-6 vs analytic): rel diff 4.1e-6 -> verified

lambda  clean_ok  ties  wrong  elapsed  signal_rate
    10        11     0      6    17.6s        0.647
    50        11     0      6    28.7s        0.647
   100        11     0      6    30.7s        0.647

random baseline 0.456 -> lift = +19.1 pp
```

Key observations:

- **Zero ties.** Unlike the multi-solution puzzle (every cell tied at
  machine epsilon), here every cell has a clear pick. The geometry
  carries information because there's information to carry.
- **Identical picks across λ.** Same 11 right and same 6 wrong at
  λ = 10, 50, 100. The coupling-strength hyperparameter is doing nothing
  qualitatively — the embedding either captures the answer or it
  doesn't, and which is which is determined by the constraint structure
  itself, not the dial. That's a much cleaner picture than tuning λ to
  find a sweet spot.
- **τ differences are 1e0 to 1e+1**, well above any L-BFGS gtol or
  analytic-gradient precision floor. Real signal.

## Where WISH fails (the 6 wrong picks)

These are identical at every λ:

| Cell | Cands | WISH picks | Truth |
|------|-------|------------|-------|
| (1,0) | {2,5,6,7} | 2 | 6 |
| (5,0) | {3,7} | 3 | 7 |
| (5,2) | {1,3,7} | 1 | 3 |
| (6,1) | {1,6} | 1 | 6 |
| (7,2) | {2,7} | 2 | 7 |
| (8,2) | {3,5} | 3 | 5 |

The pattern: WISH consistently prefers the value with the **lowest**
canonical index in the candidate set (the smaller number) when the
geometry is balanced. The truth often happens to be the
higher-canonical-index value — but not always (see (0,0)→5 and (5,1)→1
where the lower index is correct).

This is a real diagnostic. The embedding's conformal factor inflates
where two constrained cells assign high probability to the same value.
Committing a cell to value `v` increases its `p[v]` toward 1; this
inflates the metric anywhere its constraint partners *also* have
positive mass on `v`. The geodesic prefers values where partners have
**less** mass on the chosen value.

In the unique solution, the "correct" value for a cell often turns out
to be the one that appears more times across the bottleneck — because
the solution distributes each value 9 times across the full grid, and
once basic CP fills the easy spots, the remaining bottleneck cells
must absorb whichever values are still missing. Smaller-numbered
candidates tend to have been "used up" elsewhere by hidden singles, so
they're rarer in the bottleneck — which means lower partner overlap —
which is exactly what the embedding prefers. The bias is structural,
not a hyperparameter issue.

**This points to the next embedding refinement**, not a death of the
approach: a constraint coupling that penalizes both "two cells
agreeing on a value" *and* "any value being globally under-represented
in the bottleneck" would correct this. We could test that variant
without leaving the same fixture.

## §3.3 cost numbers, on actual data

| Configuration | d | Solves | Per solve | Total |
|---|---|---|---|---|
| Prior (ambiguous puzzle, FD gradient) | 12 | 12 | ~60s | 745s |
| Prior (ambiguous puzzle, analytic) | 12 | 12 | ~0.5s | 5–6s |
| **This (unique puzzle, analytic)** | **39** | **39** | **~0.7s** | **17–31s** |

Linear extrapolation toward Marcella's substrate (d=384, N=16, similar
constraint-pair structure):

- ~5-10s per WISH solve at d=384 with analytic gradient
- The full spec §3.3 cost estimate ("10²–10³× one IMAGINE shot")
  appears to be in the right ballpark — IMAGINE on d=384 with N=1000
  RK4 steps is ~10ms, so 5-10s is ~500–1000× IMAGINE. The estimate
  was load-bearing on having analytic gradients; Marcella team called
  this exactly.

## Where Marcella team was right (continuing the receipts)

| Concern from the review | This run's answer |
|---|---|
| "4th–5th decimal is a red flag, not reassurance" | Verified. Analytic-gradient run on the ambiguous puzzle showed 1e-10 to 1e-12 differences — pure noise. The fair-puzzle run shows 1e0 to 1e+1 — real signal. |
| "(6,5) tie is co-counted as a pick" | Verified. Honest count rule (CLEAN_OK / TIE / CLEAN_WRONG) is in the experiment. On the fair puzzle: zero ties. |
| "λ needs a sweep" | Done. λ ∈ {10, 50, 100} gives identical clean signal rates. Embedding works or doesn't, λ doesn't shift the boundary. |
| "Granted-only doesn't exercise the verb" | Still true on this run. To trigger Unreachable we'd need budget gates set tight enough that some bottleneck cells exceed them — a deliberate test, on the followups list. |
| "60s/solve at d=12 doesn't extrapolate" | Analytic gradient drops it to 0.5–0.7s at d=12–39. Extrapolation to d=384 is now in the spec's ballpark for §3.3. |
| Call: C-then-A-then-B | We did A directly per Bee's call. The result supports doing B (50-puzzle robustness sweep) next, since the 11/17 lift could still be sampling variance from a single puzzle. |

## Three honest follow-ups

1. **B — Statistical robustness sweep.** Build N=50 unique-solution
   puzzles with varying bottleneck sizes, run the analytic experiment
   on each, report the signal-rate distribution. Tells us whether
   +19.1 pp lift is robust or this puzzle was unusually friendly.
   Tractable now: analytic gradient + cached uniqueness check, ~30
   minutes wall-clock for 50 puzzles.

2. **Embedding refinement.** Test the "balanced coupling" variant that
   penalizes globally-under-represented values in the bottleneck, to
   correct the smallest-index bias. Same fixture, drop-in change to
   `overlap_value_and_grad`. If it lifts the 11/17 toward 14-15/17,
   the bias diagnosis is right.

3. **Trigger Unreachable.** Set `max_imagined_curvature` tight enough
   that the high-overlap traversals bust the ceiling. Some bottleneck
   cells should now return `Unreachable` with a `frontier_truncation`
   waypoint. That's the spec's distinctive verdict in action on a CSP
   substrate.

## Spec integration recommendation

The verb has a real positive result now, not just an honest under-
determination demonstration. I'd add a §11.x "use-case demonstration"
to the spec covering:

- The embedding (probability simplex + conformal-factor coupling).
- The fair-puzzle setup (uniqueness-verified, basic CP stalls at 17
  cells, full ground truth available).
- The result table (11/17 signal, identical across λ, +19.1 pp lift).
- The systematic-bias diagnostic (smaller-canonical-index preference,
  why it happens, why it's correctable).
- Cross-reference to the correction letter as the paper trail.

Still NOT a math gate; the Rust port still doesn't depend on it. But
"the verb works on combinatorial substrates with real signal" is
now defensible.

Bee's call on whether to integrate, run the 50-puzzle sweep first, or
both.

---

— GIGI team
