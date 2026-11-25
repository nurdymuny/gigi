# helike-lab

A Rust solver (and, from milestone M1, a live viewer) for Yang–Mills Hessians at left-invariant fields on compact
groups. Spec: [`docs/research/helike-lab-rust-spec-2026-11-24.md`](../docs/research/helike-lab-rust-spec-2026-11-24.md).

This is a separate Cargo workspace. It is not part of the `gigi` crate and does not touch the engine, its endpoints or
GQL. The Python scripts in `scripts/` stay the reference: no Rust number is trusted until it matches them.

## What is general, and what is Helike

**The general layer** works for any compact group K, given:
- a frame algebra (structure constants);
- a left-invariant metric diagonal in that basis;
- a gauge algebra;
- a left-invariant field;
- a representation of the frame algebra (the Peter–Weyl sector).

| module | what it gives |
| --- | --- |
| `lie` | Lie algebras by structure constants. Covers su(2) in the frame convention [e_a, e_b] = 2ε e_c and the cross-product convention, su(N) in the generalised Gell-Mann basis, and any closed orthonormal basis of anti-Hermitian matrices. Includes Jacobi and ad-invariance checks. |
| `rep` | Representations: SU(2) spin j/2; SU(N) irreps (p, 0, …, 0, q); tensor products, duals, direct sums; the submodule generated from a vector by lowering operators. Checked by the homomorphism error and the Weyl dimension formula. |
| `sector` | The Hessian H and gauge map G in one sector (a port of `hessian_and_gauge`), curl in three dimensions, curvature, energy, Levi-Civita connection, Ricci. |
| `charge` | Splits a sector by any circle symmetry (frame generator plus gauge generator) into charge blocks of H, H + GG\* and the gauge slice. Signs are fitted, and the commutator and intertwiner residuals are reported, so a non-symmetry shows up as a large residual. |
| `problem` | A JSON problem description: any of the above in one file, solved by `helike solve`. |

**The Helike instance** (`helike`) is the circle-symmetric SU(2) family on the Berger spheres. It includes:
- the twist's closed form and the Floor law;
- the fibre and cross fields, and the flux spectrum;
- the ladder's sufficient floor condition and x_flux;
- the edge functions φ_A (charge blocks) and φ_S (gauge slice), and the edges.

## Parity (milestone M0)

`oracle/make_oracle.py` writes `oracle/oracle_m0.json` from the Python reference. `cargo test --release` compares
against it. Results at commit time:

| quantity | oracle | worst difference |
| --- | --- | --- |
| H, G, curl at 8 random SU(2) family members, j ≤ 4 | `hessian_and_gauge`, `helike_curl_spectrum.operators` | 7.1e-15 entrywise |
| full spectra; gauge-slice spectra (dense and blockwise) | numpy `eigvalsh`, `slice_spectrum` | 3.0e-13 |
| H, G for SU(3) family 2b at t = 0.7, 3, sectors (1,0) and (0,1); spectra and slice in (1,1) | `su3_irrep`, `hessian_and_gauge` | 4.4e-16 entrywise; 6.1e-13 spectra |
| φ_A and φ_S at 5 points, j ≤ 12, with the minimising sector and charge | `ricci_su2_exact_edge` | below 1e-12 |
| the edges x\*_A and x\*_S at 3 cells | `ricci_su2_exact_edge.edge` | below 1e-12 |
| By Enough Table 1, all 30 entries (`helike table1`) | the paper | 0 at six decimals |
| Berger crossings | t_A = 2.73112227823811362…, t\* ≈ 2.698026940520 | 2e-15; 1e-12 |

Each of these deliberate faults must break parity, and each does:
- the flux sign flipped;
- the frame-bracket term dropped;
- a flat metric;
- the lower flux block left untransposed.

An earlier version of the last fault was a no-op, because P^T = −P. The test caught that it caught nothing, and that
version was replaced.

## Using it

```sh
cargo test --release                                  # parity and generality tests (needs oracle/oracle_m0.json)
cargo run --release -p helike-cli -- solve examples/su3_hypercharge_stretch.json
cargo run --release -p helike-cli -- edge --t 3 --y 1.5 --spectrum a
cargo run --release -p helike-cli -- --out receipts/table1_j12.json table1
cargo run --release -p helike-cli -- verify receipts/table1_j12.json
cargo run --release -p helike-cli -- ladder --x -1 --y 1.7320508 --t 3
cargo run --release -p helike-cli -- map --y 1.5 --nt 25 --nx 41 --out receipts/map_y1.5.json
```

Each receipt records:
- the command and its parameters;
- the results;
- the git revision and dirty flag;
- a SHA-256 of the solver sources;
- the platform and thread count;
- a status label: `COMPUTED`, `DISPLAY_GRADE`, `PASS_CERTIFIED_FP` or `PASS_CERTIFIED_INTERVAL`.

`verify` re-runs the recorded command and compares the results.

## The viewer (milestone M1, first cut)

`cargo run --release -p helike-view` opens three scenes. Each panel shows its frame's status label.

