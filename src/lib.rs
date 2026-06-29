//! GIGI — Geometric Intrinsic Global Index
//!
//! A fiber-bundle-based database engine.
//! Davis Geometric · 2026

pub mod aggregation;
// GIGI Encrypt v0.3.x — Gauge-equivariant aggregate inversion.
// Client-side helpers that take an encrypted-side aggregate (SUM, AVG,
// MIN, MAX, VAR, STDDEV, RANGE) and the gauge key, returning the
// plaintext aggregate via closed-form affine inverse. Closes the
// "FHE-required for analytical SQL" gap at native server speed +
// O(1) client post-processing. See `src/aggregate_helpers.rs`.
pub mod aggregate_helpers;
pub mod bundle;
// 2026-06-20 cache consolidation (workflow w2n0fgqkk): generic
// `SingleFlightCache<K, V>` extracted from three independent
// reimplementations of the same correctness-critical pattern
// (BundleFlowCache, VectorMatrixCache, MorseCache). See
// `src/caches/single_flight.rs` for the audit history and the
// load-bearing semantics the generic preserves.
pub mod caches;
pub mod coherence;
pub mod concurrent;
pub mod convert;
pub mod crypto;
pub mod curvature;
// Halcyon CHERN_CLASS + PONTRYAGIN — Chern-Weil discrete integration
// of characteristic classes (c_k, p_k) on a `Lattice` cell complex.
// Phase 1 ships ORDER 0/1/2 with the clover topological-charge
// construction for SU(2) on 4D bases. Gated on the `gauge` feature
// because the kernel composes plaquette holonomies through the
// `EdgeConnection` surface and projects them via `GroupElement`
// arithmetic. References: Lüscher 1982; Cohen Ch 3 §6.
#[cfg(feature = "gauge")]
pub mod chern_weil;
// GIGI Encrypt v0.3 — Sprint J (Aff(ℝ) capability delegation).
// Composes two GaugeKeys' Affine / Isometric / Identity transforms
// into a per-field capability the proxy applies on ciphertext, never
// touching plaintext. Renamed from "proxy re-encryption" per v0.3.1
// review §4 because the construction is NOT collusion-resistant PRE:
// Bob + capability + own key recovers Alice's key. See
// `GIGI_ENCRYPT_v0.3_SPRINT_SPEC.md` §4.7 for the documented
// limitation and Umbral as the alternative.
pub mod delegation;
// GIGI Encrypt v0.4 (planned) — Pairing-based collusion-resistant
// delegation. Sprint J's Aff(R) capability delegation is NOT
// collusion-resistant by design (additive-arithmetic limitation);
// this module documents the v0.4 path forward via BLS12-381 pairings
// (Umbral construction, Nuñez 2017). All operations currently
// `unimplemented!()` — v0.4 sprint will fill them in alongside the
// BLS12-381 dependency. See `src/pairing_delegation.rs` doc-comments
// for the full construction + security argument outline.
pub mod pairing_delegation;
// GIGI Encrypt v0.3.x — Sprint J.3 (ML-KEM trusted-delegatee delegation).
// FIPS 203 ML-KEM-768 (post-quantum KEM, NIST Level 3) wraps a session
// secret to a recipient; AES-256-GCM-SIV AEAD then encrypts the payload
// under the KEM-derived key. Trust model: trusted-delegatee (Bob holds
// Alice's full key after delegation). Quantum strength: IND-CCA under
// MLWE assumption. Closes the BLS12-381 quantum gap for the trusted-
// delegatee threat model.
pub mod mlkem_delegation;
// GIGI Encrypt v0.3.x — Sprint J.4 (Lattice threshold delegation).
// Two-layer composition: Shamir K-of-N split (info-theoretic
// security) + per-share ML-KEM-768 transport (PQ IND-CCA). Closes
// the PQ + collusion-resistance gap structurally: any K-1 colluding
// shareholders learn nothing about the delegated payload (Shamir);
// transport is PQ-safe under MLWE.
pub mod lattice_delegation;
pub mod dhoom;
pub mod edge;
pub mod engine;
// Gauge transformations (Schema Migrations §5a) + gauge-theory
// connection primitives. The `gauge` Cargo feature additionally
// pulls in `group_element`, `edge_connection`, and `holonomy` —
// general-purpose group-erased connection algebra. Halcyon's Davis
// Wilson Lattice substrate is the first consumer (see the `halcyon`
// composite feature which enables `lattice + gauge` together and
// the bit-identity integration test in `tests/`).
pub mod gauge;
// Kähler-geometry substrate (catalog.md §1, the generator
// 𝒢 = (M, g, J, ∇, B, Γ)). Gated by the `kahler` feature so the
// engine's existing surface area is bit-identical when the feature
// is OFF. See theory/kahler_upgrade/ for catalog + implementation
// plan + validation tests.
#[cfg(feature = "kahler")]
pub mod geometry;
// Kähler graph operators (catalog.md §1.1): dual principal/
// auxiliary adjacency + commutativity classifier the query planner
// uses for theorem-backed join reordering. Same feature gate as
// `geometry` — strict additive layer.
#[cfg(feature = "kahler")]
pub mod graph;
// Geometric cost-model primitives (catalog.md §1.3, §1.4, §1.5):
// Jacobi-field cardinality estimation + trajectory-ball volume
// bounds via Bishop / Günther. Feeds the query planner with
// theorem-bound cardinality estimates instead of statistics-based
// guesses.
#[cfg(feature = "kahler")]
pub mod cost;
// L6 — discrete exterior calculus + Hodge complex (catalog §2.9):
// d_0, d_1 chain operators, Hodge Laplacians, Betti numbers,
// Morse compression. Enables Marcella's transport on a compressed
// substrate without linear-walk costs (catalog §2.9 product
// application; per Marcella's 2026-05-24 letter).
#[cfg(feature = "kahler")]
pub mod discrete;
pub mod hash;
// GIGI Encrypt v0.3 — Sprint I (Curvature-MAC bundle integrity).
// HMAC-SHA256 over the canonical-encoded invariant tuple
// (K, λ_1, capacity, ⟨Hol⟩, β_0, β_1). Detects gauge-invariant
// content drift; pairs with Sprint K's extended ledger leaves
// (`record_hash`) for byte-level tamper-evidence per spec §3.8.
// See `GIGI_ENCRYPT_v0.3_SPRINT_SPEC.md` §3.
pub mod integrity;
// Halcyon ITEM 3.2 — INGEST executor. Reads NumPy NPZ archives and
// maps outermost-axis slices into Records that the engine ingests via
// `batch_insert`. Phase 1 ships NPZ-only with a generic
// auto-array-to-records mapping (HDF5/JSONL/CSV deferred to Phase 2,
// EXPLICIT_SCHEMA parser surface deferred to a follow-up sprint).
// See `src/ingest.rs` module docs and `tests/ingest_executor.rs`
// for the contract.
pub mod ingest;
pub mod invariant;
// GIGI Encrypt v0.4 — Sprint N (Invariant Consistency Verification).
// Public deterministic verification that a prover's claimed invariant
// tuple π_inv = (K, λ_1, ⟨Hol⟩, τ, β_0, β_1) agrees with the bundle's
// computed tuple. Verifier never receives the gauge key.
// See `GIGI_ENCRYPT_v0.4_SPRINT_SPEC.md` §Sprint N.
pub mod invariant_verify;
// GIGI Encrypt v0.4 — Sprint O.A (I_Aff falsification harness +
// builtin invariant computations on raw value slices). Complementary
// to the Sprint H parser-by-construction guarantee: runtime numerical
// check of gauge invariance for ad-hoc query callbacks.
pub mod invariant_ring;
// GIGI Encrypt v0.4 — Sprint O.B (Credential-gated invariant query
// authorization, HMAC-bound; BBS+ upgrade path deferred to v0.5).
pub mod credentials;
// GIGI Encrypt v0.4 — Sprint P (Geodesic-ball approximate membership
// index — centroid + isotropic σ² + dimension-aware χ² threshold).
// Not a cryptographic accumulator; see module docs for leakage scope.
pub mod membership_index;
pub mod join;
// GIGI Encrypt v0.3 — Sprint K (Holonomy ledger / tamper-evident audit log).
// Append-only Merkle tree (RFC 6962) over per-write leaves
// `(timestamp, op_id, holonomy_delta, record_hash, op_kind)`. Extended
// `record_hash` leaves close Sprint I's gauge-invariant-content blindspot
// per spec §3.8. See `GIGI_ENCRYPT_v0.3_SPRINT_SPEC.md` §5.
pub mod ledger;
pub mod metric;
pub mod mmap_bundle;
pub mod observability;
pub mod parser;
pub mod query;
// GIGI Encrypt v0.3 — Sprint M (Continuous RG-flow ratchet).
// Per-write KDF chain g_{t+1} = HKDF-SHA256(g_t, record_bytes || t).
// Checkpoints every N writes; retention horizon R drops checkpoints
// below T-R, making g_t for t < T-R computationally unrecoverable.
// See `GIGI_ENCRYPT_v0.3_SPRINT_SPEC.md` §7.
pub mod ratchet;
pub mod sheaf;
// Atlas-cover sharding model. Opt-in via the `sharded` feature flag.
// Phase A: types + skeleton; execution bodies are `todo!()` /
// NotImplementedYet errors until Phase B integrates with BundleStore.
// See `theory/poincare_to_sharding/SHARDING_SPEC.md` for the full design
// and `theory/poincare_to_sharding/validation/` for the 7 GREEN TDD
// gates the spec is built on.
#[cfg(feature = "sharded")]
pub mod sharded;
// Extrapolation verbs (IMAGINE / WALK). Opt-in via the `imagine`
// feature flag. Provides ImaginedRecord with required provenance,
// imagine_geodesic (T11-validated RK4 integrator), imagine_halo
// (T12-validated gauge-equivariance for sharded CURVATURE), and walk
// with Marcella's load-bearing curvature safety envelope
// (default max_imagined_curvature = 4.0 = K(CP^1 Fubini-Study)).
// See `theory/imagine/IMAGINE_AND_WALK.md`.
#[cfg(feature = "imagine")]
pub mod imagine;
// Atomic Sheaf Commits — cross-bundle ACID transactions. Phase 1:
// 2PC with coordinator/participant recovery (gates TX1-TX5 in
// theory/transactions/validation/). Opt-in via the `transactions`
// feature flag. ACID is what this primitive degenerates to when you
// ignore the geometry: full design adds cocycle preservation,
// K-monotone, and connection-coherent invariants atop atomicity.
// See `theory/transactions/ATOMIC_SHEAF_COMMIT_SPEC.md`.
#[cfg(feature = "transactions")]
pub mod transactions;
// Causal States v0.1 — update commutator substrate (companion to Davis
// 2026 "Causal States as Predictive Sections"). Behind `causal_states`
// feature flag. Provides TV / Hellinger / KL scalar diagnostics on
// discrete distributions and the commutator computation
// U_{ab}(p) - U_{ba}(p) for paired Bayesian updates. Math companion at
// `theory/causal_states/validation_tests.py` (36/36 green).
#[cfg(feature = "causal_states")]
pub mod causal_states;
// Lattice — graph-topology primitive (vertices + signed edges +
// face-cycle table + topology hint). General-purpose; Halcyon's
// Davis Wilson Lattice substrate is the first consumer but the type
// is not Halcyon-specific. Opt-in via the `lattice` Cargo feature.
// The LATTICE verb in src/parser.rs (Statement::Lattice /
// LatticeFromCanonical / ShowLattice) is gated on the same feature.
// See `HALCYON_PART_I_GATES.md` for the originating gate breakdown
// and `theory/halcyon/HALCYON_PART_I_IMPLEMENTATION_LOG.md` for the
// per-gate receipts (Halcyon-Part-I shipped this primitive plus the
// gauge-side connection algebra now living in `src/gauge/`).
#[cfg(feature = "lattice")]
pub mod lattice;
pub mod spectral;
// GIGI Encrypt v0.3 — Sprint L (Čech threshold sharing).
// Shamir secret sharing over secp256k1 base field F_p (p = 2^256 - 2^32 - 977),
// framed as Čech reconstruction on the share-holder cover. Each share carries
// an HMAC-SHA256 auth tag binding it to (bundle_id, share_index, holder.pubkey).
// See `GIGI_ENCRYPT_v0.3_SPRINT_SPEC.md` §6.
pub mod threshold;
pub mod types;
// Personal-list #3 (2026-06-22) — `__bundles__` virtual bundle that
// exposes the live engine registry as a queryable bundle. Read-only;
// materialized fresh on every `COVER __bundles__` call.
pub mod virtual_bundles;
// Cached `(N, D)` matrices for the vector-search brain endpoints
// (`intent_gate`, `confidence`, `confidence_with_explain`). Per Marcella's
// 2026-05-29 `GIGI_BUG_REPORT_onfields_latency.md`: pre-materialize the
// fiber column slab once and reuse it across requests instead of rebuilding
// per call. Mutation-counter invalidated, single-flight on cache miss —
// same architecture as `BundleFlowCache` in `gigi_stream.rs`.
pub mod vector_cache;
// 2026-06-02 `REPLY_TO_SEMANTIC_PERF_2026-06-02.md` follow-up: cache
// `semantic_gist()` / `morse_compress()` results keyed by
// `(bundle_name, mutation_counter)`. Defense-in-depth on top of the
// `betti_rank` algorithm fix — subsequent reads on the same bundle
// skip even the rank computation, returning the previously-computed
// Betti tuple in O(1) hashmap-lookup time. Same architecture as
// `vector_cache.rs` and `BundleFlowCache` in `gigi_stream.rs`.
#[cfg(feature = "kahler")]
pub mod morse_cache;
pub mod wal;

