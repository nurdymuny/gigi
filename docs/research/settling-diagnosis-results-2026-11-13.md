# Three settling disagreements: what the controls establish

**No engine mathematics changed. No deployment. CS08 remains OPEN.**
The prior evidence checkpoint is `633a77a`. Investigation follows the separately
frozen `settling-diagnosis-protocol-2026-11-13.md` and uses the same six file
hashes, same nominal components and same first measured initial state.

## Math is not the demonstrated failure

All 15,000 points were replayed on unchanged native CONDUCT using measured-source
left holds, in at most 64-phase batches with predicted state carried forward.
Maximum disagreement with an independent scalar recurrence was
**1.7763568394002505e-15 V**. The original median-source calculations were also
rechecked against their exponential oracle, within 1e-9 V.

A separate linear-input reconstruction agrees with an independently formed
augmented-matrix exponential to **8.970602038971265e-14 V**. Thus both the native
fixed-source evolution and the diagnostic time-varying-input calculation have
independent checks. This rules out a numerical error large enough to explain
these voltage discrepancies in the tested one-capacitor cases, not every
conceivable network or every physical modeling error.

## Source waveform matters, but is not the whole explanation

Keep the original target, original bound and original worst out-of-tolerance
timestamp. The signed decomposition in volts is:

    observed - target = (original model - target)
                      + (recorded-input model - original model)
                      + (observed - recorded-input model)

| Case | Observed - target | Original model - target | Input reconstruction contribution | Remaining residual |
|---|---:|---:|---:|---:|
| rc1 | -0.480000 | -0.246360 | -0.414705 | +0.181065 |
| rc3 | -0.880000 | -0.229462 | -0.211841 | -0.438697 |
| rc4 | -0.400000 | -0.218558 | -0.103062 | -0.078380 |

The identities were checked before rounding. "Input reconstruction" includes
finite source-edge shape and drift, not just a slow drift term. These are
counterfactual model effects, not an experimentally isolated causal assignment.

| Case | Full-trace RMSE, median source (V) | Full-trace RMSE, linear recorded input (V) | Post-bound RMSE, original -> recorded input (V) |
|---|---:|---:|---|
| rc1 | 0.506697 | 0.374715 | 0.149895 -> 0.095538 |
| rc3 | 0.613737 | 0.433254 | 0.647138 -> 0.391426 |
| rc4 | 0.516831 | 0.085621 | 0.078961 -> 0.059399 |

- **rc1:** the measured-input correction is larger than the original model
  residual and is opposed by a positive remainder. Source history matters, but
  replacing the median does not explain the entire measured response.
- **rc3:** the recorded input explains part of the discrepancy; a -0.438697 V
  residual remains at the worst departure. Source-only attribution is rejected.
- **rc4:** the full-trajectory improvement is substantial, but a -0.078380 V
  remainder persists at the original worst settling departure. This supports
  improving the input representation, not claiming a fully explained instrument.

Controls were not omitted: rc2 full-trace RMSE changes from 0.350183 to 0.419488 V;
rc5 from 0.974326 to 1.462501 V; rc6 from 0.507158 to 0.248975 V. A change that
helps one trace is not automatically a universally better hardware model.

## Can ordinary component tolerances explain the remainder?

The source's +/-5% R and +/-10% C gives a product range
`tau in [0.855, 1.155] * R_nominal*C_nominal`, not simply +/-15%.
For a scalar passive RC step, propagate extrema of
`v_next = a*v_previous + (1-a)*u`, `a=exp(-dt/tau)` over both state bounds,
both tau endpoints, and both adjacent input extrema. Positivity gives a
conservative enclosure; it even allows tau to vary between intervals.

Among the observations after the original bounds:

| Case | Outside that enclosure | Largest excess outside (V) |
|---|---:|---:|
| rc1 | 404 / 981 | 0.119602 |
| rc3 | 309 / 309 | 0.323401 |
| rc4 | 17 / 506 | 0.012174 |
| rc2 control | 4 / 248 | 0.039126 |

These recorded values are inconsistent with component tolerance alone **under
the stated input and initial-state assumptions**. The enclosure assumes actual
intersample input stays between measured adjacent values and that the recorded
voltages are exact. Neither is established by calibration here. Calculations
are floating-point enclosures with a 1e-9 V comparison margin, not an
outward-rounded formal interval certificate. Small excesses must not be
interpreted as confirmed circuit defects or assigned physical causes.

## What cannot be proved from these files

Voltage obeys `(R*C)*dv/dt = u-v`. Replacing R by a*R and C by C/a leaves the
trajectory unchanged. Voltage observations alone cannot identify those
components separately. Unknown channel calibration, probe loading, source
impedance, additional dynamics and what the recorded source channel physically
measures remain possible contributors. A fitted time constant or offset would
not identify them either. No component, calibration error or parasitic is
declared to be the proven missing cause.

Source-departure maxima in the JSON include measured finite switching edges near
the midpoint boundaries. They are not estimates of steady-plateau drift alone.
The original interval classifications and thresholds remain unchanged.

## Smallest justified next change

Add an explicit **application-level model-applicability assessment**, and retain
measured input for diagnostic replay. Do not change the RC kernel, fit hidden
components, weaken tolerances or reinterpret SETTLE's model result as a hardware
certificate. The implementation contract and strict RED-first gates are in
`docs/SETTLING_APPLICABILITY_SPEC.md`. That proposed fix is not implemented here.

## Reproduce and inspect

```powershell
python scripts/test_settling_diagnosis.py
python e2e/probes/settling_diagnosis.py --input output/conduct-settle/measured-20261113/live-02 --output output/conduct-settle/diagnosis-repeat
python scripts/test_settling_diagnosis.py --audit output/conduct-settle/diagnosis-repeat/tests.json
```

Ten helper tests first exercised unimplemented stubs and raised runtime errors;
these preliminary failures are retained but do not count as strict behavioral
RED gates. All ten now pass. Subsequent removal of source forcing or tolerance
widening produces the expected assertion failure: 2/2 mechanisms caught. Live
replay rebuilt the same binary hash and verified the original 209-file engine
source manifest before and after execution. The proposed adapter's SA00-SA12
release gates have not run and are not closed by this diagnostic exercise.

Evidence: `output/conduct-settle/diagnosis-20261113/report.json`, `receipts.json`,
`tests.json`, six pointwise diagnosis CSVs, and build/server logs. The report
records the protocol and harness hashes. The server was isolated and stopped.
