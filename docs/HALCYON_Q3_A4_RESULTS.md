# Q3 Amendment 4: results on fresh seeds k11–k19

2026-11-21. Scored by `e2e/probes/test_q3_a4.py`, committed at 5a71d341 before any of k11–k19 had been sampled, with
the frozen Amendment 4 probe `target-ym6/q3a4/correlator_probe.exe` (sha256 prefix bde9f81396de89c5).

- Receipts: `research/q3-scaling-20261119/q3a4_receipt.json` and `q4a4_receipt.json`.
- Spec: `docs/HALCYON_Q3_RUN_SPEC.md`, Amendment 4.
- Sampler: the extension's `lattice4d_ensemble.exe`, unchanged; 10,000 measurements per point, blocks of 100; seeds
  20261193 + 1000k.

## Verdict

**Outcome: failure, by the rule fixed in the spec.** One β (2.2986) passes both Q3-a and Q3-v, and fewer than two
passing β means failure. Q3-s has one point and no fit.

The failures are not new. They are the extension's failures, reproduced on fresh seeds:
- the 0⁺⁺ mass at the two finest spacings is high against LTW;
- the string tension at the larger volumes is high.

Amendment 4 changed neither, and its start bound changed no result anywhere. Per the spec's outcome rule, the next step
is longer statistics or an anisotropic lattice, **not another plateau rule**.

| gate | result | detail |
| --- | --- | --- |
| A4-path | reported | A2 path at every scalar channel. The Q3-C fallback engaged on three flux-loop channels: k12, k14 and k19. At k14 and k19 the fallback's plateau refused too |
| A4-null | pass | every shuffled series refuses on both paths, on all nine points |
| A4-start | reported: no effect | no channel had a window the start bound removed. With the frozen window rule, the GEVP plateaus all begin at t = 2 |
| Q3-a β = 2.2986 | pass | k11: a√σ pull −0.87, am₀₊₊ pull −0.52 |
| Q3-v β = 2.2986 | pass | k12 agrees with k11 in both channels |
| Q3-a β = 2.3715 | pass | k13: a√σ pull +1.36, am₀₊₊ pull +1.02 |
| Q3-v β = 2.3715 | **fail** | k14's flux loop refuses on both paths, so there is no volume check |
| Q3-a β = 2.4265 | **fail** | k15 am₀₊₊ 0.903(21) against LTW 0.834(13), pull +2.80 |
| Q3-v β = 2.4265 | fail | follows Q3-a. Also k16's a√σ is 0.2508(31) against 0.2388(9), pull +3.73 |
| Q3-a β = 2.5115 | **fail** | k17 am₀₊₊ 0.747(14) against 0.658(10), pull +5.22, and k17's a√σ pull is +3.36 |
| Q3-v β = 2.5115 | fail | follows Q3-a |
| Q3-s | fail | one passing β (2.2986; ratio 3.24(21)); no fit |
| k17 replicate of k9 | **reproduces** | k9's excess (pull 3.2) returns on a fresh seed, at pull 5.2 |
| k18 | reported | a√σ 0.1848(14) against 0.1768(8), pull +5.03. This is the large-volume string-tension excess again (k10 was +6.9) |
| k19 | reported | am₀₊₊ 1.256(52), pull +0.79. Its flux loop refuses at t = 2 under the start bound. A4-start has nothing to compare there |
| Q3-r (k6 rescored) | pass | rescored, not evidence |

## Every fresh point

Pulls are against LTW Table 5.

| k | β | L | aL√σ | a√σ | pull | am₀₊₊ (window) | pull | m₀₊₊/√σ |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 11 | 2.2986 | 10 | 3.61 | 0.3613(59) | −0.87 | 1.171(75) (2–3) | −0.52 | 3.24(21) |
| 12 | 2.2986 | 12 | 4.34 | 0.3619(81) | −0.58 | 1.142(43) (2–3) | −1.45 | 3.15(13) |
| 19 | 2.2986 | 14 | — | refused | — | 1.256(52) (2–3) | +0.79 | — |
| 13 | 2.3715 | 12 | 3.51 | 0.2922(29) | +1.36 | 1.031(32) (2–3) | +1.02 | 3.53(11) |
| 14 | 2.3715 | 16 | — | refused | — | 1.026(31) (2–3) | +0.90 | — |
| 15 | 2.4265 | 16 | 3.84 | 0.2401(20) | +0.60 | 0.903(21) (2–4) | +2.80 | 3.76(9) |
| 16 | 2.4265 | 20 | 5.02 | 0.2508(31) | +3.73 | 0.871(17) (2–3) | +1.72 | 3.47(8) |
| 17 | 2.5115 | 20 | 3.65 | 0.1824(15) | +3.36 | 0.747(14) (2–5) | +5.22 | 4.09(8) |
| 18 | 2.5115 | 24 | 4.44 | 0.1848(14) | +5.03 | 0.732(12) (2–6) | +4.84 | 3.96(7) |

## What this settles

- **The excesses are systematic, not seed noise.** Fresh seeds reproduce both patterns of the extension results:
  - the 0⁺⁺ mass is high at β = 2.4265 and 2.5115;
  - the string tension is high at the larger volume of each β, and also at k17.
- **The estimator change did nothing measurable.** The Q3-C fallback engaged only on flux loops, and two of those
  three still refused. The start bound never bound: under the frozen window rule every GEVP plateau already began at
  t = 2.
- **What it points to (not tested).** The extension's diagnosis stands, now on fresh data: with signals that last to
  t ≈ 3–5 at these volumes, the windows are short and early. Both the 0⁺⁺ mass at fine spacing and σ at large volume
  are read before the ground state dominates. That is a statistics and time-resolution problem, which is exactly why
  the spec routes failure to longer statistics or an anisotropic lattice, not to another plateau rule.

## Q4 on the A4 receipt

`research/q3-scaling-20261119/q4a4_receipt.json`. One point passes Q3-a and Q3-v (β = 2.2986).

- **C1 is vacuous, not survived.** The harness reports `survives: true` with one point, but with no pair there is
  nothing to test.
- **C2 holds** at that point: 3.24(21) ≥ 3.158 − 2σ.
- **K holds nowhere.**

## Ingest

k11–k19 are not yet ingested. That is the next mechanical step, under the step-2 gates with the A4 engine.
