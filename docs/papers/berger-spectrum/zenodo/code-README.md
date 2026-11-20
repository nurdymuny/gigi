# The Floor Theorem at the Berger Field: code and data

These are the scripts and receipts behind *The Floor Theorem for the Yang–Mills Hessian at the Berger Field: The
Low Spectrum on the Poincaré Homology Sphere* (B. R. Davis, 2026), at their paths in the GIGI repository
(<https://github.com/nurdymuny/gigi>, tag `floor-theorem-v2`).

## What reruns from this folder

The proof scripts need only Python 3.11 or later with the packages in `requirements.txt`:

```
python -m venv .venv && . .venv/bin/activate && pip install -r requirements.txt
```

Every script takes an output path; write to a fresh name and compare it with the committed receipt.

| Paper | Command | Committed receipt | Time |
| --- | --- | --- | --- |
| Prop. 3.1 (the left-invariant sector) | `python scripts/ricci_su2_left_invariant_sector.py out/sector.json` | `research/su2-left-invariant-sector-20261119/sector.json` | seconds |
| The sector matrices, spin j/2 | `python scripts/ricci_su2_spin_sector.py out/spin_sectors.json` | `…/spin_sectors.json` | seconds |
| Gauge content of each sector; Figure 1's ticks | `python scripts/ricci_su2_sector_gauge_check.py out/gauge_check.json` | `…/gauge_check.json` | seconds |
| Prop. 4.1 (the floor formula), symbolic in j | `python scripts/ricci_su2_sector_formula.py > out/sector_formula.txt` | `…/sector_formula.txt` | seconds |
| Prop. 3.1(3) exactly; the double-precision range j ≤ 120 (Remark) | `python scripts/ricci_su2_sector_minimality.py out/minimality.json 120` | `…/minimality.json` | about 2 min |
| Lemma 5.1 (Weitzenböck), Theorem 5.2 and Table 1 (certified inertia: every pivot, ρ, the backward bound, ε) | `python scripts/ricci_su2_gap_certificates.py out/gap_certificates.json` | `…/gap_certificates.json` | about 20 s |
| Proposition 6.1 (the floor law, symbolic in j and t) | `cd scripts && python ricci_su2_sector_formula_squashed.py` | `research/su2-floor-family-20261119/formula_squashed.txt` | about 30 s |
| Theorem 6.2 (the Floor Theorem at (4,1,1)) | `python scripts/ricci_su2_gap_certificates_t.py 4 out/t4.json 136 0,12,20,24,30,32 36` | `…/gap_certificates_t4.json` | about 1 min |
| Theorem 6.3 (the floor law for every t in [2.75, 10]) | `python scripts/ricci_su2_floor_interval_cert.py out/range.json 2.75 10 0,1,2,3,4,5,6,7,8,9,10,11,12,20,24 6` | `…/interval_cert_2.75_10.json` | about 15 min on 6 cores |
| Remark 6.4 (t* and the scan, double precision) | `python scripts/ricci_su2_floor_family_scan.py out/scan.json 40` | `…/scan.json` | a few minutes |
| Section 6 observations (flow; resting fields) | `ricci_su2_soft_mode_flow.py`, `ricci_su2_resting_fields.py`, `ricci_su2_resting_fields_general.py`, `ricci_su2_meron_family_soft.py` | `…/soft_mode_flow.json`, `resting_fields*.json`, `meron_family_soft.json` | minutes each |

## What does not rerun from this folder alone

- **The lattice comparison** (Table 2, the squares and circles in Figure 1). The eigenvalues are included
  (`research/su2-pg-stability-20261117/`, `research/su2-spectrum-n3-20261119/`), as are the eigenvector
  identifications (`research/su2-left-invariant-sector-20261119/eigvec_*.json`) and the script that made them
  (`scripts/ricci_su2_sector_eigvec_check.py`).
  - Recomputing them needs the meshes and the lattice forces. Those are in the code and data of *Reading the
    Curve* (<https://doi.org/10.5281/zenodo.24328085>).
- **Figure 2** is drawn by `docs/papers/berger-spectrum/make_figure_floor_family.py` (matplotlib).
- **Figure 1** is drawn by `docs/papers/two-ring-pg/make_figure_spectrum_ladder.py` from the receipts above
  (it needs matplotlib, also in `requirements.txt`).

## Notes

- `docs/research/su2-exact-sector-*.md` are the research notes written while the result was found, including
  the gauge check and the eigenvector check. They are a record, not part of the proof.
- Licence: the code is PolyForm Noncommercial 1.0.0 (`LICENSE`); the paper is CC BY 4.0.
