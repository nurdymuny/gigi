# Two-ring Petrov-Galerkin curvature: BW05 and S3 both pass (Abelian control)

2026-11-17, Claude (second session), building directly on `RICCI_PG_CURVATURE_RESULTS.md`. Branch
`claude/ricci-pg-tworing`, based on `feat/ricci-pg-curvature`. Python research scripts, the same
methods as RF2D–RF2G; nothing ran through the GIGI engine. No native change, full suite, Berger run
or merge.

**Result: the Petrov-Galerkin route Bee chose passes the unchanged BW05 numerical gate AND the
spec's S3 stability test at every level S3 covers.** This applies to the Abelian control only.
**It is not a Berger, SU(2), flow or variational result** (§5).

## 1. The one change

Her reconstruction fits a closed linear curvature on each cell's **one-ring** face-star: 5 cells,
16 faces, 5 closure relations, 11 unknowns. That is exactly determined, so it must interpolate every
data pattern, rough ones included, as a linear field. Rough data then become large linear fields,
which is the source of the S3 failure (negative real parts down to −13,479 at n = 1).

`scripts/ricci_pg_tworing.py` changes only the neighbourhood and the solve:
- **Neighbourhood:** all cells within **two** face-steps of K, each carried onto K's sheet by the
  same left-quaternion transport and coincidence check as the original, applied at the face it was
  reached through. If the ball wraps, the first sheet wins.
- **Solve:** K's own four face fluxes are exact equality constraints, and every other distinct
  neighbourhood face is fitted in unweighted least squares. This is **the original spec's step 5 as
  written**, before the one-ring amendment.

Everything else is imported unchanged: the chart, the 11-function closed linear basis, the flux
rows, the reference mapping, `build()`, the mass, the data and the BW05 norm. The fit stays exact for
closed linear data, but the system is now overdetermined, so rough patterns are no longer amplified.
No parameter was added or fitted. Only ring counts 1 (the original) and 2 were ever run.

## 2. BW05 (the unchanged `numerical_gate`)

Evidence: `research/ricci-pg-tworing-20261117/diagnostic.json`.

| n | h_max | Relative error q6 | q4 | Curvature error | Observed order |
|---|---:|---:|---:|---:|---:|
| 1 | 0.3881 | 6.2230% | 6.2230% | 3.627% | |
| 2 | 0.2017 | 3.8337% | 3.8337% | 0.924% | 0.727 |
| 3 | 0.1355 | 2.7884% | 2.7884% | 0.424% | 0.796 |
| 5 | 0.0816 | **1.4518%** | 1.4518% | 0.155% | **1.286** |

- Finest error 1.45%, against the 5% limit.
- Final two orders 0.80 and 1.29, against the 0.5 required.
- q4/q6 mass differences from 2.3e-7 down to 1e-9, all far below error/10.
- Solve-correction estimates are also below error/10.
- **`numerical_gate` returns `pass: true`.**

For comparison, the one-ring PG gave 1.17% at n = 5, and its static numbers also passed. **What
changes is S3.**

## 3. S3 stability (her criterion, unchanged)

The full generalized eigenproblem of the PG operator against the native mass. "Small" means
|λ| < 1, and that count must equal the Whitney gauge dimension; every other eigenvalue must have
strictly positive real part.

| n | Edges | Gauge (Whitney / PG) | Min physical real part | Negative real parts | Whitney min | Stable |
|---|---:|---:|---:|---:|---:|---|
| 1 | 142 | 21 / 21 | +4.0012 | 0 | 4.0170 | **yes** |
| 2 | 1124 | 163 / 163 | +3.9943 | 0 | 4.0052 | **yes** |
| 3 | 3786 | 545 / 545 | +3.9968 | 0 | 4.0024 | **yes** |

The smallest physical eigenvalue sits at the control's true value, δdA = 4A. The largest is now
close to Whitney's (2,518 vs 2,586 at n = 1), so rough modes are no longer amplified. Level n = 5
is not decomposed, as the spec states.

## 4. Mechanism removals

All of these were measured on the same data:

| Remove | Result |
|---|---|
| Second ring (back to one-ring interpolation) | S3 fails: 70 / 489 negative modes at n = 1 / 2; min −13,479 / −23,940 (her receipts, reproduced here at n = 1) |
| Reconstructed trial (Whitney trial) | BW05 fails: 26.1% floor (RF2D, reproduced exactly) |
| Petrov-Galerkin test (variational force, two-ring trial) | BW05 fails: 45.4% / 276% at n = 1 / 2, growing |
| Petrov-Galerkin symmetrised (`K_PG + K_PGᵀ − K_W`) | BW05 fails: 23% / 276%; the transpose term grows |

Focused tests (`scripts/test_ricci_pg_tworing.py`, 4 pass):
- a planted closed linear field is recovered to 1e-10 on 8 random cells, with more than 16 faces,
  so the system is overdetermined;
- K's own fluxes are preserved and the fit is linear in the data;
- zero data gives zero (gauge invariance is structural, since the fit consumes fluxes only);
- `rings=1` reproduces the original fit to 1e-10.

The harness reproduces her one-ring numbers to every printed digit (29.8111% and 4.8414% at n = 1).

## 5. What this is NOT

- **Not variational.** The PG force is not the gradient of an energy. The handoff's
  strong-adjoint-consistency question for an energy-derived force stays open. Every symmetric
  variant measured here, and in RF2H, fails because the recovery's transpose doesn't preserve
  Whitney moments of smooth fields. Bee chose PG; whether a non-variational stable consistent force
  is acceptable for the chapter is Bee's call.
- **Abelian control only.** No Berger, no SU(2) nonlinearity, no time-stepping or flow, no
  n = 5 S3.
- **Two orders are just above the threshold** (0.73 and 0.80 before the last level), with
  pre-asymptotic behaviour at coarse levels. One more refinement level would strengthen the rate
  claim.
- **No independent replay from a fresh checkout yet.** The diagnostic and source hashes are recorded
  for that purpose.

## 6. Reproduce

```bash
python scripts/ricci_pg_tworing_gate.py output/<new-dir>          # BW05 all eight cases + S3 n=1..3
python -m unittest scripts/test_ricci_pg_tworing.py
python scripts/ricci_pg_tworing_s3.py 1 0                         # one-ring S3 control (fails)
python scripts/ricci_sym_pg_probe.py 0,1 spectrum rings=2         # symmetric/variational controls
```

This needs the checkpoint's archived evidence extracted under `output/`, as for every other
Ricci script.
