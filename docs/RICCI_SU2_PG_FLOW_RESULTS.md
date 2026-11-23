# S3: time integration of the SU(2) two-ring force (results)

2026-11-18. The spec was fixed before measurement: `docs/RICCI_SU2_PG_FLOW_SPEC.md`, with
Amendment 1 (corrected negative controls), written after the n = 1 results and before any
n = 2 run finished. Research script `scripts/ricci_su2_pg_flow.py`, **not an engine run**.
A replica PASS is research evidence, not a BW07 PASS, which needs the native engine path
(BW08). Receipts: `research/su2-flow-20261118/` (one JSON per run, with the full E(t), D(t)
history).

## In plain English

We let the lattice evolve under the two-ring force for a short window (t from 0 to 0.02),
while the true geometry flows underneath it, and asked how far the lattice ends up from
where the true geometry says it should be.

At the finer lattice (n = 2) the two-ring force ends **1.76% off**, measured against how far
the field actually moved. The error drops by a factor of about 7.7 from n = 1 to n = 2. The
standard (native) force ends 2.21% off, so the two-ring force has overtaken it. Halving the
time step changes nothing visible (a part in 10¹⁰), so this is the lattice's error, not the
stepper's.

The frozen-metric control, which pretends the geometry stands still, ends 8.07% off at
n = 2, and its error grows steadily with time while the real run's error levels off. At
n = 2 that gap is finally large enough to count: the control now detects what it was built
to detect.

One thing did not go as the amendment predicted: the native force's error is still
falling at n = 2. It has not stalled.

## Results

R = E(0.02)/D(0.02): the error over the distance actually travelled. Mesh size halves per
level, so the observed order is log₂ of the ratio of E between levels.

| Run | n | N steps | E(0.02) | **R** | max E/D over t | gauge-invariant R |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| two-ring PG | 1 | 32 | 0.06886 | 13.81% | 23.87% | 10.80% |
| native Whitney | 1 | 32 | 0.03493 | 7.01% | 14.64% | 3.10% |
| two-ring, frozen metric | 1 | 32 | 0.06988 | 14.02% | 23.82% | 9.39% |
| one ring (removal) | 1 | 32 | blew up | 2,541% | — | — |
| **two-ring PG** | 2 | 64 | 0.0089955 | **1.763%** | 2.76% | 1.82% |
| two-ring PG | 2 | 128 | 0.0089955 | 1.763% | — | 1.82% |
| native Whitney | 2 | 64 | 0.011270 | 2.209% | 4.86% | 2.10% |
| two-ring, frozen metric | 2 | 64 | 0.041181 | 8.071% | 8.07% | 5.70% |

## Gates

| Gate (fixed before measurement) | Verdict |
| --- | --- |
| **G-traj**: R < 5% at the finest level, observed order ≥ 0.5 | **PASS**: R = 1.76% at n = 2; order 2.94 (E 0.0689 → 0.0090) |
| **G-time**: halving h changes E by < E/10 | **PASS** at both levels: 1.6×10⁻⁶ relative at n = 1, 2.6×10⁻¹⁰ at n = 2 (N = 64 vs 128) |
| **G-quad**: q4 vs q6 changes E by < E/10 | **PASS** at n = 1 (4.6×10⁻⁸); not rerun at n = 2 |
| Frozen metric, as written (R ≥ 10% at every level) | **FAIL** at n = 2 (8.07%). Amendment 1 explains why this bar was wrong |
| Frozen metric, amended: informative? (Δ > 2E_m) | n = 1: NOT INFORMATIVE. **n = 2: INFORMATIVE**, decided by bounds: Δ ≥ \|E_f − E_m\| = 0.0322 > 2E_m = 0.0180 |
| Frozen metric, amended: E_f ≥ Δ/2 | **PASS** at n = 2: Δ ≤ E_f + E_m = 0.0502, so Δ/2 ≤ 0.0251 < E_f = 0.0412 |
| Frozen metric, amended: E_f does not converge between informative levels | **not judgeable**: n = 2 is the only informative level |
| Native, as written (R ≥ 10%) | **FAIL** at both levels |
| Native, amended: crossover E_pg < E_native at the finest level | **PASS**: 0.00900 < 0.01127 |
| Native, amended: stall (order of E_native < 0.25) | **FAIL**: order 1.63 (0.0349 → 0.0113) |
| One ring (reported) | blew up at n = 1, as predicted by S2b's eigenvalues |

The n = 2 runs started before Amendment 1 and do not save final link states, so Δ for the
frozen control is bounded, not measured. Here the bounds decide every amended frozen-metric
verdict, so no rerun is needed.

## Honest observations

- **The two-ring force converges in time, fast.** Order 2.94 between n = 1 and n = 2 is
  well above the 0.5 the gate asked for. With only two levels this is one ratio, not a
  measured rate; n = 3 would confirm it.
- **The native force does not stall in time.** S2a found its *instantaneous* error near 25%
  at every level. Its *trajectory* error over this window falls with order 1.63. The two are
  not in contradiction: the force error need not accumulate in the links over a window this
  short, and the trajectory error includes how well each lattice represents the exact
  links. I have not separated those two pieces. The check that would: record the native
  instantaneous force error along its own trajectory, or run n = 3 to see whether the
  native trajectory error flattens there. As it stands, the amended stall prediction
  failed, and that is reported as such.
- **The crossover held.** Native beats two rings at n = 1 (7.0% against 13.8%), and two rings
  beat native at n = 2 (1.76% against 2.21%), in both the raw and the gauge-invariant
  measure.
- **The frozen control's error grows linearly in time at n = 2** (E_f: 0.0058, 0.0147,
  0.0270, 0.0412 at t = 0.005, 0.010, 0.015, 0.020), while the moving run's error levels
  off (0.0043, 0.0063, 0.0078, 0.0090). That is the signature of a missed moving
  background, seen directly. It is descriptive, not gated.
- **Early-time R is largest because D is smallest.** max E/D is reached at the first sample
  (t = 0.005) in every run except the frozen control at n = 2, whose error outgrows the
  displacement and peaks at t = 0.02. R at t = 0.02 is the gated number.
- **Not measured:** n ≥ 3, other metrics, longer windows, the native engine path (BW08).

## Where this leaves the ladder

The SU(2) two-ring force is now:
- **accurate** (S2a: BW05 on Berger, 2.51% at n = 5);
- **stable** (S2b: no rough modes at n ≤ 2);
- **free of hidden transient growth** (S2c: numerical-range left edge matching native at n ≤ 2);
- **accurate along a trajectory** (S3: R = 1.76% at n = 2, overtaking native, step- and
  quadrature-converged), as a research replica.

Open: whether native stalls in time at n = 3; the exact frozen-metric gap Δ; the engine path.
