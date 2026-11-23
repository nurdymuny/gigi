# Settling applicability: evidence without overclaiming

Status: **SA00-SA12 application-adapter gates PASS**, 2026-11-13.
CS08 hardware qualification remains OPEN. No production deployment is included.

## What changed

The measured-data adapter now keeps three different questions separate:

1. What does the declared fixed-source RC model calculate?
2. What do the recorded samples show after its deadline?
3. Does the evidence justify applying that model to this hardware and horizon?

The native CONDUCT and SETTLE equations, routes, source measurements and original
screen thresholds are unchanged. This is a Python application-level assessment,
not a new GQL verb or a change to the native REST response. General multi-node
CONDUCT/SETTLE remains available; the new applicability assessment is explicitly
limited to one-node, one-port SI RC models with a unit readout.

Calibration/forcing evidence is bound to a source, revision, content digest and
time window. Missing uncertainty stays unknown. Evidence integrity does not
prove that a caller's certificate is truthful: any positive applicability status
is conditional on that supplied evidence and restricted to its declared horizon.
Hardware qualification cannot be enabled by this adapter.

## The real-data difference

Six frozen oscilloscope exports provide 15,000 observations and 12 switch
intervals. The final isolated live run preserves every original classification:
one interval within tolerance, three observed departures, eight with insufficient
follow-up. All twelve forcing assessments are `not_established`; all remain
`not_qualified` for hardware. No missing calibration was replaced with zero.

First-switch results below are rounded for readability. Receipts retain the
full floating-point values and the entire native response.

| Trace | Model bound (s) | Tolerance (V) | Post-bound samples | Largest recorded error (V) | Observed window |
|---|---:|---:|---:|---:|---|
| rc1 | 0.000038898366 | 0.2464 | 981 | 0.48 | departure |
| rc2 | 0.000389322035 | 0.2448 | 248 | 0.16 | within tolerance |
| rc3 | 0.003762868538 | 0.2624 | 309 | 0.88 | departure |
| rc4 | 0.003891067035 | 0.2192 | 506 | 0.40 | departure |
| rc5 | 0.031824989233 | 0.3280 | 0 | unavailable | insufficient follow-up |
| rc6 | 0.300138872010 | 0.2024 | 0 | unavailable | insufficient follow-up |

The rc2 control is particularly useful. Its 248 post-bound readings span
0.0000494 seconds, and their largest recorded departure is 0.16 V, below
0.2448 V. That remains an observed success. It does not establish a continuously
constant source, calibrated voltages, or readiness outside that finite record.
The adapter returns all three facts together, with specific missing-evidence
requests, rather than forcing an engineer to infer them from one Boolean.

The rc1, rc3 and rc4 departures also remain departures. The adapter does not
explain them away or identify an unmeasured parasitic component. See the
[controlled diagnosis](research/settling-diagnosis-results-2026-11-13.md) for the
measured-input contributions, remaining residuals and identifiability limits.
The replay also preserves the adverse rc5 control: full-trace RMSE is
0.9743261557 V with the original input model and 1.4625010101 V with linear
recorded-input reconstruction. Measured-input replay is diagnostic, not a claim
that every trace improves.

## Adversarial controls

The first new-contract run had 25 assertion failures and no import/runtime
errors. A retained schema bridge makes the old adapter's absent fields explicit
as `None`; these are missing-contract controls, not claims that the old code
returned invented positive verdicts. The old Boolean qualification shortcut was
also exercised directly: all six metadata Booleans plus a numerical pass could
return true. That behavioral test failed before the shortcut was disabled.

Subsequent review added failing tests for result digest, target/schedule binding,
initial-state origin, native tolerance/component binding and scalar-model scope.
Import errors are never credited as evidence of a caught defect.

The small-disturbance test uses a one-second time constant, initial error 1 V,
tolerance 0.1 V and constant forcing 0.05 V. At the unforced deadline log(10)
seconds, the actual error is 0.05 + 0.95*0.1 = 0.145 V. The observation is a
departure and the fixed-source assumption is contradicted when the planted
uncertainty evidence establishes it, despite 0.05 being smaller than 0.1.

