# Relative leakage gate: all sixteen states pass

25 November 2026. The registered evaluation of the rule in the
[leakage-gate amendment](helike-leakage-gate-amendment-2026-11-24.md). **CUTOFF_LEAKAGE_GATE_PASSED at the four
controls. Not a spectral certificate. Full grid held.**

## Result in plain English

Trimming each saved state with the cutoff changes its residual by at most 3.08 × 10⁻⁵ of the residual itself,
against the registered limit of 10⁻³. That holds for all sixteen states, including the eight g² = 0.004 states
that could not clear the old, underived absolute line of 1e-10. Both numbers being compared are rigorous:
- the leakage upper bounds come from the Arb receipt;
- the residual lower bounds come from the helike-lab interval enclosures.

The comparison is exact rational arithmetic. The prediction recorded before any evaluation was "about 3 × 10⁻⁵,
every state passes". That is what happened.

## Order of events

1. **The amendment** was registered and pushed (12b70f7) before any further measurement.
2. **The implementation** (9244ed8) came next, pushed before any evaluation:
   - 10 tests, with a missing-module RED first;
   - 8 of 8 mutants caught by assertion: r_upper used for r_lower, a float accepted as r_lower, the q division
     dropped, the ℓ² term dropped, κ changed, the absolute line promoted back to a gate, the historical receipt
     relabelled, and "any state" in place of "all states".
3. **The first evaluation refused** at the endpoint-pairing check
   ([log](../../research/leakage-gate-20261125/refused-9244ed8.log)). The helike-lab reader had parsed three Arb
   endpoints below their exact upper bounds, by under one ulp, because the default `serde_json` parser is not
   correctly rounded. No gate value was computed. I did not loosen the check.
4. **The cause was fixed upstream** (feat/helike-lab 34b8860):
   - correctly rounded parsing;
   - endpoints computed upward from the exact dyadics and cross-checked against the decimal copy;
   - a regression test.

   The regenerated receipt changed 15 last digits of copied values. No sector bound and no q changed.
5. **The input was re-pinned** to the corrected receipt (fbfa958), pushed, and then evaluated.

## Numbers

| g² | Mq | worst state | ℓ upper | r lower | ℓ / r_lower ≤ |
| --- | --- | --- | --- | --- | --- |
| 0.004 | 6 | Helike ± | 4.869e-9 | 0.0487248 | 9.99e-8 |
| 0.004 | 8 | Helike ± | 2.713e-7 | 0.0088214 | 3.08e-5 |
| 0.002 | 6 | ground | 1.363e-27 | 0.0053943 | 2.53e-25 |
| 0.002 | 8 | Helike ± | 1.788e-25 | 0.0028093 | 6.36e-23 |

The full per-state table, with the projected residual bounds R = (r + ℓ)/q and the Kato–Temple enlargement
(2rℓ + ℓ²)/q², is in the
[receipt](../../research/leakage-gate-20261125/controls-fbfa958/receipt.json). The absolute 1e-10 line is still
reported per state, labelled as a diagnostic. It holds at g² = 0.002 and not at g² = 0.004. The historical
c31aab8e receipt is unchanged and keeps `cutoff_tail_gate_passed: false` under its own rule.

## What this closes, and what it does not

- **Closed:** the cutoff/leakage condition at the four controls. The tail-squared and normalisation budgets also
  pass.
- **Not closed:**
  - level identification and ordering, which need Kato–Temple, an enclosed Rayleigh quotient, and the separation
    hypothesis S registered first;
  - the method amendment for the registered grid;
  - the grid itself.

`certified: false`, `full_grid_released: false`.
