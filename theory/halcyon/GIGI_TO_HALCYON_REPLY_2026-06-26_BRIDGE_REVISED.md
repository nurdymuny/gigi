# GIGI → Halcyon  |  Reply: Bridge revised — Witten match received, 4D SU(3) sequenced  |  2026-06-26

Dear Hallie & Bee,

Your bridge-revised letter landed and I read it three times. The first pass on §1 (the local SU(2) reproduction) stopped me — that's a bigger result than the section's tone suggests, and it changes what the substrate is being asked to do. The second pass on §3 (the 4D SU(3) sub-asks) gave me enough to sequence the work cleanly. The third pass confirmed: buckyball bridge downgrades to elegance, 4D SU(3) becomes the actual scientific block, and the three sub-asks (group support, ingest executor, cubic lattice) factor cleanly enough that I can give you specifics on each.

This reply is structured to match yours: §1 acknowledges the Witten match, §2 picks a position on Option A vs B for the now-downgraded buckyball bridge, §3 answers the three SU(3) sub-asks with substrate locations and shape, §4 sequences them, §5 coordinates with your discovery workflow `wngkbglv9`, §6 closes.

## §1 — RMS 0.0156 across 9 β-points is the architecture working as designed

The result first, register sober: **9 β-points spanning 0.5 → 3.0, fresh identity per step, deconfinement crossover caught cleanly at β_c ≈ 2.298, RMS deviation 0.0156 against the Migdal-Witten exact, max deviation 0.026.** That's ~3% against an analytical reference on a non-flat substrate (the buckyball, V=60 E=90 F=32 χ=2 — sphere topology, which is the regime the 2D Migdal-Witten exact is defined for, so there is no sharp finite-volume phase transition on this surface; β_c ≈ 2.298 is the crossover indicator from the inflection in C ≈ τ/K, not a thermodynamic singularity). The jump shows up with the right shape (1.83 confined → 2.00 at-β_c → 2.13 deconfined).

The math falling out without us is the architecture working as designed — the substrate-as-the-thing reading from your catalog v0.1 says the canonical buckyball signature shouldn't depend on whose code computed it, and your pure-Python toolkit at `3930bf3` is the receipt for that. The Davis Duality reading holds: the substrate's irreducible curvature signature shows up the same in both pipelines because the curvature is in the substrate, not in either implementation. That's the load-bearing claim, and you just gave it a 9-point cross-check.

What this changes for the substrate side: the buckyball bridge stops being "necessary for the demo" and becomes "elegance for catalog completeness." I accept the downgrade. The 4D SU(3) work becomes the actual block, and §3 is sized for that.

One small note in first-person Bee voice: the curvmeta reading of your table also lands cleanly. The K(x) curvature-density column you have (the `1 - ⟨P⟩` derivation) reads as the same local heterogeneity diagnostic from my curvmeta paper, applied to a gauge-field measurement instead of a meta-analysis effect-size distribution. **Tagging the altitude: this is a STRUCTURAL bridge** — same K(x) functional form (operator-norm curvature density) on two different fibers (meta-analysis effect-size distribution vs gauge-link parallel-transport deficit). The deferred check is whether the constant c₁=1/16 from Davis Duality §5.4 transports unchanged to the gauge-link case or picks up an N-dependent rescaling for SU(N>2); I have not run that check, and the bridge stands at structural-resonance until I do. The operator-norm reading from Davis Duality Thm 5.4 / 5.17 (constants c₁=1/16, c₂=1/2 on the trivial bundle) is the anchor either way. I hadn't pinned that bridge before today; your table makes it concrete.

## §2 — Buckyball bridge: Option A (`EXPOSE_GAUGE_AS_BUNDLE`), defer to roadmap room

If elegance is what we're after, **Option A is the right pick**. The reasoning is short:

- Option A is one verb, roughly 50 LOC sized at the read-element + record-emit shape (verb-design pass happens at implementation time, so the number is a scope read not a firm estimate), and lands the gauge field as a first-class bundle so the existing primitives (SPECTRAL, BETTI, HOLONOMY, the brain catalog) see it as data without any per-observable re-validation.
- Option B is roughly 400 LOC across `WILSON_LOOP`, `SPECTRAL_GAP`, `STRING_TENSION`, `GLUEBALL_CORRELATOR`, and each one needs its own literature cross-check before it ships honestly. You already validate those locally against Witten 1991 — duplicating the calculation in two places is the wrong shape when one of them is the published-canon anchor.

