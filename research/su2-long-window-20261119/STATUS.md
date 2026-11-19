# S3c status: partial (5 of 8 runs)

Spec: `docs/RICCI_SU2_PG_FLOW_LONG_WINDOW_SPEC.md` (commit 72216bf, before any measurement).
Gates: `python research/su2-long-window-20261119/gates.py` (reads whatever receipts exist).

Finished receipts in this folder (final; each with its `*.final.npy` state):
L0_native_N40, L0_native_N80, L0_pg_N40, L0_pg_N80, L1_native_N40.

Still running when this was committed (2026-11-19): L1_native_N80, L1_pg_N40, L1_pg_N80.

Preliminary, from the finished runs only (not a verdict):
- C-prefix: pass for L0_native (3.1e-12), L0_pg (2.8e-12), L1_native (2.0e-11).
- G-time at n = 1: pass (native 0.19%, two-ring 0.11% worst change over the five sample times).
- E(0.1): n = 1 native 0.047552, two-ring 0.057207; n = 2 native 0.019045.
- p_native(0.1) = log2(0.047552 / 0.019045) = 1.32, against the threshold 1.63 - 0.25 = 1.38 for
  P-long (native). It cannot be judged until G-time at n = 2 (L1_native_N80) is in, and P-long
  (two-ring) and P-cross need L1_pg_N40/N80.
- Newton: every step converged (at most 13 iterations); one mass refresh in L1_native_N40.
