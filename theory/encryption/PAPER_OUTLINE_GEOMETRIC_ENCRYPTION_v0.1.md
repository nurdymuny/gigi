# Geometric Encryption: Property-Preserving Database Encryption via Gauge Invariance on Fiber Bundles

**Author:** Bee Rosa Davis · Davis Geometric
**Working title** (alt: "Compute on Encrypted Data at Native Speed: The Gauge-Invariance Framework")
**Target venue:** VLDB / SIGMOD main track + IACR ePrint companion
**Code:** [github.com/nurdymuny/gigi](https://github.com/nurdymuny/gigi/)
**Related patent:** U.S. Provisional Application No. 64/045,889 (GIGI); v0.3 continuation-in-part to be filed before submission
**Date:** 2026-05-28
**Status:** Outline + §1–§3 drafted; §4–§11 to follow

---

## Abstract (target ~250 words)

We introduce **geometric encryption**, a property-preserving database-encryption framework in which the encryption is a gauge transformation on the fiber of a fiber-bundle data store. The structure group of the gauge determines, by construction, which queries are computable on ciphertext: a query is gauge-invariant iff it can be evaluated on encrypted data without decryption.

We give a closed taxonomy of five encryption modes, each parameterized by its structure group:

- **Affine** — Aff(ℝ); preserves curvature K = Var/range², order, range queries
- **Opaque** — full bijection; preserves nothing (AES-GCM-SIV)
- **Indexed** — PRF image; preserves equality only (AES-256-CMAC)
- **Probabilistic** — Aff(ℝ) ⋉ N(0,σ²); preserves K within noise + σ-bucket equality
- **Isometric** — O(k) on grouped numeric; preserves exact pairwise distance, holonomy spectrum, K

On top of this taxonomy we develop five higher-level constructions: **Curvature-MAC** (HMAC over the gauge-invariant content fingerprint π_inv = (K, λ_1, C, ⟨Hol⟩, β_0, β_1), gauge-rotation-invariant); **Aff(ℝ) capability delegation** (zero-decrypt re-encryption with explicit trusted-delegatee threat model — *not* collusion-resistant PRE); **holonomy ledger** (RFC 6962 Merkle tree over per-write events, extended with `record_hash` leaves for byte-level tamper-evidence); **Čech threshold sharing** (Shamir over secp256k1 base field F_p with HMAC-bound Čech cocycle authentication); **continuous RG-flow ratchet** (per-write HKDF chain with checkpoint-period replay, providing forward secrecy at every write boundary).

All eight modes and five constructions ship with passing TDD test surfaces in the public GIGI Rust codebase (820+ tests). We document one floating-point limitation found empirically: f64 re-encryption introduces ~10⁻⁹ noise on capacity = τ/K, requiring 6-decimal-digit quantization of the invariant tuple before HMAC for cross-rotation tag equality. We further state three constructions (geodesic-ball ABE, spectral-signature ZKP, lattice-fiber post-quantum gauge) as derived but with reference implementation deferred to a successor paper.

---

## §1. Introduction

### §1.1 The compute-on-ciphertext problem

Three decades after Rivest, Adleman, and Dertouzos posed it (1978), the question "how do we compute on encrypted data?" remains the central tension of database confidentiality. The dominant answers cluster into two families with sharp trade-offs:

1. **Fully homomorphic encryption** (Gentry 2009 and successors) provides universality at a 10³–10⁵× slowdown. Any function on plaintext can be evaluated on ciphertext, at a cost that has slowly come down but remains incompatible with production latency budgets for analytical queries.
2. **Transparent data encryption** (Oracle TDE, AWS RDS encryption, MongoDB CSFLE) provides at-rest confidentiality at a 1× cost — but requires decryption to compute. Every analytical query passes through plaintext at some point on a privileged node.

Between these poles sits a smaller family of **property-preserving encryption** (PPE) constructions: order-preserving encryption (Boldyreva 2009 / Popa CryptDB 2011), structured encryption (Chase-Kamara 2010), searchable symmetric encryption (Curtmola et al. 2006), and functional encryption (Boneh-Sahai-Waters 2011). Each preserves one specific property (order, equality, a chosen function class). None offer a unifying framework that says "encryption mode X preserves query class Y because of mathematical reason Z."

We provide such a framework.

### §1.2 The geometric move

The Davis framework~\cite{davis2026sheafcomposition,davis2026manifold} represents a database as sections of a fiber bundle $(E, B, F, \pi, \Phi)$, where $B$ is the base space (keys), $F$ is the fiber (non-key field values), and $\pi: E \to B$ is the projection. A record is a section $\sigma: p \mapsto (p, v_1, \ldots, v_k) \in E$.

The framework's analytics — curvature, anomaly detection, spectral connectivity, prediction — are functions on the bundle's intrinsic geometry. **Crucially**, they are invariants of the bundle's geometric structure: they do not depend on which coordinate system the fiber is expressed in.

Geometric encryption is the observation that the choice of coordinate system on the fiber **is** the encryption. A secret gauge transformation $g \in G \subset \mathrm{Aut}(F)$ scrambles each record's fiber values $(v_1, \ldots, v_k) \mapsto g(v_1, \ldots, v_k)$. An attacker who sees only the gauged data cannot recover plaintext (without knowing $g$); but anyone — including analysts without decryption privilege — can compute any $G$-invariant function on the gauged data and get the same answer as on plaintext.

The encryption is not a layer bolted onto the database. **The geometry is the encryption.**

### §1.3 What this paper adds

1. **A closed taxonomy** of five encryption modes (Affine, Opaque, Indexed, Probabilistic, Isometric) keyed by their structure groups. Each row of the taxonomy is a triple (structure group $G$, AEAD/PRF primitive, query class preserved).
2. **Five higher-level constructions** built on the taxonomy: Curvature-MAC bundle integrity, Aff(ℝ) capability delegation, holonomy ledger (Merkle audit log), Čech threshold sharing, continuous RG-flow ratchet.
3. **A reference implementation** in the public GIGI Rust codebase, with TDD test surface (820+ passing tests) backing every claim. Each math primitive ships with both Rust integration tests and Python math-validation mirrors.
4. **Honest scope statements** for each construction's threat model. Particular: Aff(ℝ) capability delegation is *not* collusion-resistant PRE in the Ateniese-Hohenberger sense; we document the collusion attack explicitly and offer the trusted-delegatee threat model as the operational fit, pointing to Umbral as the collusion-resistant alternative for adversarial delegatees.
5. **Three derived-but-unimplemented constructions** (geodesic-ball ABE, spectral-signature ZKP, lattice-fiber PQ gauge) with security arguments, deferred to a v0.4 successor paper. The Davis-Manifold pattern: publish the construction, defer the production implementation until peer review of the construction.

### §1.4 What this paper is not

- *Not* a fully homomorphic encryption alternative. FHE achieves universality at a cost; we achieve specific invariant computability at native speed. Different points on the Pareto frontier.
- *Not* a confidentiality system on its own. The framework provides **selective compute on encrypted data**; pairing with AEAD on confidentiality-critical columns is necessary for full data confidentiality.
- *Not* a post-quantum primitive at v0.3. The Aff(ℝ) and HMAC-SHA256 primitives we use are pre-quantum. The PQ extension (lattice-fiber gauge) is in the deferred constructions section.

### §1.5 Roadmap

§2 reviews the prior canon — three Davis manuscripts that this work extends — plus the external math anchors. §3 states the **gauge-invariance theorem**: a query is computable on encrypted data iff it is gauge-invariant under the chosen structure group. §4 catalogs the five-mode taxonomy. §5 develops the Curvature-MAC integrity construction. §6 develops Aff(ℝ) capability delegation including the honest collusion-limitation. §7 develops the holonomy ledger. §8 develops Čech threshold sharing. §9 develops the continuous RG-flow ratchet. §10 reports the empirical evaluation: timing budgets, the f64-quantization finding, cross-sprint composition tests, validation against the Python math-oracle suite. §11 presents the three deferred constructions as derived-but-unimplemented sections. §12 surveys related work. §13 limits + future work. §14 conclusion.

---

## §2. Background

### §2.1 Prior canon — three Davis manuscripts

This paper is the cryptographic application of two prior Davis substrates:

**The Davis Manifold** \cite{davis2026manifold}: a Riemannian substrate $(M, g, P, \varepsilon, \kappa_{\text{hard}}, \kappa_{\text{soft}})$ for residue-bounded computation. The non-vacuity condition $\kappa_{\text{soft}} - 2R\varepsilon(L^*) > 0$ is the source of the Hadamard-detection invariant that Sprint M (continuous ratchet) preserves under per-write gauge advance.

**The Kähler Substrate** \cite{davis2026kahlersubstrate}: extends $(M, g)$ to $(M, g, J, \nabla, B, \Gamma)$ — a Kähler manifold with closed magnetic 2-form $B$ and discrete graph approximation $\Gamma$. The eight-layer catalog (L1–L8 + extensions) provides the invariant ring (curvature, spectral gap, Davis capacity, mean holonomy, Betti numbers) that Sprint I (Curvature-MAC) signs.

**Sheaf Composition** \cite{davis2026sheafcomposition}: the discrete section-graph runtime — the fiber-bundle data model on which both substrates execute.

### §2.2 External math anchors

- **Property-preserving encryption** (Pandey-Rouselakis CRYPTO 2012): closest prior framework. PPE generalizes deterministic / OPE encryption to "encryption that preserves a chosen property P." Geometric encryption is PPE where P is "a chosen $G$-invariant geometric quantity" and $G$ is the structure group of the encryption mode.
- **Structured encryption** (Chase-Kamara ASIACRYPT 2010): framing for "which queries are supported on which encrypted data structures." Closest in spirit but does not classify by structure group.
- **Searchable symmetric encryption** (Curtmola-Garay-Kamara-Ostrovsky CCS 2006): the search-on-ciphertext primitive that motivates the INDEXED mode of our taxonomy.
- **Order-preserving encryption** (Boldyreva FSE 2009; Popa-Redfield-Zeldovich-Balakrishnan CryptDB SOSP 2011): the order-on-ciphertext primitive that the AFFINE mode of our taxonomy generalizes (Affine preserves order when $a > 0$; OPE is a special case).
- **Homomorphic encryption** (Gentry STOC 2009; Brakerski-Gentry-Vaikuntanathan ITCS 2012; CKKS 2017): the universality target. Different cost-vs-flexibility point.
- **Proxy re-encryption** (Ateniese-Hohenberger CCS 2005; Libert-Vergnaud Eurocrypt 2008): the standard collusion-resistant PRE definition. Sprint J explicitly does *not* meet this; we document the gap.
- **AES-GCM-SIV** (RFC 8452): the AEAD primitive for OPAQUE mode.
- **AES-CMAC** (NIST SP 800-38B): the PRF for INDEXED mode.
- **HKDF** (Krawczyk CRYPTO 2010 / RFC 5869): the KDF for INTEGRITY key derivation and Sprint M ratchet.
- **Shamir secret sharing** (Shamir CACM 1979): the math underneath Sprint L.
- **RFC 6962 Certificate Transparency log** (Laurie-Langley-Kasper 2013): the Merkle structure underneath Sprint K.
- **Signal symmetric ratchet** (Marlinspike-Perrin 2016): the design analog for Sprint M continuous forward secrecy.

### §2.3 The fiber-bundle data model

A bundle $B = (E, B, F, \pi, \Phi)$ stores records as sections of the bundle. The base space $B$ is parameterized by the GIGI hash $G: K_1 \times \cdots \times K_m \to \mathbb{Z}_2^{64}$ of the record's primary keys. The fiber $F = F_1 \times \cdots \times F_k$ is the Cartesian product of non-key field types. A record is a section $\sigma: p \mapsto (p, v_1, \ldots, v_k) \in E$.

The bundle's geometric invariants:
- **Scalar curvature** $K = \mathrm{Var}/\mathrm{range}^2$ per field, averaged across fields. Quantifies field-level variability normalized for range.
- **Spectral gap** $\lambda_1$: first non-zero eigenvalue of the discrete Laplacian on the bundle's field-index bitmap graph.
- **Davis capacity** $C = \tau / K$ where $\tau$ is record count. Quantifies bundle's information-storage efficiency relative to its geometric complexity.
- **Mean holonomy** $\overline{\mathrm{Hol}}$: averaged holonomy of closed loops in the discrete base graph.
- **Betti numbers** $\beta_0, \beta_1$: connected components and 1-cycles of the field-index graph.

These six scalars constitute the **invariant tuple** $\pi_{\mathrm{inv}}(B) = (K, \lambda_1, C, \overline{\mathrm{Hol}}, \beta_0, \beta_1)$ that Sprint I signs.

### §2.4 Structure groups

A **structure group** $G \subset \mathrm{Aut}(F)$ is a subgroup of the fiber automorphism group. A *gauge transformation* is an element $g \in G$ applied uniformly to every record's fiber. A *geometric encryption* under a secret gauge $g$ replaces each stored fiber tuple $(v_1, \ldots, v_k)$ with $g(v_1, \ldots, v_k)$. The base-space coordinates (keys) are unchanged.

The taxonomy of §4 parameterizes encryption by the choice of $G$:

| Mode | Structure group $G$ |
|---|---|
| Affine | $\mathrm{Aff}(\mathbb{R})$ acting field-wise (per-field $g_i(v) = a_i v + b_i$) |
| Opaque | $\mathrm{Sym}(F)$ — all bijections (AEAD-randomized) |
| Indexed | image of a PRF on $F$ |
| Probabilistic | $\mathrm{Aff}(\mathbb{R}) \ltimes \mathcal{N}(0, \sigma^2)$ |
| Isometric | $O(k)$ on a grouped k-tuple of numeric fields |

The fundamental observation of §3 is that the query class computable on $g$-encrypted data is exactly the class of $G$-invariant functions.

---

## §3. The gauge-invariance theorem

### §3.1 Setup

Let $B = (E, B, F, \pi, \Phi)$ be a fiber-bundle data store with structure group $G \subset \mathrm{Aut}(F)$. Let $g \in G$ be a secret gauge transformation. Let $\mathrm{Enc}_g: E \to E$ be the geometric encryption under $g$:

$$
\mathrm{Enc}_g(\sigma)(p) = (p, g(v_1, \ldots, v_k))
$$

(base point $p$ unchanged; only fiber values transformed).

Let $f: E \to \mathbb{R}$ be a query function on the bundle.

**Definition 3.1** (Gauge invariance). $f$ is *gauge-invariant under $G$* iff for every $g \in G$ and every section $\sigma \in E$:

$$
f(\mathrm{Enc}_g(\sigma)) = f(\sigma).
$$

### §3.2 The theorem

**Theorem 3.1** (Gauge-invariance characterizes ciphertext computability). *Let $f: E \to \mathbb{R}$ be a query function. The following are equivalent:*

*(a) $f$ is gauge-invariant under $G$.*
*(b) For every secret gauge $g \in G$, evaluating $f$ on the $g$-encrypted bundle yields the same value as evaluating $f$ on the plaintext bundle.*

*In particular, $f$ is computable on $g$-encrypted data without knowledge of $g$ iff $f$ is in the $G$-invariant ring $\mathcal{I}_G \subset \mathbb{R}^E$.*

*Proof*: (a) $\Leftrightarrow$ (b) by Definition 3.1. The second sentence is a restatement: "computable without $g$" means there exists an evaluation algorithm that produces $f(\sigma)$ from $\mathrm{Enc}_g(\sigma)$ alone, which is equivalent to $f(\sigma) = f(\mathrm{Enc}_g(\sigma))$ holding for all $g$, which is gauge invariance. $\square$

### §3.3 Examples per mode

**Affine** ($G = \mathrm{Aff}(\mathbb{R})^k$): For a field with values $v$ and gauge $g(v) = av + b$:
- $\mathrm{Var}(g(v)) = a^2 \mathrm{Var}(v)$
- $\mathrm{range}(g(v)) = |a| \cdot \mathrm{range}(v)$
- $K = \mathrm{Var}/\mathrm{range}^2$ is invariant (the $a^2$ factors cancel).
- Range queries: order is preserved iff $a > 0$, so $\mathrm{WHERE}\ v > c$ becomes $\mathrm{WHERE}\ g(v) > g(c)$ (transform the literal, not the data).
- Equality: $v_1 = v_2 \Leftrightarrow g(v_1) = g(v_2)$ (affine is injective).

**Opaque** ($G = \mathrm{Sym}(F)$ realized by AEAD): only $f = \mathrm{const}$ is invariant. Computability class: nothing.

**Indexed** ($G = \mathrm{image}(\mathrm{PRF})$): $v_1 = v_2 \Leftrightarrow \mathrm{PRF}(v_1) = \mathrm{PRF}(v_2)$. Computability: equality and equality-based queries (GROUP BY, JOIN on equality, bitmap-index lookup).

**Probabilistic** ($G$ realized as $g(v) = av + b + \epsilon, \epsilon \sim \mathcal{N}(0, \sigma^2)$): $K$ is preserved up to noise (variance of g(v) is $a^2 \mathrm{Var}(v) + \sigma^2$; the $\sigma^2$ term is small relative to the data variance for sensible $\sigma$). σ-bucket equality: $v_1 \approx_\sigma v_2 \Leftrightarrow \mathrm{bucket}_\sigma(v_1) = \mathrm{bucket}_\sigma(v_2)$ (Davis Identity).

**Isometric** ($G = O(k)$ on a grouped k-tuple): pairwise Euclidean distance $\|u - v\|$ is preserved exactly. Hence any distance-based query (kNN, clustering, distance threshold) is invariant. $K$ and $\overline{\mathrm{Hol}}$ are also preserved because they depend on the metric.

### §3.4 The invariant ring

For each mode, the $G$-invariant ring $\mathcal{I}_G$ is the algebra of polynomial expressions in $G$-invariant scalars under $+$ and $\times$. For affine: $\mathcal{I}_{\mathrm{Aff}}$ contains at least $\{K_i\}_i$ (per-field curvatures), $\{\lambda_j\}_j$ (spectral eigenvalues of the field-index Laplacian), $\{\beta_k\}_k$ (Betti numbers), $\{C_i\}_i$ (per-field capacities), and $\overline{\mathrm{Hol}}$. The Sprint H `PROJECT INVARIANT(...)` query language is exactly the parser surface that admits expressions in this invariant ring.

### §3.5 Floating-point reality

Theorem 3.1 holds in exact arithmetic. In floating-point implementation, gauge invariance holds only **up to** the precision of the arithmetic. Specifically, re-encrypting a bundle under a new gauge $g'$ (the operation Sprint G's `ROTATE_KEY` performs) applies one affine round-trip $v \mapsto g(v) \mapsto g^{-1}(g(v)) \mapsto g'(g^{-1}(g(v)))$ to each field, introducing ~10⁻¹³ relative error per traversal.

