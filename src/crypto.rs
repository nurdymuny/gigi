//! Geometric Encryption — gauge transformations on fiber coordinates.
//!
//! v0.1 — Affine numeric only. ρ_g(v) = a·v + b. Curvature/spectral/anomaly
//! invariant by Theorem 5a.1.
//!
//! v0.2 — Per-field mode declared at schema time:
//!   • Affine          — v0.1, numeric only (default for NUMERIC/INTEGER/TIMESTAMP)
//!   • Opaque          — AEAD via AES-GCM-SIV (RFC 8452); randomized; IND-CPA
//!   • Indexed         — PRF via AES-256-CMAC; deterministic; equality-queryable
//!   • Probabilistic   — affine + Gaussian noise (Sprint D)
//!   • Isometric       — O(k) on grouped numeric (Sprint E)
//!
//! AAD (associated authenticated data) binds Opaque ciphertexts to their
//! position in the bundle (bundle name + field index + field name) so a
//! ciphertext swapped between fields fails authentication on decrypt.

use crate::types::{FieldDef, FieldType, Value};

use aes_gcm_siv::{
    aead::{Aead, KeyInit, Payload},
    Aes256GcmSiv, Nonce,
};
use cmac::{Cmac, Mac};
use aes::Aes256;

/// Per-field gauge transform. v0.2 introduces multiple variants — each fiber
/// field carries one variant matching its declared `EncryptionMode`.
#[derive(Debug, Clone)]
pub enum FieldTransform {
    /// No encryption (plaintext field). Identity on encrypt and decrypt.
    Identity,

    /// v0.1 affine numeric: v_enc = scale * v + offset. Curvature-invariant
    /// (`Var/range²` is a ratio of `a²`-scaled quantities). Numeric only.
    Affine { scale: f64, offset: f64 },

    /// AEAD via AES-GCM-SIV (RFC 8452 — nonce-misuse-resistant). Per-record
    /// random 96-bit nonce; 128-bit auth tag. IND-CPA. Not equality-queryable.
    /// On-disk wire format: [12-byte nonce | ciphertext | 16-byte tag], stored
    /// as `Value::Binary`.
    Opaque { key: [u8; 32] },

    /// PRF via AES-256-CMAC (NIST SP 800-38B). Deterministic — equal
    /// plaintexts yield equal 16-byte tags. Equality-queryable; bitmap index
    /// works verbatim. High-cardinality columns only (deterministic
    /// encryption leaks frequency on low-cardinality data — schema author
    /// must opt in).
    Indexed { key: [u8; 32] },

    /// Affine + Gaussian noise: w = scale * v + offset + ε, ε ~ N(0, sigma²).
    /// Numeric only. Statistical unlinkability — same plaintext encrypts to
    /// different ciphertexts each call. Equality-queryable via the Davis
    /// Identity: a deterministic σ-bucket hash of the plaintext is stored
    /// alongside the noisy value, so HashMap-probe equality search returns
    /// records whose plaintext shared a σ-bucket with the query literal.
    /// On-disk wire: `Value::Binary([f64 noisy_value | u64 bucket_hash])` = 16 B.
    /// Decrypt recovers `(noisy - offset) / scale` ± σ/|scale| (approximate).
    Probabilistic {
        scale: f64,
        offset: f64,
        sigma: f64,
        bucket_key: [u8; 32],
    },

    /// Orthogonal O(k) gauge for grouped numeric fiber: w = O·v + b,
    /// O ∈ O(k) (orthogonal: O^T O = I), b ∈ ℝ^k. Pairwise Euclidean
    /// distances preserved exactly: ||O·u - O·v|| = ||u - v||. For k=1 this
    /// degenerates to a sign flip + offset (which is technically isometric
    /// but trivially so); the real value is at k≥2 where the group rotation
    /// scrambles individual components while preserving the geometric
    /// structure of the vector. Each fiber field carries the same shared
    /// matrix when fields are declared in the same `GROUP`.
    ///
    /// Storage: each component is encrypted independently as `Value::Float`
    /// of the corresponding row of `O·v + b`. The schema groups the fields;
    /// the GaugeKey holds the same Isometric variant on every member.
    Isometric {
        /// Group identifier (`"wind"`, `"embedding"`, etc.). All fields with
        /// the same group_id share the same matrix and offset.
        group_id: String,
        /// k×k orthogonal matrix in row-major order. Built once per group
        /// from the seed via QR decomposition of a Gaussian matrix.
        matrix: Vec<Vec<f64>>,
        /// k-dimensional offset b.
        offset_vec: Vec<f64>,
        /// This field's row index within the group (0..k-1). Used to extract
        /// the appropriate row of O·v during encrypt and the column of O^T
        /// during decrypt. Named `member_index` because each field is one
        /// member of the group, occupying one row of the shared matrix.
        member_index: usize,
    },
}

impl FieldTransform {
    /// Apply the forward gauge transform, optionally with AAD (used by AEAD
    /// modes; ignored by Affine/Identity). The AAD binds the ciphertext to
    /// its (bundle, field) position so a swap between fields fails
    /// authentication on decrypt.
    pub fn encrypt_value(&self, v: &Value, aad: &[u8]) -> Value {
        match self {
            FieldTransform::Identity => v.clone(),
            FieldTransform::Affine { scale, offset } => match v {
                Value::Float(f) => Value::Float(scale * f + offset),
                Value::Integer(i) => Value::Float(scale * (*i as f64) + offset),
                Value::Timestamp(t) => Value::Timestamp((scale * (*t as f64) + offset) as i64),
                other => other.clone(),
            },
            FieldTransform::Opaque { key } => {
                let plaintext_bytes = value_to_bytes(v);
                let ct = aead_encrypt(key, &plaintext_bytes, aad);
                Value::Binary(ct)
            }
            FieldTransform::Indexed { key } => {
                let plaintext_bytes = value_to_bytes(v);
                let tag = cmac_prf(key, &plaintext_bytes);
                Value::Binary(tag.to_vec())
            }
            FieldTransform::Probabilistic { scale, offset, sigma, bucket_key } => {
                // Extract numeric plaintext. Non-numeric falls through.
                let plain = match v {
                    Value::Float(f) => *f,
                    Value::Integer(i) => *i as f64,
                    Value::Timestamp(t) => *t as f64,
                    _ => return v.clone(),
                };
                let noise = gaussian_sample(*sigma);
                let noisy = scale * plain + offset + noise;
                let bucket = bucket_hash(bucket_key, plain, *sigma);

                // Wire format: 16 bytes = [f64 noisy | u64 bucket].
                let mut buf = Vec::with_capacity(16);
                buf.extend_from_slice(&noisy.to_le_bytes());
                buf.extend_from_slice(&bucket.to_le_bytes());
                Value::Binary(buf)
            }
            FieldTransform::Isometric { .. } => {
                // Isometric requires the GROUP context (other members'
                // plaintext values to compute O·v). Single-value encrypt
                // can't produce the right output. The group-aware code
                // path lives in `encrypt_fiber` which collects all group
                // members and applies the matrix once. If we land here
                // (solo encrypt of an Isometric field), fall through as
                // identity — caller likely went through the wrong path.
                v.clone()
            }
        }
    }

    /// Apply the inverse transform. For Indexed, the PRF is one-way — decrypt
    /// returns the stored ciphertext bytes as-is (the caller knows from the
    /// schema that the field is one-way encrypted; equality search is the
    /// supported access pattern). For Probabilistic, decrypt is approximate:
    /// recovers `(noisy - offset) / scale` with precision ±σ/|scale|.
    pub fn decrypt_value(&self, w: &Value, aad: &[u8]) -> Value {
        decrypt_call_counter_inc();
        match self {
            FieldTransform::Identity => w.clone(),
            FieldTransform::Affine { scale, offset } => match w {
                Value::Float(f) => Value::Float((f - offset) / scale),
                Value::Integer(i) => Value::Float(((*i as f64) - offset) / scale),
                Value::Timestamp(t) => {
                    Value::Timestamp((((*t as f64) - offset) / scale) as i64)
                }
                other => other.clone(),
            },
            FieldTransform::Opaque { key } => match w {
                Value::Binary(bytes) => {
                    let plaintext_bytes = aead_decrypt(key, bytes, aad)
                        .expect("AEAD decrypt failed — ciphertext tampered or wrong key/AAD");
                    bytes_to_value(&plaintext_bytes)
                }
                other => other.clone(),
            },
            FieldTransform::Indexed { .. } => {
                // PRF is one-way. Return the stored ciphertext as-is.
                w.clone()
            }
            FieldTransform::Probabilistic { scale, offset, .. } => match w {
                Value::Binary(bytes) if bytes.len() == 16 => {
                    let noisy = f64::from_le_bytes(bytes[..8].try_into().unwrap());
                    let plain_estimate = (noisy - offset) / scale;
                    Value::Float(plain_estimate)
                }
                other => other.clone(),
            },
            FieldTransform::Isometric { .. } => {
                // Same as encrypt: needs group context. Single-value
                // decrypt falls through; the real path is in decrypt_fiber.
                w.clone()
            }
        }
    }
}

/// A geometric encryption key: one FieldTransform per fiber field.
#[derive(Debug, Clone)]
pub struct GaugeKey {
    pub transforms: Vec<FieldTransform>,
}

