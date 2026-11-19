# Social cards

Six cards in the paper's plate style, made with Astra's plate helpers and her
depth-sorted transparency renderer (`../make_polished_plates.py`,
`../transparent_tetra_renderer.py`). The 3-D scenes use the paper's saved geometry
(`../data/rings_3d.json`); the flow card reads `../data/su2_flow.json`, which
`../make_figures_su2flow.py` writes from the committed receipts.

```bash
python docs/papers/two-ring-pg/social/make_linkedin_cards.py
```

| Card | Size | Numbers, and where they come from |
| --- | --- | --- |
| `01_cover_one_vs_two_rings.png` | 1200 × 1200 | 253.5 and 23.1: cell 0's fit amplification (`data/rings_3d.json`); 26% and 1.45%: paper, Section 7 |
| `02_fold_keep_what_matters.png` | 1200 × 1200 | 2–4 of about 10, 48 runaway modes, 1.57% against 1.45%: `docs/RING_FOLD_RESULTS.md`, `docs/RING_FOLD_FORCE_RESULTS.md` |
| `03_results.png` | 1200 × 1200 | 26% to 1.45% (paper §7); 25.5% to 2.51% (paper §9.2, research code); 0.87% (paper §9.3, Table 9; `research/su2-cayley-20261118/L2_pg_N8.json`); 0 and 1,451 runaway modes at n ≤ 2 (`docs/RICCI_SU2_PG_STABILITY_RESULTS.md`); 10⁻¹⁴ engine parity (`research/recover-curvature-parity-20261117/`) |
| `04_three_ways_across_a_turn.png` | 1200 × 1200 | 57.30°, 60°, 90°: *Three Ways Across a Turn* (doi:10.5281/zenodo.24375338), the ticket game (checked by its `verify.py`) |
| `05_link_preview_1200x627.png` | 1200 × 627 | 26% and 1.45%: paper §7 |
| `06_letting_the_lattice_move.png` | 1200 × 1200 | R at n = 1, 2, 3 for two rings, standard and frozen background: `data/su2_flow.json` (receipts in `research/su2-flow-20261118/` and `research/su2-cayley-20261118/`); 10.6%: paper §9.3, `docs/RICCI_SU2_PG_FLOW_CAYLEY_RESULTS.md` |

The post text is `LINKEDIN_POST.md`.
