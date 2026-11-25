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
- a status label: `COMPUTED`, `DISPLAY_GRADE`, `INTERVAL_ENCLOSURE`, `PASS_CERTIFIED_FP` or `PASS_CERTIFIED_INTERVAL`.

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
- Here the moments are **exact** (`ConeOperator::exact`, modules `laurent` and `kron`). The cometric and potential
  are exact Laurent polynomials in the Cholesky entries; the cometric comes from running the closed-form triangular
  solves on polynomials. They fall into 24 exponential groups with (a, b, c) degree at most 4. Each group is applied
  as F[exp(α·z) q] = E(α)·q(U):
  - E(α) = ⊗_k E_k(α_k) uses the exact one-dimensional moments on shifted Gauss–Hermite nodes, as in
    `one_dimensional_moments`;
  - q(U) uses the exact Hermite position operators.

  E(α) is applied axis by axis on (output prefix, total ≤ Mq + 1) × (input suffix, total ≤ Mq + 5): two independent
  constraints, so every intermediate is a small dense product set. No grid, no dense matrix, no quadrature error.
  A component leaving the exact degree window is a refusal, never a silent drop.
- The original grid operator (`ConeOperator::new`) is kept as an independent implementation. Each moment integral is
  evaluated pointwise on a Gauss–Hermite grid, between a separable forward transform and its exact adjoint. Every
  solve is cross-checked against it (`quadrature_cross_check`).
- Exact against the Python dense matrices: 8.3e-13 entrywise at Mq = 4 (entries up to 16). Exact against the grid
  operator: 7.4e-13. Mq = 6 controls: 7 s exact against 25 s on the grid, on the same machine.
