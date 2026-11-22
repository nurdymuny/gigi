# GIGI Encrypt v0.2 — Full Surface Sprint Spec

> **Author**: Bee Rosa Davis · Davis Geometric
> **Date**: 2026-04-30
> **Version**: 0.1 (sprint planning)
> **Status**: Pre-build · companion to `GIGI_GEOMETRIC_ENCRYPTION_SPEC.md`
> **Goal**: Ship every Band 1 feature on the public `gigi-encrypt` page, behind passing TDD.
> **Principle**: One unified gauge encryption surface owned end-to-end. No hybrid AES-GCM in application code. No half-shipped features. Every claim on the landing page has a passing test in this repo.

---

## 0. What v0.1 already ships

The v0.1 spec (`GIGI_GEOMETRIC_ENCRYPTION_SPEC.md`, March 2026) committed to the affine numeric foundation. That ships today in `src/crypto.rs` and `src/parser.rs`:

- `CREATE BUNDLE name FIBER (... ENCRYPTED) ...` — single keyword at bundle level
- `FieldTransform { scale: f64, offset: f64 }` — affine gauge per fiber field, derived from a 32-byte random seed via wyhash KDF
- Encrypts `Float` / `Integer` / `Timestamp` (numeric)
- Curvature (`K`), confidence (`1/(1+K)`), capacity (`C = τ/K`), spectral gap (`λ₁`), DHOOM compression, anomaly detection — all working on encrypted data
- Encrypted insert + decrypt-on-read + `WHERE`/range comparison via `encrypt_literal`
- Tests: `geo_enc_1` … `geo_enc_15` per the v0.1 §6 matrix

**What does NOT ship in v0.1, despite being on the gigi-encrypt landing page:**

| Feature | v0.1 status |
|---|---|
| `INDEXED` mode (PRF on text/categorical, deterministic, equality-queryable) | Not implemented. Text/categorical fall through `_ => FieldTransform { 1.0, 0.0 }` (identity). |
| `OPAQUE` mode (AEAD on text/binary, randomized, IND-CPA) | Not implemented. Same fall-through. |
| `PROBABILISTIC` mode (gauge + Gaussian, statistical unlinkability + Davis-Identity equality) | Not implemented. |
| `ISOMETRIC` mode (O(k) for grouped numeric vectors) | Not implemented. Single-field affine only. |
| Per-field mode declaration in GQL (`ENCRYPTED OPAQUE`, etc.) | Not implemented. Single bundle-level `ENCRYPTED` flag. |
| User-supplied seed (`WITH ENCRYPTION SEED $hex`) | Not implemented. `random_seed()` only. |
| `GAUGE ROTATE_KEY FORWARD_SECRET` (dual-seed rotation + RG flow) | Not implemented. |
| `PROJECT INVARIANT (...)` unified query form | Not implemented. The existing `INVARIANT` keyword is for value-assertion checks, not invariant-ring projection. |
| `ALTER BUNDLE ROTATE ENCRYPTION` | Not implemented. |

**v0.2 closes all 9 of these.** This document is the sprint plan to do that.

---

## 1. The mode taxonomy

v0.2 introduces four named encryption modes, declared per fiber field at `CREATE BUNDLE` time. Per-bundle `ENCRYPTED` (v0.1) becomes the shorthand for "default mode for this fiber type":

| Field type | Default mode | Other valid modes |
|---|---|---|
| `NUMERIC`, `INTEGER`, `TIMESTAMP` | `AFFINE` (the v0.1 path) | `PROBABILISTIC`, `ISOMETRIC` (when in a group) |
| `TEXT`, `CATEGORICAL` | `OPAQUE` (AEAD) | `INDEXED` (PRF, frequency-leakage caveat) |
| `BINARY` | `OPAQUE` (AEAD) | — |
| `BOOL` | `OPAQUE` (bit-flipped + AEAD wrapper) | — |

The `AFFINE` mode is what v0.1 calls "the encryption" and what `crypto.rs::FieldTransform` implements today. We name it explicitly in v0.2 so the four-mode taxonomy is closed.

`AFFINE` and `PROBABILISTIC` are mutually exclusive on the same field (both modify the numeric value; you pick one). `ISOMETRIC` is only valid when a fiber declaration is grouped (multi-field vector).

### 1.1 Per-field GQL syntax (target)

```sql
CREATE BUNDLE jg_account
  BASE (email TEXT INDEXED)             -- key. INDEXED means base-hash; nothing changes from v0.1.
  FIBER (
    legalName        TEXT     ENCRYPTED OPAQUE,
    addressLine1     TEXT     ENCRYPTED OPAQUE,
    city             TEXT     ENCRYPTED OPAQUE,
    state            TEXT     ENCRYPTED OPAQUE,
    postalCode       TEXT     ENCRYPTED OPAQUE,
    country          TEXT     ENCRYPTED OPAQUE,
    dob              DATE     ENCRYPTED OPAQUE,
    failedAttempts   INTEGER                          -- not encrypted
  );

CREATE BUNDLE gw_events
  BASE (creator_id TEXT INDEXED, fan_id TEXT INDEXED, ts TIMESTAMP)
  FIBER (
    kind             TEXT     ENCRYPTED INDEXED,      -- enum-y; equality-queryable; freq-leak caveat OK for kind enum values
    amount_cents     INTEGER  ENCRYPTED PROBABILISTIC SIGMA 0.5,
    attachment_ref   TEXT     ENCRYPTED OPAQUE,
    meta             BINARY   ENCRYPTED OPAQUE
  );

CREATE BUNDLE wind_sensors
  BASE (sensor_id INTEGER, ts TIMESTAMP)
  FIBER (
    GROUP wind { wind_x, wind_y, wind_z } NUMERIC ENCRYPTED ISOMETRIC,
    pressure         NUMERIC  ENCRYPTED                 -- defaults to AFFINE
  );

CREATE BUNDLE secrets
  BASE (id INTEGER)
  FIBER (label CATEGORICAL, payload TEXT ENCRYPTED OPAQUE)
  WITH ENCRYPTION SEED 'a1b2c3d4...64hex...';
```

