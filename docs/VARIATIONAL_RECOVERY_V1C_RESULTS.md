# V1c: V1 with only the three constant generators enforced (results)

2026-11-19. Spec: `docs/VARIATIONAL_RECOVERY_V1C_SPEC.md` (0cd228f, before the run).

- **Script:** `scripts/ricci_variational_v1.py --generators constants`. A syntax fix (738d334) was needed before the
  first successful run. The first launch failed at parse time, with no numbers produced; the predictions were not
  touched.
- **Receipts:** `research/variational-v1c-20261119/L{0,1}.json`.
- **Kind:** research script, not an engine run.

## In plain English

Dropping the 20 linear generators from V1's column condition removes the explosion. The change the solve needs
shrinks from a median ‖C‖ of 288 to 3.7 at n = 1, and from 65 to 6.8 at n = 2. The reconstruction term stays at its
base value: 5.2% and 3.8%, against V1's 432% and 61.6%.

But the constants alone do not remove the poke term. The test-variation term is 82% at n = 1, up from 43%, and 58% at
n = 2, down from 276%. Both outcomes are what the magnet note predicted:

- the first-order conflict comes from the linear moments (Lemma C);
- a reading with seams cannot be repaired by moment conditions alone (Lemma A).

## Results

| | n = 1 | n = 2 |
| --- | --- | --- |
| reconstruction term: C = 0 / V1 / **V1c** | 4.72% / 432% / **5.20%** | 3.67% / 61.6% / **3.82%** |
| test-variation term: C = 0 / V1 / **V1c** | 43.5% / 12.2% / **82.1%** | 276% / 1.16% / **58.4%** |
| total variational error: V1c (Whitney) | 83.5% (29.8%) | 58.3% (26.0%) |
| ‖C‖ median / max: V1 → **V1c** | 288 / 289 → **3.70 / 3.76** | 65.0 / 149 → **6.79 / 9.96** |
| curvature relative error: V1c | 3.68% | 0.87% |
| C-solve relative residual | 1.2 × 10⁻¹² | 1.4 × 10⁻¹¹ (V1 failed at 6.8 × 10⁻⁵) |
| constraint rows / rank | 426 / 360 | 3,372 / 2,880 |
| S3: small count (gauge), negative below −1, min physical | 21 (21), 0, 3.90 | 163 (163), 0, 3.99 |
| seconds | 29 | 265 (V1: 4,670) |

C-repro reproduces the two-ring variational force at C = 0: 45.4% and 276.3%.

## Predictions

| prediction | threshold | measured | verdict |
| --- | --- | --- | --- |
| **P-recon-c** | reconstruction ≤ 10% at n = 1 and 2 | 5.20%, 3.82% | **PASS** |
| **P-tv-c** | test variation ≥ 20% at n = 2 | 58.4% | **PASS** |

By the spec's table, both passing **supports the magnet note**: the linear moments caused V1's explosion, and the
seams keep the poke term alive. The cure is conformity, not more constraints.

## Notes

- **The solve is now easy.** The constraint system at n = 2 converges to 10⁻¹¹, where V1's stalled at 6.8 × 10⁻⁵.
  This is consistent with Lemma C: V1's system was nearly inconsistent, because exact rows and exact linear-moment
  columns cannot both hold.
- **The poke term moves in both directions.** At n = 1 it rises (43% → 82%); at n = 2 it falls (276% → 58%). The
  constant moments are a real share of the n = 2 poke term, but they are not the part that grows under refinement.
  The trend from n = 1 to n = 2 is now a fall rather than a rise. With two levels, that is not a rate.
- **Stability is untouched,** as for every energy-based force: no negative modes, and gauge counts exact.
- **Not done:** n = 3. Whether V1c's test-variation term keeps falling (58% → ?) is the one number that could still
  overturn the reading. If it fell below Whitney's 26% at n = 3 and kept falling, constants alone might suffice
  asymptotically. Given that the poke term carries h⁻¹, that is not expected.
