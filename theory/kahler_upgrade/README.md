# Kähler Upgrade — Catalog, Validation, Engineering Notes

Drop-in folder for the v3 Kähler-structure proposal: every claim has
either a numerical proof in `validation/` or an explicit "research-mode"
flag. Independent of the main `theory/*.tex` track because this is a
synthesis + engineering catalog, not a primary derivation.

## Layout

- `catalog.md` — 15-item plan, organized as Part I (Adachi-program
  borrows) + Part II (Sudoku-forced consequences). Each item carries
  a precise claim, proof sketch, validation status, product
  applications, and Rust-module pointers.
- `validation/` — three Python files (PyTorch) that numerically validate
  11 of the 15 items at machine epsilon. Designed to be non-circular:
  every numerical computation compared against closed-form ground
  truth derived in a different formalism, with negative cases that
  must fail. Run with `PYTHONIOENCODING=utf-8 python -X utf8 <file>`
  on Windows (Unicode in the printed output).
  - `validation_tests.py` — items 1.1, 1.2, 1.3, 1.4, 1.5, 2.3, 2.5
  - `validation_tests_v2.py` — items 2.1, 2.10
  - `validation_tests_v3.py` — items 2.2, 2.8, 2.9
  - `results_v1.txt`, `results_v2.txt`, `results_v3.txt` — captured
    PASS output from a clean run on Bee's machine, 2026-05-24.

## Relationship to existing GIGI theory

The upgrade is **additive**, not a replacement. The current engine
already implements:
- Fiber bundles with sections, curvature K, Davis capacity C = τ/K
  (`src/curvature.rs`, `src/bundle.rs`, `GIGI_SPEC §3.3 Thm 3.2`)
- Holonomy as a Čech 1-cocycle (`GIGI_SPEC §2 Thm 2.5, §3.4 Defs 3.5–3.6`)
- Sheaf completion via graph Laplacian on the field-index graph
  (`src/sheaf/laplacian.rs`, `src/spectral.rs` Defs 3.9–3.11, Thm 3.4)
- Gauge transformations on schemas (`src/gauge.rs` §5a)
- Atlas with τ-thresholded charts + propagation
  (`src/coherence.rs`)
- Double Cover: S + d² = 1 (`GIGI_SPEC §6 Thm 6.1`)

The Kähler upgrade adds the **complex structure J + closed 2-form B**
on top of this Riemannian/sheaf foundation — turning it into the
Kähler 𝒢 = (M, g, J, ∇, B, Γ) of `catalog.md §1`. Everything below
that line in the catalog is a forced consequence.

## Item ↔ existing-module map (for the implementation sprint)

| Catalog item | Lives in / extends |
|---|---|
| 1.1 Dual adjacency | `src/spectral.rs` (already has graph Laplacian; add second adjacency tier) |
| 1.2 Magnetic 2-form | NEW: `src/geometry/forms.rs` |
| 1.3 Trajectory-ball cost | `src/curvature.rs` + NEW: `src/cost/jacobi_estimator.rs` |
| 1.4 Hadamard substructure | NEW: `HadamardSubstructure` trait, ride on `src/curvature.rs` |
| 1.5 Hadamard-Cartan invertibility | Adds invertible-transport guarantees to `src/sheaf/mod.rs::propagate` |
| 1.6 Hypersurface trajectories | NEW: `src/geometry/hypersurfaces.rs` (research-mode) |
| 2.1 Prequantization line bundle | NEW: `src/geometry/line_bundle.rs` |
| 2.2 Index / Riemann-Roch | NEW: capacity bound in `src/bundle.rs` |
| 2.3 Moment map / Noether | NEW: `src/geometry/moment_map.rs` — natural fit with `src/invariant.rs` |
| 2.4 K-theoretic ops | Lives in op-composition logic (lower priority) |
| 2.5 Spectral gap | `src/spectral.rs` (already implemented; gap = `spectral_gap(store)`) |
| 2.6 Floer | Research-mode |
| 2.7 Mirror symmetry | Research-mode |
| 2.8 Berezin-Toeplitz | Research-mode (clean numerical demo exists) |
| 2.9 Hodge / Witten | NEW: `src/discrete/hodge_complex.rs` (foundation: extends `src/sheaf/laplacian.rs`) |
| 2.10 Frobenius / WDVV | Marcella v3 substrate; not a GIGI engine concern in phase 1 |

## What this is NOT

- Not a replacement for `GIGI_SPEC_v0.1.md` — that remains the
  primary spec for the Riemannian core.
- Not a Marcella spec — Marcella consumes the same Kähler substrate
  but has its own architecture.
- Not yet hooked into the Rust build. Implementation lives in a
  v3 sprint (see `catalog.md §3` for ordering).