Backwards-compatible: the v0.1 `... ENCRYPTED` (no per-field mode) keeps working for numeric fiber, defaults text to `OPAQUE` (the safe choice). This is the migration story for any v0.1 production bundles.

---

## 2. Sprint plan summary

| Sprint | Feature | Scope | Unblocks |
|:--:|---|---|---|
| **A** | GQL parser + per-field mode | Parser/AST/types/types.rs `FieldDef::encryption` enum; backwards-compat for v0.1 syntax | every other sprint |
| **B** | `OPAQUE` mode (AEAD on TEXT/BINARY) | `crypto.rs` AeadTransform via AES-GCM-SIV; integrate in `bundle.rs` insert/reconstruct; per-record nonce | jg_account migration |
| **C** | `INDEXED` mode (PRF on TEXT) | `crypto.rs` PrfTransform via AES-256-CMAC OR keyed SipHash-2-4; preserves equality on bitmap index | cross-bundle joins on encrypted text keys |
| **D** | `PROBABILISTIC` mode + Davis Identity | `crypto.rs` ProbabilisticTransform (affine + Gaussian noise, schema-declared σ); equality via σ-bucket hashing; Davis Identity neighborhood lookup | privacy-preserving numeric analytics, statistical unlinkability claim |
| **E** | `ISOMETRIC` mode (O(k) for grouped numeric) | `crypto.rs` IsometricTransform via QR-decomposition of seeded Gaussian; integrate with grouped fiber declarations | wind/IMU/embedding fields, exact-distance-preserving search |
| **F** | User-supplied seed + key handle | Parser `WITH ENCRYPTION SEED $hex`; `CREATE BUNDLE ... WITH ENCRYPTION SEED FROM ENV $name`; ergonomic seed management | env-derived keys for ops; KMS integration story |
| **G** | Forward-secret rotation | `GAUGE ROTATE_KEY FORWARD_SECRET` GQL; dual-seed rotation (`s`, `g`); one RG-flow step on pre-rotation snapshot; WAL atomicity | the "no past plaintext recoverable" claim |
| **H** | `PROJECT INVARIANT (...)` query form | Parser new statement; engine routes to existing curvature/spectral/anomaly endpoints under a unified surface; output structurally proves "0 bytes decrypted" | unified analytics surface; the "0 bytes decrypted" claim becomes structural not behavioral |

Order matters; later sprints depend on earlier ones (especially A, which gates everything).

Optional follow-ons after Band 1 closes:

- **Sprint X** — `Curvature-MAC` (Band 2 first feature): sign bundle invariants, integrity becomes geometric. Unlocks the "tamper ⇔ ΔK ≠ 0" claim.

---

## 3. Sprint A — GQL parser + per-field mode declaration

> **Module**: `src/parser.rs`, `src/types.rs`
> **GQL**: `field_name TYPE ENCRYPTED [INDEXED|OPAQUE|PROBABILISTIC|ISOMETRIC] [SIGMA n]`

### 3.1 Test names (write these first)

```rust
// in src/parser.rs::tests
#[test] fn test_parse_create_bundle_field_level_opaque() { /* TEXT ENCRYPTED OPAQUE */ }
#[test] fn test_parse_create_bundle_field_level_indexed() { /* TEXT ENCRYPTED INDEXED */ }
#[test] fn test_parse_create_bundle_field_level_probabilistic_with_sigma() { /* NUMERIC ENCRYPTED PROBABILISTIC SIGMA 0.5 */ }
#[test] fn test_parse_create_bundle_field_level_isometric_in_group() { /* GROUP wind { ... } NUMERIC ENCRYPTED ISOMETRIC */ }
#[test] fn test_parse_create_bundle_default_mode_for_text_is_opaque() { /* TEXT ENCRYPTED → mode=Opaque */ }
#[test] fn test_parse_create_bundle_default_mode_for_numeric_is_affine() { /* NUMERIC ENCRYPTED → mode=Affine */ }
#[test] fn test_parse_create_bundle_v01_compat_no_mode() { /* v0.1 syntax: bundle-level ENCRYPTED still works */ }
#[test] fn test_parse_with_encryption_seed_hex() { /* WITH ENCRYPTION SEED 'abc..' */ }
#[test] fn test_parse_rejects_isometric_outside_group() { /* error: ISOMETRIC requires GROUP fiber */ }
#[test] fn test_parse_rejects_probabilistic_on_text() { /* error: PROBABILISTIC requires NUMERIC */ }
#[test] fn test_parse_sigma_value_required_with_probabilistic() { /* error: SIGMA must follow PROBABILISTIC */ }
```

### 3.2 Type changes

```rust
// src/types.rs

#[derive(Debug, Clone, PartialEq)]
pub enum EncryptionMode {
    None,
    Affine,                                  // v0.1; numeric default
    Opaque,                                  // AEAD; text/binary default
    Indexed,                                 // PRF; text equality-queryable
    Probabilistic { sigma: f64 },            // numeric + Gaussian
    Isometric,                               // O(k) on grouped numeric
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub field_type: FieldType,
    pub encryption: EncryptionMode,           // NEW; replaces the implicit on/off in bundle-level flag
    pub group: Option<String>,                // for ISOMETRIC: shared group identifier
}

pub struct BundleSchema {
    // ...existing...
    pub gauge_key: Option<GaugeKey>,           // existing
    pub encryption_seed_user_supplied: bool,   // NEW; controls KDF derivation source
}
```

