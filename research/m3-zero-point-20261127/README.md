# M3 zero-point computation (27 November 2026)

Bundle-framing math §14. Raw spectra are gzipped receipts of `helike zero-point` (gunzip before re-analysis).

- `zp_t3.0_j200_*.json.gz`: the deep run used for the background result (centre; Helike and competitor modes at
  h = 0.05, 0.1). Analysis: `scripts/helike_m3_deep_analysis.py` -> `background_deep.json`.
- `t1/zp_t1.0_j120_*.json.gz`: the t = 1 Maurer-Cartan line (exact checks: round-sphere law, gauge equivalence,
  reflection, the b0 log coefficient). Read by `scripts/gigi_m3_demo.py`.
- SUPERSEDED, kept on purpose:
  - `background_jmax130_SUPERSEDED.json` and `zp_t3.0_j130_*`: jmax 130 was not converged at the strong background
    field (curvatures moved by up to 4x between jmax and jmax - 20; the heat-trace window leaked the truncated tail).
  - `t1/eps_fit_report_SUPERSEDED.json`: the first regulator analysis (fit of 1/2 sum omega e^{-eps omega} on
    eps in [0.4, 1.2]); the asymptotic series is poor there. Replaced by the heat-trace fit.
