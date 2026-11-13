# Measured RC comparison protocol

Frozen before numerical comparison, 2026-11-13. This extends the evidence for
CONDUCT + SETTLE without changing its implementation or weakening CS08.

Admission amendment, before any prediction/residual comparison: the first live
attempt (`live-01`) refused before running physics. The initial four exports
reached an unsupported ORDER BY clause in the runner; use SELECT without that
clause and sort only for application-side round-trip verification. Exports rc5
and rc6 have explicit headers `time,source voltage,Vcapacitor`. Their primary
acquisition page states the same scope format and units as the preceding
exports; admit precisely those headers for those two named sources. Store this
mapping in the receipt, not a guessed unit conversion or fuzzy header match.
No values, component parameters, selection criteria or numerical thresholds
change. Retain the original refusals and original protocol hash in live-01.

## Admission and scope

The primary acquisition source is Stephen Mang's Chem 152 AC Circuits prelab:
https://stephenamang.github.io/AC-Circuits-PreLabVersion.html . It links six
oscilloscope exports with time (seconds), source voltage and capacitor voltage
(volts). It declares resistor tolerance +/-5% and capacitor tolerance +/-10%.
Only headers and initial rows were inspected before this protocol was written.

Use all six linked configurations, in their published order:

| ID | R (ohms) | C (farads) | Public spreadsheet ID |
|---|---:|---:|---|
| rc1 | 1000 | 1e-8 | 1UoV0Pwj85WiBQRafID7pXmjuMdURN4HAUl1gb92KEc4 |
| rc2 | 10000 | 1e-8 | 1YbKOis6QQ40RrqcI8_9EAsbye9MOVV0xxwcnO2fpFBo |
| rc3 | 100000 | 1e-8 | 1I7xIqoPz8CdR-OZh7bdxhMl3n1cZ3WTJ5YErhIdt0oU |
| rc4 | 1000 | 1e-6 | 1txaMWy5D5CtJWt_z-uxNDbZarNIzh7X3ZyduD7LoiQQ |
| rc5 | 10000 | 1e-6 | 1AJfUKWDaShTmqNfTyzUIJsalWfWIZXmyx64_a0Tfkr4 |
| rc6 | 100000 | 1e-6 | 1dyXOdOuLxcAWXYQl_L_vVUSEYxcp5GlXt_VWaLaii4U |

These are source-described measured RC waveforms, not a demonstrated sensor
multiplexer qualification dataset. No voltage/timebase calibration record,
probe-loading characterization or measured switch resistance was found.
Reported voltage units do not establish calibration. CS08 stays OPEN unless
those requirements are independently met. Downloaded files are frozen by SHA256;
do not redistribute them as licensed data without confirmed permission.

## Fixed numerical procedure

1. Read every finite time/input/output row. Require strictly increasing times,
   exact declared headers and agreement between sheet and manifest R/C. Do not
   silently remove malformed observations. Record original timestamps unchanged.
2. Detect square-wave plateaus using the midpoint between the 5th and 95th input
   voltage percentiles. Boundaries are the midpoint between adjacent samples
   whose sides differ. Report half that sample gap as event-time uncertainty.
   Use each plateau's median measured input as its constant source. This is an
   offline forcing reconstruction using input only, not a causal online estimator.
3. Use one capacitor, a port with conductance 1/R, and the published nominal C.
   Initialize once from the first measured output. Predict the entire remaining
   trace without refitting R/C, offsets, time shifts or resetting to observations.
   Missing parasitics are not invented. More than 64 plateaus is a refusal.
4. Submit every measured timestamp to the live engine in batches of at most
   2,048 query points, with the same initial state and full phase schedule.
   Independently verify predictions with the elementary exponential recurrence.
   Require engine/oracle disagreement <=1e-9 V.
5. Report RMSE, maximum error, normalized RMSE and maximum normalized error over
   all observations. Fixed diagnostic screens are normalized RMSE <=5% of input
   swing AND maximum error <=10% of swing. These screens are NOT hardware
   acceptance tolerances and cannot substitute for missing calibration metadata.
6. For each complete switch after the initial partial plateau, call SETTLE from
   the predicted switch state for 2% of input swing. Check observed output at and
   beyond its returned bound, until the next switch. At least five samples are
   needed; otherwise report insufficient follow-up, not success. Independently
   compute the one-node bound R*C*log(|v_initial-U|/tolerance), floored at zero.
7. Observed settling is the earliest sampled time after which all remaining
   samples in that plateau stay in tolerance, with at least five remaining.
   A crossing followed by departure is not settling. No qualifying suffix means
   right-censored. Neither finite observation nor a model bound proves hardware
   behavior after the observed interval.

No numerical screen is tuned after results. Both successes and disagreements
are retained. Source component tolerances are reported, but this experiment does
not implement uncertainty propagation or claim nominal predictions cover them.

## Harness gates

- Behavioral RED before implementation: malformed/nonfinite/duplicate time rows
  refuse; declared component mismatch refuses; source events have correct times;
  early crossings do not count as settling; incomplete follow-up is censored;
  zero modeled error with missing calibration still cannot close CS08.
- A time-order check removal and a settling-suffix check removal must make their
  targeted tests fail. Production physics is not altered.
- Live run: unchanged engine source manifest, recorded binary SHA, isolated
  data directory, exact input/output receipts, all six outcomes retained.
- Rebuild from the existing source before the live run. No new physical model,
  new GQL syntax, production deployment or gate-closing commit is authorized by
  a diagnostic result alone.
