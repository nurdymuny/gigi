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
//! | `CSV`  | `.csv`           | Header row names the fields; one record per |
//! |        |                  | data row. Base key = `KEY <col>` or the     |
//! |        |                  | first column. Numeric-unless-proven-        |
//! |        |                  | otherwise column typing; quoted fields and  |
//! |        |                  | embedded commas via `dhoom::csv_to_records`.|
//!
//! HDF5 and JSONL are deferred. JSON already has readers in
//! `src/convert.rs` that a later sprint can wire through the same
//! `IngestFormat` extension point CSV used.
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

/// Decode an `NpyFile` as a flat `Vec<f64>`, auto-detecting the on-disk
/// dtype from the `.npy` header.
///
/// - `float64` (`<f8` / `>f8` / `|f8`) is read directly as `f64`.
/// - `float32` (`<f4` / `>f4` / `|f4`) is read as `f32` and cast to `f64`
///   element-wise. The cast is mathematically lossless: every finite
///   `f32` has an exact `f64` representation (24-bit mantissa fits inside
///   f64's 53-bit mantissa; f32's exponent range is a strict subset of
///   f64's).
/// - Any other dtype returns [`IngestError::FormatError`] naming the
///   observed dtype string AND the supported set (`float32`, `float64`)
///   so the caller can adjust their pipeline.
///
/// `array_name` is threaded through error messages purely for
/// disambiguation when the reader is called inside a loop over
/// multi-member archives; it has no semantic effect.
fn decode_npy_as_f64<R: io::Read>(
    entry: npyz::NpyFile<R>,
    array_name: &str,
) -> Result<Vec<f64>, IngestError> {
    let dtype = entry.dtype();
    match &dtype {
        npyz::DType::Plain(ts) => match ts.type_char() {
            npyz::TypeChar::Float => match ts.size_field() {
                8 => entry.into_vec::<f64>().map_err(|e| {
                    IngestError::FormatError(format!(
                        "decode array `{array_name}` as f64: {e}"
                    ))
                }),
                4 => {
                    let raw = entry.into_vec::<f32>().map_err(|e| {
                        IngestError::FormatError(format!(
                            "decode array `{array_name}` as f32: {e}"
                        ))
                    })?;
                    Ok(raw.into_iter().map(|x| x as f64).collect())
                }
                other => Err(IngestError::FormatError(format!(
                    "INGEST NPZ: array `{array_name}` has unsupported dtype `{ts}` \
                     (float width {other} bytes); expected float32 or float64"
                ))),
            },
            _ => Err(IngestError::FormatError(format!(
                "INGEST NPZ: array `{array_name}` has unsupported dtype `{ts}`; \
                 expected float32 or float64"
            ))),
        },
        other => Err(IngestError::FormatError(format!(
            "INGEST NPZ: array `{array_name}` has non-scalar dtype `{}`; \
             expected float32 or float64",
            other.descr()
        ))),
    }
}

