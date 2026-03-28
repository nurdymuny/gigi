//! GIGI hash — coordinate chart G: K₁ × K₂ × ... × Kₘ → ℤ₂⁶⁴
//!
//! Implements Definition 1.5–1.6 and Theorem 1.1.

use crate::types::{BasePoint, BundleSchema, Record, Value};

/// Portable wyhash-style mixing function (no crate dependency).
/// Based on the wyhash finalizer: multiply-xor-shift.
#[inline]
fn wymix(a: u64, b: u64) -> u64 {
    let r = (a as u128).wrapping_mul(b as u128);
    (r as u64) ^ ((r >> 64) as u64)
}

/// Hash bytes with a seed — portable wyhash-inspired implementation.
fn wy(data: &[u8], seed: u64) -> u64 {
    let mut h = seed ^ 0x53c5ca59_e336_3290;
    let mut i = 0usize;
    let len = data.len();

    // Process 8-byte chunks
    while i + 8 <= len {
        let chunk = u64::from_le_bytes(data[i..i + 8].try_into().unwrap());
        h = wymix(h ^ chunk, 0x94d049bb_1331_11eb);
        i += 8;
    }

    // Process remaining bytes
    if i < len {
        let mut tail = 0u64;
        for (j, &b) in data[i..].iter().enumerate() {
            tail |= (b as u64) << (j * 8);
        }
        h = wymix(h ^ tail, 0x94d049bb_1331_11eb);
    }

    wymix(h ^ (len as u64), 0x9e3779b9_7f4a_7c15)
}

/// Per-field seeds derived from schema (Def 1.6).
#[derive(Debug, Clone)]
pub struct HashConfig {
    seeds: Vec<u64>,
}

impl HashConfig {
    /// Derive per-field seeds from the schema name and field names.
    pub fn from_schema(schema: &BundleSchema) -> Self {
        let base_seed = wy(schema.name.as_bytes(), 0);
        let seeds = schema
            .base_fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let field_bytes = f.name.as_bytes();
                wy(field_bytes, base_seed.wrapping_add(i as u64))
            })
            .collect();
        Self { seeds }
    }

    /// Fast hash for single-integer base field — no Vec allocation.
    #[inline]
    pub fn hash_int_fast(&self, val: i64) -> BasePoint {
        let bits = (val as u64) ^ (1u64 << 63);
        let h_i = wy(&bits.to_be_bytes(), self.seeds[0]);
        wy(&h_i.to_le_bytes(), 0xa0761d6478bd642f)
    }

    /// Compute the GIGI hash G(k₁, ..., kₘ) → BasePoint (Def 1.6).
    ///
    /// (a) Type-canonical encoding per field
    /// (b) Keyed mixing with per-field seed
    /// (c) Field composition via rotation and XOR
    pub fn hash(&self, record: &Record, schema: &BundleSchema) -> BasePoint {
        let mut combined: u64 = 0;
        for (i, field_def) in schema.base_fields.iter().enumerate() {
            let val = record.get(&field_def.name).unwrap_or(&Value::Null);
            let encoded = encode_value(val);
            let h_i = wy(&encoded, self.seeds[i]);
            // Rotation-based composition (Def 1.6c)
            let rotation = (i as u32 * 17) % 64;
            combined ^= h_i.rotate_left(rotation);
        }
        // Finalizer: one more round to ensure full avalanche
        wy(&combined.to_le_bytes(), 0xa0761d6478bd642f)
    }
}

/// Type-canonical encoding (Def 1.6a).
fn encode_value(val: &Value) -> Vec<u8> {
    match val {
        Value::Integer(v) => {
            // Big-endian with sign-bit flip for total ordering
            let bits = (*v as u64) ^ (1u64 << 63);
            bits.to_be_bytes().to_vec()
        }
        Value::Float(v) => {
            // IEEE 754 with sign-bit flip
            let mut bits = v.to_bits();
            if *v >= 0.0 {
                bits ^= 1u64 << 63;
            } else {
                bits = !bits;
            }
            bits.to_be_bytes().to_vec()
        }
        Value::Text(s) => s.as_bytes().to_vec(),
        Value::Bool(b) => vec![*b as u8],
        Value::Timestamp(v) => {
            let bits = (*v as u64) ^ (1u64 << 63);
            bits.to_be_bytes().to_vec()
        }
        Value::Null => vec![0xFF], // sentinel
        Value::Vector(v) => {
            // Encode as concatenated big-endian f64 bytes (for total-order hashing)
            let mut bytes = Vec::with_capacity(v.len() * 8);
            for &x in v {
                let mut bits = x.to_bits();
                if x >= 0.0 {
                    bits ^= 1u64 << 63;
                } else {
                    bits = !bits;
                }
                bytes.extend_from_slice(&bits.to_be_bytes());
            }
            bytes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FieldDef;
    use std::collections::HashSet;

    fn make_schema() -> BundleSchema {
        BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("value"))
    }

    /// TDD-1.7: Hash determinism — same key 1000× → all identical.
    #[test]
    fn tdd_1_7_determinism() {
        let schema = make_schema();
        let config = HashConfig::from_schema(&schema);
        let mut rec = Record::new();
        rec.insert("id".into(), Value::Integer(42));

        let first = config.hash(&rec, &schema);
        for _ in 0..1000 {
            assert_eq!(config.hash(&rec, &schema), first);
        }
    }

    /// TDD-1.4: Hash collision freedom — 10K distinct keys → 0 collisions.
    #[test]
    fn tdd_1_4_collision_freedom() {
        let schema = make_schema();
        let config = HashConfig::from_schema(&schema);
        let mut seen = HashSet::new();
        for i in 0..10_000i64 {
            let mut rec = Record::new();
            rec.insert("id".into(), Value::Integer(i));
            let h = config.hash(&rec, &schema);
            assert!(seen.insert(h), "collision at key {i}");
        }
    }

    /// TDD-1.5: Hash uniformity — χ² < 150 for 10K keys into 100 buckets.
    #[test]
    fn tdd_1_5_uniformity() {
        let schema = make_schema();
        let config = HashConfig::from_schema(&schema);
        let n_buckets = 100usize;
        let n_keys = 10_000usize;
        let mut buckets = vec![0u32; n_buckets];

        for i in 0..n_keys as i64 {
            let mut rec = Record::new();
            rec.insert("id".into(), Value::Integer(i));
            let h = config.hash(&rec, &schema);
            buckets[(h as usize) % n_buckets] += 1;
        }

        let expected = n_keys as f64 / n_buckets as f64;
        let chi_sq: f64 = buckets
            .iter()
            .map(|&b| {
                let diff = b as f64 - expected;
                diff * diff / expected
            })
            .sum();
        assert!(chi_sq < 150.0, "χ² = {chi_sq} >= 150");
    }

    /// TDD-1.6: Composite key O(1) — works with multiple fields.
    #[test]
    fn tdd_1_6_composite_key() {
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("a"))
            .base(FieldDef::categorical("b"))
            .base(FieldDef::numeric("c"));
        let config = HashConfig::from_schema(&schema);

        let mut rec = Record::new();
        rec.insert("a".into(), Value::Integer(1));
        rec.insert("b".into(), Value::Text("hello".into()));
        rec.insert("c".into(), Value::Integer(99));
        let h1 = config.hash(&rec, &schema);

        // Different composite key → different hash
        rec.insert("b".into(), Value::Text("world".into()));
        let h2 = config.hash(&rec, &schema);
        assert_ne!(h1, h2);
    }
}
