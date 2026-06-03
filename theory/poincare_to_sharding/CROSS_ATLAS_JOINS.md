# Cross-Atlas Joins

**Open question §10.4 of `SHARDING_SPEC.md`, addressed.**
**Theory + design + new TDD gates needed (T8, T9, T10).**
**Status:** Theory draft v0.1; TDD gates not yet attempted.
**Gated on:** `poincare_to_sharding.md` §2.2 (Geometry of Sameness functors); future tests T8–T10 under `validation/`.

---

## 1. The problem

Two production bundles need to share data via a cross-bundle query:

- **Marcella's `marcella_source_embeddings_bge_v2`** — 9,964 records, sharded by the Phase D Fiedler-vector partition over its own atlas $A_M$. Records are BGE-v2 embeddings of grounding-corpus chunks; the configuration manifold is approximately a 768-dimensional cluster manifold with $K \approx 4$ per-shard.
- **PRISM's reconciliation manifold** — financial reconciliation bundle with its own atlas $A_P$. Records are transaction representations; the configuration manifold is the Davis Field Equations payment-reconciliation manifold per the PRISM paper.

A consumer query: *"for this Marcella embedding, find the PRISM transactions whose semantic similarity is above threshold via cross-bundle parallel transport."* This requires:

1. A notion of "common manifold structure" between $A_M$ and $A_P$ — there must be a shared semantic sameness structure $S_{\text{shared}}$ that both bundles realize.
2. Bridge transitions from charts of $A_M$ to charts of $A_P$ that compose into the unified manifold structure.
3. An engineering API that doesn't require copying records between bundles.

The single-atlas sharding math (§3.1 – §3.6 of `poincare_to_sharding.md`) gives us *intra*-atlas composition. The Geometry of Sameness functors $F_S, G_S$ give us the *cross*-atlas bridge.

---

## 2. The Geometry of Sameness reading

For a fixed semantic sameness structure $S_{\text{shared}}$ realized by both bundles, *Geometry of Sameness* §3 + §4 + §5 give us two categories:

$$\SamTrans(S_{\text{shared}}) \qquad \SamGeom(S_{\text{shared}})$$

and explicit functors

$$F_{S_{\text{shared}}}: \SamTrans^0(S_{\text{shared}}) \to \SamGeom^0(S_{\text{shared}}), \qquad G_{S_{\text{shared}}}: \SamGeom^0(S_{\text{shared}}) \to \SamTrans^0(S_{\text{shared}})$$

The Marcella atlas $A_M$ is a translator-based realization of $S_{\text{shared}}$ (an object of $\SamTrans(S_{\text{shared}})$). Apply $F_{S_{\text{shared}}}$ to lift it to a manifold-based realization $M_M = F_{S_{\text{shared}}}(A_M)$. Same for PRISM: $A_P \mapsto M_P = F_{S_{\text{shared}}}(A_P)$.

By the **Error Budget Transfer Theorem** (Davis 2026b §6.3),

$$\bigl| \mathrm{LB}_{\mathrm{T}}(A_M) - \mathrm{LB}_{\mathrm{M}}(M_M) \bigr| \le C_F\bigl(\varepsilon_{\mathrm{trans}}^{A_M}, \varepsilon_{\mathrm{dist}}^{M_M}, \delta_{\mathrm{chart}}\bigr)$$

i.e., the detection guarantees of the two views agree up to first-order slack. Crucially, both $M_M$ and $M_P$ are now manifold-based realizations of the *same* $S_{\text{shared}}$, so they share a common substrate. The cross-atlas join is the **fiber-product of the two atlases over $S_{\text{shared}}$**.

There are two engineering choices:

**Choice A — Lift to common SamGeom.** Convert both atlases to their manifold realizations $M_M, M_P$ via $F_{S_{\text{shared}}}$. Both live in the same manifold-based realization category; the join is a single SamGeom-level operation. **Cost:** evaluating $F_S$ requires building the translator metric quotient (Davis 2026b Def 18), which is a non-trivial operation per atlas at join time.

**Choice B — Bridge via shared SamTrans.** Construct a common SamTrans realization by extending each atlas's translator graph with **bridge translators** that connect charts of $A_M$ to charts of $A_P$. The bridge translators carry their own Lipschitz constants and cocycle slack budgets. **Cost:** each bridge translator must be either learned or derived from $S_{\text{shared}}$'s rendering maps $\pi_i$; this is amortized at bundle-pair declaration time.

For production GIGI, **Choice B is the right engineering primitive** — it amortizes the bridge cost upfront, allows incremental updates as either atlas evolves, and falls out naturally from the existing atlas storage format (the `Transition` type from `src/sharded/atlas.rs` already handles cross-chart translators; it just needs a cross-atlas variant).

---

## 3. Bridge transitions and the cross-atlas cocycle condition

A **bridge transition** $B_{ij}^{MP}: V_i^M \to V_j^P$ maps a chart-coordinate in atlas $A_M$ to a chart-coordinate in atlas $A_P$, on overlap $U_i^M \cap U_j^P \cap S_{\text{shared}}$.

