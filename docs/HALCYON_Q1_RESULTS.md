# Q1 results: CORRELATOR / EFFECTIVE_MASS against Halcyon's M1

2026-11-19. Spec: `docs/HALCYON_QUANTUM_GAP_SPEC.md` §Q1. Code: `src/gauge/correlator.rs`
(probe `examples/correlator_probe.rs`). Harness: `e2e/probes/test_q1_correlator_parity.py`.
Receipts: `research/q1-halcyon-m1-20261119/`.

These are engine runs: the probe was built from this tree. They do not go through a query route yet;
the GQL and HTTP surfaces come with the verbs' registration.

## The input

Halcyon stored only the fitted receipt of its M1 runs, not the per-configuration plaquette series.
So the L = 16, β = 2.3 ensemble was regenerated with Halcyon's own CPU heat bath at the parameters
of its receipt: 200 thermalisation sweeps, 100 measurements one every 2 sweeps, seed 20260628
(`scripts/halcyon_m1_regenerate.py`, 1,598 s). ⟨P⟩ = 0.6026. The original run may have used the
GPU path, so the series is not expected to be bit-identical. The files are named `..._beta2.*`:
pathlib's `with_suffix` cut the stem at "2.3". The receipt inside records β = 2.3.

Only L = 16 was regenerated. L = 6, 8 and 12 are not yet.

## Gates

| gate | result |
|---|---|
| P1 parity, signal: C(t), m_eff(t) in units of their own error; C's error and Halcyon-normalised m_eff error relative | **pass**: 3.4e-10 σ, 3.9e-10, 2.2e-10 σ, 7.1e-10 |
| P1 parity, null | **pass** |
| N, the shuffled null refuses | **pass**: "no time slice has an effective mass that is defined in every jackknife sample" |
| R, Halcyon's auto-window reports a mass on the same null | **fail**: on this ensemble, with this null seed, Halcyon's fit also returns nothing (no finite m_eff) |

R is a demonstration that depends on the data, and on this ensemble it does not show. It was shown
on the synthetic dry run (Halcyon 4.44 against an engine refusal) and in Halcyon's original receipt
(1.118). I did not re-roll the null seed to make it pass.

## What the regenerated ensemble shows

| t | C(t) | m_eff(t) | standard jackknife error | Halcyon's error |
|---|---|---|---|---|
| 0 | 1.30e-5 ± 0.05e-5 | | | |
| 1 | 1.24e-6 ± 0.37e-6 | 2.366 | 0.291 | 0.0291 |
| 2 | 2.9e-7 ± 3.3e-7 | 1.652 | 1.165 | 0.1165 |
| 3 | 3.3e-7 ± 3.2e-7 | 0.632 | 1.653 | 0.1653 |
| ≥ 4 | consistent with 0 | undefined | | |

1. **Halcyon's effective-mass errors are √n too small.** The ratio of the standard jackknife to
   Halcyon's normalisation is 10.0 at every t, and n = 100. `jackknife_effective_mass` divides
   the jackknife sum by the number of finite samples as well, which the standard jackknife does not
   do. The receipt's ±0.0149 at L = 16 is, by this, about ±0.15. The per-L errors and the
   m_∞ = 1.900 ± 0.018 extrapolation inherit the same factor if they came from this function. That
   last point is not yet checked.
2. **The signal ends at t = 1.** C(1) is 3.4σ from zero; C(2) and C(3) are within 1σ of zero. So
   the "plateau" at L = 16 is m_eff(1), carried by its weight, with two noise points beside it.
3. **The engine's plateau rule accepts that window**: t = 1..3, m = 2.28 ± 0.28, χ²/dof 0.69.
   Halcyon's fit on the same data is 2.276 ± 0.028. The rule passes it because points with errors of
   1.2 and 1.7 agree with anything within 1σ. That is a weakness of the rule as fixed: it has no
   requirement that the correlator be significant over the window. See the proposed amendment
   below; it is not applied here.
4. **Against the original receipt**: 1.949 ± 0.0149 there, 2.28 ± 0.28 here. With honest errors
   the two agree (1.2σ). With Halcyon's errors they would be 10σ apart.

## Proposed Amendment Q1-1 (not applied; for Bee)

Add to the plateau rule: every t in the window must have C(t − 1), C(t), C(t + 1) each at least
3σ from zero (the three values m_eff(t) is built from). Under this rule the L = 16 ensemble
refuses, naming t = 2. That is, I think, the honest answer for 100 configurations at this β. The
synthetic tests and the Q2 run, which is already running under the rule as fixed, would be
re-reported with the amendment as a second column, never in place of the pre-registered one.

## What Q1 does and does not establish

- The engine computes Halcyon's M1 quantities to 1e-9 of their error bars, and refuses on a null
  instead of fitting it. That is the move of the measurement into the engine that the spec asked for.
- Q1-a as written ("reproduces 2.24 / 2.05 / 1.98 / 1.95 within their stated errors") cannot pass
  as written, because the stated errors are 10× too small. At L = 16, within honest errors, the
  regenerated value is consistent with the receipt. L = 6 / 8 / 12 still need regenerating.
- Nothing here bears on the continuum gap. At 100 configurations the plaquette channel at
  β = 2.3, L = 16 carries one significant time step.

## Fresh-seed run under the amended rule (Amendment 1)

Halcyon's L = 16, β = 2.3 ensemble regenerated with seed 20261119, null seed 20261120, as fixed in
`docs/HALCYON_Q2_RUN_SPEC.md` Amendment 1. Receipts: `research/q1-halcyon-m1-fresh-20261119/`.

| gate | result |
|---|---|
| P1 parity, signal and null | **pass** (C to 4.6e-10 σ, m_eff to 9.5e-11 σ) |
| N, the null refuses | **pass** |
| R, Halcyon's fit reports on the null | **fail** again: Halcyon's fit also returns nothing on this null |
| **Prediction fixed before the run: the amended rule refuses** | **confirmed**: "the correlator is less than 3 standard errors from zero wherever the effective mass is defined (at t = 1)" |

m_eff(1) = 1.85 ± 0.16 and m_eff(2) = 0.78 ± 0.68. C(2) is under 3σ, so the floor admits no window.
The rule as first fixed also refuses here, at t = 2.

**Three runs of Halcyon's fit on the same physics:**

| run | Halcyon's fit | Halcyon's error |
|---|---|---|
| original receipt | 1.949 | 0.015 |
| regeneration, seed 20260628 | 2.276 | 0.028 |
| regeneration, seed 20261119 | 1.791 | 0.016 |

The three values spread by 0.49 while each claims an error near 0.02, about 20 of its own error bars.
With the standard jackknife (about ±0.16 to ±0.28, √n = 10 times larger) they agree. This is direct
evidence, independent of the code reading, that the M1 error bars are understated by about √n.

(The refusal message was corrected before this record: it had named "undefined in a jackknife
sample" when the failing condition was the 3σ correlator floor. The verdict was the same.)
