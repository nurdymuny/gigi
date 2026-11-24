# Quasimode construction gates

The dated construction registration precedes these tests. Logs are retained in run order.
Numerical quadrature is not an interval certificate. The full grid remains held.

`red.log` records nine failing behavioral fixtures before implementation; `red-operator.log` records the
missing full-operator behavior. `green-full.log` and `mutations.json` retain the focused construction gates.
The audit has no interval-certification success path. Its output always preserves UNRESOLVED_INTEGRATION.

```powershell
python -m unittest discover -s scripts -p test_helike_quasimode.py -v
python scripts/check_helike_quasimode_mutations.py "$env:TEMP/helike-quasimode-mutants.json"
python -u scripts/run_helike_quasimode.py NEW_EMPTY_CONTROL_FOLDER
```

The last command runs only s=1.15, t=3, g^2=0.004,0.002 and Mq=6,8. It refuses dirty source files,
saves the state coefficients and chart, checkpoints each quadrature stage, and never calls the full grid.
The frozen 10^6 and 12^6 Gaussian nodes are numerical diagnostics; sampled zero leakage is not a tail bound.

## Recorded run

`controls-e21087a1/` is the completed four-control audit from clean source
`e21087a144b5ca9dc7fb8ecad3acdebce9e3f699`. It retains the source snapshots, four compressed state files,
receipt/hash and streamed `events.jsonl`. `control-run-e21087a1.log` is the console log;
`process-exit-e21087a1.json` and `resources-e21087a1.jsonl` retain exit and resource observations.

Status: **UNRESOLVED_INTEGRATION**, not certified, full grid held. All 160 pointwise Cartesian comparisons
pass, but 12 of 16 state/control comparisons fail the fixed 10-versus-12 integration-agreement check.
No validated interval integrator is implemented. Exit 0 means the audit completed, not that its gates passed.

`green-final.log` / `mutations-final.json` record 12 tests and eight assertion-caught mutants before the run.
`convergence-regression.log` records the 27 focused verifier regressions.
The later `red-zero-extension.log`, `green-zero-extension.log`, and `mutations-zero-extension.json` record
the negative-sheet/singular-input guard: one new behavior failure, then 13 passing tests and nine caught mutants.
`replay-e21087a1.json` / `.log` record verification of receipt, source snapshots and state-file hashes,
orthonormality checks, and all 160 point checks repeated from saved states with the post-fix evaluator.
That replay does not redo quadrature or establish rigorous integral bounds.

The first command above reproduces the current focused tests. To reproduce the physical audit exactly,
use a separate clean checkout of the full pinned source revision, install the dependencies recorded in
the receipt and research scripts, and run the last command into a new empty directory. The runner snapshots
the relevant sources itself. Do not overwrite this recorded run or edit its source snapshots.

The [results note](../../docs/research/helike-quasimode-construction-results-2026-11-24.md) includes the
numbers, limits and post-audit integration-specification finding: compact-box integration does not enclose
uncut tails. That mathematical gap must be addressed explicitly before implementing certification.
