# Yang–Mills in the engine: the SU(2) recovery, the two-ring force and the Cayley flow (spec)

2026-11-19. Written before any engine code for this exists. Branch `feat/ym-engine`.

Sources:

- `docs/research/yang-mills-strategy-2026-11-19.md`, plan step 1: put the SU(2) force and the Cayley flow into
  the engine, and close BW08.
- `docs/RECOVER_CURVATURE_SPEC.md`: the Abelian verb this extends.
- `docs/RICCI_LATTICE_CORRESPONDENCE_SPEC.md` (BW00–BW12).
- *Reading the Curve* (`docs/papers/two-ring-pg/`), §10.

**Scope, stated once.** This is the classical SU(2) lattice force and flow on a 3-manifold: the prerequisite the
strategy note names. Nothing here is a statement about the quantum theory, four dimensions or a mass gap. No
verb, document or response field built from this spec uses those words.

## What exists, and what this spec adds

**Already in the engine (reused, not rewritten).**

| piece | where | what it gives |
| --- | --- | --- |
| native SU(2) cell action and tangent mass (RF2B) | `src/gauge/ricci_simplicial.rs` (`cell_action`, `cell_tangent_mass`, `adjoint`) | the native Whitney force and the mass B, BW00–BW04 |
| Levi-Civita links | `src/gauge/ricci_link.rs` (`levi_civita_link`) | exact initial links for a left-invariant metric |
| metric masses | `src/gauge/ricci_whitney.rs` (`tetra_mass`) | M1/M2 on a radial spherical tetrahedron |
| Abelian two-ring recovery | `src/gauge/curvature_recovery.rs` (`RECOVER_CURVATURE`) | neighbourhoods, register by left multiplication, refusals |
| SU(2) exponential | `src/gauge/lie_exp.rs` (`matrix_exp_su2_q`) | closed-form exp on su(2) |
| group-erased elements and link buffers | `group_element.rs`, `dense_link_buffer.rs`, `su2_gauge_field.rs` | storage |

**Research scripts this spec ports (the references every gate compares against).**

| piece | script | status |
| --- | --- | --- |
| covariant SU(2) recovery | `scripts/ricci_su2_recovery.py` | second order on exact Levi-Civita fields (paper Table 5) |
| two-ring Petrov–Galerkin force | `scripts/ricci_su2_pg_force.py`, batched in `scripts/ricci_su2_batched.py` | BW05 on Berger: 36.25, 7.43, 4.69, 2.51% (paper Table 6); batched equals loop to 6 × 10⁻¹¹ |
| exact defect derivative | `scripts/ricci_su2_batched.py` | `docs/RICCI_SU2_EXACT_DEFECT_DERIVATIVE_SPEC.md` |
| Jacobian K | `scripts/ricci_su2_batched.py kbuild` | B5: all nine parity gates |
| Cayley flow | `scripts/ricci_su2_pg_flow_cayley.py` | S3b: n ≤ 3, receipts in `research/su2-cayley-20261118/` |

## The verbs

Each verb is named by what it returns.

