# Q3 run log (written before any Q3 correlator is computed)

Spec: `docs/HALCYON_Q3_RUN_SPEC.md` (with Amendment 1, plateau error by jackknife of the fit).

## Timing run (no spectroscopy read; the files were deleted unread)

| lattice | seconds per update (1 HB + 4 OR) | seconds per measurement (levels 0,4,8,16, alpha 0.5) |
|---|---|---|
| 10^4, beta 2.3 | 0.070 | 0.101 |
| 16^4, beta 2.3 | 0.485 | 0.955 |

## Statistics, fixed here

Every point: thermalisation 1,000 updates, then 10,000 measurements, one after every update
(an update is one heat-bath sweep and four overrelaxation sweeps). Smearing levels 0, 4, 8, 16;
alpha 0.5. Jackknife blocks of 100 measurements. GEVP at (t0, t1) = (1, 2) over the four levels,
separately for the 0++ operator and the flux loop.

## Points, in the order the seeds are assigned (seed = 20261193 + 1000 k)

| k | beta | L (L^4) | role |
|---|---|---|---|
| 0 | 2.2986 | 10 | LTW Table 5 calibration |
| 1 | 2.2986 | 14 | second volume |
| 2 | 2.3715 | 12 | LTW Table 5 calibration |
| 3 | 2.3715 | 16 | second volume |
| 4 | 2.4265 | 16 | LTW Table 5 calibration |
| 5 | 2.4265 | 20 | second volume |
| 6 | 2.3 | 16 | Halcyon comparison; level 0 alone is the Q3-r removal arm |

beta = 2.5115 is not run unless the first three betas pass Q3-a and Q3-v (decided before its data).

## Derived quantities, fixed here

- a^2 sigma from the flux-loop mass on the L^4 lattice: a^2 sigma = (am + pi/(3L)) / L (LTW eq. 14);
  the verb refuses sigma at a point where aL sqrt(sigma) < 3.
- m0++/sqrt(sigma) = am0++ / sqrt(a^2 sigma), errors by the same jackknife blocks (the two channels
  come from the same configurations).
