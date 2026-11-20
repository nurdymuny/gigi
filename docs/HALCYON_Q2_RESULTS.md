# Q2 results: EFFECTIVE_MASS against the exact 2D SU(2) answer

2026-11-19. Run spec: `docs/HALCYON_Q2_RUN_SPEC.md` (written and pushed before code or data,
ab7c0951; code 99d13059 before data). Harness: `e2e/probes/test_q2_exact_2d.py`. Receipt:
`research/q2-exact-2d-20261119/q2_receipt.json`, with the probe outputs. The eight ensembles
(24 MB each) are not committed. The seeds are in the receipt and the sampler reproduces them bit for bit.

These are engine runs: the sampler and the estimator were built from this tree and called as
probes. They do not go through a query route yet. The exact answer comes from an independent
Python script.

## The run

2D SU(2), N_s = 2, N_t = 32, periodic, β = 4/(g²a²). Metropolis, 2,000 thermalisation sweeps, then
20,000 measurements one every 5 sweeps. Acceptance 0.54–0.61. Blocked jackknife, blocks of 100. The
observable is the spatial Polyakov loop. Its exact mass is −N_s ln λ_½: 3N_s/(2β) for the heat
kernel, and −N_s ln(I₂/I₁) for Wilson.

## Results

| action | β | window | plateau | exact | pull | max \|C − C_exact\|/σ |
|---|---|---|---|---|---|---|
| heat kernel | 3 | 3–5 | 0.9857 ± 0.0240 | 1.0000 | −0.59 | 1.29 |
| heat kernel | 4 | 1–7 | 0.7483 ± 0.0024 | 0.7500 | −0.73 | 1.99 |
| heat kernel | 6 | 3–8 | 0.5022 ± 0.0052 | 0.5000 | +0.43 | 1.66 |
| heat kernel | 8 | 5–10 | 0.3804 ± 0.0070 | 0.3750 | +0.77 | 3.09 |
| Wilson | 3 | 1–7 | 1.1356 ± 0.0034 | 1.1315 | +1.18 | 2.48 |
| Wilson | 4 | 5–9 | 0.7067 ± 0.0693 | 0.8370 | −1.88 | 1.47 |
| Wilson | 6 | 1–3 | 0.5414 ± 0.0017 | 0.5417 | −0.20 | 3.11 |
| Wilson | 8 | 1–3 | 0.3978 ± 0.0016 | 0.3987 | −0.61 | 1.85 |

## Gates

| gate | result |
|---|---|
| Q2-a: heat-kernel plateau within 2σ of exact, every β | **pass** (4/4) |
| Q2-a′: whole correlator within 4σ, t ≤ 16, heat kernel | **pass** (4/4) |
| Q2-w: the same two on Wilson against Wilson's exact answer | **pass** (4/4, 4/4) |
| Q2-r: Wilson plateau > 3σ from the heat-kernel exact m | **pass at β = 3, 6, 8** (39σ, 24σ, 14σ); **fail at β = 4** (0.6σ) |
| Q2-n: refusal on the plaquette channel and on the shuffled Polyakov series | **pass** (8/8 ensembles, both nulls) |

Per the run spec, at β = 4 the run cannot tell the two actions apart, so β = 4 does not count for
Q2-a. **Q2-a stands at β = 3, 6 and 8**: there, the engine recovers the exact gap of the heat-kernel
theory within errors, and separates it from the Wilson theory's exact gap by 14–39σ.

## Why β = 4 Wilson failed: the plateau rule

| t | m_eff | error | C(t+1)/σ |
|---|---|---|---|
| 1 | 0.8378 | 0.0030 | 105 |
| 2 | 0.8296 | 0.0071 | 48 |
| 3 | 0.8235 | 0.0153 | 21 |
| 4 | 0.8722 | 0.0370 | 11 |
| 5 | 0.6898 | 0.0810 | 5.0 |
| 6–9 | 0.70–0.91 | 0.15–0.91 | 0.8–2.2 |

The values at t = 1–4 sit on the exact 0.8370. But the step 1→2 differs by 1.06 combined σ and
3→4 by 1.2σ. Those are ordinary fluctuations: a 1σ threshold is crossed about a third of the time per
pair. Each one breaks the run under "consecutive values agree within 1σ". The earliest run of three
is then t = 5–9, where the correlator is 1–2σ from zero, and noise points agree with anything. This is
the same weakness Q1 found from the other side (two noise points accepted beside m_eff(1)). The rule
is too strict where the data are precise, and has no floor where they are not.

## Proposed amendments (not applied; to be tested only on new ensembles)

- **Q1-1** (already proposed in the Q1 results): every t in a window needs C(t − 1), C(t), C(t + 1)
  each at least 3σ from zero.
- **Q1-2**: consecutive effective masses agree within 2 combined σ, not 1.

These were suggested by this data, so under the spec's own discipline they can only be judged on
fresh ensembles: new seeds at the same β grid for Q2, and the L = 16 Halcyon ensemble with a new
seed for Q1. They would be reported beside the rule as fixed, not in its place. Bee decides whether
to adopt them.

## What Q2 does and does not establish

- The engine's sampler plus estimator recover an exact quantum gap where one is known, for two
  different actions, and the nulls refuse. This is the first quantum oracle the engine is checked
  against.
- In two dimensions there are no propagating gluons. Q2 checks the estimator, the sampler and the
  plateau rule against an exact answer. It says nothing about four dimensions.
- The convention correction to Halcyon's stored 2D column (3/(2β), not 3β/8) is reported in the
  run spec and is Bee's to apply to Halcyon's record.
