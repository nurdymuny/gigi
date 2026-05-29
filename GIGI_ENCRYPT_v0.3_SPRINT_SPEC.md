# GIGI Encrypt v0.3 — Band 2 Engineering Sprint Spec

> **Author**: Bee Rosa Davis · Davis Geometric
> **Date**: 2026-05-28
> **Version**: 0.1.1 (sprint planning · post-review revision)
> **Status**: Pre-build · companion to `GIGI_GEOMETRIC_ENCRYPTION_SPEC.md` (v0.1) and `GIGI_ENCRYPT_v0.2_SPRINT_SPEC.md` (v0.2)
> **Changelog vs 0.1**: Adversarial review (2026-05-28) caught: (a) Sprint J proxy construction was not collusion-resistant; renamed and reframed as Aff(ℝ) capability delegation with explicit trusted-delegatee threat model. (b) Sprint I Curvature-MAC was overclaimed; reframed as gauge-invariant content-drift detection; paired with extended Sprint K ledger leaves (record_hash) for byte-level tamper-evidence. (c) Sprint L field choice was inconsistent (GF(2²⁵⁶) vs F_p); locked to secp256k1 base field. (d) Sprint M Thm 7.2 thermodynamic argument replaced with HKDF computational one-wayness. (e) Sprint M ratchet semantics moved from per-bundle to per-field (resolves INDEXED × ratchet O(N) tension). (f) Added §1.4 threat model, §8.5 composition tests, §13 Python validation manifest. Smaller fixes per review items 4–14.
> **Goal**: Ship the engineering-grade subset of the Band 2 "On the Horizon" surface from `gigi-encrypt.html`. After v0.3 lands, the only Band 2 rows left unshipped are the three explicitly research-grade items (Geodesic-ball ABE, Spectral-signature ZKP, Lattice-fiber post-quantum gauge), which are deferred to the v0.3 paper as **derived constructions with security arguments**, implementation deferred to a successor v0.4.
> **Principle**: Same as v0.2 — one unified gauge-encryption surface, owned end-to-end, every Band 2 (engineering subset) claim on the public landing page backed by a passing test. No hybrid AES-GCM application code. No half-shipped features. Math-validated in Python alongside the Rust test surface, matching the established `theory/<topic>/validation/` discipline.

---

## 0. What v0.2 already ships

The v0.2 spec committed to the five-mode taxonomy and the eight-sprint surface A–H plus extensions. Verified shipped (commits `3d12c08`, `b7aa823`, `e02de1b`, `33ffb5e`, `37218a2`, `4c2210b`):

- **Modes**: `Affine`, `Opaque` (AES-GCM-SIV), `Indexed` (AES-256-CMAC), `Probabilistic` (affine + Gaussian + Davis-Identity σ-bucket), `Isometric` (O(k) via QR-decomposed Gaussian)
- **Sprint A**: Per-field GQL parser (`EncryptionMode` enum, `ENCRYPTED [INDEXED|OPAQUE|PROBABILISTIC|ISOMETRIC|AFFINE]`)
- **Sprint F**: User-supplied seed (`WITH ENCRYPTION SEED $hex`, `WITH ENCRYPTION SEED FROM ENV $name`)
- **Sprint G**: Discrete forward-secret rotation (`GAUGE bundle ROTATE_KEY FORWARD_SECRET`), dual-seed (s, g), one RG-flow coarse-graining on pre-rotation snapshot, WAL atomicity, Aff(ℝ) closure rekey-without-decrypt on the affine path
- **Sprint H**: `PROJECT INVARIANT (...)` unified query surface; structural "0 bytes decrypted" guarantee via grammar restriction
- **Test surface**: 40 in-source unit tests in `src/crypto.rs`, 15 rotation tests in `src/bundle.rs`, 1 e2e live test (`e2e/encrypt_v02_live_test.mjs`)
- **Production usage**: `jg_kv` encrypted at rest (OPAQUE on payload), Phase-B tenant isolation via signed tokens

After v0.2, the gigi-encrypt page §06 "SHIPPING IN GIGI ENCRYPT" band is fully backed by passing tests. The §06 "ON THE HORIZON" band is the surface this spec attacks.

---

## 1. Scope — five engineering items in v0.3, three research items deferred

### 1.1 Ships in v0.3

| Sprint | Feature | Band 2 row | Cipher / math primitive |
|:--:|---|---|---|
| **I** | Curvature-MAC | "tamper ⇔ ΔK ≠ 0" | HMAC-SHA256 over canonical invariant tuple |
| **J** | Proxy re-encryption | "delegatable decrypt" | Aff(ℝ) closure (from Sprint G ext); O(k) closure on isometric |
| **K** | Holonomy ledger | "tamper-evident audit log" | Merkle tree (CT-log style) |
| **L** | Čech threshold sharing | "k-of-n decrypt via H¹ = 0" | Shamir secret sharing over GF(2²⁵⁶) |
| **M** | Continuous RG-flow ratchet | "continuous forward secrecy" | Per-write KDF chain (Signal symmetric-ratchet analog) + RG entropy bound |

### 1.2 Deferred to the paper as construction-only (not in v0.3 codebase)

These three are research-grade cryptographic constructions; first-pass implementations are higher risk to ship than to publish.

| Item | Why deferred | Where it goes |
|---|---|---|
| **Geodesic-ball ABE** (policy = `{x : d(x, x₀) < r}`) | Designing a new CP-ABE construction with geometric policy expression has no off-the-shelf primitive; implementation without prior peer review of the construction itself risks public break. | Paper §X.1 — "Geodesic-ball attribute-based encryption: a CP-ABE construction whose access structure is a metric ball on the bundle base." Security proof in the generic-group model; reference implementation deferred to v0.4. |
| **Spectral-signature ZKP** (Δλ₁ as membership proof) | Designing a zk-SNARK circuit for the discrete Laplacian eigenvalue computation is its own paper. | Paper §X.2 — "Spectral signatures: zero-knowledge membership proofs for bundle records via Laplacian eigenvalue perturbation." Circuit design + completeness/soundness arguments; reference implementation deferred to v0.4. |
| **Lattice-fiber post-quantum gauge** (structure group → Z_q^n under LWE) | Rebuilding the structure-group taxonomy on a lattice / LWE foundation is a full PQ-companion paper of its own. | Paper §X.3 — "Lattice-fiber gauge: post-quantum geometric encryption via Z_q^n structure groups." Reduction to LWE; construction deferred to v0.4. |

This is the **Davis-Manifold pattern** — publish the construction, defer the hardened implementation. It's how mathematical contributions are normally staged; nobody owes the community a battle-hardened ABE library before the construction is peer-reviewed.

### 1.3 The post-v0.3 status of the public page

After v0.3 ships, the gigi-encrypt page §06 "On the Horizon" band has exactly three rows remaining — the three deferred construction-only items — each with a clean tooltip: "construction published in paper [DOI]; reference implementation in v0.4." No row is silently unsupported.

### 1.4 Threat model (per sprint)

Each sprint's security claims are stated against a specific adversary. Stating these up-front prevents the per-sprint claims from being read against an unstated, stronger adversary and getting flagged in review.