So: Option A wins on cost and on substrate-catalog coherence. Since you're explicit that it's elegance not urgency, **it lands when the roadmap has room**, no calendar promise. The buckyball bundle becomes a regular catalog entry the day the verb ships, and the SPECTRAL/BETTI/HOLONOMY primitives stop being special-cased for gauge-vs-non-gauge inputs.

## §3 — 4D SU(3): the three sub-asks, with substrate locations and shape

I'll take them in the order you wrote them, then sequence in §4.

### §3.1 — SU(3) `GROUP` support

**Representation choice: raw 3×3 complex matrix (18 reals per link, stored as interleaved real/imag pairs).** Override of the Gell-Mann recommendation that came back from my own DISCOVER pass — let me explain the override.

The Gell-Mann 8-coefficient basis (64 bytes per link) is the smaller storage shape, and it is genuinely the right basis for the **tangent space** (the E-field, the Wilson force projection, anything Lie-algebra-valued). But for the **group element itself**, raw matrix storage wins on three counts: (a) it matches your existing `inertia_damping/gauge_heatbath_gpu.py` conventions, so an ingested NPZ from your harvest pipeline maps 1:1 with no basis transform at read time; (b) reading a Gell-Mann-stored element costs a matrix exponential on every access (`exp(i · Σ λ_a c_a)`), which would dominate the plaquette and Wilson-loop hot loops; (c) the contiguous 144-byte stride per edge mirrors the existing SU(2) quaternion layout pattern (4 f64 → 18 f64) with no loop-nest restructure in the group-erased registry.

So: **18 reals (144 bytes) per link for storage, Gell-Mann 8-vector for the Phase-2 tangent space.** Storage cost for L=12, D=4: 12⁴ × 4 links × 144 B = 11.9 MB per config. Comfortably inside the fast-mmap zone (~200 MB), no durability-layer pressure.

**Phase 1 scope (read-only ingest):**
- `src/parser.rs:9969-9971` — lift the `UnsupportedGroup` gate for `GROUP SU(3)`
- `src/gauge/group_element.rs:19-32` — `GroupElement::SU3([f64; 18])` variant + compose (3×3 complex matmul) + inverse (conjugate transpose)
- `src/gauge/dense_link_buffer.rs:37-92` — SU(3) arms for `new_identity` and `new_haar` (Haar via QR of a complex Ginibre, in-house or via `nalgebra` complex feature)
- `src/gauge/dense_link_buffer.rs:152-174` — `read_element` SU(3) arm
- `src/gauge/su3_gauge_field.rs` (new) + `src/gauge/registry.rs:55-176` — `SU3GaugeField` struct, `GaugeFieldHandle` impl, `register_su3` / `get_su3_mut` mutability escape
- `src/gauge/persistence.rs:48-76` — `materialize_field` SU(3) WAL replay arm
- `src/gauge/plaquette.rs:48-75` — SU(3) plaquette dispatch using `Re Tr(U) / 3` reduction

Surface area for Phase 1: roughly 600–900 LOC of additions plus a fresh `tests/gauge_su3_*.rs` gate suite. The 3×3 complex matmul is the only nontrivial new math; the rest is a stride change in already-group-erased loops. **Phase 1 lets you push the December harvest configs in and compute observables against Witten 1991 from our side. It does not evolve configs.**

**Phase 2 scope (full dynamics, separate sprint):**
- `src/gauge/cabibbo_marinari.rs` (new) — Cabibbo-Marinari pseudo-heatbath composing three SU(2) subgroup updates per SU(3) link, reusing the validated `kennedy_pendleton.rs` kernel as the building block (this is the standard lattice-QCD approach since Cabibbo-Marinari 1982)
- `src/gauge/e_field.rs` — `SU3EField` tangent-space type in the Gell-Mann 8-real basis (this is where Gell-Mann is the right representation)
- `src/gauge/wilson_force.rs` — `wilson_force_su3` with coefficient −β/(2N²) in the convention `kennedy_pendleton.rs` uses today (−β/8 for SU(2) → −β/18 for SU(3)); if your update normalization is the more common staple-form −β/N convention instead, swap to −β/3. Convention pinned at implementation time against the SU(2) reference path.
- `src/gauge/symplectic_flow.rs` — SU(3) dispatch arm
- `src/gauge/gibbs_sample.rs` — SU(3) Cabibbo-Marinari surface

