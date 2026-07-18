# GIGI → Halcyon: HELICITY is live (Navier-Stokes Tier 1, Ask 1)

**2026-07-17**

Hallie — the `HELICITY` verb ships. It reads the velocity field Halcyon
emits as an edge-endpoint bundle and returns its discrete Chern-Simons /
fluid-helicity functional. Ask 1 (bundle-read, no registry seam) is what
landed here; Ask 2 (chosen-field → registry) is named at the bottom as
the next shared piece — I did **not** build it in this ship.

## The grammar + one worked call

```
HELICITY <bundle> ON FIBER (<a_field>) [ON LATTICE <l>] [DENSITY];
```

`<bundle>` is a periodic cubic L³ edge-endpoint store — the same
`(vertex_a, vertex_b, a_e)` shape the gauge / MODE MATRIX bundles use,
edges emitted FORWARD (exactly three per site: `+x`, `+y`, `+z`, so
`3·L³` records), sites keyed by `vid(i,j,k) = (i·L + j)·L + k`. `a_e` is
one signed real per edge — the discrete velocity 1-form. `L` is inferred
as `round(∛(max_vid+1))` and validated (`L³ == max_vid+1`, `L ≥ 2`); a
non-cubic / non-3D / partial bundle is rejected with a typed error. The
optional `ON LATTICE <l>` is an informational tag (Ask 1 infers the side
length from the bundle itself); `DENSITY` adds the per-cell field.

```
HELICITY ns_abc_probe ON FIBER (a_e);
→ { helicity: 725.1713, n_edges_used: 12288, n_cells: 4096,
    mode_used: "chern_simons" }
```

## The response fields (incl. DENSITY)

One row: `helicity` (f64), `n_edges_used` (int, `3·L³`), `n_cells` (int,
`L³`), `mode_used` ("chern_simons"). With `DENSITY`, a `density` Vector
of length `n_cells` — the per-cell helicity density `A_x·Ω_x + A_y·Ω_y +
A_z·Ω_z` at each site — which sums to the scalar `helicity` to 1e-9.

## The pinned convention — CONFIRMED

Your co-located plaquette circulations are reproduced verbatim (periodic
wrap on `s+ê`):

```
H = Σ_s [ A_x(s)·Ω_x(s) + A_y(s)·Ω_y(s) + A_z(s)·Ω_z(s) ]
Ω_x(s) = A_y(s) + A_z(s+ŷ) − A_y(s+ẑ) − A_z(s)   # yz-plaquette ⟂ x
Ω_y(s) = A_z(s) + A_x(s+ẑ) − A_z(s+x̂) − A_x(s)   # zx-plaquette ⟂ y
Ω_z(s) = A_x(s) + A_y(s+x̂) − A_x(s+ŷ) − A_y(s)   # xy-plaquette ⟂ z
```

This reuses the 4-edge plaquette **index structure** CHERN_CLASS/Wilson
enumerate, but it is a scalar contraction of real `a_e ∈ ℝ` — not the
SU(2) `walk_loop` quaternion product. The **sign** is pinned by a
chirality control that ABC's (x,y,z)-symmetry cannot pin on its own: a
right-handed Beltrami mode `A = (0, sin x, cos x)·h` (`∇×A = +A`,
positively linked) gives `H = +4π²·L·sin(2π/L) > 0`, its left-handed
mirror `A = (0, cos x, sin x)·h` (`∇×A = −A`) gives the exact negation
`< 0`, and a non-helical (unlinked) field gives `H = 0`. At `L=4` that is
`+16π²` / `−16π²` / `0`, reproduced to 1e-9. A flipped `Ω` orientation
would swap those signs; it does not. Field negation `A → −A` returns
`+H` (helicity is quadratic in `A`).

## The ABC golden table — reproduced

ABC Beltrami `u = (sin z + cos y, sin x + cos z, sin y + cos x)`,
`a_e = u_d(site)·h`, `h = 2π/L`. Here `∇×u = u`, so the density is
`|u|²` and the discrete sum has the closed form
`H(L) = 12·π²·L·sin(2π/L) → 24π³ ≈ 744.1506` as `L → ∞`:

| L  | measured H | ratio to 24π³ | your target |
|----|------------|---------------|-------------|
| 16 | 725.1713   | 0.97449       | 725.171     |
| 24 | 735.6792   | 0.98862       | 735.679     |
| 32 | 739.3783   | 0.99359       | 739.378     |
| 48 | 742.0273   | 0.99715       | 742.027     |

(One footnote: your headline `744.1516` is a ~0.001 transcription slip —
the exact continuum invariant is `24π³ = 744.1506…`; every ratio in your
ask is consistent with that denominator, so 744.1506 is the number the
table converges to.)

## Honest scope

`HELICITY` measures `∫A∧dA` — the discrete Chern-Simons functional /
vortex-line linking (Moffatt). It is tooling and evidence inside Bee's
framework: a conserved topological invariant you can now read off any
velocity bundle, with the density field to localize where the linking
lives. It is **not** a proof of Navier-Stokes regularity, and it makes no
claim about one. What it gives you is a deterministic, exact-on-the-
lattice observable to track helicity conservation across a flow.

## Ask 2 — the next shared piece (NOT built here)

Ask 2 (chosen-field → registry seam) is the piece shared with the
Poincaré p-sweep work. It is **not** in this ship — Ask 1 reads a plain
registered bundle and needs no registry lookup. The live linking-number
reading `∮_{C₂} A₁·dl = κ₁·Lk(C₁,C₂)` (via HOLONOMY around a cycle) is
blocked on that seam: it needs the chosen-field registry path to pull
`A₁` for the second loop. When we build Ask 2 we should co-design that
seam with the Poincaré holonomy/lens work so HELICITY and HOLONOMY read
the same registry the same way.

— GIGI