The effect on the invariant tuple: $K$, $\lambda_1$, Betti numbers are preserved bit-identically (Betti numbers are integers; spectral computations use integer-arithmetic eigenvalue routines on a 0/1 bitmap). $\overline{\mathrm{Hol}}$ shifts by negligible amounts. **Capacity $C = \tau/K$ amplifies the K drift by τ**, so for a bundle with τ = 40 records, capacity drifts by ~4×10⁻⁹ — measurable in the canonical-bytes encoding.

We address this by **quantizing the f64 components of the invariant tuple to 6 decimal digits** before HMAC. The noise floor (~10⁻⁹ on capacity in the worst observed case) sits 3 orders of magnitude below the 10⁻⁶ quantization grain, so the gauge-invariance property holds bit-identically through the tag in practice. Any genuine drift in the invariants ≥ 10⁻⁶ — which is many orders of magnitude above any modification that could be called "noise" — is preserved through the tag and detected.

This is the "geometric encryption in floating-point arithmetic" contract: gauge invariance is exact in mathematics, 6-decimal-precise in code, both detected end-to-end via the same primitive.

### §3.6 The implications for the rest of the paper

§4 catalogs the five modes with their structure groups. §5 builds Curvature-MAC: an HMAC on the invariant tuple, gauge-rotation-invariant in floating-point thanks to the 6-dp quantization. §6 builds Aff(ℝ) capability delegation: composing two gauges produces another gauge in the structure group's closure, applied directly to ciphertext. §7–§9 build three further constructions on top.

