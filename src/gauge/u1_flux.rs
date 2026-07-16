//! `u1_flux` — `GAUGE_FIELD … GROUP U(1) INIT FLUX …` materializer
//! (Concept C of the 2026-07-16 SPECTRAL FULL + MODE MAGNETIC + U(1)
//! flux tranche; Hallie's confirmed ask).
//!
//! INIT FLUX materializes a **theta bundle**, not a gauge-registry
//! link buffer:
//!
//! - U(1)'s repr is a single angle per edge; the whole downstream
//!   loop (SPECTRAL_GAUGE ON FIBER (theta) [MODE MAGNETIC], COVER,
//!   EMIT) reads bundles, so the bundle IS the artifact. No
//!   `DenseLinkBuffer` exists for U(1) this phase
//!   (`DenseLinkBuffer::new_haar` is SU(2)/SU(3)-only) and INIT FLUX
//!   deliberately does not add one — the heatbath/registry surface
//!   stays SU(2)/SU(3), untouched.
//! - Schema (one record per lattice edge):
//!     base:  config_id (= 0), edge_id, vertex_a, vertex_b
//!     fiber: theta
//!   `vertex_a → vertex_b` is the lattice's OWN oriented edge
//!   (`lattice.edges[k] = (a, b)` is oriented a → b), which is exactly
//!   the orientation MODE MAGNETIC assembles: the record's θ applies
//!   to a → b as −e^{+iθ} (and b → a as the conjugate).
//!
//! DETERMINISM CONTRACT (part of Hallie's flux contract):
//! `FLUX RANDOM SEED n` draws θ_k = 2π · uniform_k from the house
//! xorshift64* [`SmallRng`] (`gauge::marsaglia_haar` — the same PRNG
//! and seeding INIT HAAR_RANDOM uses), one draw per edge, in the
//! lattice's edge order `k = 0..n_edges`. Same lattice + same seed →
//! byte-identical bundle, pinned by
//! `tests/u1_flux_basic.rs::test_init_flux_random_seed_deterministic`.
//! `FLUX UNIFORM phi` stamps every edge with exactly `phi`.
//!
//! PERSIST is rejected at the executor (the bundle inserts flow
//! through the engine WAL like INGEST does — the materialized bundle
//! is already the durable artifact); re-initializing an existing
//! bundle name is an error (an init is a materialization, not an
//! append).

use crate::engine::Engine;
use crate::lattice::Lattice;
use crate::types::{BundleSchema, FieldDef, Record, Value};

use super::marsaglia_haar::SmallRng;

/// Which flux pattern to materialize.
#[derive(Debug, Clone, PartialEq)]
pub enum FluxSpec {
    /// i.i.d. θ ~ Uniform[0, 2π), θ_k = 2π · uniform_k of
    /// `SmallRng::seed_from_u64(seed)`, edge order 0..n_edges.
    Random { seed: u64 },
    /// Every edge phase = phi (radians).
    Uniform { phi: f64 },
}

/// Insert-batch size for the materializer (mirrors the INGEST batch
/// shape; flux bundles are small — L=4 D=2 is 32 links — but the
/// batching keeps large lattices linear in memory).
const FLUX_BATCH_SIZE: usize = 1024;

/// Materialize the U(1) flux bundle `bundle` on `lattice` per `spec`.
/// Returns the number of edge records emitted (= `lattice.n_edges()`).
///
/// Errors (String-typed for the executor envelope):
/// - target bundle already exists ("already exists" — an init is a
///   materialization, not an append),
/// - engine create/insert failures, verbatim.
pub fn materialize_u1_flux_bundle(
    engine: &mut Engine,
    bundle: &str,
    lattice: &Lattice,
    spec: FluxSpec,
) -> Result<usize, String> {
    if engine.bundle(bundle).is_some() {
        return Err(format!(
            "gauge: INIT FLUX target bundle '{bundle}' already exists — an init \
             is a materialization, not an append; drop the bundle or pick a \
             new name"
        ));
    }

    let schema = BundleSchema::new(bundle)
        .base(FieldDef::numeric("config_id"))
        .base(FieldDef::numeric("edge_id"))
        .base(FieldDef::numeric("vertex_a"))
        .base(FieldDef::numeric("vertex_b"))
        .fiber(FieldDef::numeric("theta"));
    engine
        .create_bundle(schema)
        .map_err(|e| format!("gauge: INIT FLUX create_bundle('{bundle}'): {e}"))?;

    let two_pi = 2.0 * std::f64::consts::PI;
    let mut rng = match &spec {
        FluxSpec::Random { seed } => Some(SmallRng::seed_from_u64(*seed)),
        FluxSpec::Uniform { .. } => None,
    };

    let mut batch: Vec<Record> = Vec::with_capacity(FLUX_BATCH_SIZE.min(lattice.n_edges().max(1)));
    let mut emitted = 0usize;
    for (edge_id, &(va, vb)) in lattice.edges.iter().enumerate() {
        let theta = match &spec {
            FluxSpec::Random { .. } => {
                // One draw per edge, edge order 0..n_edges — the
                // byte-stability contract.
                two_pi
                    * rng
                        .as_mut()
                        .expect("Random spec constructs the RNG above")
                        .uniform()
            }
            FluxSpec::Uniform { phi } => *phi,
        };
        let mut rec = Record::new();
        rec.insert("config_id".to_string(), Value::Integer(0));
        rec.insert("edge_id".to_string(), Value::Integer(edge_id as i64));
        rec.insert("vertex_a".to_string(), Value::Integer(va as i64));
        rec.insert("vertex_b".to_string(), Value::Integer(vb as i64));
        rec.insert("theta".to_string(), Value::Float(theta));
        batch.push(rec);

        if batch.len() >= FLUX_BATCH_SIZE {
            emitted += engine
                .batch_insert(bundle, &batch)
                .map_err(|e| format!("gauge: INIT FLUX batch_insert('{bundle}'): {e}"))?;
            batch.clear();
        }
    }
    if !batch.is_empty() {
        emitted += engine
            .batch_insert(bundle, &batch)
            .map_err(|e| format!("gauge: INIT FLUX batch_insert('{bundle}'): {e}"))?;
    }
    Ok(emitted)
}
