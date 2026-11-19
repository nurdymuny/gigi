# S3e: a second-order L-stable start-up (SDIRK2) for the Cayley step (spec)

2026-11-19. Written before any measurement in this file. Follows S3d
(`docs/RICCI_SU2_PG_FLOW_STARTUP_RESULTS.md`), whose open question it answers. Research script
`scripts/ricci_su2_pg_flow_cayley.py` (new `--startup sdirk2`), **not an engine run**.

## In plain English

S3d's damping start-up (two backward-Euler half-steps) removed the native force's alternation but
made the first step worse, much worse for the two-ring force. Backward Euler is only first order, so
S3d could not tell whether the trouble was undamped stiff modes or the start-up's own error. S3e
uses a start-up that damps stiff modes just as completely but is second order, like the Cayley step
itself. If the N = 4 failure is stiff modes, it should now go away; if it stays, the cause lies
elsewhere.

## The start-up step

Two-stage, stiffly accurate SDIRK (the class TR-BDF2 belongs to) with γ = 1 − 1/√2: second order,
L-stable. In the Lie (Cayley) form used by S3b, with U = U_n · cay(Ω):

- stage 1: Ω₁ = γh · ξ(U_n · cay(Ω₁), t_n + γh)
- stage 2: Ω₂ = ((1 − γ)/γ) · Ω₁ + γh · ξ(U_n · cay(Ω₂), t_n + h)
- U_{n+1} = U_n · cay(Ω₂) (stiffly accurate).

(Stage 2 is the standard SDIRK2 stage, Ω₂ = h(1 − γ)K₁ + γhK₂ with K₁ = Ω₁/(γh).) The dropped
inverse-Cayley-derivative corrections are O(h²) in the stage values, so the step stays second order.
Each stage is solved by S3b's quasi-Newton iteration with the matrix B₀ + γh K₀ (the stage Jacobian),
tolerance 10⁻⁸, 40 iterations, stop on failure. Every later step is S3b's Cayley midpoint step.

## Runs

n = 1 and 2; native and two-ring; N = 4 and 8; T = 0.02, exactly as S3d.

## Predictions (fixed now; P-first, P-settle, P-harmless have S3d's thresholds)

- **P-first.** First gap at n = 2, N = 4 below half of S3b's (native < 11.2%, two-ring < 6.6%).
- **P-settle.** Native n = 2, N = 4: gap at 0.015 ≤ gap at 0.01.
- **P-harmless.** At N = 8, E(0.02) moves by less than E/20 from S3b for both forces at both levels.
- **P-order.** At n = 2, N = 4, the first gap is smaller with the SDIRK2 start-up than with S3d's
  backward-Euler start-up, for both forces.

If P-first and P-order hold, undamped stiff modes are the cause and a second-order damping start-up
is the remedy. If P-order holds but P-first fails, the start-up's order mattered but stiff modes are
not the whole story. If both fail, stiff modes are not the cause.