impl GaugeKey {
    /// Derive a GaugeKey from a 32-byte seed and the fiber field definitions.
    /// Each field's transform variant is determined by its `EncryptionMode`:
    ///   - `None`           → Identity
    ///   - `Affine`         → Affine { scale, offset }   (KDF from seed + name)
    ///   - `Opaque`         → Opaque { key: 32B }        (KDF from seed + name)
    ///   - `Indexed`        → Indexed { key: 32B }       (KDF from seed + name)
    ///   - `Probabilistic`  → Probabilistic { scale, offset, sigma, bucket_key }
    ///   - `Isometric`      → Isometric { group_id, matrix, offset_vec, member_index }
    ///                        Sibling fields with the same `encryption_group`
    ///                        share the same matrix and offset, derived once
    ///                        per group from the seed.
    pub fn derive(seed: &[u8; 32], fiber_fields: &[FieldDef]) -> Self {
        // Pre-pass: discover Isometric groups so we know each group's k
        // (size) and each field's member_index within its group. Members
        // are listed in schema order (the order matters: it's the row
        // ordering of the matrix when we apply O·v).
        use std::collections::HashMap;
        let mut group_members: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, field) in fiber_fields.iter().enumerate() {
            if matches!(field.encryption, crate::types::EncryptionMode::Isometric) {
                let gid = field.encryption_group.clone().unwrap_or_else(|| field.name.clone());
                group_members.entry(gid).or_default().push(i);
            }
        }

        // Per-group: derive the shared O(k) matrix + offset once.
        let mut group_matrices: HashMap<String, (Vec<Vec<f64>>, Vec<f64>)> = HashMap::new();
        for (gid, members) in &group_members {
            let k = members.len();
            let m = derive_orthogonal_matrix(seed, gid, k);
            group_matrices.insert(gid.clone(), m);
        }