The thread tying them together is Theorem 3.1: each construction either *preserves* a gauge invariant under some operation (Curvature-MAC, capability delegation, ledger telescope) or *advances the gauge* in a controlled way (RG-flow ratchet). The taxonomy of what's possible flows directly from the choice of structure group.

---

## §§4–13 (drafts to come; outline below)

**§4** — The mode taxonomy (one subsection per mode: Affine, Opaque, Indexed, Probabilistic, Isometric). Each: structure group, cipher primitive, gauge-invariant claims, performance, threat model, tests.

**§5** — Curvature-MAC (Sprint I). Theorem 3.3 (gauge-rotation invariance via 6-dp quantization). Domain-separated key derivation. Composition with Sprint K (extended `record_hash` leaves closes the gauge-invariant-content blindspot per §5.x Theorem 3.2).

**§6** — Aff(ℝ) capability delegation (Sprint J). Proxy-alone unrecoverability (Theorem 4.1). Zero-decrypt proxy transform (Theorem 4.2). **Limitation 4.7.1 (collusion path documented explicitly)**. Comparison to Umbral and pairing-based PRE.

**§7** — Holonomy ledger (Sprint K). RFC 6962 Merkle structure with extended leaves. Telescope recompute-and-compare (Theorem 5.2). Byte-level tamper-evidence via `record_hash` walk. Composition with Sprint G rotation events.

