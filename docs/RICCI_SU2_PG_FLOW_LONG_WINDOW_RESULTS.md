# S3c: the SU(2) flow on a five-times-longer window (results)

2026-11-19.

- **Spec:** `docs/RICCI_SU2_PG_FLOW_LONG_WINDOW_SPEC.md` (72216bf, before any measurement).
- **Receipts:** `research/su2-long-window-20261119/`. Each run has a JSON and a `*.final.npy` state, and the
  verdicts are in `gates.json`, written by the committed `gates.py` without edits.
- **Kind:** research script, not an engine run.

## In plain English

On the window [0, 0.1], five times S3b's, every control passes, and the native force does lose order, as S3a
predicted (1.63 → 1.32). But the two-ring force loses order too, and more (2.93 → 1.62), so **P-long
(two-ring) fails**. The two-ring force stays ahead of native at n = 2 at every sample time (**P-cross passes**).
Its lead shrinks steadily, though, from 20% at t = 0.02 to 2% at t = 0.1.

Two facts explain most of the failure:

- **The pre-asymptotic n = 1 level.** At n = 1 the two-ring error is large early and then *falls*
  (0.069 → 0.057), so the n = 1 → n = 2 ratio shrinks with time. The spec named this risk: n = 1 → 2 is
  pre-asymptotic, so a pass would have been evidence, not a rate.
- **A shared late-time error.** At n = 2, the two forces' errors grow at almost the same late rate, and end
  within 2% of each other. Whatever dominates the error late in the window is common to both forces.

## Results

E(t) is the trajectory error (S3's measure); D(t) is the distance travelled. N = 40, h = 0.0025, moving metric,
q6.

| t | n = 1 native | n = 1 two-ring | n = 2 native | n = 2 two-ring | two-ring / native, n = 2 |
| --- | --- | --- | --- | --- | --- |
| 0.02 | 0.03502 | 0.06896 | 0.01132 | 0.00904 | 0.80 |
| 0.04 | 0.03777 | 0.06923 | 0.01384 | 0.01256 | 0.91 |
| 0.06 | 0.04053 | 0.06235 | 0.01582 | 0.01501 | 0.95 |
| 0.08 | 0.04392 | 0.05823 | 0.01751 | 0.01696 | 0.97 |
| 0.10 | 0.04755 | 0.05721 | 0.01905 | 0.01866 | 0.98 |

The observed order between n = 1 and 2 is p(t) = log2(E at n = 1 / E at n = 2):

| t | 0.02 | 0.04 | 0.06 | 0.08 | 0.10 |
| --- | --- | --- | --- | --- | --- |
| native | 1.63 | 1.45 | 1.36 | 1.33 | **1.32** |
| two-ring | 2.93 | 2.46 | 2.05 | 1.78 | **1.62** |

R(t) = E/D at t = 0.1: native 0.0321 / 0.0127, two-ring 0.0387 / 0.0124 (n = 1 / n = 2).

## Gates

| gate | threshold | measured | verdict |
| --- | --- | --- | --- |
| C-prefix | E(0.02) matches S3b's N = 8 to 10⁻⁶ | ≤ 2.0 × 10⁻¹¹ (all four) | pass |
| G-time | halving h changes E by < E/10 | ≤ 0.40% | pass |
| **P-long (native)** | p(0.1) < 1.63 − 0.25 = 1.38 | 1.32 | **PASS** |
| **P-long (two-ring)** | p(0.1) ≥ 2.94 − 0.25 = 2.69 | 1.62 | **FAIL** |
| **P-cross** | at n = 2, two-ring E < native E at every sample time | 0.80 to 0.98 of native | **PASS** |

Newton: every step converged.

- At most 13 iterations, with one mass refresh in each n = 2 run at N = 40 and none at N = 80.
- Force evaluations: 282–358 (N = 40) and 446–557 (N = 80).

## Reading

- **S3a's statement about native is borne out.** On a longer window more directions count as slow, and
  native's order drops (1.63 → 1.32).
- **S3a's forecast for two rings is not.** Its order drops further, so P-long (two-ring) fails as written. Most
  of that drop is n = 1's own transient: the two-ring error at n = 1 decays after t = 0.04, which no
  asymptotic statement covers. The n = 2 behaviour is the informative part:
  - over [0.06, 0.1] the errors grow at nearly the same rate (dE/dt ≈ 0.077 native, 0.085 two-ring);
  - they end within 2% of each other.
- **Hypothesis (not tested here).** A common error dominates late in the window. The obvious shared ingredient
  is the **native tangent mass B**, which both flows use (B ȧ = −force). Only the force differs between them.
  - A first-order mass error enters the velocity directly and accumulates over time, whatever the force.
  - This is the mass question of `docs/research/magnet-rule-variational-2026-11-19.md` §6 ("the target is
    created by the magnets"), met from the flow side.
  - **A cheap test:** the same n = 2 runs with B replaced by a recovered (two-ring) mass. If the late-time
    errors separate, the mass is the shared error.

## What this changes for *Reading the Curve*

The paper's sentence "On a longer window more directions count as slow, and the native force's stiff error
begins to matter" is now measured, and half right. Native's order does drop. But the two-ring force does not
pull away; its lead over native shrinks to 2% by t = 0.1 at n = 2. The flow claims stay as stated ("one
window, [0, 0.02]"). The longer window belongs in the limitations, with the mass hypothesis as the open
question.

## Not done

n = 3 on the long window; the recovered-mass test; windows beyond 0.1.

## Addendum: the two forces are wrong in the same direction (diagnostic, same evening)

Script: `scripts/ricci_su2_long_window_error_split.py`. Receipt: `research/su2-long-window-20261119/error_split.json`.

- **Method.** The script reads the saved final states and splits each error vector d = log(L⁻¹U) exactly as E
  does. C-E: every receipt's E is reproduced to 2 × 10⁻¹⁶.
- **No prediction was attached**, so this is evidence, not a gate.

| at t = 0.1 | n = 1 | n = 2 |
| --- | --- | --- |
| B-cosine of the native and two-ring error vectors | 0.81 | **0.988** |
| ‖d_two-ring − d_native‖ / ‖d_native‖ | 0.70 | **0.157** |
| fraction of the error in the 9 left-invariant directions (native / two-ring) | 0.28 / 0.34 | 0.12 / 0.13 |

N = 80 gives the same numbers to three digits.

At n = 2 the two forces end up wrong in nearly the same direction, though their instantaneous errors differ
fivefold. So the late error comes from what they share:

- the native (Whitney) mass B;
- the Whitney test that B defines;
- the moving-metric assembly;
- the reference.

**Replacing B alone is not a well-posed test.** The two-ring force's target is defined through the Whitney test,
and B is the Whitney mass. Swapping B while keeping the test would introduce a moment mismatch (magnet note,
Lemma B). The well-posed test builds the mass **and** the test from the same reading: option B of
`docs/research/magnet-rule-variational-2026-11-19.md`. It runs through M0, because the mass must be invertible on
rough modes, so the reading must be seated.
