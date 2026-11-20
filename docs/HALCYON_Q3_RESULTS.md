# Q3 results: m₀₊₊/√σ in 4D SU(2), measured in the engine, against LTW

2026-11-20. Spec: `docs/HALCYON_Q3_RUN_SPEC.md` (Amendments 1 and 2). Harness
`e2e/probes/test_q3_scaling.py`, probe `correlator_probe` built from 4ef748d4 (sha256 prefix
733b08b6980fce43), sampler as in the run log. Receipt: `research/q3-scaling-20261119/q3_receipt.json`.
Every point: 10,000 measurements after 1,000 thermalisation updates, blocks of 100. These are probe
runs on the engine's kernels (the same code the EFFECTIVE_MASS verb calls), not requests to a live
gigi-stream.

**Order of events that bears on the reading.** k0 and k2 were peeked at before the harness ran;
Amendment 2 (Q1-4 two-slice window, Q3-B operator selection) was adopted after the peek. The amendment
is judged only on k1, k3, k4, k5, k6, which nobody had read.

## Summary

- **Calibration passes at all three LTW points (Q3-a).** a√σ and am₀₊₊ agree with LTW Table 5 within
  1.9σ everywhere.
- **The volume check passes at β = 2.3715 and 2.4265 and fails at β = 2.2986 (Q3-v).** At L = 14 both
  channels refused: the 0⁺⁺ GEVP returned a noise-level mode (Q3-B fired), and the flux loop is too
  heavy (am ≈ 1.8 expected) to stay above 3σ past t = 1 at this statistics.
- **Scaling passes, weakly (Q3-s).** Two β values qualify, so the fit c + d·a²σ goes exactly through
  two points (no degrees of freedom): c = 3.88(46) against 3.78(7), pull 0.2. The error is 6.5 times
  LTW's. The test is real but has little power.
- **Mechanism removal passes (Q3-r).** At β = 2.3, L = 16, the raw unsmeared plaquette refuses; the
  smeared GEVP gives am₀₊₊ = 1.190(39). Halcyon's M1 read 1.95 at the same β and L, near LTW's excited
  0⁺⁺⋆ = 2.00(11). That reading was excited-state contamination.
- **Nulls refuse everywhere a series existed (Q3-n).** At k1 there was no 0⁺⁺ projected series (the
  GEVP refused), so that channel's null is vacuous; the harness counts it as a pass.
- **β = 2.5115 was not run.** The pre-registered condition was that the first three β pass Q3-a and
  Q3-v; β = 2.2986 failed Q3-v.

## Calibration (Q3-a) and volume (Q3-v)

| β | L | a√σ (engine) | a√σ (LTW) | pull | am₀₊₊ (engine) | am₀₊₊ (LTW) | pull | m₀₊₊/√σ |
|---|---|---|---|---|---|---|---|---|
| 2.2986 | 10 | 0.3603(76) | 0.3667(18) | −0.8 | 1.184(59) | 1.212(22) | −0.5 | 3.29(18) |
| 2.2986 | 14 | refused | | | refused | | | |
| 2.3715 | 12 | 0.2912(29) | 0.2879(13) | +1.0 | 1.037(35) | 0.994(18) | +1.1 | 3.56(13) |
| 2.3715 | 16 | 0.2942(48) | | | 0.973(36) | | | 3.31(13) |
| 2.4265 | 16 | 0.2424(25) | 0.2388(9) | +1.4 | 0.888(25) | 0.834(13) | +1.9 | 3.66(11) |
| 2.4265 | 20 | 0.2449(28) | | | 0.861(49) | | | 3.52(20) |

Volume pulls: β = 2.3715, am₀₊₊ +1.3, a√σ −0.5; β = 2.4265, am₀₊₊ +0.5, a√σ −0.6. aL√σ ≥ 3.49
wherever σ was read. The ratio's error is one joint blocked jackknife over both channels.

LTW's own lattice ratios at these β are 3.305, 3.453, 3.492; the engine's are 3.29(18), 3.56(13),
3.66(11). All three β pull the same direction on am₀₊₊ (high by 1–2σ at the two finer points). This is
not significant on its own.

## Scaling (Q3-s)

Points: (a²σ, ratio) = (0.0848, 3.56(13)), (0.0588, 3.66(11)). Fit c = 3.88(46), d = −3.8, χ² = 0
with no free degree. The falsifier (monotone drift with χ²/dof > 2) cannot trigger with two points.
Pass, recorded as a two-point extrapolation.