/// Format-specific entry points. Phase 1 ships NPZ only; HDF5/JSONL
/// land as additional variants in a follow-up sprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestFormat {
    /// NumPy zip archive (`.npz`).
    Npz,
    /// Comma-separated values with a header row (`.csv`). One record
    /// per data row; the KEY clause (or, absent one, the FIRST column)
    /// names the base key; column types are inferred from the data.
    Csv,
    /// Newline-delimited JSON (`.jsonl` / `.ndjson`): one object per
    /// line. Same key/typing policy as CSV — KEY names the base column,
    /// else the first key of the first object; numeric unless proven
    /// otherwise.
    Jsonl,
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
            "CSV" => Ok(IngestFormat::Csv),
            "JSONL" | "NDJSON" => Ok(IngestFormat::Jsonl),
            other => Err(IngestError::FormatNotSupported {
                requested: other.to_string(),
                supported: vec![
                    "NPZ".to_string(),
                    "CSV".to_string(),
                    "JSONL".to_string(),
                ],
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
    // ── GAUGE_FIELD interpretation errors (feature = "gauge") ──
    /// The named lattice was not found in `lattice::registry`.
    /// Surfaced by the GAUGE_FIELD interpretation path.
    LatticeNotFound { name: String },
    /// The innermost NPZ axis width does not match `Group::repr_dim()`.
    /// `expected` is `group.repr_dim()`; `got` is `shape[shape.len()-1]`.
    FiberWidthMismatch {
        group: &'static str,
        expected: usize,
        got: usize,
    },
    /// The array's rank does not match `1 (config) + 1 (mu) + D (sites)
    /// + 1 (fiber)` for a lattice of dimension `D`.
    AxisCountMismatch {
        expected_ndim: usize,
        got_ndim: usize,
        lattice_dim: usize,
    },
    /// A site-axis extent in the array does not equal the reference L
    /// (Phase 1 CUBIC lattices are L-uniform; a non-uniform site axis
    /// is the clearest error to surface). `axis_index` is 2..2+D
    /// (0-based on the NPZ shape).
    SiteAxisExtentMismatch {
        axis_index: usize,
        expected_l: usize,
        got: u64,
    },
    /// The mu (direction) axis extent does not equal `lattice.dim`.
    DirectionAxisMismatch { expected_d: usize, got: u64 },
    /// NPZ archive contains more than one array member — the
    /// GAUGE_FIELD interpretation requires a single named-array NPZ
    /// (no multi-member archive).
    MultiArrayNotAllowedForGaugeField { got: usize },
    /// NPZ archive holds more than one member and the caller did not
    /// pass a `KEY <name>` clause selecting exactly one. Emitted from
    /// the generic and GAUGE_FIELD paths alike so multi-array archives
    /// never silently promote all members. `members` is preserved so
    /// the display can name the exact array names the caller must
    /// choose between.
    MultiArrayRequiresKey { got: usize, members: Vec<String> },
    /// The caller passed `KEY <name>` but no member of the archive
    /// carries that name. `members` lists the actual archive member
    /// names so the caller can correct the KEY in one shot.
    KeyNotInArchive { requested: String, members: Vec<String> },
    /// `INGEST … FORMAT CSV KEY <col>` named a column the header row
    /// does not contain.
    KeyNotInCsv { requested: String, columns: Vec<String> },
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
            // ── GAUGE_FIELD interpretation errors (feature = "gauge") ──
            IngestError::LatticeNotFound { name } => write!(
                f,
                "INGEST: lattice `{name}` not found — declare it with LATTICE ... FROM CUBIC ...; before INGEST"
            ),
            IngestError::FiberWidthMismatch { group, expected, got } => write!(
                f,
                "INGEST: GAUGE_FIELD width mismatch for group `{group}`: expected innermost axis of width {expected}, got {got}"
            ),
            IngestError::AxisCountMismatch { expected_ndim, got_ndim, lattice_dim } => write!(
                f,
                "INGEST: GAUGE_FIELD axis-count mismatch: lattice dim={lattice_dim} implies array ndim = 1 + 1 + {lattice_dim} + 1 = {expected_ndim}, got ndim={got_ndim}"
            ),
            IngestError::SiteAxisExtentMismatch { axis_index, expected_l, got } => write!(
                f,
                "INGEST: GAUGE_FIELD site-axis {axis_index} extent mismatch: expected L={expected_l}, got {got}"
            ),
            IngestError::DirectionAxisMismatch { expected_d, got } => write!(
                f,
                "INGEST: GAUGE_FIELD direction-axis extent mismatch: expected D={expected_d}, got {got}"
            ),
            IngestError::MultiArrayNotAllowedForGaugeField { got } => write!(
                f,
                "INGEST: GAUGE_FIELD interpretation requires a single-array NPZ; archive contained {got} members — add KEY <name> to select one"
            ),
            IngestError::MultiArrayRequiresKey { got, members } => write!(
                f,
                "INGEST NPZ: archive contains {got} arrays; add KEY <name> clause to select one — available: [{}]",
                members.join(", ")
            ),
            IngestError::KeyNotInArchive { requested, members } => write!(
                f,
                "INGEST NPZ: KEY '{requested}' not in archive — available: [{}]",
                members.join(", ")
            ),
            IngestError::KeyNotInCsv { requested, columns } => write!(
                f,
                "INGEST CSV: KEY '{requested}' is not a column of the CSV — \
                 header row has: [{}]",
                columns.join(", ")
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
///
/// `key` selects a single named member of a multi-array NPZ archive.
/// When `Some(name)`, only that member's slices become records.
/// When `None` and the archive holds more than one member the executor
/// errors with `MultiArrayRequiresKey` naming every available member
/// so the caller can add exactly the KEY clause they omitted.
/// Backwards compat: single-member archives ignore `key = None`.
pub fn execute_ingest(
    engine: &mut Engine,
    target_bundle: &str,
    source_path: &Path,
    format: IngestFormat,
    key: Option<&str>,
) -> Result<IngestStats, IngestError> {
    if !source_path.exists() {
        return Err(IngestError::FileNotFound(source_path.to_path_buf()));
    }
    let bytes_read = std::fs::metadata(source_path)
        .map(|m| m.len())
        .unwrap_or(0);

    match format {
        IngestFormat::Npz => ingest_npz(engine, target_bundle, source_path, bytes_read, key),
        IngestFormat::Csv => ingest_csv(engine, target_bundle, source_path, bytes_read, key),
        IngestFormat::Jsonl => ingest_jsonl(engine, target_bundle, source_path, bytes_read, key),
    }
}

/// NPZ-specific reader: opens the archive, enumerates `.npy` members,
/// and emits records per member's outermost axis. `key` restricts
/// ingestion to a single named member; when `None` on a multi-member
/// archive the reader errors with `MultiArrayRequiresKey`.
fn ingest_npz(
    engine: &mut Engine,
    target_bundle: &str,
    source_path: &Path,
    bytes_read: u64,
    key: Option<&str>,
) -> Result<IngestStats, IngestError> {
    let mut archive = npyz::npz::NpzArchive::open(source_path)
        .map_err(|e| IngestError::FormatError(format!("open NPZ {}: {e}", source_path.display())))?;

    let all_names: Vec<String> = archive.array_names().map(|s| s.to_string()).collect();
    if all_names.is_empty() {
        return Err(IngestError::EmptyArchive(source_path.to_path_buf()));
    }

    // Resolve which member names actually feed the ingest. When `key`
    // is `Some(name)`, restrict to that single member (error if absent).
    // When `key` is `None`, require single-member (else surface
    // `MultiArrayRequiresKey` so the caller sees exactly which members
    // they must choose between).
    let array_names: Vec<String> = match key {
        Some(k) => {
            if all_names.iter().any(|n| n == k) {
                vec![k.to_string()]
            } else {
                return Err(IngestError::KeyNotInArchive {
                    requested: k.to_string(),
                    members: all_names,
                });
            }
        }
        None => {
            if all_names.len() > 1 {
                return Err(IngestError::MultiArrayRequiresKey {
                    got: all_names.len(),
                    members: all_names,
                });
            }
            all_names.clone()
        }
    };

    // Every archive that reaches this point contributes a single
    // member (KEY-selected or the sole member of a single-array
    // archive). `multi_array` is retained so the schema-inference tail
    // stays reachable if the KEY policy is loosened later, but is
    // currently always false.
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
        let data: Vec<f64> = decode_npy_as_f64(entry, name)?;
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


/// CSV entry point — one record per data row.
///
/// Policy (documented in GQL_REFERENCE.md):
/// - The header row names the fields, in order.
/// - The base key is the `KEY <col>` clause if present, else the FIRST
///   column. An empty base-key cell is a loud per-row error.
/// - Column types are inferred from the data: a column whose every
///   non-empty value is numeric becomes Numeric; everything else
///   becomes Categorical (numbers in a mixed column are stored as
///   their text form, so the column stays one type).
/// - Reuses `crate::dhoom::csv_to_records` (quoted fields, type
///   coercion) — one CSV dialect across the whole engine.
fn ingest_csv(
    engine: &mut Engine,
    target_bundle: &str,
    source_path: &Path,
    bytes_read: u64,
    key: Option<&str>,
) -> Result<IngestStats, IngestError> {
    let text = std::fs::read_to_string(source_path).map_err(|e| {
        IngestError::FormatError(format!("read CSV {}: {e}", source_path.display()))
    })?;
    let (rows, columns) =
        crate::dhoom::csv_to_records(&text).map_err(IngestError::FormatError)?;
    if columns.is_empty() || columns.iter().all(|c| c.is_empty()) {
        return Err(IngestError::FormatError(
            "CSV has no usable header row — the first line must name the fields"
                .to_string(),
        ));
    }
    if rows.is_empty() {
        return Err(IngestError::FormatError(
            "CSV has a header but no data rows — nothing to ingest".to_string(),
        ));
    }

    let base_col = match key {
        Some(k) => {
            if columns.iter().any(|c| c == k) {
                k.to_string()
            } else {
                return Err(IngestError::KeyNotInCsv {
                    requested: k.to_string(),
                    columns,
                });
            }
        }
        None => columns[0].clone(),
    };

    // Column type inference: numeric unless a non-empty, non-numeric
    // value shows up. Empty cells and nulls don't vote.
    let mut is_numeric: Vec<bool> = vec![true; columns.len()];
    for row in &rows {
        let Some(obj) = row.as_object() else { continue };
        for (ci, col) in columns.iter().enumerate() {
            match obj.get(col) {
                None | Some(serde_json::Value::Null) => {}
                Some(serde_json::Value::Number(_)) => {}
                Some(serde_json::Value::String(sv)) if sv.is_empty() => {}
                Some(_) => is_numeric[ci] = false,
            }
        }
    }

    let inferred: Vec<InferredFieldSchema> = columns
        .iter()
        .enumerate()
        .map(|(ci, c)| InferredFieldSchema {
            name: c.clone(),
            field_type: if is_numeric[ci] {
                FieldType::Numeric
            } else {
                FieldType::Categorical
            },
            is_base: *c == base_col,
        })
        .collect();
    let bundle_created =
        ensure_bundle_compatible(engine, target_bundle, &inferred, /*allow_auto_create=*/ true)?;

    let mut records_emitted: usize = 0;
    let mut batch: Vec<Record> = Vec::with_capacity(INGEST_BATCH_SIZE.min(64));
    for (ri, row) in rows.iter().enumerate() {
        let obj = row.as_object().ok_or_else(|| {
            IngestError::FormatError(format!("CSV row {} did not parse as a record", ri + 1))
        })?;
        let mut rec: Record = Record::new();
        for (ci, col) in columns.iter().enumerate() {
            let jv = obj.get(col).unwrap_or(&serde_json::Value::Null);
            let val = if is_numeric[ci] {
                match jv {
                    serde_json::Value::Number(n) => match n.as_i64() {
                        Some(i) => Value::Integer(i),
                        None => Value::Float(n.as_f64().unwrap_or(f64::NAN)),
                    },
                    _ => Value::Null,
                }
            } else {
                match jv {
                    serde_json::Value::Null => Value::Null,
                    serde_json::Value::String(sv) if sv.is_empty() => Value::Null,
                    serde_json::Value::String(sv) => Value::Text(sv.clone()),
                    serde_json::Value::Bool(b) => Value::Text(b.to_string()),
                    serde_json::Value::Number(n) => Value::Text(n.to_string()),
                    other => Value::Text(other.to_string()),
                }
            };
            if *col == base_col && matches!(val, Value::Null) {
                return Err(IngestError::FormatError(format!(
                    "CSV row {}: base-key column '{}' is empty — every record \
                     needs an address",
                    ri + 1,
                    base_col
                )));
            }
            rec.insert(col.clone(), val);
        }
        batch.push(rec);
        if batch.len() >= INGEST_BATCH_SIZE {
            flush_batch(engine, target_bundle, &mut batch, &mut records_emitted)?;
        }
    }
    if !batch.is_empty() {
        flush_batch(engine, target_bundle, &mut batch, &mut records_emitted)?;
    }

    Ok(IngestStats { records_emitted, bundle_created, bytes_read })
}


/// JSONL entry point — one record per line, one JSON object per record.
///
/// Policy:
/// - `KEY <col>` is REQUIRED: JSON objects carry no reliable column
///   order, so "the first column" is not a thing we can promise.
/// - Types are inferred from the data with JSON's own types: a column
///   of numbers is Numeric, strings/bools are Categorical (numbers in
///   a mixed column are stored as text), and an array of numbers is a
///   Vector fiber — consistent length required, so embeddings ingest
///   as first-class vectors.
/// - Loud errors: missing/invalid JSON on a line (line number named),
///   KEY absent from an object, vector length drift, nested objects.
fn ingest_jsonl(
    engine: &mut Engine,
    target_bundle: &str,
    source_path: &Path,
    bytes_read: u64,
    key: Option<&str>,
) -> Result<IngestStats, IngestError> {
    let Some(base_col) = key else {
        return Err(IngestError::FormatError(
            "INGEST … FORMAT JSONL requires KEY <column>: JSON objects have \
             no reliable column order, so the base key must be named"
                .to_string(),
        ));
    };
    let text = std::fs::read_to_string(source_path).map_err(|e| {
        IngestError::FormatError(format!("read JSONL {}: {e}", source_path.display()))
    })?;

    #[derive(Clone, Copy, PartialEq, Debug)]
    enum Kind {
        Unknown,
        Numeric,
        Categorical,
        Vector(usize),
    }

    let mut objs: Vec<(usize, serde_json::Map<String, serde_json::Value>)> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line).map_err(|e| {
            IngestError::FormatError(format!("JSONL line {}: invalid JSON: {e}", i + 1))
        })?;
        match v {
            serde_json::Value::Object(m) => objs.push((i + 1, m)),
            other => {
                return Err(IngestError::FormatError(format!(
                    "JSONL line {}: expected an object, got {other}",
                    i + 1
                )))
            }
        }
    }
    if objs.is_empty() {
        return Err(IngestError::FormatError(
            "JSONL has no records — nothing to ingest".to_string(),
        ));
    }
    if !objs.iter().any(|(_, m)| m.contains_key(base_col)) {
        let mut cols: Vec<&str> =
            objs.iter().flat_map(|(_, m)| m.keys().map(|k| k.as_str())).collect();
        cols.sort_unstable();
        cols.dedup();
        return Err(IngestError::KeyNotInCsv {
            requested: base_col.to_string(),
            columns: cols.into_iter().map(str::to_string).collect(),
        });
    }

    // Column discovery + type inference across every object.
    let mut kinds: std::collections::BTreeMap<String, Kind> =
        std::collections::BTreeMap::new();
    for (line_no, m) in &objs {
        for (k, v) in m {
            let entry = kinds.entry(k.clone()).or_insert(Kind::Unknown);
            let seen = match v {
                serde_json::Value::Null => continue,
                serde_json::Value::Number(_) => Kind::Numeric,
                serde_json::Value::String(_) | serde_json::Value::Bool(_) => {
                    Kind::Categorical
                }
                serde_json::Value::Array(a) => {
                    if a.iter().all(|x| x.is_number()) {
                        Kind::Vector(a.len())
                    } else {
                        return Err(IngestError::FormatError(format!(
                            "JSONL line {line_no}: field '{k}' is an array with \
                             non-numeric elements — only numeric vectors ingest"
                        )));
                    }
                }
                serde_json::Value::Object(_) => {
                    return Err(IngestError::FormatError(format!(
                        "JSONL line {line_no}: field '{k}' is a nested object — \
                         GIGI fibers are flat; flatten before ingest"
                    )))
                }
            };
            *entry = match (*entry, seen) {
                (Kind::Unknown, s) => s,
                (k, Kind::Unknown) => k, // unreachable: nulls `continue` above
                (Kind::Numeric, Kind::Numeric) => Kind::Numeric,
                (Kind::Categorical, Kind::Numeric)
                | (Kind::Numeric, Kind::Categorical)
                | (Kind::Categorical, Kind::Categorical) => Kind::Categorical,
                (Kind::Vector(d1), Kind::Vector(d2)) if d1 == d2 => Kind::Vector(d1),
                (Kind::Vector(d1), Kind::Vector(d2)) => {
                    return Err(IngestError::FormatError(format!(
                        "JSONL line {line_no}: vector field '{k}' changed length \
                         ({d1} then {d2}) — vector fibers have one declared dim"
                    )))
                }
                (Kind::Vector(_), _) | (_, Kind::Vector(_)) => {
                    return Err(IngestError::FormatError(format!(
                        "JSONL line {line_no}: field '{k}' mixes vectors and \
                         scalars — pick one"
                    )))
                }
            };
        }
    }

    let inferred: Vec<InferredFieldSchema> = kinds
        .iter()
        .map(|(name, kind)| InferredFieldSchema {
            name: name.clone(),
            field_type: match kind {
                Kind::Numeric => FieldType::Numeric,
                Kind::Vector(d) => FieldType::Vector { dims: *d },
                _ => FieldType::Categorical,
            },
            is_base: name == base_col,
        })
        .collect();
    let bundle_created =
        ensure_bundle_compatible(engine, target_bundle, &inferred, /*allow_auto_create=*/ true)?;

    let mut records_emitted: usize = 0;
    let mut batch: Vec<Record> = Vec::with_capacity(INGEST_BATCH_SIZE.min(64));
    for (line_no, m) in &objs {
        let mut rec: Record = Record::new();
        for (k, kind) in &kinds {
            let jv = m.get(k).unwrap_or(&serde_json::Value::Null);
            let val = match (kind, jv) {
                (_, serde_json::Value::Null) => Value::Null,
                (Kind::Numeric, serde_json::Value::Number(n)) => match n.as_i64() {
                    Some(i) => Value::Integer(i),
                    None => Value::Float(n.as_f64().unwrap_or(f64::NAN)),
                },
                (Kind::Vector(_), serde_json::Value::Array(a)) => Value::Vector(
                    a.iter().map(|x| x.as_f64().unwrap_or(f64::NAN)).collect(),
                ),
                (Kind::Categorical, serde_json::Value::String(sv)) => {
                    Value::Text(sv.clone())
                }
                (Kind::Categorical, serde_json::Value::Bool(b)) => {
                    Value::Text(b.to_string())
                }
                (Kind::Categorical, serde_json::Value::Number(n)) => {
                    Value::Text(n.to_string())
                }
                _ => Value::Null,
            };
            if k == base_col && matches!(val, Value::Null) {
                return Err(IngestError::FormatError(format!(
                    "JSONL line {line_no}: base-key field '{base_col}' is \
                     null/absent — every record needs an address"
                )));
            }
            rec.insert(k.clone(), val);
        }
        batch.push(rec);
        if batch.len() >= INGEST_BATCH_SIZE {
            flush_batch(engine, target_bundle, &mut batch, &mut records_emitted)?;
        }
    }
    if !batch.is_empty() {
        flush_batch(engine, target_bundle, &mut batch, &mut records_emitted)?;
    }

    Ok(IngestStats { records_emitted, bundle_created, bytes_read })
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

// ─── GAUGE_FIELD interpretation for INGEST (feature = "gauge") ───
//
// INGEST ... AS GAUGE_FIELD GROUP <g> ON LATTICE <l>
//
// Interpretation clause that turns a harvest NPZ (shape
// `(n_configs, D, L, L, ..., L, repr_dim)`) into a bundle whose
// records carry canonical base fields (config_id, mu, site_x/y/z/t)
// and canonical fiber fields per group (SU(2)=q0..q3,
// SU(3)=re_00..im_22, U(1)=theta, Z(N)=index). Each fiber component is
// its OWN Numeric column so SPECTRAL_GAUGE ON FIBER (q0, q1, q2, q3)
// can read them directly.
//
// Emits one record per (config_id, mu, site) tuple. Total records =
// n_configs * D * L^D.

/// Canonical fiber column names for `Group::SU2` — scalar-first
/// quaternion, matches `src/gauge/su2_gauge_field.rs`.
#[cfg(feature = "gauge")]
pub const SU2_FIBER_NAMES: [&str; 4] = ["q0", "q1", "q2", "q3"];

/// Canonical fiber column names for `Group::SU3` — 3x3 complex matrix
/// flattened row-major with interleaved (re, im) pairs, matches
/// `Group::SU3` doc at `src/gauge/group.rs`.
#[cfg(feature = "gauge")]
pub const SU3_FIBER_NAMES: [&str; 18] = [
    "re_00", "im_00", "re_01", "im_01", "re_02", "im_02",
    "re_10", "im_10", "re_11", "im_11", "re_12", "im_12",
    "re_20", "im_20", "re_21", "im_21", "re_22", "im_22",
];

/// Canonical fiber column name for `Group::U1` — single angle.
#[cfg(feature = "gauge")]
pub const U1_FIBER_NAMES: [&str; 1] = ["theta"];

/// Canonical fiber column name for `Group::ZN { .. }` — discrete
/// index carried as f64 (per `Group::repr_dim()` note).
#[cfg(feature = "gauge")]
pub const ZN_FIBER_NAMES: [&str; 1] = ["index"];

/// Canonical site-axis names, truncated to the lattice dimension.
/// Full list is `[site_x, site_y, site_z, site_t]`; Phase 1 lattice
/// dims are 1..=4 so a `&SITE_AXIS_NAMES[..D]` slice is always valid.
pub const SITE_AXIS_NAMES: [&str; 4] = ["site_x", "site_y", "site_z", "site_t"];

/// Return the canonical fiber-name slice for a group. Length always
/// equals `group.repr_dim()`. Available under `feature = "gauge"`.
#[cfg(feature = "gauge")]
pub fn canonical_fiber_names(group: crate::gauge::Group) -> &'static [&'static str] {
    match group {
        crate::gauge::Group::SU2 => &SU2_FIBER_NAMES,
        crate::gauge::Group::SU3 => &SU3_FIBER_NAMES,
        crate::gauge::Group::U1 => &U1_FIBER_NAMES,
        crate::gauge::Group::ZN { .. } => &ZN_FIBER_NAMES,
    }
}

/// Recover the lattice dimension `D` from a `Lattice`'s topology hint.
///
/// The CUBIC constructor stamps `"CUBIC_L{L}_D{D}"` (or
/// `"..._OPEN"` for open BCs) on `lattice.topology`. Parsing D back
/// out lets the GAUGE_FIELD interpretation path route the NPZ axes
/// without adding a new `dim` field to the `Lattice` struct. If the
/// topology hint isn't a CUBIC form or D can't be parsed, returns
/// `None` and the caller surfaces a `LatticeNotFound`-style error.
///
/// Strip any recognized OBC / OPEN suffix so the caller can parse the
/// core `CUBIC_L{L}_D{D}` shape uniformly. Recognized suffixes:
///   `_OPEN`             — fully-open (Phase 2 deferred)
///   `_OBC_AXIS{k}`      — single-axis open boundary
#[cfg(feature = "gauge")]
fn strip_obc_suffix(topology: &str) -> &str {
    // Fully-open suffix.
    if let Some(rest) = topology.strip_suffix("_OPEN") {
        return rest;
    }
    // Single-axis OBC suffix `_OBC_AXIS{k}` — locate the marker then
    // check the tail is all digits.
    if let Some(idx) = topology.rfind("_OBC_AXIS") {
        let tail = &topology[idx + "_OBC_AXIS".len()..];
        if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) {
            return &topology[..idx];
        }
    }
    topology
}

/// Part of the GAUGE_FIELD interpretation path (feature = "gauge").
#[cfg(feature = "gauge")]
fn cubic_dim_from_topology(topology: &str) -> Option<usize> {
    let stripped = strip_obc_suffix(topology);
    // Expect prefix "CUBIC_L{L}_D{D}"; parse D.
    if !stripped.starts_with("CUBIC_L") {
        return None;
    }
    // Locate the "_D" segment after the "L{L}".
    let idx = stripped.find("_D")?;
    let d_tail = &stripped[idx + 2..];
    d_tail.parse::<usize>().ok()
}

/// Recover the per-axis vertex count `L` from a `Lattice`'s topology
/// hint (same convention as `cubic_dim_from_topology`).
#[cfg(feature = "gauge")]
fn cubic_l_from_topology(topology: &str) -> Option<usize> {
    let stripped = strip_obc_suffix(topology);
    if !stripped.starts_with("CUBIC_L") {
        return None;
    }
    // Extract characters between "CUBIC_L" and "_D".
    let after_l = &stripped["CUBIC_L".len()..];
    let end = after_l.find("_D")?;
    after_l[..end].parse::<usize>().ok()
}

/// Recover the OBC axis index `k` from a `Lattice`'s topology hint.
/// Returns `Some(k)` when the hint carries `_OBC_AXIS{k}` and `None`
/// otherwise (PERIODIC). The axis index is used by the GAUGE_FIELD
/// ingest path to omit records whose (mu, coords) would wrap across
/// the open boundary.
#[cfg(feature = "gauge")]
fn obc_axis_from_topology(topology: &str) -> Option<usize> {
    let idx = topology.rfind("_OBC_AXIS")?;
    let tail = &topology[idx + "_OBC_AXIS".len()..];
    if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) {
        tail.parse::<usize>().ok()
    } else {
        None
    }
}

