# Convergence amendment: measured improvement, incomplete survey

23 November 2026. Numerical sphere-model results, not a certified spectral gap.

**Terminal status: RESOURCE_STOP.** Twenty-four cases were saved through cutoff 14. No cutoff-16 case was
saved. The registered 8-GiB host-memory budget was crossed; it was not raised. All three adjusted-width tracks
improved on their alpha=1 controls, but none passed the final residual requirement at cutoff 14.

## Provenance

- Attempt 1: `20261124T032926Z-92f909f6`, source `d521bb1b3ed76b0cb9e97d78cb34ba6c4b142a3c`.
  Prerequisites and alpha selection completed; a one-ULP eigenvalue-order inversion stopped the first case.
  The failure, worker log and original source snapshot are retained, not replaced by the retry.
- Retry: `20261124T033143Z-aca2f5ca`, source `87dbfd383ad995af6295a03beb883593b74afa86`.
  Stable ordering of eigenvalues and matching eigenvectors corrected that failure without altering values,
  the model, couplings, width-selection rule, grouping tolerances or acceptance thresholds.
- Hardware recorded by the runner: NVIDIA GeForce RTX 5070 Laptop GPU, CUDA 12.8,
  PyTorch `2.11.0.dev20260108+cu128`.
- The correction passed 31 amendment tests and all 20 original focused CPU/CUDA tests. All 16 mutation
  variants were rejected by assertions, with no unexpected test errors. No full GIGI engine suite was run.

The frozen ground-energy rule at cutoff 10 selected alpha=1.7 for g2=1, alpha=1.35 for g2=0.5, and alpha=2
for g2=2. The last selection is at the grid boundary. The grid was not expanded after seeing that result.

## Latest saved comparison

All rows below use cutoff 14 and the original model's fixed energy units. These are finite-cutoff observations.
The response matrices use the physical q-observables, not unscaled x-coordinate substitutes.

| Coupling g2 | Selected alpha | Finite gap | Control worst residual | Adjusted worst residual | Allowed residual | Both adjusted probe-tail checks |
| --- | --- | --- | --- | --- | --- | --- |
| 0.5 | 1.35 | 1.5593691390 | 0.293570715 | 0.173430643 | 0.015593691 | pass |
| 1 | 1.7 | 2.2700278242 | 0.620502747 | 0.111475665 | 0.022700278 | pass |
| 2 | 2 | 3.0197343812 | 1.527998688 | 0.136490038 | 0.030197344 | pass |

Worst residuals fell by approximately 1.7x, 5.6x and 11.2x relative to the same-cutoff controls. That is evidence
that width choice matters. It is not evidence that width alone solves convergence. The adjusted residuals remain
approximately 11.1x, 4.9x and 4.5x above their respective acceptance lines.

The unchanged symmetry distinction also remains visible: at g2=0.5 and 1 the lowest excited level is accessible
to the scalar H0 channel, while H2 reports a selection-rule zero there. At g2=2 the lowest level is accessible
to H2 and forbidden to H0. The measured weights and full spin decomposition are retained in each case.

## Where the limit comes from

The residual threshold is an experimental accuracy policy fixed in the
[original registration](helike-quantum-probe-preregistration-2026-11-23.md), not a physical constant, a Clay
requirement, or a consequence of the Davis theory:

```text
allowed residual = 0.01 * max(1, finite-cutoff gap)
```

Thus a gap of 2.2700278242 gives a limit of 0.0227002782. The minimum allowance is 0.01 if the gap is below one.
The other requirements still apply: stability across two successive cutoff increments and discarded squared
probe response at most 0.001. Passing one does not waive the others.

The measured residual is the norm of the full Hamiltonian action minus energy times the candidate state.
The finite matrix solve checks the equation inside the chosen basis. The full-space check also includes the
directions the Hamiltonian reaches outside that basis. It does not mean that the reported energy error equals
the residual, or that a failed numerical tolerance disproves the theory.

For the adjusted g2=1 case at cutoff 14, the residuals identify the states driving the failure:

