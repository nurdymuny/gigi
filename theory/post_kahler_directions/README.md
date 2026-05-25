# `theory/post_kahler_directions/`

What's next after the Kähler upgrade. Nine differential-geometric
programs, each from a named non-Davis lineage and each numerically
validated, that GIGI can borrow from when the Kähler catalog is
exhausted.

## Files

- **[`catalog.md`](catalog.md)** — full catalog: claim, proof
  sketch, validation, applications, implementation pointers per
  direction. Same format as `theory/kahler_upgrade/catalog.md`.
- **[`validation_tests.py`](validation_tests.py)** — runnable
  Python tests, one section per direction. **30/30 PASS** as of
  2026-05-25. Run with:
  ```
  python -X utf8 validation_tests.py
  ```

## The nine directions

| § | Direction | Source | Cost |
|---|---|---|---|
| 1 | Sasaki / contact geometry | Boyer-Galicki, Sparks | low |
| 2 | Information geometry (Fisher) | Amari, Ay-Jost | low |
| 3 | Optimal transport / Wasserstein | Villani | low |
| 4 | Persistent homology / TDA | Carlsson, Edelsbrunner | low |
| 5 | Gromov δ-hyperbolicity | Gromov, Bridson-Haefliger | medium |
| 6 | Tropical geometry | Maclagan-Sturmfels | medium |
| 7 | Synthetic differential geometry | Kock, Lawvere | medium |
| 8 | Noncommutative geometry | Connes | research |
| 9 | CAT(κ) spaces | Ballmann, Bridson-Haefliger | research |

## How this relates to the Kähler upgrade

`theory/kahler_upgrade/catalog.md` cataloged 21 items from the
Adachi program; 16 shipped as L1–L9 (`src/geometry/`, `src/graph/`,
`src/cost/`, `src/discrete/`); 5 were deliberately deferred. The
present catalog adds 9 *new* directions from outside the Adachi
program — patent-clean public-domain math from other geometric
lineages — for the post-Kähler phase.

None of these are currently scheduled. This document is the menu.
