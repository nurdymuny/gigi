# V1c: V1 with only the three constant generators enforced (spec)

2026-11-19. Written before the run.

- **Source.** The falsification test proposed in
  `docs/research/magnet-rule-variational-2026-11-19.md` §5.
- **Script.** `scripts/ricci_variational_v1.py --generators constants`. This is V1 with one change: in the column
  condition, the enforced generators are the 3 right-invariant constant 2-forms only. The 20 linear generators are
  dropped. The objective (minimum change), the two rings, the base fit and the lift are unchanged.
- **Receipts.** `research/variational-v1c-20261119/L{0,1}.json`.
- **Kind.** Research script, not an engine run.

## What it tests

Lemma C of the note claims two things:

- Exact rows on closed linear fields, together with column conditions on the **linear** generators, conflict at first
  order. That conflict is what forced V1's change into the field's own curvature (reconstruction term 432% and 61.6%).
- With a per-cell, non-conforming reading, the **constants alone** cannot remove the test-variation term, because the
  poke term sits under a derivative (Lemma A, interface terms).

## Baselines (V1 receipts)

| | reconstruction, n = 1 / n = 2 | test variation, n = 1 / n = 2 |
| --- | --- | --- |
| C = 0 (two-ring variational force) | 4.72% / 3.67% | 43.5% / 276% |
| V1 (23 generators) | 432% / 61.6% | 12.2% / 1.16% |

## Predictions (fixed now)

- **P-recon-c.** The reconstruction term is at most **10%** at n = 1 and at n = 2. Dropping the linear generators
  removes the first-order conflict.
- **P-tv-c.** The test-variation term at n = 2 is at least **20%**. Constants alone do not remove the poke term of a
  non-conforming reading; for comparison, V1 reached 1.16%.

## Reading the outcome

| P-recon-c | P-tv-c | reading |
| --- | --- | --- |
| pass | pass | Supports the note: the linear moments caused the explosion, and the seams keep the poke term alive. The cure is conformity (the magnet rule), not more constraints. |
| fail | — | The conflict is not specific to the linear generators. Lemma C is not the explanation, or not the whole explanation. |
| pass | **fail** | Constants suffice even without conformity: the seams matter less than Lemma A says. In that case V1c is itself a variational candidate, and its BW05 error and S3 decide it. |

**Reported, not gated:**

- the total variational error, against Whitney's 29.8% and 26.0%;
- S3 (negative count, small count against the gauge counts 21 and 163);
- C-solve residual, ‖C‖ (median and max), constraint rows, rank and seconds.