| Checked level | Energy above the ground state | Full-space residual |
| --- | --- | --- |
| Ground state, spin 0 | 0 | 0.0262598382 |
| First scalar excitation, spin 0 | 2.2700278242 | 0.1114756647 |
| First spin-2 level, five states | 2.6246018492 | 0.0834419178 |

The reported worst value comes from the first scalar excitation, not the ground state. The width-selection
objective minimized ground energy; it did not minimize excited-state residuals. This identifies a distinction
to investigate, not a demonstrated unique cause or a reason to move the tolerance.

## Resource stop

The supervisor recorded a peak worker RSS of 9,302,429,696 bytes (8.66356 GiB), above the unchanged
8,589,934,592-byte budget. Sampling can overshoot between checks, as disclosed in the amendment. The stop was
observed roughly twelve and a half minutes after launch, well before the 45-minute time ceiling.

This was a configured host-RAM stop, not evidence of GPU memory exhaustion or exhaustion of the machine's
physical RAM. The separate CUDA allocation budget remained 4 GiB. The worker was stopped before it could
publish its final CUDA peak counters. Device-wide GPU samples must not be presented as process allocation peaks.

All six cutoff-14 cases completed. The next requested cutoff was 16. The current worker has no per-allocation
phase tracing, so these logs do not establish the exact allocation or function that crossed the ceiling.

## Evidence and live logs

[Complete run directory](../../research/helike-quantum-probe-20261123/convergence-20261124T033143Z-aca2f5ca/):

- `receipt.json`: authoritative stopped-run status, gates, alpha tables, all case diagnostics and array hashes.
- `worker.log`: stdout/stderr from the calculation, including each completed-case gap.
- `live-results.csv`: one row per saved case, with gaps, residuals, allowances, probe tails and timing fields.
- `live-progress.jsonl`: full observed case snapshots, resource samples and terminal receipt.
- `partial-replay.json`: the separate verification result and per-case replay timings. It cannot upgrade this run.
- `N*.npz`: all 24 saved finite matrices, eigenpairs, physical probes, alpha and coupling.
- `source/`: exact source snapshot and the registered contracts.

The live observer was attached at 03:41:58 UTC, after 23 cases were already saved. Those cases are explicitly
marked as pre-existing. It collected 27 resource samples through the terminal observation at 03:44:12 UTC.
Earlier case results were already recorded by the runner; earlier resource history cannot be reconstructed.
Observation timestamps are not invented case-completion timestamps.

Case timing covers `case()` computation. It excludes subsequent NPZ compression/writing and saved-array replay.
The same basis-build duration is attached to all cases sharing a cutoff: do not sum it six times. Solver timings
are wall-clock times for their respective solver paths, including their validation work, not isolated CUDA-kernel
times. These boundaries matter when diagnosing a bottleneck.

The follow-up replay checks source and snapshot hashes, array integrity, finite spectra, metric orthogonality,
spin labels, absolute probe norms and kernels against the saved matrices. It does not rebuild the large exterior
action a second time. Its scope and acceptance flag are explicit in `partial-replay.json`.
All 24 cases passed that replay in about 109 seconds. The verdict is `PARTIAL_EVIDENCE_VERIFIED`, with
`scientific_acceptance: false`; the original run remains `RESOURCE_STOP`.

To replay the saved finite cases without rebuilding their exterior actions, from the matching source revision:

```python
import sys
from pathlib import Path
from threadpoolctl import threadpool_limits
sys.path.insert(0, "scripts")
import run_ym_helike_convergence as runner
folder = Path("research/helike-quantum-probe-20261123/convergence-20261124T033143Z-aca2f5ca")
receipt = runner.read_json(folder / "receipt.json")
assert receipt["source_hashes"] == runner.source_hashes()
assert all(runner.digest(folder / "source" / name) == sha
           for name, sha in receipt["source_hashes"].items())
with threadpool_limits(limits=1):
    for track in receipt["results"].values():
        for case in track["history"]:
            assert runner.check_saved_case(folder, case)
```

No COMPLETE or PASS receipt was manufactured. No memory ceiling or mathematical acceptance threshold was
changed to obtain these results. HP09 usefulness remains unrun, and numerical convergence remains unaccepted.
