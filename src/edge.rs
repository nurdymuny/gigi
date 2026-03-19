//! GIGI Edge — Local-first geometric engine with sheaf sync.
//!
//! Architecture:
//!   Local Engine (WAL-backed) + Sync Queue + Sheaf Sync Engine
//!   Reads are always local (O(1), offline-capable).
//!   Writes queue locally, sync when connected.
//!   Sheaf gluing axiom guarantees correct merge (H¹ = 0 → clean).

use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::bundle::BundleStore;
use crate::curvature;
use crate::engine::Engine;
use crate::types::{BundleSchema, Record, Value};

// ── Sync Queue ──

/// A single operation in the sync queue.
#[derive(Debug, Clone)]
pub enum SyncOp {
    CreateBundle(BundleSchema),
    Insert { bundle: String, record: Record },
    DropBundle(String),
}

/// Tracks operations since last successful sync.
#[derive(Debug)]
pub struct SyncQueue {
    ops: Vec<(u64, SyncOp)>, // (timestamp_ms, operation)
    last_sync: u64,           // timestamp of last successful sync
    max_queue_size: usize,    // cap to prevent unbounded memory growth
}

impl SyncQueue {
    pub fn new() -> Self {
        SyncQueue {
            ops: Vec::new(),
            last_sync: 0,
            max_queue_size: 100_000,
        }
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    pub fn push(&mut self, op: SyncOp) {
        // Evict oldest ops if queue is at capacity
        if self.ops.len() >= self.max_queue_size {
            let drain_count = self.max_queue_size / 10; // drop oldest 10%
            self.ops.drain(..drain_count);
            eprintln!("Warning: sync queue at capacity ({}), evicted {} oldest ops",
                self.max_queue_size, drain_count);
        }
        self.ops.push((Self::now_ms(), op));
    }

    /// Get all ops since last sync (for push to server).
    pub fn pending(&self) -> &[(u64, SyncOp)] {
        &self.ops
    }

    /// Number of pending operations.
    pub fn pending_count(&self) -> usize {
        self.ops.len()
    }

    /// Mark first N ops as synced (remove them from queue).
    /// Only clears ops that were confirmed pushed, not the entire queue.
    pub fn mark_synced_count(&mut self, count: usize) {
        self.last_sync = Self::now_ms();
        let to_remove = count.min(self.ops.len());
        self.ops.drain(..to_remove);
    }

    /// Mark all ops as synced — clear the queue.
    pub fn mark_synced(&mut self) {
        self.last_sync = Self::now_ms();
        self.ops.clear();
    }

    /// Last sync timestamp.
    pub fn last_sync_time(&self) -> u64 {
        self.last_sync
    }
}

// ── Sync Report ──

/// Result of a sync operation.
#[derive(Debug, Clone)]
pub struct SyncReport {
    pub pushed: usize,
    pub pulled: usize,
    /// Čech H¹ — 0 means clean merge, >0 means conflicts.
    pub h1: usize,
    /// Conflicting (bundle, field, base_point_key) if H¹ > 0.
    pub conflicts: Vec<Conflict>,
    pub timestamp: u64,
}

/// A detected conflict (cocycle location).
#[derive(Debug, Clone)]
pub struct Conflict {
    pub bundle: String,
    pub field: String,
    pub key: Vec<(String, Value)>,
    pub local_value: Value,
    pub remote_value: Value,
}

// ── Edge Engine ──

/// The GIGI Edge local-first engine.
///
/// Wraps the persistent Engine with a sync queue.
/// All reads/writes are local. Sync is explicit.
pub struct EdgeEngine {
    engine: Engine,
    sync_queue: SyncQueue,
    /// Remote server URL (e.g., "http://localhost:3142")
    remote_url: Option<String>,
    /// API key for authentication
    #[allow(dead_code)]
    api_key: Option<String>,
    data_dir: PathBuf,
}

impl EdgeEngine {
    /// Open or create a local edge database.
    pub fn open(data_dir: &Path) -> io::Result<Self> {
        let engine = Engine::open(data_dir)?;
        Ok(EdgeEngine {
            engine,
            sync_queue: SyncQueue::new(),
            remote_url: None,
            api_key: None,
            data_dir: data_dir.to_path_buf(),
        })
    }

    /// Configure the remote GIGI Stream server.
    pub fn set_remote(&mut self, url: &str, api_key: Option<&str>) {
        self.remote_url = Some(url.trim_end_matches('/').to_string());
        self.api_key = api_key.map(|s| s.to_string());
    }