| Sprint | Adversary capabilities | Claims hold against | Out of scope |
|---|---|---|---|
| **I (Curvature-MAC)** | At-rest read+write on the database; sees ciphertexts, schema metadata, and the integrity tag | Detection of any modification that changes the bundle's gauge-invariant content (K, λ₁, capacity, ⟨Hol⟩, β₀, β₁). | Modifications that preserve all six invariants exactly (e.g., record permutations, eigenvalue-preserving graph automorphisms, trivial-position duplicates). These are detected by Sprint K's extended ledger leaves (record_hash). |
| **J (Aff(ℝ) capability delegation)** | Untrusted proxy that stores capabilities long-term and observes all ciphertexts. **Delegatee is trusted** not to combine capability with own key to recover delegator's key. | Proxy alone cannot decrypt or recover any party's key. Delegated reads succeed without exposing plaintext at the proxy. Capability revocation works via rotation. | Collusion between delegatee and proxy (delegatee + capability = delegator's full key under Aff(ℝ) — explicit limitation, §4.7). Known-plaintext attack at delegator (2 plaintext-ciphertext pairs break Aff(ℝ) — explicit limitation, §4.7). For these threat models, Umbral / pairing-based PRE is the correct primitive. |
| **K (Holonomy ledger)** | At-rest read+write on bundle data and ledger leaves; sees the published Merkle root. May attempt to substitute, reorder, insert, or delete past leaves. Concurrent-write race adversary attempts to interleave writes to confuse the ledger ordering. | Tamper-evidence at byte level (record_hash leaves) and gauge-invariant level (holonomy_delta telescoping). Concurrent writes serialize through WAL commit order. Published root commits to all leaves at publish time. | Sealed-leaf inclusion-proof forgery (would require SHA-256 collision). |
| **L (Čech threshold)** | Up to (k − 1) shareholders colluding fully. Substitution attacker who attempts to inject a forged share at reconstruct time. Adversary with read access to all share storage but lacking auth keys. | Information-theoretic privacy against any (k − 1) coalition (Shamir). Detection of forged shares via Čech-cocycle auth tag (binding to bundle, share index, holder pubkey). | k or more honest shareholders colluding — the threshold is *the* security boundary by construction; this is not an attack, it's the intended unlock path. |
| **M (Continuous ratchet)** | Adversary who compromises the engine at time T (full memory + checkpoint store) and attempts to decrypt records older than retention horizon R. Forward-secrecy adversary who has not compromised the engine at time T. | Post-compromise: records older than (T − R) writes are computationally unrecoverable (HKDF one-wayness). Forward secrecy: a non-compromising adversary cannot derive any past or future g_t state. | Compromise of the original seed g₀ before checkpoint persistence (would let attacker replay the full chain). Operators are responsible for HSM-style seed-origin protection. |

The implicit framework adversary across all five sprints is the standard cryptographic adversary: probabilistic polynomial-time, sees all public artifacts, controls the network and the database, but does not control honest parties' memory at time of key use. Any deviation from this baseline is explicit in the row above.

---

## 2. Sprint plan summary

| Sprint | Module(s) | Math primitive | Rust test count | Unblocks |
|:--:|---|---|:--:|---|
| **I** | `src/integrity.rs` (new), `src/bundle.rs`, `src/parser.rs` | HMAC-SHA256 over invariant tuple | 10 (+ 4 migration) | Gauge-invariant-drift detection across the shipped surface |
| **J** | `src/crypto.rs` (extend), `src/parser.rs` | Aff(ℝ) composite + O(k) composite | 10 | Trusted-delegatee Aff(ℝ) capability delegation |
| **K** | `src/ledger.rs` (new), `src/bundle.rs`, `src/wal.rs` | Merkle tree over `(metadata, record_hash, holonomy_delta)` | 9 | Byte-level + gauge-invariant tamper-evidence; audit-log compliance |
| **L** | `src/threshold.rs` (new), `src/parser.rs` | Shamir over secp256k1 base field F_p, p = 2²⁵⁶ − 2³² − 977 | 9 | Multi-party key custody; HSM-style integration |
| **M** | `src/ratchet.rs` (new), `src/bundle.rs`, `src/wal.rs` | Per-field KDF chain (HKDF-SHA256) | 10 | Continuous (not just rotation-discrete) forward secrecy |
| **Composition** | cross-sprint integration tests | (see §8.5) | 4 | Production-shape coverage (all 5 features enabled together) |
| **VAL** | `theory/encryption/validation/validation_tests_v0_3.py` (new) | Python mirror; see §13 | 25 Python | Math-validation parity with `theory/kahler_upgrade/validation/` |

Order matters: **I → J → K → L → M**. Sprint I introduces the integrity tag the other sprints reference. Sprint J depends on Sprint G's Aff(ℝ) closure (already shipped). Sprints K and L are independent and can run in parallel if useful. Sprint M depends on Sprint G's discrete RG step (already shipped) and is the largest individual sprint. Composition tests (§8.5) run last, after all five sprints are individually green.

**Total new test surface target: 52 Rust tests (48 sprint-level + 4 composition) + 25 Python math-validation tests.**

---

## 3. Sprint I — Curvature-MAC (bundle integrity)

> **Module**: `src/integrity.rs` (new), `src/bundle.rs`, `src/parser.rs`, `src/types.rs`
> **GQL**: `GAUGE bundle SIGN_INTEGRITY` (mint a tag), `GAUGE bundle VERIFY_INTEGRITY` (check the tag), `PROJECT INVARIANT_TAG FROM bundle` (return the tag without verifying)
> **Primitive**: HMAC-SHA256 (FIPS 198-1), key derived from a per-bundle integrity-seed via the same wyhash KDF used in `crypto.rs::mix_hash`

### 3.1 The math

**Scope statement.** Curvature-MAC detects *gauge-invariant content drift* — any modification of the bundle that changes its observable geometric output. It does **not** detect arbitrary record-level tampering. Modifications that preserve all six invariants below exactly (e.g., record permutations on a set-semantic bundle, eigenvalue-preserving graph automorphisms, trivial-position record duplicates) evade this primitive by construction. Byte-level tamper-evidence is delivered by Sprint K's extended ledger leaves (`record_hash`); the two primitives are **complementary**, and §3.8 specifies how they compose for full tamper-evidence.

The bundle has a gauge-invariant content fingerprint $\pi_\mathrm{inv}(B)$ — the **invariant tuple**:

$$
\pi_\mathrm{inv}(B) = \bigl( K, \lambda_1, C, \overline{\mathrm{Hol}}, \beta_0, \beta_1 \bigr)
$$

where $K$ is scalar curvature, $\lambda_1$ is the spectral gap, $C = \tau/K$ is Davis capacity, $\overline{\mathrm{Hol}}$ is mean holonomy, $\beta_0, \beta_1$ are Betti numbers. Every component is gauge-invariant under the structure group of the shipped modes (proved per-mode in the v0.1 / v0.2 specs).

Let $\mathrm{canonical}(\cdot)$ denote the canonical big-endian IEEE-754 byte encoding of the tuple (deterministic across platforms; spec'd in §3.7). The integrity tag is:

$$
\tau(B) \;=\; \mathrm{HMAC\text{-}SHA256}\bigl(k_\mathrm{MAC},\; \mathrm{canonical}(\pi_\mathrm{inv}(B))\bigr) \quad \in \{0,1\}^{256}
$$

**Theorem 3.1 (gauge-invariant content-drift detection)**. *Let $B$ be a bundle with integrity tag $\tau(B)$, and let $B'$ be any modification of $B$ such that $\pi_\mathrm{inv}(B') \neq \pi_\mathrm{inv}(B)$ (i.e., the canonical-encoded invariant tuple differs by at least one bit). Then with probability at least $1 - 2^{-256}$ over the choice of $k_\mathrm{MAC}$, the verification $\tau(B') = \tau(B)$ fails.*

*Proof*: Changing the canonical encoding by even one bit produces a uniformly random new HMAC output by the HMAC pseudorandomness property (Bellare 2006). Collision against the original tag occurs with probability at most $2^{-256}$. ∎

**Known evasions of Theorem 3.1** (these are *not* false alarms; they are honest scope):

- **Record permutations**: Since GIGI bundles are set-semantic over their record collection (records are indexed by base hash, not by insertion order), permuting record identifiers while preserving the value population leaves all six invariants exactly fixed. Sprint K's `record_hash` leaves detect this.
- **Eigenvalue-preserving graph automorphisms**: A modification that permutes the discrete graph $\Gamma$ by an automorphism of the spectral structure preserves $\lambda_1$, $\beta_0$, $\beta_1$, and the gauge-invariant components. Same response: Sprint K detects.
- **Trivial-position duplicate insertion**: Inserting a duplicate of an existing record at the same base point may leave invariants unchanged in some metric configurations. Same response: Sprint K detects.

The combined Sprint I + extended Sprint K coverage is complete: any modification either (i) changes the invariant tuple (caught by Sprint I), or (ii) changes at least one record's canonical bytes (caught by Sprint K's `record_hash` leaves). The set of modifications missed by both is the empty set.

**Why the full invariant tuple, not just $K$**: $K$ alone is a single f64 scalar — an attacker who knows the integrity scheme could craft modifications that preserve $K$ to f64 precision while corrupting records. Six independent invariants compose: simultaneous preservation of all six under non-trivial modification requires the attacker to solve a six-variable system over a population they can't see in plaintext, which is computationally infeasible.

**Gauge invariance of the tag under rotation**: Sprint G's `ROTATE_KEY FORWARD_SECRET` re-encrypts all fibers under a new $(s', g')$. Because $\pi_\mathrm{inv}$ is gauge-invariant, the post-rotation invariant tuple equals the pre-rotation invariant tuple, so $\tau(B_\mathrm{post}) = \tau(B_\mathrm{pre})$ — the tag survives key rotation without re-signing. This is what makes Curvature-MAC compatible with the rest of the v0.2 surface.

### 3.2 Test names (write these first)

```rust
// in src/integrity.rs::tests
#[test] fn test_integrity_tag_constant_32_bytes() { /* tag is always exactly 32 bytes */ }
#[test] fn test_integrity_tag_deterministic_under_unchanged_bundle() { /* same bundle → same tag */ }
#[test] fn test_integrity_tag_changes_on_single_record_tamper() { /* flip one fiber byte at rest → tag differs */ }
#[test] fn test_integrity_tag_changes_on_record_insertion() { /* add a record → tag differs */ }
#[test] fn test_integrity_tag_changes_on_record_deletion() { /* remove a record → tag differs */ }
#[test] fn test_integrity_tag_changes_on_field_swap_between_records() { /* swap encrypted values between records → tag differs (β_k or λ_1 shifts) */ }
#[test] fn test_integrity_tag_invariant_under_gauge_rotation() { /* ROTATE_KEY FORWARD_SECRET → tag unchanged (π_inv is gauge-invariant) */ }
#[test] fn test_integrity_tag_invariant_under_proxy_recap() { /* Sprint J Aff(ℝ) capability delegation → tag unchanged */ }
#[test] fn test_integrity_tag_verify_o1_in_record_count() { /* verify time independent of N records; bench at N = 1k, 10k, 100k */ }
#[test] fn test_integrity_signing_key_separate_from_gauge_key() { /* compromise of gauge_key does not enable tag forgery (separate KDF input) */ }

// Migration tests (also live in this sprint since it's the first schema-touching sprint):
#[test] fn test_v02_bundle_loads_on_v03_engine() { /* version=2 schema loads cleanly; integrity tag is None until SIGN_INTEGRITY is called */ }
#[test] fn test_v02_to_v03_migration_via_alter_bundle() { /* ALTER BUNDLE x ENABLE_INTEGRITY upgrades schema in place; subsequent SIGN_INTEGRITY mints tag */ }
#[test] fn test_v03_bundle_downgrade_to_v02_rejected_without_force() { /* DOWNGRADE without explicit force fails; with force, integrity_tag is dropped */ }
#[test] fn test_v03_schema_serialization_roundtrip() { /* version=3 schema serializes and re-loads with all v0.3 metadata intact */ }
```

The four migration tests are flagged separately in the test counts (10 sprint-level + 4 migration = 14 total for Sprint I).

### 3.3 Type changes

```rust
// src/types.rs

pub struct BundleSchema {
    // ...existing...
    pub integrity_seed_source: EncryptionSeedSource,    // NEW; defaults to Random
}

// src/integrity.rs (new)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntegrityTag([u8; 32]);                       // HMAC-SHA256 output

pub struct IntegrityKey([u8; 32]);                       // HMAC key, derived from integrity_seed

#[derive(Debug, Clone)]
pub struct InvariantTuple {
    pub k: f64,                  // scalar curvature
    pub lambda_1: f64,           // spectral gap
    pub capacity: f64,           // C = τ/K
    pub holonomy_mean: f64,      // ⟨Hol⟩
    pub beta_0: u64,             // 0th Betti (u64 — bundles can have > 2^32 components in principle)
    pub beta_1: u64,             // 1st Betti
}

impl InvariantTuple {
    pub fn canonical_bytes(&self) -> [u8; 52] { ... }    // see §3.7 for exact layout
    pub fn compute(store: &BundleStore) -> Self { ... }   // calls into curvature.rs / spectral.rs / invariant.rs
}

pub fn sign(key: &IntegrityKey, tuple: &InvariantTuple) -> IntegrityTag { ... }
pub fn verify(key: &IntegrityKey, tuple: &InvariantTuple, tag: &IntegrityTag) -> bool { ... }
```

### 3.4 Implementation notes

- HMAC-SHA256 via the `hmac` + `sha2` Rust crates (already in the dep tree for AEAD nonce derivation).
- `IntegrityKey` derived from the `integrity_seed` via `HKDF-SHA256(seed, salt = "gigi-integrity-v1")`. Separate domain-separation salt from the gauge KDF prevents cross-key compromise.
- The `InvariantTuple::compute` call piggybacks on the existing `PROJECT INVARIANT(...)` execution path from Sprint H — same code path, plus the canonical-bytes encoding and HMAC.
- The integrity tag is stored alongside the bundle's schema metadata, NOT alongside individual records. One tag per bundle. Verification recomputes the tuple and compares.
- Wire format: 32-byte tag, written as `Value::Binary` in a reserved bundle-meta slot. No per-record overhead.

### 3.5 GQL surface

```sql
-- Sign the current bundle state
GAUGE jg_account SIGN_INTEGRITY;
-- → returns the 32-byte tag as hex, persists in bundle meta

-- Verify the stored tag against current state
GAUGE jg_account VERIFY_INTEGRITY;
-- → returns: { verified: true, tag: "a1b2..", computed_tuple: { k: 0.034, λ_1: 0.71, ... } }
-- → or:     { verified: false, expected_tag: "..", computed_tag: "..", drift: { k: 0.034 → 0.041, ... } }

-- Surface the tag without verifying (for replication / external checkpoint)
PROJECT INVARIANT_TAG FROM jg_account;
-- → "a1b2..." (no recomputation; constant time)
```

### 3.6 Acceptance gate

- All 10 test names in §3.2 pass
- Existing v0.2 regression suite intact (598 + 43 + 34 baseline plus the v0.2 deltas)
- Microbench: `VERIFY_INTEGRITY` runs in ≤ 5ms on a 10k-record bundle (dominated by `InvariantTuple::compute`, which is already O(1) in record count thanks to Welford-streaming field stats — verify holds at 100k records too)
- Python math-validation test `validation_tests_v0_3.py::test_curvature_mac_collision_bound` confirms HMAC pseudorandomness via empirical chi-square over 10k tampering attempts

### 3.7 Canonical encoding of the invariant tuple (normative)

The `canonical_bytes` encoding is the concatenation, in this exact order, of:

1. version magic: `[0x47, 0x49, 0x47, 0x49]` ("GIGI" ASCII) — **4 bytes** (placed at front for forward-compat)
2. `k.to_be_bytes()` — **8 bytes** (IEEE-754 binary64, big-endian)
3. `lambda_1.to_be_bytes()` — **8 bytes**
4. `capacity.to_be_bytes()` — **8 bytes**
5. `holonomy_mean.to_be_bytes()` — **8 bytes**
6. `beta_0.to_be_bytes()` — **8 bytes** (u64 big-endian)
7. `beta_1.to_be_bytes()` — **8 bytes**

Running total: 4 + 8 + 8 + 8 + 8 + 8 + 8 = **52 bytes**.

NaN handling: any NaN component is canonicalized to a single quiet-NaN bit pattern (`0x7ff8000000000000`) before encoding. ±0.0 is canonicalized to +0.0. This guarantees `canonical_bytes(a) == canonical_bytes(b)` iff `a` and `b` are bit-equal after canonicalization, with no spurious tag mismatches from harmless float representations.

### 3.8 Composition with Sprint K for full tamper-evidence

Sprint I alone misses modifications that preserve the invariant tuple (record permutations, eigenvalue-preserving automorphisms, etc. — §3.1). Sprint K's extended ledger leaves close this gap.

**The combined claim** (Theorem 3.2). *Let an attacker make any non-trivial modification to a bundle protected by both Sprint I's Curvature-MAC and Sprint K's extended ledger (with `record_hash` per leaf, §5.1). At least one of:*

*(a) the invariant tuple changes, so the integrity tag verification fails, or*

*(b) at least one record's canonical bytes change, so its record_hash differs, so the ledger's Merkle root no longer matches the published root.*

*The probability of evading both detection paths is at most $2 \cdot 2^{-256}$ (union bound over the two HMAC / SHA-256 collisions required). Equivalently, the modification space missed by **both** primitives jointly is the empty set up to the cryptographic collision bound.*

This compositional property is what gives v0.3 production-grade tamper-evidence. Sprint I gives gauge-aware drift detection (cheap, O(1) in record count); Sprint K gives byte-level tamper-evidence with auditable inclusion proofs. Operators wanting cheap+frequent integrity checks call Sprint I; operators wanting forensic-grade audit verification call Sprint K. Both should be enabled in production. See §8.5 for the composition test (`test_composition_integrity_x_ledger_full_coverage`).

---

## 4. Sprint J — Aff(ℝ) capability delegation (trusted-delegatee re-encryption)

> **Module**: `src/crypto.rs` (extend), `src/parser.rs`, `src/bundle.rs`
> **GQL**: `GAUGE source DELEGATE TO target AS $capability_name`, `GAUGE source APPLY_DELEGATE $capability_name`
> **Primitive**: Aff(ℝ) composite (already shipped in Sprint G ext as the rekey path); newly exposed as a delegable capability. O(k) group composite for isometric mode.

### 4.0 Naming and scope (read this first)

This sprint is **not** "proxy re-encryption" in the Ateniese–Hohenberger 2005 / Libert–Vergnaud 2008 sense. The classical PRE security goal includes **collusion resistance**: a proxy and a delegatee colluding cannot recover the delegator's key. The Aff(ℝ) construction shipped here does **not** achieve that property — see §4.7. We call this primitive **Aff(ℝ) capability delegation** to make the security shape explicit and avoid PRE expectations.

The useful operational property the construction does deliver is: **plaintext is never materialized at the proxy**. That is genuinely valuable for delegated-read workflows where the proxy is the untrusted party and the delegatee is trusted not to combine the capability with their own key for an attack. Most enterprise compliance-review and encrypted-share-handoff scenarios fit this model. For deployments where the delegatee may be adversarial, see §4.7 for the recommended alternative (Umbral / NuCypher pairing-based PRE).

### 4.1 The math

Sprint G shipped the **Aff(ℝ) closure** result: for affine gauge keys $g_1 = (a_1, b_1)$ and $g_2 = (a_2, b_2)$, the rekey transform from $g_1$-ciphertext to $g_2$-ciphertext is itself affine:

$$
\rho_{1 \to 2}(w) \;=\; \frac{a_2}{a_1} \cdot w \;+\; \biggl(b_2 \;-\; b_1 \cdot \frac{a_2}{a_1}\biggr)
$$

Verified: $\rho_{1 \to 2}(a_1 v + b_1) = a_2 v + b_2$, so applying $\rho_{1 \to 2}$ to a $g_1$-ciphertext produces the $g_2$-ciphertext of the same plaintext, without ever materializing the plaintext.

**A capability $C_{A \to B}$** is the pair $(\alpha, \beta) = (a_B / a_A, b_B - b_A \cdot a_B / a_A)$, published by Alice (holder of $g_A$) to a designated proxy. The proxy applies $C_{A \to B}$ to Alice's stored ciphertexts on demand, producing $g_B$-ciphertexts that Bob can decrypt with his own key $g_B$.

**Theorem 4.1 (proxy-alone unrecoverability)**. *Given only the capability $C_{A \to B} = (\alpha, \beta)$, with no other information about either party's gauge key, the proxy cannot recover either party's full key. The two equations $\alpha = a_B/a_A$ and $\beta = b_B - b_A \alpha$ contain four unknowns $(a_A, b_A, a_B, b_B)$; the solution manifold is 2-dimensional in $\mathbb{R}^4$. The proxy receives no further information about the unknowns from observing ciphertexts (which are themselves arbitrary affine images of an unknown plaintext distribution).*

> **REV note (paper draft §6)**: the equation-count statement is straightforward; the *non-trivial* content is "the proxy receives no further information from observing ciphertexts." That requires a careful argument that the ciphertext distribution under an unknown plaintext distribution leaks no advantage. Strengthen the proof in the paper draft, or downgrade to **Proposition** if the ciphertext-distribution claim is left to be a security-model assumption.

*Proof*: As stated. ∎

**Theorem 4.2 (zero-decrypt proxy transform)**. *The proxy's application of $C_{A \to B}$ to ciphertext $w$ invokes only the affine composite; no decryption primitive is on the execution path. Verified by tracing the `decrypt_call_count` counter (already in place from Sprint H's `PROJECT INVARIANT` static-soundness proof).*