### 3.3 Parser additions (`src/parser.rs`)

- After parsing field type, peek for `ENCRYPTED`.
- If present, advance, then peek for one of `INDEXED|OPAQUE|PROBABILISTIC|ISOMETRIC|AFFINE`.
- If `PROBABILISTIC`, expect `SIGMA <number>`.
- If no mode keyword, apply field-type defaults (NUMERIC → Affine, TEXT/BINARY → Opaque).
- Post-CREATE BUNDLE field list, peek for `WITH ENCRYPTION SEED <hex-string>`.
- Reject incompatible combinations at parse time (ISOMETRIC requires GROUP, PROBABILISTIC requires numeric, etc.) with friendly error messages.

### 3.4 Backwards compatibility

The v0.1 syntax `CREATE BUNDLE name FIBER (... ENCRYPTED) ...` (bundle-level keyword) keeps working. Parser routes it to: every fiber field gets the type-default mode (numeric → Affine, text → Opaque, binary → Opaque). This is the "graceful upgrade" story for any v0.1 bundles already in production at Stream 0.5.0.

### 3.5 Acceptance gate

- All test names in §3.1 pass
- Existing `geo_enc_1` … `geo_enc_15` tests still pass (no regression on v0.1 path)
- Manual confirm: `CREATE BUNDLE` with each new mode keyword parses successfully and creates the right `EncryptionMode` on each field

---

## 4. Sprint B — `OPAQUE` mode (AEAD on TEXT/BINARY)

> **Module**: `src/crypto.rs` (extend `FieldTransform` enum), `src/bundle.rs` (insert/reconstruct paths)
> **Cipher**: AES-256-GCM-SIV (RFC 8452, nonce-misuse-resistant; Rust crate: `aes-gcm-siv`)

### 4.1 The math

```
Enc^opaque_g (v, n) = (AES-GCM-SIV_k(v, n),  tag_k(v, n))
                       with n = per-record nonce
```

Per-record nonce drawn from a 96-bit counter seeded at bundle creation, persisted in WAL, and incremented atomically per insert. Nonce reuse is fatal for AEAD; we use SIV mode specifically because it's resistant to *accidental* nonce reuse, but we still maintain a counter to avoid it on purpose.

### 4.2 Test names

```rust
#[test] fn test_opaque_encrypt_decrypt_roundtrip_text() { /* "Hello" → bytes → "Hello" */ }
#[test] fn test_opaque_encrypt_decrypt_roundtrip_binary() { /* random Vec<u8> survives */ }
#[test] fn test_opaque_same_plaintext_different_ciphertext() { /* "x" twice → distinct ciphertexts */ }
#[test] fn test_opaque_tamper_detection() { /* flip one ciphertext bit → decrypt errors */ }
#[test] fn test_opaque_aad_binds_to_field_position() { /* swapping ciphertext between fields fails decrypt */ }
#[test] fn test_opaque_nonce_counter_persists_across_reload() { /* counter loaded from WAL on engine restart */ }
#[test] fn test_opaque_round_trip_10000_records() { /* throughput bench, 100% correctness */ }
#[test] fn test_opaque_curvature_invariance() { /* on a numeric+text bundle, K(text-encrypted) == K(plaintext) */ }
#[test] fn test_opaque_no_equality_query_path() { /* OPAQUE columns are NOT equality-queryable; engine returns NotIndexed error */ }
#[test] fn test_opaque_select_decrypts_for_response_only() { /* SELECT with key returns plaintext; without key returns ciphertext bytes */ }
```

### 4.3 `FieldTransform` enum extension

```rust
// src/crypto.rs

#[derive(Debug, Clone)]
pub enum FieldTransform {
    Affine { scale: f64, offset: f64 },              // v0.1
    Opaque { key: [u8; 32], nonce_counter: Arc<AtomicU64> },
    Indexed { key: [u8; 32] },                       // Sprint C
    Probabilistic { scale: f64, offset: f64, sigma: f64, key_for_bucketing: [u8; 32] },  // Sprint D
    Isometric { group_id: String, matrix: DMatrix<f64>, offset: DVector<f64> },           // Sprint E
}

impl FieldTransform {
    pub fn encrypt_value(&self, v: &Value, aad: &Aad) -> Value { ... }
    pub fn decrypt_value(&self, w: &Value, aad: &Aad) -> Value { ... }
}

#[derive(Debug, Clone)]
pub struct Aad {
    pub bundle: String,
    pub field_name: String,
    pub field_idx: u16,
    pub record_id: Option<u64>,                       // base point, if known at encrypt time
}
```

The `Aad` (Associated Authenticated Data) is what binds a ciphertext to its position in the bundle. Swapping a ciphertext from `legalName` into `addressLine1` will fail authentication on decrypt — the AAD won't match.

### 4.4 `bundle.rs` integration

- `insert()` — for each fiber field, dispatch on its `FieldTransform` variant. Numeric goes through the existing affine path (unchanged). Text/Binary with `Opaque` gets AEAD-encrypted with a freshly-incremented nonce.
- `reconstruct()` — same dispatch, applying inverse. Decrypt errors (auth tag mismatch) bubble up as record-corruption signals.
- `range_query()` / `WHERE` — `Opaque` fields are NOT comparable; engine returns a typed `NotIndexed` error if a query touches them with comparison operators.
- DHOOM compression: AEAD ciphertext is high-entropy and won't compress well. That's expected; document in DHOOM §4 that OPAQUE fields skip the deviation-from-zero compression and store ciphertext verbatim.

