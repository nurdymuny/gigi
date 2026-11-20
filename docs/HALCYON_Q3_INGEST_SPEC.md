# Q3 step 2: the ensembles inside GIGI, and the Q3 numbers read back through the engine (spec)

2026-11-20. Written before any ingest code exists. Branch `feat/ym-engine`. Parent:
`docs/HALCYON_QUANTUM_GAP_SPEC.md`, purpose (1): "move the gap measurement into the engine". Until now the Q3
numbers came from `correlator_probe`, which runs the engine's kernels on JSON ensemble files. This step puts the
ensembles into GIGI as bundles and reads every Q3 number back through GQL and REST on the stored data. It is
plumbing for Q4. It adds no physics and changes no Q3 result.

## The stored form (fixed now)

- **One bundle per ensemble:** `q3_k{k}`, for k = 0 … 10 as the ensembles exist.
- **One record per (configuration, time slice):**
  - `id` = cfg · L_t + t (the key);
  - `cfg`, `t` (integers);
  - `s0`, `s1`, `s2`, `s3`: the 0⁺⁺ operator at smearing levels 0, 4, 8, 16;
  - `p0`, `p1`, `p2`, `p3`: the flux loop at the same levels;
  - `plaq`: the configuration's mean plaquette, repeated on each of its slices.
- **Records per bundle:** n_cfg × L_t, for example 100,000 at L = 10 and 240,000 at L = 24. This is exactly the
  form `read_series_bundle` requires: every (cfg, t) slot exactly once.
- **Values:** the float64 values of the ensemble file, unchanged.
- **Engine:**
  - a gigi-stream built from this branch with `--features gauge`, frozen with its SHA-256 recorded;
  - its own port, never 3142 or 3147;
  - its own key;
  - its own data directory outside OneDrive (`%USERPROFILE%\gigi-data\q3-ingest-20261120`), so sync never
    touches it;
  - Bee's engines are never touched.

## Gates

| gate | what must hold | if it fails |
| --- | --- | --- |
| **I-load** | each bundle holds exactly n_cfg × L_t records, and the verbs accept it (every slot once) | stop: ingest error |
| **I-store** | for each bundle and each of the 8 fields, CORRELATOR (GQL, BLOCK 100) equals the probe's correlator and error on the file's series to 10⁻¹² relative at every t. This tests that the stored values are the file's values | stop: storage changed the data |
| **I-parity** | for each bundle and each channel, EFFECTIVE_MASS `ON s0, s1, s2, s3` (resp. `p0 … p3`) `BLOCK 100 GEVP 1, 2`, through GQL and through REST, equals the frozen Q3-C probe on the file to 10⁻¹² relative. Compared: mass, error, t_lo, t_hi, the as-fixed plateau, the kept operators and the number of kept directions. A refusal must be a refusal in both, with the same text up to the transport prefix | stop: the engine and the probe disagree |
| **I-persist** | after the engine is stopped and restarted on the same data directory, the k0 0⁺⁺ EFFECTIVE_MASS row is bit-identical to the one before the restart | stop: the data does not persist |
| **I-null** | for k0, a bundle `q3_k0_null` holding the configuration-and-time shuffled projected 0⁺⁺ series (the harness's null, same seed rule) refuses through GQL | stop |

On a pass, the Q3 numbers are reproduced by the engine on stored data, and step 3 (Q4) reads from the bundles.

## Scope and order

- Ingest the ensembles that exist now (k0 … k8), and k9 and k10 when they finish.
- The harness is idempotent: a bundle already holding the right record count is not reloaded.
- The full Q3 scoring (`e2e/probes/test_q3_extension.py`) is unchanged and still runs on the files. This step only
  shows that the engine gives the same answers on its own stored data.
- Ingest and verbs add CPU load while k9 and k10 sample. That slows the sampling. It does not change the ensembles:
  the parallel sampler is bit-identical for any load and any thread count.

## Not in this step

- New observables or geometric verbs on the stored data. That is Q4, under its own spec.
- Any change to the Q3 gates or results.
