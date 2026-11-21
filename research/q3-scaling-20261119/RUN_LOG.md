# Q3 run log (written before any Q3 correlator is computed)

Spec: `docs/HALCYON_Q3_RUN_SPEC.md` (with Amendment 1, plateau error by jackknife of the fit).

## Timing run (no spectroscopy read; the files were deleted unread)

| lattice | seconds per update (1 HB + 4 OR) | seconds per measurement (levels 0,4,8,16, alpha 0.5) |
|---|---|---|
| 10^4, beta 2.3 | 0.070 | 0.101 |
| 16^4, beta 2.3 | 0.485 | 0.955 |

## Statistics, fixed here

Every point: thermalisation 1,000 updates, then 10,000 measurements, one after every update
(an update is one heat-bath sweep and four overrelaxation sweeps). Smearing levels 0, 4, 8, 16;
alpha 0.5. Jackknife blocks of 100 measurements. GEVP at (t0, t1) = (1, 2) over the four levels,
separately for the 0++ operator and the flux loop.

## Points, in the order the seeds are assigned (seed = 20261193 + 1000 k)

| k | beta | L (L^4) | role |
|---|---|---|---|
| 0 | 2.2986 | 10 | LTW Table 5 calibration |
| 1 | 2.2986 | 14 | second volume |
| 2 | 2.3715 | 12 | LTW Table 5 calibration |
| 3 | 2.3715 | 16 | second volume |
| 4 | 2.4265 | 16 | LTW Table 5 calibration |
| 5 | 2.4265 | 20 | second volume |
| 6 | 2.3 | 16 | Halcyon comparison; level 0 alone is the Q3-r removal arm |

beta = 2.5115 is not run unless the first three betas pass Q3-a and Q3-v (decided before its data).

## Derived quantities, fixed here

- a^2 sigma from the flux-loop mass on the L^4 lattice: a^2 sigma = (am + pi/(3L)) / L (LTW eq. 14);
  the verb refuses sigma at a point where aL sqrt(sigma) < 3.
- m0++/sqrt(sigma) = am0++ / sqrt(a^2 sigma), errors by the same jackknife blocks (the two channels
  come from the same configurations).

## Peek before the harness (2026-11-19, 22:1x)

At Bee's request, k0 and k2 were looked at with the fixed settings (GEVP 1,2 over the four levels,
blocks of 100) while k1, k3-k6 were still running. No gate was scored and nothing was changed. The
other five points were not looked at. Any amendment prompted by this peek is judged on points not
peeked at, never on k0 or k2.

## Amendment 2 adopted (Bee, 2026-11-19), before k1, k3-k6 are read

Q1-4 (two-slice window) and Q3-B (operator selection before the GEVP, refusal of a noise-level
projected mode); text in docs/HALCYON_Q3_RUN_SPEC.md. The harness runs with the probe rebuilt from
this commit: correlator_probe.exe sha256 prefix 733b08b6980fce43. The sampler binary is unchanged.

## Run complete and scored (2026-11-20)

All seven points exited 0 (done.txt). The harness was run once with the probe at sha256 prefix
733b08b6980fce43 (checked before the run). Gates: Q3-a pass at all three beta; Q3-v pass at 2.3715 and
2.4265, fail at 2.2986 (k1 refused in both channels); Q3-s pass on two points, c = 3.88(46); Q3-r pass;
Q3-n pass (vacuous for k1 0++, no projected series). beta = 2.5115 not run: the pre-registered condition
(first three beta pass Q3-a and Q3-v) is not met. Results: docs/HALCYON_Q3_RESULTS.md. The single-level
k1 numbers in that file are a diagnosis after scoring, not a gate.

## Extension run (Amendment 3 in the spec; written before any extension data, 2026-11-20)

Points k7-k10 (seed = 20261193 + 1000 k), parallel sampler built from 084de9ac with --features gauge,
frozen at target-ym6/q3ext/lattice4d_ensemble.exe, sha256 prefix 670f68e54075c509 (thread-count
identity checked on a 4^4 smoke run: THREADS 2 and 3 give identical series). Statistics as k0-k6.
Thread allocation (does not change the ensemble): k10 8 threads, k9 6, k7 1, k8 1.

| k | beta | L | role |
|---|---|---|---|
| 7 | 2.2986 | 12 | second volume at 2.2986 (replaces k1 in Q3-v) |
| 8 | 2.2986 | 14 | fresh replicate of k1; Q3-C test point |
| 9 | 2.5115 | 20 | LTW Table 5 calibration |
| 10 | 2.5115 | 24 | second volume at 2.5115 |

The Q3-C code and the extension harness are committed before any extension correlator is computed.

