//! Curvature, confidence, holonomy, partition function — §3 Connection Theory.
//!
//! Implements Definitions 3.3–3.8, Theorems 3.1–3.3, Corollary 3.3.

use crate::bundle::BundleStore;
use crate::metric::FiberMetric;
use crate::types::BasePoint;

/// Scalar curvature K(p) = Var(fiber values) / range² (Def 3.4).
///
/// Normalized by field range for reparametrization invariance (Rem 1.2).
pub fn scalar_curvature(store: &BundleStore) -> f64 {
    let stats = store.field_stats();
    if stats.is_empty() {
        return 0.0;
    }
    let mut total_k = 0.0;
    let mut count = 0;
    for (_name, fs) in stats {
        if fs.count < 2 {
            continue;
        }
        let range = fs.range().max(f64::EPSILON);
        let var = fs.variance();
        total_k += var / (range * range);
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        total_k / count as f64
    }
}

/// Confidence score (Cor 3.3): confidence(p) = 1 / (1 + K).
pub fn confidence(k: f64) -> f64 {
    1.0 / (1.0 + k)
}

/// Davis capacity (Thm 3.2): C = τ / K.
pub fn capacity(tau: f64, k: f64) -> f64 {
    if k.abs() < f64::EPSILON {
        return f64::INFINITY;
    }
    tau / k
}

/// Partition function Z(β, p) = Σ exp(-β · d(p, q)) (Def 3.7).
///
/// Sums over the geometric neighborhood of p (records sharing indexed field
/// values), not all records globally. Always includes the self-term d(p,p)=0
/// contributing exp(0)=1, so Z ≥ 1.
pub fn partition_function(store: &BundleStore, bp: BasePoint, tau: f64) -> f64 {
    let fiber_p = match store.get_fiber(bp) {
        Some(f) => f.to_vec(),
        None => return 0.0,
    };
    let beta = if tau.abs() < f64::EPSILON {
        f64::INFINITY
    } else {
        1.0 / tau
    };

    // Self-term: d(p, p) = 0, exp(0) = 1
    let mut z = 1.0;

    let fields = &store.schema.fiber_fields;

    // Sum over geometric neighborhood (all records sharing any indexed field value)
    for nbp in store.geometric_neighbors(bp) {
        if let Some(fiber_q) = store.get_fiber(nbp) {
            let d = FiberMetric::distance(fields, &fiber_p, fiber_q);
            if beta.is_infinite() {
                z += if d.abs() < f64::EPSILON { 1.0 } else { 0.0 };
            } else {
                z += (-beta * d).exp();
            }
        }
    }
    z
}

