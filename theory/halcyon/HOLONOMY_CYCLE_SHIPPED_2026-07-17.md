# HOLONOMY AROUND CYCLE — Poincaré Tier 1 readout · SHIPPED 2026-07-17

Distinguishing observable for a lens space `L(p,q) = S³/ℤ_p`: the SU(2)
holonomy around a named non-contractible lattice loop, plus its order.
Davis–Poincaré Thm 3.6 (holonomy-trivial ⟺ π₁ = 0). No new group math —
the ordered link product reuses the untouched
`gauge::holonomy::walk_loop`; this verb only builds the ordered edge
list and adapts at the call site.

## Verb

```
HOLONOMY <field> AROUND CYCLE AXIS <ax> AT (<c0>, <c1>);
HOLONOMY <field> AROUND CYCLE EDGES (<e0>, <e1>, ...);
```

- **AXIS** — named lattice cycle: all links along `<ax>` (`x`/`y`/`z`/`w`
  or a 0-based index) at fixed transverse coords `(c0, c1)`, +axis
  order, closing via the wrap edge. Requires the field's bound **CUBIC**
  lattice (DIM=3, two transverse coords). Errors clearly if the binding
  is missing or non-cubic.
- **EDGES** — explicit ordered edge-id list; no lattice binding
  required; product taken in list order, each edge Forward.

Returns one row `{ q0, q1, q2, q3, re_trace, order_estimate,
group_used }`. `re_trace = ½·Tr(U) = q0`; `order_estimate` Int
(best-effort); `group_used = "SU(2)"`. Non-SU(2) → typed error
`HOLONOMY AROUND CYCLE requires GROUP SU(2) in this phase (quaternion
readout); got {g}` (gated before the walker — no compose panic).

## Direction convention (pinned by H3, verbatim)

> AXIS walks +axis order; each edge whose stored lattice direction
> matches the walk contributes U (Forward), else U† (Reverse); on a
> periodic cubic the z-wrap link is stored (s, s+ê_z) so the +z walk
> returns Ω, not Ω†.

A reversed convention reads every class `p` as its inverse class
silently (`re_trace` even in q → order unchanged; quaternion axis sign
flips). H3 pins it: reversing a loop (reverse order + inverted links)
yields the exact conjugate `(q0, −q1, −q2, −q3)`, and `A·B ≠ B·A`.

## order_estimate

Best-effort nearest integer `p` with `gᵖ = 1`. `p` = denominator of
`x = arccos(q0)/(2π)` in lowest terms, via continued-fraction
(Stern–Brocot) approximation, `TOL = 1e-9`, `Q_MAX = 512`; identity
(`q0 ≥ 1 − TOL`) → 1. Branch-robust (`arccos` folds `φ↔2π−φ` but
denominators of `x` and `1−x` match). Nails `p ∈ {2,3,5,7}`,
`q ∈ {1,2,3}` on the fixtures; not a robust arbitrary-order finder.
Client can re-derive from `re_trace`.

## Files

- `src/holonomy_cycle.rs` (**new**, `#[cfg(feature="gauge")]`) — edge-list
  builders (`axis_cycle_edges` via `resolve_edge` over the +axis vertex
  cycle; `edges_cycle_edges` from the arg list), `parse_cubic_hint`
  (L,D from the `CUBIC_L{L}_D{D}` hint), `order_estimate`, and the shared
  executor `execute_holonomy_cycle`.
- `src/parser.rs` — `Statement::HolonomyCycle { field, spec }` +
  `pub enum CycleSpec { Axis{axis,c0,c1} | Edges(Vec<usize>) }`; the
  `AROUND` arm in `parse_holonomy` (+ `parse_axis_token`); `execute()`
  routes to the shared executor (gauge) / errors (no-gauge).
- `src/halcyon_gql_dispatch.rs` — `HolonomyCycle` arm in
  `try_dispatch_topology_statement` (bundle-pre-resolve bypass, like
  CHERN_CLASS — gauge-field target, not a bundle).
- `src/bin/gigi_stream.rs` — `HolonomyCycle` in `is_topology_verb`;
  parity arm in the streaming executor; `"HOLONOMY_CYCLE"` metric name.
