# GIGI Geometric Encryption

## Compute on Encrypted Data at Native Speed

**Author:** Bee Rosa Davis · Davis Geometric
**Date:** March 18, 2026
**Version:** 0.1

---

## 0. The Claim

GIGI can compute curvature, detect anomalies, predict future events, measure spectral connectivity, and evaluate data consistency on encrypted data — **at native speed, with zero performance penalty** — because the encryption is not a layer bolted onto the database. **The geometry IS the encryption.**

This is not homomorphic encryption (which is 10,000× slower). This is not trusted execution environments (which require special hardware). This is a consequence of differential geometry: gauge-invariant quantities are computable on ANY valid coordinate representation of the fiber, including one scrambled by a secret key.

---

## 1. Mathematical Foundation

### 1.1 Recap: The Fiber Bundle (from GIGI_SPEC §1.1)

GIGI stores data as sections of a fiber bundle (E, B, F, π, Φ):
- **B** — base space (keys), addressed by the GIGI hash G: K₁ × ... × Kₘ → ℤ₂⁶⁴
- **F** = F₁ × ... × Fₖ — fiber (non-key field values)
- A record is a section σ: p ↦ (p, v₁, ..., vₖ) ∈ E

Currently, fiber values are stored as **plaintext** — the raw `Value` enum variants go directly into `Vec<Value>`. The read path reconstructs them verbatim.

### 1.2 The Structure Group

**Definition 1.2.1 (Structure Group).** The *structure group* of the data fiber bundle is the group of fiber automorphisms G ⊂ Aut(F) that preserve the geometric content needed for GIGI operations.

For a fiber F = F₁ × ... × Fₖ with numeric components, consider the group of **affine transformations** acting component-wise:

    g(v₁, ..., vₖ) = (a₁v₁ + b₁, ..., aₖvₖ + bₖ)

where aᵢ ≠ 0 for all i. This group preserves:
- **Variance/range² ratio** (scalar curvature K): Var(av+b) = a²·Var(v), range(av+b) = |a|·range(v), so Var/range² is invariant
- **Equality of indexed values** (spectral topology): if we use index-aware encryption (see §2.3)
- **Distance ratios** in the fiber metric (partition function): d(g(u), g(v)) / range = d(u,v) / range when normalized

### 1.3 Secret Gauge Transformation = Encryption

**Definition 1.3.1 (Geometric Encryption Key).** A *geometric encryption key* is a secret element g ∈ G parameterized by a 256-bit seed. Specifically:

    GaugeKey(seed) → {(a₁, b₁), ..., (aₖ, bₖ)}

where each (aᵢ, bᵢ) is derived from the seed via a KDF (key derivation function) per field:

    (aᵢ, bᵢ) = KDF(seed, field_name_i, field_type_i)

For numeric fields (Float, Integer):
- aᵢ ∈ ℝ \ {0} — a nonzero scale factor
- bᵢ ∈ ℝ — an offset

For text fields:
- A keyed permutation cipher on characters (preserving equality when needed, or full encryption when not indexed)

For boolean fields:
- Optionally a bit flip keyed by the seed (or identity if indexed)

**Definition 1.3.2 (Geometric Encryption).** The *geometric encryption* of a section σ under key g is:

    Enc_g(σ)(p) = (p, g(v₁, ..., vₖ)) = (p, a₁v₁ + b₁, ..., aₖvₖ + bₖ)

The base point p is **unchanged** (keys are not encrypted — they define the topology). Only fiber values are transformed.

**Definition 1.3.3 (Geometric Decryption).** The *geometric decryption* applies the inverse:

    Dec_g(σ)(p) = (p, g⁻¹(w₁, ..., wₖ)) = (p, (w₁ - b₁)/a₁, ..., (wₖ - bₖ)/aₖ)

This recovers the plaintext fiber values.

---

## 2. What Works on Encrypted Data (and Why)

### 2.1 Gauge-Invariant Operations (Zero Performance Penalty)

These operations produce **identical results** on encrypted and plaintext data:

| Operation | Why It Works | Proof |
|---|---|---|
| **Scalar curvature K** | K = Var/range². Var(av+b) = a²Var(v), range(av+b) = \|a\|·range(v). Ratio is invariant. | Thm 5a.1 in GIGI_SPEC |
| **Confidence 1/(1+K)** | Derived from K. | Direct |
| **Davis capacity C = τ/K** | τ is record count (gauge-invariant). K is invariant. | Direct |
| **Spectral gap λ₁** | Computed from field_index bitmap topology, not fiber values. | §1.2 of spectral.rs |
| **RG flow (C-theorem)** | Aggregation + curvature, both invariant. | Direct |
| **DHOOM compression ratio** | Deviation from zero section counts; the *number* of deviating fields is invariant (affine transform preserves non-default status). | Def 1.4 |
| **Anomaly detection** | "Is K above threshold?" — same answer on encrypted data. | Direct |
| **Prediction (K-based)** | "Cities above median K" — ordering of K values is preserved because K is invariant. | Direct |

