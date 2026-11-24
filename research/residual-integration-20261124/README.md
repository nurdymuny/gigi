# Residual integration evidence

Registration: `docs/research/helike-residual-integration-registration-2026-11-24.md`.
The four saved controls are reused without eigensolves, changes of model, thresholds, cutoff or mixing.
The full grid is held. Algebraic numerical moments cannot close the interval certificate gate.

Logs are retained in execution order, including failures. Physical replay follows the focused RED/GREEN
and deliberate mechanism-removal gates, on committed source. No original receipt is overwritten.

## First replay: not an accepted numerical correction

`controls-f2ae42a3/` reproduced all old-rule integrals, but its uncentered algebraic moment values are
superseded as accuracy evidence. The manufactured false-agreement fixture in `red-centering.log` proves
that agreement of those two implementations can coexist with error beyond tolerance. The original
`numerical_diagnosis_passed` field means its then-registered checks passed; it is NOT a current accuracy
approval. Its physical status was and remains UNRESOLVED_INTEGRATION.

The centering amendment registers the minimal algebraic repair before a new run. The failed first repair
is retained in `green-centering.log`; the genuinely passing focused suite is `green-centered-final.log`.

## Centered replay: accepted numerical diagnosis, not a certificate

`controls-d0297072/` retains the corrected run and all 20 pinned source snapshots. All four controls
completed, all 16 moment comparisons passed, all 32 original-rule integrals replayed and all 80 Cartesian
point checks passed. `verification-centered.json` records the independently checked counts, hashes and
per-state differences; `verification-centered.log` records that verification's output.

`control-run-d0297072.log`, `process-exit-d0297072.json`, `resources-centered.jsonl` and the run's
`events.jsonl` retain progress, process outcome and sampled resource use. The runtime inventory is in
`environment.json`. The physical result remains UNRESOLVED_INTEGRATION, not certified, full grid held.

`final-focused-regression.log` records 56 passing tests. `mutations-centered-final-11.json` and its log
record all eleven deliberately removed mechanisms caught by assertion failures, not execution errors.

Reproduction from a clean checkout of `d02970726220fd161fbdf98047610fae18a0cb47`:

```powershell
$env:OPENBLAS_NUM_THREADS='1'
$env:OMP_NUM_THREADS='1'
$env:PYTHONPATH=Join-Path (Get-Location) 'scripts'
python -m unittest -v test_helike_residual_moments test_helike_quasimode test_helike_cone_convergence
python scripts/check_helike_residual_mutations.py NEW_MUTATION_RECEIPT.json
python -u scripts/run_helike_residual_moments.py NEW_EMPTY_CONTROL_FOLDER
```

Keep the original `research/quasimode-construction-20261124/controls-e21087a1/` archive present: the runner
requires those exact four state files and verifies their provenance before measurement. It does not rerun
the eigensolver. Do not overwrite either historical run. There is no interval-certification success path.

[Results and mathematical explanation](../../docs/research/helike-residual-integration-results-2026-11-24.md).
