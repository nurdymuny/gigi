# helike-lab: a Rust solver with live GPU visuals (spec)

24 November 2026. **SPEC, before implementation.** I want to watch Helike work as the solver runs: as the floor,
as a probe in the quantum condensate, and as the blade in the two-grain cut. Every frame must be a computed number,
never decoration. The Python code stays the reference. Rust earns each scene by matching it first.

## 1. Goals and non-goals

**Goals:**
- **Speed.** A fast classical solver: every sector, block and edge function of the circle-symmetric family,
  interactively.
- **The quantum probe.** The cone-Ritz solver with the quasimode certificate (see the
  [certification spec](helike-quasimode-certification-spec-2026-11-24.md)), fast enough to run the held registered
  grid on the big machine.
- **A live viewer.** It renders what the solvers stream: the ladder, the flux fan, the edge, Helike's field on the
  three-sphere, and the probe's wavefunctions with their certificate bars.
- **Receipts.** Same discipline as now: every run and every scrubbed frame can be written as a receipt, and replayed.

**Non-goals:**
- No change to the GIGI engine, its endpoints or GQL. This is a separate Cargo project in the repo, not a feature
  of the `gigi` crate.
- No new physics in Rust before parity. A Rust number with no Python oracle is not shown.
- No GPU dense eigensolver on macOS. Metal has no good one, and the CPU (`faer`) is fast enough at these sizes.

## 2. Layout

A separate workspace, `helike-lab/`, on branch `feat/helike-lab`, excluded from the `gigi` build:

| Crate | Job | Key dependencies |
| --- | --- | --- |
| `helike-core` | SU(2) representations, the sector operator (curl, flux, H, G), charge blocks, gauge-orthogonal spectra, ladder closed forms, edge functions | `faer`, `num-complex`, `rayon`; `inari` (intervals) behind a feature |
| `helike-quantum` | Cone chart, Gaussian moments, Laurent model, cone Ritz, quasimode residuals, exact SO(2) charge projection, Temple and Weinstein bounds | `faer`, `rayon`, `inari` |
| `helike-stream` | Frame protocol and receipts: solver to viewer over a local socket or WebSocket, receipt JSON writer and replay | `serde`, `rmp-serde`, `tokio-tungstenite` |
| `helike-view` | `wgpu` renderer (Metal, Vulkan, DX12) and `egui` panels; the scenes of section 5 | `wgpu`, `egui`, `winit`, `glam` |
| `helike-cli` | Headless runners: sweeps, registered grids, receipts. This is what runs on the big machine | `clap` |

## 3. Numerical design

### Classical (`helike-core`)

- **Sector operator.** Build it exactly as `ricci_su3_test2_family.hessian_and_gauge` does: a 9(j+1) Hermitian H and
  the gauge map G. The curl and flux split comes from `helike_curl_spectrum.operators`.
- **Charge blocks** of at most 9 states, from the rows of Lemma 2.3 of *By Enough*. A full sweep diagonalises
  thousands of 9×9 blocks.
  - On the CPU this is `faer` with `rayon` over parameter points.
  - On the GPU it is a batched Jacobi eigensolver in a compute shader, for the dense (t, x, y) maps only. GPU results
    are display-grade. Any claim is recomputed on the CPU.
- **Gauge-orthogonal floor.** Project onto ker G* with an SVD. Reuse the projector across a sweep while the rank is
  stable.
- **Intervals.** `inari` versions of the block bounds, to reproduce the domain certificates as a parity check. This is
  optional and comes later.

### Quantum (`helike-quantum`)

- **The key restructuring: matrix-free.** The Gaussian moment matrices factor over the six normal coordinates. Each
  Laurent term is a product of one-dimensional moment matrices (`one_dimensional_moments` in
  `helike_positive_cone.py`).
  - At Mq = 8, the Python code forms dense 5,005 × 5,005 matrices, about 200 MB each.
  - In Rust, H is applied to a vector as a sum of Kronecker-factored operators on the occupation simplex, so it is
    never assembled. The memory needed drops from gigabytes to megabytes.
  - Mq = 10 or 12 becomes practical. That is the route to the 0.003 convergence rule without a big machine.
- **Eigensolver.** Block Lanczos or LOBPCG on the matrix-free operator, for the lowest ~12 levels in each SO(2)
  charge sector. A dense check at Mq ≤ 6 is kept as an oracle.
- **Certificates.**
  - The full-operator residual ‖Hψ_c‖² comes from direct quadrature in the log chart, including the cutoff shells, as
    the certification spec requires.
  - Charge projection is exact, by the trapezoid rule in the SO(2) angle.
  - Weinstein intervals are always produced. Temple bounds are produced only with the separation hypothesis S passed
    explicitly; the code refuses without it.
- **Precision.** f64 throughout. Interval enclosures of the residual integrals (`inari`) give the
  `PASS_CERTIFIED_INTERVAL` level.

### Parity gates (nothing reaches the viewer before these pass)

