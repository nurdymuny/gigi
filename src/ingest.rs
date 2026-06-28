//! Halcyon ITEM 3.2 — INGEST executor.
//!
//! The `INGEST <bundle> FROM <source> FORMAT <format>` verb reads a
//! structured array file off disk, maps its outermost axis to one
//! `Record` per slice, and streams the records through
//! `engine.batch_insert`. This is the generic glue between the gigi
//! engine and the external array formats that downstream packages
//! (notably Halcyon's SU(3) lattice harvest) emit.
//!
//! # Supported formats (Phase 1)
//!
//! | Format | Source extension | Notes                                       |
//! |--------|------------------|---------------------------------------------|
//! | `NPZ`  | `.npz`           | NumPy zip archive of one-or-more `.npy`     |
//! |        |                  | arrays. Each archive member becomes a       |
//! |        |                  | named record stream (one record per         |
//! |        |                  | outer-axis slice).                          |
//!
//! HDF5, JSONL, and CSV are deferred to Phase 2. CSV/JSON already have
//! readers in `src/convert.rs` that the next sprint will wire through
//! an `IngestFormat` enum extension. The 3.2 surface is intentionally
//! narrow: NPZ only, because that's the format Halcyon's harvest
//! pipeline emits and the §3.2 commitment is to land THAT path
//! end-to-end first.
//!
//! # Record mapping policy (Phase 1 — `AUTO_GENERIC`)
//!
//! For an NPZ archive containing arrays of shape `(N, …)`, we iterate
//! the outermost axis and emit `N` records per array. Each record
//! carries:
//!
//! - `row_idx: Integer` — outer-axis index (0..N), monotone-increasing
//!   per array;
//! - `array_name: Text` — the `.npy` member name (only present when the
//!   archive contains more than one array);
//! - `<array_name>: Vector(Vec<f64>)` — the flattened inner slice,
//!   length `prod(shape[1..])`.
//!
//! For the Halcyon harvest shape `(L, L, L, L, 9)` with `L=12`, this
//! emits 12 records per file, each carrying a `Vector` of length
//! `12 * 12 * 12 * 9 = 15552`. Halcyon's downstream Phase-2 reader
//! reshapes that vector into the site/mu/SU(3)-matrix structure it
//! needs. Keeping the schema generic at Phase 1 means INGEST is not
//! coupled to SU(3) — future arrays (curvature scalars, observable
//! time series, lattice fermion correlators) flow through the same
//! verb without parser changes.
//!
//! A future EXPLICIT_SCHEMA path (parser surface
//! `INGEST … SCHEMA (…)`) is the obvious extension; it lands in a
//! follow-up sprint that touches `Statement::Ingest` and `parse_ingest`
//! to carry the field map. The 3.2 AST surface is unchanged.
//!
//! # Bundle auto-creation
//!
//! - If the target bundle does NOT exist, INGEST infers a schema from
//!   the first batch (one `row_idx: Numeric` base field + the per-array
//!   `Vector` fiber fields with declared `dims`) and calls
//!   `engine.create_bundle`. `IngestStats::bundle_created` reports
//!   whether this fired.
//! - If the target bundle DOES exist, INGEST validates that every
//!   inferred field is present in the existing schema with a
//!   compatible type, and rejects with
//!   `IngestError::SchemaConflict` otherwise.
//! - Virtual bundle names are rejected at the entry point via
//!   `crate::virtual_bundles::reject_virtual_write` — virtual bundles
//!   are read-only and never auto-created.
//!
//! # Streaming and memory bound
//!
//! Records are emitted in chunks of `INGEST_BATCH_SIZE = 10_000` to
//! `engine.batch_insert`. For the Halcyon `(12,12,12,12,9)` case the
//! whole file is well under one batch, so the work is a single
//! `batch_insert` call. For larger arrays peak RSS is bounded by
//! `BATCH_SIZE × slice_bytes` plus npyz's own read buffer.
//!
//! # Locked posture
//!
//! This module compiles UNCONDITIONALLY (no feature flag) because it
//! is the executor arm for a parser verb that's always present. The
//! `cargo test --no-default-features --lib` floor at 882 must not
//! regress.

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use crate::engine::Engine;
use crate::types::{BundleSchema, FieldDef, FieldType, Record, Value};

