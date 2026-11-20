# The Floor Theorem at the Berger Field: code and data

These are the scripts and receipts behind *The Floor Theorem for the Yang–Mills Hessian at the Berger Field: A
Floor Law Across the Berger Spheres* (B. R. Davis, 2026), at their paths in the GIGI repository
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
| Theorem 6.4 (the Floor Theorem at (4,1,1)) | `python scripts/ricci_su2_gap_certificates_t.py 4 out/t4.json 136 0,12,20,24,30,32 36` | `…/gap_certificates_t4.json` | about 1 min |
| Proposition 6.2 (1): the Weitzenböck identity for every t, symbolic | `python scripts/ricci_su2_weitzenbock_symbolic_t.py out/weitzenbock_t.json` | `…/weitzenbock_symbolic_t.json` | minutes |
| Proposition 6.2 (2), (3): the twist is a lowest Landau level | `python scripts/ricci_su2_landau_symbolic.py out/landau_symbolic.txt` | `…/landau_symbolic.txt` | seconds |
| Lemma 6.5 (charge blocks) and the Gershgorin quantity of Theorem 6.6, exact | `python scripts/ricci_su2_charge_selection_symbolic.py out/charge_selection_symbolic.txt` | `…/charge_selection_symbolic.txt` | a few minutes |
| The closed-form block rows (used by the two above) | `python scripts/ricci_su2_block_rows_symbolic.py out/block_rows_symbolic.json` | `…/block_rows_symbolic.json` | a few minutes |
| Theorem 6.6, sectors j ≤ 40 (block certificates) | `python scripts/ricci_su2_block_interval_cert.py out/b40.json 2.75 10 40 8` | `…/block_cert_j40.json` | about 0.8 CPU-hours (parallel over sectors) |
| Theorem 6.6, sectors 41 ≤ j ≤ 99 | `python scripts/ricci_su2_block_interval_cert.py out/b99.json 2.75 10 99 8 41` | `…/block_cert_j41_99.json` | about 2.1 CPU-hours (parallel over sectors) |
| Theorem 6.6, sectors j ≥ 100 (Gershgorin, interval bisection) | `python scripts/ricci_su2_floor_large_j.py out/large_j.json 50 20 2.75 10` | `…/floor_large_j.json` | seconds |
| Theorem 6.6, the first certificate (full sector matrices, fifteen sectors) | `python scripts/ricci_su2_floor_interval_cert.py out/range.json 2.75 10 0,1,2,3,4,5,6,7,8,9,10,11,12,20,24 6` | `…/interval_cert_2.75_10.json` | about 15 min on 6 cores |
| The charge blocks against the full sector matrices (validation) | `python scripts/ricci_su2_charge_block_direct.py` | (prints the differences) | about a minute |
| Remark 6.7 (t* and the scan, double precision) | `python scripts/ricci_su2_floor_family_scan.py out/scan.json 40` | `…/scan.json` | a few minutes |
| Remark 6.7 (t_A, the threshold of H + GG*, exact) | `python scripts/ricci_su2_tstar_exact.py out/tstar_exact.txt` | `…/tstar_exact.txt` | hours (symbolic determinants) |
| Remark 6.7 (between t_A and 2.75, double precision) | `python scripts/ricci_su2_tA_scan.py out/tA_scan.txt` | `…/tA_scan.txt` | about 10 min |
| Section 6 observation "symmetry does not make the floor": the twist on the Berger field and the squashed meron (Test 2) | `python scripts/ricci_su2_test2_family.py berger out/b.json` and `… meron out/m.json` | `…/test2_berger_control.json`, `…/test2_meron.json` | minutes each |
| … the curvature split R0 = Ric − 2[F, ·] and the twist's shares | `python scripts/ricci_su2_gyromagnetic_split.py out/split.txt` | `…/gyromagnetic_split.txt` | about a minute |
| … the meron's twist is the top of every sector j ≤ 12 for t ≤ 1 | `python scripts/ricci_su2_meron_ceiling_check.py out/ceiling.txt` | `…/meron_ceiling_check.txt` | a few minutes |
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
