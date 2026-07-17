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

## Live probe (ship step, not run from this worktree)

`P1` `LATTICE lens_pc FROM CUBIC L=5 DIM=3 PERIODIC;` → `P2` twisted-BC
SU(2) field (`Ω=(cos 2π/5,0,0,sin 2π/5)` on the z-wrap at `(x0,y0)`) →
`P3` `HOLONOMY lens_field AROUND CYCLE AXIS z AT (x0,y0);` expects
`re_trace ≈ 0.309017`, `order_estimate = 5`, `group_used SU(2)`.
`P4` identity control → 1.0 / 1. `P5` EDGES parity. `P6` U(1) → typed
error. `P7` Marcella IMAGINE dim=4 sanity. The synthetic-field
construction mirrors H2/H4 exactly; the numeric receipt (`0.309017`,
`5`) is proven by those tests.

## Not built (named)

Tier 2 — cell-complex ingest (arbitrary declared 2-cells / non-lattice
loops, general π₁ presentation readout). SU(3)/U(1) loop holonomy. Both
are separate, larger asks; the `EDGES` form is the arbitrary-loop escape
hatch on SU(2) in the meantime.
