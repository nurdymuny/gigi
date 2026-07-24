# GIGI Post-Kähler Extensions (Phase 1)
**Specification v0.1**

This specification operationalizes the first four directions of the Post-Kähler geometry catalog (`theory/post_kahler_directions/catalog.md`). These form the "Low Integration Cost" tier, building directly upon the existing streaming statistics, Hodge complexes, and graph structures shipped in the original Kähler upgrade.

This document defines the mathematical contract, the GQL grammar, the REST API endpoints, and the strict TDD gates required to claim these features as shipped.

---

## 1. Mathematical Contract & Validation

As with the Kähler upgrade, GIGI does not guess at geometry. Every implementation must tie exactly to the closed-form proofs and python validation scripts in `theory/post_kahler_directions/validation_tests.py`.

*   **PK-1 (Sasaki/Contact)**: Must exactly replicate the Reeb flow $\mathcal{L}_R \alpha = 0$ on the standard contact space $\mathbb{R}^3$. Validated against `test_1_sasaki_contact_reeb_flow`.
*   **PK-2 (Fisher Metric)**: Must extract the Fisher Information Metric natively from the variance structure of Gaussian distributions tracked via Welford statistics. Validated against `test_2_information_geometry_fisher_on_gaussians`.
*   **PK-3 (Wasserstein/OT)**: Must accurately calculate the 2-Wasserstein distance $W_2$ via the monotone rearrangement for 1D distributions, respecting Hoeffding's bound. Validated against `test_3_optimal_transport_wasserstein_gaussians`.
*   **PK-4 (Persistent Homology/TDA)**: Must extract $H_0$ (clusters) via minimum spanning tree (MST) edge reduction and return stable persistence intervals. Validated against `test_4_persistent_homology_clusters`.

---

## 2. GQL Grammar & API Surface

### 2.1 Sasaki / Contact Geometry (Time-Series / Attention)
For bundles modeled over sequences (e.g. time-series, or token embeddings), we introduce the Reeb vector field to find invariant positional flows.
*   **GQL Grammar**: 
    ```sql
    SECTION bundle ALONG REEB FLOW OF (alpha) LIMIT 10;
    ```
*   **API**: `POST /v1/bundles/{name}/brain/reeb_flow`

### 2.2 Information Geometry (Natural Gradients)
Turns statistical variance (already tracked) into a Riemannian metric, permitting exact natural-gradient queries.
*   **GQL Grammar**: 
    ```sql
    SELECT NATURAL_GRADIENT(metric) ON FIBER (f1, f2) WITH FISHER;
    ```
*   **API**: `GET /v1/bundles/{name}/fisher_metric` (returns the exact diagonal/covariance metric tensor).

### 2.3 Optimal Transport (Wasserstein Distances)
Measures the structural $W_2$ distance between two sets of data or distributions, circumventing flat $L^2$ norms.
*   **GQL Grammar**:
    ```sql
    WASSERSTEIN (cohort_A) TO (cohort_B) ON FIBER (age, income);
    ```
*   **API**: `POST /v1/bundles/{name}/ml/wasserstein`

### 2.4 Persistent Homology (TDA)
Exposes the multi-scale topological fingerprint of a bundle, revealing stable clusters ($H_0$) and cycles ($H_1$).
*   **GQL Grammar**:
    ```sql
    PERSISTENCE DIAGRAM ON bundle WHERE (time > '2026-01-01');
    ```
*   **API**: `GET /v1/bundles/{name}/topology/persistence` (returns `[birth, death)` intervals).

---

## 3. Gated TDD Rollout

The rollout is guarded by four primary gates. The feature flag for this phase is `post_kahler_phase1`.

### Gate PK-1: Sasaki & Contact Forms
- **Target**: `src/geometry/contact.rs`
- **Criteria**: Define `ContactOneForm` and `ReebField`. Implement the non-degeneracy check $\alpha \wedge (d\alpha)^n \neq 0$. 
- **Validation**: Reeb vector must preserve $\alpha$ precisely to machine epsilon.

### Gate PK-2: Fisher Metric Extraction
- **Target**: `src/geometry/fisher.rs` & `BundleStore::welford_stats`
- **Criteria**: Hook into the existing $L4$ Welford streaming stats. When a field is typed as univariate Gaussian, the Fisher metric must be automatically surfaced.
- **Validation**: $g_{\mu\mu}$ and $g_{\sigma\sigma}$ must match analytic expectations for $N(\mu, \sigma^2)$.

### Gate PK-3: Wasserstein $W_2$ Metric
- **Target**: `src/geometry/wasserstein.rs`
- **Criteria**: Implement 1D closed-form Wasserstein distance for empirical CDFs using the monotone rearrangement.
- **Validation**: Compute $W_2$ between two point clouds; must be strictly bounded by random pairings (Hoeffding's bound).

### Gate PK-4: Persistent Homology ($H_0$)
- **Target**: `src/discrete/persistent_homology.rs`
- **Criteria**: Build on the existing `HodgeComplex` from $L6$. Implement the Vietoris-Rips filtration and extract $H_0$ persistence intervals using union-find on MST edges.
- **Validation**: Gap between cluster-merge edge weights must cleanly separate distinct Gaussian blobs.

---
*Signed, Bee Rosa Davis, 2026.*
