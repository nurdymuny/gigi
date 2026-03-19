//! Geometric Encryption — Secret gauge transformations on fiber coordinates.
//!
//! A GaugeKey is a per-bundle secret that transforms fiber values via an affine
//! gauge transformation g(v) = a*v + b.  Curvature K = Var/range² is invariant
//! under affine transforms, so all geometric operations (curvature, confidence,
//! spectral gap, anomaly detection, prediction) work on encrypted data at native
//! speed.  Only SELECT (human-readable output) requires the inverse transform.

use crate::types::{Value, FieldDef, FieldType};

/// Per-field affine transform: v_enc = scale * v + offset
#[derive(Debug, Clone)]
pub struct FieldTransform {
    pub scale: f64,  // aᵢ ≠ 0
    pub offset: f64, // bᵢ
}

impl FieldTransform {
    /// Apply the forward gauge transform: v → a*v + b
    pub fn encrypt_f64(&self, v: f64) -> f64 {
        self.scale * v + self.offset
    }

    /// Apply the inverse gauge transform: w → (w - b) / a
    pub fn decrypt_f64(&self, w: f64) -> f64 {
        (w - self.offset) / self.scale
    }

    /// Encrypt a Value in place (only touches numeric types).
    pub fn encrypt_value(&self, v: &Value) -> Value {
        match v {
            Value::Float(f) => Value::Float(self.encrypt_f64(*f)),
            Value::Integer(i) => Value::Float(self.encrypt_f64(*i as f64)),
            Value::Timestamp(t) => Value::Timestamp(self.encrypt_f64(*t as f64) as i64),
            other => other.clone(), // Text, Bool, Null pass through
        }
    }