### 2.2 Operations Requiring the Metric (Invariant with Metric Adaptation)

These operations use the fiber metric `d(u, v)`. Under the affine gauge transform, the metric transforms as:

    d_g(g(u), g(v)) = √(Σ ωᵢ · (aᵢ(uᵢ - vᵢ))² / range(aᵢvᵢ + bᵢ)²)
                    = √(Σ ωᵢ · (uᵢ - vᵢ)² / range(vᵢ)²)
                    = d(u, v)

Because the FiberMetric normalizes by range (Def 1.7 in GIGI_SPEC), and affine transforms scale numerator and denominator equally, **the metric is already gauge-invariant under affine encryption**. This means:

| Operation | Status |
|---|---|
| **Partition function Z** | Works unchanged — uses normalized distances |
| **Holonomy** | Works if formulated via metric distances (not raw differences) |
| **Range queries** | Need §2.3 treatment (see below) |
| **Pullback joins** | Work — key matching is on base space (unencrypted) |

### 2.3 Operations Requiring Decryption

These operations need plaintext values:

| Operation | Why | When Decrypted |
|---|---|---|
| **SELECT field values** | Human-readable output | At query response time |
| **WHERE field = literal** | Value comparison against user-supplied literal | Encrypt the literal, compare in encrypted space |
| **WHERE field > literal** | Order comparison | Affine transform preserves order (if a > 0), so compare encrypted literal |
| **Aggregation SUM/AVG** | Sum(av+b) = a·Sum(v) + n·b — recoverable with key | At response time, apply inverse |
| **GROUP BY on fiber field** | Equality test on fiber values | Use deterministic encryption for grouped fields |
| **Text search / LIKE** | Character-level access | Decrypt at query time |

**Key insight**: most "decryption" is actually just **transforming the query literal into encrypted space** rather than decrypting the data. The query `WHERE temp < -25` becomes `WHERE temp_enc < enc(-25)` — no data is ever decrypted.

### 2.4 The Comparison Table

| | AES-256 / TDE | Homomorphic (FHE) | GIGI Geometric |
|---|---|---|---|
| Compute on ciphertext | ✗ Must decrypt | ✓ But 10,000× slower | ✓ Native speed |
| Anomaly detection | Decrypt → compute | Theoretically possible | K is gauge-invariant |
| Prediction | Decrypt → ML pipeline | Not practical | K ordering preserved |
| Performance penalty | Decrypt + re-encrypt | 10,000×–1,000,000× | **0×** |
| Key needed for analytics | Yes (always) | No (but impractical speed) | **No** |
| Key needed for read | Yes | Yes (for decryption) | Yes |
| Data at rest | Encrypted (opaque) | Encrypted (opaque) | **Encrypted (geometric)** — has exploitable structure |
| Math foundation | Symmetric cipher | Lattice problems | Riemannian gauge invariance |

---

## 3. Architecture

### 3.1 Key Management

```
BundleSchema {
    ...
    gauge_key: Option<GaugeKey>,   // None = plaintext (legacy)
}

GaugeKey {
    field_transforms: Vec<FieldTransform>,   // one per fiber field, in schema order
}

FieldTransform {
    // For numeric (Float, Integer):
    scale: f64,      // aᵢ ≠ 0
    offset: f64,     // bᵢ

    // For text:
    permutation_key: [u8; 32],

    // For bool:
    flip: bool,
}
```

**Key derivation:**
```
fn derive_gauge_key(seed: &[u8; 32], schema: &BundleSchema) -> GaugeKey {
    for each fiber field f in schema.fiber_fields:
        field_seed = HKDF-SHA256(seed, salt = f.name || f.field_type)
        match f.field_type:
            Float | Integer =>
                scale = f64_from_bytes(field_seed[0..8])    // mapped to nonzero range
                offset = f64_from_bytes(field_seed[8..16])
            Text =>
                permutation_key = field_seed[0..32]
            Bool =>
                flip = field_seed[0] & 1
}
```

### 3.2 Data Flow

