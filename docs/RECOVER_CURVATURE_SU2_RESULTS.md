# RECOVER_CURVATURE for SU(2), step S1: results

2026-11-17. Spec: `docs/RECOVER_CURVATURE_SU2_SPEC.md`. **A Python research
prototype, not an engine verb**: `scripts/ricci_su2_recovery.py` (recovery),
`scripts/test_ricci_su2_recovery.py` (G0, G1, M1, M2) and
`scripts/ricci_su2_g2_probe.py` (G2). Everything ran on the paper's frozen meshes.
No force, flow or spectrum is claimed.

## The method in one line

The face flux is the logarithm of the face holonomy. It is turned into register in
the cell's frame by link transport along a breadth-first tree from the cell's
vertex 0. The paper's two-ring fit map, unchanged, is then applied to each su(2)
component.

## G0: the embedded U(1) reproduces the paper

The paper's Abelian cochain, wrapped as SU(2) links along one axis, returns the
paper's coefficients in that component to **1.4×10⁻¹⁵** relative. The other two
components are zero to the same precision. Sample: 12 cells at n = 1 and at n = 2.

## G1: gauge covariance

Random small-angle SU(2) links, then a random gauge rotation at every vertex of the
quotient. The recovered field rotates exactly as `Ad_g` at the frame vertex,
to **1.3×10⁻¹⁴**. This pins the composition and storage conventions by
measurement.

**M1 (no register): covariance fails, with a gap of 14.3.** Turning into register is
what makes the reading covariant.

## G2: exact non-Abelian fields

The fields are the Levi-Civita spin connections of the round metric `[1,1,1]` and
the Berger metric `[3,1,1]`, with links from `ricci_consistency_math.link_quaternions`.
The exact target needs no frame convention of its own. The same exact transport is
applied around small triangles of size 2×10⁻⁵ (chart units) inside each cell, and
the holonomy logarithm is carried straight back to the frame vertex. Each sampled
cell gets 24 triangles, and 60 cells per mesh are drawn with a fixed seed. The error
reported is the relative L² error of the recovered flux over all triangles.

| field | reading | n = 1 | n = 2 | n = 3 | n = 5 | observed orders |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Berger | **covariant two-ring** | 28.8% | 2.51% | 1.12% | **0.40%** | 3.73, **2.03, 2.03** |
| Berger | M1: no register | 27.3% | 9.48% | 6.04% | 3.18% | 1.62, 1.13, 1.27 |
| Berger | M2: abelianised flux | 52.0% | 44.9% | 44.0% | 44.2% | 0.22, 0.05, −0.01 |
| round | **covariant two-ring** | 17.2% | 0.74% | 0.35% | **0.13%** | 4.80, **1.90, 1.93** |
| round | M1: no register | 17.2% | 6.18% | 3.82% | 2.04% | 1.56, 1.21, 1.24 |
| round | M2: abelianised flux | 103% | 100% | 99.8% | 99.8% | 0.05, 0.01, 0.00 |

Receipts: `research/recover-curvature-su2-20261117/g2_berger.json` and `g2_round.json`.

**Reading the table.**

- **Second order.** Past the coarsest mesh, the covariant reading converges at
  second order on both fields, the same order the paper's Abelian recovery reached.
  The non-Abelian Stokes correction the spec worried about does not cost an order
  here, so the midpoint correction S1b is **not needed**.
- **Register buys an order.** Without it the error converges at only about first
  order (1.1–1.3).
- **The abelianised flux cannot see the field.** Summing edge logarithms instead
  of taking the log of the holonomy stalls at 44% (Berger) and 100% (round). On
  the round field the summed logarithms nearly cancel, so the reading is close to
  zero. This is the removal the spec required to fail G2; it passes G0, which
  shows it is invisible to Abelian data.
- **n = 1 is pre-asymptotic.** The two-ring stencil on the 120-cell mesh is large
  relative to the space (the same caveat the paper's earlier work recorded).
  Orders are read from n ≥ 2.

**Not yet explained.** On the round field at n = 1, the full reading and the
no-register reading agree to every printed digit (17.2061%). At every finer level
they differ widely. It is recorded here and not explained.

## Status against the spec

| gate | result |
| --- | --- |
| G0 embedded U(1) | pass (1.4×10⁻¹⁵) |
| G1 gauge covariance | pass (1.3×10⁻¹⁴) |
| G2 exact non-Abelian fields | pass: second order on Berger and round |
| G3 refusals | holonomy-near-−1 refusal written, not yet exercised by a test |
| M1 no register | fails G1 (14.3) and loses an order on G2, as required |
| M2 abelianised flux | invisible on G0, stalls on G2, as required |

These gates went green on their first run. The red evidence is the two mechanism
removals, which fail exactly where the spec said they must.

## Next

- **S1 in the engine:** the `RECOVER_CURVATURE` verb gains the four-field (`q0..q3`)
  mode, with parity against this prototype.
- **S2:** the consistent non-Abelian force, specified once S1 is in the engine. It
  is gated against the native SU(2) Whitney force with BW05 unchanged.