> **REV note (paper draft §6)**: (a) this is an *operational verification* property (tracing a counter at runtime), not a mathematical theorem — demote to **Proposition** or **Observation**.
> (b) The "proxy transform" language is correct for Sprint J's Aff(ℝ) construction (the proxy applies α·w+β at the affine level). It is **NOT** correct for the Sprint J.2 pairing-based construction in `src/pairing_delegation.rs`, where the recipient (not the proxy) applies the pairing operation `e(C_1, rk)`. Be explicit about which delegation construction is meant when drafting §6 — the two have different runtime roles for the proxy. Both share the "no decrypt path" property; only the Aff(ℝ) variant is a cryptographic proxy-side transform in the strict sense.

**Isometric mode** has an analogous closure: for $O_A, O_B \in O(k)$, the rekey transform is $O_B \cdot O_A^T \in O(k)$ — also a single orthogonal matrix, also applied directly to ciphertext.

**Opaque / Indexed / Probabilistic modes have no closure**. The capability returns a typed error (`NotAffineClosure`) when attempted on these fields. This is a feature, not a limitation: the security gap is **structural** — the structure groups of those modes don't admit a closed re-encryption morphism without decrypt-then-re-encrypt.

### 4.2 Test names

```rust
#[test] fn test_capability_affine_alice_to_bob_roundtrip() { /* enc_A(v) → recap → dec_B(·) == v */ }
#[test] fn test_capability_affine_zero_decrypt_calls() { /* assert decrypt_call_count unchanged during recap application */ }
#[test] fn test_capability_proxy_cannot_decrypt_alone() { /* given C_{A→B} and enc_A(v), proxy gets enc_B(v), NOT v */ }
#[test] fn test_capability_proxy_alone_cannot_recover_alice_key() { /* given only C_{A→B}, recover (a_A, b_A) — 2 eq / 4 unknowns; verify proxy has no advantage over uniform guess */ }
#[test] fn test_capability_proxy_alone_cannot_recover_bob_key() { /* same in reverse direction */ }
#[test] fn test_capability_collusion_recovers_alice_key_explicit() { /* Limitation 4.7.1 — Bob holds g_B, runs the 2-equation solve, recovers (a_A, b_A); test PASSES with equality assertion documenting this is in-scope by design */ }
#[test] fn test_capability_revoked_after_rotation() { /* Alice rotates her gauge → old C_{A→B} no longer transforms validly */ }
#[test] fn test_capability_isometric_closure() { /* O_A → O_B via O_B · O_A^T; same round-trip + proxy-alone non-recoverability */ }
#[test] fn test_capability_opaque_returns_typed_error() { /* OPAQUE field cannot be delegated → NotAffineClosure { mode: "Opaque" } */ }
#[test] fn test_capability_indexed_returns_typed_error() { /* INDEXED → NotAffineClosure */ }
#[test] fn test_capability_probabilistic_returns_typed_error() { /* PROBABILISTIC → NotAffineClosure */ }
```

The `test_capability_collusion_recovers_alice_key_explicit` test is **load-bearing** for the spec's honesty: it asserts the collusion attack works against the implementation, making the limitation a tested, documented property rather than a hidden weakness. Without that test, a reviewer would correctly suspect the limitation was discovered late and hidden in prose; with it, the limitation is structurally in-scope.

### 4.3 Type changes

```rust
// src/crypto.rs (extend)

#[derive(Debug, Clone)]
pub struct ProxyCapability {
    pub source_bundle: String,
    pub target_bundle: String,
    pub field_transforms: Vec<FieldProxyTransform>,    // one per fiber field
}

#[derive(Debug, Clone)]
pub enum FieldProxyTransform {
    Affine { alpha: f64, beta: f64 },                  // affine composite (a_B/a_A, b_B - b_A·a_B/a_A)
    Isometric { matrix: Vec<Vec<f64>> },               // O_B · O_A^T
    NotClosed { mode: EncryptionMode },                // typed refusal — cannot proxy this mode
}

impl ProxyCapability {
    pub fn build(source: &GaugeKey, target: &GaugeKey, source_name: String, target_name: String) -> Self { ... }
    pub fn apply_to_value(&self, field_idx: usize, w: &Value) -> Result<Value, ProxyError> { ... }
}

#[derive(Debug, thiserror::Error)]
pub enum DelegationError {
    #[error("field mode {0:?} has no Aff(ℝ) closure; delegation requires decrypt-then-encrypt or a collusion-resistant primitive (see Sprint J §4.7)")]
    NotAffineClosure(EncryptionMode),
    #[error("source and target bundles have incompatible schemas")]
    SchemaMismatch,
}
```

### 4.4 Implementation notes

- `ProxyCapability::build` walks both gauge keys pairwise per field. For each field:
  - Affine + Affine → `FieldProxyTransform::Affine { alpha, beta }`
  - Isometric + Isometric (same group_id and member_index) → `Isometric { matrix: O_B · O_A^T }`
  - Anything else → `NotClosed { mode: <source mode> }`
- The capability is serializable to a 16-byte-per-affine-field flat blob (8 bytes alpha + 8 bytes beta), or O(k²) bytes per isometric group.
- **Apply path is read-only on the bundle**: proxy capabilities transform on read, not on write. The stored ciphertext remains under the source's key.
- An optional eager mode (`GAUGE source APPLY_DELEGATE $cap PERSIST AS target_bundle`) materializes a new bundle with target's gauge applied — this is the "Aff(ℝ) closure rekey" path from Sprint G, exposed as a one-shot proxy with persistence.

### 4.5 GQL surface

```sql
-- Alice delegates to Bob
GAUGE alice_bundle DELEGATE TO bob_bundle AS alice_to_bob_handoff;
-- → returns: { capability_id: "ABCD...", affine_fields: 3, isometric_groups: 1, refused_fields: 2 ([legalName: Opaque, dob: Opaque]) }

-- Proxy applies the capability to a record read on behalf of Bob
GAUGE alice_bundle APPLY_DELEGATE alice_to_bob_handoff WHERE id = 42;
-- → returns: bob-key-encrypted record; Bob can now decrypt with his own gauge key

-- One-shot materialization (persistent rekey)
GAUGE alice_bundle APPLY_DELEGATE alice_to_bob_handoff PERSIST AS bob_view;
-- → creates a new bundle "bob_view" with all of alice_bundle's records re-encrypted under Bob's gauge

-- Revoke
GAUGE alice_bundle REVOKE_DELEGATE alice_to_bob_handoff;
-- → capability id is deleted from the bundle's delegation registry
```

