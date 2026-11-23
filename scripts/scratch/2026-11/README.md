# Working scratch scripts, November 2026

These are the small helpers I ran from a scratch folder outside the repository while doing the work of 17–23 November
2026. They are kept here so that every computation is in version control. **None of them is cited as the source of a
result.** Every cited result has its own script under `scripts/` and a receipt under `research/`.

They fall into three kinds:
- **Text-edit helpers:** they applied edits to papers, notes or the book (`*_edits.py`, `draft_apply.py`,
  `edit_floor_allj.py`, `insert_figs.py`, `refs.py`, `gigi_section.py`, `mf_*.py`, `docs_update.py`). The edits they
  made are in the commit history of the files they touched.
- **Run launchers:** shell scripts that started the long runs (`run_*.sh`, `wp8G_chain.sh`). They hard-code the
  scratch paths of the machine they ran on.
- **Checks and probes:**
  - `ceiling_check.py`, `tA_scan.py`, `gscan.py`, `gdiag.py`, `su2_*_test.py`, `decomp.py`, `run_l1.py`,
    `sdirk_check.py`, `m0_main.py`, `skew_smooth*.py`, `r*.py`, `gather.py`: exploratory checks behind the floor and
    variational work.
  - The Test G helpers:
    - `gk0_test.py` (the fast code against Test F's code);
    - `g_geom.py`, `g_geom2.py`, `g_struct2.py`, `g_reach.py` (structure only: arc angles, ties, zero roots, y_θ
      placement, sector reach);
    - `g_time.py`, `g_timing.py`, `gx_time.py`, `gx_prof.py` (timing only; they print no eigenvalue);
    - `gx_dev.py` (the matrix-free check evaluator against the block evaluator, on Test F's groups only).

    These are the development runs disclosed in §0 of the Test G registration.

Paths inside them point at the scratch folder they ran from. To rerun one, change its paths.