## Mechanism removal (Q3-r), the direct answer to Halcyon M1

β = 2.3, L = 16 (k6):
- raw level-0 plaquette: refuses (correlator under 3σ at t = 2);
- smeared GEVP (levels 0, 4, 8, 16 all kept): am₀₊₊ = 1.190(39), window (2, 3).

The flux loop at k6 refused (am ≈ 2.1 expected at L = 16), so there is no ratio at β = 2.3; Q3-r does
not need one.

## Amendment 2 on the points it is judged on

**Q1-4 (two-slice window).** On the unseen points it was used where the three-slice amended rule found
no window. Where the as-fixed rule also returned a value, the two agree to a small fraction of σ:

| point | channel | amended (window) | as fixed (window) |
|---|---|---|---|
| k3 | 0⁺⁺ | 0.973(36) (2, 3) | 0.975(37) (2, 4) |
| k3 | flux | 1.319(45) (2, 3) | refused (1σ agreement at t = 2) |
| k4 | 0⁺⁺ | 0.888(25) (2, 4) | 0.888(25) (2, 4) |
| k4 | flux | 0.875(19) (2, 4) | 0.875(17) (2, 4) |
| k5 | 0⁺⁺ | 0.861(49) (3, 4) | 0.866(49) (3, 6) |
| k5 | flux | 1.147(27) (2, 3) | 1.149(27) (2, 4) |
| k6 | 0⁺⁺ | 1.190(39) (2, 3) | 1.188(40) (2, 4) |

Every value Q1-4 produced on unseen data entered a gate and passed (Q3-v at both finer β).

**Q3-B (operator selection).** Smearing level 0 was dropped from the flux loop at every point and
from 0⁺⁺ at k3, k4, k5. The noise-mode refusal fired once on unseen data: k1 0⁺⁺, projected C(1) at
2.76σ. Diagnosis after scoring (not a gate): at k1 each individual smearing level has a strong 0⁺⁺
signal (t = 1 signal-to-noise 40 to 65), and levels 4 and 8 alone give am₀₊₊ = 1.151 and 1.150. The
GEVP's largest eigenvalue went to a direction that C(1) cannot see: the operators are close to collinear,
and noise in a near-null direction of C(t₀) inflates λ. Q3-B refused it correctly rather than reporting
noise. The cost is that a point with good single-operator signal was lost. Any fix (truncating the
near-null directions of C(t₀) before the solve, the usual remedy) would be a new amendment, judged on
fresh seeds, not on k1.

**The flux loop at k1** is a separate matter: single levels give am = 2.14 and 2.00 from t = 1 to 2,
against about 1.8 expected from LTW. The signal at t = 2 is 5 to 7σ and nothing survives past it. At
L = 14 and β = 2.2986 the flux loop needs more statistics than 10,000 measurements, or a smaller second
volume.

## Q4 (exploratory, separate)

`e2e/probes/test_q4_candidates.py` on this receipt, per `docs/HALCYON_Q4_RUN_SPEC.md` (committed
before any Q3 correlator). Receipt: `research/q4-candidates-20261120/q4_receipt.json`. Two points
qualify (β = 2.3715, 2.4265).
- C1 (monotone approach from below): survives, no violation; two points.
- C2 (floor 3.158, uniform in spacing): survives at both points.
- K (strong-coupling control, must fail): holds nowhere, as required.

Two points cannot discriminate anything. A survivor here is not a conjecture yet, only a candidate not
contradicted.

## What this establishes and what it does not

- The engine's 4D sampler, smearing, GEVP and EFFECTIVE_MASS reproduce LTW's published lattice
  numbers at three spacings, and the volume check holds at two of them.
- The ratio at the two finer spacings extrapolates to 3.88(46), consistent with LTW's continuum
  3.78(7) but with no independent precision.
- Halcyon's M1 value at β = 2.3 was excited-state contamination of the raw plaquette.
- Not established: a continuum value of the engine's own; anything at β = 2.5115; the β = 2.2986
  second volume.

## What would strengthen it, for Bee to choose

1. A second volume at β = 2.2986 that the flux loop can carry (L = 12), or more statistics at L = 14.
2. β = 2.5115 (L = 20, 24) on the parallel sampler. The pre-registered rule says no as things stand; it
   would need a new decision written down before data.
3. A GEVP amendment (truncate near-null directions of C(t₀)) judged on fresh seeds.
