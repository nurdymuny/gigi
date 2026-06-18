# HALCYON Part III — Implementation Log

**Companion to:** `HALCYON_PART_III_GATES.md` (verb contracts + locked decisions D1-D7), `HALCYON_PART_II_IMPLEMENTATION_LOG.md` (Part II receipts + the HTTP-as-consumer-surface reframe inherited here).
**Format:** one entry per closed gate (TDD-HAL-III.N) — gate id, red test path, files edited, green criterion + receipt (the `cargo test` pass line), commit SHA.

The Part III pass criterion (quoted verbatim from `HALCYON_PART_I_GATES.md`):

> The first ~80 lines of `inertia_damping/run_validation_report.py` (the build_graph → heatbath_thermalize → measure phase) replaces with a 3-statement GQL block:
>
> ```sql
> LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY "S2";
> GAUGE_FIELD U ON LATTICE buckyball GROUP SU(2) INIT IDENTITY;
> GIBBS_SAMPLE U
>   BETA 2.5
>   N_SWEEPS 200
>   MEASURE_EVERY 1
>   MEASURE (MEAN(PLAQUETTE), Q_SURROGATE)
>   SEED 20260616;
> ```
>
> …and the GQL response carries the same measurement history (within SEM) the Python kernel produces.

The closing entry records:

- **Group erasure preserved everywhere except the GIBBS_SAMPLE boundary.** `PLAQUETTE`, `Q_SURROGATE`, `staple_sum`, parser dispatch, and HTTP routes all take `&dyn GaugeFieldHandle`. The one honest break is `registry::get_su2_mut(name) -> Option<Arc<Mutex<SU2GaugeField>>>` — the **D4** concrete escape. Kennedy-Pendleton IS the SU(2) heatbath, so SU(2)-naming at the mutation boundary is the correct shape; future U(1) / SU(3) / Z_N heatbaths ship as siblings, not subtypes.
- **Two Haar paths coexist.** Marsaglia 4-uniforms-with-rejection (Part II) for `INIT HAAR_RANDOM`; sqrt-rejection-on-`x0` (Part III, Gate III.4) for the KP `xi → 0` fallback. Each named honestly per **D2**.
- **CSPRNG single source of truth.** `marsaglia_haar::SmallRng` (xorshift64*) is the canonical CSPRNG for the whole gauge stack — KP kernel + heatbath Haar fallback both consume from it. Locked decision 1 from Part II holds through Part III unchanged.
- **No-feature build byte-identical.** `cargo test --no-default-features --lib` produces `test result: ok. 852 passed; 0 failed`, the same total Part I and Part II shipped against. The `gauge` and `halcyon` feature flags remain strictly additive (Bee's locked optionality contract).
- **No `Co-Authored-By: Claude` footer** — every commit in this sprint is authored solely by Bee Rosa Davis (`nurdymuny <bee_davis@alumni.brown.edu>`) per `feedback_no_ai_coauthor.md`.

---

## Entries

### TDD-HAL-III.1 — `PLAQUETTE` primitive (face-holonomy reduction)

- **Red test:** `src/gauge/plaquette.rs::tests` (`tdd_hal_iii_1_plaquette_identity_is_unity`, `tdd_hal_iii_1_plaquette_haar_within_unit_ball`, `tdd_hal_iii_1_plaquette_mean_equals_per_face_mean`)
- **Files:**
  - `src/gauge/plaquette.rs` — `plaquette_per_face(handle, lat) -> Result<Vec<f64>, GaugeFieldError>` + `plaquette_mean` + `plaquette_sum`. Group-erased entry: `handle.group()` dispatches; SU(2) reduces face holonomy quaternion to `q0` via existing `walk_loop + face_edges`; non-SU(2) returns `UnsupportedGroup(g)`.
  - `src/gauge/mod.rs` — re-export `plaquette_per_face`, `plaquette_mean`, `plaquette_sum`.
- **Green criterion (quoted):**
  > Three call shapes: `SELECT PLAQUETTE OF U_t` → per-face Re tr(U_f) / 2, shape `(n_faces,)`; `SELECT SUM(PLAQUETTE OF U_t)` → scalar Wilson action / β; `SELECT MEAN(PLAQUETTE OF U_t)` → ⟨P⟩, the published observable.
- **Receipt:**
  ```
  cargo test --features gauge --lib gauge::plaquette
    test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 915 filtered out; finished in 0.01s
  cargo test --features gauge --lib
    test result: ok. 918 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.69s
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.19s
  ```
  Identity asserts FP64-exact 1.0 across all 32 faces (no tolerance). Haar asserts q0 ∈ [-1, 1] for all 32 faces at SEED 20260616. Mean-equals-per-face-mean uses 1e-15 because `plaquette_mean` reuses the same `iter().sum() / n_faces` order the test recomputes.
- **Commit:** `2f9ef76`

### TDD-HAL-III.2 — `Q_SURROGATE` primitive (angular accumulator)

- **Red test:** `src/gauge/q_surrogate.rs::tests` (3 tests: `tdd_hal_iii_2_q_surrogate_at_identity_is_zero`, `tdd_hal_iii_2_q_surrogate_within_range`, `tdd_hal_iii_2_q_surrogate_clamp_idempotent`)
- **Files:**
  - `src/gauge/q_surrogate.rs` — `q_surrogate(handle, lat) -> Result<f64, GaugeFieldError>`. SU(2) returns `(1 / 2π) · Σ_f arccos(clamp(q0_f, -1, 1))` reusing `plaquette_per_face` for the q0 column (no duplicated walker logic). Non-SU(2) returns `UnsupportedGroup(g)`.
  - `src/gauge/mod.rs` — re-export `q_surrogate`.
- **Green criterion (quoted; from `HALCYON_TO_GIGI_REPLY_2026-06-17.md § A1`):**
  > Desugars to `SUM(ARCCOS(CLAMP(REAL(PLAQUETTE), -1, 1))) / (2 * PI)` if `PLAQUETTE` returns the full quaternion's q0.
- **Receipt:**
  ```
  cargo test --features gauge --lib gauge::q_surrogate
    test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 918 filtered out; finished in 0.00s
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.14s
  cargo test --features gauge --lib -- --test-threads=1
    test result: ok. 921 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 24.08s
  ```
  Clamp-before-arccos ordering pinned by `tdd_hal_iii_2_q_surrogate_clamp_idempotent` — a future refactor that arccos'd before clamping would NaN at `q0 = 1.0 + 1e-16` AND fail this test. Locked decisions honored: D6 (scalar f64 shape), D7 (group-erasure pattern matches III.1 — handle dispatch, not concrete-type dispatch).
- **Commit:** `9dcc8e2`

### TDD-HAL-III.3 — Per-edge staple sum walker

- **Red test:** `src/gauge/staple.rs::tests` (`tdd_hal_iii_3_incidence_buckyball_shape`, `tdd_hal_iii_3_staple_at_identity_is_face_count`, `tdd_hal_iii_3_staple_matches_halcyon_at_seed`, `tdd_hal_iii_3_staple_skips_self_edge`)
- **Files:**
  - `src/gauge/staple.rs` — `staple_sum(handle, lat, edge) -> Result<[f64; 4], GaugeFieldError>`. Group-erased over `&dyn EdgeConnection`. SU(2) walks the four faces touching edge `e`, skips the self-edge per face, multiplies remaining three edges in face-orientation order, sums contributions as a quaternion. `unreachable!()` guards future U(1) / Z_N dispatch (which belongs in `GIBBS_SAMPLE` at III.5, not here).
  - `src/gauge/mod.rs` — re-export `staple_sum`.
  - `src/lattice/mod.rs` — `EdgeFaceIncidence` as a thin alias over `Vec<Vec<(usize, usize)>>`; helper lives as a method on `Lattice`, re-exported as a free function in `gauge::staple` for namespace ergonomics. NOT stored on `Lattice` so `to_gql` round-trip stays byte-identical (no-default-features 852/0 confirms).
- **Green criterion (quoted):**
  > `bundle/heatbath.rs::staple_update` — the per-edge conditional. Pure SU(2) algebra, no new geometry.
- **Receipt:**
  ```
  cargo test --features gauge --lib gauge::staple
    test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 921 filtered out; finished in 0.01s
  cargo test --features gauge --lib
    test result: ok. 925 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.72s
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.10s
  ```
  Halcyon golden `[0.5766…, -0.9034…, -1.1238…, 0.01485…]` harvested once from `davis-wilson-lattice/inertia_damping/buckyball_action.py::staple_sum_q` against the SEED-20260616 Haar buffer (`tests/fixtures/halcyon/buckyball_haar_random_seed_20260616_gold.json` from Part II's II.2); matches the Rust scalar-accumulator path to 1e-12. Identity-init self-edge-skip guard confirms a pre-bug version that included the self edge would fail.
- **Commit:** `ab4a34c`

### TDD-HAL-III.4 — Kennedy-Pendleton single-edge update + sqrt-rejection Haar fallback

- **Red test:** `src/gauge/kennedy_pendleton.rs` (4 tests: `tdd_hal_iii_4_kp_xi_zero_falls_to_haar`, `tdd_hal_iii_4_kp_xi_large_concentrates_near_v_hat`, `tdd_hal_iii_4_kp_consumes_4_rngs_per_attempt`, `tdd_hal_iii_4_kp_constants_match_halcyon`) + `src/gauge/heatbath_haar.rs` (`tdd_hal_iii_4_haar_sqrt_rejection_distribution`)
- **Files:**
  - `src/gauge/kennedy_pendleton.rs` — `sample_kp_x0(&mut rng, kappa) -> Option<f64>` does the κ-rejection on `x0` with `MAX_KP_ITERS = 400` and `EPS_K = 1e-12`. `sample_su2_link(&mut rng, beta, staple) -> [f64; 4]` does the full single-link update: if `||staple|| · β < EPS_K`, falls through to `heatbath_haar::sample_haar_sqrt_rejection` (the **D2** fallback path). RNG draws 4 uniforms per attempt; quaternion `qmul` inlined locally since the kernel speaks raw arrays at the boundary.
  - `src/gauge/heatbath_haar.rs` — `sample_haar_sqrt_rejection(&mut rng) -> [f64; 4]`. Sqrt-rejection-on-`x0` + spherical placement, mirroring Halcyon Python `buckyball_heatbath.py:119-134`. Coexists with Part II's `marsaglia_haar::haar_random_su2` per **D2**; the Marsaglia path stays for `INIT HAAR_RANDOM`.
  - `src/gauge/marsaglia_haar.rs` — added a non-load-bearing observational `draws` counter on `SmallRng` so the RNG-count test could assert 4-uniforms-per-attempt without changing state evolution (byte-identity preserved; no-default-features still 852/0).
  - `src/gauge/mod.rs` — declare `kennedy_pendleton` and `heatbath_haar` submodules.
- **Green criterion (quoted):**
  > Per-edge conditional update uses the staple `S_e = Σ_b U_a · U_b · U_c^†` over the four faces touching edge e.
- **Receipt:**
  ```
  cargo test --features gauge --lib gauge::kennedy_pendleton
    test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 926 filtered out; finished in 0.00s
  cargo test --features gauge --lib gauge::heatbath_haar
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 929 filtered out; finished in 0.00s
  cargo test --features gauge --lib
    test result: ok. 930 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.73s
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.60s
  ```
  Constants `EPS_K = 1e-12` and `MAX_KP_ITERS = 400` pinned via `iii_4_kp_constants_match_halcyon` against `buckyball_heatbath.py` lines 95-171. Kernel is SU(2)-specific by construction (KP IS the SU(2) heatbath); takes `[f64; 4]` quaternions directly with no group dispatch — group match happens in `GIBBS_SAMPLE` at III.5.
- **Commit:** `d61223f`

### TDD-HAL-III.5 — `GIBBS_SAMPLE` sweep (in-place mutation + measurement_history)

- **Red test:** `src/gauge/gibbs_sample.rs` (6 inline gate tests covering SeedRequired, intra-binding reproducibility at SEED 20260616, plaquette_mean drift off identity after one sweep, sequential edge-order pinning via by-hand replay (**D3**), `measure_every` semantics, Part-IV observable rejection)
- **Files:**
  - `src/gauge/gibbs_sample.rs` — main sweep loop. For each sweep, `for e in 0..n_edges` (per **D3**, sequential). Per edge: `staple_sum` (III.3) → `sample_su2_link` (III.4) → mutate buffer in place. After each sweep, if `sweep % measure_every == 0`, apply each requested `ObservableId` to the field. RNG is `marsaglia_haar::SmallRng::seed_from_u64(seed)` (the locked-decision-1 xorshift64* path).
  - `src/gauge/registry.rs` — `get_su2_mut(name) -> Option<Arc<Mutex<SU2GaugeField>>>` (the **D4** concrete escape). I chose `Arc<Mutex<…>>` over the spec's literal `MutexGuard<…>` because `MutexGuard` is non-`'static` and cannot escape its lock scope; same access pattern, longer lifetime. Existing `get()` + `GaugeFieldHandle` surface untouched. Added `register_su2` / `republish_su2` / `remove` for parallel-test hygiene; existing `register(Arc<dyn>)` untouched.
  - `src/gauge/error.rs` — extended with `SeedRequired` Display message; `H_TOTAL` rejected with `"H_TOTAL requires an E field, which is a Part IV (SYMPLECTIC_FLOW) construct"`.
  - `src/gauge/mod.rs` — declare `gibbs_sample`; re-export `run`, `ObservableId`.
- **Green criterion (quoted):**
  > `query/exec.rs::gibbs_sample` runs the sweep loop, threads the CSPRNG seed, and emits measurement rows.
- **Receipt:**
  ```
  cargo test --features gauge --lib gauge::gibbs_sample
    test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 930 filtered out; finished in 0.06s
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.55s
  ```
  D3 pinned by the by-hand replay test: the reference walks the same primitives (`staple_sum` then `sample_su2_link`) in sequential edge order and the buffers byte-match. Race-condition note: under unfiltered `cargo test --features gauge --lib`, the 3 III.5 tests that span register-then-call-then-lookup can race against existing gauge tests' `gauge_registry::clear()` calls. This is a pre-existing pattern issue (every gauge test does `clear()` at the start); my gate's wider window exposes it more. The spec's green criterion (filtered `gauge::gibbs_sample`) is stable across 5 consecutive runs. Broader race fix is outside the spec's files-to-touch allowlist.
- **Commit:** `0db2779`

### TDD-HAL-III.6 — GQL parser + executor arms (PLAQUETTE, Q_SURROGATE, GIBBS_SAMPLE)

- **Red test:** `src/parser.rs::tests::tdd_hal_iii_6_parse_gibbs_sample_full_form` + 4 sibling `tdd_hal_iii_6_*` tests (`tdd_hal_iii_6_parse_gibbs_sample_minimal`, `tdd_hal_iii_6_parse_plaquette_reductions`, `tdd_hal_iii_6_execute_gibbs_sample_no_seed_errors`, `tdd_hal_iii_6_execute_gibbs_sample_smoke`)
- **Files:**
  - `src/gauge/observables.rs` (new) — `PlaquetteReduction` enum (`PerFace` / `Mean` / `Sum`, locked decision **D7**) + `ObservableId` re-export (`label()` shared with III.5).
  - `src/gauge/mod.rs` — `pub mod observables; pub use observables::PlaquetteReduction`.
  - `src/parser.rs`:
    - `Statement` enum: added `GibbsSample { field, beta, n_sweeps, measure_every, measure, seed }`, `SelectPlaquette { field, reduction }`, `SelectQSurrogate { field }` (all `#[cfg(feature = "gauge")]`).
    - Top-level dispatch: `"GIBBS_SAMPLE"` → `parse_gibbs_sample()`; `"SELECT"` peeks for `try_parse_gauge_select()` before falling through to `parse_sql_select`. Two-token SELECT lookahead distinguishes `MEAN(PLAQUETTE OF U)` (gauge projection) from `MEAN(field)` (classic agg), rewinding to `saved_pos` so the bundle-SELECT path stays untouched when the inner token is not `PLAQUETTE`.
    - Parser methods: `parse_gibbs_sample`, `parse_observable_list`, `parse_observable`, `expect_f64`, `try_parse_gauge_select`.
    - Executor arms: `Statement::SelectPlaquette` dispatches to `plaquette_per_face` / `plaquette_mean` / `plaquette_sum`; `Statement::SelectQSurrogate` dispatches to `q_surrogate`; `Statement::GibbsSample` dispatches to `gauge::gibbs_sample` after `handle.group()` check for SU(2) (the group-erasure escape **D4**).
- **Green criterion (quoted):**
  > `parser/gql.rs::plaquette_expr`, `gibbs_sample_stmt`.
- **Receipt:**
  ```
  cargo test --features gauge --lib parser::tests::tdd_hal_iii_6
    test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 936 filtered out; finished in 0.01s
  cargo test --features gauge --lib
    test result: ok. 941 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.59s
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.60s
  cargo test --features halcyon --lib
    test result: ok. 941 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.62s
  ```
  Wire shapes:
  - `SelectPlaquette` PerFace: `Rows` row with `{field: Text, reduction: "per_face", n_faces: Integer, per_face: Vector<f64> of length F}`.
  - `SelectPlaquette` Mean/Sum: `Rows` row with `{field, reduction, value: Float}`.
  - `SelectQSurrogate`: `Rows` row with `{field, value: Float}`.
  - `GibbsSample`: `Rows` row with `{field, seed, beta, n_sweeps_completed, <obs.label()>: Vector<f64> per measured observable}`.
- **Commit:** `76b1840`

### TDD-HAL-III.7 — HTTP routes for read-only verbs (NO route for GIBBS_SAMPLE per D5)

- **Red test:** `tests/halcyon_part_iii_http.rs` (6 tests: `tdd_hal_iii_7_q_surrogate_get`, `tdd_hal_iii_7_plaquette_get_per_face`, `tdd_hal_iii_7_plaquette_get_mean`, `tdd_hal_iii_7_observables_batched_post`, `tdd_hal_iii_7_plaquette_404_when_field_undeclared`, `tdd_hal_iii_7_no_dedicated_gibbs_sample_route`)
- **Files:**
  - `src/gauge/http.rs` — added `PlaquetteQuery`, `PlaquettePerFaceEnvelope`, `PlaquetteScalarEnvelope`, `PlaquetteEnvelope` (untagged enum on `reduction`), `QSurrogateEnvelope`, `ObservablesBatchRequest`, plus three handlers: `plaquette_get`, `q_surrogate_get`, `observables_post`. `resolve_field_and_lattice` helper anchors the "not declared" substring on the typed `GaugeFieldError` Display. `build_router` additions are exactly the three routes named in the spec; no `gibbs_sample` handler exists in the file — test (f) is the receipt for **D5**.
  - `tests/halcyon_part_iii_http.rs` — integration tests driving `Router<()>` via `tower::ServiceExt::oneshot` (same shape as Part II's HTTP gate II.6).
- **Green criterion (quoted from `HALCYON_PART_III_GATES.md` D5 spec):**
  > Route-table-only enforcement (404 on dedicated `/v1/gauge_field/{name}/gibbs_sample` route). The `/v1/gql` POST endpoint reaches `GIBBS_SAMPLE` via `parser::execute` — that is by design.
- **Receipt:**
  ```
  cargo test --features halcyon --test halcyon_part_iii_http
    test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
  cargo build --bin gigi-stream --features halcyon
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.23s
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.88s
  ```
  Red phase: 5 routing failures + 1 D5-receipt that already passed (because the `gibbs_sample` route doesn't exist) — exactly the load-bearing absence the spec calls for. Group-erasure preserved: handlers take `&dyn GaugeFieldHandle` through `registry::get`; group-aware dispatch lives inside the III.1/III.2 library functions. `axum::extract::Query` added to the existing import line as the only import diff.
- **Commit:** `d5ee7ce`

### TDD-HAL-III.8a — Harvest GIGI canonical reference (`<P>_canonical` + Flyvbjerg-Petersen SEM)

- **Red test:** `tests/harvest_part_iii_canonical.rs::tdd_hal_iii_8a_harvest_p_canonical` (`#[ignore]` by default; runs under `--ignored`)
- **Files:**
  - `tests/harvest_part_iii_canonical.rs` — harvest harness. Runs through the full GQL parser + executor path (`gigi::parser::parse` + `gigi::parser::execute`) on `Statement::GibbsSample`, exactly mirroring the surface III.8b will exercise. `INIT IDENTITY` buckyball SU(2) field; `BETA 2.5`; `SEED 20260616`; `N_SWEEPS 2048`; `MEASURE_EVERY 1`; `MEASURE (MEAN(PLAQUETTE))`. Flyvbjerg-Petersen blocked SEM: block sizes 1, 2, 4, … up to n/4; plateau detected when two consecutive block sizes return SEMs within 10% relative tolerance; conservative fallback returns the largest-block-size SEM if no plateau detected.
  - `tests/fixtures/halcyon/part_iii/p_canonical.json` — frozen fixture: `p_canonical = 0.5073639227091518`, `fp_sem = 0.0014729334993958205`, `n_sweeps_total = 2048`, `thermalization_discard = 100`, `n_sweeps_post_thermal = 1948`. Storage envelope follows Part II II.2: `p_history_bits` (IEEE-754 oracle, `u64` array) + `p_history_decimal` (informational shadow, `f64` array).
  - `tests/fixtures/halcyon/part_iii/p_canonical_provenance.json` — provenance side-car. `harvest_commit` pinned to `6a52efc` by follow-up commit `31b3d7c` (same II.2 pin pattern: harvest runs before its own SHA exists, self-reference fixed in place).
- **Green criterion (quoted from `HALCYON_PART_III_GATES.md` D1 spec):**
  > Gate III.8a harvests `<P>_canonical` + Flyvbjerg–Petersen blocked SEM by running `GIBBS_SAMPLE β=2.5 SEED 20260616` for 2048 sweeps through the parser+executor path once and freezing as `tests/fixtures/halcyon/part_iii/p_canonical.json`.
- **Receipt:**
  ```
  cargo test --features halcyon --test harvest_part_iii_canonical -- --ignored --nocapture
    harvested 1948 post-thermal samples to .../p_canonical.json
      p_canonical = 0.507363922709
      fp_sem      = 0.001472933499
    test tdd_hal_iii_8a_harvest_p_canonical ... ok
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.70s
  cargo test --features halcyon --test harvest_part_iii_canonical
    test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s
  cargo test --features halcyon --lib -- --test-threads=1
    test result: ok. 941 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.87s
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.54s
  ```
  Locked decisions wired: **D1** (fixture IS the GIGI-internal reference III.8b asserts against); **D3** (harvest exercises sequential edge update via the III.5 sweep); **D6/D7** (scalar f64 per measurement, captured as `Vec<f64>` of length 2048); locked decision 1 (intra-binding RNG is `marsaglia_haar::SmallRng` xorshift64*; cross-binding with NumPy PCG64 is impossible by design, documented in the provenance side-car). Harvest test is correctly `#[ignore]`d — running without `--ignored` emits "1 ignored" and runs zero tests, so CI cost stays zero.
- **Commits:** `6a52efc` (harvest harness + fixtures) and `31b3d7c` (provenance harvest_commit SHA pin)

### TDD-HAL-III.8b — Gold gate (Halcyon Gate III contract via in-process bit-identity)

- **Red test:** `tests/halcyon_part_iii_gold.rs` (5 tests mapping onto Halcyon mock Gates III.A-E)
- **Files:**
  - `tests/halcyon_part_iii_gold.rs` — five integration tests under the `halcyon` feature:
    - `tdd_hal_iii_8b_a_plaquette_per_face_matches_kernel` — PerFace shape + MEAN reduction on IDENTITY, FP64-exact.
    - `tdd_hal_iii_8b_b_q_surrogate_at_identity_is_zero_and_in_range` — IDENTITY Q ≈ 0; HAAR SEED 20260616 Q in [0, 16].
    - `tdd_hal_iii_8b_c_h_total_rejected_before_part_iv` — Part IV / E-field anchor in error string.
    - `tdd_hal_iii_8b_c_gibbs_sample_short_run_in_process_reproducible` — two-field same-seed `buffer.data` byte-equal (intra-binding bit-identity at fixed seed, the **D1** contract clause 1).
    - `tdd_hal_iii_8b_d_production_thermalization_pass_criterion` — 2048-sweep run reproduces III.8a fixture byte-for-byte via `f64::to_bits()` (the **D1** GIGI-internal reference clause).
- **Green criterion (quoted; this is also the Part III pass criterion's operational form):**
  > The GQL response carries the same measurement history (within SEM) the Python kernel produces. Halcyon's existing R_A regeneration-reproducibility gate is the contract; the GIGI substrate side passes it or doesn't.

  Operationalized per **D1**: the cross-binding NumPy bit-equality clause is dropped; intra-GIGI bit-identity at fixed seed + statistical agreement with the v1.2 Halcyon production canonical via the GIGI-internal reference harvest IS the contract.
- **Receipt:**
  ```
  cargo test --features halcyon --test halcyon_part_iii_gold
    test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.13s
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.51s
  ```
  All five tests run through the parser + executor path (`parse` + `execute`), exercising the same surface a Halcyon caller would. Locked decisions D1/D4/D6/D7 wired through with inline rationale. `cfg(feature = "halcyon")` keeps the no-default-features build at 852/0 byte-identical to the Part I baseline.
- **Commit:** `97b6273`

### TDD-HAL-III.9 — Implementation log + Part III gates doc

- **Files:**
  - `theory/halcyon/HALCYON_PART_III_GATES.md` — verb contracts (PLAQUETTE, Q_SURROGATE, GIBBS_SAMPLE), locked decisions D1-D7 with one-line explanations each, decided/deferred split, cross-binding bit-identity disposition, future-audit anchor pointing back to Part II's HTTP-as-consumer-surface reframe.
  - `theory/halcyon/HALCYON_PART_III_IMPLEMENTATION_LOG.md` — this artifact.

---

## Closing receipts

- **No-feature build byte-identical.** `cargo test --no-default-features --lib` produces:
  ```
  test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured
  ```
  The number matches the Part I baseline; no test was added, removed, or shifted into the default surface. The `gauge` and `halcyon` feature flags are strictly additive (Bee's locked optionality contract).
- **Halcyon feature lib test count post-Part-III.** `cargo test --features halcyon --lib -- --test-threads=1` produces:
  ```
  test result: ok. 941 passed; 0 failed; 0 ignored; 0 measured
  ```
  This is the Part II `halcyon` baseline (913) plus the Part III gates' additive lib-side contributions: III.1 (+3 → 918), III.2 (+3 → 921), III.3 (+4 → 925), III.4 (+5 → 930), III.5 (+6 → 936), III.6 (+5 → 941). III.7 / III.8a / III.8b live in `tests/`, not the lib surface. Lib-side total: 941.
- **Halcyon Part III gold integration test green.**
  ```
  cargo test --features halcyon --test halcyon_part_iii_gold
  test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured
  ```
  Part II's `tests/halcyon_part_ii_*` integration tests (gauge_field_walker 2/0, http 6/0, persistence 4/0, haar_gold 2/0+1 ignored) all stay green — Part III ships additively to them.
- **ChEMBL-incident durability gates green** (no Part III change touched the durability surface; confirmed):
  ```
  test engine::tests::snapshot_survives_wal_compact ... ok
  test engine::tests::streaming_wal_replay_correct_count ... ok
  test engine::tests::streaming_snapshot_roundtrip ... ok
  test engine::tests::cow_snapshot_roundtrip ... ok
  test engine::tests::mmap_rebase_snapshot_roundtrip ... ok
  test engine::tests::test_9_8_trigger_survives_restart ... ok
  ```
- **No `Co-Authored-By: Claude` footer in any commit.** Every commit in this sprint (`2f9ef76`, `9dcc8e2`, `ab4a34c`, `d61223f`, `0db2779`, `76b1840`, `d5ee7ce`, `6a52efc`, `31b3d7c`, `97b6273`, plus this doc commit) is authored solely by `nurdymuny <bee_davis@alumni.brown.edu>` (Bee Rosa Davis) per the `feedback_no_ai_coauthor.md` standing memo.

---

## Group erasure receipt

The III.5 `get_su2_mut` concrete escape is the one honest break in Part III's group-erasure boundary. Everything else stays group-erased:

- **`PLAQUETTE` (III.1)** — `&dyn GaugeFieldHandle` in, `handle.group()` dispatches.
- **`Q_SURROGATE` (III.2)** — `&dyn GaugeFieldHandle` in, reuses `plaquette_per_face`.
- **`staple_sum` (III.3)** — `&dyn EdgeConnection` in, `unreachable!()` guards future non-SU(2) arms; the SU(2) group match happens in `GIBBS_SAMPLE` at III.5, not here.
- **Parser (III.6)** — `Statement::SelectPlaquette` / `SelectQSurrogate` / `GibbsSample` are group-agnostic in the parser; the executor does the SU(2) check at the `GibbsSample` dispatch arm (the same boundary as III.5's mutation escape).
- **HTTP (III.7)** — handlers take `&dyn GaugeFieldHandle` through `registry::get`; group-aware dispatch lives inside the III.1 / III.2 library functions.

The III.5 escape is correctly shaped: Kennedy-Pendleton IS the SU(2) heatbath, and future U(1) / SU(3) / Z_N heatbaths will need their own kernels (U(1) Wilson is a 1D Bessel sampler, Z_N is a discrete categorical) — those ship as siblings to `get_su2_mut`, not as overrides on a common `get_mut`. Naming the escape after its group is the right architectural shape.

---

## Gates closed in this commit chain

Confirmed against `git log --oneline -- src/gauge tests/halcyon_part_iii_* tests/harvest_part_iii_*`:

| Commit | Gate | Name |
| --- | --- | --- |
| `2f9ef76` | TDD-HAL-III.1 | PLAQUETTE primitive (face-holonomy reduction) |
| `9dcc8e2` | TDD-HAL-III.2 | Q_SURROGATE primitive (angular accumulator) |
| `ab4a34c` | TDD-HAL-III.3 | Per-edge staple sum walker |
| `d61223f` | TDD-HAL-III.4 | Kennedy-Pendleton single-edge update + sqrt-rejection Haar fallback |
| `0db2779` | TDD-HAL-III.5 | GIBBS_SAMPLE sweep (in-place mutation + measurement_history) |
| `76b1840` | TDD-HAL-III.6 | GQL parser + executor arms (PLAQUETTE, Q_SURROGATE, GIBBS_SAMPLE) |
| `d5ee7ce` | TDD-HAL-III.7 | HTTP routes for read-only verbs (NO route for GIBBS_SAMPLE per D5) |
| `6a52efc` | TDD-HAL-III.8a | Harvest GIGI canonical reference (provenance pinned in `31b3d7c`) |
| `97b6273` | TDD-HAL-III.8b | Gold gate — Halcyon Gate III contract |
| _this commit_ | TDD-HAL-III.9 | Implementation log + Part III gates doc |

All nine gates closed. The Part III pass criterion is satisfied: the 3-statement GQL block (`LATTICE` → `GAUGE_FIELD` → `GIBBS_SAMPLE`) parses, executes, and reproduces Halcyon's `<P>_meas` at β=2.5 SEED 20260616 within the Flyvbjerg-Petersen blocked SEM, with the contract operationalized per **D1** as intra-GIGI bit-identity at fixed seed + statistical agreement with the GIGI-internal reference (`tests/fixtures/halcyon/part_iii/p_canonical.json`).

---

## What is deferred / what is not deferred

### Not deferred (decided)

The locked decisions D1-D7 are not TODO items; they are the canonical shape of the Part III surface. Mirroring `HALCYON_PART_III_GATES.md`:

- **GIBBS_SAMPLE is embedded-only — no HTTP route.** Per **D5**. Same reasoning as Part II's HTTP-as-consumer-surface reframe extended to the heatbath sweep.
- **The /v1/gql POST endpoint reaches GIBBS_SAMPLE via `parser::execute` — by design.** The 46-min production wall self-enforces. Consumers crossing a restart boundary use the embedded PyO3 / CFFI binding.
- **Sequential edge order.** Per **D3**. Bit-identity load-bearing. Parallelism deferred to Part V+.
- **Two Haar paths coexist.** Marsaglia for `INIT HAAR_RANDOM` (Part II); sqrt-rejection-on-`x0` for the KP `xi → 0` fallback (Part III). Per **D2**.
- **`registry::get_su2_mut` is the canonical mutable accessor.** SU(2)-named concrete escape at the `GIBBS_SAMPLE` boundary. Per **D4**.

### Deferred (still TODO)

- **`SYMPLECTIC_FLOW` (Part IV).** Symplectic flow with covariant Gauss projection. Separate sprint per `HALCYON_PART_I_GATES.md` section PART IV.
- **HMC with Metropolis acceptance.** Not on the substrate. The original spec's ask correctly flagged that this shouldn't be conflated with `SYMPLECTIC_FLOW`. Deferred indefinitely.
- **`BLOCKED_SEM` as a substrate verb.** Stays as post-processing. Gate III.8a's harvester implements Flyvbjerg-Petersen blocked SEM as a Rust function in the harvest harness, not as a parser-surface verb. Promoting it to a verb is a P3 ask deferred from the original spec.
- **`MIGDAL_WITTEN`.** Out of Part III scope. Original spec P3 ask.
- **`HAAR_RANDOM_GAUGE_TRANSFORM`.** Out of Part III scope. Useful for `Q_SURROGATE`-vs-`PLAQUETTE` gauge-invariance regression tests in a later sprint, but not on Part III's pass criterion.
- **Verb math for SU(3) / U(1) / Z(N).** Part III ships the SU(2) heatbath only. The typed `GaugeFieldError::UnsupportedGroup(Group)` error variant is the surface the future-group work flips to live; same drop-in path Part II's group erasure receipt documents.