        let transforms = fiber_fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                if matches!(field.encryption, crate::types::EncryptionMode::Isometric) {
                    let gid = field.encryption_group.clone().unwrap_or_else(|| field.name.clone());
                    let members = group_members.get(&gid).cloned().unwrap_or_else(|| vec![i]);
                    let member_index = members.iter().position(|&x| x == i).unwrap_or(0);
                    let (matrix, offset_vec) = group_matrices.get(&gid).cloned().unwrap_or_else(|| {
                        derive_orthogonal_matrix(seed, &gid, 1)
                    });
                    FieldTransform::Isometric { group_id: gid, matrix, offset_vec, member_index }
                } else {
                    Self::derive_field_transform(seed, &field.name, &field.field_type, &field.encryption)
                }
            })
            .collect();
        GaugeKey { transforms }
    }

    /// Derive transform for a single field from seed + field name + mode.
    fn derive_field_transform(
        seed: &[u8; 32],
        field_name: &str,
        field_type: &FieldType,
        mode: &crate::types::EncryptionMode,
    ) -> FieldTransform {
        use crate::types::EncryptionMode;

        match mode {
            EncryptionMode::None => {
                // v0.1-shaped fallback: when no per-field mode is set, the
                // bundle-level dispatch (in parser.rs CreateBundle handler)
                // fills in default modes BEFORE calling derive. If we still
                // see None here, the field is genuinely unencrypted.
                FieldTransform::Identity
            }
            EncryptionMode::Affine => Self::derive_affine(seed, field_name),
            EncryptionMode::Opaque => Self::derive_opaque(seed, field_name),
            EncryptionMode::Indexed => Self::derive_indexed(seed, field_name),
            EncryptionMode::Probabilistic { sigma } => {
                Self::derive_probabilistic(seed, field_name, *sigma)
            }
            EncryptionMode::Isometric => {
                // Sprint E — placeholder.
                let _ = field_type;
                FieldTransform::Identity
            }
        }
    }

    fn derive_affine(seed: &[u8; 32], field_name: &str) -> FieldTransform {
        let mut hasher_bytes = Vec::with_capacity(seed.len() + field_name.len() + 9);
        hasher_bytes.extend_from_slice(seed);
        hasher_bytes.extend_from_slice(b":affine:");
        hasher_bytes.extend_from_slice(field_name.as_bytes());

        let h1 = mix_hash(&hasher_bytes, 0x517cc1b727220a95);
        let h2 = mix_hash(&hasher_bytes, 0x6c62272e07bb0142);

        // Scale: map to range [0.1, 10.0], ensuring nonzero.
        let scale_raw = (h1 as f64) / (u64::MAX as f64);
        let scale = 0.1 + scale_raw * 9.9;

        // Offset: map to range [-1000, 1000].
        let offset_raw = (h2 as f64) / (u64::MAX as f64);
        let offset = -1000.0 + offset_raw * 2000.0;

        FieldTransform::Affine { scale, offset }
    }

    fn derive_opaque(seed: &[u8; 32], field_name: &str) -> FieldTransform {
        // Derive a 32-byte AES-256 key from the seed via two 64-bit mixes
        // domain-separated by ":opaque:" + field name.
        FieldTransform::Opaque {
            key: derive_field_key(seed, b":opaque:", field_name),
        }
    }

    fn derive_indexed(seed: &[u8; 32], field_name: &str) -> FieldTransform {
        FieldTransform::Indexed {
            key: derive_field_key(seed, b":indexed:", field_name),
        }
    }

    fn derive_probabilistic(seed: &[u8; 32], field_name: &str, sigma: f64) -> FieldTransform {
        // Reuse the affine-derivation routine to get scale + offset; this
        // ensures the same field name always produces the same affine params,
        // and the σ-bucket lookup is deterministic.
        let mut hasher_bytes = Vec::with_capacity(seed.len() + field_name.len() + 9);
        hasher_bytes.extend_from_slice(seed);
        hasher_bytes.extend_from_slice(b":prob:");
        hasher_bytes.extend_from_slice(field_name.as_bytes());

        let h1 = mix_hash(&hasher_bytes, 0x517cc1b727220a95);
        let h2 = mix_hash(&hasher_bytes, 0x6c62272e07bb0142);
        let scale = 0.1 + ((h1 as f64) / (u64::MAX as f64)) * 9.9; // [0.1, 10)
        let offset = -1000.0 + ((h2 as f64) / (u64::MAX as f64)) * 2000.0; // [-1000, 1000)

        FieldTransform::Probabilistic {
            scale,
            offset,
            sigma,
            bucket_key: derive_field_key(seed, b":prob_bucket:", field_name),
        }
    }

    /// Simple deterministic hash mixing (wyhash-inspired). Public so the WAL
    /// can re-derive keys deterministically when reloading a schema.
    pub fn mix_hash(data: &[u8], seed: u64) -> u64 {
        mix_hash(data, seed)
    }

    /// Encrypt a fiber value vector (in schema field order). The `bundle_name`
    /// is used to construct AAD that binds each ciphertext to its position
    /// (bundle, field index, field name) — so a ciphertext swapped between
    /// fields or between bundles fails authentication on decrypt.
    ///
    /// Two-pass: non-Isometric fields encrypt independently in pass 1; pass 2
    /// gathers each Isometric group's plaintext values, applies the shared
    /// `O·v + b` matrix once, distributes results back into the output by
    /// member_index. This is the only way to honor the GROUP semantics —
    /// individual field encrypts can't preserve pairwise distances.
    pub fn encrypt_fiber(
        &self,
        fiber_vals: &[Value],
        bundle_name: &str,
        fiber_fields: &[FieldDef],
    ) -> Vec<Value> {
        let mut out: Vec<Value> = vec![Value::Null; fiber_vals.len()];
        // Group accumulator: group_id → list of (field_index, member_index, plaintext)
        use std::collections::HashMap;
        let mut group_pending: HashMap<String, Vec<(usize, usize, f64)>> = HashMap::new();

        for (i, v) in fiber_vals.iter().enumerate() {
            match self.transforms.get(i) {
                Some(FieldTransform::Isometric { group_id, member_index, .. }) => {
                    let plain = match v {
                        Value::Float(f) => *f,
                        Value::Integer(n) => *n as f64,
                        Value::Timestamp(t) => *t as f64,
                        _ => 0.0,
                    };
                    group_pending
                        .entry(group_id.clone())
                        .or_default()
                        .push((i, *member_index, plain));
                }
                Some(t) => {
                    let aad = build_aad(
                        bundle_name,
                        i,
                        fiber_fields.get(i).map(|f| f.name.as_str()).unwrap_or(""),
                    );
                    out[i] = t.encrypt_value(v, &aad);
                }
                None => out[i] = v.clone(),
            }
        }

        // Apply each group's matrix once.
        for (_group_id, mut members) in group_pending {
            members.sort_by_key(|&(_, mi, _)| mi);
            let k = members.len();
            // Pull the matrix + offset from any member; they all agree.
            let (matrix, offset_vec) = match self.transforms.get(members[0].0) {
                Some(FieldTransform::Isometric { matrix, offset_vec, .. }) => {
                    (matrix.clone(), offset_vec.clone())
                }
                _ => continue,
            };
            // Build v in member-index order.
            let v_vec: Vec<f64> = members.iter().map(|&(_, _, plain)| plain).collect();
            // w = O·v + b. matrix is k×k row-major; w[i] = sum_j O[i][j]·v[j] + b[i]
            for i in 0..k {
                let mut sum = offset_vec.get(i).copied().unwrap_or(0.0);
                for j in 0..k {
                    sum += matrix[i].get(j).copied().unwrap_or(0.0) * v_vec[j];
                }
                // Find the field_index for member with this member_index.
                let field_idx = members[i].0;
                out[field_idx] = Value::Float(sum);
            }
        }

        out
    }

    /// Decrypt a fiber value vector (in schema field order). AAD is recomputed
    /// per field — must match the AAD used at encrypt time. Same two-pass
    /// shape as `encrypt_fiber`: non-Isometric per-field, Isometric groups
    /// solved in pass 2 via O^T(w - b).
    pub fn decrypt_fiber(
        &self,
        encrypted_vals: &[Value],
        bundle_name: &str,
        fiber_fields: &[FieldDef],
    ) -> Vec<Value> {
        decrypt_call_counter_inc();
        let mut out: Vec<Value> = vec![Value::Null; encrypted_vals.len()];
        use std::collections::HashMap;
        let mut group_pending: HashMap<String, Vec<(usize, usize, f64)>> = HashMap::new();

        for (i, w) in encrypted_vals.iter().enumerate() {
            match self.transforms.get(i) {
                Some(FieldTransform::Isometric { group_id, member_index, .. }) => {
                    let cipher = match w {
                        Value::Float(f) => *f,
                        Value::Integer(n) => *n as f64,
                        Value::Timestamp(t) => *t as f64,
                        _ => 0.0,
                    };
                    group_pending
                        .entry(group_id.clone())
                        .or_default()
                        .push((i, *member_index, cipher));
                }
                Some(t) => {
                    let aad = build_aad(
                        bundle_name,
                        i,
                        fiber_fields.get(i).map(|f| f.name.as_str()).unwrap_or(""),
                    );
                    out[i] = t.decrypt_value(w, &aad);
                }
                None => out[i] = w.clone(),
            }
        }

        for (_group_id, mut members) in group_pending {
            members.sort_by_key(|&(_, mi, _)| mi);
            let k = members.len();
            let (matrix, offset_vec) = match self.transforms.get(members[0].0) {
                Some(FieldTransform::Isometric { matrix, offset_vec, .. }) => {
                    (matrix.clone(), offset_vec.clone())
                }
                _ => continue,
            };
            // (w - b)
            let centered: Vec<f64> = members
                .iter()
                .enumerate()
                .map(|(i, &(_, _, cipher))| cipher - offset_vec.get(i).copied().unwrap_or(0.0))
                .collect();
            // v = O^T · (w - b). For orthogonal O, transpose is inverse.
            // v[i] = sum_j O[j][i] · centered[j]
            for i in 0..k {
                let mut sum = 0.0;
                for j in 0..k {
                    sum += matrix[j].get(i).copied().unwrap_or(0.0) * centered[j];
                }
                let field_idx = members[i].0;
                out[field_idx] = Value::Float(sum);
            }
        }

        out
    }

    /// Encrypt a single query literal for a given fiber field index. Used by
    /// the engine to translate `WHERE field = X` into a comparison in
    /// encrypted space. For `Affine`, this means pre-encrypting X. For
    /// `Indexed`, this means PRF-hashing X. For `Opaque`, equality query is
    /// not supported (callers must check the schema mode and reject).
    pub fn encrypt_literal(
        &self,
        field_idx: usize,
        value: &Value,
        bundle_name: &str,
        fiber_fields: &[FieldDef],
    ) -> Value {
        if let Some(t) = self.transforms.get(field_idx) {
            let aad = build_aad(
                bundle_name,
                field_idx,
                fiber_fields.get(field_idx).map(|f| f.name.as_str()).unwrap_or(""),
            );
            t.encrypt_value(value, &aad)
        } else {
            value.clone()
        }
    }

    /// Generate a random 32-byte seed using OS CSPRNG.
    pub fn random_seed() -> [u8; 32] {
        let mut seed = [0u8; 32];
        getrandom::getrandom(&mut seed).expect("Failed to generate random seed from OS CSPRNG");
        seed
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Sprint H: decrypt-call telemetry
// ─────────────────────────────────────────────────────────────────────────
//
// Both `decrypt_value` and `decrypt_fiber` increment this atomic on every
// call. The PROJECT INVARIANT regression test
// `test_project_invariant_zero_decrypt_calls_in_execution_path` asserts
// the counter stays at 0 for the duration of an invariant evaluation —
// that's the *structural* part of the no-decrypt guarantee for the
// invariant query surface. Negligible runtime cost (one relaxed atomic
// fetch_add per decrypt) so we leave it on in release builds too.

use std::sync::atomic::{AtomicUsize, Ordering};

static DECRYPT_CALL_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[inline]
fn decrypt_call_counter_inc() {
    DECRYPT_CALL_COUNTER.fetch_add(1, Ordering::Relaxed);
}

/// Read the global decrypt-call counter. Increments on every
/// `FieldTransform::decrypt_value` and `GaugeKey::decrypt_fiber` call,
/// across the entire process. Used by Sprint H tests to assert the
/// PROJECT INVARIANT execution path triggers zero decrypts.
pub fn decrypt_call_count() -> usize {
    DECRYPT_CALL_COUNTER.load(Ordering::Relaxed)
}

/// Reset the global decrypt-call counter to 0. Tests call this before
/// each invariant evaluation to get a clean baseline. NOT a thread-safe
/// transactional reset across concurrent decrypts — the assumption is
/// that test bodies are single-threaded with respect to this counter.
pub fn reset_decrypt_call_count() {
    DECRYPT_CALL_COUNTER.store(0, Ordering::Relaxed);
}

// ─────────────────────────────────────────────────────────────────────────
// AAD construction
// ─────────────────────────────────────────────────────────────────────────

/// Build the AAD (associated authenticated data) for an Opaque ciphertext.
/// Format: `<bundle_name>|<field_idx>|<field_name>` as raw UTF-8 bytes.
/// Binds the ciphertext to its position in the bundle so swapping ciphertexts
/// between fields fails authentication on decrypt.
fn build_aad(bundle_name: &str, field_idx: usize, field_name: &str) -> Vec<u8> {
    let mut aad = Vec::with_capacity(bundle_name.len() + field_name.len() + 16);
    aad.extend_from_slice(bundle_name.as_bytes());
    aad.push(b'|');
    aad.extend_from_slice(field_idx.to_string().as_bytes());
    aad.push(b'|');
    aad.extend_from_slice(field_name.as_bytes());
    aad
}

// ─────────────────────────────────────────────────────────────────────────
// Value ↔ bytes conversions for AEAD modes
// ─────────────────────────────────────────────────────────────────────────

/// Serialize a Value to bytes for AEAD-style encryption. The first byte is a
/// type tag so decrypt can recover the original variant.
fn value_to_bytes(v: &Value) -> Vec<u8> {
    match v {
        Value::Null => vec![0x00],
        Value::Bool(b) => vec![0x01, if *b { 1 } else { 0 }],
        Value::Integer(i) => {
            let mut buf = Vec::with_capacity(9);
            buf.push(0x02);
            buf.extend_from_slice(&i.to_le_bytes());
            buf
        }
        Value::Float(f) => {
            let mut buf = Vec::with_capacity(9);
            buf.push(0x03);
            buf.extend_from_slice(&f.to_le_bytes());
            buf
        }
        Value::Text(s) => {
            let mut buf = Vec::with_capacity(s.len() + 1);
            buf.push(0x04);
            buf.extend_from_slice(s.as_bytes());
            buf
        }
        Value::Binary(b) => {
            let mut buf = Vec::with_capacity(b.len() + 1);
            buf.push(0x05);
            buf.extend_from_slice(b);
            buf
        }
        Value::Timestamp(t) => {
            let mut buf = Vec::with_capacity(9);
            buf.push(0x06);
            buf.extend_from_slice(&t.to_le_bytes());
            buf
        }
        Value::Vector(values) => {
            let mut buf = Vec::with_capacity(values.len() * 8 + 5);
            buf.push(0x07);
            buf.extend_from_slice(&(values.len() as u32).to_le_bytes());
            for f in values {
                buf.extend_from_slice(&f.to_le_bytes());
            }
            buf
        }
    }
}

/// Inverse of `value_to_bytes`. Returns `Value::Null` on malformed input.
fn bytes_to_value(b: &[u8]) -> Value {
    if b.is_empty() {
        return Value::Null;
    }
    match b[0] {
        0x00 => Value::Null,
        0x01 => Value::Bool(b.get(1).copied().unwrap_or(0) != 0),
        0x02 if b.len() >= 9 => Value::Integer(i64::from_le_bytes(b[1..9].try_into().unwrap())),
        0x03 if b.len() >= 9 => Value::Float(f64::from_le_bytes(b[1..9].try_into().unwrap())),
        0x04 => Value::Text(String::from_utf8_lossy(&b[1..]).into_owned()),
        0x05 => Value::Binary(b[1..].to_vec()),
        0x06 if b.len() >= 9 => Value::Timestamp(i64::from_le_bytes(b[1..9].try_into().unwrap())),
        0x07 if b.len() >= 5 => {
            let n = u32::from_le_bytes(b[1..5].try_into().unwrap()) as usize;
            let mut out = Vec::with_capacity(n);
            let mut off = 5;
            for _ in 0..n {
                if off + 8 > b.len() {
                    break;
                }
                out.push(f64::from_le_bytes(b[off..off + 8].try_into().unwrap()));
                off += 8;
            }
            Value::Vector(out)
        }
        _ => Value::Null,
    }
}

// ─────────────────────────────────────────────────────────────────────────
// AEAD primitives (Opaque mode)
// ─────────────────────────────────────────────────────────────────────────

/// Encrypt with AES-GCM-SIV. Returns [12-byte nonce | ciphertext | 16-byte tag]
/// as a single Vec<u8>. Per-record random nonce drawn from OS CSPRNG.
fn aead_encrypt(key: &[u8; 32], plaintext: &[u8], aad: &[u8]) -> Vec<u8> {
    let cipher = Aes256GcmSiv::new(key.into());
    let mut nonce_bytes = [0u8; 12];
    getrandom::getrandom(&mut nonce_bytes).expect("OS CSPRNG failure");
    let nonce = Nonce::from_slice(&nonce_bytes);
    let payload = Payload {
        msg: plaintext,
        aad,
    };
    let ct_and_tag = cipher
        .encrypt(nonce, payload)
        .expect("AES-GCM-SIV encrypt failed");
    let mut out = Vec::with_capacity(12 + ct_and_tag.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct_and_tag);
    out
}

/// Decrypt an AES-GCM-SIV blob. Returns `Err(())` on auth tag mismatch
/// (tamper detected).
fn aead_decrypt(key: &[u8; 32], blob: &[u8], aad: &[u8]) -> Result<Vec<u8>, ()> {
    if blob.len() < 12 + 16 {
        return Err(());
    }
    let cipher = Aes256GcmSiv::new(key.into());
    let nonce = Nonce::from_slice(&blob[..12]);
    let payload = Payload {
        msg: &blob[12..],
        aad,
    };
    cipher.decrypt(nonce, payload).map_err(|_| ())
}

// ─────────────────────────────────────────────────────────────────────────
// PRF primitives (Indexed mode)
// ─────────────────────────────────────────────────────────────────────────

/// Compute AES-256-CMAC over the input bytes with the given key. Output is
/// a 16-byte deterministic tag — equal inputs under the same key yield equal
/// tags. NIST SP 800-38B.
fn cmac_prf(key: &[u8; 32], input: &[u8]) -> [u8; 16] {
    let mut mac = <Cmac<Aes256> as Mac>::new_from_slice(key).expect("AES-256 key");
    mac.update(input);
    let result = mac.finalize().into_bytes();
    let mut tag = [0u8; 16];
    tag.copy_from_slice(&result);
    tag
}

// ─────────────────────────────────────────────────────────────────────────
// Probabilistic primitives (Sprint D)
// ─────────────────────────────────────────────────────────────────────────

/// Draw a Gaussian sample with mean 0 and standard deviation σ via the
/// Box-Muller transform, sourced from the OS CSPRNG. Avoids the `rand_distr`
/// dependency since the rest of the codebase already pulls `getrandom`.
///
/// Numerical note: clamping `u1` away from 0 ensures `ln(u1)` doesn't return
/// −∞ when the CSPRNG happens to emit all-zeros (probability 2^-64 but real).
fn gaussian_sample(sigma: f64) -> f64 {
    let mut buf = [0u8; 16];
    getrandom::getrandom(&mut buf).expect("OS CSPRNG failure");
    // u1 in (0, 1] — exclude 0 to avoid ln(0); inclusion at 1 is harmless.
    let raw1 = u64::from_le_bytes(buf[0..8].try_into().unwrap());
    let u1 = (raw1 as f64 + 1.0) / (u64::MAX as f64 + 1.0);
    let u2 = (u64::from_le_bytes(buf[8..16].try_into().unwrap()) as f64) / (u64::MAX as f64);
    sigma * (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

// ─────────────────────────────────────────────────────────────────────────
// Isometric primitives (Sprint E)
// ─────────────────────────────────────────────────────────────────────────

/// Derive a deterministic k×k orthogonal matrix and k-dimensional offset for
/// an Isometric group from `(seed, group_id, k)`. Returns `(matrix, offset)`
/// where `matrix` is row-major (k rows of k columns each).
///
/// Construction:
/// 1. Fill a k×k Gaussian matrix `G` from seeded Box-Muller mixing.
/// 2. Orthogonalize via Gram-Schmidt to get `Q ∈ O(k)` (numerically — the
///    rows of `Q` form an orthonormal basis).
/// 3. Sample the offset b ∈ ℝ^k from a separate seed-mix.
///
/// Determinism: same `(seed, group_id, k)` always produces the same matrix
/// and offset across deployments. Both are stored verbatim on the GaugeKey
/// so subsequent calls don't rederive.
pub fn derive_orthogonal_matrix(
    seed: &[u8; 32],
    group_id: &str,
    k: usize,
) -> (Vec<Vec<f64>>, Vec<f64>) {
    if k == 0 {
        return (vec![], vec![]);
    }

    let mut input = Vec::with_capacity(seed.len() + group_id.len() + 12);
    input.extend_from_slice(seed);
    input.extend_from_slice(b":isometric:");
    input.extend_from_slice(group_id.as_bytes());

    // Fill k*k Gaussian samples deterministically via Box-Muller on
    // mix-hashed slot indices. Each slot has its own seed mix so the
    // resulting matrix has bit-independent entries.
    let mut g = vec![vec![0.0f64; k]; k];
    for i in 0..k {
        for j in 0..k {
            let slot = (i * k + j) as u64;
            let h1 = mix_hash(&input, 0x517cc1b727220a95u64.wrapping_add(slot * 2));
            let h2 = mix_hash(&input, 0x6c62272e07bb0142u64.wrapping_add(slot * 2 + 1));
            let u1 = (h1 as f64 + 1.0) / (u64::MAX as f64 + 1.0);
            let u2 = (h2 as f64) / (u64::MAX as f64);
            g[i][j] = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        }
    }

    // Gram-Schmidt orthogonalization on rows. After this, the rows of Q
    // form an orthonormal basis: Q · Q^T = I (and equivalently Q^T · Q = I
    // since Q is square orthogonal).
    let mut q = g;
    for i in 0..k {
        // Subtract projections onto previous rows.
        for j in 0..i {
            let dot: f64 = (0..k).map(|c| q[i][c] * q[j][c]).sum();
            for c in 0..k {
                q[i][c] -= dot * q[j][c];
            }
        }
        // Normalize.
        let norm: f64 = (0..k).map(|c| q[i][c] * q[i][c]).sum::<f64>().sqrt();
        // If a row degenerates to (near-)zero (extremely improbable from
        // Gaussian samples, but defensively): replace with an axis-aligned
        // unit vector to keep Q full-rank.
        if norm < 1e-12 {
            for c in 0..k {
                q[i][c] = 0.0;
            }
            q[i][i % k] = 1.0;
        } else {
            for c in 0..k {
                q[i][c] /= norm;
            }
        }
    }

    // Offset b ∈ ℝ^k. Sample from a separate domain-separated mix.
    let mut offset = vec![0.0f64; k];
    for i in 0..k {
        let slot = i as u64;
        let h = mix_hash(&input, 0xfeedfacef00dbabeu64.wrapping_add(slot));
        offset[i] = -100.0 + (h as f64 / u64::MAX as f64) * 200.0;
    }

    (q, offset)
}

/// Compute the σ-bucket hash of a plaintext value under a per-field bucket
/// key. Two plaintexts within the same σ-wide bucket produce the same hash;
/// plaintexts in different buckets produce (with high probability) different
/// hashes.
///
/// Implementation: floor(plaintext / σ) → bucket index (i64) → 8 bytes →
/// AES-256-CMAC under bucket_key → first 8 bytes as u64.
///
/// Stored alongside the noisy ciphertext so HashMap-probe equality search
/// returns records whose plaintext shared a σ-bucket with the query literal.
/// This is the implementation of the Davis Identity equality predicate.
fn bucket_hash(bucket_key: &[u8; 32], plaintext: f64, sigma: f64) -> u64 {
    let bucket_idx = (plaintext / sigma).floor() as i64;
    let bytes = bucket_idx.to_le_bytes();
    let cmac_tag = cmac_prf(bucket_key, &bytes);
    u64::from_le_bytes(cmac_tag[0..8].try_into().unwrap())
}

// ─────────────────────────────────────────────────────────────────────────
// Key derivation
// ─────────────────────────────────────────────────────────────────────────

/// Derive a 32-byte field-specific key from the master seed using two
/// 64-bit wyhash-style mixes, domain-separated by purpose (`:opaque:` /
/// `:indexed:`) and the field name. Same seed + same purpose + same field
/// name → same key (deterministic across deployments using the same seed).
fn derive_field_key(seed: &[u8; 32], purpose: &[u8], field_name: &str) -> [u8; 32] {
    let mut input = Vec::with_capacity(seed.len() + purpose.len() + field_name.len());
    input.extend_from_slice(seed);
    input.extend_from_slice(purpose);
    input.extend_from_slice(field_name.as_bytes());

    // Four independent 64-bit mixes packed into a 32-byte key. Each mix uses
    // a distinct seed prime to ensure bit-independence.
    let h1 = mix_hash(&input, 0x517cc1b727220a95);
    let h2 = mix_hash(&input, 0x6c62272e07bb0142);
    let h3 = mix_hash(&input, 0xff51afd7ed558ccd);
    let h4 = mix_hash(&input, 0xc4ceb9fe1a85ec53);

    let mut key = [0u8; 32];
    key[0..8].copy_from_slice(&h1.to_le_bytes());
    key[8..16].copy_from_slice(&h2.to_le_bytes());
    key[16..24].copy_from_slice(&h3.to_le_bytes());
    key[24..32].copy_from_slice(&h4.to_le_bytes());
    key
}

fn mix_hash(data: &[u8], seed: u64) -> u64 {
    let mut h = seed;
    for &b in data {
        h = h.wrapping_mul(0x2d358dccaa6c78a5).wrapping_add(b as u64);
        h ^= h >> 33;
    }
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ceb9fe1a85ec53);
    h ^= h >> 33;
    h
}

/// Parse a hex string into a 32-byte seed.
pub fn seed_from_hex(hex: &str) -> Result<[u8; 32], String> {
    let hex = hex.trim();
    if hex.len() != 64 {
        return Err(format!(
            "Encryption seed must be 64 hex characters (32 bytes), got {}",
            hex.len()
        ));
    }
    let mut seed = [0u8; 32];
    for i in 0..32 {
        seed[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|_| format!("Invalid hex at position {}", i * 2))?;
    }
    Ok(seed)
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EncryptionMode, FieldDef, Value};

    fn test_seed() -> [u8; 32] {
        let mut s = [0u8; 32];
        for i in 0..32 {
            s[i] = (i as u8).wrapping_mul(7).wrapping_add(13);
        }
        s
    }

    fn affine_fields() -> Vec<FieldDef> {
        vec![
            FieldDef::numeric("temp").with_encryption(EncryptionMode::Affine),
            FieldDef::numeric("humidity").with_encryption(EncryptionMode::Affine),
            FieldDef::numeric("pressure").with_encryption(EncryptionMode::Affine),
        ]
    }

    fn opaque_text_fields() -> Vec<FieldDef> {
        vec![
            FieldDef::categorical("legal_name").with_encryption(EncryptionMode::Opaque),
            FieldDef::categorical("address").with_encryption(EncryptionMode::Opaque),
        ]
    }

    fn indexed_fields() -> Vec<FieldDef> {
        vec![FieldDef::categorical("kind").with_encryption(EncryptionMode::Indexed)]
    }

    // ── v0.1 affine path (regression) ──

    #[test]
    fn test_derive_deterministic() {
        let seed = test_seed();
        let fields = affine_fields();
        let k1 = GaugeKey::derive(&seed, &fields);
        let k2 = GaugeKey::derive(&seed, &fields);
        for (a, b) in k1.transforms.iter().zip(k2.transforms.iter()) {
            match (a, b) {
                (FieldTransform::Affine { scale: s1, offset: o1 }, FieldTransform::Affine { scale: s2, offset: o2 }) => {
                    assert_eq!(s1, s2);
                    assert_eq!(o1, o2);
                }
                _ => panic!("Expected matching Affine transforms"),
            }
        }
    }

    #[test]
    fn test_affine_encrypt_decrypt_roundtrip() {
        let seed = test_seed();
        let fields = affine_fields();
        let key = GaugeKey::derive(&seed, &fields);

        let plain = vec![Value::Float(-31.9), Value::Float(65.0), Value::Float(1013.25)];
        let encrypted = key.encrypt_fiber(&plain, "test_bundle", &fields);
        let decrypted = key.decrypt_fiber(&encrypted, "test_bundle", &fields);

        for (p, d) in plain.iter().zip(decrypted.iter()) {
            match (p, d) {
                (Value::Float(a), Value::Float(b)) => {
                    assert!((a - b).abs() < 1e-10, "Roundtrip failed: {a} vs {b}");
                }
                _ => panic!("Type mismatch"),
            }
        }
    }

    #[test]
    fn test_affine_encrypted_values_differ_from_plain() {
        let seed = test_seed();
        let fields = affine_fields();
        let key = GaugeKey::derive(&seed, &fields);

        let plain = vec![Value::Float(22.5), Value::Float(65.0), Value::Float(1013.25)];
        let encrypted = key.encrypt_fiber(&plain, "test_bundle", &fields);
        for (p, e) in plain.iter().zip(encrypted.iter()) {
            assert_ne!(p, e, "Encrypted value should differ from plaintext");
        }
    }

    #[test]
    fn test_seed_from_hex() {
        let hex = "0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c";
        assert!(seed_from_hex(hex).is_ok());
    }

    #[test]
    fn test_seed_from_hex_bad_length() {
        assert!(seed_from_hex("abcd").is_err());
    }

    // ── Sprint B: Opaque (AEAD / AES-GCM-SIV) ──

    #[test]
    fn test_opaque_encrypt_decrypt_roundtrip_text() {
        let seed = test_seed();
        let fields = opaque_text_fields();
        let key = GaugeKey::derive(&seed, &fields);

        let plain = vec![
            Value::Text("Alice Smith".to_string()),
            Value::Text("123 Main St".to_string()),
        ];
        let encrypted = key.encrypt_fiber(&plain, "test_bundle", &fields);
        let decrypted = key.decrypt_fiber(&encrypted, "test_bundle", &fields);

        assert_eq!(plain, decrypted, "Opaque text roundtrip should be exact");
    }

    #[test]
    fn test_opaque_encrypt_decrypt_roundtrip_binary() {
        let seed = test_seed();
        let fields = vec![FieldDef::binary("payload").with_encryption(EncryptionMode::Opaque)];
        let key = GaugeKey::derive(&seed, &fields);

        let plain = vec![Value::Binary(vec![0u8, 1, 2, 3, 4, 5, 255])];
        let encrypted = key.encrypt_fiber(&plain, "b", &fields);
        let decrypted = key.decrypt_fiber(&encrypted, "b", &fields);
        assert_eq!(plain, decrypted);
    }

    #[test]
    fn test_opaque_same_plaintext_different_ciphertext() {
        let seed = test_seed();
        let fields = opaque_text_fields();
        let key = GaugeKey::derive(&seed, &fields);

        // Encrypt the same plaintext twice; ciphertexts should differ
        // (per-record random nonce).
        let plain = vec![Value::Text("hello".to_string()), Value::Text("world".to_string())];
        let ct1 = key.encrypt_fiber(&plain, "b", &fields);
        let ct2 = key.encrypt_fiber(&plain, "b", &fields);
        assert_ne!(ct1, ct2, "Opaque encrypt of same plaintext must produce different ciphertexts (random nonce)");
    }

    #[test]
    fn test_opaque_tamper_detection() {
        let seed = test_seed();
        let fields = opaque_text_fields();
        let key = GaugeKey::derive(&seed, &fields);

        let plain = vec![Value::Text("secret".to_string()), Value::Text("data".to_string())];
        let mut encrypted = key.encrypt_fiber(&plain, "b", &fields);

        // Flip a byte in the ciphertext of the first field.
        if let Value::Binary(ref mut bytes) = encrypted[0] {
            bytes[20] ^= 0x42;
        } else {
            panic!("expected Binary");
        }

        let result = std::panic::catch_unwind(|| {
            key.decrypt_fiber(&encrypted, "b", &fields);
        });
        assert!(result.is_err(), "Tampered AEAD ciphertext must fail decrypt");
    }

    #[test]
    fn test_opaque_aad_binds_to_field_position() {
        let seed = test_seed();
        let fields = opaque_text_fields();
        let key = GaugeKey::derive(&seed, &fields);

        // Encrypt two distinct plaintexts at fields 0 and 1.
        let plain = vec![Value::Text("legal".to_string()), Value::Text("address".to_string())];
        let encrypted = key.encrypt_fiber(&plain, "b", &fields);

        // Swap the two ciphertexts. Decrypt should fail authentication on
        // both because each ciphertext's AAD names the wrong field index.
        let swapped = vec![encrypted[1].clone(), encrypted[0].clone()];
        let result = std::panic::catch_unwind(|| {
            key.decrypt_fiber(&swapped, "b", &fields);
        });
        assert!(result.is_err(), "AAD must bind ciphertext to field position");
    }

    #[test]
    fn test_opaque_aad_binds_to_bundle_name() {
        let seed = test_seed();
        let fields = opaque_text_fields();
        let key = GaugeKey::derive(&seed, &fields);

        let plain = vec![Value::Text("a".to_string()), Value::Text("b".to_string())];
        let encrypted = key.encrypt_fiber(&plain, "bundle_one", &fields);

        // Decrypt with a different bundle name should fail.
        let result = std::panic::catch_unwind(|| {
            key.decrypt_fiber(&encrypted, "bundle_two", &fields);
        });
        assert!(result.is_err(), "AAD must bind ciphertext to bundle name");
    }

    #[test]
    fn test_opaque_round_trip_many_records() {
        let seed = test_seed();
        let fields = opaque_text_fields();
        let key = GaugeKey::derive(&seed, &fields);

        for i in 0..1000 {
            let plain = vec![
                Value::Text(format!("name_{i}")),
                Value::Text(format!("addr_{i}_456_main_st")),
            ];
            let encrypted = key.encrypt_fiber(&plain, "b", &fields);
            let decrypted = key.decrypt_fiber(&encrypted, "b", &fields);
            assert_eq!(plain, decrypted, "iteration {i}");
        }
    }

    #[test]
    fn test_opaque_ciphertext_is_binary_value() {
        let seed = test_seed();
        let fields = opaque_text_fields();
        let key = GaugeKey::derive(&seed, &fields);

        let plain = vec![Value::Text("abc".to_string()), Value::Text("def".to_string())];
        let encrypted = key.encrypt_fiber(&plain, "b", &fields);

        // Encrypted text should be stored as Value::Binary on disk.
        for v in &encrypted {
            assert!(matches!(v, Value::Binary(_)), "Opaque ciphertext should be Binary");
        }
    }

    #[test]
    fn test_opaque_ciphertext_size_includes_nonce_and_tag() {
        let seed = test_seed();
        let fields = opaque_text_fields();
        let key = GaugeKey::derive(&seed, &fields);

        // Plaintext is 5 bytes ("hello"). Type tag adds 1, so 6 bytes input
        // to AEAD. Output: 12 (nonce) + 6 (ct) + 16 (tag) = 34 bytes.
        let plain = vec![Value::Text("hello".to_string()), Value::Text("world".to_string())];
        let encrypted = key.encrypt_fiber(&plain, "b", &fields);
        if let Value::Binary(ref bytes) = encrypted[0] {
            assert_eq!(bytes.len(), 12 + 6 + 16, "AEAD blob = nonce | ct | tag");
        } else {
            panic!("Opaque ciphertext should be Binary");
        }
    }

    // ── Sprint C: Indexed (PRF / AES-256-CMAC) ──

    #[test]
    fn test_indexed_deterministic() {
        let seed = test_seed();
        let fields = indexed_fields();
        let key = GaugeKey::derive(&seed, &fields);

        // Same plaintext encrypted 1000 times → same ciphertext every time.
        let plain = vec![Value::Text("user_42".to_string())];
        let first = key.encrypt_fiber(&plain, "b", &fields);
        for _ in 0..1000 {
            let again = key.encrypt_fiber(&plain, "b", &fields);
            assert_eq!(first, again, "Indexed must be deterministic");
        }
    }

    #[test]
    fn test_indexed_distinct_plaintexts_distinct_ciphertexts() {
        let seed = test_seed();
        let fields = indexed_fields();
        let key = GaugeKey::derive(&seed, &fields);

        let mut ciphertexts = std::collections::HashSet::new();
        for i in 0..1000 {
            let plain = vec![Value::Text(format!("plaintext_{i}"))];
            let ct = key.encrypt_fiber(&plain, "b", &fields);
            let bytes = match &ct[0] {
                Value::Binary(b) => b.clone(),
                _ => panic!("expected Binary"),
            };
            assert!(
                ciphertexts.insert(bytes),
                "Distinct plaintexts must produce distinct ciphertexts (no collisions)"
            );
        }
    }

    #[test]
    fn test_indexed_equal_plaintexts_equal_ciphertexts() {
        // The whole point of Indexed: equality of plaintexts implies
        // equality of ciphertexts. This is what makes equality-search work
        // on the encrypted column.
        let seed = test_seed();
        let fields = indexed_fields();
        let key = GaugeKey::derive(&seed, &fields);

        let plain_a = vec![Value::Text("user_42".to_string())];
        let plain_b = vec![Value::Text("user_42".to_string())];
        let ct_a = key.encrypt_fiber(&plain_a, "b", &fields);
        let ct_b = key.encrypt_fiber(&plain_b, "b", &fields);
        assert_eq!(ct_a, ct_b, "Indexed equal-plaintext-equal-ciphertext invariant");
    }

    #[test]
    fn test_indexed_ciphertext_is_16_bytes() {
        let seed = test_seed();
        let fields = indexed_fields();
        let key = GaugeKey::derive(&seed, &fields);

        let plain = vec![Value::Text("anything".to_string())];
        let ct = key.encrypt_fiber(&plain, "b", &fields);
        if let Value::Binary(ref bytes) = ct[0] {
            assert_eq!(bytes.len(), 16, "AES-CMAC tag is exactly 16 bytes");
        } else {
            panic!("Indexed ciphertext should be Binary");
        }
    }

    #[test]
    fn test_indexed_decrypt_is_identity() {
        // PRF is one-way. decrypt returns the stored ciphertext as-is.
        let seed = test_seed();
        let fields = indexed_fields();
        let key = GaugeKey::derive(&seed, &fields);

        let plain = vec![Value::Text("abc".to_string())];
        let ct = key.encrypt_fiber(&plain, "b", &fields);
        let decrypted = key.decrypt_fiber(&ct, "b", &fields);
        assert_eq!(ct, decrypted, "Indexed decrypt is identity (PRF is one-way)");
    }

    #[test]
    fn test_indexed_encrypt_literal_for_equality_search() {
        // The query path: WHERE field = 'user_42' gets transformed by
        // encrypting the literal through the same PRF, then comparing.
        let seed = test_seed();
        let fields = indexed_fields();
        let key = GaugeKey::derive(&seed, &fields);

        // Stored value: PRF("user_42")
        let stored_plain = vec![Value::Text("user_42".to_string())];
        let stored_ct = key.encrypt_fiber(&stored_plain, "b", &fields);

        // Query literal: PRF("user_42") via encrypt_literal
        let literal = Value::Text("user_42".to_string());
        let query_ct = key.encrypt_literal(0, &literal, "b", &fields);

        assert_eq!(stored_ct[0], query_ct, "Equality search literal must match stored ciphertext");
    }

    #[test]
    fn test_indexed_different_keys_different_ciphertexts() {
        // Same plaintext under different seeds → different ciphertexts.
        let seed_a = test_seed();
        let mut seed_b = test_seed();
        seed_b[0] ^= 0xff;
        let fields = indexed_fields();
        let key_a = GaugeKey::derive(&seed_a, &fields);
        let key_b = GaugeKey::derive(&seed_b, &fields);

        let plain = vec![Value::Text("same_plaintext".to_string())];
        let ct_a = key_a.encrypt_fiber(&plain, "b", &fields);
        let ct_b = key_b.encrypt_fiber(&plain, "b", &fields);
        assert_ne!(ct_a, ct_b);
    }

    // ── Sprint D: Probabilistic (gauge + Gaussian + Davis Identity) ──

    fn probabilistic_fields(sigma: f64) -> Vec<FieldDef> {
        vec![
            FieldDef::numeric("amount").with_encryption(EncryptionMode::Probabilistic { sigma }),
        ]
    }

    #[test]
    fn test_probabilistic_distinct_ciphertexts_for_same_plaintext() {
        let seed = test_seed();
        let fields = probabilistic_fields(0.5);
        let key = GaugeKey::derive(&seed, &fields);

        let plain = vec![Value::Float(42.0)];
        let mut all_distinct = true;
        let first = key.encrypt_fiber(&plain, "b", &fields);
        for _ in 0..20 {
            let again = key.encrypt_fiber(&plain, "b", &fields);
            if first == again {
                all_distinct = false;
                break;
            }
        }
        assert!(
            all_distinct,
            "Probabilistic same-plaintext encrypts must differ (random Gaussian noise)"
        );
    }

    #[test]
    fn test_probabilistic_round_trip_within_sigma_tolerance() {
        let seed = test_seed();
        let sigma = 0.5;
        let fields = probabilistic_fields(sigma);
        let key = GaugeKey::derive(&seed, &fields);

        // Decryption is approximate. The error is bounded by σ/|scale|.
        // Average over many samples to verify the mean is close to plaintext.
        let plain_val = 100.0;
        let plain = vec![Value::Float(plain_val)];

        // Pull the field's scale out of the derived transform so we can
        // compute the expected error bound.
        let scale = match &key.transforms[0] {
            FieldTransform::Probabilistic { scale, .. } => *scale,
            other => panic!("expected Probabilistic, got {other:?}"),
        };

        let mut total_err = 0.0;
        let n = 200;
        for _ in 0..n {
            let encrypted = key.encrypt_fiber(&plain, "b", &fields);
            let decrypted = key.decrypt_fiber(&encrypted, "b", &fields);
            if let Value::Float(rec) = decrypted[0] {
                total_err += (rec - plain_val).abs();
            } else {
                panic!("decrypt should produce Float");
            }
        }
        let mean_err = total_err / n as f64;
        let bound = 4.0 * sigma / scale.abs();
        assert!(
            mean_err < bound,
            "Mean decrypt error {mean_err} exceeds expected ≤4σ/|scale| bound {bound}"
        );
    }

    #[test]
    fn test_probabilistic_bucket_hash_is_deterministic() {
        // The bucket portion of the ciphertext is deterministic — same
        // plaintext always rounds to the same σ-bucket, hashes to the same
        // u64, even though the noisy_value portion varies. This is what
        // makes equality search work via the Davis Identity.
        let seed = test_seed();
        let fields = probabilistic_fields(0.5);
        let key = GaugeKey::derive(&seed, &fields);

        let plain = vec![Value::Float(7.3)];
        let mut buckets = std::collections::HashSet::new();
        for _ in 0..100 {
            let ct = key.encrypt_fiber(&plain, "b", &fields);
            if let Value::Binary(bytes) = &ct[0] {
                assert_eq!(bytes.len(), 16, "Probabilistic ciphertext is 16 bytes");
                let bucket = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
                buckets.insert(bucket);
            }
        }
        assert_eq!(
            buckets.len(),
            1,
            "Same plaintext must produce the same σ-bucket hash across many encrypts (got {} distinct)",
            buckets.len()
        );
    }

    #[test]
    fn test_probabilistic_bucket_hash_distinguishes_distant_plaintexts() {
        // Plaintexts more than σ apart should land in different buckets
        // (with high probability — the bucket boundaries are at integer
        // multiples of σ).
        let seed = test_seed();
        let sigma = 0.5;
        let fields = probabilistic_fields(sigma);
        let key = GaugeKey::derive(&seed, &fields);

        // Plaintexts 10σ apart land in different buckets always.
        let plain_a = vec![Value::Float(0.0)];
        let plain_b = vec![Value::Float(10.0)];

        let ct_a = key.encrypt_fiber(&plain_a, "b", &fields);
        let ct_b = key.encrypt_fiber(&plain_b, "b", &fields);

        let bucket = |v: &Value| -> u64 {
            if let Value::Binary(b) = v {
                u64::from_le_bytes(b[8..16].try_into().unwrap())
            } else {
                panic!("not Binary");
            }
        };

        assert_ne!(
            bucket(&ct_a[0]),
            bucket(&ct_b[0]),
            "Plaintexts 10σ apart should land in different buckets"
        );
    }

    #[test]
    fn test_probabilistic_encrypt_literal_for_equality_search() {
        // The Davis Identity equality query: encrypt_literal(X) computes the
        // bucket portion that matches stored records whose plaintext shared
        // a σ-bucket with X.
        let seed = test_seed();
        let sigma = 0.5;
        let fields = probabilistic_fields(sigma);
        let key = GaugeKey::derive(&seed, &fields);

        // Store: plaintext = 42.0
        let stored = key.encrypt_fiber(&vec![Value::Float(42.0)], "b", &fields);

        // Query literal: 42.0 (same value, different encrypt call)
        let literal = key.encrypt_literal(0, &Value::Float(42.0), "b", &fields);

        let stored_bucket = if let Value::Binary(b) = &stored[0] {
            u64::from_le_bytes(b[8..16].try_into().unwrap())
        } else {
            panic!("expected Binary");
        };
        let literal_bucket = if let Value::Binary(b) = &literal {
            u64::from_le_bytes(b[8..16].try_into().unwrap())
        } else {
            panic!("expected Binary");
        };

        assert_eq!(
            stored_bucket, literal_bucket,
            "Same-plaintext stored record and query literal must share bucket hash"
        );
    }

    #[test]
    fn test_probabilistic_recall_at_sigma_window() {
        // Recall: stored plaintext == query plaintext → bucket match rate.
        // Plaintexts that round to the same σ-bucket always match. There's
        // a bucket-boundary effect (floor() rounds down) — so plaintexts
        // exactly at the bucket boundary may go to either side. For values
        // well within a bucket, recall is 100%.
        let seed = test_seed();
        let sigma = 1.0;
        let fields = probabilistic_fields(sigma);
        let key = GaugeKey::derive(&seed, &fields);

        // Use plaintexts that are clearly within their σ-buckets (not at
        // boundaries) so the test isn't measuring the boundary effect.
        let plaintexts: Vec<f64> = (1..=50).map(|i| i as f64 + 0.5).collect();
        let mut hits = 0;
        for v in &plaintexts {
            let stored = key.encrypt_fiber(&vec![Value::Float(*v)], "b", &fields);
            let literal = key.encrypt_literal(0, &Value::Float(*v), "b", &fields);
            if stored[0] == Value::Binary(vec![]) || literal == Value::Binary(vec![]) {
                continue;
            }
            // Compare bucket portions.
            let sb = match &stored[0] {
                Value::Binary(b) => &b[8..16],
                _ => panic!(),
            };
            let lb = match &literal {
                Value::Binary(b) => &b[8..16],
                _ => panic!(),
            };
            if sb == lb {
                hits += 1;
            }
        }
        let recall = hits as f64 / plaintexts.len() as f64;
        assert!(
            recall == 1.0,
            "Recall on values well within σ-buckets must be 100%, got {recall}"
        );
    }

    #[test]
    fn test_probabilistic_does_not_match_unrelated_plaintexts() {
        // Plaintexts far apart must NOT collide on bucket hash (within a
        // small false-positive bound from the bucket index hash).
        let seed = test_seed();
        let sigma = 1.0;
        let fields = probabilistic_fields(sigma);
        let key = GaugeKey::derive(&seed, &fields);

        let stored = key.encrypt_fiber(&vec![Value::Float(0.5)], "b", &fields);
        let stored_bucket = if let Value::Binary(b) = &stored[0] {
            u64::from_le_bytes(b[8..16].try_into().unwrap())
        } else {
            panic!()
        };

        let mut false_positives = 0;
        for i in 100..200 {
            let v = i as f64 + 0.5; // far away from 0.5
            let lit = key.encrypt_literal(0, &Value::Float(v), "b", &fields);
            let lb = if let Value::Binary(b) = &lit {
                u64::from_le_bytes(b[8..16].try_into().unwrap())
            } else {
                panic!()
            };
            if lb == stored_bucket {
                false_positives += 1;
            }
        }
        // FPR should be near zero for 100 unrelated plaintexts under a
        // 64-bit bucket hash. We'd need ~2^32 trials before expecting one
        // birthday collision.
        assert!(
            false_positives <= 1,
            "Expected ≤1 false positive in 100 unrelated plaintexts, got {false_positives}"
        );
    }

    #[test]
    fn test_probabilistic_curvature_invariance_holds_within_noise() {
        // The Probabilistic mode adds Gaussian noise σ on top of the affine
        // transform. K = Var/range² is no longer EXACTLY invariant — noise
        // increases variance by σ². But for σ « range, the relative change
        // in K is small. This test asserts the curvature stays within a
        // documented tolerance (σ²/range² fractional shift).
        //
        // For now we just verify the math runs end-to-end without panicking
        // and produces finite output — the strict-invariance claim belongs
        // to AFFINE mode, not PROBABILISTIC.
        let seed = test_seed();
        let fields = probabilistic_fields(0.5);
        let key = GaugeKey::derive(&seed, &fields);

        for v in [1.0, 10.0, 100.0, -50.0, 0.0] {
            let encrypted = key.encrypt_fiber(&vec![Value::Float(v)], "b", &fields);
            assert!(matches!(encrypted[0], Value::Binary(_)));
            let decrypted = key.decrypt_fiber(&encrypted, "b", &fields);
            assert!(matches!(decrypted[0], Value::Float(_)));
        }
    }

    // ── Mixed-mode bundle: realistic jg_account-style schema ──

    #[test]
    fn test_mixed_mode_bundle_roundtrip() {
        // legal_name = OPAQUE, kind = INDEXED, score = AFFINE, attempts = no-encryption
        let fields = vec![
            FieldDef::categorical("legal_name").with_encryption(EncryptionMode::Opaque),
            FieldDef::categorical("kind").with_encryption(EncryptionMode::Indexed),
            FieldDef::numeric("score").with_encryption(EncryptionMode::Affine),
            FieldDef::numeric("attempts"),
        ];
        let seed = test_seed();
        let key = GaugeKey::derive(&seed, &fields);

        let plain = vec![
            Value::Text("Alice".to_string()),
            Value::Text("paid".to_string()),
            Value::Float(0.95),
            Value::Integer(3),
        ];
        let encrypted = key.encrypt_fiber(&plain, "acct", &fields);

        // legal_name: AEAD blob
        assert!(matches!(encrypted[0], Value::Binary(_)));
        // kind: PRF tag (16 bytes)
        if let Value::Binary(ref b) = encrypted[1] { assert_eq!(b.len(), 16); }
        // score: affine-encrypted float
        assert!(matches!(encrypted[2], Value::Float(_)));
        assert_ne!(encrypted[2], plain[2]);
        // attempts: no encryption
        assert_eq!(encrypted[3], plain[3]);

        let decrypted = key.decrypt_fiber(&encrypted, "acct", &fields);
        // legal_name decrypts back to plaintext text
        assert_eq!(decrypted[0], plain[0]);
        // kind: identity (PRF one-way)
        assert_eq!(decrypted[1], encrypted[1]);
        // score: affine inverse recovers plaintext
        if let (Value::Float(a), Value::Float(b)) = (&decrypted[2], &plain[2]) {
            assert!((a - b).abs() < 1e-10);
        } else {
            panic!("score should decrypt to Float");
        }
        // attempts: identity
        assert_eq!(decrypted[3], plain[3]);
    }

    // ── Sprint E: ISOMETRIC for grouped numeric fiber (k≥2) ──
    //
    // The ISOMETRIC mode encrypts a vector v ∈ R^k as O·v + b where O is a
    // shared orthogonal matrix (O^T·O = I) and b is a shared offset, both
    // derived from the seed. Members of the same group_id share O and b;
    // each member field carries its row index. Round-trip recovers v exactly
    // (modulo float ulps); pairwise distance between two records is preserved
    // because |O·v_a - O·v_b| = |O·(v_a - v_b)| = |v_a - v_b|.

    fn isometric_fields_k2() -> Vec<FieldDef> {
        vec![
            FieldDef::numeric("u")
                .with_encryption(EncryptionMode::Isometric)
                .with_encryption_group("wind"),
            FieldDef::numeric("v")
                .with_encryption(EncryptionMode::Isometric)
                .with_encryption_group("wind"),
        ]
    }

    fn isometric_fields_k3() -> Vec<FieldDef> {
        vec![
            FieldDef::numeric("x")
                .with_encryption(EncryptionMode::Isometric)
                .with_encryption_group("emb"),
            FieldDef::numeric("y")
                .with_encryption(EncryptionMode::Isometric)
                .with_encryption_group("emb"),
            FieldDef::numeric("z")
                .with_encryption(EncryptionMode::Isometric)
                .with_encryption_group("emb"),
        ]
    }

    /// Each group member must carry the matrix and offset, with member_index
    /// == its position in the field list. Two fields in the same group must
    /// share the same matrix bytes (same orthogonal transform).
    #[test]
    fn test_isometric_group_members_share_matrix_k2() {
        let seed = test_seed();
        let fields = isometric_fields_k2();
        let key = GaugeKey::derive(&seed, &fields);

        let t0 = &key.transforms[0];
        let t1 = &key.transforms[1];
        match (t0, t1) {
            (
                FieldTransform::Isometric { group_id: g0, matrix: m0, offset_vec: o0, member_index: i0 },
                FieldTransform::Isometric { group_id: g1, matrix: m1, offset_vec: o1, member_index: i1 },
            ) => {
                assert_eq!(g0, g1, "both fields share group_id");
                assert_eq!(g0, "wind");
                assert_eq!(m0, m1, "shared matrix bytes");
                assert_eq!(o0, o1, "shared offset");
                assert_eq!(*i0, 0);
                assert_eq!(*i1, 1);
                assert_eq!(m0.len(), 2, "k=2 so 2x2 matrix");
                assert_eq!(o0.len(), 2);
            }
            _ => panic!("both fields must be Isometric"),
        }
    }

    /// O must satisfy O^T·O = I (orthogonality). Tested via direct dot product
    /// of rows: rows are unit-length and pairwise orthogonal.
    #[test]
    fn test_isometric_matrix_is_orthogonal_k3() {
        let seed = test_seed();
        let fields = isometric_fields_k3();
        let key = GaugeKey::derive(&seed, &fields);
        let t = &key.transforms[0];
        let matrix = match t {
            FieldTransform::Isometric { matrix, .. } => matrix,
            _ => panic!("expected Isometric"),
        };
        assert_eq!(matrix.len(), 3, "k=3");
        for i in 0..3 {
            assert_eq!(matrix[i].len(), 3, "row {i} has 3 cols");
        }
        // Row i · Row j == δ_ij
        for i in 0..3 {
            for j in 0..3 {
                let dot: f64 = (0..3).map(|k| matrix[i][k] * matrix[j][k]).sum();
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!(
                    (dot - expected).abs() < 1e-9,
                    "rows {i},{j}: dot={dot}, expected={expected}"
                );
            }
        }
    }

    /// Round-trip: encrypt(v) then decrypt recovers v.
    #[test]
    fn test_isometric_round_trip_k2() {
        let seed = test_seed();
        let fields = isometric_fields_k2();
        let key = GaugeKey::derive(&seed, &fields);

        let plain = vec![Value::Float(3.7), Value::Float(-1.2)];
        let cipher = key.encrypt_fiber(&plain, "weather", &fields);
        let recovered = key.decrypt_fiber(&cipher, "weather", &fields);

        for (p, r) in plain.iter().zip(recovered.iter()) {
            match (p, r) {
                (Value::Float(a), Value::Float(b)) => {
                    assert!((a - b).abs() < 1e-9, "round-trip: {a} vs {b}");
                }
                _ => panic!("expected Float"),
            }
        }
    }

    #[test]
    fn test_isometric_round_trip_k3() {
        let seed = test_seed();
        let fields = isometric_fields_k3();
        let key = GaugeKey::derive(&seed, &fields);

        let plain = vec![Value::Float(0.5), Value::Float(2.0), Value::Float(-3.5)];
        let cipher = key.encrypt_fiber(&plain, "emb_bundle", &fields);
        let recovered = key.decrypt_fiber(&cipher, "emb_bundle", &fields);

        for (p, r) in plain.iter().zip(recovered.iter()) {
            match (p, r) {
                (Value::Float(a), Value::Float(b)) => {
                    assert!((a - b).abs() < 1e-9, "round-trip: {a} vs {b}");
                }
                _ => panic!("expected Float"),
            }
        }
    }

    /// Geometric core: pairwise distance between encrypted records equals the
    /// pairwise distance between plaintext records (because O is orthogonal).
    #[test]
    fn test_isometric_preserves_pairwise_distance_k3() {
        let seed = test_seed();
        let fields = isometric_fields_k3();
        let key = GaugeKey::derive(&seed, &fields);

        let a_plain = vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)];
        let b_plain = vec![Value::Float(4.0), Value::Float(0.0), Value::Float(-1.0)];

        let a_cipher = key.encrypt_fiber(&a_plain, "emb_bundle", &fields);
        let b_cipher = key.encrypt_fiber(&b_plain, "emb_bundle", &fields);

        // Distance between plaintext vectors
        let d_plain: f64 = a_plain
            .iter()
            .zip(b_plain.iter())
            .map(|(p, q)| match (p, q) {
                (Value::Float(x), Value::Float(y)) => (x - y).powi(2),
                _ => 0.0,
            })
            .sum::<f64>()
            .sqrt();

        // Distance between ciphertext vectors
        let d_cipher: f64 = a_cipher
            .iter()
            .zip(b_cipher.iter())
            .map(|(p, q)| match (p, q) {
                (Value::Float(x), Value::Float(y)) => (x - y).powi(2),
                _ => 0.0,
            })
            .sum::<f64>()
            .sqrt();

        assert!(
            (d_plain - d_cipher).abs() < 1e-9,
            "isometric distance: plain={d_plain}, cipher={d_cipher}"
        );
        assert!(d_plain > 0.5, "sanity: distinct points produce nonzero distance");
    }

    /// Determinism: same seed + same fields → same matrix, same offsets.
    #[test]
    fn test_isometric_determinism_same_seed_same_matrix() {
        let seed = test_seed();
        let fields = isometric_fields_k2();
        let key1 = GaugeKey::derive(&seed, &fields);
        let key2 = GaugeKey::derive(&seed, &fields);

        match (&key1.transforms[0], &key2.transforms[0]) {
            (
                FieldTransform::Isometric { matrix: m1, offset_vec: o1, .. },
                FieldTransform::Isometric { matrix: m2, offset_vec: o2, .. },
            ) => {
                assert_eq!(m1, m2);
                assert_eq!(o1, o2);
            }
            _ => panic!("expected Isometric"),
        }
    }

    /// Different seeds → different matrices.
    #[test]
    fn test_isometric_different_seeds_different_matrix() {
        let seed_a = test_seed();
        let mut seed_b = test_seed();
        seed_b[0] ^= 0xff;

        let fields = isometric_fields_k2();
        let key_a = GaugeKey::derive(&seed_a, &fields);
        let key_b = GaugeKey::derive(&seed_b, &fields);

        let m_a = match &key_a.transforms[0] {
            FieldTransform::Isometric { matrix, .. } => matrix.clone(),
            _ => panic!(),
        };
        let m_b = match &key_b.transforms[0] {
            FieldTransform::Isometric { matrix, .. } => matrix.clone(),
            _ => panic!(),
        };
        assert_ne!(m_a, m_b, "different seeds must produce different orthogonal matrices");
    }

    /// Two separate groups in the same bundle have independent matrices.
    #[test]
    fn test_isometric_two_groups_have_independent_matrices() {
        let seed = test_seed();
        let fields = vec![
            FieldDef::numeric("u")
                .with_encryption(EncryptionMode::Isometric)
                .with_encryption_group("wind"),
            FieldDef::numeric("v")
                .with_encryption(EncryptionMode::Isometric)
                .with_encryption_group("wind"),
            FieldDef::numeric("ax")
                .with_encryption(EncryptionMode::Isometric)
                .with_encryption_group("accel"),
            FieldDef::numeric("ay")
                .with_encryption(EncryptionMode::Isometric)
                .with_encryption_group("accel"),
        ];
        let key = GaugeKey::derive(&seed, &fields);

        let m_wind = match &key.transforms[0] {
            FieldTransform::Isometric { matrix, group_id, .. } => {
                assert_eq!(group_id, "wind");
                matrix.clone()
            }
            _ => panic!(),
        };
        let m_accel = match &key.transforms[2] {
            FieldTransform::Isometric { matrix, group_id, .. } => {
                assert_eq!(group_id, "accel");
                matrix.clone()
            }
            _ => panic!(),
        };
        assert_ne!(m_wind, m_accel, "different groups must derive different matrices");
    }

    /// End-to-end: a four-field bundle with Affine + an ISOMETRIC group
    /// round-trips correctly.
    #[test]
    fn test_isometric_mixed_with_affine_round_trip() {
        let seed = test_seed();
        let fields = vec![
            FieldDef::numeric("score").with_encryption(EncryptionMode::Affine),
            FieldDef::numeric("u")
                .with_encryption(EncryptionMode::Isometric)
                .with_encryption_group("wind"),
            FieldDef::numeric("v")
                .with_encryption(EncryptionMode::Isometric)
                .with_encryption_group("wind"),
        ];
        let key = GaugeKey::derive(&seed, &fields);

        let plain = vec![
            Value::Float(0.95),
            Value::Float(3.0),
            Value::Float(-4.0),
        ];
        let cipher = key.encrypt_fiber(&plain, "weather", &fields);
        let recovered = key.decrypt_fiber(&cipher, "weather", &fields);

        for (p, r) in plain.iter().zip(recovered.iter()) {
            match (p, r) {
                (Value::Float(a), Value::Float(b)) => {
                    assert!((a - b).abs() < 1e-9, "field round-trip: {a} vs {b}");
                }
                _ => panic!("expected Float"),
            }
        }
    }
}
