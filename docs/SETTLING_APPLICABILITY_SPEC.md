# Settling applicability: smallest justified fix

Version 0.1, 2026-11-13. **SPECIFIED, NOT IMPLEMENTED. Gates below not run.**
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

Each gate requires a behavioral RED against the old adapter, a GREEN against the
new adapter, then a deliberate mechanism removal causing an assertion failure.
Import errors and missing fixtures do not count. No gate accepts two alternative
verdicts for one fixture. Freeze numerical thresholds before implementation.

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