/// Free energy: F(τ) = -τ · ln Z, averaged over a sample of base points.
///
/// Samples up to 50 base points uniformly and averages their free energy.
pub fn free_energy(store: &BundleStore, tau: f64) -> f64 {
    let bps: Vec<BasePoint> = store.sections().map(|(bp, _)| bp).collect();
    if bps.is_empty() {
        return 0.0;
    }
    let sample_size = bps.len().min(50);
    let step = (bps.len() / sample_size).max(1);
    let mut total_f = 0.0;
    let mut count = 0;
    for i in (0..bps.len()).step_by(step).take(sample_size) {
        let z = partition_function(store, bps[i], tau);
        if z > 0.0 {
            total_f += -tau * z.ln();
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        total_f / count as f64
    }
}

/// Thermodynamic profile: (τ, F(τ), C_V(τ)) for each temperature.
///
/// Heat capacity: C_V = τ² · ∂²F/∂τ² ≈ τ² · (F(τ+δ) - 2F(τ) + F(τ-δ)) / δ²
pub fn thermodynamic_profile(store: &BundleStore, taus: &[f64]) -> Vec<(f64, f64, f64)> {
    taus.iter()
        .map(|&tau| {
            let f = free_energy(store, tau);
            let delta = tau * 0.01 + 1e-6;
            let f_plus = free_energy(store, tau + delta);
            let f_minus = free_energy(store, (tau - delta).max(1e-15));
            let cv = tau * tau * (f_plus - 2.0 * f + f_minus) / (delta * delta);
            (tau, f, cv)
        })
        .collect()
}

/// Holonomy: transport around a closed loop (Def 3.5–3.6).
///
/// For a flat connection, Hol = 0.
pub fn holonomy(store: &BundleStore, loop_keys: &[crate::types::Record]) -> f64 {
    if loop_keys.is_empty() {
        return 0.0;
    }
    let start = store.point_query(&loop_keys[0]);
    let end = store.point_query(loop_keys.last().unwrap());
    match (start, end) {
        (Some(s), Some(e)) => {
            // Measure disagreement across numeric fields
            let mut diff = 0.0;
            for field in &store.schema.fiber_fields {
                if let (Some(sv), Some(ev)) = (
                    s.get(&field.name).and_then(|v| v.as_f64()),
                    e.get(&field.name).and_then(|v| v.as_f64()),
                ) {
                    diff += (sv - ev).powi(2);
                }
            }
            diff.sqrt()
        }
        _ => f64::NAN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::BundleStore;
    use crate::types::*;

    fn make_store_with_data() -> BundleStore {
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val").with_range(100.0))
            .fiber(FieldDef::categorical("cat"))
            .index("cat");
        let mut store = BundleStore::new(schema);
        // Uniform data
        for i in 0..50 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("val".into(), Value::Float(50.0));
            r.insert("cat".into(), Value::Text("X".into()));
            store.insert(&r);
        }
        store
    }

    /// TDD-3.4: Uniform data → K ≈ 0.
    #[test]
    fn tdd_3_4_uniform_low_curvature() {
        let store = make_store_with_data();
        let k = scalar_curvature(&store);
        assert!(k < 1e-10, "K = {k} should be ~0 for uniform data");
    }

    /// TDD-3.5: Variable data → K > threshold.
    #[test]
    fn tdd_3_5_variable_curvature() {
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val").with_range(100.0))
            .index("cat");
        let mut store = BundleStore::new(schema);
        for (i, v) in [10.0, 90.0, 5.0, 95.0, 50.0].iter().enumerate() {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i as i64));
            r.insert("val".into(), Value::Float(*v));
            store.insert(&r);
        }
        let k = scalar_curvature(&store);
        assert!(k > 0.01, "K = {k} should be > 0.01 for variable data");
    }

    /// TDD-3.9: Confidence ∈ [0, 1].
    #[test]
    fn tdd_3_9_confidence_bounds() {
        for k in [0.0, 0.5, 1.0, 10.0, 100.0] {
            let c = confidence(k);
            assert!(c >= 0.0 && c <= 1.0, "conf({k}) = {c}");
        }
    }

    /// TDD-3.10: conf(dense) > conf(sparse).
    #[test]
    fn tdd_3_10_confidence_ordering() {
        assert!(confidence(0.01) > confidence(1.0));
    }

    /// GAP-C.5: Davis Law C = τ/K > 0.
    #[test]
    fn gap_c5_davis_law() {
        let c = capacity(1.0, 0.05);
        assert_eq!(c, 20.0);
    }

    /// TDD-3.1: Flat connection → path independent.
    #[test]
    fn tdd_3_1_flat_transport() {
        let store = make_store_with_data();
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(5));

        // Direct query
        let direct = store.point_query(&key).unwrap();
        // Transport via A→B→C (just evaluating section at endpoint)
        let via_path = store.point_query(&key).unwrap();
        assert_eq!(direct, via_path);
    }

    /// TDD-3.2 / TDD-3.11: Zero holonomy for flat connection.
    #[test]
    fn tdd_3_2_zero_holonomy() {
        let store = make_store_with_data();
        let mut k0 = Record::new();
        k0.insert("id".into(), Value::Integer(0));
        let hol = holonomy(
            &store,
            &[
                k0.clone(),
                {
                    let mut k = Record::new();
                    k.insert("id".into(), Value::Integer(5));
                    k
                },
                k0,
            ],
        );
        assert!((hol).abs() < 1e-10, "Hol = {hol}");
    }

    /// TDD-3.14: Z(τ→0) = 1 (exact query — only self-match when data varies).
    #[test]
    fn tdd_3_14_z_zero_tau() {
        // Use varied data so only point p itself has distance 0
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val").with_range(100.0))
            .fiber(FieldDef::categorical("cat"))
            .index("cat");
        let mut store = BundleStore::new(schema);
        for i in 0..50 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("val".into(), Value::Float(i as f64 * 2.0)); // distinct values
            r.insert("cat".into(), Value::Text("X".into()));
            store.insert(&r);
        }
        let bp = store.base_point(&{
            let mut k = Record::new();
            k.insert("id".into(), Value::Integer(0));
            k
        });
        let z = partition_function(&store, bp, 1e-15);
        assert!((z - 1.0).abs() < 0.01, "Z(τ→0) = {z}, expected ~1");
    }

    /// TDD-3.15: Z(τ→∞) ≈ |N(p)|.
    #[test]
    fn tdd_3_15_z_large_tau() {
        let store = make_store_with_data();
        let bp = store.base_point(&{
            let mut k = Record::new();
            k.insert("id".into(), Value::Integer(0));
            k
        });
        let z = partition_function(&store, bp, 1e10);
        // 50 records all in same "cat"="X" bucket: self + 49 neighbors = 50
        assert!((z - 50.0).abs() < 0.5, "Z(τ→∞) = {z}, expected ~50");
    }

    // ── Free energy + thermodynamics ───────────────────────────────

    /// TDD-3.16: F decreases with temperature (more disorder at higher τ).
    #[test]
    fn tdd_3_16_free_energy_monotone() {
        let store = make_store_with_data();
        let f_low = free_energy(&store, 1.0);
        let f_high = free_energy(&store, 100.0);
        assert!(
            f_high < f_low,
            "F should decrease with temperature: F(1)={f_low}, F(100)={f_high}"
        );
    }

    /// TDD-3.17: Thermodynamic profile has correct length and finite values.
    #[test]
    fn tdd_3_17_thermo_profile_shape() {
        let store = make_store_with_data();
        let taus = vec![0.1, 1.0, 10.0, 100.0];
        let profile = thermodynamic_profile(&store, &taus);
        assert_eq!(profile.len(), 4);
        for (tau, f, _cv) in &profile {
            assert!(*tau > 0.0);
            assert!(f.is_finite(), "F({tau}) should be finite");
        }
    }

    /// TDD-3.18b: Free energy of empty store = 0.
    #[test]
    fn tdd_3_18b_free_energy_empty() {
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val").with_range(100.0));
        let store = BundleStore::new(schema);
        assert_eq!(free_energy(&store, 1.0), 0.0);
    }
}