- `src/lib.rs` — `pub mod holonomy_cycle;` (gauge-gated).
- `tests/holonomy_cycle_basic.rs` (**new**) — H1..H7.

All three call sites (in-process `execute`, `/v1/gql` dispatcher,
streaming executor) delegate to the one `execute_holonomy_cycle`, so
behavior is byte-identical everywhere. `walk_loop`'s body is unedited.

## Math anchors (H1..H7 — all green)

| # | Anchor | Result |
|---|--------|--------|
| H1 | identity loop | `(1,0,0,0)`, `re_trace` 1.0 exact, order 1 (AXIS + EDGES) |
| H2 | lens wrap | `Ω` exactly, `re_trace = cos(2πq/p)` to 1e-12, order = p for `(p,q) ∈ {(2,1),(3,1),(5,1),(5,2),(7,1),(7,3)}` |
| H3 | direction pin | reverse loop → conjugate `(q0,−q1,−q2,−q3)`; `A·B ≠ B·A` |
| H4 | EDGES vs AXIS | AXIS z at `(x0,y0)` == closed-form `eid = 2·V + site(x0,y0,z)` |
| H5 | composite loop | `Ω_a·Ω_b` genuine ordered group product (hand-computed, non-scalar) |
| H6 | non-SU(2) | typed error naming required + actual group |
| H7 | AXIS w/o lattice | typed error naming the missing binding; non-cubic → names CUBIC |

TDD: RED (test only) → all 7 fail at the grammar
(`Expected ON or NEAR after HOLONOMY bundle, got AROUND`); GREEN → 7/7.

## Gates (all green, on the merged worktree)

- `cargo check --features "kahler imagine sharded transactions patterns causal_states wish halcyon" --bin gigi-stream` ✓
- `cargo test --no-default-features --lib` → 916 passed ✓ (optionality contract)
- `cargo test --features halcyon --test holonomy_cycle_basic` → 7/7 ✓
- chern_class_basic (6), chern_class_bundle_target_basic (11), betti_pi1_basic (8), obstruction_basic (5), topology_verbs_gql_integration (9) ✓
- spectral_gauge_basic (21), spectral_gauge_where_basic (7), spectral_full_basic (12), spectral_magnetic_basic (9, 544s), u1_flux_basic (12) ✓
- gauge_su3_basic (11), gauge_su3_persistence (4) ✓
- ingest_as_gauge_field_basic (18), ingest_gauge_vertex_basic (8), ingest_npz_key_basic (4), ingest_npz_dtype_basic (4), ingest_gql_bypass_basic (5), halcyon_l24_workflow_e2e (1) ✓
- imagine_coherence_phase2 (10), halcyon_part_iv_gold (4, 1 ignored), aurora_lie_poisson_trait (12) ✓
- cubic_lattice (7), lattice_obc_basic (10), davis_conjecture_lambda_brain_ridealong (25), pattern_hunt_parser (15) ✓
- emit_csv, noop_notices, timestamp_ergonomics, gql_reference_truth, explain_kappa, ingest_dir_gate, pathguard_escapes, ingest_executor, ingest_csv_basic, ingest_jsonl_basic ✓

## Live probe (SHIPPED — prod `gigi-stream.fly.dev`, image `deployment-01KXRPWD9CKPF8VF2F9SH1PQ7M`)

All live probes PASS on the deployed image. Receipts:

- **P1** `LATTICE lp2_pc FROM CUBIC L=5 DIM=3 PERIODIC;` → `ok`.
- **P4 identity control (the p=1 lens: L(1,q)=S³, π₁=0)** —
  `GAUGE_FIELD lp2_id … INIT IDENTITY;` then
  `HOLONOMY lp2_id AROUND CYCLE AXIS z AT (1,2);` → `q0=1.0`,
  `re_trace=1.0`, `order_estimate=1`, `group_used="SU(2)"`. EXACT. This
  is a genuine live lens readout (the trivial-twist control).