    /// Decrypt a Value (inverse transform).
    pub fn decrypt_value(&self, w: &Value) -> Value {
        match w {
            Value::Float(f) => Value::Float(self.decrypt_f64(*f)),
            Value::Integer(i) => Value::Float(self.decrypt_f64(*i as f64)),
            Value::Timestamp(t) => Value::Timestamp(self.decrypt_f64(*t as f64) as i64),
            other => other.clone(),
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
    ///
    /// Uses a simple deterministic KDF: for each field, we hash the seed with
    /// the field name to produce (scale, offset).  Scale is mapped to a nonzero
    /// range that avoids degenerate transforms.
    pub fn derive(seed: &[u8; 32], fiber_fields: &[FieldDef]) -> Self {
        let transforms = fiber_fields
            .iter()
            .map(|field| Self::derive_field_transform(seed, &field.name, &field.field_type))
            .collect();
        GaugeKey { transforms }
    }

    /// Derive transform for a single field from seed + field name.
    fn derive_field_transform(seed: &[u8; 32], field_name: &str, field_type: &FieldType) -> FieldTransform {
        match field_type {
            FieldType::Numeric | FieldType::Timestamp => {
                // Hash seed || field_name to get deterministic (scale, offset)
                let mut hasher_bytes = Vec::with_capacity(seed.len() + field_name.len() + 1);
                hasher_bytes.extend_from_slice(seed);
                hasher_bytes.push(b':');
                hasher_bytes.extend_from_slice(field_name.as_bytes());

                // Simple hash: use wyhash-style mixing
                let h1 = Self::mix_hash(&hasher_bytes, 0x517cc1b727220a95);
                let h2 = Self::mix_hash(&hasher_bytes, 0x6c62272e07bb0142);

                // Scale: map to range [0.1, 10.0], ensuring nonzero
                // Use the hash to pick a scale factor
                let scale_raw = (h1 as f64) / (u64::MAX as f64); // [0, 1)
                let scale = 0.1 + scale_raw * 9.9; // [0.1, 10.0)

                // Offset: map to range [-1000, 1000]
                let offset_raw = (h2 as f64) / (u64::MAX as f64); // [0, 1)
                let offset = -1000.0 + offset_raw * 2000.0; // [-1000, 1000)

                FieldTransform { scale, offset }
            }
            // Categorical/Binary: identity transform (no numeric encryption).
            // WARNING: Text/categorical fiber values are NOT encrypted by geometric
            // encryption. Only numeric fields receive affine gauge transforms.
            // If sensitive data exists in categorical fields, use application-level
            // encryption before inserting.
            _ => FieldTransform { scale: 1.0, offset: 0.0 },
        }
    }

    /// Simple deterministic hash mixing (wyhash-inspired).
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

    /// Encrypt a fiber value vector (in schema field order).
    pub fn encrypt_fiber(&self, fiber_vals: &[Value]) -> Vec<Value> {
        fiber_vals
            .iter()
            .enumerate()
            .map(|(i, v)| {
                if let Some(t) = self.transforms.get(i) {
                    t.encrypt_value(v)
                } else {
                    v.clone()
                }
            })
            .collect()
    }

    /// Decrypt a fiber value vector (in schema field order).
    pub fn decrypt_fiber(&self, encrypted_vals: &[Value]) -> Vec<Value> {
        encrypted_vals
            .iter()
            .enumerate()
            .map(|(i, w)| {
                if let Some(t) = self.transforms.get(i) {
                    t.decrypt_value(w)
                } else {
                    w.clone()
                }
            })
            .collect()
    }

    /// Encrypt a single query literal for a given fiber field index.
    pub fn encrypt_literal(&self, field_idx: usize, value: &Value) -> Value {
        if let Some(t) = self.transforms.get(field_idx) {
            t.encrypt_value(value)
        } else {
            value.clone()
        }
    }

    /// Generate a random 32-byte seed using OS CSPRNG.
    pub fn random_seed() -> [u8; 32] {
        let mut seed = [0u8; 32];
        getrandom::getrandom(&mut seed)
            .expect("Failed to generate random seed from OS CSPRNG");
        seed
    }
}

/// Parse a hex string into a 32-byte seed.
pub fn seed_from_hex(hex: &str) -> Result<[u8; 32], String> {
    let hex = hex.trim();
    if hex.len() != 64 {
        return Err(format!("Encryption seed must be 64 hex characters (32 bytes), got {}", hex.len()));
    }
    let mut seed = [0u8; 32];
    for i in 0..32 {
        seed[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|_| format!("Invalid hex at position {}", i * 2))?;
    }
    Ok(seed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FieldDef, FieldType, Value};

    fn test_seed() -> [u8; 32] {
        let mut s = [0u8; 32];
        for i in 0..32 { s[i] = (i as u8).wrapping_mul(7).wrapping_add(13); }
        s
    }

    fn numeric_fields() -> Vec<FieldDef> {
        vec![
            FieldDef::numeric("temp"),
            FieldDef::numeric("humidity"),
            FieldDef::numeric("pressure"),
        ]
    }

    #[test]
    fn test_derive_deterministic() {
        let seed = test_seed();
        let fields = numeric_fields();
        let k1 = GaugeKey::derive(&seed, &fields);
        let k2 = GaugeKey::derive(&seed, &fields);
        for (a, b) in k1.transforms.iter().zip(k2.transforms.iter()) {
            assert_eq!(a.scale, b.scale);
            assert_eq!(a.offset, b.offset);
        }
    }

    #[test]
    fn test_different_fields_different_transforms() {
        let seed = test_seed();
        let fields = numeric_fields();
        let key = GaugeKey::derive(&seed, &fields);
        // Each field should get a different transform
        assert_ne!(key.transforms[0].scale, key.transforms[1].scale);
        assert_ne!(key.transforms[0].offset, key.transforms[1].offset);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let seed = test_seed();
        let fields = numeric_fields();
        let key = GaugeKey::derive(&seed, &fields);

        let plain = vec![
            Value::Float(-31.9),
            Value::Float(65.0),
            Value::Float(1013.25),
        ];
        let encrypted = key.encrypt_fiber(&plain);
        let decrypted = key.decrypt_fiber(&encrypted);

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
    fn test_encrypted_values_differ_from_plain() {
        let seed = test_seed();
        let fields = numeric_fields();
        let key = GaugeKey::derive(&seed, &fields);

        let plain = vec![Value::Float(22.5), Value::Float(65.0), Value::Float(1013.25)];
        let encrypted = key.encrypt_fiber(&plain);

        for (p, e) in plain.iter().zip(encrypted.iter()) {
            assert_ne!(p, e, "Encrypted value should differ from plaintext");
        }
    }

    #[test]
    fn test_scale_nonzero() {
        let seed = test_seed();
        let fields = numeric_fields();
        let key = GaugeKey::derive(&seed, &fields);
        for t in &key.transforms {
            assert!(t.scale.abs() > 0.01, "Scale must be nonzero: {}", t.scale);
        }
    }

    #[test]
    fn test_categorical_passthrough() {
        let seed = test_seed();
        let fields = vec![FieldDef::categorical("city")];
        let key = GaugeKey::derive(&seed, &fields);
        assert_eq!(key.transforms[0].scale, 1.0);
        assert_eq!(key.transforms[0].offset, 0.0);

        let plain = vec![Value::Text("Moscow".to_string())];
        let encrypted = key.encrypt_fiber(&plain);
        assert_eq!(plain, encrypted);
    }

    #[test]
    fn test_seed_from_hex() {
        let hex = "0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c";
        let result = seed_from_hex(hex);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 32);
    }

    #[test]
    fn test_seed_from_hex_bad_length() {
        assert!(seed_from_hex("abcd").is_err());
    }

    #[test]
    fn test_integer_encryption() {
        let t = FieldTransform { scale: 3.0, offset: 100.0 };
        let v = Value::Integer(42);
        let enc = t.encrypt_value(&v);
        match enc {
            Value::Float(f) => assert!((f - 226.0).abs() < 1e-10),
            _ => panic!("Expected Float"),
        }
    }

    #[test]
    fn test_encrypt_literal() {
        let seed = test_seed();
        let fields = numeric_fields();
        let key = GaugeKey::derive(&seed, &fields);

        let lit = Value::Float(-25.0);
        let enc_lit = key.encrypt_literal(0, &lit);

        // The encrypted literal should differ from plain
        assert_ne!(lit, enc_lit);

        // If we encrypt -25.0 and compare to an encrypted fiber value of -25.0,
        // they should match
        let fiber = vec![Value::Float(-25.0), Value::Float(50.0), Value::Float(1000.0)];
        let enc_fiber = key.encrypt_fiber(&fiber);
        assert_eq!(enc_lit, enc_fiber[0]);
    }
}
