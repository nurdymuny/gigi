# S3b: the Cayley time step (results, n = 1 and n = 2)

2026-11-18. Spec: `docs/RICCI_SU2_PG_FLOW_CAYLEY_SPEC.md` (with Amendment 1: Newton tolerance
10⁻⁸). Script: `scripts/ricci_su2_pg_flow_cayley.py`. Receipts:
`research/su2-cayley-20261118/L0_*.json` (with each run's final state, `*.final.npy`); gate
verdicts: `research/su2-cayley-20261118/gates_L0.json`, from `gates.py` in the same folder.
Run after the exact defect derivative (`docs/RICCI_SU2_EXACT_DEFECT_DERIVATIVE_RESULTS.md`,
merge `be318ca`). Research script, **not an engine run**. n = 3 is not yet run.

## In plain English

The Cayley step now works at n = 1. Every run converged at every step, with at most eight
Newton iterations, and its answers agree with S3's small-step reference (RKMK4) to within
the spec's 5%: from N = 8 steps for both forces, and from N = 4 for the two-ring force. The
gap to the reference shrinks four-fold each time the step is halved, as a second-order
method should. At N = 8 the Cayley runs used 40 (native) and 46 (two-ring) force evaluations,
against 128 for the RKMK4 reference at N = 32.

## Controls

| control | threshold | measured |
| --- | --- | --- |
| C-cay: \|cay(v)\| = 1 | 10⁻¹⁵ | 3.3×10⁻¹⁶ |
| C-cay: \|cay(v) − qexp(v)\| / \|v\|³ bounded as \|v\| → 0 | bounded | 0.0832, 0.0833, 0.08333, 0.08333 (→ 1/12) |
| C-order: observed order, N = 4, 8, 16 | 1.8 to 2.2 | 1.993, 1.998 |
| C-newton: every step converges within 40 iterations | all steps | all 8 runs, every step; at most 8 iterations |

The limit 1/12 is the Cayley map's own third-order gap from the exponential: the s³/12 phase
lag of *Three Ways Across a Turn*, Section 6.

## Gates at n = 1

G-agree: |E_cay(t) − E_rk4(t)| < E_rk4(t)/20 at t = 0.005, 0.01, 0.015, 0.02, against S3's
`L0_native_N32_q6.json` and `L0_pg_N32_q6.json`. G-time: halving h changes E_cay(0.02) by
less than E/10.

| force | N | status | Newton max | force evals | E(0.02) | G-agree worst gap | G-time change |
| --- | --- | --- | --- | --- | --- | --- | --- |
| native | 4 | ok | 7 | 25 | 0.035283 | 8.24% (fail) | 0.75% |
| native | **8** | ok | 5 | 40 | 0.035019 | **1.43%** | **0.19%** |
| native | 16 | ok | 5 | 73 | 0.034951 | 0.36% | 0.05% |
| native | 32 | ok | 4 | 128 | 0.034935 | 0.09% | (no N = 64) |
| two-ring | 4 | ok | 8 | 28 | 0.069255 | 4.29% | 0.43% |
| two-ring | **8** | ok | 6 | 46 | 0.068957 | **0.75%** | **0.11%** |
| two-ring | 16 | ok | 5 | 80 | 0.068882 | 0.19% | 0.03% |
| two-ring | 32 | ok | 5 | 130 | 0.068864 | 0.05% | (no N = 64) |

RKMK4 reference: E(0.02) = 0.034929 (native), 0.068857 (two-ring).

**At n = 1, N = 8 is the smallest N passing both gates for both forces** (N = 8 and 16 pass;
native N = 4 fails G-agree). The spec fixes N as the smallest value passing at both n = 1 and
n = 2, so N is not chosen until n = 2 is run; n = 1 requires N ≥ 8.

The G-agree gap falls 8.24 → 1.43 → 0.36 → 0.09% (native) and 4.29 → 0.75 → 0.19 → 0.05%
(two-ring): a factor of 4.0 per halving after the first, second order.

## Notes

- The final Newton increments were 1.9×10⁻⁹ to 9.5×10⁻⁹, all below Amendment 1's 10⁻⁸. With
  the exact derivative the force is clean to about 10⁻¹³, so the tolerance, not the force, now
  sets that number.
- Timings (this machine, eight runs in parallel, two threads each): 1.4–1.6 s per native and
  4.7–5.5 s per two-ring force evaluation. The RKMK4 references ran on another machine, so
  force-evaluation counts, not seconds, are the comparison.
- The iteration matrices K₀ were rebuilt on this commit (`buildK`, 6.8 s native, 91 s
  two-ring, 14 workers) into `outputs/su2-cayley/` (not committed).
- At n = 1 the two-ring trajectory error (R = 0.138) is about twice the native one (0.070),
  as in S3; S3 found the two-ring force overtaking the native one at n = 2.

## Gates at n = 2

Against S3's `L1_native_N64_q6.json` and `L1_pg_N64_q6.json` (RKMK4, N = 64). Verdicts:
`research/su2-cayley-20261118/gates_L1.json`. Iteration matrices rebuilt on this commit
(48.9 s native, 830.5 s two-ring, 14 workers; 3,372 unknowns).

| force | N | status | Newton max | force evals | E(0.02) | G-agree worst gap | G-time change |
| --- | --- | --- | --- | --- | --- | --- | --- |
| native | 4 | ok | 8 | 27 | 0.011452 | 22.41% (fail) | 1.13% |
| native | **8** | ok | 7 | 49 | 0.011323 | **0.47%** | **0.34%** |
| native | 16 | ok | 5 | 79 | 0.011285 | 0.33% | 0.10% |
| native | 32 | ok | 5 | 134 | 0.011274 | 0.12% | (no N = 64) |
| two-ring | 4 | ok | 8 | 26 | 0.009192 | 13.12% (fail) | 1.61% |
| two-ring | **8** | ok | 6 | 44 | 0.009044 | **1.15%** | **0.40%** |
| two-ring | 16 | ok | 5 | 75 | 0.009007 | 0.36% | 0.10% |
| two-ring | 32 | ok | 4 | 128 | 0.008998 | 0.09% | (no N = 64) |

RKMK4 reference: E(0.02) = 0.011270 (native), 0.008995 (two-ring).

**The step count is fixed: N = 8.** It is the smallest N passing both gates for both forces at
n = 1 and at n = 2 (N = 8 and 16 pass at both levels). By the spec, n = 3 runs with N = 8, and
G-time at n = 3 checks it there with N = 16.

At N = 8 the Cayley runs used 49 (native) and 44 (two-ring) force evaluations, against 256 for
the RKMK4 references at N = 64.

The two-ring force overtakes the native one at n = 2, as in S3: E(0.02) = 0.00904 against
0.01132 (R = 0.0177 against 0.0222).

### Where N = 4 fails

N = 4 fails G-agree only at the first sample time, t = 0.005, which is a single step of size
h = 0.005. The gaps by sample time are:

| force, N | t = 0.005 | 0.01 | 0.015 | 0.02 |
| --- | --- | --- | --- | --- |
| native, 4 | 22.41% | 0.44% | 4.09% | 1.62% |
| native, 8 | 0.02% | 0.35% | 0.43% | 0.47% |
| two-ring, 4 | 13.12% | 2.20% | 2.75% | 2.18% |
| two-ring, 8 | 1.15% | 0.68% | 0.60% | 0.54% |

*Hypothesis, consistent with these numbers and not tested:* the Crank–Nicolson factor
(1 − hλ/2)/(1 + hλ/2) tends to −1, not 0, as hλ grows. Stiff directions are therefore not damped
by a large Cayley step; they change sign each step. S3a found the native force's error lives in
stiff directions, and the native N = 4 gap alternates in size (22.4, 0.4, 4.1, 1.6%) instead of
settling. At N = 8 the effect is below every threshold. If a larger step is ever wanted, an
L-stable variant (for example two backward-Euler-type half steps, or a TR-BDF2 split) is the
standard remedy; that would be a new spec.

## Not run

n = 3 (with N = 8, and N = 16 for G-time), the frozen-metric control, the native-control checks
at n = 3.