The **cross-atlas cocycle condition** extends Davis 2026b Definition 21 to span both atlases:

For any triple $(i, j, k)$ with $i, j \in A_M$ and $k \in A_P$:

$$\bigl\| B_{jk}^{MP}\bigl(T_{ij}^M(v_i)\bigr) - B_{ik}^{MP}(v_i) \bigr\| \le \delta_{\mathrm{cocycle}}^{\mathrm{bridge}}$$

For triples spanning the opposite direction $(i \in A_M, j, k \in A_P)$:

$$\bigl\| T_{jk}^P\bigl(B_{ij}^{MP}(v_i)\bigr) - B_{ik}^{MP}(v_i) \bigr\| \le \delta_{\mathrm{cocycle}}^{\mathrm{bridge}}$$

And for the "round-trip" condition (if both directions exist):

$$\bigl\| B_{ji}^{PM}\bigl(B_{ij}^{MP}(v_i)\bigr) - v_i \bigr\| \le \delta_{\mathrm{asymm}}^{\mathrm{bridge}}$$

These three conditions together generalize the intra-atlas cocycle bound to the cross-atlas setting. The bound $\delta_{\mathrm{cocycle}}^{\mathrm{bridge}}$ is a **new structural budget** specific to the bundle-pair $(A_M, A_P)$, declared upfront and enforced by gates.

---

## 4. Engineering API design

A new type extending `src/sharded/atlas.rs`:

```rust
/// Bridges two atlases over a shared semantic sameness structure.
/// Each bridge atlas stores its own cocycle budget and Lipschitz
/// estimates for the bridge translators.
pub struct BridgeAtlas {
    pub source_atlas_name: String,
    pub target_atlas_name: String,
    /// Bridge transitions: (source_chart, target_chart) -> Transition.
    pub bridges: HashMap<(ChartId, ChartId), Transition>,
    /// The cross-atlas cocycle budget.
    pub delta_bridge_budget: f64,
    /// The round-trip asymmetry budget (if bidirectional).
    pub delta_bridge_asymm: f64,
    /// Declaration of which spectral regime the SHARED structure has.
    /// Need not match either source or target individually.
    pub shared_spectral_regime: SpectralRegime,
}
```

A new sharded execution recipe for cross-atlas TRANSPORT:

```rust
pub fn cross_atlas_transport(
    source_atlas: &Atlas,
    target_atlas: &Atlas,
    bridge: &BridgeAtlas,
    source_point: (ChartId, Vec<f64>),
    target_point: (ChartId, Vec<f64>),
) -> Result<RotationMatrix, ShardedExecError> {
    // 1. Transport source_point to a chart in source_atlas adjacent
    //    to the bridge target chart (intra-atlas, recipe §5.4).
    // 2. Apply bridge transition B_ij^MP to lift into target_atlas.
    // 3. Transport in target_atlas to target_point (intra-atlas).
    // 4. Compose all rotation matrices.
    //
    // Total cost: O(intra-atlas path) per side + O(1) bridge cost.
    // Verifies cross-atlas cocycle condition along the way.
    todo!("Phase F")
}
```

And for cross-atlas joins:

```rust
pub fn cross_atlas_join_query(
    source_atlas: &Atlas,
    target_atlas: &Atlas,
    bridge: &BridgeAtlas,
    source_query: &GeometricQuery,
    semantic_threshold: f64,
) -> Result<Vec<TargetMatch>, ShardedExecError> {
    // For each candidate target record c in target_atlas:
    //   1. Compute the geodesic distance from source_query to c via
    //      cross_atlas_transport.
    //   2. Apply the semantic threshold.
    // Returns matches with their cross-atlas geodesic distances.
    todo!("Phase F")
}
```

---

## 5. New TDD gates needed (T8–T10)

To gate the cross-atlas math before any implementation lands:

### T8 — Bridge transition cocycle bound

**Claim:** for an analytic shared structure $S_{\text{shared}}$, bridge transitions $B_{ij}^{MP}$ derived from $S_{\text{shared}}$'s rendering maps $\pi_i^M, \pi_j^P$ satisfy the cross-atlas cocycle exactly ($\delta_{\mathrm{cocycle}}^{\mathrm{bridge}} = 0$). For learned bridges with perturbation $\varepsilon$, the discrepancy is first-order in $\varepsilon$.

**Setup:** S² with two stereographic atlases $A_1, A_2$ chosen with overlapping but distinct chart layouts. Build closed-form bridge transitions from the shared sphere geometry. Validate the cross-atlas cocycle condition analytically + perturbed (mirrors T2).

**Pass criterion:** analytic bridge cocycle discrepancy < 1e-12; perturbed slope 0.9–1.1 (first-order).

### T9 — Cross-atlas BETTI via fiber-product Mayer-Vietoris

**Claim:** when the bridge transitions identify common overlap simplices, the global Betti numbers of the union $A_1 \cup_{\text{bridge}} A_2$ are recoverable from per-atlas Betti + bridge-overlap Betti via an extended Mayer-Vietoris.

