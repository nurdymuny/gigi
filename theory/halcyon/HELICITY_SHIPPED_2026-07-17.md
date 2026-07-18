# HELICITY — shipped 2026-07-17

Discrete Chern-Simons / fluid-helicity observable for Halcyon's velocity
bundles (Navier-Stokes Tier 1, Ask 1). Verb, kernel, grammar, and
executor arms landed additively; RED → GREEN via math anchors N1–N6.

## What it computes

`H = ∫ u·ω dV` (vorticity `ω = ∇×u`) read as the lattice Chern-Simons
functional `H = Σ A∧dA` of an edge-endpoint velocity 1-form. Dense,
`O(V)`, deterministic, exact-on-the-lattice.

```
HELICITY <bundle> ON FIBER (<a_field>) [ON LATTICE <l>] [DENSITY];
→ { helicity, n_edges_used, n_cells, mode_used = "chern_simons" }
  (+ density Vector with DENSITY)
```

Pinned co-located plaquette circulations (periodic wrap on `s+ê`):

```
H = Σ_s [ A_x(s)·Ω_x(s) + A_y(s)·Ω_y(s) + A_z(s)·Ω_z(s) ]
Ω_x(s) = A_y(s) + A_z(s+ŷ) − A_y(s+ẑ) − A_z(s)   # yz-plaquette ⟂ x
Ω_y(s) = A_z(s) + A_x(s+ẑ) − A_z(s+x̂) − A_x(s)   # zx-plaquette ⟂ y
Ω_z(s) = A_x(s) + A_y(s+x̂) − A_x(s+ŷ) − A_y(s)   # xy-plaquette ⟂ z
```

Sites `vid(i,j,k) = (i·L + j)·L + k`; `L = round(∛(max_vid+1))`,
validated (`L³ == max_vid+1`, `L ≥ 2`, exactly `3·L³` forward edges);
non-cubic / non-3D / partial / malformed → typed error.

## ABC Beltrami convergence table (N1, the headline)

`u = (sin z + cos y, sin x + cos z, sin y + cos x)`, `a_e = u_d(site)·h`,
`h = 2π/L`. `∇×u = u` ⇒ density `|u|²` ⇒ discrete closed form
`H(L) = 12·π²·L·sin(2π/L) → 24π³ = 744.150640…` as `L → ∞`.

| L  | measured H  | ratio to 24π³ | Hallie's target |
|----|-------------|---------------|-----------------|
| 16 | 725.171345  | 0.974495      | 725.171         |
| 24 | 735.679177  | 0.988616      | 735.679         |
| 32 | 739.378291  | 0.993587      | 739.378         |
| 48 | 742.027324  | 0.997147      | 742.027         |

Kernel reproduces the closed form to < 1e-9 and the golden targets to
< 5e-4. (Hallie's headline `744.1516` is a ~0.001 transcription slip;
the exact continuum invariant is `24π³ ≈ 744.1506`, consistent with
every ratio in the ask.)

## Sign / chirality control (N2, load-bearing)

ABC's (x,y,z)-symmetry cannot pin the `Ω` orientation on its own, so the
sign is pinned by an exact chirality pair (`L=4`, `C=1`):

| field | ∇×A | linking | H (exact) | measured |
|-------|-----|---------|-----------|----------|
| right-handed `A=(0,sin x,cos x)h` | `+A` | n>0 | `+16π²` | +157.913670 |
| left-handed  `A=(0,cos x,sin x)h` | `−A` | n<0 | `−16π²` | −157.913670 |
| unlinked     `A=(0,cos x,0)h`     | —    | n=0 | `0`      | 0 |

Right-handed / positively-linked ⇒ `H > 0`; its mirror is the exact
negation (`H_right + H_left = 0` to 1e-9); a flipped `Ω` orientation
would swap those signs and fail. `H(L) = ±4π²·L·sin(2π/L)`.

## Anchors

- **N1** ABC golden table — reproduced (above).
- **N2** linked-tube / chirality sign — right `+`, left `−` (exact
  mirror), unlink `0`.
- **N3** zero field → `H = 0` exactly.
- **N4** non-cubic V / partial (2D) / non-unit-step edge → typed error.
- **N5** DENSITY per-cell field sums to the scalar to 1e-9; length `L³`.
- **N6** `A → −A` ⇒ `H → +H` (quadratic in `A`).

## Files

- `src/helicity.rs` — kernel `helicity_chern_simons` (+ unit tests).
- `src/parser.rs` — `HELICITY` dispatch, `Statement::Helicity`,
  `parse_helicity`, `execute` arm, SUGGESTABLE_VERBS.
- `src/bin/gigi_stream.rs` — executor arm, `get_bundle_name`,
  `gql_stmt_type_name`.
- `tests/helicity_basic.rs` — N1–N6 integration suite.
- `src/lib.rs` — `pub mod helicity;`.

## Scope

Measures `∫A∧dA` / vortex-line linking (Moffatt) — tooling and evidence
in Bee's framework, **not** a proof of Navier-Stokes regularity. Ask 2
(chosen-field → registry seam, shared with the Poincaré p-sweep) and the
live linking-number reading `∮_{C₂} A₁·dl = κ₁·Lk` via HOLONOMY are
named but **not** built here.

## Live production receipt (2026-07-17)

Deployed to `gigi-stream.fly.dev`, image
`deployment-01KXSRPRNMRSVVAB78AJQCVQSN` (release v254), health `200`.
The ABC Beltrami L=16 bundle (12288 forward edges) built per N1, inserted,
and read end-to-end on production:

```
HELICITY ns_abc_probe ON FIBER (a_e);
→ { "helicity": 725.171344952539, "n_edges_used": 12288,
    "n_cells": 4096, "mode_used": "chern_simons" }          # THE RECEIPT
```

| probe | call | result |
|-------|------|--------|
| P2 | `HELICITY ns_abc_probe ON FIBER (a_e)` | helicity **725.171344952539**, n_edges_used 12288, n_cells 4096, mode_used `chern_simons` |
| P3 | `… DENSITY` | density Vector length 4096, Σ = 725.1713449525386 (= scalar to ~4e-13) |
| P4 | zero-field bundle (L=4) | helicity `0.0`, n_cells 64 |
| P5 | Marcella IMAGINE dim=4 | HTTP 200, endpoint_coherence `1.0` |

The live `725.171344952539` matches the in-kernel closed form
`12π²·16·sin(2π/16)` and Hallie's golden `725.171` to `< 5e-4`. Substrate
`claude_substrate_v0` was wiped by the deploy restart (known fragility)
and restored from the pre-deploy backup — 20/20 records verified.
