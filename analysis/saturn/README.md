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

Then `cargo run --release --example saturn_mode_bundle -- mode_amplitudes.json`.

## Measured

- decagon at **-63.3 deg**, n=29 independent measurements, sd 0.65 deg
  (published value: 63 deg south)
- m=10 mean amplitude 3.6x higher at the jet ring than at a -45 deg control ring
- absent in FQ889N (strong methane) -> the wave has a ceiling below the
  stratospheric haze
- **the southern jet is narrowing and becoming vertically coherent**:
  FWHM 2.80 deg (2024) -> 1.70 deg (2025), and the scatter across six
  bandpasses collapses from sd 0.65 to sd 0.16

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