1. **`RECOVER_CURVATURE … GROUP SU2`.** This extends the existing verb. It returns, per cell, the covariant
   closed linear curvature (su(2)-valued, in the cell's chart and frame) and the diagnostics the Abelian verb
   already returns: amplification, condition, rank, faces, cells, residuals. It is the covariant reading: register
   each neighbour onto the cell's sheet, take the logarithm of each face holonomy *after* register, then fit.
   - Default **`GROUP U1`**: the existing behaviour, byte-identical.
2. **`LINK_FORCE cells LINKS links METRIC g [METHOD PG2 | NATIVE]`.** Returns the force covector on every link
   (right-trivialised, three components per edge).
   - **`PG2`:** the two-ring Petrov–Galerkin force.
   - **`NATIVE`:** the RF2B Whitney force, already computable.
   - The response states, in a field, whether the force is the gradient of an energy (`variational`: true for
     NATIVE, **false** for PG2). That fact is part of the result, not a footnote.
3. **`LINK_FLOW cells LINKS links METRIC_FLOW ricci(g0) T t N steps [METHOD …] [STEPPER CAYLEY]`.** Integrates
   B(U, g(t)) ξ′ = −a(U; g(t)) with the S3b Cayley midpoint stepper. It returns:
   - the links at each sample time;
   - per step: Newton iterations and final increment;
   - E(t), D(t) and R(t) when an exact reference is supplied (the Levi-Civita links of the flowed metric).
   - Exact links are a **comparison input**, never used inside the step.

HTTP surfaces mirror GQL, as for `RECOVER_CURVATURE`:
- `POST /v1/bundles/{cells}/recover_curvature` (existing, gains `group`);
- `…/link_force`;
- `…/link_flow`.

**Metric input.** A left-invariant metric is given as three numbers (g₁, g₂, g₃) in the left-invariant frame.
Ricci flow of such a metric is an ODE: the engine integrates it, or accepts g(t) samples. **A missing metric
refuses.** No default metric is ever substituted. That is BW08's own wording.

## The algorithm, as the scripts compute it (the engine must match, not reinterpret)

- **Batched force.** Per cell:
  1. the local links and the six face defects;
  2. the exact defect derivative;
  3. transport along the padded tree layers to the cell's sheet;
  4. the face holonomies;
  5. the fit;
  6. the lift −R(U_g) of reversed edges.

  Then a scatter-add into the edge force. The metric pieces are cached by g, so they are rebuilt only when the
  metric changes.
- **Cayley step** (S3b, with Amendment 1):
  - U_{k+1} = U_k · cay(Ω), where Ω = h ξ(U_k · cay(Ω/2), t + h/2);
  - quasi-Newton with a frozen preconditioner B₀ + (h/2)K₀;
  - stop at a relative increment of 10⁻⁸ (the amended tolerance; the 10⁻¹⁰ attempt failed at n = 1, which the
    receipts keep);
  - N = 8 steps over [0, 0.02] at the levels S3b ran.
- **K₀** is the B5 Jacobian: central differences of the batched force with the exact defect derivative,
  ε = 10⁻⁶.

**[decide] The preconditioner solve.** The engine has dense linear algebra only (nalgebra 0.33); there is no
sparse LU.
- **(a) Dense LU** of B₀ + (h/2)K₀ at n ≤ 3. That is 11,358 unknowns at n = 3: a 1 GB matrix, factorised once per
  run. It runs today. **n = 5 (52,530 unknowns) refuses.**
- **(b) Add a sparse direct solver** (for example the pure-Rust `faer` crate). This is a new dependency and needs
  Bee's approval.
- **(c) Preconditioned GMRES** with a block-Jacobi preconditioner. No new dependency, but the iteration count is
  unknown, so it needs its own convergence gate.

Recommendation: (a) now, for parity at n ≤ 3; (b) when C9 (the n = 5 flow) is scheduled.

## Gates (fixed now)

Every reference number below is a committed receipt. "Parity" means the engine, built from the current tree and
called through its surface, against the research script on the same input.

- **YM1, recovery parity.** `RECOVER_CURVATURE … GROUP SU2` against `ricci_su2_recovery.py`, per cell:
  - coefficients agree to 10⁻¹² relative at n = 1, 2, 3;
  - the paper's Table 5 orders are reproduced to the printed digit (Berger and round, covariant).
  - **Removal:** without register, the error matches the research "no register" row (Berger 27.30, 9.48, 6.04,
    3.18%) to the printed digit.
- **YM2, force parity.** `LINK_FORCE … METHOD PG2` against `ricci_su2_batched.py` (gradient mode):
  - ≤ 10⁻¹² relative at n = 1, 2, 3, both at the initial links and at the S3b n = 2 final links;
  - BW05 relative errors equal Table 6 (36.25, 7.43, 4.69, 2.51%) to the printed digit, **at n = 5 too**. The force
    needs no Newton solve, so n = 5 is reachable today.
  - **NATIVE:** the existing RF2B force against the script's native mode, 10⁻¹².
- **YM3, Jacobian parity.** The engine's K₀ against the B5 matrices `outputs/su2-batched-K/L{0,1,2}_{trial}_K.npz`
  (SHA-256 in `research/su2-batched-K-20261119/`), ≤ 10⁻⁹ relative per matrix, for the forces the flow uses.
- **YM4, flow parity.** `LINK_FLOW` against the S3b receipts in `research/su2-cayley-20261118/` (n = 1, 2, 3;
  native, two-ring and frozen-metric control; N = 8):
  - every sample's E agrees to 10⁻⁸ relative;
  - the Newton iteration counts are identical;
  - the final links agree with the stored `.final.npy` to 10⁻⁸;
  - the frozen-metric control is reproduced as a control, not as a pass.
- **YM5 = BW08, the native path.** The gates above run through the built executable's surfaces: GQL and HTTP, with
  row and value verification.
  - **A missing metric refuses.**
  - A wrong group, an open mesh or a bent vertex refuse as the Abelian verb does.
  - The binary is the build target `gigi-stream`, from the current tree, and its hash goes in the receipt.
- **YM6, mechanism removals (BW10).** Each must turn a gate red with a named failure:
  - drop the register (YM1, YM2);
  - drop the exact defect derivative, i.e. use finite differences (YM3 at its tolerance);
  - flip the Cayley midpoint to an endpoint evaluation (YM4);
  - substitute the round metric for a missing one (YM5 must refuse instead).
- **YM7, the exact spectrum (a check, not a gate, until a later spec fixes one).** The engine's K₀ at n = 1, 2, 3
  reproduces the research spectra's real parts near the exact sector values of the paper's Table 9 (−14/3, −1.088
  and the rest). The comparison uses the same nearest-eigenvalue rule. Reported only.

## Data model

- **Cells:** a bundle with vertices (unit quaternions, scalar first), edges and reverse flags, as
  `RECOVER_CURVATURE`.
- **Links:** an SU(2) gauge-field bundle (`SU2GaugeField`), one unit quaternion per edge, with an explicit
  orientation convention, checked against the cells' reverse flags at load.
- **Flow outputs:** new link bundles per sample time, plus a diagnostics bundle, written through the WAL like any
  other bundle. A flow run is reproducible from its inputs and the binary hash.

## Cost (from receipts, not guessed)

- **The batched force per evaluation in the research code, one process:**

  | n | native | two-ring |
  | --- | --- | --- |
  | 1 | 0.09 s | 0.11 s |
  | 2 | 0.67 s | 0.78 s |
  | 3 | 1.7 s | 2.4 s |

  Two-ring setup is 1.4, 10 and 29 s at n = 1, 2, 3 (`research/su2-batched-20261119/timing.jsonl`). The engine
  target is parity, not speed. Speed is reported and not gated.
- **K₀** at n = 3, two-ring, takes 128 s in the research code (B5).
- **The dense LU** at n = 3 is one factorisation of an 11,358² matrix per run.

## Order of work

1. YM1, the recovery. Smallest, and it reuses the most.
2. YM2, the force, including n = 5 BW05 through the engine.
3. YM3, K₀.
4. YM4, the flow at n ≤ 3, with the dense LU.
5. YM5/YM6 through the built binary; YM7 reported.
6. Then **[decide]** (b) or (c) for n = 5, and only then C9.

Each step gets its tests before its code, and a results document when its gate is judged.

## Guardrails

- **No engine claim before its gate.** Until YM2 passes, the SU(2) numbers stay labelled "research code", as the
  paper says now.
- **Bee's running engine is never stopped.** The engine on :3147 is not killed or restarted by this work. Tests
  use their own ports and data directories.
- **Exact values are comparison inputs only.** They never enter a step.
- **Nothing in a response or a document names the quantum theory, four dimensions or a mass gap** (see scope).

## Not in this spec

- The hybrid three-ring reading for SU(2). It is Abelian-validated only; it moves into the engine after the
  research side validates it for SU(2).
- The variational force (V2, M0).
- The n = 5 flow (C9).
- Other metrics.
- Halcyon on the buckyball. Joining Halcyon is strategy step 3, after this.