| Rust quantity | Python oracle | Tolerance |
| --- | --- | --- |
| Sector H, G at random members | `hessian_and_gauge` | 1e-12 entrywise |
| Flux spectrum, ladder identity, ladder-floor condition | `ladder.json` | 1e-12, same booleans |
| Edge functions and the Table 1 edges | `exact_edge.json` | 1e-10 |
| Positive-cone controls at Mq = 4, 6 | `research/positive-cone-20261124/controls-*` receipts | 1e-9 on gaps and weights |
| Boundary oracle at the probes | `qspace-boundary-20261124` receipt | 1e-9 scaled |

Mutation checks, in the same spirit as the Python ones:
- drop the measure drift;
- flip the cubic sign;
- use a flat metric;
- drop the cutoff commutator;
- use Galerkin H² in place of the true residual.

Each must fail a parity or certificate test.

## 4. Stream and receipts (`helike-stream`)

**Frame contents.**
- The parameters, the solver revision and source hash, the computed arrays (spectra, weights, residuals, field
  samples), and a status label (`COMPUTED`, `PASS_CERTIFIED_FP`, ...).
- Frames are MessagePack. The viewer never computes physics; it only draws frames.

**Receipts.** Any frame or sweep can be written as JSON in the existing receipt style: sources, hashes, platform,
revision and dirty flag. `helike-cli --verify` replays a receipt.

**Remote use.** The solver can run on the big machine with the viewer on the laptop: WebSocket over SSH port
forwarding, with no public endpoint.

## 5. Scenes (`helike-view`)

1. **The ladder and the flux fan, live.** Sliders for t, x, y and j.
   - Rungs with their leak arrows.
   - The nine flux eigenvalues.
   - The two-prices bars.

   Each picture is drawn from a streamed computation, and checked against the closed form in the same frame.
2. **Helike on the three-sphere.**
   - Stereographic projection of S³ through the Hopf fibration, so the long circles become linked Villarceau rings.
     They are drawn as GPU-instanced tubes, stretched by t.
   - Her field f ⊗ (θ¹ + iθ²) ⊗ (τ₁ + iτ₂) is sampled on the GPU: colour by phase, opacity by
     |f|² = cos^{2j}(η/2), and a ribbon showing her winding.
   - The mirror appears as a toggle.
3. **The edge forming.** A live (x, t) map of the floor margin φ_A at fixed y, computed per pixel by the batched
   block solver.
   - The ladder-proved band and the certificate domain are drawn as overlays, as in `fig_coverage`.
   - Clicking a pixel shows that point's spectra.
4. **The probe.**
   - The sourced condensate's levels by charge sector, with Weinstein bars as the trust meter.
   - Wavefunction marginals in the invariants (det G, Helike's charge-2 plane, the competitor's mode).
   - The convergence ladder, Mq = 6, 8, 10, as it runs.
   - The flux-edge crossing (s, g²) appears only when certified, and labelled conditional on S when it is.
5. **The blade.**
   - The two-grain model: left and right helicity sectors on either side, the sweep direction (with W, against W),
     and the leaked orientation-odd energy when one partner is dropped.
   - This scene also tests the orientation conjecture: if the leak is the orientation-odd part, it is drawn and
     measured, not asserted.

## 6. Milestones

| Milestone | Deliverable | Acceptance | Where it runs |
| --- | --- | --- | --- |
| M0 | Workspace, `helike-core` sector operator and blocks, parity tests | Parity table rows 1–2 green; mutation checks caught | Laptop |
| M1 | Viewer with scenes 1–2, stream protocol, receipts | Frames match closed forms; receipts replay | Laptop |
| M2 | Edge sweeps (scene 3), GPU batched blocks for display | Table 1 edges reproduced; CPU recompute of any displayed claim | Laptop |
| M3 | `helike-quantum` matrix-free cone Ritz plus certificates (scene 4) | Positive-cone controls reproduced at Mq 4, 6; Mq 8 convergence receipt; certificate mutants caught | Big machine for Mq ≥ 8 |
| M4 | Registered grid via `helike-cli` (after the method amendment), scene 5 | Registered decision rule unchanged; receipts replay | Big machine |

M3 is where the held work moves: the convergence run and the certificate. The registered grid is released only
through the amendment and gates already written down.

## 7. Risks and open questions

- **The big machine's GPU and OS.** These decide the backend (Vulkan or DX12) and whether an optional CUDA path is
  worth it for batched work. Nothing in M0–M2 depends on the answer.
- **Matrix-free accuracy.** Kronecker application must reproduce the dense matrices bit-closely at Mq ≤ 6 before it is
  trusted at Mq ≥ 8. That is a parity gate, not an assumption.
- **Viewer honesty.** Display-grade GPU numbers must never be quoted. Each scene shows its status label, and
  certified quantities carry their certificate level.
- **Scope creep into GIGI.** There is none by construction, because the workspace is separate. A later bridge (for
  example, storing receipts as GIGI bundles) gets its own spec.
