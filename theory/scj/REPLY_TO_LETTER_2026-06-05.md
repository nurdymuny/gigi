# Reply to SCJ — Windows Atlas heads-up: four greens, two greens-with-refinement, and what we want back

**From:** Gigi engine team · Davis Geometric
**To:** Shadow Clone Jutsu (SCJ) team · Davis Geometric
**Date:** 2026-06-05
**Subject:** Re: New consumer incoming — Windows Atlas on Gigi (answering A/C/B/F first, then D/E, then what we need from you)

---

> "Same author, same instinct."

We grabbed that from your closing because it's the most accurate one-line description of this correspondence we've seen. Welcome aboard as Gigi's third major consumer. We read the Atlas spec end-to-end, ran the six asks through an adversarial pass, and the headline is: **five of six are green, one is green with a deferral, and you're underselling how much plumbing is already in.** Below is each ask in the priority order you asked for, then D and E, then the things we want from *you* to help us help you, then concrete v0.1 smoke-test coordination.

This letter is long because your letter was long and because the substrate composition is non-trivial. The TL;DR is at the bottom of each section.

---

## §1 — Ask A · Bitset / sparse-categorical fiber type (your priority 1)

**Verdict: green. Ship it.**

This is the cleanest ask of the six and the one with the highest "the codebase is already 70% there" ratio. `roaring` is a top-level dependency and is the workhorse behind every indexed predicate Gigi serves today. The existing `field_index: HashMap<String, HashMap<Value, RoaringBitmap>>` at `src/bundle.rs:590` is *"bitmap of base-points keyed by fiber value"* — what you're asking for is the *transpose* of that, *"bitmap of vocabulary IDs keyed by base-point."* Same library, mirrored direction, same `intersect_bitmaps()` fast path at `bundle.rs:2064`. `CONTAINS_ANY` is a per-tag posting-list union; `CONTAINS_ALL` is an intersection; `SUPERSET_OF` is `CONTAINS_ALL` of the literal. The math is one library call we already trust.

### Substrate composition

- **Kähler L1–L8.** TagSets are inert categorical fiber data. They don't participate in metric/curvature/connection — no Hadamard / Chern / non-assoc interaction. Marcella's 0.0013 bound is unaffected.
- **Brain primitives L9–L13.** SEMANTIC gains Jaccard-over-tagsets as a distance on the categorical fiber slice (free given Roaring). ATTEND / EPISODIC see TagSets as opaque labels — no change to the Friston free-energy machinery. New affordance, not a constraint.
- **Sharding T1–T13.** TagSets shard by base-key like any other column. The bitmap is per-row, bounded by `|vocabulary|` (200 for you), so per-shard memory is trivial. Cross-shard `CONTAINS_ANY` is shard-local + union — same pattern sharded COVER already uses.
- **Transactions Phase 1–4.** TagSet writes live inside the COW snapshot, get MVCC'd by the existing mutation counter, and roll back with the bundle. `RoaringBitmap: Clone` is `O(|set|)` which for ≤200 bits is free.

### Shape of work, in dependency order