/// Column-major site encoding used by the GAUGE_FIELD ingest emitter:
/// `site_of(&[c0, c1, ..., c_{D-1}], L) = sum_k c_k * L^k`, i.e. `c0`
/// (site_x) is least-significant. Matches `Lattice::VertexId`
/// numbering for the cubic constructor, so the `vertex_a` / `vertex_b`
/// integer values stamped on each ingested record equal
/// `lattice.site_of(coords)` for the same coords and L. Thin wrapper
/// around `crate::lattice::topology::site_of_column_major`, kept as a
/// module-local name so callsites inside `ingest_npz_as_gauge_field`
/// stay short.
#[cfg(feature = "gauge")]
fn site_of_column_major(coords: &[usize], l: usize) -> usize {
    crate::lattice::topology::site_of_column_major(coords, l)
}

/// Shift-by-+1 along axis `a`, modulo `L`. Callers detect
/// wrap edges by comparing the original `coords[a] == L - 1` before
/// invoking this helper; the modulo here is what PERIODIC lattices
/// consume unchanged.
#[cfg(feature = "gauge")]
fn shift_plus(coords: &[usize], a: usize, l: usize) -> Vec<usize> {
    let mut out = coords.to_vec();
    out[a] = (out[a] + 1) % l;
    out
}

