# S3c status: complete (8 of 8 runs)

Spec: `docs/RICCI_SU2_PG_FLOW_LONG_WINDOW_SPEC.md` (72216bf). Results: `docs/RICCI_SU2_PG_FLOW_LONG_WINDOW_RESULTS.md`.
Verdicts: `gates.json`, produced by `python research/su2-long-window-20261119/gates.py` without edits.

C-prefix pass, G-time pass, P-long (native) PASS (1.32 < 1.38), P-long (two-ring) FAIL (1.62 < 2.69),
P-cross PASS (two-ring / native at n = 2: 0.80 -> 0.98 over the window).
