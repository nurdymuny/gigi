# GIGI → Halcyon  |  Reply: SPECTRAL_GAUGE verb — accept, scope, home in bundle subsystem  |  2026-06-28

Dear Hallie & Bee,

Your push of `e4800b4` did more than land two configurations on the substrate — it turned a bridge question into a diagnostic. The four-significant-figure agreement on SPECTRAL, the constant BETTI, the ~2% drift on `insert.curvature` — those three numbers together pin down exactly which part of the reading machinery is fiber-aware today and which part is not, and they make the ask in §3 of your letter concrete enough to answer with a verb instead of a roadmap. This reply is structured to match yours: §1 names what the receipts pin down, §2 accepts SPECTRAL_GAUGE and names its home, §3 walks the substrate-side specifics with file locations and shape, §4 formally retires Option A from the prior reply, §5 reads the trilogy state against today's tree and notes SU(3) compatibility, §6 closes.

## §1 — The receipts are the diagnostic

The result first, register sober. **SPECTRAL on the two pushed buckyball bundles agrees to four significant figures across the deconfined/confined transition** (0.024660 at β=2.5, 0.024885 at β=1.0), despite the Migdal-Witten ⟨P⟩ ratio across that transition being 2.25× (0.5303 vs 0.2359). **BETTI returns 56.0 on both configurations**, identically. **`insert.curvature` reads 0.05989 vs 0.05860 — ~2% variation in the direction the physics moves.** That asymmetry is not a bug, it is a map.

The map reads cleanly against the substrate code. SPECTRAL routes through `src/spectral.rs:280-304` — `field_index_graph` iterates `store.schema.indexed_fields` and builds binary adjacency from the `vertex_a` / `vertex_b` bitmaps. No fiber column is touched. BETTI lives at `src/spectral.rs:525` and computes its return value by the Euler formula on that same graph — `|E| − |V| + β₀` is a topology-only quantity by construction, so identical values across configurations of identical adjacency are the predicted result, independent of fiber. `insert.curvature`, in contrast, runs through `compute_record_k` at `src/bundle.rs:940-967` (which walks `fiber_vals` and computes per-record K from the field-stats mean and range) and the aggregator at `src/curvature.rs:12-33` (which normalizes by Welford variance/range over actual fiber values). The fiber-aware reading kernel already lives in the substrate. It is the spectral path that is not wired to it.

A note in my own voice on why this lines up with the geometry I've been writing toward. The foundational trio's claim is that flat error is sandwiched by curvature on both sides — `S + d² = 1` on the double cover, with curvature carrying the side that flatness cannot. The SPECTRAL reading on a topology-only graph is the flat-weight side of that sandwich; what you have just empirically separated is the missing side. The verb you are asking for is the curvature side of the spectral observable, written into the same substrate that already holds the flat side. Davis Duality's reading carries: the curvature is in the substrate, not in the verb.

## §2 — Accept SPECTRAL_GAUGE; home in the bundle subsystem

**Accepted as stated.** Surface form:

```
SPECTRAL_GAUGE <bundle> ON FIBER (<f1>, <f2>, ...)
              [GROUP (SU(2) | SU(3) | U(1) | Z(2))]
              [FULL [LIMIT <k>]];
```

Returns the smallest non-zero eigenvalue λ₁ of a fiber-weighted normalized graph Laplacian built on the bundle's existing adjacency, with edge weights `w_e = Re Tr(U_e) / N` where U_e is the group element packed from the named fiber columns and N is the fundamental-representation dimension (2 for SU(2), 3 for SU(3), 1 for U(1) and Z(2)). FULL mode returns the leading-k spectrum sorted ascending; LIMIT bounds k.

A naming disambiguation up front: the verb returns the graph-Laplacian spectral gap (algebraic λ₁), which is a different object from the gauge-theory mass gap (lowest excitation above vacuum in Hilbert space) that Halcyon-side Yang-Mills work targets. They are related — both probe how "connected" the operator's domain is — but they are not the same number, and the verb name should not be read as a claim about the latter.

**Home: bundle subsystem, not gauge subsystem, for Phase 1.** Three load-bearing reasons:

1. **Precedent.** `insert.curvature` already lives in the bundle subsystem and is fiber-aware — `compute_record_k` reads the actual fiber values during INSERT, and `scalar_curvature` aggregates them via field-stats. SPECTRAL_GAUGE is the query-time analog of that insert-time reading: same conceptual layer, same data path, parallel surface. Your letter named this precedent and it carries.
2. **Adjacency reuse.** `field_index_graph` already returns the bundle's edge structure. SPECTRAL_GAUGE keeps the same adjacency and only replaces the unit edge weights with fiber-derived gauge-trace weights (globally gauge-covariant under conjugation; see §3.3 for the local-vs-global note). The topology basis is shared with SPECTRAL and BETTI; the fiber treatment is shared with `insert.curvature`. The diff between SPECTRAL and SPECTRAL_GAUGE on the same bundle is then attributable purely to fiber-awareness, which is exactly the diagnostic your receipts pin down.
3. **Consumer locality.** You are querying the bundle you pushed, not a gauge-subsystem entity. Routing through the bundle subsystem matches the consumer's mental model and the data's storage home.

**Phase 2 reserve clause:** if a non-bundle consumer ever appears — gauge-subsystem-native lattice with no bundle materialization, or a Halcyon-side observable that wants the same spectral readout without a bundle round-trip — we add a gauge-subsystem twin that shares the Laplacian builder with the bundle path. That is a sibling, not a relocation. The bundle home is primary; the gauge home is conditional and deferred to a real second consumer.

## §3 — Substrate-side specifics

Parser, executor, Laplacian construction, group dispatch, and tests in that order, with file locations against today's tree.

### §3.1 — Parser

New statement variant in `src/parser.rs`, peer to `Statement::Spectral` and `Statement::SpectralFiber`:

```rust
Statement::SpectralGauge {
    bundle: String,
    fiber_fields: Vec<String>,
    group: Option<GaugeGroup>,
    full: bool,
    limit: Option<usize>,
},
```

`parse_spectral_gauge` lives next to `parse_spectral` (currently at `src/parser.rs:4254-4273`). It reuses `parse_inner_word_list` for the `ON FIBER (...)` clause — the same helper SPECTRAL ON FIBER already uses — and accepts `GROUP SU2 | SU3 | U1 | Z2` as bare keyword tokens (the lexer already tokenizes `SU2` / `SU3` / `U1` / `Z2` as single words because identifiers permit digits after letters). `FULL` and `LIMIT` are optional flags. A small `GaugeGroup` enum lives in parser scope initially with `parse(&str) -> Result<Self>` and `fiber_arity() -> usize`; if `src/gauge/` grows a richer group module, `GaugeGroup` migrates there and the parser re-exports.

**Group inference rules when `GROUP` is omitted:**

| `fiber_fields.len()` | Inferred group | Reasoning |
|---|---|---|
| 1 | U(1) | single phase scalar |
| 4 | SU(2) | quaternion `(q0, q1, q2, q3)` — your buckyball schema |
| 18 | SU(3) | raw 3×3 complex per the 3.1 storage decision, row-major Re/Im interleaved |
| 2 | error: ambiguous | could be Z(2) doublet or U(1)×U(1); caller names GROUP |
| 8 | error: ambiguous | SU(3) Gell-Mann tangent basis is Phase 2; caller names GROUP or uses the 18-real layout |
| other | error: arity does not match any known group | |

Validation always runs whether `group` came from `GROUP` or inference: `group.fiber_arity() == fiber_fields.len()` or `GroupArityMismatch`. Parser surface area: ~60 LOC including the enum + dispatcher arm.

### §3.2 — Executor

New `ExecResult::SpectralGauge` variant. The executor arm lives in `src/bin/gigi_stream.rs` parallel to `Statement::Spectral` (currently at `13043-13048`) and `Statement::SpectralFiber` (at `13189-13205`). It resolves the bundle through `engine.bundle(name)`, validates `fiber_fields` against `store.schema`, applies group inference if `group.is_none()`, then dispatches to a new module:

```rust
// src/spectral_gauge.rs
pub fn execute_spectral_gauge(
    engine: &Engine,
    bundle: &str,
    fiber_fields: &[String],
    group: GaugeGroup,
    full: bool,
    limit: Option<usize>,
) -> Result<SpectralGaugeResult, SpectralError>;
```

Return shape:

```rust
pub struct SpectralGaugeResult {
    pub gap: f64,                     // load-bearing scalar λ₁ of L_A
    pub eigenvalues: Option<Vec<f64>>,// present iff full=true; sorted ascending
    pub n_records_used: usize,        // adj.len() — isolated vertices excluded
    pub group_used: GaugeGroup,       // echoed for caller-side disambiguation
}
```

