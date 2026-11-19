# S3d: does the Cayley step fail at N = 4 because it does not damp stiff modes? (spec)

2026-11-19. Written before any measurement in this file. Research script
`scripts/ricci_su2_pg_flow_cayley.py` (new `--startup be2` option), **not an engine run**.
It tests the hypothesis recorded in S3b's results (`docs/RICCI_SU2_PG_FLOW_CAYLEY_RESULTS.md`,
"Where N = 4 fails").

## In plain English

With four steps over the window, the Cayley (Crank–Nicolson) step missed S3b's reference by
22% (native) and 13% (two-ring) at the first sample time, n = 2, which is one single large step,
and the native gap then alternated (22.4, 0.4, 4.1, 1.6%) instead of settling. The hypothesis:
Crank–Nicolson's factor (1 − hλ/2)/(1 + hλ/2) tends to −1, not 0, for stiff directions, so a large
first step flips the stiff part of the initial error instead of damping it. S3a found the native
force's error lives almost entirely in stiff directions.

The standard remedy (Rannacher's start-up) replaces only the first step by two backward-Euler
half-steps, whose factor tends to 0 for stiff directions, and keeps the Cayley step afterwards.
If the hypothesis is right, that one change should remove most of the N = 4 failure. If it is
wrong, the failure should stay.

## The start-up step

Backward Euler in Lie form, step h/2, twice: U ← U · cay(Ω), Ω solving
Ω = (h/2) ξ(U · cay(Ω), t + h/2), with t advanced by h/2 after each half-step. It is first order
and L-stable. Newton as in S3b, with the same iteration matrix A = B₀ + (h/2)K₀ (which is exactly
the backward-Euler Jacobian for step h/2), tolerance 10⁻⁸, 40 iterations, stop on failure.
Every later step is S3b's Cayley midpoint step, unchanged.

## Runs

n = 1 and n = 2; native and two-ring; N = 4 and N = 8; T = 0.02 (S3b's window), with and without
the start-up. The runs without it are S3b's receipts, reused.

## Predictions (fixed now)

- **P-first.** With the start-up, the G-agree gap at t = 0.005 for N = 4 at n = 2 falls below
  half its S3b value for both forces (native < 11.2%, two-ring < 6.6%).
- **P-settle.** With the start-up, the native N = 4 gaps at n = 2 no longer alternate: the gap
  at t = 0.015 is not larger than the gap at t = 0.01.
- **P-harmless.** At N = 8, where S3b already passed, the start-up changes E(0.02) by less than
  E/20 for both forces at both levels (it costs one order of accuracy in one step only).

If P-first and P-settle hold, the hypothesis is supported. If they fail, it is rejected and the
N = 4 failure has another cause. Either way N = 8 stays the step count for S3b and S3c; this is a
diagnosis, not a change of method.

## Not measured

n = 3, longer windows, other L-stable schemes (TR-BDF2, SDIRK).