```
INSERT record
  │
  ├─ hash base fields → BasePoint (unchanged)
  │
  ├─ extract fiber values → Vec<Value>
  │
  ├─ IF gauge_key is Some:
  │     apply g(v) to each fiber value          ◄── NEW: ~5ns per field
  │
  ├─ store encrypted Vec<Value> at BasePoint
  │
  ├─ update FieldStats on encrypted values      ◄── K still correct (gauge-invariant)
  │
  └─ update field_index bitmaps                 ◄── uses encrypted values (equality preserved
                                                     for deterministic encryption)

CURVATURE / SPECTRAL / ANOMALY DETECTION
  │
  └─ unchanged — operates on encrypted fiber values
     K(encrypted) = K(plaintext)                ◄── Thm 5a.1

SELECT / point_query
  │
  ├─ lookup encrypted Vec<Value>
  │
  ├─ IF gauge_key is Some:
  │     apply g⁻¹(v) to each fiber value        ◄── NEW: ~5ns per field
  │
  └─ return plaintext Record

WHERE comparisons
  │
  ├─ transform the user literal through g       ◄── NEW: ~5ns
  │
  └─ compare in encrypted space (no data decrypted)
```

### 3.3 FieldStats Adaptation

FieldStats tracks `sum, sum_sq, min, max, count` for curvature:

- **Under affine transform** g(v) = av + b:
  - sum(g(v)) = a·sum(v) + n·b
  - sum_sq(g(v)) = a²·sum_sq(v) + 2ab·sum(v) + n·b²
  - min(g(v)) = a·min(v) + b (if a > 0)
  - max(g(v)) = a·max(v) + b (if a > 0)

- **Variance** = sum_sq/n - (sum/n)² → a²·Var(v). **Range** = max - min → |a|·range(v). **K = Var/range²** = Var(v)/range(v)² — **unchanged**.

No changes needed to FieldStats or curvature computation. They work on encrypted values and produce correct geometric results.

---

## 4. Security Analysis

### 4.1 What an Attacker Sees

An attacker with access to the stored data (but not the gauge key) sees:

- **Base points**: the u64 hashes of keys (already non-invertible via GIGI hash, but note: base points are deterministic from keys — key privacy is NOT a goal of geometric encryption)
- **Fiber values**: affine-transformed numbers. Without knowing (a, b) per field, the values are meaningless. An attacker sees `327.41` instead of `-31.9°C` — it could be any number.
- **FieldStats**: encrypted statistics. Since stats are affine transforms of the real stats, they leak information about variance ratios (which ARE curvature — and curvature is by design public). They do NOT leak absolute values.
- **Curvature K**: invariant — visible whether encrypted or not. This is a feature, not a bug. K is the geometric content; it's what GIGI advertises.

### 4.2 Threat Model