**The ladder and flux fan.** Sliders for t, for x and y (or "on the Berger field"), and for the sector. The scene
shows:
- the nine flux eigenvalues, computed (dots) on the closed form (ticks), with the difference printed;
- the rungs of the ladder with their leak arrows, the computed leak against 2[l(l+1) − m(m+1)] (port of the
  receipt's ladder check, now also a unit test);
- per sector, Helike's price against her cheapest gauge-orthogonal rival, with a green dot where the ladder's
  sufficient condition proves the floor.

**Helike on the three-sphere.** Hopf fibres (the long circles) after stereographic projection, and her amplitude
|f|² = cos^{2j}(η/2) and phase as colour and ticks. These are closed forms. The picture is not to scale in t, and the
scene says so.

**The edge forming.**
- φ_A (or φ_S) on an (x, t) grid at fixed y, streamed row by row in a refining order, with the edge φ = 0 drawn as a
  line.
- Overlays: ladder-proved cells, the x_flux curve, the certificate domain D, and the Berger field.
- Clicking a pixel streams that point's margins in every sector.

The solver runs in its own thread and sends `helike_stream::Frame`s, and the drawing code never calls the solver
(`crates/helike-view/src/solver.rs` versus `main.rs`). The same frames can later come over a socket from the big
machine.

`helike-view --shot ladder|sphere|edge out.png` renders one scene once its frames are complete, saves the window as a
PNG and exits. It is used to check the scenes without a person at the screen.

The viewer is drawn with egui's painter on the wgpu backend. It does not yet use the GPU-instanced tubes or the
batched GPU block solver of the spec (M2). It pins eframe 0.35, because 0.37 needs rustc 1.95 and this machine has
1.92.

## The quantum probe (milestone M3, first part): `helike-quantum`

A port of the positive-cone Ritz solver (`scripts/helike_positive_cone.py`, `ConeRitz`) for the sourced
homogeneous condensate. It never forms a moment matrix.
- The Python code builds H from an exact Laurent expansion, one dense moment matrix per term (5,005 × 5,005 at
  Mq = 8).
- Here each moment integral ∫ φ_n φ_m f is evaluated pointwise on a Gauss–Hermite grid, between a separable
  forward transform and its exact adjoint, one (q0, q1, q2) sub-slab at a time. Memory stays at megabytes.
- The integrands are entire functions times polynomials, so the quadrature error falls by about 1000x per extra
  node. It reaches roundoff at 4 extra nodes per axis on the controls. Every result also records the change in
  the levels at 6 extra nodes (`quadrature_check`).
- The lowest levels come from block Lanczos (block 4, for the Helike doublet). Convergence is judged on true
  residuals ‖Hx − θx‖.

The model and chart are general: any helicities ω, coupling g and per-row sources, and any stationary positive
definite background G0 (`Background::from_stationary`). The Helike/competitor identification is a layer on top.

**Parity** (`cargo test --release -p helike-quantum`, against `oracle/oracle_cone.json` from
`oracle/make_cone_oracle.py`):

| quantity | worst difference |
| --- | --- |
| background frequencies, chart S and u0, energy shift | 1e-12 |
| Gram matrix, cometric, log measure, potential at random chart points | 1e-10 relative |
| H entrywise at Mq = 2 and 4; T at Mq = 2 | 6e-13 |
| mass matrix against the identity (Mq = 4) | 1e-12 |
| the lowest 12 levels at Mq = 4 and 6, both controls | 9e-13 |
| Helike and competitor gaps, weights, level indices at Mq = 4 and 6, both controls | 2e-13, same indices |
| reference amplitudes (Mq = 2, order 3) | 1e-10 |

Each of these deliberate faults breaks entrywise parity of H:
- the measure drift dropped;
- the cubic sign flipped;
- a flat metric.

**Commands.**

```sh
cargo run --release -p helike-cli -- cone --g2 0.004 --mq 8
cargo run --release -p helike-cli -- cone-controls --mq 6
cargo run --release -p helike-cli -- --out receipts/cone_convergence_mq6_mq8.json cone-convergence --from 6 --to 8
```

`cone-controls` and `cone-convergence` apply the decision rules of `run_helike_positive_cone_controls.py` and
`run_helike_cone_convergence.py` as written there. The tolerances are unchanged: coefficients within 0.03,
weights at least 0.95, and gap changes at most 0.003.

**Not built yet:** the quasimode certificates (cutoff residuals, Weinstein and Temple bounds, exact SO(2) charge
projection) of `docs/research/helike-quasimode-certification-spec-2026-11-24.md`, and viewer scene 4.

## Status

**Computed, all in double precision on the CPU:**
- sector operators;
- charge blocks;
- gauge slices;
- the edge functions and edges;
- positive-cone Ritz levels of the sourced condensate (interior trial values; no boundary certificate).

**Not yet built:**
- the viewer's remote stream (WebSocket) and scenes 4–5;
- GPU batched blocks (M2);
- the quasimode certificates and scene 4 (the rest of M3);
- interval enclosures (`inari`).

Nothing here is a proof.
