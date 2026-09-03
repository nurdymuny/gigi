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
- **locked to System III**: drift -0.0141 deg/day, residual 1.2 deg rms against
  a 1.1 deg same-day noise floor; the hexagon's published value is about
  -0.013 deg/day. Method calibrated on the hexagon itself in 2018-19.
- **the southern jet is narrowing and becoming vertically coherent**:
  FWHM 2.80 deg (2024) -> 1.70 deg (2025); scatter across bandpasses in width
  (sd 0.65 -> 0.16), peak latitude (0.45 -> 0.15) and phase (0.62 -> 0.51,
  from 2.9 in 2023) all collapse together
- **two lobes in 2023, one by 2024**: a stronger m=10 at -69 deg and a weaker
  one at -63 deg, separated by an amplitude node at -65; the poleward lobe
  collapsed >10x within a year and the equatorward one won
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