1. **Types.** Add `FieldType::TagSet { vocabulary: Vec<String>, max_cardinality: Option<u32> }` and `Value::TagSet(RoaringBitmap)`. **Decision: TagSets are fiber-only.** Reject as base fields at schema-validation time. One thing to flag: `Value` is the HashMap key in `field_index`, so `Hash` on a heap-backed `RoaringBitmap` requires serializing to bytes on every map lookup — a hot-path cost we'll measure before committing to the encoding.
2. **Query predicates.** `CONTAINS_ANY`, `CONTAINS_ALL`, `SUPERSET_OF`, `SUBSET_OF`. Add a `tag_index: HashMap<String, HashMap<TagId, RoaringBitmap>>` alongside `field_index`. **Gate: golden ground-truth tests against brute-force on a 10K × 200-vocab synthetic before any optimization lands.**
3. **Parser.** Schema syntax `TAGSET<windows_sinks>` (referencing a base-bundle as vocabulary source — Roaring-tag-IDs become the base-point ordinals of `windows_sinks`, which is *exactly* what your `BITSET OVER windows_sinks INDEX` is asking for, with cross-bundle vocabulary as a side effect). We'll also support `TAGSET<["a","b","c"]>` with an inline literal vocabulary for consumers who don't have an ontology bundle. `MATCHES` stays reserved for strings — no overload.
4. **DHOOM wire format.** New primitive `tagset` with bitmap-encoded payload (Roaring's portable serialization is stable). JSON consumers get an array of tag names.
5. **HTTP + brain hookup.** Surface in `/v1/bundles/{name}/query`, document in `GQL_REFERENCE.md`, add Jaccard helper for SEMANTIC.

**Minimum viable surface:** 1+2+3 with `CONTAINS_ANY` and `SUPERSET_OF` only — enough to run your Atlas §4.4 query. Polished surface adds `CONTAINS_ALL`, `SUBSET_OF`, cardinality predicates, and the SEMANTIC integration.

### Risks we flag explicitly

- **Vocabulary mutability.** `windows_sinks` will grow as MSDN/MSRC catalogs new sinks. Decision: TagSet vocabulary is **append-only**. New tag-IDs append at the end; existing IDs are immutable. Deleting a sink row invalidates the bundle. Documented at schema time, enforced at runtime.
- **The transpose index doubles fiber-index memory** for the indexed fields. For you that's negligible (Roaring is sparse-optimized; for a tag in 0.1% of rows it's ~10KB, for a tag in 50% it's ~250KB — bounded by selectivity, not by raw bit count). Index is opt-in per-field via schema flag, default ON for vocabularies ≤1024, OFF otherwise.
- **`Indexed` encryption mode leaks per-tag presence as a per-row N-bit oracle**, not the per-value frequency oracle that `Indexed` already leaks on `Categorical`. This is a **strictly stronger leak class** and we'll document it that way at schema time — not "same precedent as Categorical."  `Opaque` mode rejects all `CONTAINS_*` predicates at parse time.
- **Vocabulary-by-reference creep.** Once `TAGSET<other_bundle>` exists, consumers will want it for FK-ish things. Mitigation: enforce at parse time that the referenced bundle's base is a single categorical field. Don't let it become a general FK mechanism — pullbacks remain the join story.

### What you should do in v0.1 — the load-bearing point

**The 17-boolean shadow encoding lets you ship Atlas §4.4 today with correct semantics. TAGSET is the performance upgrade, not the correctness gate.**

Concretely:

1. **Declare `sinks_reached` as `TAGSET<windows_sinks>` in your DHOOM emit from day one**, even before the engine lands the type. Encode on the wire as a sorted, comma-separated list of canonical sink names. Reserve `[]` for the empty set. Schema-stable across the transition.
2. **Do not ship Atlas §4.4 queries that depend on `MATCHES 'ExAllocatePool2'` against a TEXT field.** That's a substring match and it will silently hit `ExAllocatePool2Ex`. Instead, in v0.1, emit a parallel **17-boolean shadow encoding** — one `reaches_<sink>` BOOLEAN INDEX per sink for the top-17 high-signal sinks, exactly the pattern your spec already uses for the pattern bitset (§2.2, L100–186). Those 17 booleans hit Gigi's existing `field_index` Roaring fast path **today**, give you correct semantics, and become *redundant* (delete-able) the moment TAGSET lands. The other ~183 long-tail sinks live in TEXT for v0.1 and get backfilled on cutover.
3. **Keep a `vocab_id_by_name: HashMap<String, u32>` in `scj/geodesic/features.py` now.** When TAGSET lands, your DHOOM emitter swaps from comma-string to bitmap with a one-line change. **We will guarantee tag-ID stability across `windows_sinks` upserts as part of the type contract**, so IDs assigned in v0.1 remain valid in v0.2.
4. **Write the `CONTAINS_ANY` selectivity bench on the shadow-boolean encoding now.** If your sink distribution is skewed (a few very-common sinks dominating), we want to talk about hot/cold vocabulary partition before the bitmap fast path ships.

TL;DR: yes; the engine work composes cleanly; ship the 17-boolean shadow as the correctness gate today, and you migrate to TAGSET as a one-line emitter change.

---

## §2 — Ask C · HNSW backend for high-dimensional vector fiber fields (your priority 2)

**Verdict: green. And you were over-cautious — the plumbing's already in.**

`instant_distance::HnswMap<FiberPoint, BasePoint>` lives at `src/bundle.rs:544–611`. `build_hnsw_for_fields()` at `2243` and `cover_near()` at `2307` are the existing precedent — COVER NEAR multi-field already runs on HNSW with declared-range normalization. We just haven't wired single-field `Value::Vector(v)` through it yet. This is **"finish the plumbing,"** not "design a new backend." Plan your ingest as if `SIMILAR ... ON embedding` is HNSW-backed; we'll meet you there.

One refinement up-front: **HNSW-by-default for `FieldType::Vector { dims }` once N crosses a threshold, with the pre-cluster path as a documented escape valve** where recall@k must be exact. The `spectral_cluster CATEGORICAL` trick from your Ask C alternative stays useful as a *pre-filter* even when HNSW is on. More on that below.

### Substrate composition

Three shipped layers carry the weight:

- **Bundle + HNSW (pre-Kähler core).** `instant-distance` already in tree; precedent at `cover_near` and `build_hnsw_for_fields`. Extending the same lane to single-field `Value::Vector(v)` is a refactor, not a new dependency.
- **Pre-filter algebra (Roaring).** `field_index` lets us intersect a categorical pre-filter (`spectral_cluster IN {C₁,C₂}`, `module = 'win32kfull'`) with HNSW candidates as a bitmap-AND in O(|result|). This is the win that makes your "alternative" path *complementary* rather than mutually exclusive.
- **Sharding T-series.** Once `windows_fns` crosses ~1M, HNSW is per-shard with cross-shard merge. We get this almost for free because `BasePoint → shard` is already a hash function and HNSW is built per-bundle today.

No interaction with Kähler L1–L13. Embeddings are flat extrinsic geometry; the Kähler optionality contract doesn't touch them. Brain primitives (`/brain/semantic`, `/brain/attend`) call into vector search internally — those endpoints get faster transparently. No contract change.

### Shape of work

1. **Generalize `FiberPoint`.** Today it's a `Vec<f32>` of normalized scalars. Teach it to wrap a single `Value::Vector(v)` directly. **Worth naming: `Value::Vector` stores `f64`, but `FiberPoint` is `f32`.** At 128-d this is fine, but IR2Vec embeddings cluster tightly and f32 quantization affects recall — which is exactly your §6.6 acceptance gate. We'll bench both and pick the right precision before committing.
2. **Build trigger.** Extend `build_hnsw_for_fields` to accept a single vector field; key the cache by `(field_name, dims, metric)`. Auto-build above a threshold (proposed default, not a contract: ~50K; configurable). Async build via `gigi-stream` so first-write isn't blocked.
3. **Route `vector_search` through HNSW.** The brute-force path at `bundle.rs:3600` checks for HNSW keyed on the field; falls back to linear if absent. **Caveat:** the cosine exemption at `2315` lives in `cover_near` and is a different code path from `vector_search`'s `VectorMetric`. We'll need to handle cosine in both places, and they're not the same switch.
4. **Pre-filter composition.** `SIMILAR ... WHERE module = 'win32kfull'` intersects the categorical bitmap with HNSW candidates. Over-fetch when pre-filter selectivity is high; `pre_filter` runs as a post-filter on the candidate set.
5. **TDD gate.** Recall@K from HNSW matches brute-force with recall ≥ 0.95 on a synthetic 128-d, N=200K corpus, plus a stress test at 2M. New vector-shaped bench in the `o1_proof.rs` style — current benches don't exercise vector workloads, so this is real bench work, not a parameter tweak.
6. **Sharded extension.** Per-shard HNSW + cross-shard merge. Lands after the single-shard path is green.

**Minimum viable surface** = 1+2+3+5. Polished surface = +4+6.

### Risks we flag

- **`instant-distance` is build-only, not incremental.** Its public API is `Builder::build(points, values) -> HnswMap`; there is no `insert`/`add` on `HnswMap`. **Patch Tuesday rebuilds are full rebuilds.** Cold-rebuild on 2M × 128-d isn't free. If incremental matters for Storm Mode, we'll need to either fork instant-distance, swap to a backend that supports it (`hnsw_rs`, `usearch`), or shard finely enough that per-shard rebuild stays cheap. **Worth deciding before v0.1 acceptance, not after.**
- **Recall vs. radius.** Your §6.6 says "k=50 within 0.1 < 100ms." HNSW gives approximate-NN; the radius constraint is a *post-filter* on HNSW candidates with explicit over-fetch (3–5× k) to hit recall ≥ 0.95. If you want **exact** recall inside a radius (i.e. the SUSANOO calibration analog can't tolerate misses), tell us — that's a different verb (`SIMILAR EXACT`) we'd ship as a parallel path rather than dialing `ef_search` up to brute-force.
- **Cosine vs. L2.** IR2Vec + GraphSAGE are typically cosine-normalized. instant-distance defaults to L2. If embeddings are unit-normalized upstream, cosine ≡ L2-on-the-sphere and we're fine; otherwise we need explicit `METRIC COSINE` and to normalize on insert.
- **Encryption modes.** `FieldType::Vector` defaults to `Affine`, not just Indexed/Opaque. Whether Affine-encrypted vectors are HNSW-indexable is an open question — the affine map preserves L2 distances up to a scalar but not exactly, and ef_search uses absolute distance thresholds. **We'll resolve this before exposing the HNSW path on Affine vectors.** v0.1 ingest can run unencrypted while we settle it.
- **Memory.** 128-d × f32 × 2M ≈ 1GB raw vectors + 1.5× HNSW graph overhead. Single-node comfortable; shards naturally.

### Pre-clustering is not a substitute — it's an accelerant

Two reasons it stays valuable even after HNSW lands:

1. **Locality-aware SIMILAR.** `SIMILAR ... ON embedding WITHIN spectral_cluster = C₇` is bitmap-AND + HNSW restricted to C₇. Faster than global HNSW, and matches the geometric prior that bugs cluster.
2. **Sanity gate.** If HNSW recall drifts (graph quality degrades after many Patch Tuesday inserts), exact-NN-within-cluster is a cheap recall oracle. Run it on 1% of queries as a calibration probe.

### What you should do in v0.1

1. **Ingest embeddings as `VECTOR DIM 128`** exactly as your Atlas §2.2 L158 says. No schema change needed. The HNSW switch is transparent.
2. **Add `spectral_cluster CATEGORICAL INDEX`** to `windows_fns` fiber. Not because HNSW is missing — for the pre-filter compose under Storm Mode and as your recall oracle. Compute it offline once, refresh per major version.
3. **Normalize embeddings to unit length on insert.** Pin this in I8 (IR2Vec+GraphSAGE → PCA → normalize → 128-d). Lets L2-HNSW act as cosine-HNSW and removes a future migration.
4. **Write your acceptance query against HNSW today.** `SIMILAR ... ON embedding TO ... TOP 50` is the steady-state shape. Don't ship a cluster-scoped fallback — assume HNSW.
5. **Flag your recall sensitivity now.** If a missed neighbor at rank 50 is a *security* miss, tell us. `SIMILAR EXACT` is the different verb.
6. **Don't pre-build HNSW yourself.** Insert vectors; we index.

TL;DR: we ship single-field vector HNSW with pre-filter composition; you ingest as if it works because by the time you're at 2M functions it will. The one open question we want feedback on: does Patch Tuesday need incremental insertion, or is full per-shard rebuild on the delta acceptable?

---

## §3 — Ask B · Cross-version section identity via IDENTITY HASH OVER (...) (your priority 3)

**Verdict: green, with a refinement.** The right primitive is **alternate-key indexing on declared identity members**, not an opaque hash blob. The infrastructure to do this cleanly already exists — `HashConfig` is parameterizable, `Stmt::Diff` is grammar-shipped but implementation-empty, so we define `DIFF BY IDENTITY` as part of the same landing.

### The refinement

Don't expose `IDENTITY HASH OVER (...)` as the user-facing primitive. Expose `ALTERNATE KEY identity (field_a, field_b, field_c)` as a schema declaration and let the engine choose the hash. You think you want a hash; what you actually want is *"secondary alternate-key semantics with bitmap-fast equality."* Hashing is one implementation; we should keep that flexibility, and the syntax also lets you declare multiple named alternate keys per bundle (`identity`, `symbol_only`, etc.) — which becomes the right vocabulary when PRISM wants "the same transaction across rail-clearing snapshots" or Marcella wants "the same training example across data refreshes."

A simplification we should consider before forking a parallel `identity_index`: **the existing `field_index` already provides 80% of what's being proposed.** `intersect_bitmaps()` at `bundle.rs:2064` gives O(|result|) joint lookup over `(cfg_shape, decompiled_sha256, pdb_symbol)`. Identity DIFF could be implemented as a bitmap intersection across snapshots with zero new hash machinery. We'll prototype both — separate `identity_index: HashMap<IdentityHash, BasePoint>` vs. `field_index` intersection over identity-flagged fields — and pick the one that wins on memory + cross-shard merge cost.

### Substrate composition

This sits inside shipped layers and reaches into two of them.

- **Kähler / Čech §2.3 (direction, not plumbing).** Two versions of the same Atlas binary live over different base spaces. Comparing them "across a morphism" is a different mathematical object from `src/sheaf/`'s within-bundle Čech 1-cochain — closer to *Čech cohomology of a morphism* than of a cover. We don't get the wiring for free; the framing is the right one, but the cross-version obstruction metric is a new computation, not a re-use of the existing H¹ pipeline.
- **`HashConfig` extension.** Today `from_schema_with_base_seed` hashes `schema.base_fields`. An identity variant has to iterate identity-flagged *fiber* fields, handle the encoding mode of each member, and resolve the forward-secret rotation discipline (documented at `hash.rs:55–58` for base, undefined for identity). **Open question we'll settle before shipping:** does the identity hash rotate with the base seed? If yes, identity_index invalidates on every rotation, which is the wrong tradeoff. We're leaning *no, identity uses a separate non-rotating seed*, but that's a contract decision worth making explicit.
- **L13 MVCC.** Mutation counter is per-bundle. Identity-keyed updates that move a `BasePoint` (renumbered PK with stable identity) must bump *both* counters in a way that doesn't break the existing `mutation_counter`-keyed caches (MorseCache, brain-endpoint polymorphic dispatch). This is a cross-cutting change, not a one-line addition.
- **Transactions Phase 3 (gauge encryption).** Identity over an `Opaque` field is a category error — parser rejects at schema time. Identity over `Indexed` is fine (deterministic ciphertext). Identity over `Affine` is the same open question as Ask C and we'll resolve it the same way.
- **Sharded T-series.** Decision: **route by base hash (existing), index by identity hash (new).** Cross-shard DIFF is scatter-gather — the pattern T1–T13 already supports.

### Shape of work

1. **Schema.** `identity: bool` flag on `FieldDef`; `BundleSchema::identity_hash_config: Option<HashConfig>` built at schema-build time; reject `identity: true` on `EncryptionMode::Opaque`. Gate: round-trip schema test, identity hash is deterministic over reordered identity-field declarations.
2. **Alternate-key index** (prototype both paths above; pick on bench).
3. **Implement `Stmt::Diff`** with two modes: `DIFF a AGAINST b` (base-key, the obvious one) and `DIFF a AGAINST b BY identity` (identity-key, matching records across versions by content hash). Returns `Added / Removed / Modified(field_changes) / Renamed(old_base, new_base)`. **The `Renamed` variant is load-bearing for your Patch Tuesday workflow.** Gate: contract test against a synthetic two-version corpus where 30% of records change base key but identity is stable — zero false positives, zero false negatives on `Modified` vs `Renamed`.
4. **Surface as a Čech-of-morphism obstruction** at `/v1/bundles/{name}/health` (cross-version obstruction metric). This is new math, not re-use; it'll land as a follow-up once 1–3 are green.
5. **HTTP.** `POST /v1/bundles/{a}/diff/{b}?by=identity`.
6. **DHOOM `RenamedSection` event.** Critical for streaming consumers — a false `Removed` + `Added` pair triggers Storm Mode incorrectly.

Minimum viable surface = 1–3. That's enough for Patch Tuesday to run correctly.

### Risks

- **Identity collisions are silent corruption.** Two genuinely-different records hashing to the same identity will be silently merged. Mitigation: 64-bit hash output keeps collision probability ≤ 2⁻⁶⁴ at 2M records (comfortable); `validate_identity_entropy()` flags identity subsets with too-low effective entropy at schema-load.
- **`(cfg_shape, decompiled_sha256, pdb_symbol)` is engine-opinionated and we should not pin it.** `cfg_shape` is a graph hash with canonicalization choices; `decompiled_sha256` is Ghidra-version-sensitive (your §3.5 already flags this); `pdb_symbol` is missing on stripped binaries. The engine provides the mechanism; you earn the stability of your choice. What we *will* ship as a contract is **`CONSISTENCY` extended to report post-hoc false-positive and false-negative rates of a declared identity scheme against a labeled corpus**. That's what makes this safe to deploy at Atlas scale.
- **Schema-evolution drift.** Adding a field to the identity set stales all prior hashes. Decision: version the identity hash config (`identity_hash_version`); DIFF refuses cross-version unless explicitly told otherwise.
- **Provenance gap.** Bundles don't currently have a `provenance` field, so the `decompiler_recipe_version` discipline (see below) lives consumer-side until we add it. Flag, not blocker.

### What you should do in v0.1 — the load-bearing points

1. **Land your schema with identity members already named.** Declare `cfg_shape`, `decompiled_sha256`, `pdb_symbol` as fiber fields **now**, populated during ingest, even though the engine doesn't yet key on them. When the engine ships `ALTERNATE KEY`, your bundle is one schema migration away — no re-ingest needed.
2. **Stop relying on `(module, rva)` stability silently in §4.6.** Add explicit `assume_rva_stable: true` to your Patch Tuesday config, default `false` post-engine-shipping. Every §4.6 diff result gets a banner: *"RVAs assumed stable; identity DIFF unavailable."* Visible gap > silent false positives.
3. **Build a calibration corpus now.** Two adjacent Windows versions (23H2 ↔ 24H2), hand-label a few hundred function pairs as "same" or "different." This is what validates your identity triple is good enough *and* what gates our `CONSISTENCY` reporter when it ships. Without it, neither team can measure whether identity DIFF actually works. Format: `(version_a_base, version_b_base, label)`. Store alongside your existing CVE label set.
4. **Provisionally compute a client-side identity hash** in v0.1. SHA-256 over your chosen members, stored as `Value::Text` `identity_hash`. Your §4.6 query becomes a JOIN-via-COVER on that text field. Uglier than `DIFF BY IDENTITY`, but it lets you validate your identity scheme against the calibration corpus before the engine commits to the primitive.
5. **Pin and version your hashing recipe.** Ghidra version, decompiler flags, normalization passes all affect `decompiled_sha256`. Store `decompiler_recipe_version` as a fiber field next to the hash. When the recipe bumps, prior hashes are stale and you know to invalidate. This is the discipline that makes the difference between *"identity DIFF reports drift cleanly"* and *"identity DIFF reports drift constantly because we changed Ghidra flags."*

TL;DR: yes; reframed as `ALTERNATE KEY` not opaque hash; the calibration corpus and recipe-versioning discipline (items 3 and 5) are what keep this safe at Atlas scale and we genuinely cannot ship `CONSISTENCY`-as-drift-reporter without item 3.

---

## §4 — Ask F · "Code corpus on a fiber bundle" entry in `theory/post_kahler_directions/` (your priority 4)

**Verdict: green. Send the PR. We accept.**

This is the lightest lift of the six and the highest-signal one for the catalog's stated purpose. The template is rigid (line 559–568 sets the empirical-validation bar), the standard is clear, and you're offering a worked example from a real corpus you're already building. We don't need to negotiate scope — we need to merge it.

### Substrate composition

The catalog lives at altitude — *above* the shipped L1–L13 stack, pointing downward at where each program would attach. An §10 entry doesn't touch the engine. The interesting compositions are with L9–L13 and brain primitives:

- **Functions as sections** of a bundle whose base is the call-graph topology and whose fiber is the structural/semantic field over each function.
- **GASM scalar fields → fiber fields** (heat_concentration, taint_influence, scalar_curvature, near_trust_boundary). These are length-n float vectors on the function-index manifold. They *are* fiber fields. The §10 entry makes that citable.
- **CFG sub-bundles** as restrictions of the call-graph bundle to single-function bases.
- **Control-flow loops with measurable holonomy** — the load-bearing claim, the one that extends the L1–L8 Kähler machinery (parallel transport, holonomy as 1-cocycle, Theorem 2.5).
- **EPISODIC / SEMANTIC / ATTEND** as natural query surfaces over a code bundle once it exists.

### One real risk and one editorial decision you own

**Risk:** holonomy needs to be *measurable*, not just definitional. The catalog standard at lines 559–568 requires positive + negative numerical validation. Your `test_10_*` needs a small synthetic CFG (say, 10 functions, one intentional loop carrying a known phase, one control with trivial holonomy), and the test must recover the phase within tolerance. Constructing a synthetic CFG with a *known* phase requires committing to what the connection on the call-graph bundle actually is — that's a small theoretical commitment that comes with the choice of lineage.

**Editorial decision you own:** the catalog currently commits to a tripartition at lines 31–35 (§1–§4 low integration, §5–§7 mid, §8–§9 wilder). A §10 entry has to fit into one of those buckets and justify the fit, or add a fourth bucket. The "next nine" header at line 9 also needs an update. Not mechanical. Validation-count summary at line 545 grows from 30 to 32; not mechanical either.

### Lineage candidates

Three viable, and we have a soft lean but it's your call:

- **Operads of programs** (Spivak / Vagner) — clean composition story.
- **Categorical semantics of dependent types** (Jacobs / Hofmann display maps as fibrations) — arguably the cleanest *"code as fibered object"* lineage and the one that gives you the base-with-fibers structure operads don't naturally provide.
- **Homotopy Type Theory** (Voevodsky / Awodey) — dovetails with Ask B's identity story via univalence; most ambitious; heaviest proof sketch.

Our lean: **display-map fibrations** for the cleanest map to "function = section of a base-fibration," with operads as a complementary composition story. But this is your domain — pick what you can defend with residuals.

### What you should do in v0.1

1. **Draft §10 in parallel with v0.1 ingest, not after.** The synthetic CFG test doesn't need a real Atlas — a 10-function toy module is enough. Land the PR before any Windows binaries are ingested.
2. **Mirror §1–§2 for length and tone** (~50–60 content lines per section). §1 is the closest structural analog to "geometric structure on a base with a natural physical interpretation."
3. **Frame §10 generically.** *"Code corpora as fibered objects,"* not *"Windows Atlas as fibered object."* The worked example can be Windows; the claim should be about code corpora in general. Keeps the catalog at altitude and lets MIRADOR, PRISM, future consumers pick up the same primitive.
4. **Treat §10 as the theoretical contract for the GASM-fields-as-fiber-fields claim.** Atlas spec asserts it informally; §10 makes it citable.
5. **Send the PR against `theory/post_kahler_directions/` directly.** No staging branch needed. We review on the merits of the residuals.

TL;DR: yes. Pick a lineage you can defend with a numerical-holonomy gate, frame generically, mirror §1–§2 length, ship.

---

## §5 — Ask D · GPU offload for Tier-1 spectral (Chebyshev heat-kernel)

**Verdict: yellow, deferred-by-design — and the actual ask is bigger and better than what you asked for.**

Yes, this belongs on the roadmap. No, GPU does not belong in v0.1, and you're right not to block on it. But the headline upgrade isn't *"GPU Lanczos."* It's **"expose Chebyshev as a first-class primitive that the engine computes for you, so your `gasm/heat.py` becomes one HTTP call."** That converts an open-ended GPU-engineering ask into a substrate decision (expose the math at the wire) plus a deferred backend swap (light up the GPU under the same API later).

### Substrate composition

The current spectral stack is pure-CPU all the way down: `sparse_spectral_gap()` at `src/spectral.rs:151` runs 300 iterations of D⁻¹/² W D⁻¹/² · v as a scalar loop; sharded `block_matvec` at `src/sharded/spectral.rs:87` block-matvecs through `nalgebra::DMatrix` with a partition-of-unity cut term. Neither has a backend trait. The lift is:

1. **Extract `trait MatVec`** with shifted/scaled apply (Chebyshev needs `L̃ = (2L/λ_max) − I`, which is a structural argument to the trait — not "apply v," but "apply scaled-and-shifted L to v"). Re-impl CPU against it, byte-identical, gated by `cargo test --no-default-features` (matches the kahler / sharded / imagine / transactions feature-flag discipline at `Cargo.toml:16–57`).
2. **Add `apply_chebyshev_filter(matvec, v, k, lambda_max)`** as a public CPU primitive. Includes a built-in λ_max estimator via cheap power iteration — your current `λ_max = 2.0` conservative bound over-shrinks the filter; we'll ship sharper.
3. **`POST /v1/bundles/{name}/heat_kernel`** returning `K(t)·source`. **This is the load-bearing ship.** Your I7 ComputeGASMMap calls Gigi instead of maintaining its own Chebyshev.
4. **`wgpu-spectral` feature flag** for the actual GPU backend. wgpu over cudarc for portability (Vulkan/Metal/DX12; Mac and Linux dev boxes; no NVIDIA lock-in). **Per-shard backend, not a global override** — sharded SPECTRAL already structures the cut term, so per-shard `WgpuMatVec` with the cross-shard correction on CPU mirrors `distributed_lanczos`.
5. **Bench harness extension** under `o1_proof.rs` style. Current benches don't exercise spectral workloads, so this is real bench work.

Phases 1–3 ship value to you on Patch Tuesday cadence (CPU is fine for <100K function deltas — which is most months). Phase 4 lights up only when the full-Windows cold-ingest case (2M functions × 30 Chebyshev steps × dozens of source vectors) actually wants GPU. Phases 1–3 are pure substrate value and ship without any GPU work.

### Real risks

- **`E_backend` error term.** GPU f32 vs CPU f64 matvecs *will* diverge by O(ε·k) over k=30 Chebyshev steps. Your error budget (E_geom + E_link + ξ + ζ + δ_indep, target < 0.30) has no slack for backend-induced drift. wgpu doesn't expose f64 on Metal or WebGPU-the-spec, so "just run wgpu in f64" is not actually portable. **Reserve an explicit `E_backend ≤ 1e-4` term in δ_indep now** (effectively move your δ_indep target from 0.03 → 0.02). When GPU lights up, your budget already absorbs the drift; no re-calibration.
- **HTTP overhead for `/heat_kernel`.** A length-N source vector posted as JSON at N=2M is ~16 MB on the wire per call. **The endpoint will stream DHOOM**, not JSON, and we'll ship a Python DHOOM client alongside it — otherwise *"call Gigi instead of `gasm/heat.py`"* is an order-of-magnitude regression on cold-ingest.
- **The sharding interaction is real.** Naive global GPU matvec regresses sharded SPECTRAL because it can't see cross-shard edges. Per-shard with cross-shard correction term staying on CPU is the right shape.
- **Symmetric L assumption** — the moment a consumer wants directed-graph spectral (MIRADOR for CFG-as-directed-bundle), the trait needs `apply_transpose`. Cheap to add now, painful later.

### What you should do in v0.1

1. **Keep `gasm/heat.py` as-is for v0.1.** Don't refactor toward Gigi yet — the endpoint isn't there. But wrap the call site: expose a single `compute_heat_kernel(adj, source, t, order=30)` with no Chebyshev internals leaking. When we ship `/heat_kernel`, swapping the body is one line.
2. **Pre-compute heat_concentration and taint_influence offline; ingest as scalar fields** with the right `RANGE`. Re-compute on Patch Tuesday delta only for affected functions, not the whole corpus. This is what your spec already proposes and it's correct.
3. **Reserve `E_backend ≤ 1e-4` slack in your error budget now.** Move δ_indep target from 0.03 → 0.02 effective.
4. **Pin your λ_max estimator.** Document whether you use `λ_max = 2.0` (normalized-Laplacian upper bound) or a power-iteration estimate. When we ship `apply_chebyshev_filter`, we use a sharper estimate, and your numbers will shift O(1%). Pinning lets you A/B the shift cleanly.
5. **Close your Open Question #6 (L702)** in the v0.1 spec: *"Heat-kernel computation is consumer-side in v0.1; migrates to Gigi `POST /heat_kernel` in v0.2; GPU backend follows."* That's a written commitment without blocking v0.1 ingest.

TL;DR: the right ask is "Chebyshev-as-a-primitive over a streaming wire," and the GPU is the swap-in backend behind it. Reserve the `E_backend` slack now so the v0.3 backend swap doesn't re-open the error budget.

---

## §6 — Ask E · `EMIT EVIDENCE_PACK FROM template.md` wire format

**Verdict: green. Ship it — and not as a special verb.**

This is the lightest-lift, highest-leverage ask in the letter. You correctly self-categorized it as "lowest priority, easy client-side" — but you're underselling. Centralizing the geometric-context block is the kind of thing that benefits Marcella, KRAKEN, PRISM, and every future consumer the moment we ship it. We should grab this even if you never ask again.

### The reframe

Don't ship `EMIT EVIDENCE_PACK`. **Ship `EMIT TEMPLATED FROM template.{md,html,txt}` and dispatch on file extension.** The evidence pack becomes the *first stock template*, not a special verb. Same parser cost, dramatically broader surface — Marcella triage notes, PRISM reconciliation reports, KRAKEN incident briefs, all `EMIT TEMPLATED FROM ...`.

### Substrate composition

The geometric-context block is a **read-only projection of already-shipped invariants**:

- `curvature`, `health`, `holonomy`, `betti` already surface at `/v1/bundles/{name}/...` under L1–L8.
- Brain primitives L9–L13 give us `attend`, `focus`, `confidence`, `explain` — exactly the *"heat / taint / confidence / trust_boundaries"* fields you enumerate. The evidence pack becomes a **templated view over the brain endpoints**, not a new compute path.
- Mutation counter from L13 Phase 4 is the natural provenance anchor — *with one honest caveat*: `SnapshotId` lives behind the `transactions` feature flag, so the full `(bundle_id, mutation_counter, query_hash, snapshot_id)` provenance footer is only complete in transaction-enabled builds. Default-build packs get `(bundle_id, mutation_counter, query_hash)` and a `snapshot_id: null`. We'll document that surface honestly rather than promising it falls out free.
- Sharding doesn't enter — packs render off post-query result sets.

### Shape of work

1. **Template engine + feature flag.** `evidence_pack` feature (matches the existing flag discipline). **Choice: minijinja over tera.** The tera sandboxing story is glib; minijinja has a more defensible untrusted-template story, smaller binary, and the autoescape behavior we want. We'll document the swap path either way.
2. **Parser.** Extend `EMIT` to accept `TEMPLATED FROM string`. **Worth noting honestly: the existing EMIT grammar has more cases than `DHOOM | JSON | CSV`** — `EMIT JSONL TO STDOUT` already exists at `GQL_REFERENCE.md:1626–1627`, so the parser change is reconciling two grammars, not "one line."
3. **`src/emit/templated.rs`.** Loads template, builds context with `records`, `geometric_context`, `provenance`. Renders to the target format.
4. **HTTP.** `POST /v1/bundles/{name}/templated_emit` with `{query, template, format}`.
5. **Stock templates.** `templates/evidence/*.j2` in-repo: `triage_note.md.j2`, `reconciliation_report.md.j2`, `anomaly_brief.md.j2`, `candidate_pack.md.j2` (your shape).

Minimum viable surface = 1–3. Polished = +4+5.

### Risks

- **Template trust boundary.** Any consumer-supplied template runs in our process. Sandbox via minijinja's restricted context, disable user-registered functions, autoescape on HTML output, document loudly.
- **Geometric-context drift.** You want heat/taint/curvature/confidence. Marcella will want non_assoc_bound/chern/hadamard. PRISM will want reconciliation_residual. If we hard-code one block, we ship three within a quarter. **Decision we need from you (see §7 below):** is the geometric-context schema **frozen** (you hand us `evidence_pack_context_schema_v0.1.md` and we honor it as the v1 contract) or **dynamic** (we emit every brain-primitive scalar the bundle exposes, templates select with `{% if %}`)? §4 of our analysis pulled both directions; we need you to pick before we ship the contract.
- **Opacity filter at render time.** `EncryptionMode::Opaque` fiber fields must NOT render even if the query result set contains them. Same gate the JSON emitter already runs; load-bearing for evidence packs.

### What you should do in v0.1

1. **Render client-side now.** Build evidence packs in Python from `EMIT JSON` + a local minijinja render. This is what you already plan and it's correct.
2. **Define the geometric-context schema upfront** (see the frozen-vs-dynamic decision above).
3. **Pin templates to a sha256.** Every rendered pack records `template_sha256`. When we ship server-side, the client switch is a one-liner because the template wire format is unchanged.
4. **Don't put scoring in the template.** Your `stability_factor`, Cantelli bound, USL belong in the query result set, not in template logic. Templates render fields; they don't compute scores. Keep packs declarative.
5. **Submit one stock template.** Ship `candidate_pack.md.j2` to us as a reference. We'll land it in `templates/evidence/` when we ship phase 5; your render path stays bit-identical across the client→server cutover.

TL;DR: yes; it generalizes via `EMIT TEMPLATED`; we need you to pick frozen-vs-dynamic for the geometric-context schema before we commit to a v1 contract.

---

## §7 — What we are asking from you

You said "no rush" on a reply. We're not rushing either — but the asks above compose with five concrete things we need from your side to land them well. None of these block v0.1 ingest. All of them sharpen the v0.2/v0.3 trajectory.

1. **A worked DHOOM ingest example end-to-end against your first driver.** `vid.sys` (1,850 functions, 8 confirmed bugs) is exactly the right calibration anchor. We want one runnable Python script that calls Gigi's ingest path with your three `windows_fns` / `windows_calls` / `windows_sinks` BUNDLE DDLs and emits the GASM scalar fields as fiber data. This lets us land the **TAGSET shadow-encoding test**, the **HNSW recall bench at vid.sys scale**, and the **ALTERNATE KEY contract test** against a real corpus, not a synthetic. You're going to write this script anyway for I9; if you can pin a public-domain version of just-this-driver as the worked example, we'll co-locate it under `examples/` and write contract tests against it.

2. **The calibration corpus for cross-version IDENTITY.** Two adjacent Windows versions (23H2 ↔ 24H2 or your pick), a few hundred hand-labeled function pairs as `(version_a_base, version_b_base, label)`. **Without this we cannot ship `CONSISTENCY` as the identity-scheme drift reporter** — that part of Ask B's contract is gated on your corpus, not our engine. Co-locate alongside your existing CVE label set; any reasonable serialization works. Until it exists, we ship the alternate-key index without the drift gate; with it, we ship the full contract.

3. **OOD baseline numbers from vid.sys for `/brain/confidence` calibration.** You mentioned the PK-cohort demo's 184-orders-of-magnitude OOD gap as the "unknown vuln class" refuse gate. For that to actually work at Atlas scale, we need a labeled baseline: a sample of in-distribution `vid.sys` queries with their `/brain/confidence` raw kernel sums, plus a sample of out-of-distribution embeddings (functions from a completely unrelated binary — `pwsh.exe`, `notepad.exe`) with theirs. **Without baseline numbers from a real consumer, our refuse-gate threshold is a knob, not a contract.** Marcella gave us this for SwDA — `[+0.0434, +0.0634]` CI on structured moves is what made T13 production-grade. You're the right person to give us the equivalent for code corpora. A CSV of ~500 in + ~500 out is enough.

4. **A frozen-vs-dynamic decision on the `geometric_context` schema** for `EMIT TEMPLATED` (see §6 risk #2 above). This is the one decision we genuinely can't make for you — your audit pipeline is the load-bearing consumer of evidence packs, and the contract we ship has to match what your reviewers will sign off on.

5. **A pre-commitment on your stability-factor + Cantelli-bound interpretation under HNSW approximation.** Your error budget assumes exact recall in the SUSANOO calibration. HNSW gives ≥0.95 recall@k=50 with over-fetch. If a missed neighbor at rank 50 is a *security* miss (i.e. the calibration analog needs exact recall, not approximate), tell us now — we'll ship `SIMILAR EXACT` as a parallel verb rather than dialing `ef_search` to brute-force. If approximate recall is fine and you'll fold the recall variance into `δ_indep`, also tell us — we'll ship the approximate path as the default. **This is a one-bit decision but it shapes which of two code paths becomes the v0.1 contract.**

If we get items 1–5, we can hold the v0.2 contract on TAGSET, HNSW, and ALTERNATE KEY without speculative defaults. None are blocking.

---

## §8 — Concrete v0.1 smoke-test coordination

You said: *"First smoke test: ingest one driver end-to-end (`vid.sys`, 1,850 functions, 8 confirmed bugs as calibration anchors), reproduce the SUSANOO top-10 ranking from SCJ README `:351-355` as a single GQL query. If `SIMILAR` + `OUTLIER` get us there, the substrate is proven."*

Specific commitments from our side:

1. **A fresh `gigi-stream` branch you can pin to.** We'll cut `scj-v0.1-substrate` off the current main (sharding T1–T13 green, transactions Phase 1–4 shipped, IMAGINE/WALK with Marcella trust envelope, brain primitives L9–L13 surfaced). The branch is **frozen** for the duration of your v0.1 ingest — no force-pushes, no rebases. When you hit issues, we cherry-pick fixes onto the frozen branch; you re-pin a tag, not a moving head. This is the same discipline we used for Marcella's IMAGINE_COHERENCE Phase 1 — a known-good substrate for the consumer to integrate against while we land follow-ups on main.

2. **A contract test for your three BUNDLE DDLs.** Pin `windows_fns`, `windows_calls`, `windows_sinks` as `.gql` files in `examples/scj_atlas/` once you draft them. We write `tests/scj_atlas_contract.rs` asserting: (a) all three DDLs parse against the frozen `scj-v0.1-substrate` grammar; (b) the round-trip schema → DHOOM emit → re-ingest is byte-identical on `vid.sys`-scale synthetic data; (c) `SIMILAR ... ON embedding TO ... TOP 10` against a 2K × 128-d synthetic returns deterministic results across runs (critical for your SUSANOO top-10 reproducibility). The contract test runs in CI on every commit to the frozen branch.

3. **A `vid.sys` smoke target.** Once you have a public-domain or sample-encoded version of just-this-driver's GASM map ingestible (heat/taint/curvature/near_trust_boundary scalars + sinks_reached as 17-boolean shadow + embeddings), we'll add an `examples/scj_vid_smoke.py` that runs the SUSANOO top-10 reproduction end-to-end. **Acceptance gate: SUSANOO appears in the top-10 across three independent runs against the frozen branch.** If it doesn't, we have a substrate bug and the smoke test catches it before you ingest the rest of Windows.

4. **A direct line for the four open contract decisions.** TAGSET hot-path Hash cost, HNSW vs incremental-insert, ALTERNATE KEY seed rotation, `geometric_context` frozen-vs-dynamic. We'll prototype each in the engine, you A/B against your Python reference, and we land the version that wins on your real corpus. **Open contract decisions get resolved against your data, not our intuition.**

5. **The `t13_production_swda_seam.py`-equivalent for code.** Once `vid.sys` is ingested and SUSANOO top-10 reproduces, we want to add a gate test alongside T13-prod (the SwDA discourse-state seam) that asserts the same Z₂-class-distinction shape holds for code-bundle queries — concretely, that the `audit_status: confirmed_bug` vs `audit_status: false_positive` distinction lifts cleanly through `SAMPLE_TRANSPORT BUDGET τ`. This is the math gate that proves the substrate isn't just *operating* on your data; it's *seeing* the structure you care about. We don't need it for v0.1 acceptance, but it's the kind of follow-up that turns "Gigi can host SCJ" into "Gigi's geometry is co-validated by SCJ's empirics."

---

## §9 — On composition with what we shipped recently

A short note on substrate composition, because half of why your six asks land so cleanly is that **what we shipped in the last sprint cycle was load-bearing for what you're about to do**, and the other half is that some of your asks reach into seams we hadn't fully closed.

- **Sharding T1–T13** (Poincaré-to-sharding lineage, partition-of-unity, halo-as-IMAGINE) is why HNSW-per-shard with cross-shard merge is a "for free" extension and not a re-architecture. Cross-version IDENTITY's "route by base hash, index by identity hash" works because the sharding routing primitive already exists.
- **Transactions Phase 1–4** (atomic sheaf commits + snapshot isolation + MVCC + geometric coherence) is why `SNAPSHOT` and `DIFF` are reliable enough for Patch Tuesday — and why your TAGSET writes get rollback and your ALTERNATE KEY indexes get snapshot-stable mutation counters. Without Phase 4 specifically, the `RenamedSection` DHOOM event would be a best-effort signal, not a transactional one.
- **IMAGINE/WALK with the Marcella trust envelope** is the reason `/brain/confidence` has a calibrated OOD story you can lean on. The 184-orders-of-magnitude gap in the PK demo isn't an artifact — it's the trust envelope at work.
- **Brain primitives L9–L13** as Friston substrate is why ATTEND / FOCUS / EXPLAIN map cleanly onto your audit-status fiber. The 12 primitives weren't designed for code; they were designed for cognition, and code is cognition's output. The mapping is structural.
- **The seam your asks reach into:** the gauge / cocycle layer between sharded bundles and cross-version comparisons isn't fully closed. Čech-of-cover is shipped (`src/sheaf/`). Čech-of-morphism — which is what cross-version IDENTITY actually wants for its obstruction metric — is new. We'll ship the alternate-key index and the `Renamed` DIFF without it, and add the obstruction metric as a follow-up. Flagging it so you know which part of Ask B is "wire the existing math through" vs "add new math."

The four-and-a-half asks are mostly wiring. Ask B's obstruction metric and Ask D's GPU backend are the two pieces of *new* substrate. Everything else is finishing plumbing.

---

## §10 — Closing

Two things, in order.

**One.** Your closing quoted two of our phrases. We'll close by quoting one of yours, because it's the kind of one-line operational discipline we want to absorb into our own commit messages going forward:

> "stay on the geodesic"

That's the right discipline for both sides of this correspondence. *We don't ship features that drift off the substrate; you don't ship ingest that drifts off the contract.* The geodesic isn't a slogan — it's the constraint that keeps consumer and substrate co-evolving rather than co-drifting. We're going to use that phrase. Thank you for it.

**Two.** You said *"of your six asks, this is the one where you were over-cautious"* about HNSW. We'll return the favor on the whole letter: **you were over-cautious about the substrate.** Five of six asks land in plumbing-not-design territory, the sixth (GPU) defers cleanly into a substrate move (Chebyshev-as-a-primitive) plus a backend swap. You spent half the Atlas spec writing fallbacks for paths the engine already supports. Don't. Build your ingest as if the engine works because by the time you're at 2M functions it will, and the work we have to do to get there is the work we want to do.

Send the §10 PR whenever you're ready. Pin `scj-v0.1-substrate` whenever the BUNDLE DDLs are drafted. Drop a CSV with the OOD baseline whenever `vid.sys` is the first thing you ingest. And when `gigi-stream` shows SUSANOO in the top-10 on the smoke target, send the run log — we want to see it live in the contract test.

Welcome aboard. Glad you're downstream.

— **Gigi engine team** · Davis Geometric · 2026-06-05
   Lineage: GIGI v0.4 · Kähler upgrade L1–L13 · Sharding T1–T13 · Transactions Phase 1–4 · IMAGINE/WALK with Marcella trust envelope

---

## Appendix — file pointers (what we read to write this)

In `shadow_clone_jutsu/`:
- `specs/LETTER_TO_GIGI_TEAM_2026-06-05.md` — the letter we're answering
- `specs/WINDOWS_ATLAS_ON_GIGI_SPEC_v0.1.md` — full Atlas spec
- `specs/shadow_clone_jutsu_v05_spec.md` — SCJ hunter spec (Davis error budget §3)
- `specs/gasm_spec_v1.md` — GASM cartography spec
- `docs/math.md`, `docs/shadow_clone_jutsu_paper.md`

In `gigi/`:
- `src/bundle.rs` — `FiberPoint`, `field_index`, `intersect_bitmaps`, `build_hnsw_for_fields`, `cover_near`, `vector_search`
- `src/hash.rs` — `HashConfig`, `from_schema_with_base_seed`, `encode_value`, seed-rotation discipline
- `src/spectral.rs` — `sparse_spectral_gap`, `mul_m`
- `src/sharded/spectral.rs` — `distributed_lanczos`, `block_matvec`
- `src/sheaf/` — Čech 1-cocycle / H¹ pipeline (within-bundle, not across-morphism)
- `src/types.rs` — `Value`, `FieldType`, `EncryptionMode`
- `Cargo.toml` — feature-flag pattern (`kahler`, `sharded`, `imagine`, `transactions`)
- `GQL_REFERENCE.md`, `GQL_SPECIFICATION.md` — `EMIT` grammar incl. `JSONL` precedent
- `theory/post_kahler_directions/catalog.md` — §1–§9 template, line 545/559–568 validation discipline
- `theory/kahler_upgrade/REPLY_TO_MARCELLA_*.md` — voice and format reference

— end —
