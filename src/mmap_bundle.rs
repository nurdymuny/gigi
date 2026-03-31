//! Memory-mapped DHOOM bundles (Feature #11).
//!
//! Maps a DHOOM snapshot file into virtual memory so the OS page cache
//! manages record residency. Only actively queried pages consume RSS,
//! giving ~20× memory reduction for typical query workloads (Thm 11.1).

use std::collections::HashSet;
use std::fs::File;
use std::io;
use std::path::Path;
use std::sync::RwLock;

use memmap2::Mmap;
use serde_json::Value as JsonValue;

use crate::bundle::{matches_filter, AnomalyRecord, BundleStats, BundleStore, CurvatureStats, FieldStats, QueryCondition, QueryPlan, TransactionOp, TransactionResult, VectorMetric};
use crate::curvature;
use crate::spectral;
use crate::dhoom::{parse_fiber, DhoomRecordParser, Fiber, Modifier};
use crate::types::{BasePoint, BundleSchema, FieldDef, Record, Value};

/// Memory-mapped DHOOM bundle — records parsed on demand from OS page cache.
pub struct MmapBundle {
    /// Memory-mapped file bytes.
    mmap: Mmap,
    /// Shared record parser (uses Fiber internally).
    parser: DhoomRecordParser,
    /// Bundle/collection name.
    name: String,
    /// Byte offset of each record line start in the mmap.
    line_offsets: Vec<usize>,
    /// Byte offset where the records region begins (after header + pools).
    _data_start: usize,
}

impl MmapBundle {
    /// Open a DHOOM file as memory-mapped.
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        Self::from_mmap(mmap)
    }

    /// Build from raw mmap bytes (also usable in tests).
    pub fn from_mmap(mmap: Mmap) -> io::Result<Self> {
        let text = std::str::from_utf8(&mmap)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Find header end marker '}:'
        let header_end = text
            .find("}:")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing DHOOM header"))?
            + 2; // include '}:'

        let header = text[..header_end].trim();
        let fiber = parse_fiber(&header[..header.len() - 1]) // strip trailing ':'
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

        // Reject delta-encoded files (stateful — incompatible with random access)
        for fd in &fiber.fields {
            if matches!(fd.modifier, Some(Modifier::Delta)) {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    format!("Delta-encoded field '{}' not supported in mmap mode", fd.name),
                ));
            }
        }

        let name = fiber.name.clone().unwrap_or_default();
        let parser = DhoomRecordParser::new(fiber);

        // Skip pool lines (start with '&') before records.
        // Empty lines in the data section ARE valid records (all fields arithmetic/default).
        let body = &text[header_end..];
        let mut line_offsets = Vec::new();
        let mut pos = header_end;
        let mut in_pools = true;
        let mut saw_pool = false;
        let mut data_start = header_end;

        for line in body.lines() {
            let trimmed = line.trim();
            let line_byte_len = line.len() + 1; // +1 for newline (approximate)

            if in_pools {
                if trimmed.starts_with('&') {
                    saw_pool = true;
                    pos += line_byte_len;
                    continue;
                }
                // Blank lines are pool separators only if we've seen at least one '&' line
                if trimmed.is_empty() && saw_pool {
                    pos += line_byte_len;
                    continue;
                }
                // Skip leading blank line right after header (before any data)
                if trimmed.is_empty() && pos == header_end {
                    pos += line_byte_len;
                    continue;
                }
                in_pools = false;
                data_start = pos;
            }

            // Every non-pool line is a record (including empty lines = all-default records)
            line_offsets.push(pos);
            pos += line_byte_len;
        }

        Ok(Self {
            mmap,
            parser,
            name,
            line_offsets,
            _data_start: data_start,
        })
    }

    /// Bundle name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Number of records.
    pub fn len(&self) -> usize {
        self.line_offsets.len()
    }

    /// True if empty.
    pub fn is_empty(&self) -> bool {
        self.line_offsets.is_empty()
    }

    /// Read a single record by ordinal index (0-based).
    /// Parses the DHOOM line on demand from mmap'd bytes.
    pub fn get(&self, index: usize) -> Option<JsonValue> {
        if index >= self.line_offsets.len() {
            return None;
        }
        let start = self.line_offsets[index];
        let end = if index + 1 < self.line_offsets.len() {
            self.line_offsets[index + 1]
        } else {
            self.mmap.len()
        };
        let raw = std::str::from_utf8(&self.mmap[start..end]).ok()?;
        let line = raw.lines().next().unwrap_or("").trim();
        Some(self.parser.decode_line(line, index))
    }

    /// Sequential scan over all records.
    pub fn scan(&self) -> MmapScanIter<'_> {
        MmapScanIter {
            bundle: self,
            index: 0,
        }
    }

    /// Access the raw mmap (for advise calls).
    #[cfg(unix)]
    pub fn advise_sequential(&self) {
        self.mmap.advise(memmap2::Advice::Sequential).ok();
    }

    #[cfg(unix)]
    pub fn advise_random(&self) {
        self.mmap.advise(memmap2::Advice::Random).ok();
    }

    /// Access the raw fiber schema.
    pub fn fiber(&self) -> &Fiber {
        self.parser.fiber()
    }
}

/// Iterator over mmap'd records.
pub struct MmapScanIter<'a> {
    bundle: &'a MmapBundle,
    index: usize,
}

impl<'a> Iterator for MmapScanIter<'a> {
    type Item = JsonValue;

