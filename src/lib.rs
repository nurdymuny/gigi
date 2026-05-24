//! GIGI — Geometric Intrinsic Global Index
//!
//! A fiber-bundle-based database engine.
//! Davis Geometric · 2026

pub mod aggregation;
pub mod bundle;
pub mod coherence;
pub mod concurrent;
pub mod convert;
pub mod crypto;
pub mod curvature;
pub mod dhoom;
pub mod edge;
pub mod engine;
pub mod gauge;
// Kähler-geometry substrate (catalog.md §1, the generator
// 𝒢 = (M, g, J, ∇, B, Γ)). Gated by the `kahler` feature so the
// engine's existing surface area is bit-identical when the feature
// is OFF. See theory/kahler_upgrade/ for catalog + implementation
// plan + validation tests.
#[cfg(feature = "kahler")]
pub mod geometry;
// Kähler graph operators (catalog.md §1.1): dual principal/
// auxiliary adjacency + commutativity classifier the query planner
// uses for theorem-backed join reordering. Same feature gate as
// `geometry` — strict additive layer.
#[cfg(feature = "kahler")]
pub mod graph;
pub mod hash;
pub mod invariant;
pub mod join;
pub mod metric;
pub mod mmap_bundle;
pub mod observability;
pub mod parser;
pub mod query;
pub mod sheaf;
pub mod spectral;
pub mod types;
pub mod wal;

pub use bundle::{
    detect_base_geometry, BaseGeometry, BundleStats, BundleStore, QueryCondition, QueryPlan,
    TransactionOp, TransactionResult, VectorMetric,
};
pub use engine::{Engine, MutationOp, Notification, QueryCache, TriggerDef, TriggerKind, TriggerManager, query_fingerprint};
pub use metric::FiberMetric;
pub use mmap_bundle::{BundleMut, BundleRef, OverlayBundle};
pub use query::QueryResult;
pub use types::{
    AdjacencyDef, AdjacencyKind, BundleSchema, FieldDef, FieldType, TransformFn, Value,
};