### 4.5 Wire format additions

The existing serialized record format extends with a per-field tag byte:

```
field tag byte (1B):
  0x00  Plaintext
  0x01  Affine  (no on-disk format change vs v0.1)
  0x02  Opaque  → followed by [u96 nonce | u32 ct_len | ct bytes | u128 tag]
  0x03  Indexed → followed by [u32 ct_len | ct bytes]                       (Sprint C)
  0x04  Probabilistic → [f64 noisy_value | u64 bucket_hash]                  (Sprint D)
  0x05  Isometric → [k×8 bytes for k components]                             (Sprint E)
```

v0.1 records (no tag byte) are forward-compatible: the bundle's `EncryptionMode::Affine` declaration tells the reader to expect the v0.1 serialized form. New v0.2 bundles always emit the tag byte.

### 4.6 Acceptance gate

- All test names in §4.2 pass
- `geo_enc_1` … `geo_enc_15` still pass
- New: K(plaintext bundle) == K(equivalent OPAQUE-encrypted bundle on numeric fields) — verifies cross-mode invariance
- Performance: AEAD per-field cost <10µs (the AES-GCM-SIV crate runs ~1GB/s on modern x86; per-field encrypt is dominated by tag computation)

---

## 5. Sprint C — `INDEXED` mode (PRF on TEXT)

> **Module**: `src/crypto.rs`, `src/bundle.rs`
> **PRF**: AES-256-CMAC (preferred — NIST-standardized; via `aes` + `cmac` Rust crates) OR keyed SipHash-2-4 (smaller dependency surface)

### 5.1 The math

```
Enc^indexed_g (v) = PRF_k (v)
v_1 = v_2  ⟺  PRF_k(v_1) = PRF_k(v_2)
```

Deterministic. Equal plaintexts produce equal ciphertexts. This makes the categorical bitmap index work verbatim on encrypted columns — the equivalence relation is preserved.

**Frequency-leakage caveat (already documented on the gigi-encrypt page §06):** INDEXED is ONLY safe for high-cardinality columns where frequency analysis yields no useful signal (UUIDs, row keys, 64-bit IDs, email addresses). The parser SHOULD warn (or refuse) when INDEXED is declared on a column with cardinality declared low (e.g., enum-y CATEGORICAL).

### 5.2 Test names

```rust
#[test] fn test_indexed_deterministic() { /* same plaintext → same ciphertext, 1000× */ }
#[test] fn test_indexed_distinct_plaintexts_distinct_ciphertexts() { /* all-pairs check on 10k random strings */ }
#[test] fn test_indexed_round_trip_via_prf_inverse() { /* PRF is one-way; verify decrypt path stores plaintext-PRF table when bundle has key */ }
#[test] fn test_indexed_equality_query_works_on_encrypted_column() { /* SELECT WHERE field = 'x' returns rows where stored value matches PRF('x') */ }
#[test] fn test_indexed_bitmap_index_intact_under_encryption() { /* spectral.rs test: λ₁(plaintext) == λ₁(encrypted) */ }
#[test] fn test_indexed_warns_on_low_cardinality_column() { /* declared CATEGORICAL with INDEXED → engine warning logged */ }
#[test] fn test_indexed_aad_binds_to_field_position() { /* PRF key is per-(bundle, field), not global */ }
```

### 5.3 Implementation notes

- AES-CMAC needs a 128-bit (or 256-bit) PRF output. We use 128 bits truncated, encoded as `Vec<u8>` of length 16. Sufficient for collision resistance at our cardinalities.
- The PRF KEY is derived from `(seed, field_name)` via the same wyhash KDF used for the affine path — already in `crypto.rs::mix_hash`.
- Storage: 16 bytes per `INDEXED` field per record. The DHOOM compressor sees this as high-entropy and stores it verbatim.
- The plaintext-PRF mapping (for SELECT decryption) requires either (a) re-running the PRF on a plaintext-input table, or (b) storing a separate decrypt-key. We pick (a) for INDEXED — engines that hold the GaugeKey can recover plaintext by re-PRF'ing the question. PRFs are one-way; you can never decrypt INDEXED without already knowing what to look for. This is fine for an equality-search column (the lookup IS the question).

### 5.4 Acceptance gate

- All test names in §5.2 pass
- `geo_enc_*` regression intact
- Spectral gap invariance holds: encrypted bitmap-index Laplacian λ₁ matches plaintext to machine precision
- New: cross-bundle join on an encrypted INDEXED key works (both sides have the same PRF key derived deterministically)

---

## 6. Sprint D — `PROBABILISTIC` mode (gauge + Gaussian + Davis Identity)

> **Module**: `src/crypto.rs`, `src/bundle.rs`, `src/parser.rs` (SIGMA literal)
> **Math**: `Enc^prob(v) = a·v + b + ε`, `ε ~ N(0, σ²)`, equality via `Eq(w₁, w₂) = 𝟙[d²(w₁, w₂) ≤ (3σ)²]`

### 6.1 The math

From `gigi-encrypt.html` §05 and the published Davis-Identity research:

```
Enc^prob_g (v) = a·v + b + ε,   ε ~ N(0, σ²)
Eq(w₁, w₂)    = 𝟙[ d²(w₁, w₂) ≤ (3σ)² ]
```

The Davis Identity `S + d² = 1` (sameness + squared deviation = unity) makes equality a distance check that survives noise. For O(1) lookup, we bucket plaintext at σ-resolution before hashing:

```
bucket(v) = hash_k(round_to_sigma(v))
```