| Threat | Protected? | Notes |
|---|---|---|
| Data breach (disk stolen) | ✓ | Fiber values are affine-scrambled; a, b unknown |
| Insider reading raw values | ✓ | Must have gauge key to recover plaintext |
| Curvature visible to attacker | By design | K is gauge-invariant and public (it's the analytics output) |
| Known-plaintext attack | Partial | If attacker knows one (plaintext, ciphertext) pair, they can recover (a, b) for that field. Mitigations: field-level key rotation, composite transforms |
| Frequency analysis | Low risk | Affine transform on continuous values doesn't produce frequency patterns (unlike character-level substitution) |
| Admin computing analytics | ✓ | Admin sees curvature/anomalies without seeing data values |

### 4.3 What This Is NOT

- **Not a replacement for TLS** — data in transit still needs transport encryption
- **Not hiding the schema** — field names and types are visible (needed for geometric operations)
- **Not hiding keys** — base space (primary keys) are plaintext (needed for lookup)
- **Not hiding data distributions** — curvature reveals how "spread out" data is. This is the analytics output.
- **Not post-quantum** — if you need that, use it as a layer on top

### 4.4 What This IS

- **Data-at-rest protection** with zero-cost analytics — compute curvature/anomalies/predictions on scrambled data
- **Separation of concerns** — data scientists get geometric insights (K, λ₁, predictions) without seeing actual values
- **Compliance-friendly** — PII is never stored in plaintext; field values are geometrically transformed
- **Zero performance overhead** — the affine transform is ~5ns per field. On a 7-field record, that's 35ns per INSERT, negligible vs. the ~7μs total insert time

---

## 5. Implementation Plan

### 5.1 New Types (src/crypto.rs — new file)

| Type | Fields | Purpose |
|---|---|---|
| `GaugeKey` | `field_transforms: Vec<FieldTransform>` | Per-bundle encryption key |
| `FieldTransform` | `scale: f64, offset: f64` (numeric) or `perm_key: [u8; 32]` (text) | Per-field affine transform |

### 5.2 Modified Files

| File | Change | Impact |
|---|---|---|
| `src/types.rs` | Add `gauge_key: Option<GaugeKey>` to `BundleSchema` | Minimal |
| `src/bundle.rs` | Apply `g(v)` after fiber extraction in `insert()` | ~3 lines |
| `src/bundle.rs` | Apply `g⁻¹(v)` before reconstruction in `reconstruct()` | ~3 lines |
| `src/bundle.rs` | Transform query literals in `range_query()`, `filter()` | ~5 lines per |
| `src/parser.rs` | New GQL: `CREATE BUNDLE ... WITH ENCRYPTION SEED '...'` | New parse branch |
| `src/parser.rs` | New GQL: `DECRYPT bundle USING SEED '...'` for explicit read | New parse branch |
| `src/curvature.rs` | **Zero changes** — already gauge-invariant | None |
| `src/spectral.rs` | **Zero changes** — topology-only | None |
| `src/dhoom.rs` | Works on encrypted values (deviation from encrypted zero section) | None |

### 5.3 Untouched Files (Gauge-Invariant by Construction)

- `src/curvature.rs` — scalar_curvature, confidence, capacity, partition_function
- `src/spectral.rs` — spectral_gap, field_index_graph
- `src/aggregation.rs` — aggregations on encrypted values; inverse transform applied at response time
- `src/hash.rs` — base point computation (keys unaffected)
- `src/join.rs` — pullback joins (key-based)

### 5.4 New GQL Syntax

```sql
-- Create encrypted bundle
CREATE BUNDLE sensors (
    id INTEGER KEY,
    temp FLOAT,
    humidity FLOAT,
    pressure FLOAT
) WITH ENCRYPTION;
-- Server generates and stores a GaugeKey internally

-- Create with user-supplied seed (for reproducible keys)
CREATE BUNDLE sensors (...) WITH ENCRYPTION SEED $seed;

-- Insert (transparent — encryption happens internally)
INSERT INTO sensors {id: 1, temp: 22.5, humidity: 65, pressure: 1013.25};

-- Query (transparent — decryption happens at response)
SELECT FROM sensors WHERE id = 1;
-- Returns: {id: 1, temp: 22.5, humidity: 65, pressure: 1013.25}

-- Curvature (works on encrypted data — no key needed)
CURVATURE sensors;
-- Returns: K = 0.0341 (same as plaintext)

-- Analytics without decryption privilege
CURVATURE sensors;          -- works
SPECTRAL sensors;           -- works
SECTIONS sensors;           -- returns encrypted values (scrambled numbers)

-- Rotate encryption key
ALTER BUNDLE sensors ROTATE ENCRYPTION;
```

---

## 6. Test Matrix

| Test ID | Tests | Assertion |
|---|---|---|
| GEO-ENC-1 | Insert N records into encrypted bundle | All inserts succeed, stored values ≠ plaintext |
| GEO-ENC-2 | K(encrypted) vs K(plaintext) on same data | Identical to 10⁻¹⁰ |
| GEO-ENC-3 | Confidence(encrypted) vs Confidence(plaintext) | Identical |
| GEO-ENC-4 | λ₁(encrypted) vs λ₁(plaintext) | Identical |
| GEO-ENC-5 | Point query with key → plaintext values | Exact match |
| GEO-ENC-6 | Point query without key → encrypted values | Values ≠ plaintext |
| GEO-ENC-7 | WHERE comparison on encrypted bundle | Same result set as plaintext |
| GEO-ENC-8 | Range query on encrypted bundle | Same result set as plaintext |
| GEO-ENC-9 | Partition function Z on encrypted data | Same as plaintext |
| GEO-ENC-10 | DHOOM encoding of encrypted bundle | Valid encoding; different bytes; same compression ratio |
| GEO-ENC-11 | Key rotation: re-encrypt all fibers | K unchanged, new stored values ≠ old stored values |
| GEO-ENC-12 | Performance: encrypted insert vs plaintext | < 1% overhead |
| GEO-ENC-13 | Aggregation SUM on encrypted data | After inverse transform: correct sum |
| GEO-ENC-14 | NASA demo: all 7,320 records encrypted | K per city identical; predictions identical; anomaly detection identical |
| GEO-ENC-15 | Known-plaintext resistance | Given one (plain, enc) pair, cannot derive other field keys |

---

## 7. The Punchline

Every other database encrypts data by making it **opaque** — you can't compute on it until you decrypt it. Homomorphic encryption lets you compute but makes it **10,000× slower**.

GIGI encrypts data by changing **which coordinate system** the fiber is expressed in. The geometry — curvature, connectivity, anomalies, predictions — doesn't depend on coordinates. It depends on the **intrinsic structure of the manifold**.

The data is encrypted. The analytics are free. The geometry doesn't care.

```
Traditional:   Data → Encrypt → [opaque blob] → Decrypt → Compute → Result
Homomorphic:   Data → Encrypt → [opaque blob] → Compute (10,000× slower) → Decrypt → Result
GIGI:          Data → Gauge transform → [geometric representation] → Compute (native speed) → Result
                                                                          ↑
                                                                   Key not needed here
```