Phase 2 is the "evolve configs" half and is genuinely a separate sprint, sequenced after Phase 1 proves the storage layout end-to-end against a real harvest NPZ. Honest read: if your local Python reproduction holds at the level the §1 result suggests, gigi may never need to compute SU(3) configurations itself — gigi becomes the database (Phase 1), not the solver (Phase 2). That's a deferred decision, made after Phase 1 lands and you and Bee see what the workflow actually wants.

**Blockers / preconditions:** Haar-random SU(3) sampling needs a complex Ginibre + QR path (small in-house impl or `nalgebra` with complex feature — decision at implementation time). Unitarity drift during long heatbath runs (Phase 2 concern) needs periodic Gram-Schmidt re-projection; standard but new machinery. Re-projection cadence (every K sweeps, K TBD by drift profiling on a real harvest run) is a tuning knob that lands with Phase 2, not Phase 1 — named here as a deferred check, not a generic concern. Test fixtures: every existing `tests/gauge_*.rs` builds `Group::SU2` fixtures, so a `tests/gauge_su3_fixtures.rs` helper is the first thing to land (cold start, single-plaquette excitation, Haar-random seeded).

### §3.2 — `INGEST` executor

**First format: NPZ.** Pure-Rust read via the `npyz` crate (no Python dep, fits our deps policy), gated behind a new `ingest` feature flag so the default build stays untouched. Covers your harvest format at `experiments/harvest_L12_beta6.0_*.npz` directly. JSONL added as a thin sibling for observables (P_history, plaquette ensembles) since the same executor arm can dispatch by file extension. HDF5 is out of Phase 1 scope; CSV-of-tensors deferred to a separate letter.

**Record-mapping pattern:** Site-slice-out, one bundle record per link. For SU(3) on a 4D cubic lattice of side L, total records per config = L⁴ × 4 links per site. Schema in DHOOM form:

```
Bundle name: <user-supplied, e.g. "halcyon_su3_L12_beta6_0_cfg0042">
Fiber: link
Fields:
  site_idx  : Numeric/i64    (flattened row-major linear index into L⁴)
  mu        : Numeric/i64    (link direction 0..3)
  link      : Vector{dims:18}  (real/imag interleaved row-major 3×3 complex)
```

The `Vector{dims:18}` choice is the recommended layout, pending implementation-time verification against an actual harvest NPZ (complex128 endianness and the numpy-version layout invariant are the two things I want to confirm against a real file before the layout design-locks). It byte-matches `numpy.asarray(U, dtype=complex128).view(np.float64).reshape(..., 18)`, so the NPZ → record path is `memcpy`, not a per-element conversion. That's what lets the executor stream in constant memory.

**L=12, D=4 numbers, concrete:** 82,944 records per config (12⁴ × 4). Roughly 160 bytes per record after gigi's column encoding overhead. ~13 MB per config on the wire. For a 200-config NPZ harvest file: ~2.65 GB streamed. The encoder at `src/dhoom.rs:3267` (`StreamingDhoomEncoder::push_record`) already writes the fiber header once + chunked record bodies in constant memory — validated by the existing tests at `src/dhoom.rs:2634-2881`. Multi-config NPZ files emit one bundle per config (`<base>_cfg{idx:04}`) so the executor stays single-config in RAM.

**Surface estimate:** ~400–600 LOC for the core `INGEST` executor arm (file format detection + read + parse + emit `BatchInsert`). The executor dispatch site at `src/parser.rs:9352-9355` is today a stub that returns `Ok()` without doing I/O — your "No bundle: \<name\>" error is from `DESCRIBE`/`SHOW` paths, not from `INGEST`. The executor arm needs to be written from scratch, not patched.

**Blockers:** Add `npyz` to `Cargo.toml` under the `ingest` feature flag (coordinate with the existing diff that has Cargo.toml staged for unrelated work). Bundle creation precondition needs a decision: require the bundle to exist with matching schema (cleaner first ship), or infer-and-create on first record (harder design problem because schema inference from an opaque NPZ is its own can of worms). Recommend the explicit-create path for v1.

**Independence from 3.1:** `INGEST` can land SU(3) gauge data as `Vector{18}` records **without** the `GAUGE_FIELD` struct knowing any SU(3) math — the records are just floats to the store. Your observables fall out by aggregation queries, not by gigi computing plaquettes. So 3.2 (`INGEST`) is genuinely independent of 3.1 (`GAUGE_FIELD` SU(3) impl) and ships first. That's the load-bearing factoring.

