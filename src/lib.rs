//! GIGI — Geometric Intrinsic Global Index
//!
//! A fiber-bundle-based database engine.
//! Davis Geometric · 2026

pub mod types;
pub mod hash;
pub mod bundle;
pub mod metric;
pub mod query;
pub mod curvature;
pub mod join;
pub mod aggregation;
pub mod wal;
pub mod engine;
pub mod gauge;
pub mod spectral;
pub mod parser;
pub mod concurrent;
pub mod dhoom;
pub mod convert;
pub mod edge;
pub mod crypto;

pub use bundle::{BundleStore, BaseGeometry, detect_base_geometry, QueryCondition, BundleStats, QueryPlan, TransactionOp, TransactionResult};
pub use types::{BundleSchema, FieldDef, FieldType, Value};
pub use query::QueryResult;
pub use metric::FiberMetric;
pub use engine::Engine;