/// Batch size for `engine.batch_insert` calls — chosen so a single
/// batch stays under ~80 MB for the Halcyon `(L=12, D=4, 9)` shape
/// (12 records × ~15552 floats × 8 bytes ≈ 1.5 MB; well under the
/// budget).
const INGEST_BATCH_SIZE: usize = 10_000;

/// Format-specific entry points. Phase 1 ships NPZ only; HDF5/JSONL
/// land as additional variants in a follow-up sprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestFormat {
    /// NumPy zip archive (`.npz`).
    Npz,
}

impl IngestFormat {
    /// Parse a format string from the AST (`FORMAT NPZ`) into the
    /// typed enum. Case-insensitive on the format name to match the
    /// parser's word tokenization. The list of accepted format names
    /// is the SINGLE source of truth — `FORMAT JSON`, `FORMAT CSV`,
    /// etc. all fail through the same `FormatNotSupported` error so
    /// the surface is uniform.
    pub fn from_name(name: &str) -> Result<Self, IngestError> {
        match name.to_ascii_uppercase().as_str() {
            "NPZ" => Ok(IngestFormat::Npz),
            other => Err(IngestError::FormatNotSupported {
                requested: other.to_string(),
                supported: vec!["NPZ".to_string()],
            }),
        }
    }
}

/// Statistics returned on a successful INGEST. The parser executor
/// surfaces these via `ExecResult::Ok` today (Phase 1) and will
/// graduate to `ExecResult::Stats` in a follow-up when the GqlStats
/// shape supports `records_emitted` natively.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestStats {
    /// Number of records emitted into the target bundle.
    pub records_emitted: usize,
    /// `true` if the executor created the bundle from the inferred
    /// schema; `false` if the bundle pre-existed.
    pub bundle_created: bool,
    /// Number of bytes read from the source file (file size).
    pub bytes_read: u64,
}

/// Errors surfaced by the ingest executor. Every variant is mapped to
/// a `String` in the parser entry point so the existing parser error
/// envelope (`Result<ExecResult, String>`) carries the right context
/// at the GQL surface.
#[derive(Debug)]
pub enum IngestError {
    /// The source file does not exist on disk.
    FileNotFound(PathBuf),
    /// The requested format is not in the supported set.
    FormatNotSupported {
        requested: String,
        supported: Vec<String>,
    },
    /// The format reader failed (npyz / zip surfaced an error).
    FormatError(String),
    /// Existing bundle's schema is incompatible with the array's
    /// inferred schema.
    SchemaConflict {
        bundle: String,
        field: String,
        existing: String,
        incoming: String,
    },
    /// The engine returned an error (WAL write, schema mismatch on
    /// insert, etc.).
    EngineError(String),
    /// The NPZ archive contained zero `.npy` members.
    EmptyArchive(PathBuf),
    /// The INGEST target bundle does not exist and auto-create was
    /// disabled (or failed) — caller should create the bundle first.
    /// Surfaced under ergonomics #5 (2026-06-28) to replace the
    /// misleading wrapped engine error when an INGEST targets a
    /// non-existent bundle.
    TargetBundleNotFound { bundle: String },
}

impl std::fmt::Display for IngestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IngestError::FileNotFound(p) => write!(f, "INGEST: source file not found: {}", p.display()),
            IngestError::FormatNotSupported { requested, supported } => write!(
                f,
                "INGEST: format `{}` not supported (Phase 1 supports: {})",
                requested,
                supported.join(", ")
            ),
            IngestError::FormatError(msg) => write!(f, "INGEST: format error: {msg}"),
            IngestError::SchemaConflict { bundle, field, existing, incoming } => write!(
                f,
                "INGEST: schema conflict on bundle `{bundle}` field `{field}`: existing={existing}, incoming={incoming}"
            ),
            IngestError::EngineError(msg) => write!(f, "INGEST: engine error: {msg}"),
            IngestError::EmptyArchive(p) => write!(f, "INGEST: NPZ archive is empty: {}", p.display()),
            IngestError::TargetBundleNotFound { bundle } => write!(
                f,
                "INGEST: destination bundle '{bundle}' does not exist — create the bundle first via 'CREATE BUNDLE {bundle} ...' or pass --auto-create to infer schema from the source"
            ),
        }
    }
}