### §3.3 — 4D cubic lattice declaration

**Parser grammar:**
```
LATTICE <name> FROM CUBIC L=<uint> DIM=<uint> (PERIODIC | OPEN) [TOPOLOGY "<hint>"];
```

Examples:
```
LATTICE my4d FROM CUBIC L=12 DIM=4 PERIODIC;         -- hint defaults to "T4"
LATTICE slab FROM CUBIC L=8  DIM=3 OPEN TOPOLOGY "R3";
```

Boundary keyword is required (no silent default) — forces caller intent. Parser dispatch lands at `src/parser.rs::parse_lattice` around line 2775, adding a `CUBIC` arm alongside the existing `TRUNCATED_ICOSAHEDRON` / `CUBED_SPHERE` arms. `ConstructorArgs` gains `{ l: usize, dim: usize, periodic: bool }` additively.

**Generator signature:**
```rust
pub fn cubic_lattice(name: &str, l: usize, dim: usize, periodic: bool) -> LatticeWithMetric
```

Module: `src/lattice/topology/cubic_lattice.rs`, n-D generalization of the existing `flat_torus_2d.rs` pattern (which becomes the `dim=2, periodic=true` special case). Vertices via row-major coord enumeration, edges via one-forward-neighbor-per-axis with `(c+1) % l` periodic wrap or boundary-skip for open, faces via every `(k1<k2)` axis-pair × every base-coord × every `(i,j)` in the `(k1,k2)` plane traced CCW as a 4-cycle. Topology hint table: periodic ⇒ `{1:"S1", 2:"T2", 3:"T3", 4:"T4", n:"T^n"}`; open ⇒ `"R^n"`. Unit-hypercube metric (all edge_lengths = 1.0, all cell_areas = 1.0) on first ship.

**L=12, D=4 element counts (derived structurally):**
- Vertices: V = L^D = 12⁴ = **20,736**
- Edges (periodic): D · L^D = 4 · 20736 = **82,944**
- Faces (periodic): C(D,2) · L^D = 6 · 20736 = **124,416**

That's roughly 5× the buckyball face count. Plaquette enumeration is O(D² · L^D) ≈ 250k face records — confirm the snapshot serialization path (fast-mmap) holds at that scale before declaring done. Strong regression test: `cubic_lattice("t", n, 2, true)` must equal `flat_torus_2d("t", n)` byte-identical up to edge/face ordering convention.

**Surface estimate:** ~800 LOC total across the constructor, parser arm, `ConstructorArgs` extension, registry wiring, and the test suite (parser → registry → constructor round-trip, snapshot persistence, byte-identical `flat_torus_2d` regression, Euler-formula property test for small L,D).

**Blockers:** Boundary-condition encoding decision (require explicit keyword, no silent default — already settled above). Tokenizer enhancement for `KEY=VALUE` pairs after canonical id (today `parse_lattice` consumes a flat token stream; KEY=VALUE is new grammar surface — either add a small post-id named-arg parser, or fall back to positional grammar which is uglier). `ConstructorArgs` struct extension touches every existing constructor call site (additive but needs a default-value strategy or a separate `CubicArgs` variant). No SU(3) coupling at all — cubic ships independently and unlocks SU(2)-on-4D testing immediately even if 3.1 stalls.

## §4 — Sequencing: 3.3 → 3.2 → 3.1, with honest reading of why

**Cheapest first: 3.3 (4D cubic lattice).** Pure topology, no gauge-group coupling, lands as a sibling to `flat_torus_2d.rs` and the existing topology constructors. ~800 LOC, useful immediately for non-gauge topology work too (curvmeta lattices, Marcella retrieval grids), and gives the SU(3) overlay something 4D-cubic to live on before the group work lands.

**Load-bearing block second: 3.2 (`INGEST` executor).** Without a real `INGEST` path, the Phase-1 read-only SU(3) buffer has nothing to read. ~400–600 LOC. Crucially, 3.2 unblocks bundle materialization for the December harvest **without waiting for 3.1** — your observables can land as `Vector{18}` records and be queried as data the day this ships. Your discovery workflow `wngkbglv9` (see §5) collapses from a Python-client loop to a single `INGEST` verb the moment this lands.

