# HALCYON Part III — Gates

**Companion to:** `HALCYON_PART_I_GATES.md` section PART III, `HALCYON_TO_GIGI_REPLY_2026-06-17.md` section A1 (Q_SURROGATE math) and Q5 (GIBBS_SAMPLE naming).
**Voice:** first-person, mine (Bee). Sober register. I spec the algorithm here, not the prose around it.

This document fixes the verb contracts and the locked decisions I made on 2026-06-18 going into the Part III sprint. The receipts (per-gate red/green, commit SHAs, test counts) live in `HALCYON_PART_III_IMPLEMENTATION_LOG.md`.

---

## Part III pass criterion

Quoted verbatim from `HALCYON_PART_I_GATES.md` section PART III:

> The first ~80 lines of `inertia_damping/run_validation_report.py` (the build_graph → heatbath_thermalize → measure phase) replaces with a 3-statement GQL block:
>
> ```sql
> LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY "S2";
>
> GAUGE_FIELD U ON LATTICE buckyball GROUP SU(2) INIT IDENTITY;
>
> GIBBS_SAMPLE U
>   BETA 2.5
>   N_SWEEPS 200
>   MEASURE_EVERY 1
>   MEASURE (MEAN(PLAQUETTE), Q_SURROGATE)
>   SEED 20260616;
> ```
>
> …and the GQL response carries the same measurement history (within SEM) the Python kernel produces. Halcyon's existing R_A regeneration-reproducibility gate is the contract; the GIGI substrate side passes it or doesn't.

I extended the production block to `N_SWEEPS 2048` for the III.8a harvest so the Flyvbjerg–Petersen plateau detector actually has block sizes to walk; the 200-sweep shape stays the published spec line.

---

## Per-verb specs

### `PLAQUETTE OF field` (Gate III.1, SELECT-side lift Gate III.6)

Pure-library primitive `gauge::plaquette_per_face` / `plaquette_mean` / `plaquette_sum`. Group-erased entry: takes `&dyn GaugeFieldHandle` and dispatches on `handle.group()`. SU(2) reduces the face holonomy quaternion to `q0` (= Re tr(U_f) / 2 for SU(2)); other groups return `GaugeFieldError::UnsupportedGroup(g)` without touching the walker.

Three call shapes ship at parser-level Gate III.6 as `Statement::SelectPlaquette { field, reduction }` where `reduction` is the `gauge::observables::PlaquetteReduction` enum:

- `SELECT PLAQUETTE OF U` → `PerFace` → row carries `per_face: Vector<f64>` of length F (q0 column only, per **D7**).
- `SELECT MEAN(PLAQUETTE OF U)` → `Mean` → row carries scalar `value: Float`.
- `SELECT SUM(PLAQUETTE OF U)` → `Sum` → row carries scalar `value: Float`.

HTTP surface at Gate III.7: `GET /v1/gauge_field/{name}/plaquette?reduction=…` mirrors the same three shapes.

### `Q_SURROGATE OF field` (Gate III.2, SELECT-side lift Gate III.6)

Pure-library primitive `gauge::q_surrogate`. SU(2) returns `(1 / 2π) · Σ_f arccos(clamp(q0_f, −1, 1))`, the second of A1's two desugarings (the `PLAQUETTE` returns `q0` branch). The clamp BEFORE arccos ordering is pinned by a test that recomputes the sum with the same clamp-then-arccos sequence and asserts byte-identity with the library output — a future refactor that arccos'd before clamping would NaN at `q0 = 1.0 + 1e-16` AND fail that test.

Scalar `f64`, per **D6**. Other groups error with `UnsupportedGroup(g)`.

HTTP surface at Gate III.7: `GET /v1/gauge_field/{name}/q_surrogate` returns `{field, value: f64}`.

### `GIBBS_SAMPLE field BETA β [N_SWEEPS n] [MEASURE_EVERY k] [MEASURE (...)] [SEED int]` (Gates III.3 → III.5, parser-lift Gate III.6)

Heatbath sweep on a `GAUGE_FIELD`. Live at launch for SU(2) only; non-SU(2) groups error with `UnsupportedGroup(g)` at the executor boundary before mutation begins.

Per-edge update path:

1. `gauge::staple::staple_sum(handle, lat, e) → [f64; 4]` (Gate III.3): walks the four faces touching edge `e` via the lattice incidence table; for each face skips the self-edge, then multiplies the remaining three edges in face-orientation order; sums the four contributions as a quaternion. Group-erased over `&dyn EdgeConnection`; SU(2) is the only implemented arm; the unreachable arm guards future U(1) / Z_N dispatch which belongs in GIBBS_SAMPLE not in the walker.
2. `gauge::kennedy_pendleton::sample_su2_link(&mut rng, beta, staple) → [f64; 4]` (Gate III.4): Kennedy–Pendleton heatbath kernel. `sample_kp_x0` does the κ-rejection on `x0` with `MAX_KP_ITERS = 400` and `EPS_K = 1e-12`. If `||staple|| · β < EPS_K`, falls through to `gauge::heatbath_haar::sample_haar_sqrt_rejection` (the second Haar path per **D2**). RNG draws 4 uniforms per attempt (pinned by a counter test).
3. `gauge::gibbs_sample::run` (Gate III.5): for each sweep, `for e in 0..n_edges` walk the edges in sequential order (**D3** — bit-identity load-bearing; parallelism deferred to Part V+). After each sweep, if `sweep % measure_every == 0`, apply each requested observable. Field is mutated in place via `gauge::registry::get_su2_mut(name)` (the **D4** concrete escape — SU(2)-named because Kennedy–Pendleton IS the SU(2) heatbath; the `get()` / `GaugeFieldHandle` surface is untouched).

Measurement battery at launch:

- `MEAN(PLAQUETTE)` — scalar f64 per measurement.
- `Q_SURROGATE` — scalar f64 per measurement.
- `H_TOTAL` — rejected at parse time with `"H_TOTAL requires an E field, which is a Part IV (SYMPLECTIC_FLOW) construct"`. The Part IV anchor lives in the test name `tdd_hal_iii_8b_c_h_total_rejected_before_part_iv`.

Wire shape: GQL response is a single `Rows` row with `{field, seed, beta, n_sweeps_completed, <obs.label()>: Vector<f64>}` per measured observable. There is **no** dedicated HTTP route for `GIBBS_SAMPLE`; the route table at `/v1/gauge_field/{name}/gibbs_sample` returns 404. See **D5** below.

---

## Locked decisions inherited from Bee 2026-06-18

Each holds through every Part III gate. None get relitigated mid-sprint.

- **D1 (SEM contract).** Gate III.8b's envelope assertion writes against a GIGI-internal reference, NOT Halcyon's hand-rolled `3 · sem_chain + 0.02` envelope. Gate III.8a harvests `<P>_canonical` + Flyvbjerg–Petersen blocked SEM by running `GIBBS_SAMPLE β=2.5 SEED 20260616` for 2048 sweeps through the parser+executor path once and freezing as `tests/fixtures/halcyon/part_iii/p_canonical.json`. Cross-binding bit-identity (Rust vs NumPy PCG64 mock) is impossible by design; the contract is intra-GIGI bit-identity at fixed seed.
- **D2 (Haar fallback).** A SECOND Haar sampler ships at `src/gauge/heatbath_haar.rs` using sqrt-rejection-on-`x0` + spherical placement (mirrors `buckyball_heatbath.py:119-134`). Part II's `marsaglia_haar` stays for `INIT HAAR_RANDOM`. Two Haar paths coexist; each named honestly. Locked decision 1 (xorshift64* RNG) holds for both.
- **D3 (Edge update order).** Sequential `for e in 0..n_edges`. Mirrors Halcyon Python. Bit-identity load-bearing. Parallelism deferred to Part V+.
- **D4 (Registry mutability).** `pub fn get_su2_mut(name) -> Option<Arc<Mutex<SU2GaugeField>>>` ships alongside the existing `get()`. SU(2)-named concrete escape at the `GIBBS_SAMPLE` boundary; II.4 registry signature (the `Arc<dyn GaugeFieldHandle>` accessor) stays untouched. I chose `Arc<Mutex<…>>` over the spec's literal `MutexGuard<…>` because `MutexGuard` is non-`'static` and cannot escape its lock scope; same access pattern, longer lifetime.
- **D5 (/v1/gql soft-edge).** Route-table-only enforcement: 404 on a dedicated `/v1/gauge_field/{name}/gibbs_sample` route. The `/v1/gql` POST endpoint reaches `GIBBS_SAMPLE` via `parser::execute` — that is by design. The ~46-min wall on the production sweep self-enforces the soft-edge; nobody runs a 46-minute mutation verb over HTTP and waits for it to JSON-tax. See "Future-audit anchor" below.
- **D6 (Q_SURROGATE shape).** Scalar `f64`. Mirrors Halcyon mock byte-for-byte at the JSON level.
- **D7 (PLAQUETTE shape).** `per_face` returns `Vec<f64>` of length F (q0 column only). Mirrors mock + spec example shapes. `Mean` and `Sum` reductions return scalar `f64`.

---

## Cross-binding bit-identity disposition

GIGI uses xorshift64* (`marsaglia_haar::SmallRng`, the canonical CSPRNG for the entire gauge stack — KP kernel + heatbath Haar fallback both consume from it). Halcyon's mock uses NumPy PCG64. In-process bit-identity holds within GIGI at fixed seed. Cross-binding bit-identity is impossible by design and is NOT the contract.

The contract has two clauses:

1. **Intra-GIGI bit-identity at fixed seed.** Same binary, same seed → byte-identical `p_history` over an arbitrary number of sweeps. Pinned by `tdd_hal_iii_8b_c_gibbs_sample_short_run_in_process_reproducible` and by the III.8a → III.8b fixture replay (production 2048-sweep run reproduces the III.8a fixture byte-for-byte via `f64::to_bits()`).
2. **Statistical agreement with the v1.2 Halcyon production canonical via the GIGI-internal reference harvest.** Gate III.8a is the harvest; the fixture at `tests/fixtures/halcyon/part_iii/p_canonical.json` is the GIGI-internal reference. Future envelope assertions (Part IV+ regression sentinels, or any consumer wanting a SEM band around `<P>_canonical`) write against this fixture, not against the NumPy mock.

---

## What is decided / what is not

### Decided (not deferred)

These are not TODO items; they are the canonical shape of the Part III surface.

- **GIBBS_SAMPLE is embedded-only — no HTTP route.** Same reasoning as Part II's HTTP-as-consumer-surface reframe extended to the heatbath sweep: heavy mutation verb, production hot path, ~46-min wall on O(10^6) per-edge updates per run. The route table enforces it at III.7; the soft-edge through `/v1/gql` is by design.
- **The /v1/gql POST endpoint reaches GIBBS_SAMPLE via parser::execute — by design.** I am not gating mutation at the GQL endpoint. The 46-min wall self-enforces. Consumers crossing a restart boundary or running the heavy verbs use the embedded PyO3 / CFFI binding, same as Part II's canonical declarer surface.
- **Sequential edge order (mirrors Halcyon).** Per **D3**. Bit-identity load-bearing. Parallelism deferred to Part V+.
- **Two Haar paths coexist.** Marsaglia 4-uniforms-with-rejection for `INIT HAAR_RANDOM` (Part II's path). Sqrt-rejection-on-`x0` for the KP `xi → 0` fallback (Part III's path, mirrors Halcyon Python `buckyball_heatbath.py:119-134`). Each named honestly. Per **D2**.
- **`registry::get_su2_mut` is the canonical mutable accessor.** SU(2)-named concrete escape at the `GIBBS_SAMPLE` boundary. Per **D4**. `get()` / `GaugeFieldHandle` surface untouched. The mutable accessor returns `Arc<Mutex<SU2GaugeField>>` (lifetime-extended over the spec's literal `MutexGuard<…>` because `MutexGuard` is non-`'static`).