impl std::error::Error for IngestError {}

impl From<io::Error> for IngestError {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::NotFound {
            // Caller has the path; default to a sentinel here so the
            // generic path through `?` still produces a typed error.
            // The format-specific entry points construct the
            // `FileNotFound` variant directly with the real path.
            IngestError::EngineError(format!("io error: {e}"))
        } else {
            IngestError::EngineError(format!("io error: {e}"))
        }
    }
}

/// Inferred record schema from a chunk — captures the field names and
/// types that will be present on every emitted `Record`. Used to
/// auto-create the bundle if missing, or to validate against an
/// existing bundle.
#[derive(Debug, Clone)]
pub struct InferredFieldSchema {
    name: String,
    field_type: FieldType,
    is_base: bool,
}

impl InferredFieldSchema {
    /// Construct a numeric base-or-fiber inferred field. Exposed for
    /// the integration test that exercises the `TargetBundleNotFound`
    /// error path (ergonomics #5, 2026-06-28).
    #[doc(hidden)]
    pub fn numeric(name: &str, is_base: bool) -> Self {
        InferredFieldSchema {
            name: name.to_string(),
            field_type: FieldType::Numeric,
            is_base,
        }
    }

    fn type_label(&self) -> String {
        match &self.field_type {
            FieldType::Numeric => "Numeric".to_string(),
            FieldType::Categorical => "Categorical".to_string(),
            FieldType::OrderedCat { .. } => "OrderedCat".to_string(),
            FieldType::Timestamp => "Timestamp".to_string(),
            FieldType::Binary => "Binary".to_string(),
            FieldType::Vector { dims } => format!("Vector(dims={dims})"),
        }
    }
}

/// Public entry point — called by `parser::execute` for the
/// `Statement::Ingest` arm. Reads the source file according to the
/// requested format, maps it to records via the auto-generic policy,
/// and streams the records into `engine` via `batch_insert`.
pub fn execute_ingest(
    engine: &mut Engine,
    target_bundle: &str,
    source_path: &Path,
    format: IngestFormat,
) -> Result<IngestStats, IngestError> {
    if !source_path.exists() {
        return Err(IngestError::FileNotFound(source_path.to_path_buf()));
    }
    let bytes_read = std::fs::metadata(source_path)
        .map(|m| m.len())
        .unwrap_or(0);

    match format {
        IngestFormat::Npz => ingest_npz(engine, target_bundle, source_path, bytes_read),
    }
}