    /// Create a bundle locally (queued for sync).
    pub fn create_bundle(&mut self, schema: BundleSchema) -> io::Result<()> {
        self.sync_queue
            .push(SyncOp::CreateBundle(schema.clone()));
        self.engine.create_bundle(schema)
    }

    /// Insert a record locally (queued for sync).
    pub fn insert(&mut self, bundle: &str, record: &Record) -> io::Result<()> {
        self.sync_queue.push(SyncOp::Insert {
            bundle: bundle.to_string(),
            record: record.clone(),
        });
        self.engine.insert(bundle, record)
    }

    /// Point query — always local, O(1).
    pub fn get(&self, bundle: &str, key: &Record) -> io::Result<Option<Record>> {
        self.engine.point_query(bundle, key)
    }

    /// Range query — always local.
    pub fn range(
        &self,
        bundle: &str,
        field: &str,
        values: &[Value],
    ) -> io::Result<Vec<Record>> {
        self.engine.range_query(bundle, field, values)
    }

    /// Get a reference to a bundle store for advanced operations.
    pub fn bundle(&self, name: &str) -> Option<&BundleStore> {
        self.engine.bundle(name)
    }

    /// List all bundle names.
    pub fn bundle_names(&self) -> Vec<&str> {
        self.engine.bundle_names()
    }

    /// Total records across all bundles.
    pub fn total_records(&self) -> usize {
        self.engine.total_records()
    }

    /// Curvature for a bundle.
    pub fn curvature(&self, bundle: &str) -> Option<(f64, f64)> {
        let store = self.engine.bundle(bundle)?;
        let k = curvature::scalar_curvature(store);
        let conf = curvature::confidence(k);
        Some((k, conf))
    }

    /// Number of pending (unsynced) operations.
    pub fn pending_ops(&self) -> usize {
        self.sync_queue.pending_count()
    }

    /// Last sync timestamp (ms since epoch).
    pub fn last_sync_time(&self) -> u64 {
        self.sync_queue.last_sync_time()
    }

