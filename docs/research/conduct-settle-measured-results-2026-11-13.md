# CONDUCT + SETTLE: measured RC evidence

2026-11-13. **CS08 remains OPEN. No production deployment or commit.**

## What actually ran

All six oscilloscope exports linked by [Stephen Mang's AC Circuits
prelab](https://stephenamang.github.io/AC-Circuits-PreLabVersion.html) were run
through the native REST physics endpoints on an isolated GIGI server. Each has
2,500 measured time/source-voltage/capacitor-voltage rows: 15,000 total. Every
observation was stored and read back unchanged. No waveform was discarded.

The [fixed protocol](conduct-settle-measured-protocol-2026-11-13.md) uses nominal
published R/C values, one initial measured capacitor voltage, and input-only
source reconstruction. It never fits R/C, resets the predicted state to later
observations, adjusts an offset, or tunes a time shift. Source event times are
midpoints between consecutive input samples crossing the declared threshold.

CONDUCT received the full three-phase schedule for each trace, queried in
batches at every original observation time plus the switching boundaries.
SETTLE received the predicted state at each of the two switches per trace.
These are REST operations, not newly implemented GQL syntax.

The application parses the exports, reconstructs constant source phases,
orchestrates requests and compares results. GIGI performs circuit evolution and
settling calculations. An independent scalar exponential recurrence checks the
same model, separately from the comparison with measured capacitor voltage.

## Numerical results

Maximum live-engine/oracle disagreement: **3.552713678800501e-15 V**.
This validates agreement with the declared model; it is not a hardware accuracy
claim. Published nominal R/C values span four orders of magnitude in RC time.

The fixed diagnostic screen requires BOTH RMSE <=5% of input swing and maximum
error <=10% of input swing. Input swing is its 95th minus 5th percentile.

| Trace | R (ohm) | C (nF) | RC (ms) | RMSE (V) | RMSE / swing | Max error / swing | Screen |
|---|---:|---:|---:|---:|---:|---:|---|
| rc1 | 1000 | 10 | 0.01 | 0.506696580 | 4.112797% | 9.689044% | Pass |
| rc2 | 10000 | 10 | 0.1 | 0.350183180 | 2.860974% | 5.422656% | Pass |
| rc3 | 100000 | 10 | 1 | 0.613737098 | 4.677874% | 8.602046% | Pass |
| rc4 | 1000 | 1000 | 1 | 0.516830584 | 4.715608% | 11.469180% | Fail |
| rc5 | 10000 | 1000 | 10 | 0.974326156 | 5.941013% | 11.687952% | Fail |
| rc6 | 100000 | 1000 | 100 | 0.507157610 | 5.011439% | 8.668353% | Fail |

Do not round rc6's 5.011439% down to a passing 5%. These screens were fixed
before results and are diagnostic, not hardware acceptance criteria.

### Does the settling decision hold up?

Tolerance is 2% of each trace's input swing. Of 12 post-switch intervals:

- 1 has at least five samples beyond the predicted bound and all remain inside
  tolerance until the next switch.
- 3 have enough follow-up but include observations outside tolerance.
- 8 lack the required follow-up after the bound; they are not successes.

For rc2's first switch, SETTLE returns **0.0003893220346021672 s** at a
**0.2448 V** tolerance. All **248** subsequent observed samples before the next
switch are within tolerance; their largest deviation from the declared target
is **0.16000000000000014 V**. The first measured suffix that stays in tolerance
begins at **0.0003443000000000001 s**. This is evidence for that finite interval,
not a proof about all future hardware behavior.

For rc1's first switch, SETTLE returns **0.000038898365995851165 s** at a
**0.2464 V** tolerance. Observations after that time include **0.48 V** deviation;
the qualifying measured suffix starts later, at **0.00004394 s**. A broadly
plausible curve and a passing whole-trace screen did not justify this tighter
sampling decision. This is precisely why both checks are needed.

## Interpretation and no-go

The plot shows several nonconstant measured source signals within the inferred
plateaus, especially rc5 and rc6. The frozen model uses each plateau's median;
it does not track that drift. Thus discrepancies can include forcing-model
error as well as component tolerance, acquisition effects and unmodeled physics.
These data do not identify which cause dominates. No retrospective parameter
fit or altered source reconstruction is presented as a held-out pass.

The source declares resistor +/-5% and capacitor +/-10% tolerance. We did not
find voltage/timebase calibration certificates, independently measured component
values, probe loading or switch/source-impedance characterization. This is a
switched RC teaching experiment, not a sensor-multiplexer qualification package.
We also have no independent hardware acquisitions under an adaptive-wait policy
versus a fixed-wait policy. **The requested calibrated switching gate cannot be
closed from this evidence.**

The useful result is a working, general-purpose path from real observations and
declared components to predicted trajectories, candidate sampling times and
falsifiable comparisons. It also prevents an unsafe inference: correct RC math
does not establish that a particular instrument is adequately modeled by RC.

## Tests and provenance

- 11 behavioral RED tests were retained before helper implementation. A twelfth
  header-mapping test was added RED before supporting the two published variants.
- 12 final helper tests pass. Removing time-order validation or the all-remaining-
  samples settling check makes the respective test fail by assertion, not import
  or compilation error: 2/2 removals caught.
- The focused Rust circuit and explicit-encoding suites were rerun after the
  experiment: 17 + 4 passed, zero failures. Receipt: `core-tests.log` in the
  measured evidence directory. Existing unrelated compiler warnings remain.
- Native engine source was not changed for this experiment. Its 209-file source
  manifest exactly matches the prior full regression acceptance run: 1,580
  default passes and 2,159 kahler passes, with their disclosed ignored tests.
  Those full suites were not rerun for Python-only research additions.
- `cargo build --features kahler --bin gigi-stream --target-dir target` succeeded
  before the live run. Binary SHA256:
  `11a247319deaebdb8df0544688db1ac4d5b7fb64441fbc24dbb68561eb91b2be`.
- Starting HEAD: `bd47a0b6745ccb2f455c92310c28e7068ca61014` plus the recorded dirty
  source manifest. HEAD alone is not sufficient to reconstruct this build.
- The first attempt (`live-01`) refused before physics because the harness used
  unsupported SELECT ordering syntax and expected longer headers in two exports.
  The admission-only amendment is documented in the protocol. Final `live-02`
  uses supported SELECT and explicit source-specific header mappings.
- The isolated server was stopped after the run. No production data was touched.

Evidence is under `output/conduct-settle/measured-20261113/`:

- [Final numerical report, source/data hashes](evidence/conduct-settle-20261113/report.json)
- Exact HTTP requests and responses (`../../output/conduct-settle/measured-20261113/live-02/receipts.json`, not committed to this repository)
- Six-trace comparison figure (`../../output/conduct-settle/measured-20261113/live-02/measured-comparison.png`, not committed to this repository)
- [Final helper and mechanism-removal receipts](evidence/conduct-settle-20261113/harness-audit.json)

The live directory also contains all downloaded source CSVs, all six pointwise
comparison CSVs, build log and server log. No third-party data redistribution
license is assumed by retaining local research copies.

## Reproduce

From the repository, with a fresh output directory and the existing scientific
Python dependencies:

```powershell
python scripts/test_conduct_settle_measured.py --audit output/conduct-settle/measured-repeat/harness-audit.json
python e2e/probes/conduct_settle_measured.py --build --output output/conduct-settle/measured-repeat/live
python scripts/plot_conduct_settle_measured.py output/conduct-settle/measured-repeat/live
```

The online exports can change; compare their SHA256s to this receipt before
calling a later download an exact reproduction. The retained original bytes
identify this experiment. All failures remain in the report; none are removed.

## Next evidence needed

Obtain a sensor/multiplexer dataset with raw synchronized source/output voltages,
actual switch timestamps, calibration and uncertainty records, independently
measured R/C, source/switch resistance, probe loading, initial conditions and
operating temperature. Reserve held-out acquisitions before choosing tolerances.
Compare early/fixed/model-informed acquisition policies with independent measured
traces, including known omitted-dynamics controls. Require prediction accuracy
AND correct sampling decisions; a low mean error alone must not close the gate.

The [domain dataset audit](conduct-settle-domain-datasets-2026-11-13.md) identifies
concrete virus-sensing, market and ICARUS studies, their public source files and
which parts fit the present engine versus needing a different model.