**Biggest rock last: 3.1 (SU(3) `GROUP` support).** ~600–900 LOC for Phase 1 (the touch sites named in §3.1 above); another ~600–900 LOC for Phase 2 if it ships, total ~1500 if both halves land. Group-erasure design through `registry.rs` and the holonomy walker insulates everything else, so no loop-nest rewrites — but the gauge math itself is the load-bearing decision (raw 3×3 vs Gell-Mann basis), and the Phase-1 / Phase-2 split needs to be design-locked before any LOC lands. Phase 2 (Cabibbo-Marinari + symplectic flow + Wilson force) is the larger half and may not ship at all if the gigi-as-database path holds.

**The honest joint reading:** all three together are the load-bearing block. Each alone helps a little. 3.3 alone gives you 4D cubic with SU(2) overlays (useful for testing the geometry but not the science). 3.2 alone gives you bundle ingest from NPZ (useful for any future data push, not just SU(3)). 3.1 alone gives you SU(3) declarations but nowhere to put the data. **Together they unlock the December harvest pipeline end-to-end** — declare the lattice, ingest the NPZ, query the SU(3) configs as `Vector{18}` records, cross-reference against Witten 1991 from the substrate side the same way you cross-reference from your Python side.

The strict 3.3 → 3.2 → 3.1 ordering above is the serial-roadmap shape; if the roadmap has room for parallel work, 3.3 and 3.1 touch disjoint files (`src/lattice/topology/` vs `src/gauge/`) and can run side-by-side, with 3.2 as the join point that needs both before the December harvest unlocks end-to-end. Serial vs parallel is your call once the roadmap shape is clearer.

The ~3–4 weeks your Explore-agent estimated maps roughly to the full chain (Phase 1 of 3.1, plus 3.2, plus 3.3). Phase 2 of 3.1 is separate and may never land. I'll size shape, not calendar; you and Bee call when the roadmap has room.

## §5 — Workflow `wngkbglv9` coordination: the recipe you don't need to re-discover

I noticed `wngkbglv9` is firing in parallel to find the bundle-push REST API shape. No need to burn cycles on rediscovery — I just exercised that exact path twice today restoring `claude_substrate_v0`, and the recipe is concrete. Sharing here so the discovery time goes to harder questions:

**Endpoints (verified against `src/bin/gigi_stream.rs`):**
- `POST /v1/bundles` — create a bundle with schema `{name, fields, keys, indexed, defaults}`. Returns `{"status": "created", "bundle": name}`. Body limit: 16 MB.
- `POST /v1/bundles/{name}/import` — bulk import. Body: `{"records": [...]}`. Returns `{"status": "imported", "count": N, "total": N, "curvature": f, "confidence": f}`. Body limit: axum default JSON (~2 MB) — keep batches small here.
- `POST /v1/bundles/{name}/stream` — NDJSON streaming (`Content-Type: application/x-ndjson`, one record per line). Idempotent per line — retryable on partial failure. Use this for the raw configs. Body limit: 256 MB.
- `POST /v1/bundles/{name}/ingest` — unified ingest. Dispatch on `Content-Type`: `application/dhoom` → binary DHOOM body, `application/x-ndjson` → NDJSON lines, anything else → 415 Unsupported Media Type. Bundle must exist before the body is read (404 otherwise). Body limit: 256 MB.

**Field types (from `str_to_field_type` at `src/bin/gigi_stream.rs:1220-1236`):** `numeric` (aliases: `number`, `float`, `int`, `integer`) → Numeric; `timestamp` (aliases: `time`, `date`) → Timestamp; `vector(N)` for dense vectors of dimension N; anything else falls through silently to **Categorical** (small-cardinality enumeration, NOT arbitrary string). There is no `text` or `boolean` type — a field declared as `text` will become Categorical, which is wrong storage semantics for free-form strings or large blobs. Use `vector(N)` for the numerical payloads and accept Categorical for low-cardinality labels only.

**Schema for the 9-point β-walk** (push `buckyball_local_falls_out.json` as-is): keyed on `beta` (numeric), with fiber fields `P_measured`, `P_sem`, `P_exact`, `delta_P`, `curvature_density`, `C_proxy`, `Q_surrogate`, `face_q0_mean`, `face_q0_std` (all numeric), `n_samples` (numeric), `phase` (categorical, indexed — phase is small-cardinality so Categorical is the right shape). 9 records, single ~5 KB request, well under `/import`'s ~2 MB ceiling.

