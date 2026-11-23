# Settling applicability: smallest justified fix

Version 0.2, 2026-11-13. **IMPLEMENTED in the application adapter.**
Release status is recorded separately in `SETTLING_APPLICABILITY_RESULTS.md`;
implementation alone does not close a gate. CS08 hardware qualification is OPEN.
Diagnosis: `docs/research/settling-diagnosis-results-2026-11-13.md`.

## Decision and non-goals

No evidence justifies changing CONDUCT's RC equations or SETTLE's fixed-source
modal bound. The demonstrated mismatch is between declared forcing/model and
measurements; exact physical attribution remains underdetermined. The smallest
change belongs in the measured-data application adapter, not the physics kernel.

Replace the adapter's Boolean qualification shortcut with a structured
assessment. Keep the existing REST routes and native model responses unchanged.
No new GQL verb, parameter fitting, source smoothing, calibration inference,
new parasitic circuit, threshold relaxation or production deployment.

## Mathematics

For one capacitor, tau=R*C (tau, pronounced "tau", is the time constant).
For fixed target U and a disturbance d(t), write:

    tau * dv/dt = -(v-U) + d(t).
    v(t)-U = (v(0)-U)*exp(-t/tau)
             + integral_0^t exp(-(t-s)/tau)*d(s)/tau ds.

If an independently justified bound |d(s)|<=D holds over the entire relevant
horizon, then

    |v(t)-U| <= E0*exp(-t/tau) + D*(1-exp(-t/tau)),
    E0=|v(0)-U|.

The current fixed-source bound is the special case D=0. A source departure
smaller than the requested tolerance still consumes error budget: with tau=1 s,
E0=1 V, tolerance=0.1 V and constant d=0.05 V, the unforced bound log(10) seconds
leaves 0.145 V error, not 0.1 V. This analytic counterexample is mandatory.

For D<tolerance and E0>tolerance, the corresponding one-node bound would be
`tau*log((E0-D)/(tolerance-D))`. If E0<=tolerance and D<=tolerance, the bound is
zero. If D>=tolerance and E0>tolerance, this envelope supplies no finite
settling time. These are reference tests and justification for applicability
checks, **not authorization to ship a new uncertain-input solver**.

A measured maximum in a finite record is not a justified bound on future input.
A median is not a constant source. A fitted residual is not calibration.
Uncertainties absent from the source cannot be replaced with zero.

## Adapter contract

Inputs: immutable source/evidence IDs and digests; raw ordered times and voltages;
declared R/C and source schedule; initial-state origin; native model result;
target/tolerance; acquisition window; calibration/uncertainty evidence references;
any independently justified forcing bound and its time horizon.

Return separate fields, never collapse them into a bare `passed`:

- `model_calculation`: numerical result and original conditional scope.
- `observed_window`: `within_tolerance`, `observed_departure`, or
  `insufficient_followup`, with interval/sample counts. Keep the five-sample
  minimum and all-remaining-samples check; disclose duration as well as count.
- `forcing_applicability`: `contradicted`, `not_established`, or
  `supported_over_declared_horizon`; include the exact assumption, evidence
  window, uncertainty policy and corrective reason. No positive status from
  observed source spread alone or from spread merely being below tolerance.
- `hardware_qualification`: `not_qualified` unless a separately approved
  hardware protocol has independently passed every requirement. Unknown is
  not false physics, and correct model arithmetic is not qualified hardware.
- `next_evidence`: field-specific missing measurements or incompatible
  assumptions, not a generic error. Preserve original model values and receipts.

Measured-input replay is diagnostic and opt-in: explicit left/right/linear
interpolation label, raw bytes preserved, no output-based reinitialization.
Use the existing native <=64-phase batching for left holds. Linear interpolation
can remain an independently verified application diagnostic; do not advertise it
as a native CONDUCT input mode. Never silently change a requested source model.

The assessment is pure/read-only. No writes to source measurements or production
objects; model-state bundles used for replay are isolated. Evidence and settings
are part of identity; no cross-revision reuse.

## Strict gated TDD

New adapter behavior requires a behavioral RED against a schema-compatible
bridge to the old Boolean shortcut, a GREEN against the new adapter, then a
deliberate mechanism removal causing an assertion failure. Existing mathematical
controls are regression tests, not retrospectively claimed RED-first tests.
Release and real-data gates require their actual receipts, not fabricated REDs.
Import errors and missing fixtures do not count. No gate accepts two alternative
verdicts for one fixture. Freeze numerical thresholds before implementation.

### Evidence and interpretation details frozen before coding

- The pure adapter accepts source bytes, a declared parsing recipe, aligned
  observations, and content-addressed evidence documents supplied by the caller.
  It checks digests and binds its identity to the complete request, including
  evidence revisions. The ingesting application is responsible for applying the
  declared parser to the source; a digest alone does not prove that mapping.
- Calibration documents must name the source, quantity, SI unit, finite
  nonnegative absolute uncertainty, and coverage interval. Validity means the
  document covers this analysis interval, not merely that a filename exists.
  SHA256 verifies integrity, not the truth or independence of a certificate.
- Positive forcing applicability is conditional on an explicitly supplied
  independent-characterization document bounding departure from the exact
  requested target by zero throughout the entire declared horizon, plus usable
  voltage and timebase uncertainty documents. A nonzero bound does not establish
  the fixed-source assumption, even if below the voltage tolerance. Actual
  samples whose uncertainty intervals exclude the target contradict it.
- Unknown calibration prevents a physical contradiction claim based solely on
  recorded voltages. Report their raw spread separately, with the uncertainty
  unresolved. A positive status is horizon-scoped, never a future extrapolation.