/// NPZ-specific reader: opens the archive, enumerates `.npy` members,
/// and emits records per member's outermost axis.
fn ingest_npz(
    engine: &mut Engine,
    target_bundle: &str,
    source_path: &Path,
    bytes_read: u64,
) -> Result<IngestStats, IngestError> {
    let mut archive = npyz::npz::NpzArchive::open(source_path)
        .map_err(|e| IngestError::FormatError(format!("open NPZ {}: {e}", source_path.display())))?;

    let array_names: Vec<String> = archive.array_names().map(|s| s.to_string()).collect();
    if array_names.is_empty() {
        return Err(IngestError::EmptyArchive(source_path.to_path_buf()));
    }

    // Multi-array vs single-array: when there's more than one member
    // we tag each record with `array_name` so callers can demux.
    let multi_array = array_names.len() > 1;

    // First pass: read all arrays into in-memory float buffers so we
    // can infer one unified schema before any side effect on the
    // engine. For Halcyon's harvest (~one array of ~1.5 MB after f64
    // expansion at L=12) this is comfortably bounded. The streaming
    // tail below DOES NOT re-read the arrays — it consumes these
    // buffers in chunks.
    struct ArrayBuf {
        name: String,
        shape: Vec<u64>,
        data: Vec<f64>,
    }
    let mut buffers: Vec<ArrayBuf> = Vec::with_capacity(array_names.len());
    for name in &array_names {
        let entry = archive
            .by_name(name)
            .map_err(|e| IngestError::FormatError(format!("read array `{name}`: {e}")))?
            .ok_or_else(|| {
                IngestError::FormatError(format!(
                    "NPZ member `{name}` listed but not retrievable",
                ))
            })?;
        let shape: Vec<u64> = entry.shape().to_vec();
        if shape.is_empty() {
            return Err(IngestError::FormatError(format!(
                "array `{name}` has zero-rank shape; INGEST requires at least one axis",
            )));
        }
        let data: Vec<f64> = entry
            .into_vec::<f64>()
            .map_err(|e| IngestError::FormatError(format!("decode array `{name}` as f64: {e}")))?;
        buffers.push(ArrayBuf { name: name.clone(), shape, data });
    }

    // Infer a unified schema from the buffers. The schema is the union
    // of {row_idx: Numeric (base)} ∪ {array_name: Categorical (also
    // base when multi_array, so (array_name, row_idx) is the primary
    // key and records from different arrays don't collide)}
    // ∪ {<array_name>: Vector(dims=inner_len) (fiber) for each array}.
    // Auto-create when missing; otherwise validate.
    let mut inferred: Vec<InferredFieldSchema> = Vec::new();
    if multi_array {
        // For multi-array archives the composite key is
        // (array_name, row_idx) so each member's slices land in
        // disjoint base points. `array_name` is placed FIRST so the
        // base-key tuple sorts naturally by member.
        inferred.push(InferredFieldSchema {
            name: "array_name".to_string(),
            field_type: FieldType::Categorical,
            is_base: true,
        });
    }
    inferred.push(InferredFieldSchema {
        name: "row_idx".to_string(),
        field_type: FieldType::Numeric,
        is_base: true,
    });
    for buf in &buffers {
        let inner_len: u64 = buf.shape[1..].iter().product::<u64>().max(1);
        inferred.push(InferredFieldSchema {
            name: buf.name.clone(),
            field_type: FieldType::Vector {
                dims: inner_len as usize,
            },
            is_base: false,
        });
    }

    // GQL surface preserves auto-create-by-default. A future
    // NO_AUTO_CREATE keyword on the INGEST AST will flip this to
    // `false`; until then, every INGEST that arrives via the parser
    // still auto-creates the bundle from inferred schema.
    let bundle_created =
        ensure_bundle_compatible(engine, target_bundle, &inferred, /*allow_auto_create=*/ true)?;

    // Stream records: for each array, iterate outermost axis and emit
    // a record per slice. We accumulate into a Vec<Record> of size
    // BATCH_SIZE and flush via `engine.batch_insert`. The field set is
    // EXACTLY the inferred schema so the existing-bundle compatibility
    // check above is sufficient.
    let mut records_emitted: usize = 0;
    let mut batch: Vec<Record> = Vec::with_capacity(INGEST_BATCH_SIZE.min(64));
    let array_names_for_dummy: Vec<String> = buffers.iter().map(|b| b.name.clone()).collect();

    for buf in &buffers {
        let outer_len = buf.shape[0] as usize;
        let inner_len: usize = buf.shape[1..].iter().product::<u64>().max(1) as usize;
        if buf.data.len() != outer_len * inner_len {
            return Err(IngestError::FormatError(format!(
                "array `{}` data length {} != shape product {}",
                buf.name,
                buf.data.len(),
                outer_len * inner_len,
            )));
        }
        for row_idx in 0..outer_len {
            let start = row_idx * inner_len;
            let end = start + inner_len;
            let slice = buf.data[start..end].to_vec();

            let mut record: Record = Record::new();
            record.insert("row_idx".to_string(), Value::Integer(row_idx as i64));
            if multi_array {
                record.insert(
                    "array_name".to_string(),
                    Value::Text(buf.name.clone()),
                );
            }
            // Add the slice under the array's own name.
            record.insert(buf.name.clone(), Value::Vector(slice));
            // For every OTHER fiber field declared in the inferred
            // schema (i.e. other arrays in a multi-array archive) we
            // still emit `Value::Null` so the record's field set is
            // self-consistent and the engine's record store can fill
            // every column. This lets downstream UNION-style readers
            // assume a uniform projection across rows.
            if multi_array {
                for other in &array_names_for_dummy {
                    if other != &buf.name {
                        record.entry(other.clone()).or_insert(Value::Null);
                    }
                }
            }
            batch.push(record);

            if batch.len() >= INGEST_BATCH_SIZE {
                flush_batch(engine, target_bundle, &mut batch, &mut records_emitted)?;
            }
        }
    }
    if !batch.is_empty() {
        flush_batch(engine, target_bundle, &mut batch, &mut records_emitted)?;
    }

    Ok(IngestStats {
        records_emitted,
        bundle_created,
        bytes_read,
    })
}