- The lowest levels come from block Lanczos (block 4, for the Helike doublet). Convergence is judged on true
  residuals ‖Hx − θx‖ inside the trial space. Receipts call these `in_space_lanczos_residuals` and
  `max_in_space_lanczos_residual`, because they are not the full-operator residuals ‖(H − E)ψ‖ of the quasimode
  certificate. Those are about 10⁻² to 10⁻³ at the controls, versus about 10⁻¹⁴ to 10⁻¹⁰ here.

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
cargo run --release -p helike-cli -- --out receipts/cone_convergence_mq6_mq8_exact.json cone-convergence --from 6 --to 8
```

`cone-controls` and `cone-convergence` apply the decision rules of `run_helike_positive_cone_controls.py` and
`run_helike_cone_convergence.py` as written there. The tolerances are unchanged: coefficients within 0.03,
weights at least 0.95, and gap changes at most 0.003.

### Residual enclosures (`interval`, `residual`)

These are rigorous bounds on r = ‖(H − E)ψ‖ / ‖ψ‖ for the saved trial states of the quasimode construction
(feat/leakage-gate-amendment, `research/quasimode-construction-20261124/controls-e21087a1`). The code is an interval
port of the centred moment sum `helike_residual_moments.norm_squared` (d0297072):
- (H − E)ψ / f is a finite sum of exp(α·z) times a polynomial (24 groups at every control);
- each pairwise integral is exact by completing the square and translating the polynomial by its finite Taylor sum;
- everything is evaluated in outward-rounded binary64 intervals. That means round to nearest, then one ulp outward,
  with exact zeros kept exact.

The exponential does not use the platform's `exp`. It uses a Cody–Waite reduction and a Taylor polynomial with an
explicit remainder. Both parts of ln 2 are checked against 50 published digits in exact integer arithmetic. Long
sums are added in a balanced tree.

**What is taken as given.** The saved binary64 data define the state exactly:
- the chart (u0, S), which the reference's `ConeChart` rebuilds bit for bit from the saved G0 and M;
- the frequencies w, the coefficients `coeff · mixing` (multiplied here in intervals) and the centres E.

**What is enclosed:**
- S⁻¹: a verified inverse, X + XR + a bounded tail.
- The potential's coefficients: the hull of three readings, namely the declared decimals (s = 1.15, t = 3,
  g² = 0.004 or 0.002), exact arithmetic on the binary64 inputs, and the reference's own binary64 coefficients.
- The u-cometric: an exact integer Laurent polynomial, checked term by term against the reference's sympy expansion.

`oracle/export_quasimode_states.py` writes the states as IEEE bit patterns. It checks the parent receipt and the
SHA-256 of every NPZ file.

```sh
cargo run --release -p helike-cli -- --out receipts/cone_residual_enclosures_e21087a1.json cone-residuals
```

The receipt status is `INTERVAL_ENCLOSURE`: bounds, not the pass or fail of any gate. The four controls take 4 s
(Mq = 6) and 11 s (Mq = 8) each on 16 threads. Every reference value lies inside its interval, from both of the
reference's moment paths, for all 16 states. The relative width of r is 6e-7 to 5e-4. The deliberate faults, each
caught by a test, are:
- the density drift dropped;
- the translation cross terms dropped;
- the exponential factor dropped;
- the complex conjugate dropped.

**Scope.** This is the uncut residual of each saved state under the registered operator. It does not include the
cutoff leakage (Arb, `helike_cutoff_bounds.py`). It does not evaluate the relative leakage gate, and it makes no
spectral statement.

### Charge sectors (`projection`)

This is the exact SO(2) charge projector of the quasimode construction registration (§2), by its norm bound rather
than a finite angular sum. For an integer charge m on this representation, ‖(I − P_m)ψ‖ ≤ ‖(L − m)ψ‖. Take genuine
upper bounds for normalised ψ:
- t ≥ ‖(1 − η)ψ‖;
- c ≥ ‖(L − m)ψ‖;
- r ≥ ‖(H − E)ψ‖;
- ℓ ≥ ‖[H, η]ψ‖.

If t < 1 and q² = (1 − t)² − c² > 0, then:
- ‖P_m ηψ‖ ≥ q;
- the projected state's residual is at most (r + ℓ)/q.

**The generator.** R_θ = diag(1, rot θ) acts by G ↦ R_θ G R_θᵀ, with L = −i d/dθ. In the log-Cholesky chart its
velocity is an exact Laurent field:

    u̇ = (0, −c e^{−u1}, c e^{−u1}, −b, a, e^{u1} − e^{2u2−u1})

This is the same field as `velocity` in `helike_quasimode.geometry`. So (L − m)ψ/f has four exponential groups, and
the residual's centred moment sum encloses its norm. Three checks tie the field to the rotation:
- J_u(L)u̇ = JG − GJ at sample points;
- finite differences of the saved trial function under the actual rotation (Cholesky of R_θGR_θᵀ, back to z)
  agree with the group sum to about 1e-12 in every column;
- the construction receipt's order-12 quadrature `charge_sq` lies inside the enclosure for all 16 states. Order 10
  misses at Mq = 8, by up to 2.8e-5 relative.

**Which trial.** The Arb cutoff run (`helike_cutoff_bounds.py`, receipt c31aab8e) froze the binary64 product
`coeff @ mixing`. Each coefficient here is the interval hull of that stored value and the exact product, so r and c
hold for the same vector as the Arb bounds on t and ℓ. `oracle/export_projection_inputs.py` carries over:
- the stored trial, with its NPZ hash checked against the Arb receipt;
- the Arb upper endpoints of the tail and leakage norms (exact dyadics, rounded upward to binary64);
- the quadrature references.

```sh
cargo run --release -p helike-cli -- --out receipts/cone_sector_bounds_c31aab8e.json cone-sectors
```

| g² | Mq | state | m | c ≤ | q ≥ | (r + ℓ)/q ≤ |
| --- | --- | --- | --- | --- | --- | --- |
| 0.004 | 6 | ground | 0 | 1.591e-3 | 0.999998735 | 0.0151473 |
| 0.004 | 6 | Helike ± | ±2 | 8.234e-3 | 0.999966104 | 0.0487265 |
| 0.004 | 6 | competitor | 0 | 9.700e-3 | 0.999952962 | 0.0801271 |
| 0.004 | 8 | ground | 0 | 5.746e-4 | 0.999999835 | 0.0064685 |
| 0.004 | 8 | Helike ± | ±2 | 1.180e-3 | 0.999999304 | 0.0088218 |
| 0.004 | 8 | competitor | 0 | 1.729e-3 | 0.999998507 | 0.0176969 |
| 0.002 | 6 | ground | 0 | 5.598e-4 | 0.999999843 | 0.0053947 |
| 0.002 | 6 | Helike ± | ±2 | 3.933e-3 | 0.999992267 | 0.0238943 |
| 0.002 | 6 | competitor | 0 | 4.570e-3 | 0.999989561 | 0.0390191 |
| 0.002 | 8 | ground | 0 | 1.896e-4 | 0.999999982 | 0.0021924 |
| 0.002 | 8 | Helike ± | ±2 | 3.645e-4 | 0.999999934 | 0.0028104 |
| 0.002 | 8 | competitor | 0 | 5.153e-4 | 0.999999867 | 0.0055267 |

**Exact endpoints (2026-11-25).** The default `serde_json` float parser is not correctly rounded. It read some
decimal endpoints one ulp off, including the Arb leakage endpoint 2.9219858911830733e-09, which came back one ulp
below the rigorous upper bound. The workspace now enables `float_roundtrip`. The projection also computes every
tail and leakage endpoint from the exact Arb dyadic, rounded upward in integer arithmetic (`upward_dyadic`), and
refuses if the inputs file's decimal copy disagrees. The regenerated receipt differs from the superseded one in 15
last-digit values (copied endpoints and reference diagnostics). No sector bound and no q changed. The earlier
receipt is kept as `cone_sector_bounds_c31aab8e.SUPERSEDED.json`.

**Deliberate faults**, each caught by a test:
- reversed charge sign;
- the density derivative dropped;
- the e^{2u2−u1} generator term dropped;
- plain complex squares.

Targeting the wrong Helike sector gives c > 3, and the bound refuses.

**Scope.** These are bounds, with status `INTERVAL_ENCLOSURE`. E is the saved centre, not the projected Rayleigh
quotient. There is no separation hypothesis, no Temple bound, no gate verdict and no level identification beyond
the exact charge sector.

**Not built yet:** Temple bounds under the stated separation hypothesis S, and viewer scene 4.

## Status

**Computed, all in double precision on the CPU:**
- sector operators;
- charge blocks;
- gauge slices;
- the edge functions and edges;
- positive-cone Ritz levels of the sourced condensate (interior trial values; no boundary certificate).

**Enclosed (outward-rounded intervals):** for the 16 saved quasimode control states:
- the full-operator residual norms;
- the charge defects;
- the projected-sector residual bound, using the Arb tail and leakage endpoints.

**Not yet built:**
- the viewer's remote stream (WebSocket) and scenes 4–5;
- GPU batched blocks (M2);
- Temple bounds under S, and scene 4 (the rest of M3).

Nothing here is a proof.
