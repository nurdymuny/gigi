# S3c: the SU(2) flow on a five-times-longer window (spec)

2026-11-19. Written after S3b's n = 3 results and **before any measurement in this file**.
Research script `scripts/ricci_su2_pg_flow_cayley.py` (new `--T` option), **not an engine
run**. It follows S3 (`docs/RICCI_SU2_PG_FLOW_RESULTS.md`), S3a
(`docs/RICCI_SU2_PG_FLOW_FILTER_RESULTS.md`) and S3b
(`docs/RICCI_SU2_PG_FLOW_CAYLEY_RESULTS.md`).

## In plain English

At n = 3, S3b found the native force's trajectory error still converging (order 1.80), so the
amended native control ("native must stall") failed again, and S3a's forecast that the
two-ring error would fall faster missed. S3a had already explained why native does well on
the short window: 99% of its 25% instantaneous error lies in stiff directions, which the
dynamics damps within a fraction of the window, and only the slow part reaches the
trajectory. S3a also said where that explanation stops: on a longer window more directions
count as slow (the threshold is 1/T), and native's stiff error begins to matter.

This experiment tests that statement directly. The window grows from [0, 0.02] to [0, 0.1].
The Berger metric then moves from (3, 1, 1) to (1.41, 0.98, 0.98), still smooth (the flow
collapses near t = 0.378). The Cayley step makes the longer window affordable.

## What is run

- Levels n = 1 and n = 2; native and two-ring forces; moving metric; quadrature q6.
- Cayley midpoint step with S3b's step size, h = 0.0025 (N = 40 over [0, 0.1]), and h/2
  (N = 80) for G-time.
- Errors E(t), D(t), R(t) = E/D at t = 0.02, 0.04, 0.06, 0.08, 0.1, as in S3.
- Newton: S3b's rule (tolerance 10⁻⁸, 40 iterations, stop on failure), with one addition fixed
  now: the iteration matrix A = B + (h/2) K₀ keeps K₀ from t = 0, and its mass B is
  reassembled at the current time whenever a step needs more than 12 iterations. Every
  refresh is counted and reported. With the default T = 0.02 nothing changes, so S3b's runs
  are reproduced exactly.

## Controls (thresholds fixed now)

- **C-prefix.** At t = 0.02 the N = 40 runs take exactly S3b's N = 8 steps (same h). Their
  E(0.02) must match S3b's `L{0,1}_{native,pg}_N8.json` to 10⁻⁶ relative.
- **G-time.** At every level and both forces, halving h changes E at every sample time by less
  than E/10.

## The prediction tested (fixed now, from S3a)

Write p(T) for the observed order of E(T) between n = 1 and n = 2 (log2 of the ratio).
At T = 0.02 these are known: p_native = 1.63, p_two-ring = 2.94 (S3b's N = 8 values give
log2(0.035019/0.011323) = 1.63 and log2(0.068957/0.009044) = 2.93).

- **P-long (native).** Native's stiff error starts to count: p_native(0.1) < p_native(0.02) − 0.25.
- **P-long (two-ring).** Two rings do not lose order: p_two-ring(0.1) ≥ p_two-ring(0.02) − 0.25.
- **P-cross.** At n = 2 the two-ring error stays below native at every sample time.

Each is reported PASS or FAIL. If P-long (native) fails, native's stiff error does not reach
the trajectory even over [0, 0.1], and S3a's caveat is not borne out on this window; that is
reported as the finding. n = 1 to n = 2 is pre-asymptotic (S2a), so a pass is evidence, not a
rate; n = 3 on the long window is the follow-up if the n = 1 to 2 result is informative.

## Not measured

n = 3 on the long window (cost: about 5 times S3b's n = 3 runs), windows beyond 0.1, other
metrics, the engine path.