/// Drain `batch` into `engine.batch_insert` and tally records.
fn flush_batch(
    engine: &mut Engine,
    bundle: &str,
    batch: &mut Vec<Record>,
    tally: &mut usize,
) -> Result<(), IngestError> {
    let n = engine
        .batch_insert(bundle, batch)
        .map_err(|e| IngestError::EngineError(e.to_string()))?;
    *tally += n;
    batch.clear();
    Ok(())
}

/// Ensure the target bundle either exists with a compatible schema or
/// can be auto-created from the inferred fields. Returns `true` if the
/// bundle was created in this call, `false` if it pre-existed.
///
/// When `allow_auto_create` is `false` and the bundle does not exist,
/// returns `IngestError::TargetBundleNotFound` instead of silently
/// creating the bundle (ergonomics #5, 2026-06-28). The existing GQL
/// surface keeps `allow_auto_create = true` for backwards compatibility.
fn ensure_bundle_compatible(
    engine: &mut Engine,
    target_bundle: &str,
    inferred: &[InferredFieldSchema],
    allow_auto_create: bool,
) -> Result<bool, IngestError> {
    if engine.bundle(target_bundle).is_some() {
        // Bundle exists — validate the inferred schema is a subset of
        // the existing schema with compatible types.
        let bundle = engine.bundle(target_bundle).unwrap();
        let store = bundle.as_heap().ok_or_else(|| IngestError::EngineError(format!(
            "bundle `{target_bundle}` is not heap-resident; INGEST into mmap bundles is deferred"
        )))?;
        let schema = &store.schema;
        let mut existing_fields: HashSet<String> = HashSet::new();
        for f in &schema.base_fields {
            existing_fields.insert(f.name.clone());
        }
        for f in &schema.fiber_fields {
            existing_fields.insert(f.name.clone());
        }
        for inf in inferred {
            if !existing_fields.contains(&inf.name) {
                return Err(IngestError::SchemaConflict {
                    bundle: target_bundle.to_string(),
                    field: inf.name.clone(),
                    existing: "missing".to_string(),
                    incoming: inf.type_label(),
                });
            }
            // Type-compat check: find the field in the existing
            // schema and confirm the type matches the inferred kind.
            let existing_type = schema
                .base_fields
                .iter()
                .chain(schema.fiber_fields.iter())
                .find(|f| f.name == inf.name)
                .map(|f| f.field_type.clone());
            if let Some(et) = existing_type {
                if !types_compatible(&et, &inf.field_type) {
                    return Err(IngestError::SchemaConflict {
                        bundle: target_bundle.to_string(),
                        field: inf.name.clone(),
                        existing: format!("{:?}", et),
                        incoming: inf.type_label(),
                    });
                }
            }
        }
        Ok(false)
    } else {
        // Bundle missing: either auto-create or surface the typed
        // TargetBundleNotFound error per `allow_auto_create`.
        if !allow_auto_create {
            return Err(IngestError::TargetBundleNotFound {
                bundle: target_bundle.to_string(),
            });
        }
        // Auto-create from inferred.
        let mut schema = BundleSchema::new(target_bundle);
        for inf in inferred {
            let mut field = match &inf.field_type {
                FieldType::Numeric => FieldDef::numeric(&inf.name),
                FieldType::Categorical => FieldDef::categorical(&inf.name),
                FieldType::Vector { dims } => {
                    let mut f = FieldDef::numeric(&inf.name);
                    f.field_type = FieldType::Vector { dims: *dims };
                    f
                }
                _ => {
                    // Phase 1 doesn't infer Timestamp/Binary/OrderedCat
                    // — coerce to Categorical as a safe default. The
                    // EXPLICIT_SCHEMA path is where these get
                    // user-specified.
                    FieldDef::categorical(&inf.name)
                }
            };
            // Ensure the field carries the inferred type verbatim
            // (defensive against builder defaults).
            field.field_type = inf.field_type.clone();
            if inf.is_base {
                schema = schema.base(field);
            } else {
                schema = schema.fiber(field);
            }
        }
        engine
            .create_bundle(schema)
            .map_err(|e| IngestError::EngineError(format!("create_bundle: {e}")))?;
        Ok(true)
    }
}