- `observed_window` describes recorded samples only, at nominal timestamps, not
  a continuous-time certificate. Its count/duration, comparison policy and
  unresolved calibration remain explicit. This preserves the frozen 12 verdicts.
- No caller-supplied Boolean or evidence document can enable hardware
  qualification in this version. That requires a separate approved protocol,
  outside this adapter. Every response remains `not_qualified`.
- Native response content must match its declared digest; target and native
  equilibrium must agree within the existing 1e-12 absolute/relative numerical
  check. This check does not alter the observation tolerance. The declared
  schedule must cover the full horizon without any different source value for
  a positive applicability assessment. A conflicting schedule is a declared
  model contradiction, not proof of a physical voltage error.
- This adapter is restricted to a one-node, one-port SI RC model with unit
  readout. Its components must match the native response (1e-12 relative check,
  no absolute slack at small capacitances), and observation tolerance must equal
  the native request exactly. Multi-node native CONDUCT/SETTLE remains available,
  but this scalar applicability layer cannot certify its forcing assumptions.
  A `first_measured_output` initial-state declaration must match that reading.
- Diagnostic replay/enclosure is opt-in, requires explicit SI units and initial
  state origin, and returns its interpolation/uncertainty assumptions. It never
  changes the native result. Enclosures are numerical diagnostic bounds, not
  outward-rounded interval certificates.
- No website deployment or REST response change is part of this adapter gate.
  SA11 tests the published assessment JSON consumed by applications, with the
  three scopes present; the website retains its separately verified labels.

| Gate | Planted answer and required control |
|---|---|
| SA00 Scope | Native fixed-source math still returns its original values. Missing applicability evidence yields `not_established` and `not_qualified`, never a fabricated numerical failure. |
| SA01 Constant control | Exact RC trace, explicitly justified constant source over a finite horizon, adequate samples: `within_tolerance` and horizon-scoped applicability; no automatic hardware certification. |
| SA02 Small disturbance | The 0.05 V forcing counterexample above must reject application of the old 0.1 V fixed-source deadline despite source departure < tolerance. Removing the forcing term must fail. |
| SA03 Crossing trap | Known cancellation/crossing followed by departure is `observed_departure`; suffix-check removal must fail. |
| SA04 Unknown future | Same observed prefix, two declared future source histories with different answers. No evidence for either future: `not_established`, not whichever future looks favorable. |
| SA05 Calibration | Missing/expired/inapplicable voltage or clock calibration is explicitly unresolved. Do not default its uncertainty to zero. A filename containing calibration is not evidence. |
| SA06 Follow-up | Four samples after bound is `insufficient_followup`; five with a final departure is `observed_departure`; five in band supports only that observed interval. |
| SA07 Input reconstruction | Constant and ramp analytic oracles, tiny-step cancellation control, nonuniform times, both hold conventions, and >64-interval native state carry agree within 1e-9 V. Removing source input or state carry fails. |
| SA08 Tolerance enclosure | Cover both tau endpoints and interpolated forcing controls; do not identify R versus C. Missing/wrong units or unknown initial state refuses the enclosure claim. Removing widening fails. |
| SA09 Provenance | Reordered, duplicated or nonfinite times refuse. Changing a source digest, evidence revision or schedule cannot reuse a prior assessment. Raw bytes survive unchanged. |
| SA10 Real-data regression | Preserve all six frozen traces and 12 original classifications. Explain rc1/rc3/rc4 without tuning; retain rc2 control and rc5 deterioration. No new hardware pass may appear. |
| SA11 Publication | Page/API consumers distinguish model calculation, finite observation and hardware qualification. Removing a scope/status label fails a content gate. No medical, trading or flight validation claimed for selected datasets. |
| SA12 Release | Run affected adapter tests, two independent mathematical oracles, removal checks and rebuilt isolated live replay on one pinned source/binary. If Rust changes become necessary, rerun all targets with and without kahler. Require explicit review before any deployment. |

## No-go and remaining experiment

Closing these gates improves interpretation; it does **not** close CS08. That
requires calibrated held-out hardware acquisitions under the competing sampling
policies. Acquire source-terminal and output measurements on a shared calibrated
clock, independent R/C and source/switch/probe characterization, and instrument
channel settings. Those observations distinguish missing dynamics from channel
measurement effects; current files cannot. Do not select a circuit fix until
that evidence identifies one.

## Implemented surfaces and reproduction

- `e2e/probes/settling_applicability.py`: pure assessment and opt-in diagnostic
  reconstruction. Neither is a new REST route or GQL verb.
- `e2e/probes/conduct_settle_measured.py`: includes the structured assessment
  alongside each unchanged native SETTLE result. The old Boolean helper is
  compatibility-only and cannot qualify hardware.
- `e2e/probes/settling_applicability_live.py`: rebuilds an isolated engine,
  preserves original observations and verdicts, rechecks native batching and
  two independent oracles, and records source/binary/evidence identities.

```powershell
python scripts/audit_settling_applicability.py --output output/settling-applicability-tests.json
python scripts/test_conduct_settle_measured.py --audit output/settling-applicability-measured-tests.json
python scripts/test_settling_diagnosis.py --audit output/settling-applicability-diagnosis-tests.json
python e2e/probes/settling_applicability_live.py --input output/conduct-settle/measured-20261113/live-02 --output output/conduct-settle/applicability-repeat
```

The frozen CSVs are local source artifacts, not republished data. The original
measured protocol records their public origins and hashes. `--output` must name
a new directory. Assessment documents refer to a source parser; the live harness
actually applies that parser and verifies the stored measurements round-trip.