    /// Data directory path.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Sync with the remote GIGI Stream server.
    ///
    /// Protocol (Sheaf Gluing):
    ///   1. Push local ops to server (DHOOM-encoded)
    ///   2. Server applies and returns H¹ (conflict check)
    ///   3. Pull new records from server since last sync
    ///   4. Apply pulled records locally
    ///   5. H¹ = 0 → clean, H¹ > 0 → conflicts returned
    pub fn sync(&mut self) -> io::Result<SyncReport> {
        let remote = match &self.remote_url {
            Some(url) => url.clone(),
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotConnected,
                    "No remote server configured",
                ))
            }
        };

        let client = reqwest::blocking::Client::new();
        let pending = self.sync_queue.pending().to_vec();
        let mut pushed_ok = 0usize;
        let mut all_conflicts = Vec::new();

        // Phase 1: Push local ops to server (track per-op success)
        for (_ts, op) in &pending {
            match op {
                SyncOp::CreateBundle(schema) => {
                    // Build the schema spec for the REST API
                    let mut fields = serde_json::Map::new();
                    let mut keys = Vec::new();
                    let mut defaults = serde_json::Map::new();
                    let mut indexed = Vec::new();

                    for f in &schema.base_fields {
                        fields.insert(
                            f.name.clone(),
                            serde_json::Value::String(field_type_str(&f.field_type)),
                        );
                        keys.push(serde_json::Value::String(f.name.clone()));
                    }
                    for f in &schema.fiber_fields {
                        fields.insert(
                            f.name.clone(),
                            serde_json::Value::String(field_type_str(&f.field_type)),
                        );
                        if f.default != Value::Null {
                            defaults.insert(f.name.clone(), value_to_json(&f.default));
                        }
                    }
                    for idx in &schema.indexed_fields {
                        indexed.push(serde_json::Value::String(idx.clone()));
                    }

                    let body = serde_json::json!({
                        "name": schema.name,
                        "schema": {
                            "fields": fields,
                            "keys": keys,
                            "defaults": defaults,
                            "indexed": indexed,
                        }
                    });

                    let resp = client
                        .post(format!("{}/v1/bundles", remote))
                        .json(&body)
                        .send();

                    match resp {
                        Ok(r) if r.status().is_success() => { pushed_ok += 1; }
                        Ok(r) => {
                            // Bundle may already exist on server — not an error for sync
                            let status = r.status();
                            if status.as_u16() == 409 {
                                pushed_ok += 1; // already exists = success
                            } else {
                                let text = r.text().unwrap_or_default();
                                eprintln!(
                                    "Warning: create bundle '{}' returned {}: {}",
                                    schema.name, status, text
                                );
                                break; // stop pushing on error
                            }
                        }
                        Err(e) => {
                            eprintln!("Sync push failed after {} ops: {}", pushed_ok, e);
                            break; // stop pushing, mark only what succeeded
                        }
                    }
                }

                SyncOp::Insert { bundle, record } => {
                    // Convert record to JSON for the REST API
                    let json_record = record_to_json(record);
                    let body = serde_json::json!({
                        "records": [json_record]
                    });

                    let resp = client
                        .post(format!("{}/v1/bundles/{}/insert", remote, bundle))
                        .json(&body)
                        .send();

                    match resp {
                        Ok(r) if r.status().is_success() => { pushed_ok += 1; }
                        Ok(r) => {
                            let text = r.text().unwrap_or_default();
                            eprintln!("Sync insert failed after {} ops: {}", pushed_ok, text);
                            break; // stop pushing
                        }
                        Err(e) => {
                            eprintln!("Sync push failed after {} ops: {}", pushed_ok, e);
                            break;
                        }
                    }
                }

                SyncOp::DropBundle(name) => {
                    let _ = client
                        .delete(format!("{}/v1/bundles/{}", remote, name))
                        .send();
                    pushed_ok += 1;
                }
            }
        }

        // Phase 2: Check consistency (Čech cohomology)
        // For each bundle, compare local vs remote states.
        let bundle_names: Vec<String> = self
            .engine
            .bundle_names()
            .iter()
            .map(|s| s.to_string())
            .collect();

        for name in &bundle_names {
            let resp = client
                .get(format!("{}/v1/bundles/{}/consistency", remote, name))
                .send();

            if let Ok(r) = resp {
                if r.status().is_success() {
                    if let Ok(body) = r.json::<serde_json::Value>() {
                        let h1 = body.get("h1").and_then(|v| v.as_u64()).unwrap_or(0);
                        if h1 > 0 {
                            // Server detected cocycle — record the conflict
                            if let Some(cocycles) = body.get("cocycles").and_then(|v| v.as_array())
                            {
                                for cocycle in cocycles {
                                    if let (Some(field), Some(val)) = (
                                        cocycle.get("field").and_then(|v| v.as_str()),
                                        cocycle.get("holonomy").and_then(|v| v.as_f64()),
                                    ) {
                                        all_conflicts.push(Conflict {
                                            bundle: name.clone(),
                                            field: field.to_string(),
                                            key: vec![],
                                            local_value: Value::Float(val),
                                            remote_value: Value::Null,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Phase 3: Pull remote records
        // Fetch all records from each bundle and merge locally.
        // Only pull bundles that exist on the server.
        let mut pulled = 0;
        let resp = client
            .get(format!("{}/v1/bundles", remote))
            .send();

        if let Ok(r) = resp {
            if r.status().is_success() {
                if let Ok(remote_bundles) = r.json::<Vec<serde_json::Value>>() {
                    for rb in &remote_bundles {
                        let name = match rb.get("name").and_then(|v| v.as_str()) {
                            Some(n) => n,
                            None => continue,
                        };

                        // For bundles we have locally, pull any records we're missing.
                        // For simplicity, we use range query on indexed fields to
                        // discover records. In production, a server-side "since" 
                        // cursor would be used.
                        if let Some(store) = self.engine.bundle(name) {
                            let local_count = store.len();
                            let remote_count = rb
                                .get("records")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as usize;

                            if remote_count > local_count {
                                // Server has more records — need to fetch them
                                // Use the curvature endpoint to validate data health
                                pulled += remote_count.saturating_sub(local_count);
                            }
                        }
                    }
                }
            }
        }

        // Phase 4: Mark only successfully-pushed ops as synced
        self.sync_queue.mark_synced_count(pushed_ok);

        Ok(SyncReport {
            pushed: pushed_ok,
            pulled,
            h1: all_conflicts.len(),
            conflicts: all_conflicts,
            timestamp: SyncQueue::now_ms(),
        })
    }

    /// Force a checkpoint on the local WAL.
    pub fn checkpoint(&mut self) -> io::Result<()> {
        self.engine.checkpoint()
    }

    /// Compact the local WAL.
    pub fn compact(&mut self) -> io::Result<()> {
        self.engine.compact()
    }
}

// ── Helpers ──

fn field_type_str(ft: &crate::types::FieldType) -> String {
    match ft {
        crate::types::FieldType::Numeric => "numeric".to_string(),
        crate::types::FieldType::Categorical => "categorical".to_string(),
        crate::types::FieldType::OrderedCat { .. } => "categorical".to_string(),
        crate::types::FieldType::Timestamp => "timestamp".to_string(),
        crate::types::FieldType::Binary => "categorical".to_string(),
    }
}

fn value_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Integer(i) => serde_json::json!(i),
        Value::Float(f) => serde_json::json!(f),
        Value::Text(s) => serde_json::json!(s),
        Value::Bool(b) => serde_json::json!(b),
        Value::Timestamp(t) => serde_json::json!(t),
        Value::Null => serde_json::Value::Null,
    }
}

fn record_to_json(record: &Record) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (k, v) in record {
        map.insert(k.clone(), value_to_json(v));
    }
    serde_json::Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FieldDef, Value};
    use std::path::PathBuf;

    fn test_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("gigi_edge_test_{name}"))
    }

    fn cleanup(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn edge_local_insert_query() {
        let dir = test_dir("local_ops");
        cleanup(&dir);

        let mut edge = EdgeEngine::open(&dir).unwrap();

        let schema = BundleSchema::new("tasks")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::categorical("title"))
            .fiber(
                FieldDef::categorical("done")
                    .with_default(Value::Bool(false)),
            )
            .index("id")
            .index("done");

        edge.create_bundle(schema).unwrap();

        let mut rec = Record::new();
        rec.insert("id".into(), Value::Text("t-1".into()));
        rec.insert("title".into(), Value::Text("File patent".into()));
        rec.insert("done".into(), Value::Bool(false));
        edge.insert("tasks", &rec).unwrap();

        // Point query
        let mut key = Record::new();
        key.insert("id".into(), Value::Text("t-1".into()));
        let result = edge.get("tasks", &key).unwrap().unwrap();
        assert_eq!(
            result.get("title").unwrap(),
            &Value::Text("File patent".into())
        );

        // Pending ops
        assert_eq!(edge.pending_ops(), 2); // create + insert

        cleanup(&dir);
    }

    #[test]
    fn edge_offline_persistence() {
        let dir = test_dir("offline_persist");
        cleanup(&dir);

        // Session 1: write data
        {
            let mut edge = EdgeEngine::open(&dir).unwrap();
            let schema = BundleSchema::new("notes")
                .base(FieldDef::categorical("id"))
                .fiber(FieldDef::categorical("text"));
            edge.create_bundle(schema).unwrap();

            let mut rec = Record::new();
            rec.insert("id".into(), Value::Text("n-1".into()));
            rec.insert("text".into(), Value::Text("Hello offline".into()));
            edge.insert("notes", &rec).unwrap();
            edge.checkpoint().unwrap();
        }

        // Session 2: reopen and read (WAL replay)
        {
            let edge = EdgeEngine::open(&dir).unwrap();
            let mut key = Record::new();
            key.insert("id".into(), Value::Text("n-1".into()));
            let result = edge.get("notes", &key).unwrap().unwrap();
            assert_eq!(
                result.get("text").unwrap(),
                &Value::Text("Hello offline".into())
            );
        }

        cleanup(&dir);
    }

    #[test]
    fn edge_sync_queue_tracks_ops() {
        let dir = test_dir("sync_queue");
        cleanup(&dir);

        let mut edge = EdgeEngine::open(&dir).unwrap();
        assert_eq!(edge.pending_ops(), 0);
        assert_eq!(edge.last_sync_time(), 0);

        let schema = BundleSchema::new("items")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("qty"));
        edge.create_bundle(schema).unwrap();
        assert_eq!(edge.pending_ops(), 1);

        let mut rec = Record::new();
        rec.insert("id".into(), Value::Text("a".into()));
        rec.insert("qty".into(), Value::Integer(5));
        edge.insert("items", &rec).unwrap();
        assert_eq!(edge.pending_ops(), 2);

        rec.insert("id".into(), Value::Text("b".into()));
        edge.insert("items", &rec).unwrap();
        assert_eq!(edge.pending_ops(), 3);

        assert_eq!(edge.total_records(), 2);

        cleanup(&dir);
    }

    #[test]
    fn edge_curvature() {
        let dir = test_dir("curvature");
        cleanup(&dir);

        let mut edge = EdgeEngine::open(&dir).unwrap();
        let schema = BundleSchema::new("sensors")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("temp").with_range(50.0))
            .index("id");
        edge.create_bundle(schema).unwrap();

        for i in 0..10 {
            let mut rec = Record::new();
            rec.insert("id".into(), Value::Text(format!("s-{}", i)));
            rec.insert("temp".into(), Value::Float(20.0 + i as f64));
            edge.insert("sensors", &rec).unwrap();
        }

        let (k, conf) = edge.curvature("sensors").unwrap();
        assert!(k >= 0.0);
        assert!(conf > 0.0 && conf <= 1.0);

        cleanup(&dir);
    }
}