Existing mathematical controls cover exact constant/ramp forcing, tiny-step
cancellation, nonuniform clocks, left/right holds and positive tolerance
enclosures. Native batch carry is checked over more than 64 intervals, with a
removal control that resets state and must fail. No measured output is used to
reinitialize subsequent predictions.

## Gate Record

| Gate | Evidence |
|---|---|
| SA00 | Native content retained; digest, scalar scope, components, target and tolerance checked. |
| SA01 | Planted constant-source control with explicitly supplied evidence; no hardware certificate. |
| SA02 | 0.145 V counterexample and small-disturbance removal control. |
| SA03 | In-band crossing followed by departure; suffix-check removal fails. |
| SA04 | Observed prefix and short certificate cannot establish a future horizon. |
| SA05 | Missing, expired and wrong-source calibration; clock/voltage uncertainty controls. |
| SA06 | Four/five/zero post-bound samples; count and duration retained. |
| SA07 | Independent exponential/matrix oracles; live 64-phase batches and state-carry removal. |
| SA08 | Tau endpoint and forcing enclosures; unit/origin refusal and identifiability disclosure. |
| SA09 | Raw/evidence/result integrity, monotone finite clocks, revision-sensitive identity, read-only inputs. |
| SA10 | Six frozen traces and twelve verdicts unchanged; no hardware qualification. |
| SA11 | Consumer JSON retains model/observation/hardware scope; label removal fails. No website changes in this gate. |
| SA12 | Final pinned live replay, both mathematical oracles, removal controls, repository regression and native wire parity pass. |

## Final receipts

- Python: 38 applicability tests, 13 measured-adapter tests, 11 diagnostic tests:
  **62 pass**. **30 mechanism removals** produce assertion failures, not runtime
  errors (24 adapter, 3 measured-adapter, 3 diagnostic).
- `cargo test --all-targets`: **1,579 passed, 0 failed, 19 pre-existing ignored**,
  across 220 reported test targets with default features (no features enabled).
- `cargo test --features kahler --test conduct_settle --test conduct_settle_wire`:
  17 kernel tests pass. The receipt-dependent wire test is then explicitly run
  with `--ignored` and the final live receipt file: **1 pass**, verifying both
  CONDUCT and SETTLE through the actual DHOOM decoder against paired JSON.
- No Rust was edited. The full all-targets `kahler` suite was not rerun; the
  feature-enabled checks here are the affected kernel/wire tests and live engine.
- Final live run: **591 HTTP receipts**, six exact observation-storage
  round-trips, all twelve frozen interval verdicts, and 15,000 native left-hold
  points checked. Maximum independent hold-oracle error:
  **1.7763568394002505e-15 V**. Maximum linear-reconstruction/augmented-matrix
  oracle error: **8.970602038971265e-14 V**.
- The engine source manifest matches the previous experiment. The actual final
  executable SHA256 is
  `e83f531647a9da416d502dbdc1d8b9afc3ae9e12532d1970684138251f1b2a8e`.
  All runs used isolated storage and the owned server was stopped.

Retained review artifacts: [live report](research/evidence/settling-applicability-20261113/report.json),
[adapter tests and removals](research/evidence/settling-applicability-20261113/adapter-tests.json),
[measurement tests](research/evidence/settling-applicability-20261113/measured-tests.json),
[diagnostic tests](research/evidence/settling-applicability-20261113/diagnosis-tests.json).
The same directory retains direct behavioral RED logs and Rust command outputs
as text. The local full HTTP receipt file is
`output/conduct-settle/applicability-20261113/live-02/receipts.json`, SHA256
`3dc5893a08d0727ff4c5b676e3d20cef04cddeeebec19fcf7dfab8736f4a2c5f`.
It contains raw observation responses and is not republished in this archive.

## Reproduction

Commands and input locations are in
[the specification](SETTLING_APPLICABILITY_SPEC.md#implemented-surfaces-and-reproduction).
The source CSVs stay local and are not republished. No parameters were fitted,
no source channel was smoothed, and no thresholds were weakened. Native linear
input support is not claimed: linear reconstruction is an application diagnostic.

The current engine checkout contains pre-existing uncommitted changes. A Git
revision alone does not identify the tested engine: release receipts must also
retain the complete source manifest and actual executable hash. The initial live
run's engine manifest matches the previous experiment, but its rebuilt binary
hash differs; byte identity with the earlier build is not claimed.
