# Support diagnostic: focused gate record

The mathematical scope and stop rule were recorded before the run in
`docs/research/helike-flux-support-registration-2026-11-26.md`.

- `red.log`: the seven initial Rust tests ran against explicit unimplemented helpers. Six failed and the
  bad-input test passed. This is absent-functionality evidence, not a claim that the old solver was wrong.
- `green-final.log`: eight focused tests pass, including the added non-unit-mass normalization test.
  The nested-block test covers s=1.15, t=3, g^2=0.002 and 0.004 at Mq=2 and 4, using operator actions,
  not full control eigenproblems.
- `python-red.log`: independent-summary test module initially could not import its absent implementation.
- `python-green-final.log`: all four independent-summary tests pass.
- `mutations/mutation-results.json`: the unmodified generated test crate passes; all four deliberately broken
  variants compile, run the eight tests and fail behavioral assertions. Production source is never mutated.
  The per-variant logs retain the precise failing tests.
- `build.log`: the release binary was rebuilt before the live diagnostic.
- `run.log`: final diagnostic status, not a grid-convergence status.
- `live-adversarial.log`: six corruptions of the actual completed export are rejected, with reasons.
  The reusable `check_helike_flux_support_evidence.py` reproduces those checks and the exploratory summaries.

The actual solver's progress log is `../flux-support-20261126/stderr.log`. Raw state coefficients and
projected residuals are stored as binary64 bit patterns in `../flux-support-20261126/support.json`.
Python reads the JSON integers without routing them through a browser's floating-point number type.

## Reproduction

```powershell
cargo test --release --manifest-path helike-lab/Cargo.toml -p helike-quantum --test support_diagnostic -- --nocapture
python -m unittest discover -s scripts -p test_helike_flux_support.py -v
python scripts/check_helike_flux_support_mutations.py research/support-mutation-NEW-RUN
python scripts/check_helike_flux_support_evidence.py research/flux-support-20261126
```

No full suite ran. No new basis family was solved. No domain certificate or full-grid release was produced.
