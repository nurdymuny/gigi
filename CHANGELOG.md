# GIGI Changelog

Detailed "What's new" entries that used to live in the README, collected
here so the README can stay short. Newest first.

For a compact summary of the most recent ships, see
[README.md § Recent changes](README.md#recent-changes).

---

## 2026-06-04 — sharding complete + atomic sheaf commits Phase 1 + Marcella SwDA WIN

**Sharding initiative end-to-end.** All 14 math gates green (T1–T10 + TFP1 + TFP2 + TFH1 + TFH2), Rust scaffold and HTTP routes shipped, deployed to production at v199 with `kahler imagine sharded` features ON. The cross-atlas BETTI (T9) Rust port closes the last queued item — fiber-product Mayer-Vietoris via vertex-identification + union-find + the existing F₂ rank pipeline.

| Sharded module | Surface |
|---|---|
| `wrap_hash_sharded` / `wrap_fiedler_sharded` | Hash + topology-aware partitioning |
| `shard_curvature` / `shard_betti_disjoint` / `shard_betti_mayer_vietoris` | Per-chart CURVATURE, disjoint-union BETTI, M-V corrected BETTI |
| `shard_holonomy_along_path` / `shard_holonomy_around_loop` + `mat2x2_det` | Open + closed loop holonomy; Möbius det = -1 detection |
| `distributed_lanczos` / `shard_lambda_1_from_bundle` | T7 universal SPECTRAL, end-to-end from any bundle |
| `cross_atlas_*` + `cross_atlas_betti_via_fiber_product` | Marcella + PRISM bridge contract; cocycle check + Clean Finger Move + fiber-product BETTI |

HTTP routes shipped: `POST /v1/bundles/{name}/sharded/spectral_gap`, `…/curvature`, `…/holonomy_loop`. Each materializes records into a fresh `ShardedBundle` and dispatches through the canonical end-to-end primitive.

**Atomic Sheaf Commits Phase 1 shipped.** 2-phase commit with full coordinator/participant failure recovery: presumed-abort, log replay, partial-notify catch-up, all 5 TX2 failure scenarios green at the Rust level. New module `src/transactions/` behind the `transactions` feature flag. Math contract: "ACID is what our primitive degenerates to when you ignore the geometry" — the additional invariants (cocycle bound, K-monotone, connection-coherent) are the cocycle bound from Davis 2026b applied to time. Spec: [`theory/transactions/ATOMIC_SHEAF_COMMIT_SPEC.md`](theory/transactions/ATOMIC_SHEAF_COMMIT_SPEC.md).

**Marcella SwDA discourse-flow WIN.** CI=[+0.0434, +0.0634] on structured moves, fiber 6.57% vs bigram 1.50% on dispreferred (4.4×). The IMAGINE substrate earns its seat exactly where the protocol predicted. T13 production gate (real SwDA labels `ny/nn/ng/na` vs `%/x`) shipped. Reply letter: [`theory/kahler_upgrade/REPLY_TO_MARCELLA_SWDA_DISCOURSE_WIN_2026-06-03.md`](theory/kahler_upgrade/REPLY_TO_MARCELLA_SWDA_DISCOURSE_WIN_2026-06-03.md).

**Transitions JSON-key serde adapter.** `HashMap<TransitionKey, Transition>` and `HashMap<BridgeChartKey, BridgeTransition>` now JSON-roundtrip via `Vec<…>` adapters; the canonical key is recovered from each value's own `from`/`to` fields on deserialize.

Full Rust suite: 1643 passed, 0 failed, 11 ignored with `kahler sharded imagine transactions`.

---

## 2026-06-03 (evening) — IMAGINE / WALK lands: extrapolation verbs with Marcella's trust envelope

**The cognitive analog Bee named: humans imagine the path before walking it. We solve a geodesic in our head, describe the path, then walk it.** The math GIGI already has — connection, parallel transport, double cover, fault tolerance — is the engine that does this. This sprint named the verb, spec'd the trust envelope, TDD-gated the math at three independent claims, and shipped the Rust scaffold.

The pivot started from the Phase D learning: hash-sharded CURVATURE has a partition-dependent `k_sum` because GIGI's `compute_record_k` derives K from the per-bundle neighborhood graph, which fragments under hash partition. The honest disclosure was a `RED` in `ShardedCurvatureReport`. Marcella's "this looks like the encrypt parity work — gauge-equivariance, design ρ⁻¹ so it commutes" diagnosis was right: the fix was to give each chart **halo records** so the per-chart K computation sees the same neighborhood as the unsharded one. That's exactly what IMAGINE produces.

**Three TDD math gates, all GREEN** (run via [`theory/imagine/validation/run_all.py`](theory/imagine/validation/run_all.py)):

| Gate | Validates | Result |
|---|---|---|
| **T11** | RK4 geodesic integrator on S²/T²/CP¹ via Christoffel symbols from conformal factor | errors 6.66e-16 to 1.36e-14 (vs 1e-9 tol) |
| **T12** | Halo-as-IMAGINE makes sharded CURVATURE partition-invariant | residual = **0.000e+00 exactly** across n_charts ∈ {2, 4, 8} |
| **T13** | Double cover monodromy resolution: synthetic Möbius + discourse-state seam at `act_history=("qy",)` | -1 / +1 holonomy lift exact; discourse seam refuses without cover, returns definite class with cover |

T12 is the load-bearing one. Same 60 records partitioned three ways: without halos, `k_sum` = 35.6 / 68.5 / 122.8 (102% / 289% / 597% off from baseline). With halos populated via `imagine_halo`, all three partitions produce **exactly** `k_sum = 17.618609` — matching the unsharded direct computation to floating-point precision, with zero residual.

**Marcella's trust envelope** is load-bearing in the spec at three positions:

1. **`ImaginedRecord` carries required `ImaginedProvenance`.** Every imagined record renders with an explicit prefix: `[imagined: projected from <bundle> via geodesic, path_length=0.31, accumulated_holonomy=0.04]`.
2. **`WalkConfig::max_imagined_curvature` defaults to 4.0 = K(CP¹ Fubini-Study).** "Walking into regions of higher Gaussian curvature than complex projective space requires explicit opt-in."
3. **FORECAST vs IMAGINE routing rule is computable:** `if query_grounding_normalized > 0.5: FORECAST else: IMAGINE`. Same threshold as Gate J.

`IMAGINE_COHERENCE` HTTP endpoint at `POST /v1/bundles/{name}/imagine_coherence` shipped same day. Marcella's round-3 trust envelope upgrade added `is_imagined()` accessor, `CurvatureGateRaisedAboveDefault` audit signal, and the routing helper at [`src/imagine/routing.rs`](src/imagine/routing.rs).

Spec: [`theory/imagine/IMAGINE_AND_WALK.md`](theory/imagine/IMAGINE_AND_WALK.md).

---

## 2026-06-03 (afternoon) — sharding lands as substrate, not as compromise

**Ten TDD-gated math claims, Phase A scaffold, Phase B runtime wrapper, cross-atlas joins spec — all in one afternoon.** Most databases face CAP-style trade-offs where sharding adds coordination cost. GIGI's geometric substrate inverts the cost curve: per-query cost goes *down* with shard count because every verb except SPECTRAL is sheaf-glued natively.

The push back came from Bee's three companion papers (Davis 2026a *The Davis Manifold*, 2026b *The Geometry of Sameness*, 2026c *Smooth 4D Poincaré Conjecture*): shards are charts, transitions between shards are the connection 1-form data, cocycle bound (Davis 2026b Def 21) controls multi-hop slack, Clean Finger Move Theorem (Davis 2026c Thm 5.3) gives a constructive write-conflict resolver.

| Gate | What it validates | Result |
|---|---|---|
| **T1** | Sharded BETTI exact via Mayer-Vietoris | β_n on S¹, S², T² recovered exactly from per-chart data |
| **T2** | Cocycle bound: 0 for analytic, first-order for learned | analytic δ=1.78e-14; perturbed slope 0.924 |
| **T3** | Sharded CURVATURE via sheafification | CP¹ Fubini-Study K=4 from each chart, charts hold 4× different raw ρ |
| **T4** | Sharded HOLONOMY w/ non-trivial gauge transition | T² closed loop with A_L ≠ A_R on overlap, holonomy invariant |
| **T5** | Honest sharded λ₁ bounds (NON-universal disclosure) | Universal Weyl holds; naive bound FAILS on expanders by 5-7× |
| **T6** | Clean Finger Move conflict resolver | terminates in N/2 steps, density- and ordering-invariant |
| **T7** | Distributed Lanczos closes the expander gap | all 7 graph cases converge to machine precision, K=25–99 |
| **T8** | Cross-atlas bridge cocycle bound | analytic ~1e-14; perturbed slopes 0.961–1.088 |
| **T9** | Cross-atlas BETTI via fiber-product Mayer-Vietoris | S² and T² fiber products exact via per-atlas + bridge data |
| **T10** | Cross-atlas Clean Finger Move resolver | atlas-agnostic; terminates in N/2 across all distributions |

Three of these (T2, T5, T6) were **red on first run** and caught real math errors. The red-then-green cycles are the most valuable receipts.

Theory + spec: [`theory/poincare_to_sharding/poincare_to_sharding.md`](theory/poincare_to_sharding/poincare_to_sharding.md), [`theory/poincare_to_sharding/SHARDING_SPEC.md`](theory/poincare_to_sharding/SHARDING_SPEC.md), [`theory/poincare_to_sharding/CROSS_ATLAS_JOINS.md`](theory/poincare_to_sharding/CROSS_ATLAS_JOINS.md).

---

## 2026-06-03 — LOCAL_HOLONOMY (5th Cognitive Geometry verb) + intent_gate perf fix + PolyForm NC license

**LOCAL_HOLONOMY — the windowed-holonomy coherence signal.** Cognitive Geometry family now **five verbs**: CAPACITY · HORIZON · DEPTH · PERCEIVE · **LOCAL_HOLONOMY**. Marcella's gain gate needs: *"between time t−w and time t, how much did the cumulative frame rotate, and therefore how trustworthy is the local coherence regime?"*

Math: `R_window = R_current · R_past^T`; defect = `‖R_window − I‖_F` (gauge-invariant under simultaneous orthogonal conjugation); coherence = `1 − defect/(2·√dim) ∈ [0, 1]`. Pinned reference points: identity → 1.0 (laminar), 30° + (30°)⁻¹ → 0.5 (moderate), I + (−I) → 0.0 (turbulent).

Ships in [`src/curvature.rs`](src/curvature.rs) and `POST /v1/bundles/{name}/local_holonomy`. End-to-end chain test verifies the defect against the closed-form Rodrigues prediction `sqrt(4·(1−cos θ))` to 1e-5.

**`/brain/intent_gate` empty-constraints fix.** When a caller passes zero constraints, the SUDOKU half was walking all records via `solve_constraints()` (~5s on a 10k bundle). Fix at the endpoint layer: synthesize a trivial `SudokuResponse { verdict: Sat, coverage: 1.0, n_considered: 0 }`. ~5 s → 0 ms.

**License transition: PolyForm Noncommercial 1.0.0.** Free for personal use, research, education, and noncommercial organizations; commercial use is reserved by the copyright holder under a separate written agreement.

Gates: **1124/1124** lib tests with `--features kahler`; **847/847** no-feature regression. Production deploy: v195.

---

## 2026-06-02 (later) — SEMANTIC perf polish (MorseCache + column-indexed rank)

Three follow-ups on top of the betti-rank merge: (1) `MorseCache` keyed by `BundleStore::mutation_counter()` for O(1) second+ reads; (2) column-indexed pivot search in `F2Matrix::rank()` cut bucket-32 worst-case 12.4× (6.9 s → 557 ms); (3) 8th cross-check fixture (`cross_check_production_shape_complex`) asserting F₂ ≡ ℝ Betti on production-shape complexes (the Hausmann safety net).

MorseCache lifts the `vector_cache::VectorMatrixCache` pattern (RwLock<HashMap> + per-key Arc<Mutex<()>> single-flight + mutation_counter invalidation). Capacity via `GIGI_MORSE_CACHE_SIZE` env (default 64).

Net effect on the Stacks UI: first-call latency consistently sub-second; second+ calls O(1) cached. Gates: 1118/1118 lib with `kahler` (+15); 841/841 no-feature; sub-quadratic complexity gate tightened to 200× (measured: 45×).

---

## 2026-06-02 — SEMANTIC perf rewrite (rank-based Betti)

**`/v1/bundles/{name}/brain/semantic` now skips the dense Laplacian eigendecomposition entirely.** The original L6.3 implementation ran `nalgebra::SymmetricEigen` on three dense Hodge Laplacians (`O(V³ + E³ + F³)` per call). On Marcella's 9,964-record bundle it took 10–30s and blocked the Stacks UI.

The rewrite replaces eigendecomposition with sparse F₂ Gaussian elimination on the boundary matrices: `Betti_n = |C_n| − rank(d_{n-1}) − rank(d_n)` (rank-nullity on the chain complex). New module [`src/discrete/f2_rank.rs`](src/discrete/f2_rank.rs) implements bitset-packed F₂ matrices with in-place XOR Gaussian elimination — ~450 LOC, no new crate dependencies.

Coefficient choice: F₂ vs ℝ Betti agree exactly when integral homology has no 2-torsion. For the flag complexes GIGI builds, 2-torsion is empirically absent on every fixture, but per Hausmann this is plausible-in-practice not theorem, so the 7-fixture cross-check is the load-bearing safety net.

Measured speedups (release build): T² 12×12 — **2260× (12.27 s → 5.4 ms)**; real-sensor smoke — 263 s → 30 s (~8.5×). Gates: 1103/1103 lib with `kahler` (+22 new); 841/841 no-feature. Reply letter: [`theory/kahler_upgrade/REPLY_TO_SEMANTIC_PERF_2026-06-02.md`](theory/kahler_upgrade/REPLY_TO_SEMANTIC_PERF_2026-06-02.md).

---

## 2026-05-30 — Cognitive Geometry verbs (Branch VII)

> The four verbs that landed in this sprint became the **first four of five** when LOCAL_HOLONOMY shipped on 2026-06-03.

**CAPACITY · HORIZON · DEPTH · PERCEIVE — the four Cognitive Geometry verbs from Davis's *Cognitive Geometry Correspondence* (Branch VII, Theorems 8.1 / 8.6 / 8.14).** Where the older Kähler analytics expose static geometric scalars (K, λ₁, holonomy_debt, …), the CG verbs translate those into builder-facing routing decisions: *can the substrate hold this interpretation?* (CAPACITY = τ/K), *how deep does coherent context extend before the accumulated frame rotation becomes irrecoverable?* (HORIZON = τ/(K·ℓ_c)), *what's the erasure energy of writing here?* (DEPTH classifier I/II/III/IV), and *what does the substrate actually perceive this vector to be after parallel transport, and how much should we trust that perception?* (PERCEIVE = (R_acc·v, ‖R_acc−I‖_F)).

All four ship with HTTP endpoints (`GET /v1/bundles/{name}/capacity`, `…/horizon`, `…/depth`, `POST …/perceive`), GQL verbs, backwards-compatible config surfaces. 35 new tests; 1082 lib tests with `kahler`, 841 no-feature, 0 regressions.

---

## Late May 2026 — GIGI Encrypt v0.3 + v0.4 ship

The encryption surface jumped two minor versions in one window. v0.3 fleshed out the gauge-mode primitives and shipped the full delegation family. v0.4 added the verification layer that turns the invariant tuple into a public, deterministic audit primitive.

### v0.3 — gauge-mode completion + delegation family

| Sprint | What it adds |
|---|---|
| **I** Curvature-MAC | HMAC-SHA256 over the canonical π_inv tuple. Tag changes iff the bundle's *invariants* change, regardless of gauge. |
| **J.1** Aff(ℝ) delegation | Compose two `GaugeKey`s' transforms into a per-field capability the proxy applies on ciphertext. Honestly labeled: **not collusion-resistant** — *capability delegation*, not PRE. |
| **J.2** Pairing-based PRE | BLS12-381 Ateniese-Hohenberger 2005. Delegatee-vs-proxy collusion resistance reducing to DLP on G₂ (~2^128 work). Pre-quantum by design. |
| **J.3** ML-KEM trusted delegation | FIPS 203 ML-KEM-768 (post-quantum KEM, NIST Level 3). Trust model: trusted delegatee. Closes the BLS12-381 quantum gap. |
| **J.4** Lattice threshold delegation | Shamir K-of-N over F_p + per-share ML-KEM-768 envelope. Closes the **PQ + collusion-resistance** gap structurally for K-of-N quorum trust. |
| **K** Holonomy ledger | RFC 6962 Merkle audit log over per-write leaves; gauge-invariant. |
| **L** Čech threshold | Same Shamir-over-F_p that J.4 reuses, surfaced as a primitive. |
| **M** RG-flow ratchet | HKDF chain for continuous forward secrecy on the integrity key. |

**Honest carveout shipped with v0.3**: `decrypt_min` / `decrypt_max` / `decrypt_range` **refuse** Probabilistic gauges with σ > 0 — order statistics don't commute with additive Gaussian noise. Bias is `Θ(σ √(2 log n) / |a|)` and doesn't vanish as n → ∞. Rigor suite (Rust 25 + Python 66/66) locks the behavior.

### v0.4 — invariant verification + the four follow-up sprints

| Sprint | What it adds |
|---|---|
| **N** Invariant Consistency Verification | Public deterministic verification that π_inv = (K, λ₁, ⟨Hol⟩, τ, β₀, β₁) agrees with the bundle's computed tuple. No gauge key required. Bundle-id binding. HTTP: `POST /v1/bundles/{name}/verify_invariant`. |
| **O** Credential-Gated Invariant Queries | HMAC-SHA256-bound credentials; constant-time tag comparison; typed domain separator. BBS+ unlinkability pinned as v0.5. |
| **Q** K-Preserving Transformation Characterization | Identifies the **diagonal affine group** `(ℝ*)ᵏ ⋉ ℝᵏ` as the exact K-preserving subgroup. **Roadmap only — not a shipped PQ mode.** |
| **P** Geodesic-Ball Membership Index | Chi-square / Mahalanobis dimension-aware threshold. Explicit leakage scope: index reveals centroid + covariance + count. |

Every Sprint N–P primitive has a parallel Python oracle in `theory/encryption/validation/`. Rust + Python agree to 1e-10 on every cross-checked assertion.

### Numbers

```
Rust  lib (--features kahler):                999 / 999  pass
Rust  lib (no-feature):                       781 / 781  pass
Rust  integration (50+ test binaries):        all "0 failed"
Python  FHE/PQ rigor oracle:                   66 / 66
Python  Sprint N oracle:                       17 / 17
```

### Paper

**Published on Zenodo, 2026-05-29:** Davis, B. R. (2026). *Geometric Encryption: Property-Preserving Database Encryption via Gauge Invariance on Fiber Bundles.* Zenodo. [10.5281/zenodo.20438796](https://doi.org/10.5281/zenodo.20438796). 28 pp / 731 KB. Twelve worked Alice/Bob examples in Appendix A; per-mode leakage profiles graded under the Chase-Kamara structured-encryption taxonomy; formal BDH security reduction for BLS12-381 pairing-PRE; lattice-threshold + ML-KEM-768 PQ delegation modes.

---

## Late May 2026 — the SUDOKU + SAMPLE_TRANSPORT sprint

Six waves of work landed on top of the brain catalog, taking the substrate from "we have 12 brain primitives" to "we have a constrained-inference meta-primitive that solves real problems across unrelated domains" plus a neighborhood-sampling primitive.

The work shares the same Davis-manifold machinery as the **sudoky-energy** sister project (Bee Davis, U.S. Provisional Patent Feb 2026 — a GPU-accelerated CSP solver using `K_loc` curvature scheduling + `V(c)` information value + Γ trichotomy routing + holonomy pruning).

### SUDOKU — constrained inference on a learned affordance manifold

The primitive: a consumer hands SUDOKU a constraint set; it returns satisfying records, near-miss records, a Pareto frontier of multi-violation alternatives, a counterfactual relaxation menu, per-constraint diagnostics, and an **honest tristate verdict** — `Sat` / `Unsat` / `Unknown`.

| Wave | What it adds |
|---|---|
| **W3** | Per-violation `relaxation_cost` (Kähler-natural z-score). `SelectivityReport`. `RelaxationOption` menu. |
| **W4** | `Solution.quality_score` (soft-constraint posterior under independent half-normal priors). `Eq(Vector)` upgraded to bundle-derived L2 distance. |
| **W5** | `ParetoNearMiss` — Pareto frontier on (n_violations, total_cost). |
| **W6.1** | `SelectivityReport.raw_curvature` — `K_c` = fraction of records that fail this constraint regardless of others. |
| **W6.2** | Čech-cohomology **holonomy pre-flight** — O(C²) pairwise scan for trivially self-contradictory constraint pairs. Fires before any record IO. |
| **S3.5** | **Puzzle expansion** — when UNSAT, opt-in walks the relaxation menu until ≥1 solution found. |

Wire surface: `POST /v1/bundles/{name}/brain/sudoku`. Verified end-to-end across **24 distinct domains** in the demo set.

### SAMPLE_TRANSPORT — curvature-bounded neighborhood sampling

When deterministic `TRANSPORT` returns one geodesic, `SAMPLE_TRANSPORT` returns a neighborhood of `k` valid destinations within a curvature budget τ. Candidates weighted by `exp(-β · d²)`, sampled via the **Efraimidis-Spirakis priority algorithm**.

### Worked-example demos

Eight self-contained demos under `e2e/probes/` — each exercises a slice of functionality across distinct domains: `sudoku_six_domains_demo.py`, `sudoku_six_more_domains_demo.py`, `sudoku_geometry_diagnostics_demo.py`, `sudoku_expansion_demo.py`, `sudoku_at_scale_demo.py`, `sudoku_32x32_grid_demo.py`, `sample_transport_demo.py`, `preship_audit.py`.

### #107 — brain endpoints work on reloaded (mmap+overlay) bundles

Pre-existing limitation closed: every brain endpoint had the guard `as_heap().ok_or(404)`, so after any server restart bundles reloaded from snapshot became inaccessible until manual recreation. Fix: `OverlayBundle::to_temp_heap_store()` materializes the merged view into a fresh heap store in ~10ms per 10k records. **15 brain endpoints updated; live verified after deploy (4,961 bundles / 12.8M records reloaded with zero loss).**

---

## 2026-05-29 — encryption paper deposit + vector-search cache

**Geometric Encryption paper deposited on Zenodo.** Davis, B. R. (2026). *Geometric Encryption: Property-Preserving Database Encryption via Gauge Invariance on Fiber Bundles.* Zenodo. [10.5281/zenodo.20438796](https://doi.org/10.5281/zenodo.20438796). 28 pp / 731 KB. Theorem 3.3 (ρ-equivariant ciphertext-computability), five-mode taxonomy (Affine / Opaque / Indexed / Probabilistic / Isometric), v0.3 cryptographic suite, v0.4 follow-ups. Marketing page with interactive demo at [davisgeometric.com/gigi/gigi-encrypt](https://davisgeometric.com/gigi/gigi-encrypt).

**New `vector_cache` module** ([`src/vector_cache.rs`](src/vector_cache.rs)). General-purpose primitive backing the vector-search brain endpoints. Cached `(N, D)` materialized matrices with mutation-counter invalidation and per-key single-flight compute on miss. New operator-facing env var `GIGI_VECTOR_CACHE_SIZE` (default 64). 21 new unit tests.

---

## 2026 — the Kähler upgrade

GIGI v3 shipped the **Kähler upgrade**: twelve layers (L1–L7, L8 cross-team handoff, L9 moment maps, L10 generative flow, L11 predictive coding, L12 attention + memory) of geometric machinery extending the fiber-bundle substrate with a complex structure J, a closed 2-form B, and everything that falls out of the pair — Hadamard substructure detection, holomorphic curvature decomposition, Morse compression, line-bundle integrality checks, quantum cohomology on toy manifolds, Berezin-Toeplitz operators, Riemann-Roch representational capacity, moment-map / Noether conservation along Hamiltonian B-flows, **the Friston-FEP keystone — generative flow on the Kähler bundle that parametrizes its boundary conditions to deliver SAMPLE / FORECAST / DREAM / RECONSTRUCT as one piece of infrastructure**, **predictive-coding primitives stacked on top: INPAINT, PREDICT, and SELF-MONITOR**, and **the attention + memory pillar: ATTEND, FOCUS, EPISODIC, SEMANTIC**.

All 12 brain primitives operational. The Kähler catalog ([`theory/kahler_upgrade/`](theory/kahler_upgrade/)) closes at **16 of 21 items shipped** — 100% of items the catalog itself classified as ship-able.

GIGI ships **three companion catalogs**:

- [`theory/kahler_upgrade/`](theory/kahler_upgrade/) — the 21-item Kähler catalog with 16 shipped; 15/15 Python validation tests pass.
- [`theory/post_kahler_directions/`](theory/post_kahler_directions/) — nine **post-Kähler** geometric programs (Sasaki, information geometry, OT/Wasserstein, persistent homology, Gromov δ-hyperbolicity, tropical, synthetic DG, NCG, CAT(κ)); 30/30 numerical checks pass.
- [`theory/brain_primitives/`](theory/brain_primitives/) — the Sudoku-10× reading. **Twelve brain-like operations forced by one master equation** `ẋ = B⁻¹∇(−log p)` on the Kähler bundle — the same equation Friston writes down for variational free-energy minimization. 26/26 numerical checks pass.

Three properties worth calling out:

**1. Strict additivity. The optionality contract holds across all twelve layers.** The entire Kähler upgrade lives behind a single Cargo feature flag (`kahler`). With the feature off, the engine is **bit-identical to pre-upgrade GIGI**. With the feature on, 821 tests pass.

**2. Math predictions validated by production observation to rounding precision.** The first downstream consumer (Marcella) ran a 30-prompt A/B harness + 10-turn deep-trace on her actual embedding substrate. Perfect monotonicity: 21/21 reply-different when the residue moved, 9/9 byte-identical when it didn't. Peak per-turn Δ-residue measured at **0.0747**, matching the closed-form non-associativity bound of **7.6pp** to within rounding (0.0013). The deep-trace held coherence through 86° accumulated rotation across 10 turns — exactly 10 × 8.6° per turn, linear.

**3. Geometric machinery doing real work in user-facing behavior.** The non-associativity meter that started as a math sanity check turned out to be a **conversation-stationarity signal**: 4-of-4 stationary sessions show monotonic decay at ~2pp per turn toward the calibrated floor.

The full audit trail is in [`theory/kahler_upgrade/`](theory/kahler_upgrade/) and [`tests/kahler_*`](tests/).