    fn next(&mut self) -> Option<Self::Item> {
        let rec = self.bundle.get(self.index)?;
        self.index += 1;
        Some(rec)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.bundle.len().saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for MmapScanIter<'a> {}

// ── OverlayBundle ───────────────────────────────────────────────────────────

/// Overlay bundle: mmap base + BundleStore overlay with full index support.
///
/// The overlay is a real `BundleStore`, so Feature #4's hash index acceleration
/// works on recently-written records. The base is read-only mmap.
/// Interior mutability via `RwLock<BundleStore>` allows concurrent reads with
/// occasional writes — reads take a shared lock, writes take an exclusive lock.
///
/// Tombstones are tracked inside the overlay's RwLock to keep the critical
/// section atomic (delete = tombstone + remove from overlay in one lock).
pub struct OverlayBundle {
    /// Mmap-backed snapshot data (immutable, shared across readers).
    base: MmapBundle,
    /// In-memory overlay with full BundleStore (indexes, stats, etc).
    overlay: RwLock<BundleStore>,
    /// Deleted keys from the base (tombstones). Inside the same RwLock
    /// would require a custom struct; we use a separate RwLock for simplicity.
    tombstones: RwLock<HashSet<String>>,
    /// Schema cached outside the RwLock for lock-free access.
    bundle_schema: BundleSchema,
}

impl OverlayBundle {
    /// Create an overlay on top of an mmap'd bundle.
    pub fn new(base: MmapBundle, schema: BundleSchema) -> Self {
        Self {
            base,
            overlay: RwLock::new(BundleStore::new(schema.clone())),
            tombstones: RwLock::new(HashSet::new()),
            bundle_schema: schema,
        }
    }

    /// Insert a record into the overlay (acquires write lock).
    pub fn insert(&self, record: &Record) {
        if let Ok(mut ts) = self.tombstones.write() {
            // If there's a key field, remove from tombstones
            if let Some(key_val) = record.values().next() {
                ts.remove(&format!("{key_val:?}"));
            }
        }
        if let Ok(mut store) = self.overlay.write() {
            store.insert(record);
        }
    }

    /// Update a record in the overlay.
    pub fn update(&self, key: &Record, patches: &Record) -> bool {
        if let Ok(mut store) = self.overlay.write() {
            store.update(key, patches)
        } else {
            false
        }
    }

    /// Mark a key as deleted (tombstone hides base record).
    pub fn delete(&self, key: &str, key_record: Option<&Record>) {
        if let Ok(mut ts) = self.tombstones.write() {
            ts.insert(key.to_string());
        }
        // Also remove from overlay if present
        if let Some(kr) = key_record {
            if let Ok(mut store) = self.overlay.write() {
                store.delete(kr);
            }
        }
    }

    /// Point lookup in overlay (acquires read lock).
    pub fn point_query_overlay(&self, key: &Record) -> Option<Record> {
        if let Ok(store) = self.overlay.read() {
            store.point_query(key)
        } else {
            None
        }
    }

    /// Check if a key is tombstoned.
    pub fn is_tombstoned(&self, key: &str) -> bool {
        self.tombstones.read().map_or(false, |ts| ts.contains(key))
    }

    /// Get the mmap base bundle.
    pub fn base(&self) -> &MmapBundle {
        &self.base
    }

    /// Number of overlay entries (acquires read lock).
    pub fn overlay_len(&self) -> usize {
        self.overlay.read().map_or(0, |s| s.len())
    }

    /// Number of tombstones.
    pub fn tombstone_len(&self) -> usize {
        self.tombstones.read().map_or(0, |ts| ts.len())
    }

    /// Access overlay BundleStore for queries (acquires read lock).
    /// The caller can use `filtered_query()` etc on the overlay store.
    pub fn with_overlay<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&BundleStore) -> R,
    {
        self.overlay.read().ok().map(|store| f(&*store))
    }

    /// Clear overlay and tombstones (after compaction to new mmap).
    pub fn clear_overlay(&self) {
        if let Ok(mut store) = self.overlay.write() {
            // Re-create with same schema
            let schema = store.schema.clone();
            *store = BundleStore::new(schema);
        }
        if let Ok(mut ts) = self.tombstones.write() {
            ts.clear();
        }
    }

    /// Swap the base mmap to a new snapshot file (after compaction).
    pub fn rebase(&mut self, new_base: MmapBundle, schema: BundleSchema) {
        self.base = new_base;
        self.bundle_schema = schema.clone();
        if let Ok(mut store) = self.overlay.write() {
            *store = BundleStore::new(schema);
        }
        if let Ok(mut ts) = self.tombstones.write() {
            ts.clear();
        }
    }

    // ── Schema & Stats (lock-free) ─────────────────────────────────────────

    /// Schema for this bundle (lock-free — stored outside RwLock).
    pub fn schema(&self) -> &BundleSchema {
        &self.bundle_schema
    }

    /// Curvature statistics (from overlay BundleStore).
    pub fn curvature_stats(&self) -> CurvatureStats {
        self.overlay
            .read()
            .map_or(CurvatureStats::default(), |s| s.curvature_stats.clone())
    }

    // ── Internal helpers ───────────────────────────────────────────────────

    /// Primary key field name (first base field).
    fn pk_field(&self) -> Option<&str> {
        self.bundle_schema
            .base_fields
            .first()
            .map(|f| f.name.as_str())
    }

    /// Extract tombstone key string from a Record.
    fn tombstone_key(&self, record: &Record) -> Option<String> {
        self.pk_field()
            .and_then(|f| record.get(f))
            .map(|v| format!("{v:?}"))
    }

    /// All PK strings currently in the overlay (for dedup during base scan).
    fn overlay_pk_set(&self) -> HashSet<String> {
        let pk = match self.pk_field() {
            Some(f) => f.to_string(),
            None => return HashSet::new(),
        };
        self.overlay.read().map_or(HashSet::new(), |s| {
            s.records()
                .filter_map(|r| r.get(&pk).map(|v| format!("{v:?}")))
                .collect()
        })
    }

    /// Convert a serde_json::Value (from MmapBundle) into a GIGI Record.
    fn json_to_record(jv: &serde_json::Value) -> Record {
        match jv {
            serde_json::Value::Object(map) => map
                .iter()
                .map(|(k, v)| (k.clone(), Self::json_val(v)))
                .collect(),
            _ => Record::new(),
        }
    }

    fn json_val(v: &serde_json::Value) -> Value {
        match v {
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Integer(i)
                } else {
                    Value::Float(n.as_f64().unwrap_or(0.0))
                }
            }
            serde_json::Value::String(s) => Value::Text(s.clone()),
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Array(arr) => {
                let floats: Vec<f64> = arr.iter().filter_map(|x| x.as_f64()).collect();
                if floats.len() == arr.len() && !arr.is_empty() {
                    Value::Vector(floats)
                } else {
                    Value::Null
                }
            }
            _ => Value::Null,
        }
    }

    /// Check if a base record is visible (not tombstoned, not overridden by overlay).
    fn is_visible(
        record: &Record,
        pk_field: Option<&str>,
        tombstones: &Option<std::sync::RwLockReadGuard<'_, HashSet<String>>>,
        overlay_keys: &HashSet<String>,
    ) -> bool {
        if let Some(pk) = pk_field.and_then(|f| record.get(f)) {
            let key_str = format!("{pk:?}");
            if let Some(ref ts) = tombstones {
                if ts.contains(&key_str) {
                    return false;
                }
            }
            if overlay_keys.contains(&key_str) {
                return false;
            }
        }
        true
    }

    // ── Merged Metadata ────────────────────────────────────────────────────

    /// Total record count (base − tombstones + overlay).
    pub fn len(&self) -> usize {
        let ts_count = self.tombstones.read().map_or(0, |ts| ts.len());
        self.base.len().saturating_sub(ts_count) + self.overlay_len()
    }

    pub fn is_empty_bundle(&self) -> bool {
        self.len() == 0
    }

    /// All field names from schema.
    pub fn field_names(&self) -> Vec<String> {
        self.bundle_schema
            .base_fields
            .iter()
            .chain(self.bundle_schema.fiber_fields.iter())
            .map(|f| f.name.clone())
            .collect()
    }

    pub fn storage_mode(&self) -> &'static str {
        "mmap+overlay"
    }

    pub fn next_auto_id(&self) -> i64 {
        self.overlay.write().map_or(0, |mut s| s.next_auto_id())
    }

    /// Per-field statistics (from overlay only — base has no running stats).
    pub fn field_stats(&self) -> std::collections::HashMap<String, FieldStats> {
        self.overlay
            .read()
            .map_or(std::collections::HashMap::new(), |s| {
                s.field_stats().clone()
            })
    }

    // ── Merged Query Methods ───────────────────────────────────────────────

    pub fn filtered_query(
        &self,
        conditions: &[QueryCondition],
        sort_by: Option<&str>,
        sort_desc: bool,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Vec<Record> {
        self.filtered_query_ex(conditions, None, sort_by, sort_desc, limit, offset)
    }

    pub fn filtered_query_ex(
        &self,
        conditions: &[QueryCondition],
        or_conditions: Option<&[Vec<QueryCondition>]>,
        sort_by: Option<&str>,
        sort_desc: bool,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Vec<Record> {
        let pk_field = self.pk_field();

        // Phase 1: Query overlay (has indexes → fast).
        let overlay_results = self
            .overlay
            .read()
            .ok()
            .map(|s| s.filtered_query_ex(conditions, or_conditions, sort_by, sort_desc, None, None))
            .unwrap_or_default();

        let overlay_keys: HashSet<String> = overlay_results
            .iter()
            .filter_map(|r| pk_field.and_then(|f| r.get(f)).map(|v| format!("{v:?}")))
            .collect();

        let tombstones = self.tombstones.read().ok();

        // ── Fast-path: no sort → streaming with early termination ──
        if sort_by.is_none() {
            let start = offset.unwrap_or(0);
            let take = limit.unwrap_or(usize::MAX);
            let need = start.saturating_add(take);

            let mut results = overlay_results;

            if results.len() < need {
                for i in 0..self.base.len() {
                    if let Some(jv) = self.base.get(i) {
                        let record = Self::json_to_record(&jv);
                        if !Self::is_visible(&record, pk_field, &tombstones, &overlay_keys) {
                            continue;
                        }
                        if matches_filter(&record, conditions, or_conditions) {
                            results.push(record);
                            if results.len() >= need {
                                break;
                            }
                        }
                    }
                }
            }

            return results.into_iter().skip(start).take(take).collect();
        }

        // ── Sorted path: buffer all matches (capped) ──
        let max_rows: usize = std::env::var("GIGI_QUERY_MAX_ROWS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10_000_000);

        let mut results = overlay_results;

        for i in 0..self.base.len() {
            if let Some(jv) = self.base.get(i) {
                let record = Self::json_to_record(&jv);
                if !Self::is_visible(&record, pk_field, &tombstones, &overlay_keys) {
                    continue;
                }
                if matches_filter(&record, conditions, or_conditions) {
                    results.push(record);
                    if results.len() > max_rows {
                        break;
                    }
                }
            }
        }

        let field = sort_by.unwrap().to_string();
        results.sort_by(|a, b| {
            let va = a.get(&field).unwrap_or(&Value::Null);
            let vb = b.get(&field).unwrap_or(&Value::Null);
            if sort_desc {
                vb.cmp(va)
            } else {
                va.cmp(vb)
            }
        });

        let start = offset.unwrap_or(0);
        if start > 0 {
            results = results.into_iter().skip(start).collect();
        }
        if let Some(lim) = limit {
            results.truncate(lim);
        }
        results
    }

    pub fn filtered_query_projected(
        &self,
        conditions: &[QueryCondition],
        sort_by: Option<&str>,
        sort_desc: bool,
        limit: Option<usize>,
        offset: Option<usize>,
        fields: Option<&[&str]>,
    ) -> (Vec<Record>, usize) {
        let sort_fields: Option<Vec<(&str, bool)>> = sort_by.map(|f| vec![(f, sort_desc)]);
        self.filtered_query_projected_ex(
            conditions,
            None,
            sort_fields.as_deref(),
            limit,
            offset,
            fields,
        )
    }

    pub fn filtered_query_projected_ex(
        &self,
        conditions: &[QueryCondition],
        or_conditions: Option<&[Vec<QueryCondition>]>,
        sort_fields: Option<&[(&str, bool)]>,
        limit: Option<usize>,
        offset: Option<usize>,
        fields: Option<&[&str]>,
    ) -> (Vec<Record>, usize) {
        let pk_field = self.pk_field();

        // Phase 1: Overlay matches (no limit — need total count).
        let overlay_all = self
            .overlay
            .read()
            .ok()
            .map(|s| {
                s.filtered_query_ex(
                    conditions,
                    or_conditions,
                    None,  // no sort yet
                    false,
                    None,
                    None,
                )
            })
            .unwrap_or_default();

        let overlay_keys: HashSet<String> = overlay_all
            .iter()
            .filter_map(|r| pk_field.and_then(|f| r.get(f)).map(|v| format!("{v:?}")))
            .collect();

        let tombstones = self.tombstones.read().ok();
        let max_rows: usize = std::env::var("GIGI_QUERY_MAX_ROWS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10_000_000);

        // Phase 2: Base matches.
        let mut base_matches: Vec<Record> = Vec::new();
        for i in 0..self.base.len() {
            if let Some(jv) = self.base.get(i) {
                let record = Self::json_to_record(&jv);
                if !Self::is_visible(&record, pk_field, &tombstones, &overlay_keys) {
                    continue;
                }
                if matches_filter(&record, conditions, or_conditions) {
                    base_matches.push(record);
                    if base_matches.len() > max_rows {
                        break;
                    }
                }
            }
        }

        let total = overlay_all.len() + base_matches.len();
        let mut results = overlay_all;
        results.extend(base_matches);

        // Sort (multi-field).
        if let Some(sorts) = sort_fields {
            results.sort_by(|a, b| {
                for &(field, desc) in sorts {
                    let va = a.get(field).unwrap_or(&Value::Null);
                    let vb = b.get(field).unwrap_or(&Value::Null);
                    let ord = if desc { vb.cmp(va) } else { va.cmp(vb) };
                    if ord != std::cmp::Ordering::Equal {
                        return ord;
                    }
                }
                std::cmp::Ordering::Equal
            });
        }

        // Offset + Limit.
        let start = offset.unwrap_or(0);
        let mut paged: Vec<Record> = results.into_iter().skip(start).collect();
        if let Some(lim) = limit {
            paged.truncate(lim);
        }

        // Project fields.
        if let Some(fields) = fields {
            let field_set: HashSet<&str> = fields.iter().copied().collect();
            for r in &mut paged {
                r.retain(|k, _| field_set.contains(k.as_str()));
            }
        }

        (paged, total)
    }

    pub fn count_where(&self, conditions: &[QueryCondition]) -> usize {
        self.count_where_ex(conditions, None)
    }

    pub fn count_where_ex(
        &self,
        conditions: &[QueryCondition],
        or_conditions: Option<&[Vec<QueryCondition>]>,
    ) -> usize {
        let pk_field = self.pk_field();

        let (overlay_count, overlay_keys) = self.overlay.read().map_or(
            (0, HashSet::new()),
            |s| {
                let count = s.count_where_ex(conditions, or_conditions);
                let pk = pk_field.unwrap_or("");
                let keys: HashSet<String> = s
                    .records()
                    .filter_map(|r| r.get(pk).map(|v| format!("{v:?}")))
                    .collect();
                (count, keys)
            },
        );

        let tombstones = self.tombstones.read().ok();

        let mut base_count = 0usize;
        for i in 0..self.base.len() {
            if let Some(jv) = self.base.get(i) {
                let record = Self::json_to_record(&jv);
                if !Self::is_visible(&record, pk_field, &tombstones, &overlay_keys) {
                    continue;
                }
                if matches_filter(&record, conditions, or_conditions) {
                    base_count += 1;
                }
            }
        }

        overlay_count + base_count
    }

    pub fn exists(&self, conditions: &[QueryCondition]) -> bool {
        self.exists_ex(conditions, None)
    }

    pub fn exists_ex(
        &self,
        conditions: &[QueryCondition],
        or_conditions: Option<&[Vec<QueryCondition>]>,
    ) -> bool {
        if self
            .overlay
            .read()
            .map_or(false, |s| s.exists_ex(conditions, or_conditions))
        {
            return true;
        }

        let pk_field = self.pk_field();
        let overlay_keys = self.overlay_pk_set();
        let tombstones = self.tombstones.read().ok();

        for i in 0..self.base.len() {
            if let Some(jv) = self.base.get(i) {
                let record = Self::json_to_record(&jv);
                if !Self::is_visible(&record, pk_field, &tombstones, &overlay_keys) {
                    continue;
                }
                if matches_filter(&record, conditions, or_conditions) {
                    return true;
                }
            }
        }
        false
    }

    /// Point lookup: overlay first (indexed), then base scan.
    pub fn point_query(&self, key: &Record) -> Option<Record> {
        // Check overlay first.
        if let Ok(store) = self.overlay.read() {
            if let Some(rec) = store.point_query(key) {
                return Some(rec);
            }
        }

        // Check tombstones.
        let pk_field = self.pk_field()?;
        let key_val = key.get(pk_field)?;
        let key_str = format!("{key_val:?}");
        if self.is_tombstoned(&key_str) {
            return None;
        }

        // Linear scan base for matching key.
        for i in 0..self.base.len() {
            if let Some(jv) = self.base.get(i) {
                let record = Self::json_to_record(&jv);
                if record.get(pk_field) == Some(key_val) {
                    return Some(record);
                }
            }
        }
        None
    }

    /// Range query: merge overlay + base.
    pub fn range_query(&self, field: &str, values: &[Value]) -> Vec<Record> {
        let pk_field = self.pk_field();

        let overlay_results = self
            .overlay
            .read()
            .map_or(Vec::new(), |s| s.range_query(field, values));
        let overlay_keys: HashSet<String> = overlay_results
            .iter()
            .filter_map(|r| pk_field.and_then(|f| r.get(f)).map(|v| format!("{v:?}")))
            .collect();

        let tombstones = self.tombstones.read().ok();
        let value_set: HashSet<&Value> = values.iter().collect();

        let mut results = overlay_results;
        for i in 0..self.base.len() {
            if let Some(jv) = self.base.get(i) {
                let record = Self::json_to_record(&jv);
                if !Self::is_visible(&record, pk_field, &tombstones, &overlay_keys) {
                    continue;
                }
                if let Some(val) = record.get(field) {
                    if value_set.contains(val) {
                        results.push(record);
                    }
                }
            }
        }
        results
    }

    /// Merged record iterator: overlay records first, then visible base records.
    pub fn records(&self) -> Box<dyn Iterator<Item = Record> + '_> {
        let pk_field = self.pk_field().map(|s| s.to_string());
        let overlay_records: Vec<Record> = self
            .overlay
            .read()
            .map_or(Vec::new(), |s| s.records().collect());

        let overlay_keys: HashSet<String> = overlay_records
            .iter()
            .filter_map(|r| {
                pk_field
                    .as_deref()
                    .and_then(|f| r.get(f))
                    .map(|v| format!("{v:?}"))
            })
            .collect();

        let base_len = self.base.len();
        let tombstones: HashSet<String> = self
            .tombstones
            .read()
            .map_or(HashSet::new(), |ts| ts.clone());

        // Chain overlay then base.
        let overlay_iter = overlay_records.into_iter();
        let base_iter = (0..base_len).filter_map(move |i| {
            let jv = self.base.get(i)?;
            let record = Self::json_to_record(&jv);
            if let Some(ref pk_f) = pk_field {
                if let Some(pk) = record.get(pk_f.as_str()) {
                    let key_str = format!("{pk:?}");
                    if tombstones.contains(&key_str) || overlay_keys.contains(&key_str) {
                        return None;
                    }
                }
            }
            Some(record)
        });

        Box::new(overlay_iter.chain(base_iter))
    }

    /// Distinct values for a field (merged).
    pub fn distinct(&self, field: &str) -> Vec<Value> {
        let mut seen: HashSet<Value> = HashSet::new();

        // Overlay values.
        if let Ok(store) = self.overlay.read() {
            for v in store.distinct(field) {
                seen.insert(v);
            }
        }

        // Base values (skip tombstoned/overridden records).
        let pk_field = self.pk_field();
        let overlay_keys = self.overlay_pk_set();
        let tombstones = self.tombstones.read().ok();

        for i in 0..self.base.len() {
            if let Some(jv) = self.base.get(i) {
                let record = Self::json_to_record(&jv);
                if !Self::is_visible(&record, pk_field, &tombstones, &overlay_keys) {
                    continue;
                }
                if let Some(val) = record.get(field) {
                    seen.insert(val.clone());
                }
            }
        }

        seen.into_iter().collect()
    }

    /// Indexed values for a field (from overlay; base has no index).
    pub fn indexed_values(&self, field: &str) -> Vec<Value> {
        self.overlay
            .read()
            .map_or(Vec::new(), |s| s.indexed_values(field))
    }

    /// Anomaly detection — delegates to overlay only (base lacks BaseStorage internals).
    pub fn compute_anomalies(
        &self,
        n_sigma: f64,
        pre_filter: Option<&[QueryCondition]>,
        limit: usize,
    ) -> Vec<AnomalyRecord> {
        self.overlay
            .read()
            .map_or(Vec::new(), |s| s.compute_anomalies(n_sigma, pre_filter, limit))
    }

    /// Vector search — delegates to overlay only.
    pub fn vector_search(
        &self,
        field: &str,
        query: &[f64],
        top_k: usize,
        metric: VectorMetric,
        pre_filter: &[QueryCondition],
    ) -> Vec<(f64, Record)> {
        self.overlay
            .read()
            .map_or(Vec::new(), |s| s.vector_search(field, query, top_k, metric, pre_filter))
    }

    // ── Merged Write Methods ───────────────────────────────────────────────

    pub fn batch_insert(&self, records: &[Record]) -> usize {
        if let Ok(mut store) = self.overlay.write() {
            if let Ok(mut ts) = self.tombstones.write() {
                let pk = self.pk_field();
                for r in records {
                    if let Some(key_str) = pk.and_then(|f| r.get(f)).map(|v| format!("{v:?}")) {
                        ts.remove(&key_str);
                    }
                }
            }
            store.batch_insert(records)
        } else {
            0
        }
    }

    pub fn upsert(&self, record: &Record) -> bool {
        if let Ok(mut ts) = self.tombstones.write() {
            if let Some(key_str) = self.tombstone_key(record) {
                ts.remove(&key_str);
            }
        }
        self.overlay
            .write()
            .map_or(false, |mut s| s.upsert(record))
    }

    pub fn update_returning(&self, key: &Record, patches: &Record) -> Option<Record> {
        self.overlay
            .write()
            .ok()
            .and_then(|mut s| s.update_returning(key, patches))
    }

    pub fn update_versioned(
        &self,
        key: &Record,
        patches: &Record,
        expected_version: i64,
    ) -> Result<i64, &'static str> {
        self.overlay
            .write()
            .map_err(|_| "lock poisoned")
            .and_then(|mut s| s.update_versioned(key, patches, expected_version))
    }

    pub fn increment(&self, key: &Record, field: &str, amount: f64) -> bool {
        self.overlay
            .write()
            .map_or(false, |mut s| s.increment(key, field, amount))
    }

    pub fn bulk_delete(&self, conditions: &[QueryCondition]) -> usize {
        // Find matching records in overlay + base, tombstone base keys.
        let pk_field = self.pk_field();

        // Delete from overlay.
        let overlay_deleted = self
            .overlay
            .write()
            .map_or(0, |mut s| s.bulk_delete(conditions));

        // Tombstone matching base records.
        let tombstones_guard = self.tombstones.read().ok();
        let overlay_keys = self.overlay_pk_set();
        let mut base_deleted = 0usize;
        let mut new_tombstones: Vec<String> = Vec::new();

        for i in 0..self.base.len() {
            if let Some(jv) = self.base.get(i) {
                let record = Self::json_to_record(&jv);
                if !Self::is_visible(&record, pk_field, &tombstones_guard, &overlay_keys) {
                    continue;
                }
                if conditions.iter().all(|c: &QueryCondition| c.matches(&record)) {
                    if let Some(key_str) = pk_field
                        .and_then(|f| record.get(f))
                        .map(|v| format!("{v:?}"))
                    {
                        new_tombstones.push(key_str);
                        base_deleted += 1;
                    }
                }
            }
        }
        drop(tombstones_guard);

        if !new_tombstones.is_empty() {
            if let Ok(mut ts) = self.tombstones.write() {
                for k in new_tombstones {
                    ts.insert(k);
                }
            }
        }

        overlay_deleted + base_deleted
    }

    pub fn delete_returning(&self, key: &Record) -> Option<Record> {
        // Try overlay first.
        if let Ok(mut store) = self.overlay.write() {
            if let Some(old) = store.delete_returning(key) {
                return Some(old);
            }
        }
        // Tombstone in base + return the record.
        let pk_field = self.pk_field()?;
        let key_val = key.get(pk_field)?;
        let key_str = format!("{key_val:?}");
        if self.is_tombstoned(&key_str) {
            return None;
        }
        // Scan base.
        for i in 0..self.base.len() {
            if let Some(jv) = self.base.get(i) {
                let record = Self::json_to_record(&jv);
                if record.get(pk_field) == Some(key_val) {
                    if let Ok(mut ts) = self.tombstones.write() {
                        ts.insert(key_str);
                    }
                    return Some(record);
                }
            }
        }
        None
    }

    // ── Schema Mutation ────────────────────────────────────────────────────

    pub fn drop_field(&self, field_name: &str) -> bool {
        self.overlay
            .write()
            .map_or(false, |mut s| s.drop_field(field_name))
    }

    pub fn add_field(&self, field_def: FieldDef) {
        if let Ok(mut store) = self.overlay.write() {
            store.add_field(field_def);
        }
    }

    pub fn add_index(&self, field_name: &str) {
        if let Ok(mut store) = self.overlay.write() {
            store.add_index(field_name);
        }
    }

    pub fn truncate(&self) -> usize {
        let base_count = self.base.len();
        // Tombstone ALL base records.
        if let Ok(mut ts) = self.tombstones.write() {
            ts.clear();
            for i in 0..base_count {
                if let Some(jv) = self.base.get(i) {
                    let record = Self::json_to_record(&jv);
                    if let Some(pk_field) = self.pk_field() {
                        if let Some(pk) = record.get(pk_field) {
                            ts.insert(format!("{pk:?}"));
                        }
                    }
                }
            }
        }
        let overlay_count = self
            .overlay
            .write()
            .map_or(0, |mut s| s.truncate());
        base_count + overlay_count
    }

    pub fn bulk_update(&self, conditions: &[QueryCondition], patches: &Record) -> usize {
        self.overlay
            .write()
            .map_or(0, |mut s| s.bulk_update(conditions, patches))
    }

    pub fn expire_ttl(&self, now_epoch_ms: i64) -> usize {
        self.overlay
            .write()
            .map_or(0, |mut s| s.expire_ttl(now_epoch_ms))
    }
}

// ── BundleRef: Unified read-only access to heap or mmap bundles ────────────

/// Enum dispatch for read-only bundle access.
/// Returned by `Engine::bundle()`.
pub enum BundleRef<'a> {
    Heap(&'a BundleStore),
    Overlay(&'a OverlayBundle),
}

impl<'a> BundleRef<'a> {
    /// Downcast to heap BundleStore (for analytics that need internal BaseStorage).
    pub fn as_heap(&self) -> Option<&BundleStore> {
        match self {
            BundleRef::Heap(s) => Some(s),
            BundleRef::Overlay(_) => None,
        }
    }

    pub fn schema(&self) -> &BundleSchema {
        match self {
            BundleRef::Heap(s) => &s.schema,
            BundleRef::Overlay(o) => o.schema(),
        }
    }

    pub fn curvature_stats(&self) -> CurvatureStats {
        match self {
            BundleRef::Heap(s) => s.curvature_stats.clone(),
            BundleRef::Overlay(o) => o.curvature_stats(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            BundleRef::Heap(s) => s.len(),
            BundleRef::Overlay(o) => o.len(),
        }
    }

    pub fn storage_mode(&self) -> &'static str {
        match self {
            BundleRef::Heap(s) => s.storage_mode(),
            BundleRef::Overlay(o) => o.storage_mode(),
        }
    }

    pub fn field_names(&self) -> Vec<String> {
        match self {
            BundleRef::Heap(s) => s.schema.base_fields.iter()
                .chain(s.schema.fiber_fields.iter())
                .map(|f| f.name.clone())
                .collect(),
            BundleRef::Overlay(o) => o.field_names(),
        }
    }

    pub fn field_stats(&self) -> std::collections::HashMap<String, FieldStats> {
        match self {
            BundleRef::Heap(s) => s.field_stats().clone(),
            BundleRef::Overlay(o) => o.field_stats(),
        }
    }

    pub fn filtered_query(
        &self,
        conditions: &[QueryCondition],
        sort_by: Option<&str>,
        sort_desc: bool,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Vec<Record> {
        match self {
            BundleRef::Heap(s) => s.filtered_query(conditions, sort_by, sort_desc, limit, offset),
            BundleRef::Overlay(o) => {
                o.filtered_query(conditions, sort_by, sort_desc, limit, offset)
            }
        }
    }

    pub fn filtered_query_ex(
        &self,
        conditions: &[QueryCondition],
        or_conditions: Option<&[Vec<QueryCondition>]>,
        sort_by: Option<&str>,
        sort_desc: bool,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Vec<Record> {
        match self {
            BundleRef::Heap(s) => {
                s.filtered_query_ex(conditions, or_conditions, sort_by, sort_desc, limit, offset)
            }
            BundleRef::Overlay(o) => {
                o.filtered_query_ex(conditions, or_conditions, sort_by, sort_desc, limit, offset)
            }
        }
    }

    pub fn filtered_query_projected(
        &self,
        conditions: &[QueryCondition],
        sort_by: Option<&str>,
        sort_desc: bool,
        limit: Option<usize>,
        offset: Option<usize>,
        fields: Option<&[&str]>,
    ) -> (Vec<Record>, usize) {
        match self {
            BundleRef::Heap(s) => {
                s.filtered_query_projected(conditions, sort_by, sort_desc, limit, offset, fields)
            }
            BundleRef::Overlay(o) => {
                o.filtered_query_projected(conditions, sort_by, sort_desc, limit, offset, fields)
            }
        }
    }

    pub fn filtered_query_projected_ex(
        &self,
        conditions: &[QueryCondition],
        or_conditions: Option<&[Vec<QueryCondition>]>,
        sort_fields: Option<&[(&str, bool)]>,
        limit: Option<usize>,
        offset: Option<usize>,
        fields: Option<&[&str]>,
    ) -> (Vec<Record>, usize) {
        match self {
            BundleRef::Heap(s) => s.filtered_query_projected_ex(
                conditions,
                or_conditions,
                sort_fields,
                limit,
                offset,
                fields,
            ),
            BundleRef::Overlay(o) => o.filtered_query_projected_ex(
                conditions,
                or_conditions,
                sort_fields,
                limit,
                offset,
                fields,
            ),
        }
    }

    pub fn count_where(&self, conditions: &[QueryCondition]) -> usize {
        match self {
            BundleRef::Heap(s) => s.count_where(conditions),
            BundleRef::Overlay(o) => o.count_where(conditions),
        }
    }

    pub fn count_where_ex(
        &self,
        conditions: &[QueryCondition],
        or_conditions: Option<&[Vec<QueryCondition>]>,
    ) -> usize {
        match self {
            BundleRef::Heap(s) => s.count_where_ex(conditions, or_conditions),
            BundleRef::Overlay(o) => o.count_where_ex(conditions, or_conditions),
        }
    }

    pub fn exists(&self, conditions: &[QueryCondition]) -> bool {
        match self {
            BundleRef::Heap(s) => s.exists(conditions),
            BundleRef::Overlay(o) => o.exists(conditions),
        }
    }

    pub fn exists_ex(
        &self,
        conditions: &[QueryCondition],
        or_conditions: Option<&[Vec<QueryCondition>]>,
    ) -> bool {
        match self {
            BundleRef::Heap(s) => s.exists_ex(conditions, or_conditions),
            BundleRef::Overlay(o) => o.exists_ex(conditions, or_conditions),
        }
    }

    pub fn point_query(&self, key: &Record) -> Option<Record> {
        match self {
            BundleRef::Heap(s) => s.point_query(key),
            BundleRef::Overlay(o) => o.point_query(key),
        }
    }

    pub fn range_query(&self, field: &str, values: &[Value]) -> Vec<Record> {
        match self {
            BundleRef::Heap(s) => s.range_query(field, values),
            BundleRef::Overlay(o) => o.range_query(field, values),
        }
    }

    pub fn records(&self) -> Box<dyn Iterator<Item = Record> + '_> {
        match self {
            BundleRef::Heap(s) => s.records(),
            BundleRef::Overlay(o) => o.records(),
        }
    }

    pub fn distinct(&self, field: &str) -> Vec<Value> {
        match self {
            BundleRef::Heap(s) => s.distinct(field),
            BundleRef::Overlay(o) => o.distinct(field),
        }
    }

    pub fn indexed_values(&self, field: &str) -> Vec<Value> {
        match self {
            BundleRef::Heap(s) => s.indexed_values(field),
            BundleRef::Overlay(o) => o.indexed_values(field),
        }
    }

    pub fn compute_anomalies(
        &self,
        n_sigma: f64,
        pre_filter: Option<&[QueryCondition]>,
        limit: usize,
    ) -> Vec<AnomalyRecord> {
        match self {
            BundleRef::Heap(s) => s.compute_anomalies(n_sigma, pre_filter, limit),
            BundleRef::Overlay(o) => o.compute_anomalies(n_sigma, pre_filter, limit),
        }
    }

    pub fn vector_search(
        &self,
        field: &str,
        query: &[f64],
        top_k: usize,
        metric: VectorMetric,
        pre_filter: &[QueryCondition],
    ) -> Vec<(f64, Record)> {
        match self {
            BundleRef::Heap(s) => s.vector_search(field, query, top_k, metric, pre_filter),
            BundleRef::Overlay(o) => o.vector_search(field, query, top_k, metric, pre_filter),
        }
    }

    // ── Convenience aliases / analytics helpers ────────────────────────────

    pub fn get_field_stats(&self) -> std::collections::HashMap<String, FieldStats> {
        self.field_stats()
    }

    pub fn stats(&self) -> BundleStats {
        match self {
            BundleRef::Heap(s) => s.stats(),
            BundleRef::Overlay(_) => BundleStats {
                name: self.schema().name.clone(),
                record_count: self.len(),
                base_fields: self.schema().base_fields.len(),
                fiber_fields: self.schema().fiber_fields.len(),
                indexed_fields: self.schema().indexed_fields.clone(),
                storage_mode: self.storage_mode().to_string(),
                index_sizes: Vec::new(),
                field_cardinalities: Vec::new(),
            },
        }
    }

    pub fn explain(
        &self,
        conditions: &[QueryCondition],
        or_conditions: Option<&[Vec<QueryCondition>]>,
        sort_fields: Option<&[(&str, bool)]>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> QueryPlan {
        match self {
            BundleRef::Heap(s) => s.explain(conditions, or_conditions, sort_fields, limit, offset),
            BundleRef::Overlay(_) => QueryPlan {
                scan_type: "overlay_merge_scan".to_string(),
                total_records: self.len(),
                index_scans: Vec::new(),
                full_scan_conditions: conditions.iter().map(|c| format!("{:?}", c)).collect(),
                or_group_count: or_conditions.map_or(0, |g| g.len()),
                has_sort: sort_fields.is_some(),
                has_limit: limit.is_some(),
                has_offset: offset.is_some(),
                storage_mode: "mmap+overlay".to_string(),
            },
        }
    }

    pub fn scalar_curvature(&self) -> f64 {
        match self {
            BundleRef::Heap(s) => curvature::scalar_curvature(s),
            BundleRef::Overlay(o) => o.curvature_stats().mean(),
        }
    }

    pub fn base_point(&self, key: &Record) -> BasePoint {
        match self {
            BundleRef::Heap(s) => s.base_point(key),
            BundleRef::Overlay(_) => Default::default(),
        }
    }

    pub fn holonomy(&self, loop_keys: &[Record]) -> f64 {
        match self {
            BundleRef::Heap(s) => curvature::holonomy(s, loop_keys),
            BundleRef::Overlay(_) => 0.0,
        }
    }

    pub fn betti_numbers(&self) -> (usize, usize) {
        match self {
            BundleRef::Heap(s) => spectral::betti_numbers(s),
            BundleRef::Overlay(_) => (0, 0),
        }
    }

    pub fn entropy(&self) -> f64 {
        match self {
            BundleRef::Heap(s) => spectral::entropy(s),
            BundleRef::Overlay(_) => 0.0,
        }
    }

    pub fn spectral_gap(&self) -> f64 {
        match self {
            BundleRef::Heap(s) => spectral::spectral_gap(s),
            BundleRef::Overlay(_) => 0.0,
        }
    }

    pub fn free_energy(&self, tau: f64) -> f64 {
        match self {
            BundleRef::Heap(s) => curvature::free_energy(s, tau),
            BundleRef::Overlay(_) => 0.0,
        }
    }
}

// ── BundleMut: Unified mutable access to heap or mmap bundles ──────────────

/// Enum dispatch for mutable bundle access.
/// Returned by `Engine::bundle_mut()`.
pub enum BundleMut<'a> {
    Heap(&'a mut BundleStore),
    Overlay(&'a OverlayBundle),
}

impl<'a> BundleMut<'a> {
    /// Convert to a read-only BundleRef (borrows self).
    pub fn as_ref(&self) -> BundleRef<'_> {
        match self {
            BundleMut::Heap(s) => BundleRef::Heap(s),
            BundleMut::Overlay(o) => BundleRef::Overlay(o),
        }
    }

    // ── Read methods (same as BundleRef) ───────────────────────────────────

    pub fn as_heap(&self) -> Option<&BundleStore> {
        match self {
            BundleMut::Heap(s) => Some(s),
            BundleMut::Overlay(_) => None,
        }
    }

    pub fn as_heap_mut(&mut self) -> Option<&mut BundleStore> {
        match self {
            BundleMut::Heap(s) => Some(s),
            BundleMut::Overlay(_) => None,
        }
    }

    pub fn schema(&self) -> &BundleSchema {
        match self {
            BundleMut::Heap(s) => &s.schema,
            BundleMut::Overlay(o) => o.schema(),
        }
    }

    pub fn curvature_stats(&self) -> CurvatureStats {
        match self {
            BundleMut::Heap(s) => s.curvature_stats.clone(),
            BundleMut::Overlay(o) => o.curvature_stats(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            BundleMut::Heap(s) => s.len(),
            BundleMut::Overlay(o) => o.len(),
        }
    }

    pub fn filtered_query(
        &self,
        conditions: &[QueryCondition],
        sort_by: Option<&str>,
        sort_desc: bool,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Vec<Record> {
        match self {
            BundleMut::Heap(s) => s.filtered_query(conditions, sort_by, sort_desc, limit, offset),
            BundleMut::Overlay(o) => {
                o.filtered_query(conditions, sort_by, sort_desc, limit, offset)
            }
        }
    }

    pub fn filtered_query_ex(
        &self,
        conditions: &[QueryCondition],
        or_conditions: Option<&[Vec<QueryCondition>]>,
        sort_by: Option<&str>,
        sort_desc: bool,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Vec<Record> {
        match self {
            BundleMut::Heap(s) => {
                s.filtered_query_ex(conditions, or_conditions, sort_by, sort_desc, limit, offset)
            }
            BundleMut::Overlay(o) => {
                o.filtered_query_ex(conditions, or_conditions, sort_by, sort_desc, limit, offset)
            }
        }
    }

    pub fn point_query(&self, key: &Record) -> Option<Record> {
        match self {
            BundleMut::Heap(s) => s.point_query(key),
            BundleMut::Overlay(o) => o.point_query(key),
        }
    }

    pub fn records(&self) -> Box<dyn Iterator<Item = Record> + '_> {
        match self {
            BundleMut::Heap(s) => s.records(),
            BundleMut::Overlay(o) => o.records(),
        }
    }

    pub fn count_where(&self, conditions: &[QueryCondition]) -> usize {
        match self {
            BundleMut::Heap(s) => s.count_where(conditions),
            BundleMut::Overlay(o) => o.count_where(conditions),
        }
    }

    pub fn exists(&self, conditions: &[QueryCondition]) -> bool {
        match self {
            BundleMut::Heap(s) => s.exists(conditions),
            BundleMut::Overlay(o) => o.exists(conditions),
        }
    }

    pub fn distinct(&self, field: &str) -> Vec<Value> {
        match self {
            BundleMut::Heap(s) => s.distinct(field),
            BundleMut::Overlay(o) => o.distinct(field),
        }
    }

    pub fn storage_mode(&self) -> &'static str {
        match self {
            BundleMut::Heap(s) => s.storage_mode(),
            BundleMut::Overlay(o) => o.storage_mode(),
        }
    }

    pub fn field_names(&self) -> Vec<String> {
        match self {
            BundleMut::Heap(s) => s.schema.base_fields.iter()
                .chain(s.schema.fiber_fields.iter())
                .map(|f| f.name.clone())
                .collect(),
            BundleMut::Overlay(o) => o.field_names(),
        }
    }

    pub fn field_stats(&self) -> std::collections::HashMap<String, FieldStats> {
        match self {
            BundleMut::Heap(s) => s.field_stats().clone(),
            BundleMut::Overlay(o) => o.field_stats(),
        }
    }

    pub fn compute_anomalies(
        &self,
        n_sigma: f64,
        pre_filter: Option<&[QueryCondition]>,
        limit: usize,
    ) -> Vec<AnomalyRecord> {
        match self {
            BundleMut::Heap(s) => s.compute_anomalies(n_sigma, pre_filter, limit),
            BundleMut::Overlay(o) => o.compute_anomalies(n_sigma, pre_filter, limit),
        }
    }

    // ── Convenience aliases / analytics helpers ────────────────────────────

    pub fn get_field_stats(&self) -> std::collections::HashMap<String, FieldStats> {
        self.field_stats()
    }

    pub fn stats(&self) -> BundleStats {
        match self {
            BundleMut::Heap(s) => s.stats(),
            BundleMut::Overlay(_) => BundleStats {
                name: self.schema().name.clone(),
                record_count: self.len(),
                base_fields: self.schema().base_fields.len(),
                fiber_fields: self.schema().fiber_fields.len(),
                indexed_fields: self.schema().indexed_fields.clone(),
                storage_mode: self.storage_mode().to_string(),
                index_sizes: Vec::new(),
                field_cardinalities: Vec::new(),
            },
        }
    }

    pub fn base_point(&self, key: &Record) -> BasePoint {
        match self {
            BundleMut::Heap(s) => s.base_point(key),
            BundleMut::Overlay(_) => Default::default(),
        }
    }

    pub fn explain(
        &self,
        conditions: &[QueryCondition],
        or_conditions: Option<&[Vec<QueryCondition>]>,
        sort_fields: Option<&[(&str, bool)]>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> QueryPlan {
        match self {
            BundleMut::Heap(s) => s.explain(conditions, or_conditions, sort_fields, limit, offset),
            BundleMut::Overlay(_) => QueryPlan {
                scan_type: "overlay_merge_scan".to_string(),
                total_records: self.len(),
                index_scans: Vec::new(),
                full_scan_conditions: conditions.iter().map(|c| format!("{:?}", c)).collect(),
                or_group_count: or_conditions.map_or(0, |g| g.len()),
                has_sort: sort_fields.is_some(),
                has_limit: limit.is_some(),
                has_offset: offset.is_some(),
                storage_mode: "mmap+overlay".to_string(),
            },
        }
    }

    pub fn scalar_curvature(&self) -> f64 {
        match self {
            BundleMut::Heap(s) => curvature::scalar_curvature(s),
            BundleMut::Overlay(o) => o.curvature_stats().mean(),
        }
    }

    pub fn holonomy(&self, loop_keys: &[Record]) -> f64 {
        match self {
            BundleMut::Heap(s) => curvature::holonomy(s, loop_keys),
            BundleMut::Overlay(_) => 0.0,
        }
    }

    pub fn betti_numbers(&self) -> (usize, usize) {
        match self {
            BundleMut::Heap(s) => spectral::betti_numbers(s),
            BundleMut::Overlay(_) => (0, 0),
        }
    }

    pub fn entropy(&self) -> f64 {
        match self {
            BundleMut::Heap(s) => spectral::entropy(s),
            BundleMut::Overlay(_) => 0.0,
        }
    }

    pub fn spectral_gap(&self) -> f64 {
        match self {
            BundleMut::Heap(s) => spectral::spectral_gap(s),
            BundleMut::Overlay(_) => 0.0,
        }
    }

    pub fn free_energy(&self, tau: f64) -> f64 {
        match self {
            BundleMut::Heap(s) => curvature::free_energy(s, tau),
            BundleMut::Overlay(_) => 0.0,
        }
    }

    pub fn execute_transaction(
        &mut self,
        ops: &[TransactionOp],
    ) -> Result<Vec<TransactionResult>, String> {
        match self {
            BundleMut::Heap(s) => s.execute_transaction(ops),
            BundleMut::Overlay(_) => Err("transactions not supported in mmap mode".to_string()),
        }
    }

    // ── Write methods ──────────────────────────────────────────────────────

    pub fn insert(&mut self, record: &Record) {
        match self {
            BundleMut::Heap(s) => s.insert(record),
            BundleMut::Overlay(o) => o.insert(record),
        }
    }

    pub fn batch_insert(&mut self, records: &[Record]) -> usize {
        match self {
            BundleMut::Heap(s) => s.batch_insert(records),
            BundleMut::Overlay(o) => o.batch_insert(records),
        }
    }

    pub fn upsert(&mut self, record: &Record) -> bool {
        match self {
            BundleMut::Heap(s) => s.upsert(record),
            BundleMut::Overlay(o) => o.upsert(record),
        }
    }

    pub fn update(&mut self, key: &Record, patches: &Record) -> bool {
        match self {
            BundleMut::Heap(s) => s.update(key, patches),
            BundleMut::Overlay(o) => o.update(key, patches),
        }
    }

    pub fn update_returning(&mut self, key: &Record, patches: &Record) -> Option<Record> {
        match self {
            BundleMut::Heap(s) => s.update_returning(key, patches),
            BundleMut::Overlay(o) => o.update_returning(key, patches),
        }
    }

    pub fn update_versioned(
        &mut self,
        key: &Record,
        patches: &Record,
        expected_version: i64,
    ) -> Result<i64, &'static str> {
        match self {
            BundleMut::Heap(s) => s.update_versioned(key, patches, expected_version),
            BundleMut::Overlay(o) => o.update_versioned(key, patches, expected_version),
        }
    }

    pub fn increment(&mut self, key: &Record, field: &str, amount: f64) -> bool {
        match self {
            BundleMut::Heap(s) => s.increment(key, field, amount),
            BundleMut::Overlay(o) => o.increment(key, field, amount),
        }
    }

    pub fn delete(&mut self, key: &Record) -> bool {
        match self {
            BundleMut::Heap(s) => s.delete(key),
            BundleMut::Overlay(o) => {
                let key_str = o.tombstone_key(key).unwrap_or_default();
                o.delete(&key_str, Some(key));
                true
            }
        }
    }

    pub fn delete_returning(&mut self, key: &Record) -> Option<Record> {
        match self {
            BundleMut::Heap(s) => s.delete_returning(key),
            BundleMut::Overlay(o) => o.delete_returning(key),
        }
    }

    pub fn bulk_delete(&mut self, conditions: &[QueryCondition]) -> usize {
        match self {
            BundleMut::Heap(s) => s.bulk_delete(conditions),
            BundleMut::Overlay(o) => o.bulk_delete(conditions),
        }
    }

    pub fn bulk_update(&mut self, conditions: &[QueryCondition], patches: &Record) -> usize {
        match self {
            BundleMut::Heap(s) => s.bulk_update(conditions, patches),
            BundleMut::Overlay(o) => o.bulk_update(conditions, patches),
        }
    }

    pub fn truncate(&mut self) -> usize {
        match self {
            BundleMut::Heap(s) => s.truncate(),
            BundleMut::Overlay(o) => o.truncate(),
        }
    }

    pub fn next_auto_id(&mut self) -> i64 {
        match self {
            BundleMut::Heap(s) => s.next_auto_id(),
            BundleMut::Overlay(o) => o.next_auto_id(),
        }
    }

    pub fn drop_field(&mut self, field_name: &str) -> bool {
        match self {
            BundleMut::Heap(s) => s.drop_field(field_name),
            BundleMut::Overlay(o) => o.drop_field(field_name),
        }
    }

    pub fn add_field(&mut self, field_def: FieldDef) {
        match self {
            BundleMut::Heap(s) => s.add_field(field_def),
            BundleMut::Overlay(o) => o.add_field(field_def),
        }
    }

    pub fn add_index(&mut self, field_name: &str) {
        match self {
            BundleMut::Heap(s) => s.add_index(field_name),
            BundleMut::Overlay(o) => o.add_index(field_name),
        }
    }

    pub fn expire_ttl(&mut self, now_epoch_ms: i64) -> usize {
        match self {
            BundleMut::Heap(s) => s.expire_ttl(now_epoch_ms),
            BundleMut::Overlay(o) => o.expire_ttl(now_epoch_ms),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    use crate::types::{BundleSchema, FieldDef, Value as GigiValue};

    /// Write DHOOM text to a temp file and open as MmapBundle.
    fn mmap_from_dhoom(dhoom: &str) -> MmapBundle {
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(dhoom.as_bytes()).unwrap();
        tmp.flush().unwrap();
        MmapBundle::open(tmp.path()).unwrap()
    }

    /// TDD-11.1: Mmap bundle reads correct data.
    #[test]
    fn tdd_11_1_mmap_reads_correct_data() {
        let dhoom = "items{id@1, name, score}:\nAlice, 95\nBob, 87\nCarol, 42\n";
        let bundle = mmap_from_dhoom(dhoom);

        assert_eq!(bundle.len(), 3);
        assert_eq!(bundle.name(), "items");

        let r0 = bundle.get(0).unwrap();
        assert_eq!(r0["id"], 1);
        assert_eq!(r0["name"], "Alice");
        assert_eq!(r0["score"], 95);

        let r1 = bundle.get(1).unwrap();
        assert_eq!(r1["id"], 2);
        assert_eq!(r1["name"], "Bob");

        let r2 = bundle.get(2).unwrap();
        assert_eq!(r2["id"], 3);
        assert_eq!(r2["name"], "Carol");
        assert_eq!(r2["score"], 42);
    }

    /// TDD-11.2: Point query by index is correct.
    #[test]
    fn tdd_11_2_point_query() {
        let dhoom = "drugs{id@100+10, drug_name, organism|E. coli}:\nrifampin, :S. aureus\namoxicillin\nvancomycin, :K. pneumoniae\n";
        let bundle = mmap_from_dhoom(dhoom);

        assert_eq!(bundle.len(), 3);
        let r0 = bundle.get(0).unwrap();
        assert_eq!(r0["id"], 100);
        assert_eq!(r0["drug_name"], "rifampin");
        assert_eq!(r0["organism"], "S. aureus");

        let r1 = bundle.get(1).unwrap();
        assert_eq!(r1["id"], 110);
        assert_eq!(r1["drug_name"], "amoxicillin");
        assert_eq!(r1["organism"], "E. coli"); // default

        // Out of bounds
        assert!(bundle.get(999).is_none());
    }

    /// TDD-11.3: Sequential scan correctness.
    #[test]
    fn tdd_11_3_sequential_scan() {
        let mut lines = String::from("data{id@1, value}:\n");
        for i in 0..100 {
            lines.push_str(&format!("{}\n", i * 10));
        }
        let bundle = mmap_from_dhoom(&lines);
        assert_eq!(bundle.len(), 100);

        let all: Vec<JsonValue> = bundle.scan().collect();
        assert_eq!(all.len(), 100);

        for (i, rec) in all.iter().enumerate() {
            assert_eq!(rec["id"], (i + 1) as i64);
            assert_eq!(rec["value"], (i * 10) as i64);
        }
    }

    /// TDD-11.4: Overlay with BundleStore — insert lands in indexed overlay.
    #[test]
    fn tdd_11_4_overlay_masks_updates() {
        let dhoom = "items{id@1, val}:\n10\n20\n30\n";
        let bundle = mmap_from_dhoom(dhoom);
        let schema = BundleSchema::new("items")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val"));
        let overlay = OverlayBundle::new(bundle, schema);

        // Base record
        let base_r0 = overlay.base().get(0).unwrap();
        assert_eq!(base_r0["val"], 10);

        // Insert into overlay (indexed BundleStore)
        let mut rec = Record::new();
        rec.insert("id".into(), GigiValue::Integer(99));
        rec.insert("val".into(), GigiValue::Integer(999));
        overlay.insert(&rec);

        // Overlay has the record
        let found = overlay.point_query_overlay(&{
            let mut k = Record::new();
            k.insert("id".into(), GigiValue::Integer(99));
            k
        });
        assert!(found.is_some());
        assert_eq!(found.unwrap().get("val"), Some(&GigiValue::Integer(999)));

        // Overlay count
        assert_eq!(overlay.overlay_len(), 1);
    }

    /// TDD-11.5: Tombstones hide mmap records.
    #[test]
    fn tdd_11_5_tombstones_hide_records() {
        let dhoom = "items{id@1, val}:\n10\n20\n";
        let bundle = mmap_from_dhoom(dhoom);
        let schema = BundleSchema::new("items")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val"));
        let overlay = OverlayBundle::new(bundle, schema);

        // Delete key "1"
        overlay.delete("1", None);
        assert!(overlay.is_tombstoned("1"));
        assert_eq!(overlay.tombstone_len(), 1);

        // Base data still exists
        assert!(overlay.base().get(0).is_some());
    }

    /// TDD-11.6: Base scan still works through overlay.
    #[test]
    fn tdd_11_6_scan_base_still_works() {
        let dhoom = "items{id@1, val}:\n10\n20\n30\n";
        let bundle = mmap_from_dhoom(dhoom);
        let schema = BundleSchema::new("items")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val"));
        let overlay = OverlayBundle::new(bundle, schema);

        // Base scan still returns all 3 records
        let all: Vec<JsonValue> = overlay.base().scan().collect();
        assert_eq!(all.len(), 3);
    }

    /// TDD-11.7: Clear overlay resets everything.
    #[test]
    fn tdd_11_7_compact_clears_overlay() {
        let dhoom = "items{id@1, val}:\n10\n20\n";
        let bundle = mmap_from_dhoom(dhoom);
        let schema = BundleSchema::new("items")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val"));
        let overlay = OverlayBundle::new(bundle, schema);

        let mut rec = Record::new();
        rec.insert("id".into(), GigiValue::Integer(99));
        rec.insert("val".into(), GigiValue::Integer(0));
        overlay.insert(&rec);
        overlay.delete("old", None);
        assert_eq!(overlay.overlay_len(), 1);
        assert_eq!(overlay.tombstone_len(), 1);

        overlay.clear_overlay();
        assert_eq!(overlay.overlay_len(), 0);
        assert_eq!(overlay.tombstone_len(), 0);
    }

    /// TDD-11.8: Default (modal) fields handled correctly in mmap.
    #[test]
    fn tdd_11_8_default_fields() {
        // Record fields (non-arithmetic) are: level, msg
        // Empty first field → level takes default "INFO"
        let dhoom = "log{ts@1000+1, level|INFO, msg}:\n, hello\n, world\n:WARN, alert\n";
        let bundle = mmap_from_dhoom(dhoom);

        assert_eq!(bundle.len(), 3);
        let r0 = bundle.get(0).unwrap();
        assert_eq!(r0["level"], "INFO"); // default (empty value)
        assert_eq!(r0["msg"], "hello");

        let r2 = bundle.get(2).unwrap();
        assert_eq!(r2["level"], "WARN"); // deviation override
        assert_eq!(r2["msg"], "alert");
    }

    /// TDD-11.9: Concurrent read safety — multiple readers on same mmap.
    #[test]
    fn tdd_11_9_concurrent_reads() {
        use std::sync::Arc;
        use std::thread;

        let mut lines = String::from("data{id@1, x}:\n");
        for i in 0..500 {
            lines.push_str(&format!("{}\n", i));
        }
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(lines.as_bytes()).unwrap();
        tmp.flush().unwrap();
        let bundle = Arc::new(MmapBundle::open(tmp.path()).unwrap());

        let handles: Vec<_> = (0..4)
            .map(|t| {
                let b = Arc::clone(&bundle);
                thread::spawn(move || {
                    for i in (t * 100)..((t + 1) * 100).min(500) {
                        let rec = b.get(i).unwrap();
                        assert_eq!(rec["id"], (i + 1) as i64);
                        assert_eq!(rec["x"], i as i64);
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }
    }

    /// TDD-11.10: Mmap survives reopen (read-only, no corruption).
    #[test]
    fn tdd_11_10_mmap_survives_reopen() {
        let dhoom = "items{id@1, name}:\nAlice\nBob\nCarol\n";
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(dhoom.as_bytes()).unwrap();
        tmp.flush().unwrap();
        let path = tmp.path().to_path_buf();

        // First open
        {
            let b1 = MmapBundle::open(&path).unwrap();
            assert_eq!(b1.len(), 3);
            assert_eq!(b1.get(0).unwrap()["name"], "Alice");
        }

        // Second open (simulates restart)
        {
            let b2 = MmapBundle::open(&path).unwrap();
            assert_eq!(b2.len(), 3);
            assert_eq!(b2.get(2).unwrap()["name"], "Carol");
        }
    }
}
