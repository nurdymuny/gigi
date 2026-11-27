# Focused acceptance evidence

These are reporting/acceptance tests, not numerical evidence for the Hamiltonian. The actual numerical receipts
are in `../flux-basis-smoke-20261126/`.

## Red, then green

Before the diagnostic repair, the new test
`Robustness.test_export_refusal_keeps_the_actual_message_and_exit_code` failed with
`KeyError: 'returncode'`. The old wrapper kept only `CalledProcessError: ...`, discarding the refusal's actual
stderr, stdout and return code. The three-line repair retains those fields and the command, while still
returning OPEN. This failure was observed in the terminal before editing the wrapper; this paragraph is a
transcription of that observation, not a saved raw red-test log.

The new smoke-harness test file was written first and initially failed to import the absent implementation.
That establishes missing functionality, not a mathematical failure. The subsequent eight deliberate mechanism
removals establish that the implemented acceptance checks actually reject broken behavior.

## Focused results

- `focused-tests.log`: 10 smoke-harness tests pass, including timeout and nonzero-exit cases. Their solver
  calls are mocked; messages about a timeout in this file are planted fixtures, not a real numerical timeout.
- `mutations.log`: all 8 deliberately broken variants are rejected by behavioral tests: ignore basis change,
  use the wrong overlap, weaken export selection, bypass reproduction, ignore residuals, release the grid,
  reuse a previous output directory, or report failed execution as completed.
- `grid-decision-tests.log`: 36 grid acceptance/aggregation tests pass, including the new refusal test.
- `grid-tests.log`: the earlier attempt at all 37 tests in that one module passed 36 and errored in
  `DomainStage.test_control_reproduces_the_archived_bounds` because the new binary was still compiling.
  No physics computation occurred in that failed test. It is explicitly excluded from the subsequent
  36-test run: it is a separate live interval-domain experiment, not a reporting/decision test.

No full repository suite was run, and no full-suite pass is claimed. No numerical solver source was edited.

## Commands

From the repository root:

```powershell
python -m unittest discover -s scripts -p test_helike_flux_basis_smoke.py -v
python scripts/check_helike_flux_basis_smoke_mutations.py
$env:PYTHONPATH='scripts'
python -m unittest test_helike_flux_grid.Design test_helike_flux_grid.Point test_helike_flux_grid.Case test_helike_flux_grid.Robustness test_helike_flux_grid.Dressed test_helike_flux_grid.Release test_helike_flux_grid.CrossCheck test_helike_flux_grid.CutGate test_helike_flux_grid.DomainStage.test_verdict_aggregation -v
```

The report-only numerical experiment is separately registered in
`docs/research/helike-flux-basis-smoke-registration-2026-11-26.md`. `run.log` preserves its streamed output.