### 4.6 Acceptance gate

- All 10 test names in §4.2 pass
- Existing v0.2 regression suite intact
- Sprint G's existing `test_rotate_key_affine_closure_zero_decrypt_calls` continues to pass (the delegation path reuses the same Aff(ℝ) composite)
- Python math validation: `validation_tests_v0_3.py::test_capability_proxy_alone_unrecoverability` — empirical check that a uniformly random guess of $(a_A, b_A)$ from a given capability has no measurable advantage over a guess from no capability at all

### 4.7 Known limitations and recommended alternatives

These limitations are **stated up-front, not deferred**, because they bound the construction's applicable threat model.

**Limitation 4.7.1 (delegatee + capability collusion recovers delegator key)**. Given the capability $C_{A \to B} = (\alpha, \beta) = (a_B/a_A, b_B - b_A \alpha)$ and Bob's own key $g_B = (a_B, b_B)$, Bob can solve:

$$
a_A = \frac{a_B}{\alpha}, \qquad b_A = \frac{b_B - \beta}{\alpha}
$$

— recovering Alice's full key. This is **two equations in two unknowns**; no information-theoretic gap is available. The classical PRE collusion-resistance property does not hold for this construction.

*Operational implication*: Use this primitive only when the delegatee is trusted not to extract the delegator's key from the capability. Typical fits: regulated entities with compliance obligations (HIPAA-covered review platforms, internal multi-team encrypted-share workflows). Avoid when the delegatee may be adversarial.

*Test gate*: `test_capability_collusion_recovers_alice_key_explicit` — the collusion attack is exercised against the implementation and a passing test confirms the attack works, proving the limitation is in scope by design, not a hidden bug.

**Limitation 4.7.2 (known-plaintext attack on Aff(ℝ))**. Given two pairs $(v_1, w_1 = a v_1 + b)$ and $(v_2, w_2 = a v_2 + b)$ with $v_1 \neq v_2$, an attacker solves $a = (w_2 - w_1) / (v_2 - v_1)$ and $b = w_1 - a v_1$. Two known plaintexts break the affine layer.

*Operational implication*: Affine encryption alone is not IND-CPA secure. It is useful for the gauge-invariant analytics path (where plaintext is not exposed) but should not be relied on as a confidentiality primitive against attackers who can observe even a small number of plaintext-ciphertext pairs. Combine with OPAQUE on confidentiality-sensitive fields.

**Recommended collusion-resistant alternative**. When a collusion-resistant PRE is required, point operators at **Umbral** (NuCypher; Nuñez–Agudo–Alonso 2017), which uses pairing-based PRE on BLS12-381 to achieve unidirectional, multi-hop, collusion-resistant re-encryption. Umbral's security goal matches the classical PRE definition; this is the correct primitive when the delegatee may be adversarial. A future GIGI Encrypt sprint may integrate Umbral as an additional delegation mode (`PROXY UMBRAL`); deferred to v0.4+ as it requires pairing-friendly curve infrastructure not currently in the gigi dep tree.

---

## 5. Sprint K — Holonomy ledger (tamper-evident audit log)

> **Module**: `src/ledger.rs` (new), `src/bundle.rs`, `src/wal.rs`, `src/parser.rs`
> **GQL**: `GAUGE bundle PUBLISH_LEDGER_ROOT`, `GAUGE bundle INCLUSION_PROOF FOR_WRITE $write_id`, `GAUGE bundle VERIFY_LEDGER_PROOF $proof`
> **Primitive**: Merkle tree (RFC 6962 / Certificate Transparency log structure) over per-write `(timestamp, write_id, holonomy_delta)` records

### 5.1 The math

