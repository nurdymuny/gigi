# The per-point domain gate: built, verified on a control, with one exposure disclosed

25 November 2026. Implements §3.3 of the [flux-grid method amendment](helike-flux-method-amendment-2026-11-25.md).
**No registered grid point has been evaluated.** One accidental exposure during testing is disclosed below.

## What the stage does

For a grid point's final rung, every step is either exact or a rigorous enclosure.
1. **Rust `cone-export`** solves with exact moments and exports the four trial states as IEEE bit patterns,
   with the chart (u0, S), frequencies and basis.
   - **Selection** is the rule of `run_helike_quasimode.select`: the ground state; the Helike pair, chosen by weight
     on her harmonic doublet, each ≥ 0.95, one of them the identified Helike level; and the competitor (≥ 0.95).
     The pair's centres are averaged.
   - **The chiral mixing** comes from the exact 2×2 charge compression on the pair, obtained by polarization of
     linear charge moments, instead of the quadrature grid of `chiral_frame`. The mixing is only a choice of trial
     vector; the rigorous charge bound certifies whatever it is.
2. **State file.** The export is written in the `helike-lab/quasimode-states/v1` schema. The Python potential and
   metric expansions are added for the interval code's coefficient checks. Floating reference moments are absent,
   and are now optional in the Rust schema; when present, as for the controls, they serialize exactly as before.
3. **Arb.** `helike_cutoff_bounds.bound_state` (128-bit, registered cutoff and plateau rule) runs on the binary64
   product C = coeff @ mixing, giving t and ℓ.
4. **Rust `cone-sectors`.** It runs on the same frozen product, giving r_lower, r_upper, c, q and R.
5. **The registered relative gate.** It is evaluated on helike_plus, helike_minus and the competitor. PASS only if
   all three pass. Any refusal inside the stage is OPEN, never PASS.

**The authority for the chart is the export itself.** All three tools evaluate the same (u0, S), so the Rust
solver, the Arb bounds and the interval residual describe one function.

## Verification on a control (not a grid point)

At (s, t, g², Mq) = (1.15, 3, 0.004, 6) the stage returns PASS in about 12 seconds. It reproduces the archived
Python-construction sector bounds from an independent solver:

| state | r_lower | ℓ ≤ | c ≤ | R ≤ |
| --- | --- | --- | --- | --- |
| ground | 0.0151472 | 2.922e-9 | 1.591e-3 | 0.0151473 |
| Helike ± | 0.0487248 | 4.869e-9 | 8.233e-3 | 0.0487265 |
| competitor | 0.0801233 | 2.853e-9 | 9.699e-3 | 0.0801271 |

The compressed charges are ∓2.000006. A test asserts agreement with the archived receipt to 10⁻⁶ relative for r and
c, and 10⁻³ for ℓ.

**Runner integration.** `run_helike_flux_grid.py` calls the stage at the final rung (Mq = 10).
- A solver refusal at any rung is recorded as a failed point (UNRESOLVED under the registered rule), never a crash.
- `--check-release` now reports no unmet conditions.
- The grid still starts only with an explicit `--registered-grid`, intended for the big machine.

**Tests and mutants.**
- Rust: the export test (selection, unitary mixing, charges ∓2).
- Python: 17 tests, including the end-to-end control parity.
- 9 of 9 mutants are caught by assertion:
  - relaxing 0.003, 0.9 or 0.015;
  - dropping the domain gate;
  - letting one good point stand for the case;
  - forcing the gate to PASS;
  - turning a refusal into PASS;
  - removing the explicit grid flag;
  - not gating the competitor.

  A safety net now replaces the real solver during mutation runs, so a mutant cannot compute anything.

## Exposure, disclosed

While testing the explicit-flag guard, a mutated `main` twice reached the real solver at the first registered grid
point, (s, t, g², Mq) = (1.12, 3, 0.03, 6).
- **Cause.** The mutant ran against a snapshot of the module's globals, so the test's stubs did not apply. This is
  fixed: mutants run in the live namespace, and the solver is replaced by a failing stub during mutation runs.
- **What happened.** Both attempts exited with an error (exit 2) after about 4 seconds. No gap, weight or level was
  printed, saved or inspected, and the refusal's reason was not examined.
- **What is known.** Only that the Rust solve at that point refused at Mq = 6. In the registered run, a refusal is a
  failed point and its (t, g²) is UNRESOLVED. It is recorded here so that this outcome is not read later as a
  surprise.

No other grid point was touched. The controls, s = 1.15, t = 3, g² ∈ {0.004, 0.002}, are not grid points.

## Environment note

The Arb stage needs `python-flint==0.9.0`. Where this was built, pip's global index points at a private registry
without it. It was installed from public PyPI into a scratch folder only, without changing any configuration.
