# Q3 extension and Amendment 3 (Q3-C): results

2026-11-20. Scored by `e2e/probes/test_q3_extension.py` (committed before any extension correlator was computed)
with the frozen probe `target-ym6/q3ext/correlator_probe.exe` (sha256 034b57b2d7dde5a8…). Receipt:
`research/q3-scaling-20261119/q3ext_receipt.json`. Spec: `docs/HALCYON_Q3_RUN_SPEC.md`, "Amendment 3 and the
extension run". k7 was peeked earlier and logged (`k7_peek_receipt.json`).

## Verdict

**Amendment 3 fails C-harm. With the Q3-C rule, only β = 2.2986 passes both Q3-a and Q3-v, so Q3-s has one point and
no fit.** Q3-C did the job it was written for (C-job passes at k8, where the Amendment-2 GEVP refused), but it moves a
result it should have left alone, and rescoring k0–k6 under Q3-C loses two of the three β that passed before.

| gate | result | detail |
| --- | --- | --- |
| C-job | pass | k8 0⁺⁺ = 1.234(43) against LTW 1.212(22), pull 0.46. The Amendment-2 GEVP refused at k8, so this tests the fix, not only correctness |
| C-null | pass | every channel refuses on its shuffled series at k7–k10; no vacuous channel |
| C-harm | **fail** | k10 flux loop: Amendment 2 gives 0.7720(107), Q3-C 0.7937(108), a shift of 2.0σ (the gate allows 1σ). Same window (2–4) in both. Passes at k7 0⁺⁺, k9 (both channels, identical), k10 0⁺⁺; not applicable where Amendment 2 refused (k7 flux loop, k8 both) |
| Q3-a β = 2.2986 | pass | k0 |
| Q3-v β = 2.2986 | pass | k0 with k7 (k7 replaces k1, which stays reported as failed) |
| Q3-a β = 2.3715 | pass | k2 |
| Q3-v β = 2.3715 | **fail** | k3's flux loop now refuses (no plateau, at t = 2). Under Amendment 2 it gave 1.319(45) |
| Q3-a β = 2.4265 | **fail** | k4 am₀₊₊ = 0.908(22) against LTW 0.834(13), pull 2.95. Under Amendment 2: 0.888(25), pull 1.9 |
| Q3-v β = 2.4265 | fail | follows Q3-a |
| Q3-a β = 2.5115 | **fail** | k9 am₀₊₊ = 0.714(14) against LTW 0.658(10), pull 3.2. Q3-C and Amendment 2 agree here to 10⁻¹³, so this is not Q3-C |
| Q3-v β = 2.5115 | fail | follows Q3-a; also a√σ differs between k9 (0.1762(43)) and k10 (0.1868(12)), pull 2.4 |
| Q3-s | fail | one passing β (2.2986, ratio 3.31(16)); no fit |
| Q3-r | pass | k6 |
| Q3-n | pass | all eleven points |

## Every point under the Q3-C rule

Pulls are against LTW Table 5 where a calibration exists.

| k | β | L | aL√σ | a√σ | pull | am₀₊₊ | pull | m₀₊₊/√σ |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | 2.2986 | 10 | 3.64 | 0.3642(67) | −0.37 | 1.206(52) | −0.10 | 3.31(16) |
| 1 | 2.2986 | 14 | 5.38 | 0.3842(20) | 6.56 | 1.152(38) | −1.36 | 3.00(10) |
| 7 | 2.2986 | 12 | 4.39 | 0.3661(74) | −0.08 | 1.256(41) | 0.94 | 3.43(13) |
| 8 | 2.2986 | 14 | 5.41 | 0.3862(19) | 7.41 | 1.234(43) | 0.46 | 3.20(11) |
| 2 | 2.3715 | 12 | 3.50 | 0.2913(29) | 1.08 | 1.046(34) | 1.36 | 3.59(12) |
| 3 | 2.3715 | 16 | — | refused | — | 1.010(26) | 0.51 | — |
| 4 | 2.4265 | 16 | 3.89 | 0.2430(24) | 1.67 | 0.908(22) | 2.95 | 3.74(9) |
| 5 | 2.4265 | 20 | 4.91 | 0.2453(30) | 2.09 | 0.923(19) | 3.93 | 3.77(9) |
| 6 | 2.3 | 16 | 6.28 | 0.3926(24) | — | 1.191(38) | — | 3.03(10) |
| 9 | 2.5115 | 20 | 3.52 | 0.1762(43) | −0.14 | 0.714(14) | 3.22 | 4.05(13) |
| 10 | 2.5115 | 24 | 4.48 | 0.1868(12) | 6.90 | 0.663(21) | 0.21 | 3.55(11) |

## What the failures have in common (diagnosis after scoring; not tested)

Two patterns, both visible in the table and neither caused by Q3-C alone:

1. **The larger volume's string tension is high at every β where it can be read**: k1 and k8 (pulls 6.6 and 7.4),
   k10 (6.9), k5 (2.1). At these volumes the flux loop is heavy (am ≈ 2.0 at L = 14, β = 2.2986), its correlator
   dies within a few slices, and the plateau rule lands on t = 1–2 or 2–3, where excited states have not yet decayed.
   The Amendment-2 GEVP shows the same excess at k10 (0.1844), so this predates Q3-C.
2. **Where Q3-C moved a 0⁺⁺ mass, the effective-mass curves did not move much; the window did.** At k4 the
   effective masses under both estimators agree within errors at t = 2, 3, 4 (0.908/0.893, 0.909/0.868,
   0.871/0.867). Amendment 2's plateau rule chose t = 2–4, Q3-C's chose t = 2–3. At k5 the curves agree to about
   0.015 at every t, and the windows are 3–4 against 2–3. The shift in am₀₊₊ comes from the plateau rule's window
   choice responding to small changes, at a t where the 0⁺⁺ effective mass is still falling.

What these suggest: with the GEVP at (1, 2) and signals that last only to t ≈ 3–5, the plateau windows are short and
early, and both the string tension at larger volumes and the 0⁺⁺ mass at finer spacing are read before the ground
state dominates. My first guess, that truncation removes excited-state directions, is not what the effective-mass
curves show.

This is a diagnosis on data already scored. Any rule built from it (a later t₀, a minimum window start, a longer
minimum window) would be Amendment 4 and could be judged only on fresh seeds.

## Q4 on the new receipt

`research/q3-scaling-20261119/q4ext_receipt.json`. One point passes Q3-a and Q3-v (β = 2.2986), so C1 has no pair to
test: **vacuous, not survived**. C2 holds at that point (3.31(16) ≥ 3.158 − 2σ). K holds nowhere.

## Step 2 on k9 and k10

See `ingest_receipt_k0_k10.json`. The ingest gates compare the engine with the probe; they are independent of the
physics verdict above.
