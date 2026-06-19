# HALCYON Part IV — Implementation Log

**Companion to:** `HALCYON_PART_IV_GATES.md` (verb contracts + locked decisions IV-A through IV-K), `HALCYON_PART_III_IMPLEMENTATION_LOG.md` (Part III receipts + the HTTP-as-consumer-surface reframe inherited here, plus the III.5 `PartIvObservableNotReady` stubs that IV.7 reverses at the SELECT-projection layer).
**Format:** one entry per closed gate (TDD-HAL-IV.N) — gate id, red test path, files edited, green criterion + receipt (the `cargo test` pass line), commit SHA.

The Part IV pass criterion (quoted verbatim from `HALCYON_PART_I_GATES.md` section PART IV):

> **`(U, E)` tangent-space machinery.** E lives in `su(2)` (the Lie algebra), U in `SU(2)` (the group). `exp(dt · E) · U` step requires the exponential map and the right type discipline so the verb math doesn't silently round U off the group manifold.
>
> **Covariant Gauss projector — CG with Tikhonov.** Exposed as `PROJECT_GAUSS { tikhonov: 1e-12, cg_tol: 1e-10, cg_max_iter: 200 }` (Q3).
>
> **Symplecticness verification.** Explicit `H_total` drift bound under a long integration as the receipt that the verb is what it claims. Halcyon's existing tolerance `dH/H_0 < 1e-3` over 1000 steps at dt=0.02, β=2.5 is the reference contract.

The closing entry records:

- **Group erasure preserved on every group-agnostic surface; SU(2)-named concrete escapes are honest.** Per the closing receipt section below: parser (`Statement::EField`, `SymplecticFlow`, `SelectHTotal`, `SelectGaussResidualMax` are group-agnostic), registry shape (sibling registry parallel to Part II/III), and HTTP read routes (`SHOW E_FIELD`, `SELECT H_TOTAL`, `SELECT GAUSS_RESIDUAL_MAX`, `GET /v1/symplectic_flow/diagnostics/:run_id`) stay group-erased. The honest SU(2)-named escapes are: `wilson_force_per_edge` force coefficient `-β/8`, `matrix_exp_su2_q` Rodrigues closed-form, Kennedy-Pendleton (carried forward from III.4, unchanged here), `register_su2_e` / `get_su2_e_mut` sibling registry, and the Kogut-Susskind Hamiltonian wiring `g² = 4/β`. Each is named after its group because the kernel IS group-specific; future U(1) / SU(3) / Z_N ship as siblings, not subtypes.
- **CSPRNG single source of truth.** `marsaglia_haar::SmallRng` (xorshift64*) is the canonical CSPRNG for the whole gauge stack — KP kernel (III.4), heatbath Haar fallback (III.4), MAXWELL_BOLTZMANN E init (IV.1), and any future Part IV stochastic kernels all consume from it. Locked decision 1 from Part II holds through Part IV unchanged.
- **No-feature build byte-identical.** `cargo test --no-default-features --lib` produces `test result: ok. 852 passed; 0 failed`, the same total Parts I-III shipped against. The `gauge` and `halcyon` feature flags remain strictly additive (Bee's locked optionality contract).
- **No `Co-Authored-By: Claude` footer** — every commit in this sprint is authored solely by Bee Rosa Davis (`nurdymuny <bee_davis@alumni.brown.edu>`) per `feedback_no_ai_coauthor.md`.

---

## Entries

### TDD-HAL-IV.1 — `E_FIELD` sibling Lie buffer (SU2EField + sibling registry + MAXWELL_BOLTZMANN init)

- **Red test:** `tests/gauge_e_field_unit.rs` (7 tests covering ZERO/MB/FROM constructors, `q0 = 0` invariant on every mutation, sibling-registry round-trip, `EFieldSourceMismatch` cross-lattice refuse, MB sigma packing match against Halcyon canonical)
- **Files:**
  - `src/gauge/e_field.rs` — `SU2EField` struct: `(n_edges, 4)` quaternion-packed Lie buffer in a flat `Vec<f64>`. Constructors: `new_zero(lat)`, `new_maxwell_boltzmann(lat, beta, seed)`, `new_from(other)`. `write_element_q` enforces `q0 = 0` on every row mutation. Does NOT impl `EdgeConnection` (no group inverse on the Lie algebra).
  - `src/gauge/marsaglia_haar.rs` — added `maxwell_boltzmann_su2` per-edge sampler. Each edge draws exactly 4 uniforms (2 Box-Muller pairs) → 3 standard normals `(g1, g2, g3)`; the fourth normal is consumed-then-discarded so different β values share an identical RNG-state advance per edge (A2 row 1 bit-identity contract preserved). `q0` forced to 0; `q_k = sigma · g_k` for k=1..3 with `sigma = sqrt(1.0 / (β · 1.5))` (Halcyon canonical packing).
  - `src/gauge/registry.rs` — sibling registry: `register_su2_e`, `get_su2_e`, `get_su2_e_mut`, `clear_e_registry`. Parks `Arc<Mutex<SU2EField>>` in a separate map parallel to the existing `register_su2` / `get_su2_mut` for `Arc<Mutex<SU2GaugeField>>`. `test_serial_lock` cfg widened to include `feature = "gauge"` so the integration-test binary can hold the same singleton-serialization mutex used by the III.5 lib tests.
  - `src/gauge/error.rs` — added `EFieldNotDeclared(String)` + `EFieldSourceMismatch { e_lattice, u_lattice }`. Existing variants untouched.
  - `src/gauge/dense_link_buffer.rs` + `src/gauge/mod.rs` — surface wiring.
- **Green criterion (quoted):**
  > `(n_edges, 4)` `q0=0` quaternion-packed Lie buffer with invariant at every constructor entry + every mutation. Matches Halcyon Python torch.float64 (n_edges, 4) representation byte-for-byte.
- **Receipt:**
  ```
  cargo test --features gauge --test gauge_e_field_unit
    test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
  cargo test --features gauge --lib gauge::e_field
    test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 941 filtered out; finished in 0.00s
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.38s
  cargo test --features gauge --lib  (single-thread serialized, full lib)
    test result: ok. 944 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.68s
  ```
  Locked decisions wired: **IV-B** (sibling struct, NOT a field on `SU2GaugeField`), **IV-C** ((n_edges, 4) `q0 = 0` Lie buffer, invariant at every constructor entry + every mutation), CSPRNG single source of truth (MAXWELL_BOLTZMANN consumes `marsaglia_haar::SmallRng`).
- **Commit:** `0e27b75`

### TDD-HAL-IV.2 — Covariant Gauss residual observable

- **Red test:** `tests/gauge_gauss_unit.rs` (6 tests: incidence shape on buckyball, identity-U covariant residual equals flat residual at zero-E, thermalized covariant finiteness, `max_inf_norm` on empty, Ad action identity + norm-preservation lib tests)
- **Files:**
  - `src/lattice/mod.rs` — `Lattice::build_vertex_edge_incidence` opt-in O(E) one-shot helper. Each vertex gets a `Vec<(EdgeId, EdgeOrientation)>`; Forward means vertex is head (b) of edges[i]=(a,b), Reverse means tail (a). Iterates edges in ascending `edge_id` order — load-bearing for A2 row 1 bit-identity in IV.10. NOT stored on `Lattice` (to_gql byte-identity preserved).
  - `src/gauge/gauss.rs` — three pub surfaces + one internal helper: `VertexEdgeIncidence` alias = `Vec<Vec<(EdgeId, EdgeOrientation)>>`; `build_vertex_edge_incidence(&Lattice)` thin re-export over the Lattice method; `compute_gauss_residual_covariant(u, e, lat, inc) -> Result<Vec<[f64;3]>, GaugeFieldError>` group-dispatched on `handle.group()` — SU(2) returns the Ad-sandwich, non-SU(2) returns `UnsupportedGroup`; `compute_gauss_residual_flat(e, lat, inc)` abelian signed sum (baseline + zero-cost U=I cross-check); `max_inf_norm(&[[f64;3]]) -> f64` max-over-vertices of max-over-3-components of `|entry|`. Internal `ad_action_su2` helper performs `U · (0, v) · U^†` with the scalar-first quaternion convention pinned across the gauge crate.
- **Green criterion (quoted):**
  > Group-dispatched on `handle.group()`; SU(2) returns the Ad-sandwich; non-SU(2) returns `UnsupportedGroup`.

  Spec adjustment: spec said thermalized-U covariant-norm < 1.0; the un-projected residual at thermalized U with MB-canonical E is naturally `O(σ · sqrt(deg)) ≈ 2-3` on the buckyball before any PROJECT_GAUSS run. I relaxed the assertion to "finite and < 10.0" (covers `σ · sqrt(3)` tail with headroom; pathological FP paths still trip it). This is consistent with the locked spec's parenthetical "the un-projected residual is bounded but non-zero before projection" — IV.4's PROJECT_GAUSS drives it to `≤ tikhonov ≈ 1e-14`.
- **Receipt:**
  ```
  cargo test --features gauge --test gauge_gauss_unit
    test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.68s
  cargo test --features gauge --lib
    test result: ok. 947 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.74s
  cargo test --features gauge --lib gauge::gauss
    test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 944 filtered out; finished in 0.00s
  ```
  Group-erasure note: `VertexEdgeIncidence` is group-agnostic (lattice-only). `compute_gauss_residual_covariant` dispatches on `u.group()` and `e.group()`; both must be SU2 at launch. Future U(1) E-fields would use abelian divergence (no Ad needed) and would reach for `compute_gauss_residual_flat` directly. III.5 `PartIvObservableNotReady` stub NOT touched here — that reversal lands in IV.7 per spec.
- **Commit:** `cef383f`

### TDD-HAL-IV.3 — `PROJECT_GAUSS` (unpreconditioned CG with Tikhonov)

- **Red test:** `tests/gauge_project_gauss_unit.rs` (6 tests: default config matches Halcyon production, flatten/unflatten round-trip, identity-U CG convergence, thermalized-U CG convergence < 200 iters at β=2.5, byte-identical CG output at fixed seed, post-projection residual ≤ `tikhonov`)
- **Files:**
  - `src/gauge/project_gauss.rs` — `ProjectGaussConfig::default() = { tikhonov: 1e-14, cg_tol: 1e-10, cg_max_iter: 200 }` (the locked-**IV-A** Halcyon production defaults). `project_gauss(u, e, lat, inc, config) -> Result<ProjectGaussReport, GaugeFieldError>` runs unpreconditioned Hestenes-Stiefel CG on `L_cov(U) λ = G_cov(E)`, then writes `E_clean = E_dirty - D_cov(U)^T λ` back into the `SU2EField` buffer with `q0 = 0` re-enforced on every row. Operator helpers (`apply_l_cov_matvec`, `apply_d_cov_transpose_per_edge`, `apply_d_cov_per_vertex`) ship here rather than extending `gauss.rs` — keeps `gauss.rs` as the pure-observable surface (`compute_gauss_residual_covariant` + `max_inf_norm` only) while every CG-internal operator lives in the projector module. III.5 `PartIvObservableNotReady` stub path untouched (IV.7 reverses it).
- **Green criterion (quoted):**
  > Solves `L_cov(U) λ = G_cov(E)` via unpreconditioned Hestenes-Stiefel CG with Tikhonov regularization; writes `E_clean = E_dirty - D_cov(U)^T λ` back into the SU2EField buffer with `q0 = 0` re-enforced.
- **Receipt:**
  ```
  cargo test --features gauge --test gauge_project_gauss_unit
    test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.47s
  cargo test --features gauge --lib gauge::
    test result: ok. 81 passed; 0 failed; 0 ignored; 0 measured; 868 filtered out; finished in 0.08s
  cargo test --features gauge --test gauge_gauss_unit
    test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s  (IV.2 sibling regression-clean)
  ```
  Group-erasure: `project_gauss` returns `UnsupportedGroup` for non-SU(2) U at entry; future SU(3) ships sibling `project_gauss_su3`. Per **IV-E**, NO preconditioner (buckyball `cond(L_cov) ~ 16` needs none; Jacobi is P1). CG converges in ~11 iterations on identity-U and well under 200 on thermalized U at β=2.5 (production-canonical 1e-9 target hit). A2 row 2 byte-identity verified by the `byte_identical_same_seed` test — load-bearing for the IV.10 gold gate.
- **Commit:** `866c4f8`

### TDD-HAL-IV.4 — Wilson force kick (`-β/(2·N²)` coefficient + Lie-projected kick)

- **Red test:** `tests/gauge_wilson_force_unit.rs` (6 tests: identity-link zero force, thermalized finiteness, kick `q0 = 0` boundary, β-coefficient scaling F(5.0) = 2·F(2.5), kick q0=0 invariant under buffer mutation, dt-halving linearity)
- **Files:**
  - `src/gauge/wilson_force.rs` — `wilson_force_per_edge(handle, lat) -> Result<Vec<[f64; 4]>, GaugeFieldError>` reuses III.3's `staple_sum_at_edge` unchanged. Coefficient is `-β/(2·N²) = -β/8` for SU(2) — load-bearing for second-order symplectic accuracy in IV.6's leapfrog driver (Halcyon's bug #3 fix). `project_lie` zeroes q0 inside the kernel. `apply_force_kick(&mut e_field, force, dt)` writes `E[e] += dt · F[e]` through `SU2EField::write_element_q` so the `q0 = 0` invariant is restored at the buffer boundary on every row mutation (defends against FP roundoff in the integrator loop). Group dispatch on `handle.group()` — non-SU(2) returns `UnsupportedGroup`; future SU(3) ships sibling `wilson_force_su3.rs`.
- **Green criterion (quoted):**
  > Coefficient is `-β/(2·N²) = -β/8` for SU(2) — load-bearing for second-order symplectic accuracy in IV.6's leapfrog driver.
- **Receipt:**
  ```
  cargo test --features gauge --test gauge_wilson_force_unit
    test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
  cargo test --features gauge --lib gauge
    test result: ok. 100 passed; 0 failed; 0 ignored; 0 measured; 852 filtered out; finished in 0.10s
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.42s
  ```
  The 6 red-test fns + 3 in-lib unit tests cover identity links, thermalized finiteness, q0=0 boundary, β-coefficient scaling, kick q0=0 invariant, dt-halving linearity. Optionality contract intact (852/0 byte-identical).
- **Commit:** `34db978`

### TDD-HAL-IV.5 — SU(2) Lie-exponential + U full-step drift via `exp(dt · E) · U`

- **Red test:** `tests/gauge_lie_exp_unit.rs` (6 tests: closed-form exp known value, identity-link drift is identity-multiplied, drift left-multiplication, drift preserves unit-quaternion manifold over 1000 chained steps, exp Taylor-fallback at θ < 1e-8, drift renorm bound 2 ULP)
- **Files:**
  - `src/gauge/lie_exp.rs` — `matrix_exp_su2_q(omega: [f64; 4]) -> [f64; 4]` is the closed-form SU(2) Rodrigues exponential with a 4th-order Taylor fallback at `θ < 1e-8`. `drift_step(&mut u_field, e_field, lat, dt)` left-multiplies `U_new[e] = matrix_exp_su2_q(g² · dt · E[e]) · U[e]` with per-edge renormalization (`U_new[e] /= qnorm(U_new[e])`) mirroring Halcyon `buckyball_integrator.py::_drift`.
- **Green criterion (quoted):**
  > `U_new[e] = exp(g² · dt · E[e]) · U[e]` LEFT-multiplied, with per-edge renormalization mirroring Halcyon `buckyball_integrator.py::_drift`.

  Two spec-text adjustments made during implementation:
  1. `tdd_hal_iv_5_exp_known_value`: spec text said `exp([0, π/2, 0, 0]) ≈ [cos(π/4), sin(π/4), 0, 0]` which contradicts the math definition (`θ = sqrt(x²+y²+z²) = π/2` so the answer is `[cos(π/2), sin(π/2), 0, 0] = [0, 1, 0, 0]`). Switched the test input to `[0, π/4, 0, 0]` so the expected `[cos(π/4), sin(π/4), 0, 0]` matches the actual Halcyon math (mirrors `q_validation_su2.py::matrix_exp_su2_q`).
  2. `tdd_hal_iv_5_drift_left_multiplication`: spec asked for byte-identity vs `matrix_exp_su2_q(omega)` after drift; added per-edge renormalization to match Halcyon production (`U_new[e] = U_new[e] / qnorm(U_new[e])` inside `_drift`), so the comparison is now 4 ULP tolerance instead of byte-strict (renorm divides by ~1 + a few ULP).

  Per-edge renormalization is what makes the manifold-preservation receipt achievable — without it, 1000 chained `qmul(exp_q, U)` operations accumulate ~85 ULP drift off the unit sphere; with renorm, `||U_final||² stays within 2 ULP of 1.0`.
- **Receipt:**
  ```
  cargo test --features gauge --test gauge_lie_exp_unit
    test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
  cargo test --features gauge --lib gauge::lie_exp
    test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 952 filtered out; finished in 0.00s
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.56s
  ```
- **Commit:** `91c6365`

### TDD-HAL-IV.6 — `SYMPLECTIC_FLOW` KDK leapfrog + Kogut-Susskind Hamiltonian + per-step projection

- **Red test:** `tests/gauge_symplectic_flow_unit.rs` (6 tests: 5-step smoke + response shape; A2 row 1 in-process reproducibility — byte-identical U + E + measurement chains; hand-rolled KDK-reference equality — any reordering diverges in the f64 floor; per-step projection bounds Gauss residual vs `project_gauss=None` letting it grow; A2 row 6 prefix equality on the HTotal measurement chain — 5-step vs 10-step; Halcyon IV-F (a) acceptance bound `max|ΔH/H_0|<1e-3` over 200 steps)
- **Files:**
  - `src/gauge/symplectic_flow.rs` — main KDK loop. Per step: kick(dt/2) → drift(dt) → kick(dt/2) → project_gauss → measurement epilogue at `(s+1) % measure_every == 0`. The Hamiltonian uses Kogut-Susskind convention with `g² = 4/β`: `K = g² · Σ_e |E_vec[e]|²`, `V = (F / g²) · (1 - ⟨P⟩)`. The naive `(1/2)|E|² + β·F·(1-⟨P⟩)` convention gave 65% drift over 200 steps; Kogut-Susskind gave `< 1e-4`. Group-erasure escape: holds dual `Arc<Mutex<…>>` handles via `get_su2_mut` + `get_su2_e_mut`; epilogue republishes U for dyn-map coherence.
  - `src/gauge/mod.rs` — wire `symplectic_flow`.
- **Green criterion (quoted):**
  > KDK leapfrog with `g² = 4/β` Kogut-Susskind Hamiltonian; per-step PROJECT_GAUSS cadence; A2 row 1 bit-identical at fixed seed; max|ΔH/H_0| < 1e-3 over 200 steps.
- **Receipt:**
  ```
  cargo test --features gauge --test gauge_symplectic_flow_unit
    test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.40s
  cargo test --features gauge --lib gauge::symplectic_flow
    test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 955 filtered out; finished in 0.00s
  cargo test --features gauge --lib gauge:: -- --test-threads=1
    test result: ok. 89 passed; 0 failed; 0 ignored; 0 measured; 868 filtered out; finished in 0.15s
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.21s
  cargo test --features gauge --release --test gauge_symplectic_flow_unit
    test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
  ```
  The IV-J `PartIvObservableNotReady` stub in `gibbs_sample.rs` stays — GIBBS_SAMPLE has no E parameter so the stub is the correct contract there; the E-aware `observe()` dispatch lands in IV.7 where both U and E are in scope. `cg_iterations_per_step_p99` is diagnostic-only and excluded from every A2 row per spec. No persistence op (mirrors III.5 GIBBS_SAMPLE non-persistence).
- **Commit:** `66be31d`

### TDD-HAL-IV.7 — GQL parser + executor arms (E_FIELD, SYMPLECTIC_FLOW, SHOW E_FIELD, SELECT H_TOTAL / GAUSS_RESIDUAL_MAX) + IV-J stub removal at SELECT-projection layer

- **Red test:** `src/parser.rs::tests::tdd_hal_iv_7_*` (8 tests: parse E_FIELD ZERO / MB_SEED / FROM; parse SYMPLECTIC_FLOW full / struct-override / project_false; executor E_FIELD-then-SYMPLECTIC_FLOW; executor SELECT H_TOTAL now works)
- **Files:**
  - `src/parser.rs` — `Statement::EField`, `Statement::SymplecticFlow`, `Statement::ShowEField`, `Statement::SelectHTotal`, `Statement::SelectGaussResidualMax` — all gated `cfg(feature = "gauge")`, group-agnostic per **IV-B**. EBNF implemented exactly per spec (`E_FIELD INIT ZERO | MAXWELL_BOLTZMANN | FROM`; `SYMPLECTIC_FLOW` with `PROJECT_GAUSS TRUE/FALSE/struct sugar`; `SHOW E_FIELD [BUFFER]`; `SELECT H_TOTAL/GAUSS_RESIDUAL_MAX OF (U, E)`). `PROJECT_GAUSS` struct-literal sugar uses new `Token::LBrace/RBrace`; lexer also gained scientific-notation suffix support (load-bearing for `1e-12` spec-default override). `ProjectGaussConfig` gained `PartialEq` so `Statement::SymplecticFlow` can derive `PartialEq` through `Option<ProjectGaussConfig>`.
  - `src/gauge/observables.rs` — `GaussReduction { Covariant | Flat }` added (default `Covariant`).
  - `src/gauge/project_gauss.rs` — `PartialEq` derived on `ProjectGaussConfig`.
  - `src/gauge/mod.rs` — wire new exports.
- **Green criterion (quoted):**
  > Statement::EField / SymplecticFlow / ShowEField / SelectHTotal / SelectGaussResidualMax all parse and dispatch; SELECT H_TOTAL returns a finite f64; IV-J PartIvObservableNotReady stub at SELECT-projection layer REMOVED.
- **Receipt:**
  ```
  cargo test --features gauge --lib parser::tests::tdd_hal_iv_7
    test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 957 filtered out; finished in 0.04s
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.37s
  ```
  IV-J reversal lands at the SELECT-projection layer (`SelectHTotal` executor returns a finite f64 via the explicit Hamiltonian formula); the `gibbs_sample.rs` sweep-time `PartIvObservableNotReady` stub stays intact because GIBBS_SAMPLE has no E field — the error there is honest, and the III.5 test asserting that error remains valid per the spec's "Pick the cleanest path" option.
- **Commit:** `3fc2824`

### TDD-HAL-IV.8 — HTTP routes for read-only verbs (NO route for SYMPLECTIC_FLOW per IV-6; NO route for E_FIELD declare per IV-I)

- **Red test:** `tests/halcyon_part_iv_http.rs` (8 tests: `tdd_hal_iv_8_h_total_get_at_identity_zero_e`, `tdd_hal_iv_8_e_field_get_buffer`, `tdd_hal_iv_8_no_dedicated_symplectic_flow_route`, `tdd_hal_iv_8_no_dedicated_e_field_declare_route`, `tdd_hal_iv_8_diagnostics_get`, `tdd_hal_iv_8_gauss_residual_get_flat_optional`, `tdd_hal_iv_8_gauss_residual_get_covariant`, `tdd_hal_iv_8_diagnostics_404_on_unknown_run_id`)
- **Files:**
  - `src/gauge/http.rs` — four GET routes: `h_total_get`, `gauss_residual_max_get`, `e_field_get`, `symplectic_flow_diagnostics_get`. Process-local LRU diagnostics cache keyed by `run_id`. The two ABSENT POST routes (E_FIELD declare, SYMPLECTIC_FLOW) are load-bearing receipts for locked decisions **IV-I** and IV.6 — both tests assert via 404.
  - `src/gauge/symplectic_flow.rs` — `SymplecticFlowResponse` now carries a `run_id: String` (UUID v4). `symplectic_flow()` populates the diagnostics LRU unconditionally on success.
  - `src/gauge/mod.rs` — wire `get_symplectic_flow_diagnostics(run_id)` for IV.10 consumption and `clear_symplectic_flow_diagnostics_cache()` for test cleanup.
  - `src/parser.rs` — `SymplecticFlow` executor arm exposes `run_id` on the Rows envelope.
- **Green criterion (quoted from `HALCYON_PART_IV_GATES.md` IV-I + IV.6 spec):**
  > Route-table-only enforcement (404 on dedicated `POST /v1/symplectic_flow` and `POST /v1/e_field`). The `/v1/gql` POST endpoint reaches both via `parser::execute` — that is by design.
- **Receipt:**
  ```
  cargo test --features halcyon --test halcyon_part_iv_http
    test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.25s
  cargo build --bin gigi-stream --features halcyon
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.73s
  ```
  H_TOTAL HTTP handler mirrors the executor's SELECT H_TOTAL arm exactly (β taken from the E field's MaxwellBoltzmann init when present, else 1.0). For the IDENTITY+Zero fixture both kinetic and potential are exactly 0.0 (FP64-exact) so the test asserts byte-equality. The diagnostics envelope serializes `measurement_history` keyed by `ObservableId.label()` (`"HTotal"`, `"Energy"`, etc.) — matches the same key shape SYMPLECTIC_FLOW emits in the Rows envelope.
- **Commit:** `1ae6034`

### TDD-HAL-IV.9 — Harvest GIGI canonical reference (`symplectic_flow_canonical.json`)

- **Red test:** `tests/harvest_part_iv_canonical.rs::tdd_hal_iv_9_harvest_canonical` (`#[ignore]` by default; runs under `--ignored --release`)
- **Files:**
  - `tests/harvest_part_iv_canonical.rs` — harvest harness. Runs through the full GQL parser + executor path on `Statement::SymplecticFlow`, exactly mirroring the surface IV.10 will exercise. `INIT IDENTITY` buckyball SU(2) field + `INIT MAXWELL_BOLTZMANN SEED 20260616` E field; `BETA 2.5`; `DT 0.02`; `N_STEPS 1000`; `MEASURE_EVERY 20`; `MEASURE (H_TOTAL, GAUSS_RESIDUAL_MAX, EDGE_KINETIC)`; `PROJECT_GAUSS` at IV-A defaults.
  - `tests/fixtures/halcyon/part_iv/symplectic_flow_canonical.json` — frozen fixture: 50 entries per measurement chain (the `(s+1) % measure_every` convention gives `{20, 40, …, 1000}`). Storage envelope follows III.8a: bits-oracle + decimal-shadow shape, group-tagged with `{"group": "SU(2)", "n_edges": 90, "n_vertices": 60, "n_faces": 32}`.
  - `tests/fixtures/halcyon/part_iv/symplectic_flow_canonical_provenance.json` — provenance side-car. `harvest_commit` pinned to the harvest SHA `60231605cf2239d42a81717bf3c865e08033fb76` by follow-up commit `d5ed8a18b80fadc7a35d3d1ba93d4e22d3fdf9f0` (same II.2 / III.8a pin pattern: harvest runs before its own SHA exists, self-reference fixed in place).
- **Green criterion (quoted from `HALCYON_PART_IV_GATES.md` IV-F (b) regression arm):**
  > Byte-identical match to GIGI-internal harvested fixture under `--release` (same pattern as III.8b).
- **Receipt:**
  ```
  cargo test --features halcyon --test harvest_part_iv_canonical --release -- --ignored --nocapture
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.44s
  cargo test --features halcyon --test harvest_part_iv_canonical
    test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s
  cargo test --features halcyon --lib -- --test-threads=1
    test result: ok. 965 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.53s
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
  ```
  Harvest receipts under `--release`: `max_energy_drift_rel = 6.900151596789e-5` (well under the 1e-3 Halcyon acceptance bound for IV.10 gate (a)); `gauss_residual_max_final = 2.220446049250e-16` (well under the 1e-9 per-step-projection bound); `cg_iterations_per_step_p99 = 64` (**DIAGNOSTIC ONLY** — excluded from every A2 row).

  Chain length deviation from the spec sketch: the spec named "length 51" for each measurement chain. The SYMPLECTIC_FLOW executor triggers the measurement epilogue when `(s+1) % measure_every == 0`, so `N_STEPS=1000 / MEASURE_EVERY=20 → step indices {20, 40, ..., 1000} = exactly 50 entries`, not 51. The fixture and the IV.10 gold gate both agree on 50; this is the same `(s+1)`-based convention III.6 / III.8a already use. The harness `assert_eq!`s the actual length so a future executor cadence change would not silently mis-shape the fixture.

  q0=0 IV-C invariant: the harness asserts `q0 == 0.0` on every one of the 90 Lie rows of the final E buffer BEFORE serializing. A drift here aborts the harvest before the bad fixture lands. Acceptance gating: harness asserts `max_energy_drift_rel < 1e-3` and `gauss_residual_max < 1e-9` BEFORE writing. A drifted run aborts without overwriting the previous fixture.
- **Commits:** `60231605cf2239d42a81717bf3c865e08033fb76` (harvest harness + fixtures) and `d5ed8a18b80fadc7a35d3d1ba93d4e22d3fdf9f0` (provenance harvest_commit SHA pin)

### TDD-HAL-IV.10 — Gold gate (Halcyon Gate IV contract + A2 matrix verdicts)

- **Red test:** `tests/halcyon_part_iv_gold.rs` (5 tests: `tdd_hal_iv_10_a_symplectic_flow_canonical`, `tdd_hal_iv_10_b_energy_drift_two_tier`, `tdd_hal_iv_10_c_gauss_residual_two_tier`, `tdd_hal_iv_10_d_h_total_now_returns`, `tdd_hal_iv_10_e_diagnostics_envelope_shape`) + `tests/halcyon_part_iv_a2_matrix.rs` (6 tests, one per A2 row)
- **Files:**
  - `tests/halcyon_part_iv_gold.rs` — five integration tests mapping onto Halcyon mock Gates IV.A-E. Per **IV-F**: gate_iv_a (full byte-equality vs IV.9 fixture) is debug-ignored, release-required; gate_iv_b / gate_iv_c each run their acceptance arm in debug AND their regression arm only in release (via inline `cfg(not(debug_assertions))` block inside test body, NOT outer `#[cfg_attr]` — this matches the spec's green criterion of "4 PASS / 1 ignored in debug"). gate_iv_d asserts `SELECT H_TOTAL` now returns a finite f64 (IV-J reversal receipt). gate_iv_e asserts the diagnostics envelope shape and that `cg_iterations_per_step_p99` is PRESENT but never byte-compared.
  - `tests/halcyon_part_iv_a2_matrix.rs` — six integration tests, one per A2 row. **Row 1** (same-process strict) under `--release`. **Row 2** (cross-process strict) uses the IV.9 on-disk fixture as the "different process" reference rather than spawning a child binary at runtime — the fixture WAS frozen by a separate process invocation under `--release`, so this satisfies the cross-process same-OS contract without runtime spawn complexity. **Row 3** (cross-OS) is a permanent `#[ignore]` with documented 2 ULP envelope per **IV-G**. **Row 4** (different β) and **Row 5** (different dt) assert divergence + tolerance survival in debug. **Row 6** (different n_steps) tests prefix equality at MEASURE_EVERY=20 cadence (first 5 entries of the 200-step run match the 100-step run byte-for-byte) under `--release`.
- **Green criterion (quoted from `HALCYON_PART_IV_GATES.md` IV-F two-tier):**
  > (a) acceptance: `max|ΔH/H_0| < 1e-3` (Halcyon bound) — debug-safe; (b) regression: byte-identical match to GIGI-internal harvested fixture under `--release`.

  A2 matrix verdicts: Row 1 PASS, Row 2 PASS, Row 3 SKIP (cross-OS, documented 2 ULP envelope, not enforced), Row 4 PASS, Row 5 PASS, Row 6 PASS.
- **Receipt:**
  ```
  cargo test --features halcyon --test halcyon_part_iv_gold --release
    test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.43s
  cargo test --features halcyon --test halcyon_part_iv_a2_matrix --release
    test result: ok. 5 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 1.02s
  cargo test --features halcyon --test halcyon_part_iv_gold  (debug)
    test result: ok. 4 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 14.46s
  cargo test --features halcyon --test halcyon_part_iv_a2_matrix  (debug)
    test result: ok. 2 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 3.38s
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.12s
  ```
  Group-erasure note: every test is SU(2)-only as the spec asks; sibling future-group flows (SU(3), U(1)) will ship parallel A2 matrix files. The optionality contract is preserved at 852/0 byte-identical no-default-features.
- **Commit:** `f1561de9bd5e52e897fa419efc5eda8e2261ce5f`

### TDD-HAL-IV.11 — Implementation log + Part IV gates doc

- **Files:**
  - `theory/halcyon/HALCYON_PART_IV_GATES.md` — verb contracts (`E_FIELD`, `PROJECT_GAUSS`, `SYMPLECTIC_FLOW`, `SHOW E_FIELD`, `SELECT H_TOTAL`, `SELECT GAUSS_RESIDUAL_MAX`), locked decisions IV-A through IV-K with one-line explanations each, A2 bit-identity matrix (6 rows), decided/deferred split, cross-binding bit-identity disposition, future-audit anchor pointing to II.6c reframe (`d5d3853`) and `HALCYON_PART_III_GATES.md`.
  - `theory/halcyon/HALCYON_PART_IV_IMPLEMENTATION_LOG.md` — this artifact.

---

## Closing receipts

- **No-feature build byte-identical.** `cargo test --no-default-features --lib` produces:
  ```
  test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured
  ```
  The number matches the Part I baseline; no test was added, removed, or shifted into the default surface. The `gauge` and `halcyon` feature flags are strictly additive (Bee's locked optionality contract).
- **Halcyon feature lib test count post-Part-IV.** `cargo test --features halcyon --lib -- --test-threads=1` produces 965 passing in the last verified release before IV.10 landed; the IV.10 tests live in `tests/` and don't shift the lib total. Lib-side total: 965 (Part III baseline 941 + Part IV gate contributions IV.1 (+3) → 944, IV.2 (+3) → 947, IV.3 (+2) → 949, IV.4 (+3) → 952, IV.5 (+3) → 955, IV.6 (+2) → 957, IV.7 (+8) → 965).
- **Halcyon Part IV gold integration test green (under `--release`).**
  ```
  cargo test --features halcyon --test halcyon_part_iv_gold --release
  test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured
  ```
- **Halcyon Part IV A2 matrix integration test green (under `--release`).**
  ```
  cargo test --features halcyon --test halcyon_part_iv_a2_matrix --release
  test result: ok. 5 passed; 0 failed; 1 ignored; 0 measured
  ```
  Row 3 (cross-OS) is the permanent `#[ignore]` per **IV-G** — documented 2 ULP envelope, not enforced in single-OS CI.
- **ChEMBL-incident durability gates green** (no Part IV change touched the durability surface; confirmed):
  ```
  test engine::tests::snapshot_survives_wal_compact ... ok
  test engine::tests::streaming_wal_replay_correct_count ... ok
  test engine::tests::streaming_snapshot_roundtrip ... ok
  test engine::tests::cow_snapshot_roundtrip ... ok
  test engine::tests::mmap_rebase_snapshot_roundtrip ... ok
  test engine::tests::test_9_8_trigger_survives_restart ... ok
  ```
- **No `Co-Authored-By: Claude` footer in any commit.** Every commit in this sprint (`0e27b75`, `cef383f`, `866c4f8`, `34db978`, `91c6365`, `66be31d`, `3fc2824`, `1ae6034`, `60231605cf2239d42a81717bf3c865e08033fb76`, `d5ed8a18b80fadc7a35d3d1ba93d4e22d3fdf9f0`, `f1561de9bd5e52e897fa419efc5eda8e2261ce5f`, plus this doc commit) is authored solely by `nurdymuny <bee_davis@alumni.brown.edu>` (Bee Rosa Davis) per the `feedback_no_ai_coauthor.md` standing memo.

---

## Group erasure receipt

Per-gate enumeration of where SU(2) is hardcoded vs where the surface stays group-agnostic:

| Gate | SU(2)-hardcoded | Group-agnostic |
| --- | --- | --- |
| **IV.1 E_FIELD** | `SU2EField` struct, `maxwell_boltzmann_su2` per-edge sampler (sigma packing), `register_su2_e` / `get_su2_e_mut` sibling registry | `Statement::EField` parser surface (group-tagged via the underlying U field), error variants `EFieldNotDeclared` / `EFieldSourceMismatch` |
| **IV.2 Gauss residual** | `ad_action_su2` Ad-sandwich helper (scalar-first quaternion convention) | `VertexEdgeIncidence` (lattice-only), `compute_gauss_residual_covariant` group-dispatched on `handle.group()`, `compute_gauss_residual_flat` abelian baseline (group-agnostic by construction), `max_inf_norm` (group-agnostic) |
| **IV.3 PROJECT_GAUSS** | CG operators (`apply_l_cov_matvec`, `apply_d_cov_transpose_per_edge`, `apply_d_cov_per_vertex`) all assume SU(2) Ad action; future SU(3) ships sibling `project_gauss_su3` | `ProjectGaussConfig` struct + parser sugar |
| **IV.4 Wilson force** | Coefficient `-β/(2·N²) = -β/8` (the SU(2) N² = 4 substitution), `project_lie` quaternion-q0 zeroing | `wilson_force_per_edge` dispatches on `handle.group()`; future SU(3) ships sibling `wilson_force_su3.rs` |
| **IV.5 Lie-exp + drift** | `matrix_exp_su2_q` Rodrigues closed-form (the SU(2) Lie group is the only group with such a clean closed form; SU(3) needs full matrix-exp), per-edge renormalization assumes unit-quaternion manifold | `drift_step` dispatches on `handle.group()` |
| **IV.6 SYMPLECTIC_FLOW** | Kogut-Susskind `g² = 4/β` (the SU(2) N=2 substitution), `Σ_e |E_vec[e]|²` kinetic shape assumes 3-component Lie algebra, holds dual `Arc<Mutex<…>>` handles via the SU(2)-named registry escapes | Per-step KDK loop structure is group-agnostic, measurement epilogue cadence is group-agnostic, `run_id` UUID is group-agnostic |
| **IV.7 Parser + executor** | Executor arm dispatches on `handle.group()` and calls the SU(2) kernels; non-SU(2) returns `UnsupportedGroup` at the dispatch boundary before mutation | `Statement::EField`, `SymplecticFlow`, `ShowEField`, `SelectHTotal`, `SelectGaussResidualMax` are all group-agnostic in the parser; the executor does the SU(2) check at the dispatch arms |
| **IV.8 HTTP** | Handlers take `&dyn GaugeFieldHandle` through `registry::get`; group-aware dispatch lives inside the IV.1 / IV.6 library functions | All four GET routes (`h_total_get`, `gauss_residual_max_get`, `e_field_get`, `symplectic_flow_diagnostics_get`) are group-agnostic; route-table absence of POST mutators is group-agnostic enforcement |
| **IV.9 Harvest** | Fixture is group-tagged with `{"group": "SU(2)", "n_edges": 90, "n_vertices": 60, "n_faces": 32}` | Fixture envelope shape is group-agnostic — future SU(3) ships parallel files |
| **IV.10 Gold gate + A2 matrix** | Every test is SU(2)-only as the spec asks | Test scaffold is group-agnostic — sibling future-group flows ship parallel `halcyon_part_iv_a2_matrix_su3.rs` etc. |

The honest break is at the kernel layer (Wilson force coefficient, Lie-exp closed form, Kennedy-Pendleton from III.4, Ad-sandwich, Kogut-Susskind `g²`). Each is named after its group because the kernel IS group-specific; future U(1) / SU(3) / Z_N ship as siblings, not subtypes. Same architectural shape Part III shipped (`get_su2_mut` concrete escape).

---

## Gates closed in this commit chain

Confirmed against `git log --oneline -- src/gauge tests/halcyon_part_iv_* tests/harvest_part_iv_*`:

| Commit | Gate | Name |
| --- | --- | --- |
| `0e27b75` | TDD-HAL-IV.1 | E_FIELD sibling Lie buffer (SU2EField + sibling registry + MAXWELL_BOLTZMANN init) |
| `cef383f` | TDD-HAL-IV.2 | Covariant Gauss residual observable |
| `866c4f8` | TDD-HAL-IV.3 | PROJECT_GAUSS (unpreconditioned CG with Tikhonov) |
| `34db978` | TDD-HAL-IV.4 | Wilson force kick (`-β/(2·N²)` coefficient + Lie-projected kick) |
| `91c6365` | TDD-HAL-IV.5 | SU(2) Lie-exponential + U full-step drift via `exp(dt · E) · U` |
| `66be31d` | TDD-HAL-IV.6 | SYMPLECTIC_FLOW KDK leapfrog + Kogut-Susskind Hamiltonian + per-step projection |
| `3fc2824` | TDD-HAL-IV.7 | GQL parser + executor arms + IV-J stub removal at SELECT-projection layer |
| `1ae6034` | TDD-HAL-IV.8 | HTTP routes for read-only verbs (NO route for SYMPLECTIC_FLOW per IV.6; NO route for E_FIELD declare per IV-I) |
| `6023160` | TDD-HAL-IV.9 (harvest) | Harvest GIGI canonical reference (`symplectic_flow_canonical.json`) |
| `d5ed8a1` | TDD-HAL-IV.9 (pin) | Provenance harvest_commit SHA pin |
| `f1561de` | TDD-HAL-IV.10 | Gold gate (Halcyon Gate IV contract + A2 matrix verdicts) |
| _this commit_ | TDD-HAL-IV.11 | Implementation log + Part IV gates doc |

All eleven gates closed. The Part IV pass criterion is satisfied: the canonical 90-edge buckyball SU(2) `SYMPLECTIC_FLOW` at β=2.5, dt=0.02, N_STEPS=1000, SEED=20260616 reproduces the GIGI-internal harvested fixture byte-for-byte under `--release` (the **IV-F (b)** regression arm), and satisfies `max|ΔH/H_0| < 1e-3` AND `gauss_residual_max < 1e-9` AND CG p99 finite (the **IV-F (a)** acceptance arm, debug-safe). A2 matrix verdicts: Row 1 PASS, Row 2 PASS, Row 3 SKIP, Row 4 PASS, Row 5 PASS, Row 6 PASS.

---

## A2 matrix verdicts

| Row | Gate | Verdict |
| --- | --- | --- |
| Row 1 — Same process | `tdd_hal_iv_10_a2_row_1_same_process_strict` (release) | **PASS** |
| Row 2 — Cross-process, same OS | `tdd_hal_iv_10_a2_row_2_cross_process_same_os_strict` (release) | **PASS** |
| Row 3 — Cross-OS | `tdd_hal_iv_10_a2_row_3_cross_os_2ulp` | **SKIP** (per **IV-G**; documented 2 ULP envelope, not enforced in single-OS CI) |
| Row 4 — Different β | `tdd_hal_iv_10_a2_row_4_different_beta_no_byte_id` | **PASS** |
| Row 5 — Different dt | `tdd_hal_iv_10_a2_row_5_different_dt_no_byte_id` | **PASS** |
| Row 6 — Different n_steps | `tdd_hal_iv_10_a2_row_6_different_n_steps_prefix_equality` (release) | **PASS** |

`cg_iterations_per_step_p99` is DIAGNOSTIC ONLY and never compared in any row.

---

## What is deferred / what is not deferred

### Not deferred (decided)

The locked decisions IV-A through IV-K are not TODO items; they are the canonical shape of the Part IV surface. Mirroring `HALCYON_PART_IV_GATES.md`:

- **SYMPLECTIC_FLOW is embedded-only — no HTTP route.** Same reasoning as Part III's `GIBBS_SAMPLE`. Route table absence at IV.8 is the enforcement.
- **E_FIELD declarer is embedded-only — no HTTP POST.** Per **IV-I**.
- **Sequential KDK per step.** Standard second-order leapfrog (kick-drift-kick).
- **No CG preconditioner.** Per **IV-E**.
- **Two-tier energy drift gate.** Per **IV-F**.
- **Tikhonov default 1e-14.** Per **IV-A**.
- **Sibling `SU2EField` struct + `(n_edges, 4)` `q0 = 0` quaternion buffer.** Per **IV-B / IV-C**.

### Deferred (still TODO)

- **HMC with Metropolis acceptance.** Not on the substrate.
- **`BLOCKED_SEM` as a substrate verb.** Stays as post-processing.
- **`MIGDAL_WITTEN`.** Out of Part IV scope.
- **`HAAR_RANDOM_GAUGE_TRANSFORM`.** Out of Part IV scope.
- **`ENSEMBLE_FROM_TRAJECTORY` / `KNN_LOO` / `LABEL_PERMUTATION_NULL` / `FEATURE_ABLATION`.** Probably don't belong in GIGI.
- **GPU.** Not in any current sprint. Future P3.
- **Multi-group beyond SU(2).** Future-group work flips on the typed `GaugeFieldError::UnsupportedGroup(Group)` error variant.
- **Non-Wilson actions.** Part IV ships the Wilson force only.
- **External validator.** Per **IV-K**, Halcyon team authors `test_gigi_part_iv_symplectic_flow.py` after IV.10 lands.
- **Cross-OS CI matrix.** Per **IV-G**.
- **Configurable projection cadence.** Per **IV-D**.
- **Jacobi preconditioner.** Per **IV-E**.
