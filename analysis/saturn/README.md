# Saturn polar wave modes — OPAL analysis

Public data, reproducible from a URL. HST/WFC3-UVIS cylindrical maps from the
Outer Planet Atmospheres Legacy program (PI Amy Simon, DOI 10.17909/T9G593).
Legacy program, so no proprietary period.

```
https://archive.stsci.edu/hlsps/opal/cycle32/saturn/hlsp_opal_hst_wfc3-uvis_saturn-2025b_f631n_v1_globalmap.fits
```

1800x900 equirectangular, 0.2 deg/px, BITPIX -32 big-endian. Header is plain
ASCII in 2880-byte blocks — astropy is not required.

| script | what it does |
|---|---|
| `series.py`     | mode power vs latitude vs epoch, both poles, with coverage as the control |
| `amplitudes.py` | per-mode unnormalised fractional amplitudes -> the GIGI bundle input |
| `jetwidth.py`   | latitudinal envelope FWHM at both poles; the lambda/W test |
| `phase.py`      | System III longitude of the pattern per epoch; drift rate; hexagon calibration |
| `phase2.py`     | phase as fingerprint: the 2023 two-lobe question and the FQ889N question |
| `vertical.py`   | amplitude and peak latitude per filter (= per altitude) per year |

Then `cargo run --release --example saturn_mode_bundle -- mode_amplitudes.json`.

The five forward predictions these support are registered, with falsification
criteria, in [`PREDICTIONS.md`](PREDICTIONS.md).

## Measured

- decagon at **-63.3 deg**, n=29 independent measurements, sd 0.65 deg
  (published value: 63 deg south)
- m=10 mean amplitude 3.6x higher at the jet ring than at a -45 deg control ring
- **System III drift: under-determined.** Phase is known mod 36 deg and three
  annual epochs admit four drifts within noise (+0.46, -0.12, -0.014, +0.36
  deg/day; `stress_p2.py`). "Locked, at the hexagon's rate" is one of the
  four, not a finding. An earlier version of this file claimed it was; the
  alias sweep retracted it. Each new epoch removes aliases.
- **the southern jet is narrowing**: FWHM 2.9 deg (2024) -> 1.7 deg (2025) on
  contrast-matched filters, and its cross-filter width scatter fell 0.33 -> 0.13
  deg, beyond what the 1.1x amplitude rise explains. An earlier version of this
  line added peak-latitude and phase scatter "collapsing together"; gated to
  reliable filters, tilt was flat and phase loosened (`stress_p3.py`). One
  observable, not three.
- **no second lobe.** An earlier version of this line reported a stronger m=10
  at -69 deg in 2023 that "collapsed" by 2024. The spectrum at -69 is a smooth
  1/f slope with m=10 below the flat level (`stress_p4.py`); it was red noise
  in a limb ring at 84 deg emission angle, inflated by the fractional-amplitude
  estimator's small denominator. The decagon was born at 63S as published.
- FQ889N (strong methane, stratosphere): m=10 **not detected above noise** -
  its phase is random relative to the wave filters (rms 12.1 deg vs 10.4
  expected for noise). An earlier note here said "absent" and "ceiling"; that
  overstated it.

## Two nulls, recorded as nulls

**Bimodality.** If the decagon were a two-regime transition in the Davis sense,
m=10 at the jet ring should drive var/range^2 toward the Popoviciu bound 0.25.
Measured 0.054. Nothing on any ring exceeds 0.11. Under-powered by construction:
after excluding the contaminated 2021 epoch the clean set is three time points,
all of them after the transition.

**lambda/W = constant.** Inconclusive, because the southern jet is not in steady
state — and that is the finding. lambda/W is 4.85 north, and in the south it
goes 6.11 (2024) -> 10.07 (2025). The south is moving AWAY from the northern
value, not converging on it.

## Forward prediction

If the decagon is locking in rather than dispersing, the 2026 OPAL epoch should
show **W_south <= 1.8 deg with cross-filter sd <= 0.2 deg**, and lambda/W at or
above 10. If instead W widens back toward 2.8 deg, the wave is oscillating, not
settling. Falsifiable at the next observation, on data that will be public.