**Setup:** two triangulations of the same closed surface (e.g., two different triangulations of $T^2$) bridged by an identifying transition. Compute Betti via fiber-product M-V vs direct on the union (where the identification has been resolved).

**Pass criterion:** Betti numbers equal to direct simplicial homology of the identified union.

### T10 — Cross-atlas write-conflict resolver

**Claim:** when a write batch contains conflicts spanning both atlases (some in $A_1$, some in $A_2$, some at bridges), the Clean Finger Move resolver (T6) extends to the cross-atlas case as long as the H_2 = 0 precondition holds across the bridge: every conflict has a canceling partner, possibly in the *other* atlas.

**Setup:** mixed conflict graph with conflicts in atlas 1, atlas 2, and bridge overlaps. Pairings span all three. Run the resolver.

**Pass criterion:** terminates in N/2 steps, no new conflicts; matches T6 termination guarantee.

---

## 6. Migration path

Cross-atlas joins are **Phase F**, slotting in after Phase E (Schur complement / distributed Lanczos for expanders, gated on T7 — already GREEN as of commit `1aba0d8`):

- **Phase F.0 (theory + TDD)**: implement T8, T9, T10 in `validation/`. Gate cross-atlas claims on green tests.
- **Phase F.1 (skeleton)**: add `BridgeAtlas` type to `src/sharded/`. Stubs return `NotImplementedYet`.
- **Phase F.2 (analytic bridges)**: implement bridge construction for closed-form shared structures (CP¹, T², etc.).
- **Phase F.3 (learned bridges)**: implement bridge construction from $S_{\text{shared}}$'s rendering maps via learned approximations. Cocycle slack is the perturbation budget.
- **Phase F.4 (production)**: wire up `cross_atlas_join_query` HTTP endpoint, contract-tested against Marcella + PRISM.

---

## 7. Pre-conditions for the Marcella + PRISM bridge

Before Phase F can target this pair specifically, we need:

1. **Identification of $S_{\text{shared}}$.** For Marcella + PRISM: probably some financial-grounding semantic space. Both bundles render into this space differently — Marcella's grounding-corpus chunks vs. PRISM's transaction tuples. We need to specify what "same entity" means across these.

2. **Rendering map agreement.** $\pi_i^M$ and $\pi_j^P$ must agree on $S_{\text{shared}}$'s rendering: an entity $u \in S_{\text{shared}}$ has rendered observations in both Marcella's and PRISM's spaces, with a known *transformation* between them (the bridge translator).

3. **Cocycle budget declaration.** The bundle-pair declares $\delta_{\mathrm{cocycle}}^{\mathrm{bridge}}$ at registration. Recommended starting value: $0.05$ (5% bridge slack budget, consistent with Marcella's other slack budgets).

4. **Bridge transition implementation.** Either closed-form (if $S_{\text{shared}}$ has a known structure) or learned (if the bridge is empirical). For the Marcella + PRISM case, likely learned — a small neural network mapping Marcella embedding coordinates to PRISM transaction coordinates, trained on entity pairs with known cross-bundle correspondence.

These pre-conditions are NOT specific to GIGI — they are the engineering shape of any cross-database semantic join under the Davis Geometric framework. The PRISM paper already specifies the relevant rendering maps for the financial-transaction side; Marcella's side is specified by the corpus-chunking pipeline.

---

## 8. Why this matters

Once Phase F lands, GIGI supports:

- **Cross-bundle parallel transport** along bridge translators, with first-order slack guarantees.
- **Cross-bundle BETTI** via fiber-product Mayer-Vietoris, exact for analytic bridges.
- **Cross-bundle write-conflict resolution** via the extended Clean Finger Move resolver, terminating in O(N) steps.
- **Cross-bundle semantic search** as a database primitive, not an ETL job.

This is what "different DBs can compose without losing geometric structure" looks like in practice. It is also the engineering form of the *Geometry of Sameness* equivalence: SamTrans ≃ SamGeom up to first-order slack, lifted to the multi-database setting.

The "huge client" who will need scale (per the conversation that motivated this sprint) will almost certainly need cross-bundle joins on day one — most production deployments span multiple data domains. Phase F is the gate they have to clear, and the math is ready as soon as T8–T10 green.

---

## References

- Davis, B. R. (2026a). *The Davis Manifold.* Manuscript. §A5 non-vacuity.
- Davis, B. R. (2026b). *The Geometry of Sameness.* Manuscript. §3 categories, §4 F_S, §5 G_S, §6 Error Budget Transfer Theorem.
- Davis, B. R. (2026c). *Smooth 4D Poincaré.* Manuscript. §5 Clean Finger Move.
- Davis, B. R. (2025). *PRISM: Payment Rail Integration via Semantic Matching.* Marketing page + paper.
- `theory/poincare_to_sharding/poincare_to_sharding.md` §2.2, §3.6.
- `theory/poincare_to_sharding/SHARDING_SPEC.md` §10.4 (cross-atlas joins — this document addresses it).