`round_to_sigma` snaps the plaintext to the nearest multiple of σ before hashing. Two plaintexts within σ/2 round to the same bucket and hash equal. The 3σ recall claim (96.6% on N=20k, per the live test suite) is the validated end-state.

### 6.2 Test names

```rust
#[test] fn test_probabilistic_distinct_ciphertexts_for_same_plaintext() { /* same v, 1000 encrypts, 1000 distinct values */ }
#[test] fn test_probabilistic_equality_recall_at_3sigma() { /* same plaintext → equal-by-distance ≥96% on N=20k */ }
#[test] fn test_probabilistic_equality_fpr_at_5sigma_separation() { /* Δv = 5σ/|a| → equal-by-distance ≤8% */ }
#[test] fn test_probabilistic_curvature_invariance() { /* K invariant under affine; noise increases variance by σ² but the affine ratio still holds */ }
#[test] fn test_probabilistic_chosen_plaintext_distinguisher_advantage_below_sigma_target() { /* empirical advantage matches schema-declared σ */ }
#[test] fn test_probabilistic_round_trip_below_sigma_tolerance() { /* decrypt(encrypt(v)) = v ± σ */ }
#[test] fn test_probabilistic_bucket_lookup_o1() { /* equality check is HashMap probe, not linear scan */ }
#[test] fn test_probabilistic_sigma_must_be_positive() { /* parser rejects SIGMA 0, SIGMA -1 */ }
```

### 6.3 Implementation notes