- **P3h non-trivial live group walk** — `INIT HAAR_RANDOM SEED 42` (a
  deterministic non-identity SU(2) field), `AXIS z AT (1,2)` →
  `re_trace=-0.23984`, `q=(-0.23984, 0.69927, -0.54940, -0.38943)`,
  `order_estimate=454`, `group_used="SU(2)"`. A real ordered quaternion
  product (not identity), with the best-effort order bounded ≤ Q_MAX.
- **P5 AXIS↔EDGES parity (live, on the non-trivial field)** —
  `HOLONOMY lp2_haar AROUND CYCLE EDGES (261, 286, 311, 336, 361);`
  returns the EXACT same quaternion as the AXIS form (`261 = 2·125 +
  site(1,2,0)`, …). Proves the AXIS lattice-enumeration reproduces the
  closed-form z-link ids on live data.
- **P6 group gate** — a registry-resolvable non-SU(2) field
  (`GROUP SU3 INIT IDENTITY`) → typed error `HOLONOMY AROUND CYCLE
  requires GROUP SU(2) in this phase (quaternion readout); got SU(3)`.
  (Note: a `GROUP U1 INIT FLUX` field is materialized off-registry by
  `gauge::u1_flux` and reads back as "not declared" rather than the
  group error, so the SU(3) field — matching test H6 — is the correct
  non-SU(2) live probe.)
- **P7 Marcella sanity** — `imagine_coherence` dim=4 on
  `claude_substrate_v0` → `200`, seed `coherence=1.0`,
  `endpoint_coherence=0.99961`, `refused=false`.

**The exact p=5 lens twist receipt** (`re_trace = cos(2π/5) = 0.309017`,
`q3 = +sin(2π/5) = 0.951057`, `order_estimate = 5`) is **gate-locked by
H2 (p=5) + H4** — the acceptance fixtures build the byte-identical
twisted-BC field that `pc_gigi_lens_space.py` will emit and run the
identical `execute_holonomy_cycle`. It was NOT reproduced against prod
because planting a *controlled* SU(2) link (`Ω` on one z-wrap) requires a
server-side NPZ ingest (`INGEST … AS GAUGE_FIELD` reads a file under
`GIGI_INGEST_DIR`); the public HTTP API exposes only IDENTITY / HAAR /
FROM_FIELD field init, none of which plants a chosen link. Hallie's
Halcyon-side ingest path DOES plant it, so her first read reproduces the
0.309017 / 5 receipt that H2/H4 pin.

## Concurrency + durability notes (ship record)

- **Integrated-main ship.** A peer ship agent shipped the `SPECTRAL MODE
  MATRIX` verb concurrently, cherry-picking its 4 commits onto this
  verb's `cb07bc9` (my `fix(review)`) to make `main = c377d68` (both
  verbs). The grep gate is clean across the whole `c0370e3..c377d68`
  range (0 AI co-author trailers; author = Bee). The HOLONOMY gates passed on
  the clean `cb07bc9` tree (check_bin, lib_nodefault, holonomy_cycle
  7/7, topology, spectral); the orthogonal suites passed on the
  `5af1175` superset; the deploy's Docker build re-compiled the full
  `c377d68`; the live probe is the true acceptance. Both agents converge
  on `origin/main = deployed = c377d68`.
- **Substrate durability.** The deploy restart wiped
  `claude_substrate_v0` (records fragile until snapshotted — the known
  wedge). Restored from `.deploy-backups/2026-07-17/` (create with
  `keys=["thought_id"]` + import) → count **20** (t001–t020), verified.
  `POST /v1/admin/snapshot` still FAILS —
  `Invalid arithmetic pattern: Invalid step in
  'binary_version@v6.7.0+gemini-drift-v08+0'` — a *different* bundle's
  field value breaks the global snapshot encoder, so the substrate could
  not be persisted durably this session (flagged for follow-up; outside
  the HOLONOMY change and the locked durability surface).

## Not built (named)

Tier 2 — cell-complex ingest (arbitrary declared 2-cells / non-lattice
loops, general π₁ presentation readout). SU(3)/U(1) loop holonomy. Both
are separate, larger asks; the `EDGES` form is the arbitrary-loop escape
hatch on SU(2) in the meantime.
