# CONDUCT + SETTLE: dataset audit and domain examples

2026-11-13. **Discovery is not validation.** Only the six RC traces in the
[measured results](conduct-settle-measured-results-2026-11-13.md) ran through
CONDUCT + SETTLE in this follow-up. The examples below identify real sources,
specific useful questions, implementation fit and missing acceptance evidence.

The current verbs implement passive linear RC networks in declared SI electrical
units. They do not infer components from arbitrary data. Naming a market field
`voltage`, or an aircraft state `capacitance`, would not make its physics valid.

## Search for a qualifying switching dataset

| Primary source | What is available | Why it does not close CS08 |
|---|---|---|
| [Shabanski and Marinov, Sensors 2025](https://www.mdpi.com/1424-8220/25/16/4960), [author repository](https://github.com/bshabanski/stm32_bsrr_mux_control) | Actual sensor/ADC switching research, BOM, control code, scope images and simulation archive | No raw numeric measured waveforms found in the inspected repository tree. Simulation files and figure pixels cannot substitute for calibrated traces. |
| [LabJack analog-input settling application note](https://support.labjack.com/docs/analog-input-settling-time-app-note) | Instrument-specific measured settling plots and source-impedance discussion | No synchronized raw waveform/calibration package found; active amplifier behavior may also exceed the RC model. |
| [NASA PCoE capacitor repository](https://www.nasa.gov/intelligent-systems-division/discovery-and-systems-health/pcoe/pcoe-data-set-repository/) | Capacitor electrical-stress experiment described by the original research group | Current official page directs a data request; legacy EOS archive request returned 403. No downloaded measured dataset to validate. |
| [Mang RC scope exports](https://stephenamang.github.io/AC-Circuits-PreLabVersion.html) | Six measured source/output traces with times, nominal R/C and tolerances | Ran all six, but calibration and switch/probe characterization were not found. Several source signals are not constant between switches. Results are mixed. |
| [Morales and Risch electrochemistry dataset](https://doi.org/10.6084/m9.figshare.14143751.v1), [paper](https://doi.org/10.1088/2515-7655/ABEE33) | Public RC-dummy-cell/electrochemical measurement data | Inspected cyclic-voltammetry export supplies voltage/current at scan rates, not measured time-domain switching records. Do not manufacture timestamps to qualify it. |

This is a record of the sources inspected, not a claim that no suitable public
dataset exists anywhere. We have not contacted authors or requested accounts.

## 1. Virus sensing: readiness before interpretation

**Research question:** Is a changed electrical reading a reproducible assay
response, or are we reading while the instrument is still relaxing after a
voltage/channel change?

Public example: Guo et al., *Rapid single-molecule detection of COVID-19 and MERS
antigens via nanobody-functionalized organic electrochemical transistors*,
[paper](https://doi.org/10.1038/s41551-021-00734-9),
[versioned dataset](https://doi.org/10.6084/m9.figshare.14420588.v2).
The authors provide figure data for antigen sensing, including clinical sample
experiments. The CC BY 4.0 repository's 22 workbooks were downloaded, hashed and
checked as intact XLSX archives. Their numerical assay results were not analyzed
or replayed in GIGI in this task.

**Useful proposed GIGI workflow:** retain sample, device, batch, blank/positive
control and acquisition identities; keep biological evidence separate from a
separately characterized passive acquisition circuit. For that circuit only,
CONDUCT predicts switching transients and SETTLE computes a candidate wait for a
declared voltage-error requirement. Compare with observed blank traces before
using the assay reading. A failed readiness check says "measurement not yet
qualified", not "virus absent" or "virus present".

**What is missing:** independent frontend R/C and switching characterization,
calibrated timestamped transient measurements, blinded assay batches and a
separate diagnostic validation protocol. The active transistor and electrochemical
reaction are not themselves the passive RC network. The file named
`SupplementaryFigS25_calibration.xlsx` is not treated as an instrument calibration
certificate merely because of its name. No GIGI sensitivity/specificity or
diagnostic benefit is claimed.

A second particularly relevant study is [Gecgel et al., Selective Electrochemical
Detection of SARS-CoV-2](https://pmc.ncbi.nlm.nih.gov/articles/PMC9502341/): its
published protocol uses short voltage pulses and time-resolved currents. Its raw
data is available by request, so it is a promising author collaboration rather
than a completed public-data benchmark.

## 2. Finance: liquidity recovery, not electrical settling

**Research question:** After a visible execution removes liquidity, does the
spread/depth actually recover, or merely cross a threshold briefly before
deteriorating again?

Public example: [LOBSTER's NASDAQ sample files](https://php.lobsterdata.com/info/DataSamples.php).
The downloaded AAPL 2012-06-21 level-one archive contains **118,497 message rows
and 118,497 corresponding order-book rows**. Integrity checks passed. Its
README specifies seconds after midnight, prices scaled by 10,000, order events,
and best bid/ask price/size. This is actual market data, not a simulated tape.
Use its academic/research access terms; do not assume commercial redistribution.

**Useful proposed GIGI workflow:** preserve event sequence IDs to align the two
files. Decode prices explicitly, retain the real clock, handle halts and dummy
empty levels by their documented meanings, and derive spread, depth and signed
executed volume in an auditable application step. Existing CADENCE can examine
the event clock; TEXTURE and PRECEDENCE can examine ordered fields and return
their uncertainty diagnostics. Their routes and implementation exist, but this
sample has not been run through those verbs in this task.

Define recovery episodes, baselines, overlapping-shock handling and minimum
follow-up before evaluation. Compare first crossing with a finite-window
stay-in-band rule. Report censored episodes rather than declaring recovery from
insufficient follow-up. This is the same *experimental discipline* as SETTLE,
not the same physical equation or an implemented market-recovery verb.

**What is missing for CONDUCT + SETTLE:** a justified dynamical model with correct
financial units, identified parameters and held-out predictive evidence. Markets
are driven by new orders and shocks; passive RC decay cannot certify that future
prices or liquidity remain in a band. No trading return, causal lead, or risk
reduction was computed or claimed here.

## 3. ICARUS / aerospace: sensor readiness and real flight faults

**Research question:** After a sensor handoff or actuator fault, is a changed
reading an acquisition transient, an aircraft response, or an actual fault?

Public example: [CMU ALFA](https://theairlab.org/alfa-dataset/),
[archived dataset](https://doi.org/10.1184/R1/12707963). Its published processed
collection has 47 autonomous-flight sequences with fault type/time labels and
CSV, MAT and ROS formats. Metadata and README were downloaded and hashed; the
271,996,798-byte processed archive was located but not downloaded or replayed.
The README declares CC0 and describes real flights recorded in 2018.

A concrete comparison pair from the publisher's sequence table is
`2018-10-05-14-37-22_1` (no failure) and `_2` (right aileron stuck at zero, no
emergency trajectory reaction). These are source labels, not GIGI findings.

**Useful proposed ICARUS workflow:** retain time, sensor/actuator identity,
commanded versus measured state, flight regime and fault ground truth. Compare
pre-fault and post-fault behavior without training or tuning on held-out flight
labels. Keep the acquisition-layer readiness calculation separate from the
flight-dynamics/fault-detection calculation.

For an independently characterized onboard ADC/multiplexer frontend, CONDUCT
and SETTLE could determine when a newly selected channel is ready to sample.
Then ICARUS can evaluate the qualified observation against its flight model.
ALFA provides a real-flight benchmark for that downstream analysis, but does
not by itself supply the electrical calibration needed to validate the first
step. Closed-loop aircraft dynamics and stuck control surfaces are not passive
RC decay, and no flight-release/safety claim follows from this experiment.

The NASA capacitor-aging source above is a closer electrical-aerospace match
for CONDUCT, but its measured archive must first be obtained from the data owner.

## Reproducible source inventory

`output/conduct-settle/measured-20261113/domain-inventory/inventory.json` retains
source URLs, original metadata versions, file names/sizes, byte hashes, archive
checks and financial row counts. Downloaded files remain local research inputs.

```powershell
python scripts/conduct_settle_dataset_inventory.py --output output/conduct-settle/domain-inventory-repeat
```

The directory must be new. This command downloads and inventories data; it does
not run domain physics, train a classifier or declare a benchmark pass.

**Priority recommendation:** acquire the missing calibrated sensor/ADC switching
package first, using the concrete requirements in the measured-results report.
For the next domain study, virus-sensor measurement readiness is the nearest
instrumentation use case; market recovery and aircraft fault response need
their own model and acceptance contracts rather than a relabeled RC example.