// Halcyon Part V P-1 — `/v1/gql` dispatch helper for gauge-feature
// statements. Closes the §2.5 drop bug: the binary `gql_query` used
// to early-return `{"status":"ok"}` for LATTICE / GAUGE_FIELD /
// GIBBS_SAMPLE / E_FIELD / SYMPLECTIC_FLOW / SHOW E_FIELD /
// SELECT H_TOTAL / SELECT GAUSS_RESIDUAL_MAX / SELECT PLAQUETTE /
// SELECT Q_SURROGATE / SHOW (GAUGE_FIELD | LATTICE) /
// LATTICE FROM TRUNCATED_ICOSAHEDRON. This module is the testable
// boundary the binary now consults BEFORE the bundle-name
// extraction. Same `gauge` feature gate as the Statement variants it
// dispatches through.
#[cfg(feature = "gauge")]
pub mod halcyon_gql_dispatch;

pub use bundle::{
    detect_base_geometry, BaseGeometry, BundleStats, BundleStore, QueryCondition, QueryPlan,
    TransactionOp, TransactionResult, VectorMetric,
};
pub use engine::{Engine, MutationOp, Notification, QueryCache, TriggerDef, TriggerKind, TriggerManager, query_fingerprint};
pub use metric::FiberMetric;
pub use mmap_bundle::{BundleMut, BundleRef, OverlayBundle};
pub use query::QueryResult;
pub use types::{
    AdjacencyDef, AdjacencyKind, BundleSchema, FieldDef, FieldType, TransformFn, Value,
};
