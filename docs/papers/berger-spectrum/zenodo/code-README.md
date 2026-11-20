# The Floor Theorem at the Berger Field: code and data

These are the scripts and receipts behind *The Floor Theorem for the Yang–Mills Hessian at the Berger Field: The
Low Spectrum on the Poincaré Homology Sphere* (B. R. Davis, 2026), at their paths in the GIGI repository
(<https://github.com/nurdymuny/gigi>, tag `floor-theorem-v1`).

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
| Lemma 5.1 (Weitzenböck) and Theorem 5.2 (certified inertia) | `python scripts/ricci_su2_gap_certificates.py out/gap_certificates.json` | `…/gap_certificates.json` | about 20 s |

## What does not rerun from this folder alone

- **The lattice comparison** (Table 1, the squares and circles in Figure 1). The eigenvalues are included
  (`research/su2-pg-stability-20261117/`, `research/su2-spectrum-n3-20261119/`), as are the eigenvector
  identifications (`research/su2-left-invariant-sector-20261119/eigvec_*.json`) and the script that made them
  (`scripts/ricci_su2_sector_eigvec_check.py`).
  - Recomputing them needs the meshes and the lattice forces. Those are in the code and data of *Reading the
    Curve* (<https://doi.org/10.5281/zenodo.24328085>).
- **Figure 1** is drawn by `docs/papers/two-ring-pg/make_figure_spectrum_ladder.py` from the receipts above
  (it needs matplotlib, also in `requirements.txt`).

## Notes

- `docs/research/su2-exact-sector-*.md` are the research notes written while the result was found, including
  the gauge check and the eigenvector check. They are a record, not part of the proof.
- Licence: the code is PolyForm Noncommercial 1.0.0 (`LICENSE`); the paper is CC BY 4.0.