### Deferred

- **`SYMPLECTIC_FLOW` (Part IV).** Symplectic flow with covariant Gauss projection. Separate sprint per `HALCYON_PART_I_GATES.md` section PART IV. The Part III measurement gate decides whether IV earns a sprint.
- **HMC with Metropolis acceptance.** Not on the substrate. The original spec's ask correctly flagged that this shouldn't be conflated with `SYMPLECTIC_FLOW`. Deferred indefinitely.
- **`BLOCKED_SEM` as a substrate verb.** Stays as post-processing. Gate III.8a's harvester implements Flyvbjerg–Petersen blocked SEM as a Rust function in the harvest harness, not as a parser-surface verb. Promoting it to a verb is a P3 ask deferred from the original spec.
- **`MIGDAL_WITTEN`.** Out of Part III scope. Original spec P3 ask.
- **`HAAR_RANDOM_GAUGE_TRANSFORM`.** Out of Part III scope. Useful for `Q_SURROGATE`-vs-`PLAQUETTE` gauge-invariance regression tests in a later sprint, but not on Part III's pass criterion.
- **Verb math for SU(3) / U(1) / Z(N).** Part III ships the SU(2) heatbath. The typed `GaugeFieldError::UnsupportedGroup(Group)` error variant is the surface the future-group work flips to live; same drop-in path Part II's group erasure receipt documents.

---

## Future-audit anchor

Why is `GIBBS_SAMPLE` missing from HTTP routes? Why doesn't `/v1/gql` gate mutation?

See `HALCYON_PART_II_IMPLEMENTATION_LOG.md` section "HTTP-as-consumer-surface (architectural framing)" and the II.6c reframe (commit `d5d3853`). The summary: embedded GQL (PyO3 / CFFI) is the canonical declarer and mutator surface for any consumer crossing a restart boundary or running heavy mutation verbs. HTTP is the consumer-facing introspect channel (`GET`) plus declare-ephemeral-by-default (`POST` lattice / gauge_field) for the mock-to-live swap. `GIBBS_SAMPLE` and any future heavy mutation verb (e.g. `SYMPLECTIC_FLOW`) are pre-committed embedded-only. The route table is the enforcement surface; the soft-edge through `/v1/gql` is by design and the 46-min production wall self-enforces.

Any auditor reading the HTTP routes and noticing `GIBBS_SAMPLE`'s absence should land here.
