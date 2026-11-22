# Pre-registration S6: the current sweep: results (22 November 2026)

> *She does not need much motion. She needs some.*

**Registered:** [current-sweep-preregistration-2026-11-22.md](current-sweep-preregistration-2026-11-22.md), commit
78414bc6, pushed before the sweep ran.
**Script and receipt:** [`stress-2026-11-22/current_sweep.py`](stress-2026-11-22/current_sweep.py) and
`current_sweep_receipt.json`, plus the printed summary `current_sweep_stdout.txt`.

These are my scripts, using the Floor paper's charge blocks (H + GG\*). Families j ≤ 40, cross field y = 0.

**Two corrections to the script before the run.** Both are recorded here because they matter.
1. **Her mirror twin.** Her value comes twice in every family: she and her mirror twin. The first version removed
   only one copy before comparing her with the rest, so she was compared with her own twin. I stopped it, removed both
   copies, and checked that her value is found in every family (the script asserts it).
2. **Exact ties.** At x = 0 the flat field is degenerate: families 0 and 12 have a state that exactly ties her. A tie
   counts as her still holding the floor (tolerance 10⁻⁸).

The stopped first run's process kept going and overwrote the receipt. That receipt was deleted, and the sweep was run
again from scratch. The receipt in the folder is from the corrected script.

## Scorecard

| id | prediction | verdict |
| --- | --- | --- |
| S6a | at the Berger end she is the floor of every family; she loses a family only as x nears 0, at a single threshold, and stays undercut beyond it | **held** at all 5 stretches |
| S6b | the families lost by x = 0 are exactly S1b's: 1–5 at t = 3, fewer as t grows, family 2 alone at t = 7 and 10 | **held** |

## How much current she needs (S6c, exploratory)

The threshold is x\* for each family she loses, and beside it the fraction of the Berger current x\*/x_B that is still
flowing when she loses that family. A smaller fraction means she needs less motion.

| t | Berger x_B | families lost at x = 0 | thresholds (fraction of the Berger current still flowing) |
| --- | --- | --- | --- |
| 3 | −0.5774 | 1, 2, 3, 4, 5 | family 2 at 65%, 3 at 49%, 1 at 42%, 4 at 33%, 5 at 17% |
| 3.55 | −0.8227 | 1, 2, 3, 4 | family 2 at 36%, 3 at 22%, 1 at 12%, 4 at 8% |
| 5 | −1.3416 | 2, 3 | family 2 at 13%, 3 at 3.3% |
| 7 | −1.8898 | 2 | family 2 at 5.2% |
| 10 | −2.5298 | 2 | family 2 at 2.0% |

- **Family 2 is always the first to go.** At every stretch it is the first family she loses as the current drains,
  and the last one she still needs current to hold.
- **The more the sphere is stretched, the less motion she needs.** At t = 3 she needs two thirds of the Berger
  current to keep every family. At t = 10 she needs 2% of it. But at every stretch tested she needs **some**: at
  x = 0 exactly, she loses at least one family.
- **At perfect rest she is never alone.** At x = 0, every family she does not lose has another state **exactly**
  tied with her, to about 10⁻¹³. That is family 0 and families 6 to 40 at t = 3, and similarly at every stretch.
  With as little as 2% of the Berger current flowing, she is strictly separate again in all of them: at t = 3 her
  margin is 0.07 in family 0, 0.11 in family 6 and 0.60 in family 40; at t = 10 it is 0.38, 0.57 and 1.66. So at
  rest, in every family, she is either undercut or merely tied. The smallest motion is what separates her.
  (Margins: `current_sweep.margin`, checked after the run.)

## What it says

This is the measurement behind Bee's reading of S1b. The flat field is the perfect resting state, with nothing in
motion along her circle, and it is the one place where she stops being singled out. Here is the same thing as a
number. With the cross field already off, she is the floor of every family while the field along her circle still
turns. She loses families only as that motion drains away, one threshold each, never more than five families, and
none above family 5. In every other family, at rest, she is exactly tied. **Her being chosen is carried by motion.
The perfect resting state is where the race stops: there she is undercut or tied in every family, and never alone.**

| claim | tag |
| --- | --- |
| a single threshold per family, losses only toward x = 0 | COMPUTED, 5 stretches, families j ≤ 40 |
| the families lost at x = 0 are S1b's | COMPUTED |
| family 2 needs the most current; the need shrinks with stretch (65% at t = 3, 2% at t = 10) | COMPUTED |
| at x = 0 she is exactly tied in every family she keeps, and separated by any motion (2% of the current suffices) | COMPUTED (ties to 10⁻¹³) |
| "her being chosen is carried by motion" | STRUCTURAL: Bee's reading, now measured on this family |