/// GAUGE_FIELD-interpretation INGEST. Called by the parser when
/// `AS GAUGE_FIELD GROUP <g> ON LATTICE <l>` is present on the INGEST
/// statement. Reads a single NPZ array of shape
/// `(n_configs, D, L, L, ..., L, group.repr_dim())` and emits one
/// record per (config_id, mu, site_tuple) point with canonical
/// base + fiber fields.
///
/// Backwards compat: this is a distinct entry point; the AUTO_GENERIC
/// `execute_ingest` path is untouched.
///
/// GAUGE_FIELD interpretation for INGEST (feature = "gauge").
///
/// `key` selects a single NPZ member for interpretation. When `None`
/// on a multi-member archive, `MultiArrayNotAllowedForGaugeField`
/// fires — canonical GAUGE_FIELD schema still requires one U-array
/// per file. Backwards compat is preserved: single-member archives
/// still work when `key = None`.
#[cfg(feature = "gauge")]
pub fn execute_ingest_as_gauge_field(
    engine: &mut Engine,
    target_bundle: &str,
    source_path: &Path,
    format: IngestFormat,
    group: crate::gauge::Group,
    lattice_name: &str,
    key: Option<&str>,
) -> Result<IngestStats, IngestError> {
    if !source_path.exists() {
        return Err(IngestError::FileNotFound(source_path.to_path_buf()));
    }
    let bytes_read = std::fs::metadata(source_path).map(|m| m.len()).unwrap_or(0);

    // NPZ member selection precedes lattice resolution: a caller who
    // forgot KEY on a multi-array archive should see the KEY-remedy
    // message even when the lattice name is bogus, so the fix is one
    // edit rather than two round trips.
    match format {
        IngestFormat::Csv | IngestFormat::Jsonl => {
            return Err(IngestError::FormatError(
                "INGEST … AS GAUGE_FIELD requires FORMAT NPZ — a gauge \
                 field is a shaped array, and CSV/JSONL carry no shape"
                    .to_string(),
            ));
        }
        IngestFormat::Npz => {
            let archive = npyz::npz::NpzArchive::open(source_path)
                .map_err(|e| IngestError::FormatError(format!(
                    "open NPZ {}: {e}", source_path.display()
                )))?;
            let all_names: Vec<String> = archive.array_names().map(|s| s.to_string()).collect();
            if all_names.is_empty() {
                return Err(IngestError::EmptyArchive(source_path.to_path_buf()));
            }
            match key {
                Some(k) => {
                    if !all_names.iter().any(|n| n == k) {
                        return Err(IngestError::KeyNotInArchive {
                            requested: k.to_string(),
                            members: all_names,
                        });
                    }
                }
                None => {
                    if all_names.len() != 1 {
                        return Err(IngestError::MultiArrayNotAllowedForGaugeField {
                            got: all_names.len(),
                        });
                    }
                }
            }
        }
    }

    // Resolve lattice — its `D` (recovered from the topology hint)
    // drives every shape assertion below.
    let lattice = crate::lattice::registry::get(lattice_name).ok_or_else(|| {
        IngestError::LatticeNotFound { name: lattice_name.to_string() }
    })?;
    let topology = lattice.topology.as_deref().ok_or_else(|| IngestError::EngineError(
        format!("lattice `{lattice_name}` carries no topology hint; GAUGE_FIELD interpretation requires a CUBIC lattice")
    ))?;
    let d = cubic_dim_from_topology(topology).ok_or_else(|| IngestError::EngineError(format!(
        "lattice `{lattice_name}` topology hint `{topology}` is not a CUBIC form; GAUGE_FIELD interpretation requires CUBIC"
    )))?;
    let l_from_hint = cubic_l_from_topology(topology);
    let obc_axis = obc_axis_from_topology(topology);

    match format {
        IngestFormat::Csv | IngestFormat::Jsonl => {
            unreachable!("rejected above before lattice resolution")
        }
        IngestFormat::Npz => ingest_npz_as_gauge_field(
            engine,
            target_bundle,
            source_path,
            bytes_read,
            group,
            d,
            l_from_hint,
            obc_axis,
            key,
        ),
    }
}