A bundle's connection induces, for each closed loop $\gamma$ in the discrete graph $\Gamma$ approximating the base, a holonomy element $\mathrm{Hol}(\gamma) \in U(1)$ (or in the structure group of the encryption, in the v0.3 framing). The bundle's "geometric state" at a moment in time is summarized by an averaged holonomy scalar $\overline{\mathrm{Hol}}$ (already computed by Sprint H's `PROJECT INVARIANT(holonomy_avg)`).

A write event modifies one or more records, which perturbs the connection, which changes the holonomy of any loop passing through the affected records. The **holonomy delta** of a write event $w_t$ is:

$$
\Delta_t \;=\; \overline{\mathrm{Hol}}(B_t) - \overline{\mathrm{Hol}}(B_{t-1})
$$

**The extended ledger** is an append-only sequence of leaves $L_t = (\text{timestamp}_t, \mathrm{op\_id}_t, \Delta_t, \mathrm{record\_hash}_t)$, where $\mathrm{record\_hash}_t = \mathrm{SHA\text{-}256}(\mathrm{canonical\_record\_bytes}_t)$ is the SHA-256 of the canonically-encoded record participating in this write event (or, for multi-record writes, a Merkle commitment over the affected records). The leaves are organized as an outer Merkle tree à la RFC 6962. The Merkle root $R_T$ at time $T$ commits to all four-tuples through time $T$.

The inclusion of `record_hash` in each leaf is the closure-of-blindspot fix for Sprint I's gauge-invariant-only detection. Combined, the two primitives detect any non-trivial modification — see §3.8 Theorem 3.2.

**Theorem 5.1 (Merkle tamper-evidence)**. *Suppose an attacker modifies any past leaf $L_t$ (e.g., to hide a write, alter its $\Delta_t$, or alter its $\mathrm{record\_hash}_t$). The recomputed Merkle root $R'_T$ differs from the originally-published $R_T$ with probability $1 - 2^{-256}$ (SHA-256 collision resistance).*

**Theorem 5.2 (recompute-and-compare integrity check)**. *Suppose an attacker modifies records in the live bundle store between writes $w_1, \ldots, w_T$, without modifying the ledger leaves. Let:*

- *$\overline{\mathrm{Hol}}_{\mathrm{ledger}}(T) \;\stackrel{\mathrm{def}}{=}\; \overline{\mathrm{Hol}}(B_0) + \sum_{t=1}^{T} \Delta_t$ — the holonomy implied by replaying the ledger's deltas,*
- *$\overline{\mathrm{Hol}}_{\mathrm{store}}(T) \;\stackrel{\mathrm{def}}{=}\; \overline{\mathrm{Hol}}\bigl(B_{\mathrm{live\;store}}\bigr)$ — the holonomy computed by recomputing directly from current stored records.*

*If the live store has been modified in a way that changes $\overline{\mathrm{Hol}}$, then $\overline{\mathrm{Hol}}_{\mathrm{ledger}}(T) \neq \overline{\mathrm{Hol}}_{\mathrm{store}}(T)$. The `VERIFY_TELESCOPE` operation checks this equality and surfaces tamper detection.*

*Inheriting Sprint I's gauge-invariant-content blindspot*: The telescope check alone misses modifications that preserve $\overline{\mathrm{Hol}}$ exactly (e.g., record permutations). The `record_hash` per leaf closes this gap: re-hashing the live record at index $t$ and comparing to the ledger's $\mathrm{record\_hash}_t$ detects any byte-level mutation. The `VERIFY_RECORD_HASHES` operation walks all leaves and re-verifies.

Together, **VERIFY_TELESCOPE + VERIFY_RECORD_HASHES** is the byte-AND-gauge-content tamper-evidence path. The two operations are independent and can be run on different cadences (telescope is O(1) on streaming invariants; record-hash walk is O(N) but only needed for full forensic audit).

### 5.2 Test names

```rust
#[test] fn test_holonomy_ledger_append_only() { /* attempt to overwrite a sealed leaf returns LedgerSealed error */ }
#[test] fn test_holonomy_ledger_merkle_root_deterministic() { /* same 100 writes in same order → same root */ }
#[test] fn test_holonomy_ledger_root_changes_on_new_write() { /* root before write ≠ root after */ }
#[test] fn test_holonomy_ledger_inclusion_proof_verifies() { /* for write t in a log of N=1000 entries, proof has log₂(N)≈10 hashes and verifies against root */ }
#[test] fn test_holonomy_ledger_inclusion_proof_fails_on_tampered_leaf() { /* flip one bit in leaf t → proof verification against published root fails */ }
#[test] fn test_holonomy_ledger_recompute_and_compare_detects_holonomy_tamper() { /* Thm 5.2 — modify a record in live store; VERIFY_TELESCOPE returns mismatch */ }
#[test] fn test_holonomy_ledger_record_hash_walk_detects_byte_tamper() { /* modify a record's byte that preserves Hol; VERIFY_RECORD_HASHES catches it via record_hash mismatch */ }
#[test] fn test_holonomy_ledger_concurrent_appends_serialize() { /* 10 parallel writes; ledger leaves are linearized in WAL order */ }
#[test] fn test_holonomy_ledger_rotation_compatible() { /* Sprint G key rotation creates a "rotation event" leaf; root continues across rotation; pre-rotation entries remain verifiable */ }
```

### 5.3 Type changes

```rust
// src/ledger.rs (new)

#[derive(Debug, Clone, Copy)]
pub struct LeafHash([u8; 32]);

#[derive(Debug, Clone)]
pub struct LedgerLeaf {
    pub timestamp: i64,                // UNIX millis
    pub op_id: u64,                    // monotonic per-bundle write ID
    pub holonomy_delta: f64,           // Hol(B_t) - Hol(B_{t-1})
    pub record_hash: [u8; 32],         // SHA-256 of canonical record bytes (NEW v0.3.1 — closes Sprint I blindspot per §3.8)
    pub op_kind: OpKind,               // Insert / Update / Delete / Rotate / Split
}

#[derive(Debug, Clone, Copy)]
pub enum OpKind { Insert, Update, Delete, Rotate, Split }

pub struct HolonomyLedger {
    leaves: Vec<LedgerLeaf>,       // append-only; persisted in WAL
    tree: MerkleTree,              // computed on-demand or incrementally
}

#[derive(Debug, Clone)]
pub struct InclusionProof {
    pub leaf_index: usize,
    pub leaf_hash: LeafHash,
    pub root: LeafHash,
    pub siblings: Vec<LeafHash>,   // log_2(N) hashes for verification
}

impl HolonomyLedger {
    pub fn append(&mut self, leaf: LedgerLeaf) -> Result<usize, LedgerError> { ... }
    pub fn root(&self) -> LeafHash { ... }
    pub fn inclusion_proof(&self, leaf_index: usize) -> Result<InclusionProof, LedgerError> { ... }
    pub fn verify_proof(&self, proof: &InclusionProof) -> bool { ... }
    pub fn telescope_check(&self, baseline_holonomy: f64, current_holonomy: f64) -> bool { ... }
}
```

### 5.4 Implementation notes

- Merkle tree: standard SHA-256-based RFC 6962 construction. Internal nodes are hashes of children: `H(0x01 || left || right)`. Leaves: `H(0x00 || leaf_bytes)`. Empty subtree: zero hash.
- Incremental computation: maintain a tower of "compressed subtree hashes" so appending leaf $N+1$ is $O(\log N)$, not $O(N)$.
- WAL integration: each write transaction includes (a) the bundle data change and (b) the ledger leaf append. Atomic together — if either fails, both roll back.
- The `holonomy_delta` is computed lazily: at write commit, we capture $\overline{\mathrm{Hol}}$ before and after; the delta lands in the leaf. For high-throughput workloads, the delta computation is amortized via the same Welford-streaming invariant cache that powers Sprint H.
- Rotation: when `ROTATE_KEY FORWARD_SECRET` fires, an `OpKind::Rotate` leaf is appended with `holonomy_delta = 0` (rotation is gauge-invariant by construction). The ledger continues across rotation.

### 5.5 GQL surface

```sql
-- Publish current root as a checkpoint (e.g., to a public CT-log peer or to operator's audit storage)
GAUGE jg_account PUBLISH_LEDGER_ROOT;
-- → "8a9f3c4e..." (32-byte SHA-256 root, hex)

-- Get an inclusion proof for a specific past write
GAUGE jg_account INCLUSION_PROOF FOR_WRITE 1234;
-- → { leaf_index: 1233, leaf_hash: "...", root: "...", siblings: ["...", "...", ...] }

-- Verify a proof (against a stored / external root)
GAUGE jg_account VERIFY_LEDGER_PROOF $proof;
-- → { verified: true | false, ledger_intact: true | false }

-- Telescope check: compare ledger's expected current holonomy against actual
GAUGE jg_account VERIFY_TELESCOPE;
-- → { expected: 0.0341, actual: 0.0341, drift: 0.0, intact: true }
```

### 5.6 Acceptance gate

- All 9 test names in §5.2 pass
- Existing v0.2 regression suite intact
- Inclusion proof size grows as $\lceil \log_2 N \rceil$ hashes; verified at $N = 100, 1000, 10000$
- Microbench: append leaf in $O(\log N)$ time; verified bench at $N = 10^6$ shows append latency ≤ 50µs
- Python validation: `validation_tests_v0_3.py::test_holonomy_ledger_telescoping_correctness` — generate 5000 random writes, compute the ledger, verify $\overline{\mathrm{Hol}}(B_N) = \overline{\mathrm{Hol}}(B_0) + \sum \Delta_t$ to f64 precision

---

## 6. Sprint L — Čech threshold sharing (k-of-n decrypt)

> **Module**: `src/threshold.rs` (new), `src/parser.rs`, `src/bundle.rs`
> **GQL**: `GAUGE bundle SPLIT INTO k OF n WITH HOLDERS [(pubkey, label), ...]`, `GAUGE bundle RECONSTRUCT FROM SHARES [...]`
> **Primitive**: Shamir secret sharing over the **secp256k1 base field** $\mathbb{F}_p$, $p = 2^{256} - 2^{32} - 977$, framed as Čech section reconstruction on a presheaf over the share-holder cover.

### 6.1 The math

Shamir's secret sharing (1979) splits a secret $s \in \mathbb{F}_p$ into $n$ shares such that:

1. **Reconstruction**: Any $k$ shares recover $s$ via Lagrange interpolation of the unique degree-$(k-1)$ polynomial passing through them.
2. **Privacy**: Any $k - 1$ shares yield zero information about $s$ — for any candidate secret $s'$, exactly one degree-$(k-1)$ polynomial passes through both the $k-1$ shares and $s'$ at $x = 0$. Information-theoretic security.

**Field choice.** We work in the prime field $\mathbb{F}_p$ where $p = 2^{256} - 2^{32} - 977$ — the base field of the secp256k1 elliptic curve (Certicom SEC2, Bitcoin's curve). Reasons:

- **Well-established**: Identical arithmetic to billions of deployed signing keys.
- **Fast software implementation**: Multiple audited Rust crates (`k256`, `secp256k1`, `ff`); we use `k256::Scalar` to inherit constant-time field arithmetic.
- **256-bit secret space**: Exactly fits gauge-key and integrity-key sizes shipped in v0.1–v0.2 without padding or truncation.

The earlier draft of this spec referenced both $\mathrm{GF}(2^{256})$ and $\mathbb{F}_p$ with $p = 2^{256} - 189$; that was an error. $\mathrm{GF}(2^{256})$ is the binary extension field used by GHASH; $\mathbb{F}_p$ for the secp256k1 prime is the prime field used by ECC. These are different rings with different arithmetic. The v0.3.1 spec is unambiguous: **prime field, secp256k1 base prime, `k256` crate**. The polynomial coefficients are 256-bit scalars; share evaluation is at distinct nonzero field elements $x_1, \ldots, x_n$.

**The Čech-cohomology framing.** Let $\mathcal{S}$ be the presheaf assigning to each subset $U \subseteq \{1, \ldots, n\}$ the "subspace of partial reconstructions of $s$ visible to holders in $U$." For $|U| < k$, $\mathcal{S}(U) = \{0\}$ (privacy: $k-1$ shares know nothing). For $|U| \geq k$, $\mathcal{S}(U) = \mathrm{GF}(2^{256})$ (any $k$-or-more subset uniquely determines $s$). The restriction maps are inclusions; the **gluing condition** for a global section is precisely: a global $s$ exists iff every $k$-element subcover's local data agrees on overlaps, which is the Čech cocycle condition.

This is poetic but it cashes out engineering-wise:
- The GQL surface refers to "Čech reconstruction" rather than "Shamir reconstruction," matching the page's geometric framing.
- The vanishing condition $H^1 = 0$ corresponds to "every $k$-subset's reconstruction agrees" — which Shamir guarantees by construction, but the Čech framing makes the invariant explicit and gives us a place to plug in **share authentication** (§6.4) as a Čech 1-cocycle.

**Theorem 6.1 (k-of-n threshold security)**. *Let $s$ be a 256-bit seed and let $S_1, \ldots, S_n$ be Shamir shares over $\mathrm{GF}(2^{256})$. For any subset $T \subseteq \{1, \ldots, n\}$ with $|T| < k$, the conditional distribution $P(s \mid \{S_i : i \in T\})$ is uniform over $\mathrm{GF}(2^{256})$ — information-theoretically independent of the partial-share view.*

*Proof*: Standard Shamir. ∎

**Theorem 6.2 (Čech cocycle authentication)**. *Each share $S_i$ carries an authentication tag $T_i = \mathrm{HMAC}(k_\mathrm{auth}, \text{bundle\_id} \| i \| \text{holder\_pubkey}_i \| S_i)$. Substituting a forged share $S'_i$ during reconstruction is detected with probability $1 - 2^{-256}$ over the auth-key randomness.*

### 6.2 Test names

```rust
#[test] fn test_cech_split_n_recoverable_from_k() { /* for (k,n) ∈ {(2,3), (3,5), (5,9)} — any k shares recover seed */ }
#[test] fn test_cech_split_k_minus_1_information_theoretic_security() { /* given k-1 shares, distinguisher cannot beat 1/2^256 */ }
#[test] fn test_cech_share_size_32_bytes_plus_tag() { /* each share is ≤ 64 bytes regardless of n */ }
#[test] fn test_cech_lagrange_correctness() { /* for 100 random seeds, k-share reconstruction matches original to byte equality */ }
#[test] fn test_cech_cocycle_authentication_verifies() { /* share auth tag verifies against bundle's auth key */ }
#[test] fn test_cech_substitution_attack_fails() { /* forge a share with valid Shamir polynomial value but wrong holder_pubkey → reconstruction returns ShareAuthFailed */ }
#[test] fn test_cech_threshold_gql_split() { /* GAUGE bundle SPLIT INTO 3 OF 5 WITH HOLDERS [alice, bob, carol, dave, eve] parses + executes */ }
#[test] fn test_cech_threshold_gql_reconstruct() { /* GAUGE bundle RECONSTRUCT FROM SHARES [s_alice, s_bob, s_carol] parses + executes; engine loads seed into memory; subsequent reads decrypt successfully */ }
#[test] fn test_cech_revocation_re_splits_seed() { /* revoke one holder → re-split required; old shares no longer reconstruct */ }
```

### 6.3 Type changes

```rust
// src/threshold.rs (new)

#[derive(Debug, Clone)]
pub struct Holder {
    pub pubkey: [u8; 32],            // Ed25519 pubkey or compatible 32-byte identity
    pub label: String,               // human-readable handle ("alice@davisgeometric.com")
}

#[derive(Debug, Clone)]
pub struct ShamirShare {
    pub bundle_id: String,
    pub holder: Holder,              // locked at split time; the pubkey is bound into auth_tag
    pub share_index: u8,             // 1..n; the x-coordinate
    pub value: [u8; 32],             // the y-coordinate, encoded as a k256::Scalar in F_p
    pub auth_tag: [u8; 32],          // HMAC-SHA256(auth_key, bundle_id || share_index || holder.pubkey || value)
}

pub struct ThresholdScheme {
    pub k: u8,
    pub n: u8,
}

pub fn split(
    seed: &[u8; 32],
    scheme: ThresholdScheme,
    holders: &[Holder],
    auth_key: &[u8; 32],
    bundle_id: &str,
) -> Result<Vec<ShamirShare>, ThresholdError> { ... }

pub fn reconstruct(
    shares: &[ShamirShare],
    auth_key: &[u8; 32],
    bundle_id: &str,
) -> Result<[u8; 32], ThresholdError> { ... }
```

### 6.4 Implementation notes

- Field arithmetic over $\mathbb{F}_p$, $p = 2^{256} - 2^{32} - 977$: use the `k256` crate's `Scalar` type (constant-time, audited, already in scope for any future ECDSA work in the project).
- Lagrange interpolation: O(k²) field ops; trivial for k ≤ 9 (the realistic max).
- The auth key $k_\mathrm{auth}$ is per-bundle, derived from the bundle's `auth_seed` via HKDF (separate from gauge seed and integrity seed — three independent KDF inputs to bound blast radius).
- **Holder identity is locked at split time**: holders are pairs `(holder_pubkey: [u8; 32], label: String)` provided at SPLIT call. The pubkey is bound into the share's auth tag (§6.1 Thm 6.2), so reconstruction cannot succeed with a forged share even if an attacker knows the holder's label. This closes the chicken-and-egg gap flagged in the v0.1 spec review. Holders typically use Ed25519 pubkeys (the gigi-stack standard); other key types are accepted at the schema level as fixed-length byte strings.
- **Persistence**: shares are NOT stored by the engine. The engine emits them at split-time as a query result; holders are responsible for persisting their own share. Reconstruction is a one-shot operation that loads the seed into the engine's in-memory key cache for the duration of a session or until a `GAUGE bundle FORGET_SEED` command.

### 6.5 GQL surface

```sql
-- Split the bundle's seed into 3-of-5 shares.
-- Each holder is (pubkey_hex, label) — the pubkey is bound into the auth tag at split time.
GAUGE jg_account SPLIT INTO 3 OF 5 WITH HOLDERS [
  (0xa1b2c3...alice_ed25519_pubkey..., 'alice@davisgeometric.com'),
  (0xd4e5f6...bob_ed25519_pubkey...,   'bob@davisgeometric.com'),
  (0x718293...carol_ed25519_pubkey..., 'carol@davisgeometric.com'),
  (0xa4b5c6...dave_ed25519_pubkey...,  'dave@davisgeometric.com'),
  (0xd7e8f9...eve_ed25519_pubkey...,   'eve@davisgeometric.com')
];
-- → returns 5 share blobs, each ~96 bytes (32-byte y + 32-byte auth tag + 32-byte pubkey echo)
-- → engine "forgets" the seed; cannot decrypt until reconstruct

-- Three holders cooperate to reconstruct
GAUGE jg_account RECONSTRUCT FROM SHARES [
  '0x1a2b...',   -- alice's share
  '0x3c4d...',   -- bob's share
  '0x5e6f...'    -- carol's share
];
-- → engine reloads seed into in-memory cache; bundle decryption resumes
-- → returns: { reconstructed: true, holders_used: ["alice", "bob", "carol"], seed_in_memory: true }

-- Forget the seed (revert to threshold-locked state)
GAUGE jg_account FORGET_SEED;
-- → in-memory key cache cleared; bundle is now read-only ciphertext until next reconstruct

-- Revoke a holder (requires re-split)
GAUGE jg_account REVOKE_HOLDER 'eve@davisgeometric.com';
-- → marks current shares stale; new SPLIT command required with remaining holders
```

### 6.6 Acceptance gate

- All 9 test names in §6.2 pass
- Existing v0.2 regression suite intact
- Information-theoretic security: empirical chi-square over 10⁴ guess attempts on k-1 shares shows uniform distribution of reconstructed candidates (no detectable bias)
- Microbench: split for $(k, n) = (5, 9)$ runs in ≤ 1ms; reconstruct in ≤ 1ms
- Python validation: `validation_tests_v0_3.py::test_cech_shamir_correctness` — for random seeds and random (k, n) parameters, reconstruction matches original; `test_cech_information_theoretic_security` — k-1 shares produce uniform candidate distribution

---

## 7. Sprint M — Continuous RG-flow ratchet (per-write forward secrecy)

> **Module**: `src/ratchet.rs` (new), `src/bundle.rs`, `src/wal.rs`, `src/parser.rs`
> **GQL**: `CREATE BUNDLE name FIBER (...) WITH RATCHET CHECKPOINT_EVERY N`, `GAUGE bundle FORGET_HISTORY BEFORE $write_id`, `GAUGE bundle RATCHET_STATE` (introspection)
> **Primitive**: Per-write KDF chain $g_{t+1} = \mathrm{HKDF}(g_t, \mathrm{record}_t)$; Signal symmetric-ratchet analog (Marlinspike–Perrin 2016) with RG-entropy bound on irrecoverable history.

### 7.1 The math

Sprint G shipped **discrete** forward secrecy: a single `ROTATE_KEY` command advances the gauge from $g$ to $g'$ and irreversibly coarse-grains the dropped snapshot. Sprint M generalizes this to **continuous** forward secrecy: every insert advances the gauge by one ratchet step.

**The KDF chain.** Starting from a seed $g_0$ (derived from the bundle's `gauge_seed` via the existing v0.1 KDF), each insert at logical time $t$ derives the next gauge state from the prior plus a per-record salt:

$$
g_{t+1} \;=\; \mathrm{HKDF}\text{-}\mathrm{SHA256}\bigl(g_t,\; \text{salt} = \text{record\_bytes}_t \,\|\, t\bigr)
$$

After advancing, the engine drops $g_t$ from memory. Reading record $t$ requires either:
- (a) Holding $g_0$ and replaying the chain forward to step $t$ (O(t) work), or
- (b) Holding a **checkpoint** $g_T$ for $T \geq t$ and replaying back-forward (the chain is one-way; checkpoint replay still requires $T - t$ steps forward).

**Checkpoint policy.** A schema-declared `CHECKPOINT_EVERY N` (default $N = 1024$) causes the engine to persist $g_{kN}$ for $k = 0, 1, 2, \ldots$ to durable storage. Reading record $t$ then costs $O(N)$ replay work, not $O(t)$.

**Theorem 7.1 (continuous forward secrecy)**. *Given the engine's current state $(g_T, \text{checkpoints}_{\{0, N, 2N, \ldots, \lfloor T/N \rfloor \cdot N\}})$ and an arbitrary write index $t < T - R$ where $R$ is the retention horizon, an attacker who compromises the engine cannot recover $g_t$. Specifically: $g_t$ is a function of $g_{\lfloor t/N \rfloor \cdot N}$ and records $\text{record}_{\lfloor t/N \rfloor \cdot N + 1}, \ldots, \text{record}_t$. If the operator's retention policy deletes records older than $R$ writes AND deletes checkpoints $g_{kN}$ with $kN < T - R$, then for $t < T - R$ both inputs to the chain step are gone, and $g_t$ is computationally unrecoverable.*

**Theorem 7.2 (HKDF computational one-wayness of the chain)**. *Under the HKDF-Extract pseudorandomness assumption (Krawczyk, CRYPTO 2010, Theorem 4.4), given $g_{t+1} = \mathrm{HKDF}(g_t, \mathrm{salt} = \mathrm{record}_t \,\|\, t)$ and the salt, no probabilistic polynomial-time adversary can recover $g_t$ from $g_{t+1}$ with probability non-negligibly better than $2^{-256}$. Composing the chain step $T - t$ times: an adversary holding $g_T$ and all salts cannot recover $g_t$ for any $t < T$ where the chain step inputs in between have been deleted, with probability beyond $(T - t) \cdot 2^{-256}$ via union bound.*

*Note*: this is a cryptographic, not thermodynamic, irreversibility. HKDF's one-wayness comes from SHA-256's preimage resistance plus the hash-chain composition; the term "RG flow" earlier in the v0.1 draft was metaphorical and has been removed from the formal claim. The metaphor is useful pedagogically (the chain "coarse-grains" prior state irreversibly), but the proof obligation is HKDF security, not the second law of thermodynamics.

This is the **continuous** analog of Sprint G's single-shot RG step. Where Sprint G dropped one snapshot at one moment, Sprint M is dropping chain state continuously, achieving the Signal-ratchet property: **the compromise of the engine at time $T$ does not enable decryption of records written more than $R$ steps in the past**.

**Theorem 7.3 (gauge invariance under ratchet)**. *Each KDF step $g_t \to g_{t+1}$ is a different element of the same structure group (e.g., $\mathrm{Aff}(\mathbb{R})$). Since the bundle's geometric invariants $\pi_\mathrm{inv}(B)$ are gauge-invariant by construction, the invariant tuple does not change across ratchet steps. The Curvature-MAC tag $\tau(B)$ from Sprint I remains valid across the ratchet — verification works identically before and after ratchet steps.*

> **REV note (paper draft §9)**: this is a *corollary* of §5's affine-mode invariant theorem applied to a different gauge index. Restate as **Corollary** in §9.

**Interop with Sprint J (proxy re-encryption)**. The proxy capability $C_{A \to B}$ is built from a specific gauge state. When the ratchet advances, the capability becomes stale (it would proxy ciphertexts encrypted under $g_t$, but new writes are encrypted under $g_{t+k}$). The semantics: capabilities are pinned to a checkpoint; using a capability beyond its pinned validity window returns a typed `CapabilityStale` error.

### 7.2 Test names

```rust
#[test] fn test_ratchet_advances_per_write() { /* g_t ≠ g_{t-1} after each insert; verified by tracking engine's current key hash */ }
#[test] fn test_ratchet_forward_secrecy_past_retention() { /* delete records before write t=10; engine cannot decrypt record t=5 even with full schema access */ }
#[test] fn test_ratchet_replay_from_seed_matches_current_state() { /* given seed g_0 and write count N, manually replay chain → matches engine's g_N */ }
#[test] fn test_ratchet_checkpoint_skip_o_n() { /* CHECKPOINT_EVERY 1024; recovery from checkpoint 5000 to write 5500 is ≤ 500 KDF steps, not 5500 */ }
#[test] fn test_ratchet_kdf_chain_deterministic() { /* same seed + same record sequence → bit-identical key chain; verified by replaying 10k writes */ }
#[test] fn test_ratchet_concurrent_writes_serialize_via_wal() { /* 8 parallel inserts; ratchet advances in WAL commit order, not insert-arrival order */ }
#[test] fn test_ratchet_hkdf_one_wayness_indistinguishability() { /* given g_{t+1} and salt, attacker's recovered candidate g_t passes uniform-distribution chi-square test (no advantage over guess) */ }
#[test] fn test_ratchet_curvature_invariant_under_ratchet() { /* K(bundle) unchanged across 10k ratchet steps; the invariants are gauge-invariant */ }
#[test] fn test_ratchet_integrity_tag_invariant_under_ratchet() { /* Sprint I curvature-MAC tag unchanged across ratchet steps */ }
#[test] fn test_ratchet_proxy_capability_pins_to_checkpoint() { /* Sprint J capability built at checkpoint 1024; using it after ratchet to 2048 returns CapabilityStale */ }
```

### 7.3 Type changes

```rust
// src/ratchet.rs (new)

pub struct RatchetState {
    pub current_key: [u8; 32],         // g_t in memory
    pub write_count: u64,              // t
    pub checkpoint_every: u32,         // N
    pub checkpoints: BTreeMap<u64, [u8; 32]>,  // {kN: g_{kN}} for k = 0, 1, 2, ...
    pub retention_horizon: u64,        // R, in writes; past T-R, records are gone
}

impl RatchetState {
    pub fn new(seed: [u8; 32], checkpoint_every: u32, retention_horizon: u64) -> Self { ... }
    pub fn advance(&mut self, record_bytes: &[u8]) -> [u8; 32] { ... }
    pub fn key_at_index(&self, t: u64) -> Result<[u8; 32], RatchetError> { ... }
    pub fn forget_history_before(&mut self, t: u64) -> usize { /* drop checkpoints before t */ }
}

#[derive(Debug, thiserror::Error)]
pub enum RatchetError {
    #[error("write index {0} predates retention horizon at {1}; records and key unrecoverable")]
    BeyondRetentionHorizon(u64, u64),
    #[error("proxy capability pinned to checkpoint {pinned}; current state at {current}")]
    CapabilityStale { pinned: u64, current: u64 },
}
```

### 7.4 Implementation notes (per-field ratchet semantics)

- HKDF via the `hkdf` Rust crate (already in tree from Sprint G).
- Per-write salt is `record_bytes_t || t` — including the write index in the salt prevents replay attacks where an attacker substitutes a different record at the same logical position.
- The KDF chain is **schema-opt-in per field**, not per bundle. This is the right boundary because INDEXED-mode equality search is fundamentally incompatible with per-write key rotation: if the PRF key changes on every insert, equal plaintexts no longer produce equal ciphertexts, and the bitmap index degenerates.

**Per-field ratchet semantics** (corrected from v0.1 draft — was previously "per-bundle with INDEXED opt-out warning"):

| Field encryption mode | Ratchet default | Behavior under `WITH RATCHET` |
|---|---|---|
| **Affine** | **Ratchets per write** | Each step derives new $(a_t, b_t)$ from the chain key. Old ciphertext readable via checkpoint replay. |
| **Opaque (AEAD)** | **Ratchets per write** | Each step derives a new AEAD key; per-record nonce continues to increment within the key's lifetime. |
| **Indexed (PRF)** | **Does NOT ratchet** (default) | PRF key stays stable across ratchet steps, preserving equality search. INDEXED fields are explicitly opted out at schema declaration time. |
| **Probabilistic** | **Ratchets per write** for the noise sample; bucket key stable | New ε sampled per step from a per-step seed; σ-bucket hash key remains stable so equality lookup continues to work. |
| **Isometric** | **Ratchets per write** | Each step derives a new orthogonal matrix O_t from the chain key. |

If a schema declares `WITH RATCHET` at the bundle level but contains INDEXED fields, the engine produces a warning at `CREATE BUNDLE` time and creates the bundle with INDEXED fields on a non-ratcheting subkey while all other fields ratchet. This is the production-default sensible behavior. An operator wanting full per-write key rotation on an INDEXED field (with the loss of equality search) must explicitly declare `INDEXED RATCHETING` at the field level — that option exists for completeness but is not the path most workloads should take.

- **Migration**: existing v0.2 bundles can be `ALTER BUNDLE ENABLE_RATCHET` to begin ratcheting from their current state. Non-INDEXED fields begin ratcheting on the next insert; INDEXED fields stay non-ratcheting unless explicitly upgraded via `ALTER BUNDLE ... ENABLE_INDEX_RATCHET fieldname`.
- WAL integration: each write's WAL entry records (a) the insert, (b) the ratchet advance for ratcheting fields, (c) the checkpoint persistence if the boundary is crossed. Atomic together — engine crash during a ratchet step rolls back to the prior state.

### 7.5 GQL surface

```sql
-- Create a bundle with ratchet enabled; checkpoint every 1024 writes
CREATE BUNDLE sensors
  BASE (sensor_id INTEGER, ts TIMESTAMP)
  FIBER (
    temp NUMERIC ENCRYPTED,
    humidity NUMERIC ENCRYPTED
  )
  WITH RATCHET CHECKPOINT_EVERY 1024 RETENTION 10000;

-- Migrate an existing v0.2 bundle to ratchet mode
ALTER BUNDLE jg_account ENABLE_RATCHET CHECKPOINT_EVERY 512;

-- Drop pre-retention checkpoints (operator-triggered forward-secrecy advance)
GAUGE sensors FORGET_HISTORY BEFORE 8000;
-- → drops g_0, g_512, g_1024, ..., g_7680 from memory and durable storage
-- → records before write 8000 are now ciphertext-only

-- Introspect ratchet state
GAUGE sensors RATCHET_STATE;
-- → { current_write: 10342, checkpoint_every: 1024, retention_horizon: 10000,
--     active_checkpoints: 11, oldest_recoverable_write: 8192,
--     forward_secret_writes: 8192 }
```

### 7.6 Acceptance gate

- All 10 test names in §7.2 pass
- Existing v0.2 regression suite intact (specifically: Sprint G's discrete rotation still works on ratcheting bundles — rotation is now the equivalent of a "force-checkpoint and re-key the chain")
- Microbench: per-insert overhead from ratchet ≤ 1µs on a modern x86 (dominated by one HKDF-SHA256 call)
- Python validation: `validation_tests_v0_3.py::test_ratchet_kdf_chain_consistency` — replay 10k writes and verify chain consistency
- Python validation: `validation_tests_v0_3.py::test_ratchet_rg_entropy_monotone` — model the engine memory state across 10k writes + 100 retention-horizon deletions and verify entropy curve is monotonic

---

## 8. TDD discipline + acceptance gates

Every sprint follows the discipline already running in this repo (same as v0.2 §11):

1. **Test names FIRST.** Acceptance test names land in the relevant `tests/` module or in-source `mod tests` as `#[test] fn name() {}` with no body or `unimplemented!()`. They show up as failing before any implementation.
2. **Implementation BEHIND a test.** A commit that adds production code without a test in the same commit (or a prior commit naming the test) is a discipline violation.
3. **v0.2 regression suite stays green.** Each sprint completes only when its named tests pass AND the v0.2 baseline of `598 + 43 + 34 + Sprint A..H tests` remains intact.
4. **Latency budgets per primitive** (table in §11.1).
5. **The math has a witness in the test suite.** Every theorem in this spec (Thm 3.1, Thm 4.1, Thm 4.2, Thm 5.1, Thm 5.2, Thm 6.1, Thm 6.2, Thm 7.1, Thm 7.2, Thm 7.3) maps to a named passing test.
6. **Python math validation parity.** Each sprint's mathematical claims have a mirror in `theory/encryption/validation/validation_tests_v0_3.py` matching the discipline established by `theory/kahler_upgrade/validation/`.

Sprint completion checklist (copy-paste from each sprint's §X.6 acceptance gate):

- [ ] All sprint test names pass
- [ ] v0.2 regression suite intact
- [ ] Perf bench within budget
- [ ] Python math-validation mirror green
- [ ] Documentation updated (`GQL_REFERENCE.md`, `GIGI_API.md`, `gigi-encrypt.html` row evidence)
- [ ] `gigi-encrypt.html` corresponding "On the Horizon" row migrates to "Shipping in GIGI Encrypt" band; evidence numbers verified against fresh runs

### 8.5 Cross-sprint composition tests

Each sprint is tested in isolation; the production case is all five features enabled together on a single bundle (e.g., a `jg_account`-style bundle with INTEGRITY + LEDGER + RATCHET + threshold-split seed). The interaction matrix has known tensions that must be tested explicitly, not assumed.

These tests run **after** all five sprints individually pass and live in a new file `tests/encrypt_v03_composition.rs`:

```rust
#[test] fn test_composition_integrity_x_ledger_append() {
    // Setup: bundle with INTEGRITY + LEDGER both enabled, 100 records.
    // Sign integrity tag τ_1 at write 100.
    // Append 50 more writes (each appends a ledger leaf with record_hash).
    // Re-sign integrity tag τ_2 at write 150.
    // Assertions:
    //   - τ_1 was valid for state at write 100 (verified before further writes)
    //   - τ_2 reflects the new state
    //   - Ledger inclusion proofs for writes 1..150 all verify against current root
    //   - VERIFY_RECORD_HASHES walk passes
}

#[test] fn test_composition_integrity_x_ratchet_step() {
    // Setup: bundle with INTEGRITY + RATCHET on Affine fiber.
    // Sign integrity tag τ at write 1.
    // Insert 1000 records (1000 ratchet steps).
    // Verify integrity at write 1001.
    // Assertion: τ verifies — Theorem 7.3 holds (invariants are gauge-invariant across ratchet).
}

#[test] fn test_composition_ledger_x_ratchet_rotation_event() {
    // Setup: bundle with LEDGER + RATCHET enabled.
    // Insert 500 records.
    // Trigger ROTATE_KEY FORWARD_SECRET (the v0.2 Sprint G command).
    // Assertions:
    //   - Ledger has a leaf at the rotation point with OpKind::Rotate and holonomy_delta = 0
    //   - Pre-rotation ledger entries remain verifiable (Merkle proofs still hold)
    //   - Post-rotation writes continue the ledger; new root commits to all entries including the rotation event
}

#[test] fn test_composition_proxy_x_ratchet_capability_stale() {
    // Setup: bundle with RATCHET enabled. Build proxy capability C_{A→B} at write 500
    //         (pinned to checkpoint at 512).
    // Continue inserting until write 2048 (advances past the next checkpoint).
    // Attempt to apply C_{A→B} to a record encrypted under the new key.
    // Assertion: returns RatchetError::CapabilityStale { pinned: 512, current: 2048 }
    //   — the design contract from §7.1 Theorem 7.3 (interop note) is enforced at runtime.
}
```

These four tests are the **production-shape gate**: they prove that the five primitives compose correctly when all are enabled, which is the real deployment scenario for any production bundle.

---

## 9. Validation matrix — what proves what

| Sprint | New Rust tests | New Python tests | Validates `gigi-encrypt.html` claim |
|:---:|:---:|:---:|---|
| I | 10 + 4 migration | 3 | "Curvature-MAC — gauge-invariant content-drift detection" (reframed from "tamper ⇔ ΔK ≠ 0" per v0.3.1 review §3.1) |
| J | 11 (incl.\ explicit collusion test §4.7) | 2 | "Aff(ℝ) capability delegation — trusted-delegatee re-encryption" (reframed from "proxy re-encryption" per v0.3.1 review §4) |
| K | 9 | 3 | "Holonomy ledger — Merkle tamper-evidence + byte-level record_hash" (extended per v0.3.1 review §5.1) |
| L | 9 | 3 | "Čech threshold sharing — k-of-n decrypt over secp256k1 base field" (field locked per v0.3.1 review §6.1) |
| M | 10 | 4 | "Continuous forward secrecy via HKDF chain" (HKDF reframe per v0.3.1 review §7.1; per-field semantics per §7.4) |
| Composition (§8.5) | 4 | 4 | Production-shape integration across all 5 features |
| Cross-cutting golden vectors | — | 6 | Field arithmetic, HMAC, HKDF, SHA-256 known-answer tests pinned for cross-language consistency |
| **Total** | **52 Rust** (48 sprint + 4 migration + 4 composition) | **25 Python** | **5 of 8 Band 2 rows shipped; 3 remain as paper-only deferred constructions** |

After v0.3 ships:

- The gigi-encrypt page §06 "On the Horizon" band has exactly 3 rows remaining: Geodesic-ball ABE, Spectral-signature ZKP, Lattice-fiber PQ — each with the tooltip "construction in paper [DOI]; implementation in v0.4."
- The "Shipping in GIGI Encrypt" band is the full v0.2 surface + 5 new rows from v0.3.
- The published paper documents both: shipped (v0.2 + v0.3) and constructed (the 3 deferred items).

---

## 10. Wire format / on-disk schema additions

### 10.1 New per-bundle metadata slots

```
BundleSchema metadata extends with three new slots:

integrity_tag           [u8; 32]              // Sprint I — current Curvature-MAC tag
integrity_seed_source   EncryptionSeedSource  // Sprint I — source of integrity HMAC key

ledger_root             [u8; 32]              // Sprint K — current Merkle root of holonomy ledger
ledger_leaf_count       u64                   // Sprint K — number of leaves
ledger_baseline_hol     f64                   // Sprint K — Hol(B_0) for telescope check

ratchet_state           Option<RatchetState>  // Sprint M — None for v0.2 bundles; Some for ratcheting
auth_seed_source        EncryptionSeedSource  // Sprint L — source of Čech share-auth HMAC key
```

No per-record overhead from any of the five sprints. All v0.3 additions live in bundle metadata.

### 10.2 Schema version bump

`BundleSchema.version` bumps from 2 (v0.2) to 3 (v0.3). Engine reading a `version = 2` schema applies the v0.2 path (no integrity tag, no ledger, no ratchet, single-key gauge). `version = 3` honors all five new features. Migration is in-place: `ALTER BUNDLE ENABLE_INTEGRITY` / `ENABLE_LEDGER` / `ENABLE_RATCHET` / `ENABLE_THRESHOLD` are no-ops for any feature already enabled.

### 10.3 GaugeKey serialization

The `GaugeKey` itself doesn't change. The bundle's auxiliary state (integrity tag, ledger root, ratchet checkpoints) is serialized independently and bundled with the schema for backup.

---

## 11. Operational concerns

### 11.1 Performance envelope

| Operation | Budget | Notes |
|---|---|---|
| Sprint I — sign / verify integrity | ≤ 5 ms | Dominated by `InvariantTuple::compute`, already O(1) via Welford streaming |
| Sprint J — build capability | ≤ 1 ms | Per-field affine composite is constant time |
| Sprint J — apply capability per record | ≤ 1 µs | Per-field affine multiply-add |
| Sprint K — append ledger leaf | ≤ 50 µs at N = 10⁶ | O(log N) hash work via incremental subtree |
| Sprint K — inclusion proof | ≤ 50 µs at N = 10⁶ | Same |
| Sprint K — telescope verify | ≤ 1 ms at N = 10⁶ | One invariant recompute + one f64 comparison |
| Sprint K — record-hash walk (full audit) | ≤ N µs | One SHA-256 per record; only run on demand, not on every read |
| Sprint L — split into 5-of-9 | ≤ 1 ms | Polynomial evaluation at 9 points in F_p (secp256k1 base field) |
| Sprint L — reconstruct from 5 shares | ≤ 1 ms | Lagrange interpolation |
| Sprint M — per-insert ratchet step | ≤ 1 µs | One HKDF-SHA256 call per ratcheting field |
| Sprint M — read current record | ≤ 0 extra | Current-key reads cost nothing beyond v0.2 path |
| Sprint M — read historical record at index t with checkpoint period N | ≤ N µs | Up to N HKDF replays from nearest checkpoint; tune `CHECKPOINT_EVERY` per workload |
| Sprint M — replay from checkpoint K writes back | ≤ K µs | Bounded by checkpoint period |

These budgets are set conservatively; first benchmarks will likely show 2–5× headroom. Targets are intentionally not tighter than necessary so we don't optimize prematurely.

### 11.2 Memory

Per-bundle additional memory:

- Integrity: 32 bytes (tag) + 32 bytes (HMAC key) = 64 bytes constant
- Ledger: 32 bytes (current root) + 32 × log₂(N) bytes (compressed subtree tower) — ~640 bytes at N = 10⁹
- Ratchet: 32 bytes (current key) + 32 × ⌈T/N⌉ bytes (active checkpoints) — at retention 10⁴ writes and checkpoint every 1024, ~320 bytes
- Threshold: 0 bytes runtime (shares are not held by the engine after split; reconstruct produces ephemeral in-memory seed)

Total v0.3 metadata overhead: under 2 KB per bundle. Negligible vs. record data.

### 11.3 Backup compatibility

v0.3 backups serialize `version = 3` schemas with full v0.3 metadata. v0.2 backups load on v0.3 engines via the version-discriminated load path (test `test_v02_bundle_loads_on_v03_engine`). v0.3 bundles cannot be loaded on v0.2 engines without explicit `DOWNGRADE` migration (which drops the v0.3-only metadata — destructive, requires confirmation).

### 11.4 Patent boundary

The five v0.3 sprints extend constructions covered under the existing GIGI provisional (U.S. Provisional Application No. 64/045,889). The Curvature-MAC construction, the Aff(ℝ)-closure proxy delegation, the Čech-cohomology framing of Shamir sharing, the holonomy-ledger geometric framing, and the continuous RG-flow ratchet are arguably each independently patentable as continuations-in-part. **Action item before v0.3 release**: draft a continuation-in-part filing covering these five constructions, before publishing the paper. Filing timeline: aim for 2–3 weeks before paper submission to establish priority.

---

## 12. Out of scope for v0.3 (paper-only deferred constructions)

Three Band 2 items are deliberately not implemented in v0.3, deferred to the v0.3 encryption paper as **derived constructions with security arguments**. Reference implementations land in v0.4 after construction-level peer review.

### 12.1 Geodesic-ball attribute-based encryption

**Construction sketch.** Let $(M, d)$ be a metric on the bundle's base. A *geodesic-ball policy* is a pair $(x_0, r)$; a record at base point $x$ satisfies the policy iff $d(x, x_0) < r$. The CP-ABE construction:

1. KP-side: a recipient is issued a key bound to their base point $x_0$.
2. CT-side: ciphertext is encrypted under a policy $(x_c, r_c)$.
3. Decrypt: succeeds iff $d(x_0, x_c) < r_c$, by a pairing-friendly evaluation of the metric.

The non-trivial part: implementing the metric evaluation inside the pairing primitives. The paper's §X.1 develops this as a generic-group-model construction with a security reduction to the bilinear Diffie-Hellman assumption. Reference implementation in v0.4.

### 12.2 Spectral-signature zero-knowledge proofs

**Construction sketch.** A record $r$ belongs to bundle $B$ iff including $r$ in $B$ changes the bundle's Laplacian eigenvalue $\lambda_1$ by less than a threshold $\epsilon$. A spectral-signature ZKP is a zk-SNARK whose circuit:

1. Inputs: the bundle's pre-record Laplacian summary (in a Pedersen commitment), the record $r$, the prover's witness.
2. Verifies: $|\lambda_1(B \cup \{r\}) - \lambda_1(B)| < \epsilon$.
3. Outputs: a succinct proof of membership, without revealing $r$.

Paper's §X.2 develops the circuit, the completeness/soundness arguments, and the parameter trade-offs (proof size vs. SNARK setup complexity). Implementation in v0.4 against the `arkworks` SNARK framework.

### 12.3 Lattice-fiber post-quantum gauge

**Construction sketch.** Rebuild the v0.2 mode taxonomy with the structure group $\mathrm{Aff}(\mathbb{R})$ replaced by a lattice structure group $G_q \subseteq \mathbb{Z}_q^n$ under the LWE assumption. Specifically:

- **Affine-PQ**: $\rho_g(v) = M v + e$, $M \in \mathbb{Z}_q^{n \times n}$, $e \in \mathbb{Z}_q^n$ with $\|e\|$ small. Curvature analog: a discrete Gauss-Bonnet over the lattice.
- **Opaque-PQ**: Kyber / ML-KEM KEM-based AEAD.
- **Indexed-PQ**: lattice-based deterministic PRF.

Paper's §X.3 develops the lattice analog of the gauge-invariance theorem (curvature defined on the discrete lattice is invariant under the LWE-secret transformation iff the lattice is well-conditioned). Implementation in v0.4 against `kyber` and `pqcrypto` Rust crates.

---

## 13. Python math-validation manifest (§VAL)

The Python math-validation suite at `theory/encryption/validation/validation_tests_v0_3.py` mirrors the Rust test surface and acts as an independent oracle on every mathematical claim. The discipline matches `theory/kahler_upgrade/validation/validation_tests_v5.py` (one Python implementation per claim, run via `python -X utf8 validation_tests_v0_3.py`, captures `results_v0_3.txt`).

**Manifest** (25 tests total):

### 13.1 Sprint I — Curvature-MAC math validation (3 tests)

| Python test | Rust test it mirrors | What it proves |
|---|---|---|
| `test_curvature_mac_canonical_encoding_52_bytes` | `test_integrity_tag_constant_32_bytes` | The canonical-bytes layout in §3.7 produces exactly 52 bytes for known invariant tuples |
| `test_curvature_mac_hmac_pseudorandomness_chi_square` | `test_integrity_tag_changes_on_single_record_tamper` | Over 10⁴ single-bit-flips of the canonical bytes, HMAC outputs pass chi-square uniformity test (Bellare 2006 confirmation) |
| `test_curvature_mac_gauge_invariance_under_rotation_simulated` | `test_integrity_tag_invariant_under_gauge_rotation` | Simulated affine rotation of a population produces unchanged invariant tuple to 1 ULP |

### 13.2 Sprint J — Aff(ℝ) capability delegation math validation (2 tests)

| Python test | Rust test | What it proves |
|---|---|---|
| `test_capability_proxy_alone_unrecoverability` | `test_capability_proxy_alone_cannot_recover_alice_key` | Proxy-alone advantage over uniform guess is at the chi-square noise floor (10⁴ trials) |
| `test_capability_collusion_recovers_alice_key` | `test_capability_collusion_recovers_alice_key_explicit` | The collusion path is exercised: given (α, β, a_B, b_B), the solve recovers (a_A, b_A) exactly — confirming Limitation 4.7.1 is in scope by design |

### 13.3 Sprint K — Holonomy ledger math validation (3 tests)

| Python test | Rust test | What it proves |
|---|---|---|
| `test_holonomy_ledger_merkle_inclusion_proof_correctness` | `test_holonomy_ledger_inclusion_proof_verifies` | RFC 6962 Merkle inclusion proofs verify against root for N = {100, 1000, 10000} |
| `test_holonomy_ledger_telescoping_correctness` | `test_holonomy_ledger_recompute_and_compare_detects_holonomy_tamper` | Recompute vs sum-of-deltas matches to f64 precision under unmodified bundles; detects tamper otherwise |
| `test_holonomy_ledger_record_hash_byte_tamper` | `test_holonomy_ledger_record_hash_walk_detects_byte_tamper` | Byte-level tamper detection — single-bit modification of a record's canonical bytes changes its record_hash with probability $1 - 2^{-256}$ |

### 13.4 Sprint L — Čech threshold math validation (3 tests)

| Python test | Rust test | What it proves |
|---|---|---|
| `test_shamir_reconstruction_at_threshold_secp256k1` | `test_cech_split_n_recoverable_from_k` | For 100 random seeds and (k, n) ∈ {(2,3), (3,5), (5,9)}, reconstruction from any k shares matches original. F_p arithmetic via SageMath cross-check |
| `test_shamir_information_theoretic_security_k_minus_1` | `test_cech_split_k_minus_1_information_theoretic_security` | k-1 shares yield uniform candidate distribution; chi-square against uniform fails to reject |
| `test_cech_auth_tag_holder_pubkey_binding` | `test_cech_cocycle_authentication_verifies` | Auth tag changes under any pubkey substitution; cross-verifies the §6.1 Thm 6.2 binding |

### 13.5 Sprint M — Continuous ratchet math validation (4 tests)

| Python test | Rust test | What it proves |
|---|---|---|
| `test_ratchet_hkdf_chain_determinism` | `test_ratchet_kdf_chain_deterministic` | HKDF chain replay matches Rust implementation bit-for-bit for 10k steps |
| `test_ratchet_hkdf_one_wayness_empirical` | `test_ratchet_hkdf_one_wayness_indistinguishability` | Given g_{t+1}, no efficient algorithm recovers g_t — empirical chi-square over 10⁴ guesses |
| `test_ratchet_curvature_invariance_across_steps` | `test_ratchet_curvature_invariant_under_ratchet` | K of bundle unchanged across simulated 10k ratchet steps (Thm 7.3) |
| `test_ratchet_checkpoint_replay_correctness` | `test_ratchet_checkpoint_skip_o_n` | Replay from checkpoint matches direct chain advance |

### 13.6 Composition validation (4 tests)

| Python test | Rust test | What it proves |
|---|---|---|
| `test_composition_integrity_x_ledger_full_coverage` | `test_composition_integrity_x_ledger_append` | Combined coverage of Sprint I + Sprint K closes all known modification classes (Thm 3.2) |
| `test_composition_integrity_x_ratchet_invariance` | `test_composition_integrity_x_ratchet_step` | Integrity tag unchanged across ratchet steps |
| `test_composition_ledger_x_ratchet_rotation_event` | `test_composition_ledger_x_ratchet_rotation_event` | Ledger continues across Sprint G rotation event |
| `test_composition_capability_x_ratchet_stale` | `test_composition_proxy_x_ratchet_capability_stale` | Stale capability rejection mathematically modeled |

### 13.7 Cross-cutting golden vectors (6 tests)

| Python test | Rust test | What it proves |
|---|---|---|
| `test_golden_vectors_secp256k1_field_arithmetic` | (k256 crate test vectors) | F_p arithmetic matches secp256k1 reference test vectors |
| `test_golden_vectors_hmac_sha256` | (RFC 4231 test vectors) | HMAC-SHA256 outputs match RFC 4231 known-answer tests |
| `test_golden_vectors_hkdf_sha256` | (RFC 5869 test vectors) | HKDF-SHA256 outputs match RFC 5869 known-answer tests |
| `test_golden_vectors_sha256_merkle_node` | (RFC 6962 test vectors) | Merkle node hashing matches RFC 6962 Certificate Transparency test vectors |
| `test_golden_vectors_aes_gcm_siv` | (RFC 8452 test vectors) | (v0.2 regression) AEAD outputs match RFC 8452 reference |
| `test_golden_vectors_aes_cmac` | (NIST SP 800-38B vectors) | (v0.2 regression) CMAC outputs match NIST reference |

The golden-vector tests are the cross-language consistency floor: if any one of them fails between Rust and Python implementations, the gauge-encryption interop story breaks. They're run on every CI invocation, not just at sprint completion.

---

## 14. The closing line

After v0.3 ships:

- Every Band 2 engineering item on the public `gigi-encrypt.html` page has a passing test in this repo
- The five new sprints add ~48 Rust tests and ~15 Python math-validation tests
- The three remaining "On the Horizon" rows have published constructions in the v0.3 encryption paper, marked clearly as deferred to v0.4
- Continuous forward secrecy makes GIGI's encryption story stronger than Signal's symmetric ratchet alone (Signal has forward secrecy on message keys; GIGI has it on the gauge that governs the entire bundle's geometric content)
- Curvature-MAC + holonomy ledger together make the bundle's geometric state tamper-evident at the operator level *and* the auditor level
- Proxy re-encryption opens the door to encrypted-share workflows (Alice's encrypted PII → Bob's compliance review, no plaintext exposure anywhere)
- Čech threshold sharing lets enterprise customers split the gauge across HSMs / KMS providers / human key custodians, with k-of-n unlock — the table-stakes feature for B2B encrypted-database deployments

The paper documents all of v0.2 + v0.3 + the three deferred constructions. Total: a complete framework for property-preserving encryption via gauge invariance, with 13 named modes / features, every one backed by published construction and (for the 10 shipped) passing tests.

— Bee, with Claude Code

---

*Companion documents: `GIGI_GEOMETRIC_ENCRYPTION_SPEC.md` (v0.1 foundation), `GIGI_ENCRYPT_v0.2_SPRINT_SPEC.md` (v0.2 five modes), `gigi-encrypt.html` (public landing), `theory/encryption/validation/validation_tests_v0_3.py` (Python math validation, to be created in §VAL).*
*Document owner: Bee Rosa Davis*
*Next review: at end of Sprint I.*