- Gaussian sample: Rust crate `rand_distr::Normal`. Seed the RNG per-field from the GaugeKey + field-position hash (deterministic across engine restarts is NOT required for noise; we want fresh randomness per encrypt).
- Bucketing: store `(noisy_value, bucket_hash)` as the on-disk record. Equality query computes `bucket_hash` from the literal and probes the HashMap. The `noisy_value` is what enables decryption (subtract the noise's expected value, recover `a·v + b`, invert to `v`).
- Decryption error scales with σ. The plaintext recovery is approximate: `v_hat = (w_no_noise - b) / a`, with `|v_hat - v| ~ σ/|a|`. Acceptable for analytics; not acceptable for record-fidelity reads. We document this in the spec and require schema designers to choose σ relative to the precision they need.
- The Davis Identity test (`tests/encryption_strong_claims_validation.py::D_DavisIdentity_NeighborhoodEquality`) becomes the recall/FPR validation. We translate that test into Rust against the live engine.

### 6.4 Acceptance gate

- All test names in §6.2 pass
- Recall ≥96% at 3σ on a numeric column with 20,000 records — matches the published evidence
- FPR ≤8% at 5σ separation — matches the published evidence
- Statistical-unlinkability empirical distinguisher advantage ≤ schema-declared σ at parameter (a=2.5, σ=0.5)

---

## 7. Sprint E — `ISOMETRIC` mode (O(k) for grouped numeric)

> **Module**: `src/crypto.rs`, `src/parser.rs` (GROUP fiber syntax)
> **Math**: `ρ_g(v) = O·v + b`, `O ∈ O(k)` orthogonal

### 7.1 The math

For grouped numeric fiber declared with `GROUP wind { wind_x, wind_y, wind_z } NUMERIC ENCRYPTED ISOMETRIC`, the encryption applies a single shared orthogonal matrix `O ∈ O(k)` to the k-component vector:

```
ρ_g(v) = O·v + b,   O ∈ O(k),   O^T O = I
||O·u - O·v||  =  ||u - v||              (isometry)
```

Sample `O` from the GaugeKey via QR decomposition of a seeded random Gaussian matrix:

```
G ~ N(0,1)^{k×k}     (seeded from GaugeKey)
O, R = QR(G)
O = O · diag(sign(diag(R)))             (canonicalize sign)
```

Pairwise distances in the group are preserved exactly. Holonomy eigenvalue spectrum is preserved. Useful for: wind components, IMU readings, embedding vectors where Euclidean-geometry queries (KNN, clustering, similarity) need exact distance preservation.

### 7.2 Test names

```rust
#[test] fn test_isometric_orthogonality_to_machine_precision() { /* O^T O ≈ I with max |O^T O - I| < 1e-10 */ }
#[test] fn test_isometric_pairwise_distance_preservation() { /* ||O u - O v|| == ||u - v|| for 100 random pairs */ }
#[test] fn test_isometric_holonomy_eigenvalue_invariant() { /* eigenvalues of holonomy operator equal pre/post encryption */ }
#[test] fn test_isometric_round_trip() { /* O^T (O·v + b - b) == v */ }
#[test] fn test_isometric_only_valid_in_group() { /* parser error if ISOMETRIC declared on non-grouped fiber */ }
#[test] fn test_isometric_group_size_k_supported_up_to_64() { /* k=2, 3, 4, 8, 16, 32, 64 */ }
```

### 7.3 Implementation notes

- Use `nalgebra` crate (already a likely dep for spectral.rs) for matrix ops + QR decomposition.
- Sample `O` deterministically from the GaugeKey: derive `k×k` f64 values via repeated wyhash mixing, fill a Gaussian matrix via Box-Muller transform, QR-decompose.
- Storage: same number of bytes as plaintext (k×8 per record). DHOOM compression: works as for plaintext numeric (zero-deviation comparisons still meaningful since distances preserved).

### 7.4 Acceptance gate

- All test names in §7.2 pass
- Pairwise distances exact to 10⁻¹⁰ — matches the gigi-encrypt page evidence
- Holonomy eigenvalue spectrum invariant to 10⁻⁹ — matches the page evidence
- `Ask4B_IsometricEncryption` Python tests (rev-1 suite) pass against the Rust engine

---

## 8. Sprint F — User-supplied seed + key handle

> **Module**: `src/parser.rs`, `src/crypto.rs`, `src/types.rs`
> **GQL**: `WITH ENCRYPTION SEED 'a1b2c3...64hex'`, `WITH ENCRYPTION SEED FROM ENV $JG_GIGI_SEED`

### 8.1 The need

v0.1 generates a random seed inside `crypto.rs::random_seed()`. That seed is stored in the schema and persisted through WAL — but it cannot be re-derived from anything outside the engine. This means:

- Same bundle on a different deployment has a different key
- Cannot rotate keys via env var change + redeploy
- Cannot integrate with KMS / HSM
- Backups need to include the schema (which has the GaugeKey) — increases blast radius

v0.2 adds:
1. `WITH ENCRYPTION SEED $hex` — directly supply a 32-byte seed at bundle creation
2. `WITH ENCRYPTION SEED FROM ENV $name` — read the seed from a named env var at engine startup; the schema stores only the env-var name, not the seed itself
3. `WITH ENCRYPTION SEED FROM KMS $arn` — placeholder for AWS KMS / similar; not in v0.2 scope, but parser leaves room

### 8.2 Test names

```rust
#[test] fn test_create_bundle_with_encryption_seed_hex() { /* parses; stores hex seed verbatim */ }
#[test] fn test_create_bundle_seed_hex_must_be_64_chars() { /* parser error on length != 64 */ }
#[test] fn test_create_bundle_seed_hex_must_be_valid_hex() { /* parser error on non-hex chars */ }
#[test] fn test_create_bundle_seed_from_env_resolves_at_engine_load() { /* schema stores env-var name; engine resolves at startup */ }
#[test] fn test_engine_load_fails_when_env_seed_missing() { /* required seed env var not set → engine startup error */ }
#[test] fn test_two_bundles_same_seed_produce_same_gauge_key_for_same_field_set() { /* deterministic across deployments */ }
#[test] fn test_random_seed_path_still_works_for_v0_1_compat() { /* CREATE BUNDLE ... ENCRYPTED (no WITH SEED) generates random */ }
```

### 8.3 Schema additions

```rust
// src/types.rs

#[derive(Debug, Clone)]
pub enum EncryptionSeedSource {
    Random,                                  // v0.1; default
    Hex([u8; 32]),                           // direct
    Env(String),                             // resolved at engine load
    Kms { provider: String, arn: String },   // future
}

pub struct BundleSchema {
    // ...
    pub encryption_seed_source: EncryptionSeedSource,
}
```

The seed itself (after resolution) lives only in the GaugeKey held in memory; the schema metadata stores only the source description. Backups serialize the source, not the seed.

### 8.4 Acceptance gate

- All test names pass
- Engine startup with `WITH ENCRYPTION SEED FROM ENV $X` and `X` unset fails with a clean error message naming the env var
- v0.1 random-seed path is unchanged; existing bundles continue to work

---

## 9. Sprint G — Forward-secret key rotation

> **Module**: `src/parser.rs` (ROTATE_KEY GQL), `src/crypto.rs` (rotation primitive), `src/bundle.rs` (atomic re-key), `src/wal.rs` (atomicity), new RG-flow snapshot module
> **GQL**: `GAUGE bundle ROTATE_KEY FORWARD_SECRET`

### 9.1 The math

From the gigi-encrypt page §05 and §06:

```
ROTATE_KEY FORWARD_SECRET:
  (s, g) → (s', g')                           # both seeds rotate
  Φ_RG (snapshot_t)  applied before drop      # one RG-flow step on pre-rotation snapshot
  ΔS_RG ≥ 0                                   # entropy monotonic by 2nd law
```

Two seeds rotate atomically:
- `s` — the base-space hash seed (the keyed hash that maps plaintext keys → u64 base points)
- `g` — the GaugeKey seed (the affine/AEAD/PRF parameters per fiber field)

The pre-rotation snapshot of the bundle's data passes through one RG-flow step (coarse-graining at bin width `w`) before being dropped. This achieves **differential forward secrecy**: an attacker with the post-rotation `(s', g')` cannot:
1. Resolve old base points (`s` is gone; cannot re-hash plaintext keys to old positions)
2. Decrypt backed-up ciphertext (`g` is gone)
3. Refine RG-coarsened aggregates below `w` (entropy increased irreversibly)

### 9.2 Test names

```rust
#[test] fn test_rotate_key_atomicity_via_wal() { /* engine crash mid-rotation rolls back cleanly */ }
#[test] fn test_rotate_key_old_seed_cannot_lookup_post_rotation() { /* 1000 keys: 0% lookup hit rate with old s */ }
#[test] fn test_rotate_key_old_gauge_cannot_decrypt_post_rotation() { /* 1000 records: 0% decrypt success with old g */ }
#[test] fn test_rotate_key_record_count_invariant() { /* records before == records after */ }
#[test] fn test_rotate_key_curvature_after_rg_step_increases_or_equal() { /* RG entropy monotonicity: ΔS ≥ 0 */ }
#[test] fn test_rotate_key_aggregates_above_bin_width_preserved() { /* SUM, AVG at coarse resolution unchanged */ }
#[test] fn test_rotate_key_aggregates_below_bin_width_lost() { /* fine-grained queries return coarse-only result */ }
#[test] fn test_rotate_key_concurrent_writes_block_during_rotation() { /* writes during ROTATE_KEY queue, complete after */ }
```

### 9.3 Implementation notes

- WAL: rotation is a single transaction. Begin → re-derive `(s', g')` from new seed source → re-encrypt all records under `(s', g')` → run RG step on the old snapshot → commit. Engine crash during this transaction: WAL replay completes the rotation OR rolls back to pre-rotation state. No half-rotated bundle.
- Re-encrypt cost: O(N) where N is record count. For 11M-record bundles, this takes minutes — schedule during low-traffic windows. The Aff(ℝ) closure (gigi-encrypt page row "Rotate-key composition") means the affine numeric path can re-encrypt without materializing plaintext: `(a₂/a₁, b₂ - b₁·a₂/a₁)` is the rekey transform applied directly to ciphertext. OPAQUE / INDEXED / PROBABILISTIC need full decrypt + re-encrypt (no closure).
- RG flow: implement as an aggregate-bin coarsening over the snapshot. For a numeric column at bin width `w`, replace the records with one summary record per bin. Categorical columns: one summary per distinct value (no coarsening below cardinality). Document `w` as schema-declared per-bundle.
- Schedule: rotation is admin-triggered (`GAUGE bundle ROTATE_KEY FORWARD_SECRET` GQL command). Not automatic. Apps that want scheduled rotation issue the command from a cron.

### 9.4 Acceptance gate

- All test names in §9.2 pass
- The 1,000-key forward-secrecy test (gigi-encrypt page §06 evidence: "old-seed lookup hit rate post-rotation: 0.00%") passes against the live engine
- WAL crash-mid-rotation test passes: kill -9 the engine during rotation, restart, verify either pre-rotation or post-rotation state (never a mix)

---

## 10. Sprint H — `PROJECT INVARIANT (...)` query form

> **Module**: `src/parser.rs` (new statement), `src/engine.rs` (statement dispatch), no new computation (existing curvature/spectral/anomaly endpoints supply the values)
> **GQL**: `PROJECT INVARIANT (curvature, confidence, capacity, spectral_gap, holonomy, beta_1, ...) FROM bundle [WHERE ...]`

### 10.1 The need

Today, gauge-invariant analytics are reachable via separate REST endpoints (`/v1/bundles/<name>/curvature`, `/v1/bundles/<name>/spectral`, `/v1/bundles/<name>/explain`, etc.). They WORK on encrypted data — that's the v0.1 win. But they're not unified under a single GQL query form, and they're not structurally guaranteed to never decrypt — that property holds only because each individual endpoint happens to be implemented that way.

`PROJECT INVARIANT (...)` makes the property structural:

- It's a single GQL statement
- The parser accepts only invariant-ring operations (curvature, confidence, capacity, spectral_gap, holonomy, beta_k, sheaf_h1, plus + and × of those)
- The engine routes to the existing endpoints internally
- The execution path has NO branch that calls `decrypt_fiber()`. Static-analysis-friendly. A test asserts no decryption code is reachable from the PROJECT INVARIANT execution path.

### 10.2 Test names

```rust
#[test] fn test_project_invariant_returns_curvature_value() { /* PROJECT INVARIANT (curvature) FROM b → {curvature: 0.034} */ }
#[test] fn test_project_invariant_returns_multiple_invariants() { /* PROJECT INVARIANT (curvature, confidence, beta_1) returns all three */ }
#[test] fn test_project_invariant_arithmetic_on_invariants() { /* PROJECT INVARIANT (capacity * confidence) returns the product */ }
#[test] fn test_project_invariant_rejects_non_invariant_ops() { /* PROJECT INVARIANT (sum(field)) → parser error: sum not in invariant ring */ }
#[test] fn test_project_invariant_zero_decrypt_calls_in_execution_path() { /* tracing test: count of `decrypt_fiber` calls during PROJECT INVARIANT == 0 */ }
#[test] fn test_project_invariant_works_on_encrypted_bundle() { /* same invariant values as plaintext bundle */ }
#[test] fn test_project_invariant_with_where_clause() { /* WHERE filters records before invariant computation */ }
```

### 10.3 Parser additions

```rust
// AST
Statement::ProjectInvariant {
    bundle: String,
    invariants: Vec<InvariantExpr>,        // closed under +, ×
    where_clause: Option<Predicate>,
}

enum InvariantExpr {
    Op(InvariantOp),                       // curvature, confidence, capacity, spectral_gap, holonomy, beta_k(k), sheaf_h1
    Add(Box<InvariantExpr>, Box<InvariantExpr>),
    Mul(Box<InvariantExpr>, Box<InvariantExpr>),
    Const(f64),
}
```

The parser maintains a whitelist of invariant operations; anything outside it is a syntax error AT PARSE TIME. This is what makes "0 bytes decrypted" a structural property: a query that compiles is one that the engine can prove never reaches a decryption code path.

### 10.4 Acceptance gate

- All test names pass
- Tracing test: zero `decrypt_*` function calls during PROJECT INVARIANT execution
- Existing per-endpoint behavior unchanged (curvature, spectral, etc. still callable directly via REST)
- The "Invariant-ring query surface" line in `gigi-encrypt.html` §06 has a passing test backing it

---

## 11. TDD discipline + acceptance gates

Every sprint follows the discipline already running in this repo:

1. **Test names FIRST.** Acceptance test names land in the relevant `tests/` module as `#[test] fn name() {}` with no body or `unimplemented!()`. They show up as failing before any implementation.
2. **Implementation BEHIND a test.** A commit that adds production code without a test in the same commit (or in a prior commit naming the test) is a discipline violation.
3. **Existing 598+43 (Rust) and 34+ (Python) tests stay green.** Sprint completion = sprint tests pass + regression suite intact.
4. **Latency budgets per primitive.** Each new mode has a perf microbench; encryption per-field cost stays under documented bounds (Affine: 5ns; Opaque: 10µs; Indexed: 1µs; Probabilistic: 2µs; Isometric: 50ns × k²).
5. **The math has a witness in the test suite.** Every claim on `gigi-encrypt.html` Band 1 traces to a Rust test that asserts the mathematical property numerically.

Sprint completion checklist (machine-readable, copy-paste from each sprint's §X.4 acceptance gate):
- [ ] All sprint test names pass
- [ ] Existing regression suite intact (598+43+34 = 675 baseline, growing per sprint)
- [ ] Perf bench within budget
- [ ] Documentation updated (`GQL_REFERENCE.md`, `GIGI_API.md`, the gigi-encrypt page if claims change)
- [ ] `gigi-encrypt.html` corresponding row's evidence numbers verified or updated against fresh runs

---

## 12. Validation matrix — what proves what

| Sprint | New tests | Validates `gigi-encrypt.html` claim |
|:---:|:---:|---|
| A | 11 | (none — plumbing) |
| B | 10 | "AEAD on opaque TEXT/BINARY" row |
| C | 7 | "Deterministic PRF on TEXT/CATEGORICAL" row |
| D | 8 | "Statistical unlinkability + queryable equality (PROBABILISTIC)" row |
| E | 6 | "Isometric O(k) group gauge" row |
| F | 7 | (operational; supports backup/key-management story) |
| G | 8 | "Forward-secret key rotation" row + "Rotate-key composition" row |
| H | 7 | "Invariant-ring query surface" row + the structural "0 bytes decrypted" claim |
| **Total** | **~64** | All 9 currently-unbacked Band 1 rows |

After v0.2 ships, every row in the gigi-encrypt page §06 "Shipping in GIGI Encrypt" band has a passing test in this repo. The page becomes literal-truth instead of aspirational.

---

## 13. Wire format / on-disk schema

### 13.1 Per-field tag byte (already covered §4.5)

```
0x00  Plaintext
0x01  Affine            v0.1, no on-disk change vs current
0x02  Opaque            [u96 nonce | u32 ct_len | ct | u128 tag]
0x03  Indexed           [u32 ct_len | ct]                    (16 B for AES-CMAC)
0x04  Probabilistic     [f64 noisy_value | u64 bucket_hash]
0x05  Isometric         [k × f64]
0x06  RESERVED          (Band 2: lattice-fiber PQ)
```

### 13.2 Schema version bump

`BundleSchema` gains a `version: u32` field. v0.1 schemas serialize as `version=1`; v0.2 as `version=2`. Engine reading a `version=1` schema applies the v0.1 path (single bundle-level ENCRYPTED → all numeric fields Affine, all text identity). Engine reading `version=2` honors the per-field `EncryptionMode`. No v0.1 bundle requires migration; the upgrade is in-place.

### 13.3 GaugeKey serialization

The `GaugeKey` struct extends to hold the `FieldTransform` enum variants. Serialization adds a per-field discriminator byte matching the §13.1 tag. Backup files include the schema (with key) — same blast radius as v0.1, no change.

---

## 14. Operational concerns

### 14.1 Performance envelope

| Operation | v0.1 | v0.2 |
|---|---|---|
| Affine encrypt (per field) | dominated by f64 mul-add | unchanged |
| Opaque encrypt (per field) | n/a | dominated by AES-GCM-SIV tag computation |
| Indexed encrypt (per field) | n/a | dominated by AES-CMAC over canonical-length input |
| Probabilistic encrypt (per field) | n/a | dominated by Gaussian sample + bucket hash |
| Isometric encrypt (per group of k components) | n/a | dominated by k×k matrix-vector multiply |
| Curvature on encrypted bundle | unchanged | unchanged |

Each new mode gets its own microbench in `benches/` so regressions surface immediately. Absolute targets are set after the first bench run lands, not in advance.

### 14.2 Memory

Per-bundle GaugeKey grows: from 16 B per numeric field (v0.1) to a small per-field union covering whichever mode applies (v0.2). For a 7-field bundle the GaugeKey stays well under 1 KB.

### 14.3 Backup compatibility

v0.2 backups include the version-2 schema with full GaugeKey. v0.1 backups load on v0.2 engines via the version-discriminated load path (tested via Sprint A's acceptance gate).

---

## 15. Out of scope (Band 2 — explicit non-goals for v0.2)

Per the gigi-encrypt page "On the Horizon" band — these are not in v0.2:

- Curvature-MAC bundle integrity
- Čech-threshold key sharing
- Geodesic-ball ABE
- Spectral-signature ZKP
- Holonomy ledger
- Lattice-fiber PQ structure group
- RG-flow ratchet (continuous forward secrecy)
- Proxy re-encryption (note: Aff(ℝ) closure result IS in v0.2 via Sprint G; the proxy variant of it is Band 2)

If any of these become urgent ahead of plan, they get their own sprint specs.

---

## 16. The closing line

v0.2 ships every Band 1 feature on the gigi-encrypt landing page. After it lands, every claim on that page has a passing test in this repo, every mode is per-field declarable in GQL, every key has a managed lifecycle including forward-secret rotation, and `PROJECT INVARIANT (...)` makes "0 bytes decrypted" a structural property of the query language.

The downstream effect on the application stack: the `jg_account` migration in `gworls-platform-spec.md` becomes a pure schema declaration, no application-side AEAD, no `lib/jg-pii-crypto.js`, no envelope routing. One source of truth, one test surface, one lifecycle.

Owned end-to-end. The way it should be.

— Bee, with Claude Code

---

*Companion documents: `GIGI_GEOMETRIC_ENCRYPTION_SPEC.md` (v0.1 foundation), `gigi-encrypt.html` (public landing), `tests/encryption_strong_claims_validation.py` + `tests/encryption_math_validation.py` (math validation suite).*
*Document owner: Bee Rosa Davis*
*Next review: at end of Sprint A.*