/// GAUGE_FIELD-interpretation NPZ reader. See
/// `execute_ingest_as_gauge_field` for surface semantics.
#[cfg(feature = "gauge")]
fn ingest_npz_as_gauge_field(
    engine: &mut Engine,
    target_bundle: &str,
    source_path: &Path,
    bytes_read: u64,
    group: crate::gauge::Group,
    d: usize,
    l_from_hint: Option<usize>,
    obc_axis: Option<usize>,
    key: Option<&str>,
) -> Result<IngestStats, IngestError> {
    // Open the archive. Selection policy:
    //  - `key = Some(name)`: use exactly that member (error if absent).
    //  - `key = None`      : require a single-member archive (else
    //                        `MultiArrayNotAllowedForGaugeField` fires
    //                        with the observed member count).
    let mut archive = npyz::npz::NpzArchive::open(source_path)
        .map_err(|e| IngestError::FormatError(format!(
            "open NPZ {}: {e}", source_path.display()
        )))?;
    let all_names: Vec<String> = archive.array_names().map(|s| s.to_string()).collect();
    if all_names.is_empty() {
        return Err(IngestError::EmptyArchive(source_path.to_path_buf()));
    }
    let selected_name: String = match key {
        Some(k) => {
            if all_names.iter().any(|n| n == k) {
                k.to_string()
            } else {
                return Err(IngestError::KeyNotInArchive {
                    requested: k.to_string(),
                    members: all_names,
                });
            }
        }
        None => {
            if all_names.len() != 1 {
                return Err(IngestError::MultiArrayNotAllowedForGaugeField {
                    got: all_names.len(),
                });
            }
            all_names[0].clone()
        }
    };
    let entry = archive
        .by_name(&selected_name)
        .map_err(|e| IngestError::FormatError(format!("read array `{}`: {e}", selected_name)))?
        .ok_or_else(|| IngestError::FormatError(format!(
            "NPZ member `{}` listed but not retrievable", selected_name
        )))?;
    let shape: Vec<u64> = entry.shape().to_vec();

    // Shape validation. Expected ndim = 1 (config) + 1 (mu) + D (sites) + 1 (fiber).
    let expected_ndim = 1 + 1 + d + 1;
    if shape.len() != expected_ndim {
        return Err(IngestError::AxisCountMismatch {
            expected_ndim,
            got_ndim: shape.len(),
            lattice_dim: d,
        });
    }
    // Direction axis extent equals D.
    if shape[1] != d as u64 {
        return Err(IngestError::DirectionAxisMismatch {
            expected_d: d,
            got: shape[1],
        });
    }
    // Per-axis L uniformity. The reference L is taken from the
    // topology hint when available; otherwise from the first site axis.
    let l_ref = l_from_hint.unwrap_or(shape[2] as usize);
    for (i, axis_ext) in shape[2..2 + d].iter().enumerate() {
        if *axis_ext as usize != l_ref {
            return Err(IngestError::SiteAxisExtentMismatch {
                axis_index: 2 + i,
                expected_l: l_ref,
                got: *axis_ext,
            });
        }
    }
    // Fiber width = group.repr_dim().
    let expected_fiber = group.repr_dim();
    let got_fiber = *shape.last().unwrap() as usize;
    if got_fiber != expected_fiber {
        return Err(IngestError::FiberWidthMismatch {
            group: group.label(),
            expected: expected_fiber,
            got: got_fiber,
        });
    }

    // Decode as flat Vec<f64> in row-major (NPZ default). Total len
    // = n_configs * D * L^D * repr_dim. On-disk dtype is auto-detected
    // (f32 upconverts to f64 element-wise; f64 reads as-is); any other
    // dtype errors with the dtype name and the supported set.
    let data: Vec<f64> = decode_npy_as_f64(entry, &selected_name)?;
    let n_configs = shape[0] as usize;
    let expected_len = n_configs * d * l_ref.pow(d as u32) * expected_fiber;
    if data.len() != expected_len {
        return Err(IngestError::FormatError(format!(
            "GAUGE_FIELD data length {} != product {}", data.len(), expected_len
        )));
    }

    // Build inferred schema — canonical base + fiber fields.
    // Base fields carry (config_id, mu, site_*..., vertex_a, vertex_b).
    // The vertex_a / vertex_b endpoints are computed per record with
    // the lattice's column-major site encoding (`stride[k] = L^k`),
    // matching `Lattice::VertexId` numbering. This gives SPECTRAL_GAUGE
    // the edge set it consumes without a separate site-decoding
    // fallback, and lets future verbs look up records by
    // `lattice.vertex(vertex_a)` directly.
    let mut inferred: Vec<InferredFieldSchema> = Vec::new();
    inferred.push(InferredFieldSchema::numeric("config_id", /*is_base=*/ true));
    inferred.push(InferredFieldSchema::numeric("mu", /*is_base=*/ true));
    for i in 0..d {
        inferred.push(InferredFieldSchema::numeric(SITE_AXIS_NAMES[i], true));
    }
    inferred.push(InferredFieldSchema::numeric("vertex_a", /*is_base=*/ true));
    inferred.push(InferredFieldSchema::numeric("vertex_b", /*is_base=*/ true));
    let fiber_names = canonical_fiber_names(group);
    for name in fiber_names {
        inferred.push(InferredFieldSchema::numeric(name, /*is_base=*/ false));
    }
    let bundle_created = ensure_bundle_compatible(
        engine, target_bundle, &inferred, /*allow_auto_create=*/ true,
    )?;

    // Stream records in row-major order matching the NPZ layout.
    // Shape = [n_configs, D, L, L, ..., L, fiber]. Site coordinates
    // are decoded row-major (site_x is most-significant).
    //
    // OBC AXIS k semantics: when the lattice hint carries `_OBC_AXIS{k}`,
    // records whose mu = k AND coords[k] = L - 1 are the wrap edges the
    // lattice's own edge constructor drops. Those records are omitted
    // entirely from the ingested bundle so its record set matches the
    // lattice edge set exactly.
    let mut batch: Vec<Record> = Vec::with_capacity(INGEST_BATCH_SIZE.min(64));
    let mut records_emitted: usize = 0;
    let ln = l_ref.pow(d as u32); // sites per (config, mu)

    for config_id in 0..n_configs {
        for mu in 0..d {
            for site_flat in 0..ln {
                // Decode site_flat into per-axis coordinates in
                // row-major order (site_x most-significant).
                let mut site = [0usize; 4];
                let mut rem = site_flat;
                for axis in (0..d).rev() {
                    site[axis] = rem % l_ref;
                    rem /= l_ref;
                }

                // OBC omission: drop records that would wrap across
                // the open boundary. Everything else stays.
                if let Some(k) = obc_axis {
                    if mu == k && site[k] == l_ref - 1 {
                        continue;
                    }
                }

                let base = ((config_id * d + mu) * ln + site_flat) * expected_fiber;
                let slice = &data[base..base + expected_fiber];

                // vertex_a / vertex_b from the column-major site
                // encoding + shift-by-+1 along mu. Values equal
                // `lattice.site_of(coords)` for the same coords and L,
                // so `vertex_a` and `vertex_b` are the lattice's own
                // `VertexId` integers. On PERIODIC lattices the shift
                // wraps modulo L, matching the lattice's own edge
                // (s → site_of(shift_plus(coords, mu))).
                let coords = &site[..d];
                let vertex_a = site_of_column_major(coords, l_ref);
                let vertex_b = site_of_column_major(&shift_plus(coords, mu, l_ref), l_ref);

                let mut record: Record = Record::new();
                record.insert("config_id".to_string(), Value::Integer(config_id as i64));
                record.insert("mu".to_string(), Value::Integer(mu as i64));
                for axis in 0..d {
                    record.insert(
                        SITE_AXIS_NAMES[axis].to_string(),
                        Value::Integer(site[axis] as i64),
                    );
                }
                record.insert("vertex_a".to_string(), Value::Integer(vertex_a as i64));
                record.insert("vertex_b".to_string(), Value::Integer(vertex_b as i64));
                for (i, name) in fiber_names.iter().enumerate() {
                    record.insert((*name).to_string(), Value::Float(slice[i]));
                }
                batch.push(record);

                if batch.len() >= INGEST_BATCH_SIZE {
                    flush_batch(engine, target_bundle, &mut batch, &mut records_emitted)?;
                }
            }
        }
    }
    if !batch.is_empty() {
        flush_batch(engine, target_bundle, &mut batch, &mut records_emitted)?;
    }
    Ok(IngestStats { records_emitted, bundle_created, bytes_read })
}

#[cfg(test)]
mod tests {
    //! Module-internal smoke tests — full integration coverage lives
    //! in `tests/ingest_executor.rs`.

    use super::*;

    #[test]
    fn format_from_name_known_set() {
        assert_eq!(IngestFormat::from_name("NPZ").unwrap(), IngestFormat::Npz);
        assert_eq!(IngestFormat::from_name("npz").unwrap(), IngestFormat::Npz);
        assert_eq!(IngestFormat::from_name("CSV").unwrap(), IngestFormat::Csv);
        assert_eq!(IngestFormat::from_name("csv").unwrap(), IngestFormat::Csv);
        assert_eq!(IngestFormat::from_name("JSONL").unwrap(), IngestFormat::Jsonl);
        assert_eq!(IngestFormat::from_name("ndjson").unwrap(), IngestFormat::Jsonl);
        let err = IngestFormat::from_name("HDF5").unwrap_err();
        match err {
            IngestError::FormatNotSupported { requested, supported } => {
                assert_eq!(requested, "HDF5");
                assert_eq!(
                    supported,
                    vec!["NPZ".to_string(), "CSV".to_string(), "JSONL".to_string()]
                );
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
