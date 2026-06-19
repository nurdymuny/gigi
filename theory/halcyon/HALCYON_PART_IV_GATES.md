# HALCYON Part IV — Gates

**Companion to:** `HALCYON_PART_I_GATES.md` section PART IV, `HALCYON_TO_GIGI_REPLY_2026-06-17.md` section A1 (Q_SURROGATE math), A2 (bit-identity contract for SYMPLECTIC_FLOW), Q5 (verb naming), Q3 (PROJECT_GAUSS struct knob shape), and `HALCYON_PART_III_GATES.md` (Part III/IV continuity, especially **D5** HTTP-as-consumer-surface reframe and **D4** registry mutability concrete escape).
**Voice:** first-person, mine (Bee). Sober register. I spec the algorithm here, not the prose around it.

This document fixes the verb contracts and the locked decisions I made on 2026-06-18 going into the Part IV sprint. The receipts (per-gate red/green, commit SHAs, test counts) live in `HALCYON_PART_IV_IMPLEMENTATION_LOG.md`.

---

## Part IV pass criterion

Quoted verbatim from `HALCYON_PART_I_GATES.md` section PART IV:

> **`(U, E)` tangent-space machinery.** E lives in `su(2)` (the Lie algebra), U in `SU(2)` (the group). `exp(dt · E) · U` step requires the exponential map and the right type discipline so the verb math doesn't silently round U off the group manifold.
>
> **Covariant Gauss projector — CG with Tikhonov.** Exposed as `PROJECT_GAUSS { tikhonov: 1e-12, cg_tol: 1e-10, cg_max_iter: 200 }` (Q3). The verb that decides whether the integrator preserves the constraint surface to machine precision or just close to it. Hard-coded defaults will fail in some β regime; the exposed knobs are necessary, not nice-to-have.
>
> **Symplecticness verification.** Explicit `H_total` drift bound under a long integration as the receipt that the verb is what it claims. Halcyon's existing tolerance `dH/H_0 < 1e-3` over 1000 steps at dt=0.02, β=2.5 is the reference contract.
>
> Bit-identity contract per `HALCYON_TO_GIGI_REPLY_2026-06-17.md § A2`. CG iteration count varies across β at fixed seed; that's a diagnostic, not a regression.

Operationalized as: the canonical 90-edge buckyball SU(2) `SYMPLECTIC_FLOW` at β=2.5, dt=0.02, N_STEPS=1000, SEED=20260616, MEASURE_EVERY=20, PROJECT_GAUSS at default, reproduces the GIGI-internal harvested fixture at `tests/fixtures/halcyon/part_iv/symplectic_flow_canonical.json` byte-for-byte under `--release` (the **IV-F (b)** regression arm), and satisfies `max|ΔH/H_0| < 1e-3` AND `gauss_residual_max < 1e-9` AND CG p99 finite (the **IV-F (a)** acceptance arm, debug-safe).

---

## Per-verb specs

### `E_FIELD name FROM ... { INIT ZERO | MAXWELL_BOLTZMANN [BETA β] [SEED int] | FROM e_other }` (Gate IV.1, parser-lift Gate IV.7)

Declares a sibling Lie-algebra buffer parked next to a `GAUGE_FIELD`. Per **IV-B / IV-C**, `SU2EField` is a sibling struct (NOT a field on `SU2GaugeField`), backed by a `(n_edges, 4)` quaternion-packed Lie buffer with the `q0 = 0` invariant enforced at every constructor entry point AND on every buffer mutation. Mirrors Halcyon Python `torch.float64 (n_edges, 4)` byte-for-byte.

Three init shapes:

- `INIT ZERO` → all rows zero. `q0 = 0` trivially holds.
- `INIT MAXWELL_BOLTZMANN [BETA β] [SEED int]` → per-edge Gaussian draws via `marsaglia_haar::maxwell_boltzmann_su2` (CSPRNG single source of truth: `marsaglia_haar::SmallRng` xorshift64*). Sigma is the Halcyon canonical packing `sigma = sqrt(1.0 / (β · 1.5))`. Each edge draws exactly 4 uniforms (2 Box-Muller pairs) → 3 standard normals `(g1, g2, g3)`; the fourth normal is consumed-then-discarded so different β values share an identical RNG-state advance per edge (the **A2 row 1** bit-identity contract is preserved). `q0` forced to 0; `q_k = sigma · g_k` for k=1..3.
- `INIT FROM e_other` → deep-copy via `registry::get_su2_e_mut(other)`. Cross-lattice mismatch surfaces as `GaugeFieldError::EFieldSourceMismatch { e_lattice, u_lattice }` before any state mutates.

Registry surface (sibling-parallel to Part II's `register_su2` / `get_su2_mut`): `register_su2_e`, `get_su2_e`, `get_su2_e_mut`, `clear_e_registry`. `SU2EField` does NOT impl `EdgeConnection` (no group inverse on the Lie algebra).

**HTTP routing:** declarer is embedded-only. No `POST /v1/e_field` route. Per **IV-I**.

### `PROJECT_GAUSS { tikhonov: f64, cg_tol: f64, cg_max_iter: u32 }` (Gate IV.3, struct sugar lifted at Gate IV.7)

Covariant Gauss projector. Solves `L_cov(U) λ = G_cov(E)` via unpreconditioned Hestenes-Stiefel CG with Tikhonov regularization, then writes `E_clean = E_dirty - D_cov(U)^T λ` back into the `SU2EField` buffer with `q0 = 0` re-enforced on every row.

Per **IV-A**, defaults are `{ tikhonov: 1e-14, cg_tol: 1e-10, cg_max_iter: 200 }` (matches Halcyon Python production). The spec-default `1e-12` lands behind explicit struct sugar `PROJECT_GAUSS { tikhonov: 1e-12 }` per Q3.

Per **IV-E**, preconditioner is NONE. The buckyball `cond(L_cov) ~ 16` needs no preconditioner. Jacobi preconditioner is P1 future-tense.

Internal operators live in `src/gauge/project_gauss.rs` (not extending `gauss.rs`): `apply_l_cov_matvec`, `apply_d_cov_transpose_per_edge`, `apply_d_cov_per_vertex`. Keeps `gauss.rs` as the pure-observable surface (`compute_gauss_residual_covariant` + `max_inf_norm` only).

**HTTP routing:** not an HTTP verb on its own. Lives inside `SYMPLECTIC_FLOW`'s per-step projection cadence (per **IV-D**).

### `SYMPLECTIC_FLOW field ON lattice BETA β DT dt N_STEPS n [MEASURE_EVERY k] [PROJECT_GAUSS { ... } | TRUE | FALSE] [MEASURE (...)] [SEED int]` (Gates IV.4 → IV.6, parser-lift Gate IV.7)

Second-order Kick-Drift-Kick (KDK) leapfrog integrator on `(U, E)`. Live at launch for SU(2) only; non-SU(2) groups error with `UnsupportedGroup(g)` at the executor boundary before mutation begins.

Per-step KDK path:

1. **Kick (IV.4):** `wilson_force_per_edge(handle, lat) -> Vec<[f64; 4]>` computes `F[e] = (-β / (2·N^2)) · staple_sum_at_edge(handle, lat, e)` projected to Lie via `project_lie` (q0 forced to 0). Coefficient `-β/(2·N²) = -β/8` for SU(2) is load-bearing for second-order symplectic accuracy. `apply_force_kick(&mut e_field, force, dt)` writes `E[e] += dt · F[e]` with `q0 = 0` re-enforced through `SU2EField::write_element_q`.
2. **Drift (IV.5):** `matrix_exp_su2_q(omega) -> [f64; 4]` is the closed-form SU(2) Rodrigues exponential with a 4th-order Taylor fallback at `θ < 1e-8`. `drift_step(&mut u_field, e_field, lat, dt)` left-multiplies `U_new[e] = matrix_exp_su2_q(g² · dt · E[e]) · U[e]` with per-edge renormalization (`U_new[e] /= qnorm(U_new[e])`) mirroring Halcyon `buckyball_integrator.py::_drift`. Renorm keeps 1000 chained `qmul(exp_q, U)` operations within ~2 ULP of the unit sphere; without it, ~85 ULP drift accumulates.
3. **Kick (IV.4 repeat):** second half-step of the KDK pattern.
4. **Project (IV.3, optional but default-on):** `project_gauss` runs unpreconditioned CG with Tikhonov on the post-kick `E` field. Per **IV-D**, cadence is per leapfrog step (mirror Halcyon production-canonical). Configurable cadence is P1 future-tense; no cadence knob ships at launch.
5. **Measure epilogue:** if `(s + 1) % measure_every == 0`, apply each requested `ObservableId` to `(U, E)`. The `(s + 1)`-based convention matches III.6 / III.8a; a 1000-step run at MEASURE_EVERY=20 yields exactly 50 entries per chain (not 51).

Hamiltonian wiring uses Kogut-Susskind convention with `g² = 4/β`:

- Kinetic: `K = g² · Σ_e |E_vec[e]|²` where `E_vec[e]` is the imaginary part of the quaternion (q0 already zero).
- Potential: `V = (F / g²) · (1 - ⟨P⟩)` where `F = n_faces`, `⟨P⟩ = plaquette_mean`.
- `H_total = K + V`.

The naive `(1/2)|E|² + β · F · (1 - ⟨P⟩)` convention gives 65% drift over 200 steps; the Kogut-Susskind convention gives `< 1e-4`. Load-bearing for the IV-F gate.

Measurement battery at launch:

- `H_TOTAL` — scalar f64 per measurement.
- `GAUSS_RESIDUAL_MAX` — scalar f64 per measurement (with `GaussReduction::Covariant` default; `Flat` available for the abelian baseline).
- `EDGE_KINETIC` — scalar f64 per measurement (K alone).
- `VERTEX_GAUSS` — vector f64 per measurement (the full `(n_vertices, 3)` Ad-sandwich array flattened).
- `ENERGY` — alias for `H_TOTAL` per spec.

Wire shape: GQL response is a single `Rows` row with `{field, e_field, seed, beta, dt, n_steps_completed, run_id, <obs.label()>: Vector<f64> per measured observable, diagnostics: { cg_iterations_per_step_p99 }}`.

Per-run diagnostics envelope (the `run_id` is a UUID v4 created at executor entry; populates a process-local LRU cache for the HTTP `GET /v1/symplectic_flow/diagnostics/:run_id` route):

- `measurement_history` keyed by `ObservableId.label()` (`"HTotal"`, `"Energy"`, `"GaussResidualMax"`, `"EdgeKinetic"`, `"VertexGauss"`)
- `cg_iterations_per_step_p99` — **DIAGNOSTIC ONLY**, never compared in any A2 row.
- `max_energy_drift_rel` — the IV-F (a) acceptance bound's input.
- `gauss_residual_max_final` — the per-step-projection bound's input.

**HTTP routing:** declarer is embedded-only. No `POST /v1/symplectic_flow` route. Per **IV-6 spec + IV-I**.

### `SHOW E_FIELD name [BUFFER]` (Gate IV.7, HTTP at IV.8)

Read-only introspection of a declared `SU2EField`. Inherits Part II's locked decision 4 (HTTP-safe for read-only verbs).

Wire shape (HTTP): `GET /v1/e_field/{name}` returns the metadata envelope; `GET /v1/e_field/{name}?buffer=true` returns the full `(n_edges, 4)` Lie buffer.

### `SELECT H_TOTAL OF (U, E)` (Gate IV.7, HTTP at IV.8)

Pure-observable scalar f64. Hamiltonian computed via the Kogut-Susskind formula above. β is taken from the E field's MaxwellBoltzmann init when present, else 1.0.

The **IV-J** anchor: Part III's `PartIvObservableNotReady` stub for `H_TOTAL` at the SELECT-projection layer is REMOVED in IV.7. III.5's GIBBS_SAMPLE sweep-time stub stays intact because GIBBS_SAMPLE has no E parameter — the error there is honest and the III.5 test asserting that error remains valid. The IV-J removal lands only at the SELECT-projection layer where both U and E are in scope.

Wire shape (HTTP): `GET /v1/select/h_total?u=<name>&e=<name>` returns `{u, e, value: f64}`.

### `SELECT GAUSS_RESIDUAL_MAX OF (U, E)` (Gate IV.7, HTTP at IV.8)

Pure-observable scalar f64. Computes `max_inf_norm(compute_gauss_residual_covariant(u, e, lat, incidence))` by default. Optional `WITH FLAT` keyword switches to `compute_gauss_residual_flat` for the abelian baseline cross-check.

Wire shape (HTTP): `GET /v1/select/gauss_residual_max?u=<name>&e=<name>&reduction=covariant|flat` returns `{u, e, reduction, value: f64}`.

---

## Locked decisions inherited from Bee 2026-06-18

Each holds through every Part IV gate. None get relitigated mid-sprint.

- **IV-A (PROJECT_GAUSS defaults).** `tikhonov = 1e-14` (matches Halcyon Python production). `cg_tol = 1e-10`. `cg_max_iter = 200`. The spec-default `1e-12` accessible only via explicit `PROJECT_GAUSS { tikhonov: 1e-12 }` struct sugar.
- **IV-B (Sibling struct).** `SU2EField` is a sibling struct (NOT a field on `SU2GaugeField`). Does NOT impl `EdgeConnection`. Sibling registry: `register_su2_e` + `get_su2_e_mut` parallel to existing `register_su2` + `get_su2_mut`.
- **IV-C (Lie buffer shape).** `(n_edges, 4)` `q0 = 0` quaternion-packed Lie buffer. Matches Halcyon Python `torch.float64 (n_edges, 4)` byte-for-byte. `q0 = 0` invariant enforced at every constructor entry point AND on every buffer mutation.
- **IV-D (Projection cadence).** Per leapfrog step (mirror Halcyon production-canonical). No cadence knob at launch. Configurable cadence is P1 future-tense.
- **IV-E (CG preconditioner).** NONE (mirror Halcyon production exactly; buckyball `cond(L_cov) ~ 16` needs no preconditioner). Jacobi preconditioner is P1.
- **IV-F (Energy drift gold-gate).** Two-tier in IV.10: (a) acceptance `max|ΔH/H_0| < 1e-3` (Halcyon bound) — debug-safe; (b) regression byte-identical match to GIGI-internal harvested fixture under `--release` — same pattern as III.8b.
- **IV-G (A2 cross-OS row).** Skipped in single-OS test runs via `#[cfg_attr(...)]` gate; same-OS rows are hard byte-equality gates. Documented 2 ULP cross-OS envelope in this doc but NOT enforced in CI.
- **IV-H (HTTP for read-only verbs).** Inherit Part II locked decision 4. `SHOW E_FIELD`, `SELECT H_TOTAL OF (U, E)`, `SELECT GAUSS_RESIDUAL_MAX OF (U, E)`, `GET /v1/symplectic_flow/diagnostics/:run_id` all HTTP-safe (lands in IV.8).
- **IV-I (E_FIELD declarer is embedded-only).** NO `POST /v1/e_field` route. Rationale: `MAXWELL_BOLTZMANN` couples CSPRNG + project_gauss + U state in an awkward HTTP shape. Conservative call matching `SYMPLECTIC_FLOW` embedded-only.
- **IV-J (Part III stub removed in IV.7).** Part III's `PartIvObservableNotReady` stub for `HTotal` / `GaussResidualMax` / `EdgeKinetic` / `VertexGauss` / `Energy` is REMOVED at the SELECT-projection layer in IV.7. The III.5 hooks are already in place; Part IV replaces error returns with real computations against the new E buffer. Positive case for `SELECT H_TOTAL` lands in IV.10 gold gate.
- **IV-K (Sprint sequencing).** GIGI ships IV.10 first (defines contract). Halcyon team authors `test_gigi_part_iv_symplectic_flow.py` + extends `mock.py` after live binding lands. NOT in this Part IV plan.

---

## A2 bit-identity matrix

Per `HALCYON_TO_GIGI_REPLY_2026-06-17.md § A2`. `cg_iterations_per_step_p99` is a **DIAGNOSTIC ONLY** and is NOT compared in any row.

| Row | Contract | GIGI-side gate |
| --- | --- | --- |
| **Row 1 — Same process** | Same seed/β/dt/n_steps, SAME process → STRICT byte-identical on `(U_i, E_i, H_total, gauss_residual_max)` at every step. | `tdd_hal_iv_10_a2_row_1_same_process_strict` under `--release`. HARD GATE. |
| **Row 2 — Cross-process, same OS** | Same seed/β/dt/n_steps, DIFFERENT process, same OS, same BLAS → STRICT byte-identical. | `tdd_hal_iv_10_a2_row_2_cross_process_same_os_strict` under `--release` (replays against the IV.9 frozen fixture, which was harvested by a separate `--release` process invocation). HARD GATE. |
| **Row 3 — Cross-OS** | Same seed/β/dt/n_steps, CROSS-OS → 2 ULP tolerance in trig reductions. Documented, not enforced. | `tdd_hal_iv_10_a2_row_3_cross_os_2ulp` marked `#[ignore]` permanently with documented 2 ULP envelope. SKIP in single-OS CI per **IV-G**. |
| **Row 4 — Different β** | Same seed, DIFFERENT β → NOT bit-identical; tolerances (energy drift, Gauss residual) hold. | `tdd_hal_iv_10_a2_row_4_different_beta_no_byte_id`. NO byte gate — asserts measurement chains DIVERGE byte-wise but each independently satisfies `max|ΔH/H_0| < 1e-3` and `gauss_residual_max < 1e-9`. |
| **Row 5 — Different dt** | Same seed, DIFFERENT dt → NOT bit-identical; tolerances reapply. | `tdd_hal_iv_10_a2_row_5_different_dt_no_byte_id`. NO byte gate — asserts divergence + tolerance survival. |
| **Row 6 — Different n_steps** | Same seed, DIFFERENT n_steps → prefix-equality: first `min(n1, n2)` steps byte-identical. | `tdd_hal_iv_10_a2_row_6_different_n_steps_prefix_equality` under `--release`. HARD GATE on prefix-equality at MEASURE_EVERY=20 cadence (first 5 entries of the 200-step run match the 100-step run byte-for-byte). |

---

## Cross-binding bit-identity disposition

Same as Part III. GIGI uses xorshift64* (`marsaglia_haar::SmallRng`, the canonical CSPRNG for the entire gauge stack — KP kernel, heatbath Haar fallback, MAXWELL_BOLTZMANN E init, and any future Part IV stochastic kernels all consume from it). Halcyon's mock uses NumPy PCG64. In-process bit-identity holds within GIGI at fixed seed. Cross-binding bit-identity is impossible by design and is NOT the contract.

The contract has two clauses (mirroring Part III's structure):

1. **Intra-GIGI bit-identity at fixed seed.** Same binary, same seed → byte-identical `(U_i, E_i)` and measurement chains over an arbitrary number of steps. Pinned by **Row 1** and by the IV.9 → IV.10 fixture replay (canonical 1000-step run reproduces the IV.9 fixture byte-for-byte via `f64::to_bits()`).
2. **Cross-process bit-identity within same OS, same BLAS.** Pinned by **Row 2** via fixture replay (the fixture WAS frozen by a separate `--release` process invocation in IV.9).

Cross-OS drift (Row 3) is documented but not enforced. The 2 ULP envelope covers trig reductions (`matrix_exp_su2_q`'s `sin`/`cos` and the SU(2) Rodrigues path's `acos` clamp) where x87 vs SSE2 vs the Windows MSVC C runtime can differ by 1-2 ULPs on the same f64 input.

---

## What is decided / what is not

### Decided (not deferred)

These are not TODO items; they are the canonical shape of the Part IV surface.

- **SYMPLECTIC_FLOW is embedded-only — no HTTP route.** Same reasoning as Part III's `GIBBS_SAMPLE` (mutation verb, production hot path, long wall on the 1000-step canonical run). Route table absence at IV.8 is the enforcement.
- **E_FIELD declarer is embedded-only — no HTTP POST.** Per **IV-I**. `MAXWELL_BOLTZMANN` couples CSPRNG + project_gauss + U state in an awkward HTTP shape; conservative call matching `SYMPLECTIC_FLOW` embedded-only.
- **Sequential KDK per step.** Standard second-order leapfrog (kick-drift-kick) at every step. Symplectic by construction.
- **No CG preconditioner.** Per **IV-E**. Buckyball `cond(L_cov) ~ 16` needs none.
- **Two-tier energy drift gate.** Per **IV-F**. Acceptance arm (`max|ΔH/H_0| < 1e-3`) is debug-safe; regression arm (byte-identical to IV.9 fixture) is release-only via `cfg(not(debug_assertions))` inline guards in the gold-gate test.
- **Tikhonov default 1e-14.** Per **IV-A**. Matches Halcyon Python production.
- **Sibling `SU2EField` struct + `(n_edges, 4)` `q0 = 0` quaternion buffer.** Per **IV-B / IV-C**. Does NOT impl `EdgeConnection`.

### Deferred

- **HMC with Metropolis acceptance.** Not on the substrate. The original spec's ask correctly flagged that this shouldn't be conflated with `SYMPLECTIC_FLOW`. Deferred indefinitely.
- **`BLOCKED_SEM` as a substrate verb.** Stays as post-processing. Same disposition as Part III.
- **`MIGDAL_WITTEN`.** Out of Part IV scope. Original spec P3 ask.
- **`HAAR_RANDOM_GAUGE_TRANSFORM`.** Out of Part IV scope.
- **`ENSEMBLE_FROM_TRAJECTORY` / `KNN_LOO` / `LABEL_PERMUTATION_NULL` / `FEATURE_ABLATION`.** Probably don't belong in GIGI. Stay external (per Part I gates).
- **GPU.** Not in any current sprint. Future P3.
- **Multi-group beyond SU(2).** Part IV ships the SU(2) symplectic flow only. Group-agnostic surfaces (parser, registry shape, HTTP read routes) are already in place; future SU(3) / U(1) / Z_N ship as sibling kernels alongside `wilson_force_per_edge` and `matrix_exp_su2_q`.
- **Non-Wilson actions.** Part IV ships the Wilson force (`F = -β/(2·N²) · staple_sum`). Improved actions (Symanzik, Iwasaki) are future-tense.
- **External validator.** Halcyon team authors `test_gigi_part_iv_symplectic_flow.py` after IV.10 lands. Per **IV-K**.
- **Cross-OS CI matrix.** Per **IV-G**. Documented 2 ULP envelope; CI stays single-OS.
- **Configurable projection cadence.** Per **IV-D**. P1 future-tense.
- **Jacobi preconditioner.** Per **IV-E**. P1 future-tense.

---

## Future-audit anchor

Why is `SYMPLECTIC_FLOW` missing from HTTP routes? Why is `E_FIELD` declare missing from HTTP POST? Why doesn't `/v1/gql` gate mutation?

See `HALCYON_PART_II_IMPLEMENTATION_LOG.md` section "HTTP-as-consumer-surface (architectural framing)" and the II.6c reframe (commit `d5d3853`), plus `HALCYON_PART_III_GATES.md` (the Part III/IV continuity carrier). The summary: embedded GQL (PyO3 / CFFI) is the canonical declarer and mutator surface for any consumer crossing a restart boundary or running heavy mutation verbs. HTTP is the consumer-facing introspect channel (`GET`) plus declare-ephemeral-by-default (`POST` lattice / gauge_field) for the mock-to-live swap. `SYMPLECTIC_FLOW` and `E_FIELD` declare are pre-committed embedded-only. The route table is the enforcement surface; the soft-edge through `/v1/gql` is by design and the long production wall self-enforces.

Any auditor reading the HTTP routes and noticing `SYMPLECTIC_FLOW`'s absence (or `E_FIELD` declarer absence) should land here.