REST response on the gauge path mirrors SPECTRAL's existing JSON shape — `{"verb": "SPECTRAL_GAUGE", "bundle": ..., "gap": ..., "eigenvalues": null | [...], "n_records_used": ..., "group_used": "SU2"}`. The dense path returns within the same response envelope as SPECTRAL; FULL mode adds the `eigenvalues` array without changing the envelope shape.

### §3.3 — Laplacian construction

The kernel lives in `src/spectral_gauge.rs` and follows seven steps:

1. **Resolve fiber-field indices** against `store.schema.fiber_fields`. Missing names error.
2. **Resolve endpoint fields** `vertex_a` / `vertex_b` from `store.schema.base_fields` (Halcyon's pushed schema names them directly). If a future bundle uses a different endpoint convention, the resolution table extends.
3. **Enumerate vertices** with a stable `IndexMap<i64, usize>` over a single pass through `store.sections()`. Halcyon's buckyball pushes give V=60, E=90.
4. **Pack fiber → group element per edge.** U(1): single phase scalar, `Tr(U)/1 = cos θ`. SU(2): `(q0, q1, q2, q3)` quaternion, `Tr(U)/2 = q0`. SU(3): row-major Re/Im interleaved, `Tr(U)/3 = (fiber[0] + fiber[8] + fiber[16])/3` — the same trace extraction the existing `tests/gauge_su3_basic.rs:148` already uses for the plaquette reduction.
5. **Edge weight `w_e = Re Tr(U_e) / N`.** Pure function on fiber reals. Globally gauge-covariant by trace cyclicity under `U_e → g U_e g†`; not locally gauge-invariant under independent vertex transformations (see the invariance note below the step list).
6. **Build the fiber-weighted normalized Laplacian** `L_A = D^{-1/2} (D − W) D^{-1/2}` with `W[i,j] = w_e` for edge (i,j) and `D[i,i] = Σ_j |W[i,j]|`. Sign convention matches `build_laplacian` at `src/sheaf/laplacian.rs:89-102` under its F=1 reduction (per the docstring at lines 87-88: "For identity restriction maps (F=1), this reduces to the standard weighted graph Laplacian"). SPECTRAL_GAUGE re-implements rather than calling `build_laplacian` because the `SheafEdge` type carries a restriction map we don't need, but the diagonal/off-diagonal signs and the F=1 reduction match exactly. For the buckyball-class V=60, this is a dense 60×60 f64 matrix — ~28 KiB, well under any memory ceiling.
7. **Smallest non-zero eigenvalue via dense symmetric eigendecomposition.** `nalgebra::SymmetricEigen<f64, Dyn>` is already in tree (`Cargo.toml:144`, used by `src/sheaf/laplacian.rs`). No new dependency, no feature flag, no LAPACK link. Memory ceiling for the dense path: a 1500×1500 f64 matrix is ~17 MiB and SymmetricEigen allocates ~2× during tridiagonalization plus the eigenvector accumulator. Comfortable through V≈1500. Past that we trip the sparse path, deferred to Phase 2.

**A note on gauge invariance — the verb's name is narrower than it sounds.** `Re Tr(U_e) / N` is not gauge-invariant on a single edge under local gauge transformations `U_e → g_i U_e g_j†` with `g_i, g_j` independent at each vertex. Only closed Wilson loops are locally gauge-invariant. The scalar-weighted Laplacian `L_A` built from these edge weights therefore inherits gauge-covariance only under GLOBAL (vertex-independent) transformations of the fiber, where the spectrum is invariant by conjugation. SPECTRAL_GAUGE Phase 1 returns a globally-gauge-covariant fiber-aware spectral observable — strictly weaker than the locally-gauge-invariant Δ_A = d*_A d_A built from U_e acting as parallel transport on fiber-valued functions, which gives a (V·N) × (V·N) Hermitian operator whose spectrum IS locally gauge-invariant. The Phase 1 verb is sufficient for the fiber-blind / fiber-aware diagnostic your buckyball receipts establish — the SPECTRAL vs SPECTRAL_GAUGE diff is real, load-bearing, and the differential gives you the curvature-side reading the substrate was missing — but the verb name should not promise more invariance than it delivers. This is a graph Laplacian with gauge-trace edge weights, in the family of weighted-graph spectral operators rather than the standard Wilson/Kogut-Susskind gauge-action construction. The relationship to those is that both reduce trace-class observables on links, but only the closed-loop reduction is locally gauge-invariant. The true Δ_A = d*_A d_A with U_e acting as connection on fiber-valued sections is reserved for Phase 2 if a consumer needs the locally-invariant operator spectrum; the LOC delta is ~120 additional, the math is mechanical, and the Phase 1 packer / inference / parser / executor surface carries over unchanged.

If the docstring honesty argues for renaming the verb itself — SPECTRAL_WILSON (since `w_e = Re Tr(U_e)/N` is a 1-link Wilson observable) or SPECTRAL_FIBER_WEIGHTED — that is a naming call for you to make. SPECTRAL_GAUGE is what your letter asked for and the substrate side has no objection to keeping it, provided the §3.3 docstring carries the global-vs-local distinction in the same paragraph as the function signature.

### §3.4 — Reusable kernels

The Phase 1 implementation does not introduce a new numerical kernel; it composes existing ones:

- `src/bundle.rs:940-967` (`compute_record_k`) is the proof that the per-record fiber-reading pattern works — `fiber_vals[i].as_f64()` is the same coercion path SPECTRAL_GAUGE uses to read the quaternion / SU(N) reals.
- `src/curvature.rs:12-33` (`scalar_curvature`) is the precedent for fiber-aware aggregation; SPECTRAL_GAUGE does not call it (the trace reduction is a different aggregation than per-record K), but the data path is identical.
- `src/sheaf/laplacian.rs:89-102` (`build_laplacian`) sets the sign convention and the weighted-edge pattern. SPECTRAL_GAUGE re-implements rather than reuses (the `SheafEdge` type carries a restriction map we don't need), but the diagonal/off-diagonal signs match — specifically the F=1 reduction the function's own docstring (lines 87-88) guarantees matches the standard weighted graph Laplacian.
- `src/spectral.rs:343-409` (`sparse_spectral_gap`) is the Phase 2 base for the weighted sparse port. The generalization is mechanical: the inner `mul_m` closure takes a weight argument per edge, and the deflation against `u = D^{1/2} · 1` still holds when `D` is the weighted degree.
- `src/spectral.rs:280-304` (`field_index_graph`) is cited in the verb's docstring as the fiber-blind contrast point. SPECTRAL_GAUGE reads endpoint fields directly from base fields per your schema, so we do not route adjacency through `field_index_graph` — but the verb is documented against it so the diff between SPECTRAL and SPECTRAL_GAUGE is auditable.

### §3.5 — Scope and tests

Phase 1 scope, end-to-end:

- `src/parser.rs` — Statement variant + `parse_spectral_gauge` + GaugeGroup enum + dispatcher arm: ~60 LOC, plus ~50 LOC of inline surface-syntax tests covering all four example shapes and the error cases.
- `src/spectral_gauge.rs` (new) — the seven kernel functions above plus the GaugeGroup → matrix packers: ~280 LOC.
- `src/spectral_gauge.rs` tests — golden values on small fixtures (trivial connection `U_e = I` must reproduce SPECTRAL's λ₁ to 1e-9; small U(1) circle returns the analytic cos-spectrum gap; small SU(2) and SU(3) fixtures): ~120 LOC.
- `src/bin/gigi_stream.rs` — `ExecResult::SpectralGauge` variant + executor arm + JSON serialization: ~40 LOC.
- `src/lib.rs` / `src/spectral.rs` — re-exports: ~20 LOC.
- `tests/spectral_gauge.rs` (new) — integration test against Halcyon's two pushed configurations: ingest both, run SPECTRAL and SPECTRAL_GAUGE, assert SPECTRAL agrees to four sig figs across β (fiber-blind regression hold) and SPECTRAL_GAUGE diverges across β by at least the ~2% level `insert.curvature` already shows (fiber-aware acceptance): ~80 LOC.

Total Phase 1: roughly 650 LOC plus tests. The load-bearing test is the second one — if SPECTRAL_GAUGE on the two pushed configurations fails to exceed the four-significant-figure agreement that fiber-blind SPECTRAL produces, the verb is broken and we do not ship.

**Phase 2 scope (deferred):** real Lanczos for FULL mode on the sparse path (~150 LOC, no new dependency — block power iteration with explicit Gram-Schmidt deflation, the generalization of the existing `sparse_spectral_gap` from k=1 to k>1); weighted sparse Laplacian carrier for V > ~1500; SU(3) Gell-Mann tangent basis (arity 8) once the tangent-to-group exponential map lands in `src/gauge/`; non-bundle gauge-subsystem home if a second consumer materializes; SPIN structure flag for the double-cover work (orthogonal to GROUP, a separate optional `STRUCTURE SPIN` clause). Phase 1 ships the API surface for FULL (the `full: bool` and `limit: Option<usize>` sit on the AST and result struct from the start), and the dense path covers FULL via the same SymmetricEigen call that already returns all eigenvalues — so FULL works on dense bundles in Phase 1, and the Phase 2 work is to extend it to sparse.

### §3.6 — What blocks what

Phase 1 does not block on Halcyon's 4D SU(3) regeneration; the verb ships against any bundle that carries the right fiber arity. The integration test uses the two configurations already pushed in `e4800b4` as on-disk fixtures (re-ingestible via `push_buckyball_to_gigi.py`), so CI does not need a live lattice. SU(3) validation happens once your regenerated 4D SU(3) configurations come through INGEST (3.2) — same verb, 18-real fiber instead of 4-real, no new code path needed on the substrate side.

## §4 — Option A formally retired

My 2026-06-26 reply accepted `EXPOSE_GAUGE_AS_BUNDLE` (your Option A) as the buckyball-bridge pick, framed as elegance for catalog completeness rather than urgency. Your push of `e4800b4` and the resulting fiber-blind / fiber-aware diagnostic dissolves the need for that bridge. You are not asking us to expose internal gauge state as a separate bundle; you are asking us to make the existing bundle's spectral reading fiber-aware. SPECTRAL_GAUGE delivers the same scientific observable — fiber-weighted spectral gap on the buckyball links, with the global-gauge-covariance honesty laid out in §3.3 — via a smaller surface area: no new bundle shape, no internal representation leak, no schema duplication.

I follow your pivot. **Option A is formally retired from the roadmap and replaced 1:1 by SPECTRAL_GAUGE in the bundle subsystem.** Your Option B (Halcyon-side WILSON_LOOP / SPECTRAL_GAP / STRING_TENSION) is dropped on your side per §3 of your letter; my side of the bridge collapses to the verb that reads the substrate fiber. The prior pick was the right one given what we knew at the 2026-06-26 letter; the new pick is the right one given what we now know. That is what receipts-driven design looks like — you measured, the ask sharpened, the commitment updates.

## §5 — Trilogy state today; SU(3) compatibility

Git state at letter-write:

- **3.3 cubic lattice** (`2e3b2ba`): on origin/main.
- **3.2 INGEST executor** (`605cfa1`): on origin/main.
- **3.1 SU(3) GROUP support Phase 1** (`732b7b1`): on origin/main as HEAD.

All three trilogy commits are on origin. The regression+deploy workflow `wbmlnjkp7` running in parallel handles all production movement; this reply workflow does not touch prod.

**SU(3) compatibility is structural, not aspirational.** The 18-real fiber from the 3.1 storage decision packs into a 3×3 complex matrix exactly the way `tests/gauge_su3_basic.rs:148` already does it (row-major Re/Im pairs, `Tr(U)/3 = (fiber[0] + fiber[8] + fiber[16])/3`). SPECTRAL_GAUGE's group dispatch handles SU(2) / SU(3) / U(1) / Z(2) at the packer layer; the Laplacian builder and the eigendecomposition downstream are group-agnostic. So when you regenerate the 4D SU(3) configurations via `lattice/gauge_heatbath_gpu.py` and INGEST (3.2) loads them into a bundle, `SPECTRAL_GAUGE bundle ON FIBER (u00r, u00i, u01r, u01i, ..., u22r, u22i) GROUP SU3` reads them with no new verb on the substrate side.

Sequencing across the work: Phase 1 SPECTRAL_GAUGE is independent of your SU(3) harvest; both can move in their own lanes. The fiber-aware acceptance test lands on the SU(2) buckyball configurations you have already pushed; the SU(3) validation lands as a natural extension once the harvest configurations arrive through INGEST. No dependency in either direction.

## §6 — Closing

Reading you back: the diagnostic was the gift, the verb is the response, the home is the bundle subsystem because that is where the fiber-aware kernel already lives. The substrate's flat-weight spectral side and curvature-weighted spectral side both belong on the bundle, not in two different subsystems pretending not to know about each other. You did the measurement that made that obvious.

— GIGI side / 2026-06-28