/// Test-only entry point that surfaces the `allow_auto_create=false`
/// path so the integration test can observe `TargetBundleNotFound`
/// without changing the public INGEST surface (ergonomics #5,
/// 2026-06-28). The GQL surface always passes `true`.
#[doc(hidden)]
pub fn ensure_bundle_compatible_for_test(
    engine: &mut Engine,
    target_bundle: &str,
    inferred: &[InferredFieldSchema],
    allow_auto_create: bool,
) -> Result<bool, IngestError> {
    ensure_bundle_compatible(engine, target_bundle, inferred, allow_auto_create)
}

/// Loose type compatibility — used when validating that an existing
/// bundle's schema can accept the inferred field set. We require
/// matching `FieldType` variants and, for `Vector`, matching `dims`.
fn types_compatible(existing: &FieldType, inferred: &FieldType) -> bool {
    match (existing, inferred) {
        (FieldType::Numeric, FieldType::Numeric) => true,
        (FieldType::Categorical, FieldType::Categorical) => true,
        (FieldType::Timestamp, FieldType::Timestamp) => true,
        (FieldType::Binary, FieldType::Binary) => true,
        (FieldType::OrderedCat { .. }, FieldType::OrderedCat { .. }) => true,
        (FieldType::Vector { dims: a }, FieldType::Vector { dims: b }) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    //! Module-internal smoke tests — full integration coverage lives
    //! in `tests/ingest_executor.rs`.

    use super::*;

    #[test]
    fn format_from_name_npz_only() {
        assert_eq!(IngestFormat::from_name("NPZ").unwrap(), IngestFormat::Npz);
        assert_eq!(IngestFormat::from_name("npz").unwrap(), IngestFormat::Npz);
        let err = IngestFormat::from_name("CSV").unwrap_err();
        match err {
            IngestError::FormatNotSupported { requested, supported } => {
                assert_eq!(requested, "CSV");
                assert_eq!(supported, vec!["NPZ".to_string()]);
            }
            other => panic!("expected FormatNotSupported, got {other:?}"),
        }
    }

    #[test]
    fn types_compatible_vector_dims_must_match() {
        assert!(types_compatible(
            &FieldType::Vector { dims: 9 },
            &FieldType::Vector { dims: 9 },
        ));
        assert!(!types_compatible(
            &FieldType::Vector { dims: 9 },
            &FieldType::Vector { dims: 10 },
        ));
        assert!(!types_compatible(
            &FieldType::Numeric,
            &FieldType::Vector { dims: 9 },
        ));
    }
}