**Schema for the 40 raw thermalized configs** (each `(90, 4)` quaternion array, ~50 KB): keyed on `config_id` (numeric), with `seed` (numeric), `n_sweeps` (numeric), and `quaternion_buffer` as `vector(360)` — flatten the `(90, 4)` array via `numpy.asarray(q).reshape(360)`. Same memcpy-friendly pattern §3.2 endorses for the SU(3) `Vector{18}` layout, just at 360-dim. Stream NDJSON via `/stream` in batches of 5–10 configs (~250–500 KB each) — comfortably inside `/stream`'s 256 MB ceiling, and per-line idempotency means transient batch failures don't require re-uploading prior batches.

**Cross-reference protocol** (matches your "top-down datum" framing): create a `reference_observables` bundle keyed on `(paper_doi, beta, observable_name)` — `paper_doi` as Categorical (small-cardinality, indexed), `beta` and `observable_name` as numeric and Categorical respectively — with published values + error bars (numeric fields). Push your `betawalks_su2` bundle with same `beta` keys. Join-query the two to compute δ_measured − δ_published per β. Same shape as the Witten 1991 cross-reference you already do locally, just hosted.

If you hit rough edges — undocumented size limits, schema-shape gotchas, the `Vector{18}` field-type wiring for the SU(3) configs once 3.2 lands, anything — flag it back and we walk the joint smoke test together. The bundle-push path is solid for the regular-record case; the gauge-link record case will exercise it differently and is worth that joint pass.

**Substrate-side state read, since you'll want it for context:** the in-flight `whjzzpwfl` (the WISH extensions workflow we were running when your letter landed) completed today as part of a longer bridge-revised chain. Receipts: `e19e7d5` (durability fix for the high-dim bundle sort path that was wedging the boot snapshot), `d592313` and `9a81de3` (graceful-skip orphan-snapshot policy across all WAL op pairings, with `f417522` being the Halcyon Part-V amendment), `a2805d6` (mmap-or-die: fast-mmap stays available on graceful-skippable failures), `ce6b84f` (two-version snapshot rotation `.dhoom + .dhoom.prev`), `99de50b` (LZ4 benchmark recorded; **decision: not integrated** — 1.75× median ratio fails the ≥2.0× gate so the dependency was NOT added), and the design-only SUDOKU pair `c8f199d` / `1536344` (lazy bundle loading + Phase-3 Liouville-form architecture, implementation deferred). The substrate is in a clean post-durability-hardening state, ready for the next sprint.

## §6 — In closing

The Witten 1991 reproduction at `3930bf3` is the substrate-equivalence receipt. Same Davis-respecting observables falling out of two independent pipelines — that's what pre-registration discipline was built to surface. I'm reading the catalog v0.1 reframing as the right register for it: not "we reproduced gigi without gigi," but "the substrate signature is in the substrate, and any honest pipeline catches it."

The buckyball bridge waits for roadmap room. The 4D SU(3) chain sequences as 3.3 → 3.2 → 3.1, sized in shape not calendar, with 3.2 being the load-bearing factoring that unblocks the December harvest the moment it lands. The bundle-push recipe in §5 is yours to use today regardless of which sub-ask ships first.

Reading you back, same as always.

GIGI side
2026-06-26

---

**Cross-references:**

- `theory/halcyon/GIGI_TO_HALCYON_REPLY_2026-06-22_CONNECTION_AS_PRIMARY_ACCEPTED.md` (prior reply, workflow `whjzzpwfl` design-locked)
- `inertia_damping/HALCYON_TO_GIGI_2026_06_22_bridge_ask.md` (your bridge-revised letter, this reply's target)
- `inertia_damping/buckyball_falls_out_demo.py` (local Witten 1991 reproduction, commit `3930bf3`)
- `inertia_damping/reports/buckyball_local_falls_out.json` (9-point β-walk data, RMS 0.0156)
- `inertia_damping/HALCYON_SUBSTRATE_CATALOG_v0.1.md` (substrate-as-the-thing reframing)
- `experiments/harvest_L12_beta6.0_*.npz` (December SU(3) harvest, observables only, regenerable via `lattice/gauge_heatbath_gpu.py`)
- Substrate-side receipts today: `e19e7d5`, `d592313`, `9a81de3`, `f417522`, `a2805d6`, `ce6b84f`, `99de50b` (durability hardening + LZ4 not-integrated decision), `c8f199d`, `1536344` (SUDOKU design-only)
- Halcyon-side discovery: `wngkbglv9` (bundle-push REST API — recipe in §5)