**§8** — Čech threshold sharing (Sprint L). Shamir over secp256k1 base field. Čech-cocycle authentication binding to (bundle_id, share_index, holder_pubkey). Theorem 6.1 (information-theoretic privacy for ≤ k−1 coalitions). Theorem 6.2 (substitution-attack detection).

**§9** — Continuous RG-flow ratchet (Sprint M). Per-write HKDF chain with checkpoint policy. Per-field ratchet semantics (INDEXED stays non-ratcheting). Theorem 7.1 (continuous forward secrecy under HKDF one-wayness). Theorem 7.2 (HKDF computational one-wayness, replacing the v0.1-draft thermodynamic argument).

**§10** — Empirical evaluation.
- Test surface: 820 passing, 11 honestly deferred (all parser/WAL-bound).
- Performance budgets per primitive (table from spec §11.1).
- The f64-quantization finding: empirically measured noise on capacity, 6-dp threshold rationale.
- Cross-sprint composition tests confirming theorems 3.2, 3.3, 4.1, 4.2, 7.3 end-to-end.
- Python math-validation suite: 25 independent oracle tests in `theory/encryption/validation/validation_tests_v0_3.py`.

**§11** — Deferred constructions.
- §11.1 Geodesic-ball ABE: policy = geodesic ball in the bundle metric; CP-ABE construction with generic-group-model security argument.
- §11.2 Spectral-signature ZKP: zk-SNARK on Laplacian eigenvalue membership predicate.
- §11.3 Lattice-fiber PQ gauge: structure-group taxonomy on $\mathbb{Z}_q^n$ under LWE.

**§12** — Related work (extended). Position against CryptDB, Arx, Seabed, Acumen, Mylar in the systems family; against Boldyreva, Pandey-Rouselakis, Chase-Kamara, Boneh-Sahai-Waters in the theory family.

**§13** — Limits.
- Does not claim collusion-resistant PRE (Sprint J is trusted-delegatee only).
- Does not claim post-quantum security (lattice-fiber gauge deferred).
- Gauge-invariance is exact in math, 6-dp in implementation.
- The information-leakage analysis on INDEXED is per-cardinality and not formally bounded; future work.

**§14** — Conclusion. Geometric encryption gives a unifying theoretical framework for property-preserving encryption, parameterized by structure group, with five shipping modes and five higher-level constructions in production code today. The mathematical contribution is the gauge-invariance theorem (§3); the engineering contribution is the 820-test reference implementation. The paper documents what shipped, where the math holds exactly vs. up to floating-point precision, and where the threat-model boundaries genuinely lie.

---

## Appendix A — Canonical encoding spec (Sprint I §3.7 reproduced)

## Appendix B — secp256k1 base field constants (Sprint L)

## Appendix C — Python math validation manifest (spec §13)

## Appendix D — Performance benchmarks (sprint impl notes + §11.1 budgets)