Q3-C coded and tested before any extension correlator (unit: on eight noise draws of a near-collinear
third operator the untruncated GEVP fails on all eight and Q3-C recovers m = 0.5 within 3 sigma on all
eight, dropping exactly one direction; on clean operators Q3-C keeps every direction and the vector equals
the untruncated one). An earlier single draw of that fixture (noise seed 23) did not make the
untruncated GEVP fail; that is why the test runs over several draws. Extension harness
e2e/probes/test_q3_extension.py; probe frozen at target-ym6/q3ext/correlator_probe.exe, sha256 prefix
034b57b2d7dde5a8.

## Peek at k7 (Bee's request, 2026-11-20 ~07:20, while k8-k10 sample)

k7 finished at 06:45 and was scored early with a copy of e2e/probes/test_q3_extension.py restricted to k0 and k7 (same
frozen probe 034b57b2d7dde5a8; outputs outside the run directory; receipt k7_peek_receipt.json). Nothing was changed
after it: the harness, the probe, the gates and the full scoring of k0-k10 stand as committed. k7 counts for C-harm
and C-null exactly as registered; the peek is recorded so no one reads it as unseen when the full receipt lands.
Read: Q3-a and Q3-v at beta 2.2986 pass (k0 with k7); C-harm at k7 0++ within 0.64 sigma of the Amendment-2 value;
the Amendment-2 flux loop refused at k7 and the Q3-C one did not (C-harm does not apply there); nulls refuse.

## Step 2: the ensembles inside GIGI (k0-k8), 2026-11-20 ~09:00

docs/HALCYON_Q3_INGEST_SPEC.md (286a7dd1, before code), harness e2e/probes/test_q3_ingest.py (fc436643, before its run).
Engine target-ym6/q3ext/gigi-stream.exe (sha256 prefix cc121c0f174ad266), own port, key and data dir
%USERPROFILE%\gigi-data\q3-ingest-20261120. All gates pass: I-load (1,300,000 records in nine bundles), I-store
(worst relative difference 0.0), I-parity (GQL and REST equal the frozen Q3-C probe, refusals included), I-null,
I-persist (engine killed and restarted: the k0 row is bit-identical). Receipt ingest_receipt_k0_k8.json. The receipt
contains the computed k8 values (parity needs them); they were not read. k8 is scored, with its C-job gate, only by
the registered extension harness.

## 2026-11-20: k9, k10 done; full extension scoring

k7-k10 sampling exited 0 (done_ext.txt). Full scoring by test_q3_extension.py -> q3ext_receipt.json; Q4 rerun -> q4ext_receipt.json. Verdict: Amendment 3 fails C-harm (k10 flux loop shifts 2.0 sigma); C-job, C-null pass; only beta = 2.2986 passes Q3-a and Q3-v under Q3-C, so Q3-s has no fit; Q4 C1 vacuous (one point). Write-up: docs/HALCYON_Q3_EXTENSION_RESULTS.md. Ingest of k9, k10 started into the same data dir (receipt ingest_receipt_k0_k10.json).

## 2026-11-21: Amendment 4 code, frozen binaries, fresh points k11-k19

Amendment 4 adopted (019e934d) before code. Code: `gevp_effective_mass` (A4-1 fallback), `PlateauRule::gevp_amended`
(A4-2), verb and probe (probe "rule": a4 | a4_nostart | q3c | a2). Unit tests: 15/15 correlator tests, including
the four A4 tests. Test 2: the Amendment-2 path refused on 8/8 noise draws (never a wrong mass); the fallback engaged
and recovered 0.5103(100) on 8/8. Test 1 was corrected before data (spec note): A4-2 moves a t = 1 window by design.
Verb tests 2/2.

Frozen at target-ym6/q3a4/ (sha256 prefixes): correlator_probe.exe bde9f81396de89c5, gigi-stream.exe
6422f11170c35aa3, lattice4d_ensemble.exe 670f68e54075c509 (the extension's sampler, copied unchanged).
k11-k19: statistics as before, 1,000 thermalisation, 10,000 measurements, 1 HB + 4 OR, levels 0,4,8,16, alpha 0.5,
seeds 20261193 + 1000 k. Completion in done_a4.txt.

I-parity rerun with the A4 engine and probe on the stored k0-k10 bundles: run 1 (receipt
ingest_receipt_a4_k0_k10_run1_stale_null.json) failed I-null only because of a harness bug: the idempotent loader
reused the null bundle stored from the Q3-C vector while the probe shuffled the A4 vector (both refused). The null
bundle is now keyed to its vector's hash. Run 2 (ingest_receipt_a4_k0_k10.json): I-load, I-store, I-parity (including
the GEVP path), I-null, I-persist all pass.

## 2026-11-21: Amendment 4 scored

k11-k19 sampled (all exit 0). test_q3_a4.py run with the frozen q3a4 probe: outcome failure (one beta passes Q3-a/Q3-v). k17 reproduces k9 excess (pull 5.2); k18 large-volume sigma pull 5.0. A4-start bound changed nothing. Q4: C1 vacuous, C2 holds at 2.2986, K nowhere. See docs/HALCYON_Q3_A4_RESULTS.md.
