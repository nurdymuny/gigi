//! Persistent storage engine — ties BundleStore + WAL together.
//!
//! Provides crash-safe, disk-backed bundle management.
//! On startup, replays the WAL to reconstruct in-memory state,
//! then loads DHOOM snapshots for any bundle whose snapshot predates the WAL.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use crate::bundle::BundleStore;
use crate::types::{BundleSchema, Record, Value};
use crate::wal::{WalEntry, WalReader, WalWriter};

/// The persistent database engine.
pub struct Engine {
    /// Data directory for WAL and data files.
    data_dir: PathBuf,
    /// In-memory bundle stores keyed by bundle name.
    bundles: HashMap<String, BundleStore>,
    /// Schemas stored separately for WAL replay.
    schemas: HashMap<String, BundleSchema>,
    /// Write-ahead log.
    wal: WalWriter,
    /// Count of ops since last checkpoint.
    ops_since_checkpoint: u64,
    /// Checkpoint interval (number of ops between auto-checkpoints).
    checkpoint_interval: u64,
}

impl Engine {
    /// Open or create a database at the given directory.
    ///
    /// Startup sequence — three logical phases in a single streaming WAL pass:
    ///
    ///   Phase 1 — Schema: CreateBundle entries create empty BundleStores.
    ///
    ///   Phase 2 — Bulk load: at the first Checkpoint we load DHOOM snapshot
    ///             files for every bundle that has 0 in-memory records.  This
    ///             is the correct insertion point because `snapshot()` compacts
    ///             the WAL to [CreateBundle* Checkpoint] and then new inserts
    ///             follow the checkpoint.  Loading snapshots here means records
    ///             0-N arrive before post-snapshot WAL inserts N+1..M, which is
    ///             the order the Sequential storage's start/step require.
    ///
    ///   Phase 3 — Incremental: WAL entries after the checkpoint (post-snapshot
    ///             inserts/updates/deletes) are applied on top of the snapshot.
    pub fn open(data_dir: &Path) -> io::Result<Self> {
        Self::open_inner(data_dir, true)
    }

    /// Open the engine without replaying the WAL. The engine is empty but
    /// ready to accept new writes. Call `replay_wal()` separately to load
    /// existing data. Used for early HTTP bind during startup.
    pub fn open_empty(data_dir: &Path) -> io::Result<Self> {
        Self::open_inner(data_dir, false)
    }

    fn open_inner(data_dir: &Path, replay: bool) -> io::Result<Self> {
        fs::create_dir_all(data_dir)?;

        // Set restrictive permissions on data directory (Unix only: owner rwx only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o700);
            fs::set_permissions(data_dir, perms)?;
        }

        let wal_path = data_dir.join("gigi.wal");

        let mut bundles: HashMap<String, BundleStore> = HashMap::new();
        let mut schemas: HashMap<String, BundleSchema> = HashMap::new();

        if replay && wal_path.exists() {
            Self::do_replay(&wal_path, data_dir, &mut bundles, &mut schemas)?;
        }

        // Open WAL for appending new operations
        let wal = WalWriter::open(&wal_path)?;

        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            bundles,
            schemas,
            wal,
            ops_since_checkpoint: 0,
            checkpoint_interval: 10_000,
        })
    }

    /// Replay the WAL from disk into this engine's in-memory state.
    /// Call this after `open_empty()` to load existing data.
    /// Logs progress every 100K entries to stderr.
    pub fn replay_wal(&mut self) -> io::Result<()> {
        let wal_path = self.data_dir.join("gigi.wal");
        if !wal_path.exists() {
            return Ok(());
        }
        Self::do_replay(
            &wal_path,
            &self.data_dir,
            &mut self.bundles,
            &mut self.schemas,
        )
    }

    fn do_replay(
        wal_path: &Path,
        data_dir: &Path,
        bundles: &mut HashMap<String, BundleStore>,
        schemas: &mut HashMap<String, BundleSchema>,
    ) -> io::Result<()> {
        let snapshots_dir = data_dir.join("snapshots");
        let mut snapshots_loaded = false;
        let mut entry_count: u64 = 0;
        let start = std::time::Instant::now();

        let mut reader = WalReader::open(wal_path)?;
        eprintln!("  WAL replay starting...");

        reader.replay(|entry| {
            entry_count += 1;
            if entry_count.is_multiple_of(100_000) {
                let elapsed = start.elapsed().as_secs_f64();
                let rate = entry_count as f64 / elapsed;
                let total: usize = bundles.values().map(|s| s.len()).sum();
                eprintln!(
                    "  WAL replay: {entry_count} entries ({total} records) — {rate:.0} entries/s"
                );
            }
            match entry {
                WalEntry::CreateBundle(schema) => {
                    bundles
                        .entry(schema.name.clone())
                        .or_insert_with(|| BundleStore::new(schema.clone()));
                    schemas.insert(schema.name.clone(), schema);
                }
                WalEntry::Checkpoint if !snapshots_loaded => {
                    snapshots_loaded = true;
                    if snapshots_dir.exists() {
                        for (name, store) in bundles.iter_mut() {
                            if !store.is_empty() {
                                continue;
                            }
                            let snap_path = snapshots_dir.join(format!("{name}.dhoom"));
                            if !snap_path.exists() {
                                continue;
                            }
                            match load_dhoom_snapshot(&snap_path) {
                                Ok(records) => {
                                    let n = records.len();
                                    store.batch_insert(&records);
                                    eprintln!("  Loaded snapshot {name}: {n} records from DHOOM");
                                }
                                Err(e) => {
                                    eprintln!("  WARNING: failed to load snapshot {name}: {e}");
                                }
                            }
                        }
                    }
                }
                WalEntry::Checkpoint => {}
                WalEntry::Insert {
                    bundle_name,
                    record,
                } => {
                    if let Some(store) = bundles.get_mut(&bundle_name) {
                        store.insert(&record);
                    }
                }
                WalEntry::Update {
                    bundle_name,
                    key,
                    patches,
                } => {
                    if let Some(store) = bundles.get_mut(&bundle_name) {
                        store.update(&key, &patches);
                    }
                }
                WalEntry::Delete { bundle_name, key } => {
                    if let Some(store) = bundles.get_mut(&bundle_name) {
                        store.delete(&key);
                    }
                }
                WalEntry::DropBundle(bundle_name) => {
                    bundles.remove(&bundle_name);
                    schemas.remove(&bundle_name);
                }
            }
            Ok(())
        })?;

        let elapsed = start.elapsed().as_secs_f64();
        let total: usize = bundles.values().map(|s| s.len()).sum();
        eprintln!("  WAL replay complete: {entry_count} entries, {total} records in {elapsed:.1}s");
        Ok(())
    }

    /// Create a new bundle (table).
    pub fn create_bundle(&mut self, schema: BundleSchema) -> io::Result<()> {
        self.wal.log_create_bundle(&schema)?;
        let store = BundleStore::new(schema.clone());
        self.bundles.insert(schema.name.clone(), store);
        self.schemas.insert(schema.name.clone(), schema);
        self.maybe_checkpoint()?;
        Ok(())
    }

    /// Insert a record into a named bundle.
    pub fn insert(&mut self, bundle_name: &str, record: &Record) -> io::Result<()> {
        self.wal.log_insert(bundle_name, record)?;
        if let Some(store) = self.bundles.get_mut(bundle_name) {
            store.insert(record);
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Bundle '{}' not found", bundle_name),
            ));
        }
        self.maybe_checkpoint()?;
        Ok(())
    }

    /// Update a record: partial field patches applied to existing record.
    pub fn update(
        &mut self,
        bundle_name: &str,
        key: &Record,
        patches: &Record,
    ) -> io::Result<bool> {
        self.wal.log_update(bundle_name, key, patches)?;
        let updated = if let Some(store) = self.bundles.get_mut(bundle_name) {
            store.update(key, patches)
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Bundle '{}' not found", bundle_name),
            ));
        };
        self.maybe_checkpoint()?;
        Ok(updated)
    }

    /// Delete a record by key.
    pub fn delete(&mut self, bundle_name: &str, key: &Record) -> io::Result<bool> {
        self.wal.log_delete(bundle_name, key)?;
        let deleted = if let Some(store) = self.bundles.get_mut(bundle_name) {
            store.delete(key)
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Bundle '{}' not found", bundle_name),
            ));
        };
        self.maybe_checkpoint()?;
        Ok(deleted)
    }

    /// Drop (remove) a bundle entirely.
    pub fn drop_bundle(&mut self, name: &str) -> io::Result<bool> {
        self.wal.log_drop_bundle(name)?;
        let existed = self.bundles.remove(name).is_some();
        self.schemas.remove(name);
        self.maybe_checkpoint()?;
        Ok(existed)
    }

    /// Batch insert — single WAL flush + single checkpoint check for N records.
    pub fn batch_insert(&mut self, bundle_name: &str, records: &[Record]) -> io::Result<usize> {
        // WAL: log all records first (sequential writes, single flush)
        for record in records {
            self.wal.log_insert(bundle_name, record)?;
        }
        self.wal.sync()?;

        // In-memory: batch insert into BundleStore
        let store = self.bundles.get_mut(bundle_name).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Bundle '{}' not found", bundle_name),
            )
        })?;
        let count = store.batch_insert(records);

        // Single checkpoint check for entire batch
        self.ops_since_checkpoint += count as u64;
        if self.ops_since_checkpoint >= self.checkpoint_interval {
            self.checkpoint()?;
        }

        Ok(count)
    }

    /// Point query on a named bundle.
    pub fn point_query(&self, bundle_name: &str, key: &Record) -> io::Result<Option<Record>> {
        match self.bundles.get(bundle_name) {
            Some(store) => Ok(store.point_query(key)),
            None => Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Bundle '{}' not found", bundle_name),
            )),
        }
    }

    /// Range query on a named bundle.
    pub fn range_query(
        &self,
        bundle_name: &str,
        field: &str,
        values: &[crate::types::Value],
    ) -> io::Result<Vec<Record>> {
        match self.bundles.get(bundle_name) {
            Some(store) => Ok(store.range_query(field, values)),
            None => Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Bundle '{}' not found", bundle_name),
            )),
        }
    }

    /// Get a reference to a bundle store for advanced operations.
    pub fn bundle(&self, name: &str) -> Option<&BundleStore> {
        self.bundles.get(name)
    }

    /// Get a mutable reference to a bundle store.
    pub fn bundle_mut(&mut self, name: &str) -> Option<&mut BundleStore> {
        self.bundles.get_mut(name)
    }

    /// List all bundle names.
    pub fn bundle_names(&self) -> Vec<&str> {
        self.bundles.keys().map(|s| s.as_str()).collect()
    }

    /// Number of records across all bundles.
    pub fn total_records(&self) -> usize {
        self.bundles.values().map(|b| b.len()).sum()
    }

    /// Force a checkpoint — syncs WAL to disk.
    pub fn checkpoint(&mut self) -> io::Result<()> {
        self.wal.log_checkpoint()?;
        self.wal.sync()?;
        self.ops_since_checkpoint = 0;
        Ok(())
    }

    /// Compact the WAL: write a fresh WAL from current state (full WAL replay format).
    /// For large datasets prefer `snapshot()` which uses DHOOM encoding.
    pub fn compact(&mut self) -> io::Result<()> {
        let wal_path = self.data_dir.join("gigi.wal");
        let tmp_path = self.data_dir.join("gigi.wal.tmp");

        // Write fresh WAL
        {
            let mut new_wal = WalWriter::open(&tmp_path)?;
            for (name, schema) in &self.schemas {
                new_wal.log_create_bundle(schema)?;
                if let Some(store) = self.bundles.get(name) {
                    for record in store.records() {
                        new_wal.log_insert(name, &record)?;
                    }
                }
            }
            new_wal.log_checkpoint()?;
            new_wal.sync()?;
        }

        // Atomic rename
        fs::rename(&tmp_path, &wal_path)?;
        self.wal = WalWriter::open(&wal_path)?;
        self.ops_since_checkpoint = 0;
        Ok(())
    }

    /// DHOOM snapshot — persist every bundle as a DHOOM file, then compact the WAL
    /// to schema-only entries.
    ///
    /// After this call:
    /// - `/data/snapshots/{bundle}.dhoom` contains all records.
    /// - `gigi.wal` contains only `CreateBundle` headers (fast startup).
    /// - New inserts go to the WAL as normal; the next `snapshot()` will absorb them.
    ///
    /// On restart, `Engine::open()` replays the (now tiny) WAL to get schemas, then
    /// loads each DHOOM snapshot for bundles with 0 WAL records.
    pub fn snapshot(&mut self) -> io::Result<usize> {
        let snapshots_dir = self.data_dir.join("snapshots");
        fs::create_dir_all(&snapshots_dir)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&snapshots_dir, fs::Permissions::from_mode(0o700));
        }

        let mut total_records = 0usize;

        for (name, store) in &self.bundles {
            let snap_path = snapshots_dir.join(format!("{name}.dhoom"));
            let tmp_path = snapshots_dir.join(format!("{name}.dhoom.tmp"));

            let records: Vec<serde_json::Value> = store
                .records()
                .map(|rec| record_to_serde_json(&rec))
                .collect();

            let count = records.len();
            if count == 0 {
                continue;
            }

            let encoded = crate::dhoom::encode_json(&records, name);
            {
                let mut f = fs::File::create(&tmp_path)?;
                f.write_all(encoded.dhoom.as_bytes())?;
                f.sync_all()?;
            }
            fs::rename(&tmp_path, &snap_path)?;
            total_records += count;
            eprintln!("  Snapshot written: {name} ({count} records)");
        }

        // Compact WAL to schema-only (no insert entries).
        // On next startup the DHOOM files provide the bulk data.
        let wal_path = self.data_dir.join("gigi.wal");
        let tmp_path = self.data_dir.join("gigi.wal.tmp");
        {
            let mut new_wal = WalWriter::open(&tmp_path)?;
            for schema in self.schemas.values() {
                new_wal.log_create_bundle(schema)?;
            }
            new_wal.log_checkpoint()?;
            new_wal.sync()?;
        }
        fs::rename(&tmp_path, &wal_path)?;
        self.wal = WalWriter::open(&wal_path)?;
        self.ops_since_checkpoint = 0;

        Ok(total_records)
    }

    fn maybe_checkpoint(&mut self) -> io::Result<()> {
        self.ops_since_checkpoint += 1;
        if self.ops_since_checkpoint >= self.checkpoint_interval {
            self.checkpoint()?;
        }
        Ok(())
    }
}

// ── DHOOM snapshot helpers ────────────────────────────────────────────────────

/// Convert a GIGI Record into a serde_json Object (for DHOOM encoding).
fn record_to_serde_json(rec: &Record) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (k, v) in rec {
        map.insert(k.clone(), value_to_serde_json(v));
    }
    serde_json::Value::Object(map)
}

fn value_to_serde_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Integer(i) => serde_json::json!(i),
        Value::Float(f) => serde_json::json!(f),
        Value::Text(s) => serde_json::json!(s),
        Value::Bool(b) => serde_json::json!(b),
        Value::Timestamp(t) => serde_json::json!(t),
        Value::Null => serde_json::Value::Null,
        Value::Vector(vs) => {
            serde_json::Value::Array(vs.iter().map(|x| serde_json::json!(x)).collect())
        }
    }
}

/// Load records from a DHOOM snapshot file into a Vec<Record>.
fn load_dhoom_snapshot(path: &Path) -> io::Result<Vec<Record>> {
    let text = fs::read_to_string(path)?;
    let parsed = crate::dhoom::decode_legacy(&text)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let json_records = crate::dhoom::dhoom_to_json_array(&parsed);
    let records = json_records
        .iter()
        .filter_map(|jv| {
            if let serde_json::Value::Object(map) = jv {
                Some(
                    map.iter()
                        .map(|(k, v)| (k.clone(), serde_json_to_value(v)))
                        .collect::<Record>(),
                )
            } else {
                None
            }
        })
        .collect();
    Ok(records)
}

fn serde_json_to_value(v: &serde_json::Value) -> Value {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FieldDef, Value};

    fn test_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("gigi_engine_test_{name}"))
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    /// Engine: create bundle + insert + query.
    #[test]
    fn engine_basic_ops() {
        let dir = test_dir("basic");
        cleanup(&dir);

        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("users")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("name"))
                .fiber(FieldDef::numeric("salary").with_range(100_000.0));
            engine.create_bundle(schema).unwrap();

            let mut rec = Record::new();
            rec.insert("id".into(), Value::Integer(1));
            rec.insert("name".into(), Value::Text("Alice".into()));
            rec.insert("salary".into(), Value::Float(75000.0));
            engine.insert("users", &rec).unwrap();

            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(1));
            let result = engine.point_query("users", &key).unwrap().unwrap();
            assert_eq!(result.get("name"), Some(&Value::Text("Alice".into())));
        }

        cleanup(&dir);
    }

    /// Engine: WAL replay on reopen.
    #[test]
    fn engine_wal_replay() {
        let dir = test_dir("replay");
        cleanup(&dir);

        // Write data
        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("employees")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("name"));
            engine.create_bundle(schema).unwrap();

            for i in 0..100 {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("name".into(), Value::Text(format!("Emp_{i}")));
                engine.insert("employees", &rec).unwrap();
            }
            engine.checkpoint().unwrap();
        }

        // Reopen and verify data survived
        {
            let engine = Engine::open(&dir).unwrap();
            assert_eq!(engine.total_records(), 100);

            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(42));
            let result = engine.point_query("employees", &key).unwrap().unwrap();
            assert_eq!(result.get("name"), Some(&Value::Text("Emp_42".into())));
        }

        cleanup(&dir);
    }

    /// Engine: compaction reduces WAL size.
    #[test]
    fn engine_compaction() {
        let dir = test_dir("compact");
        cleanup(&dir);

        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("data")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::numeric("val").with_range(1000.0));
            engine.create_bundle(schema).unwrap();

            // Insert 100, then overwrite 50 of them
            for i in 0..100i64 {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("val".into(), Value::Float(i as f64));
                engine.insert("data", &rec).unwrap();
            }
            for i in 0..50i64 {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("val".into(), Value::Float(i as f64 * 10.0));
                engine.insert("data", &rec).unwrap();
            }
            engine.checkpoint().unwrap();

            let wal_path = dir.join("gigi.wal");
            let size_before = fs::metadata(&wal_path).unwrap().len();

            engine.compact().unwrap();

            let size_after = fs::metadata(&wal_path).unwrap().len();
            // After compaction, WAL should be smaller (no duplicate overwrites)
            assert!(
                size_after < size_before,
                "compact: {size_after} >= {size_before}"
            );
        }

        // Verify data after compaction + reopen
        {
            let engine = Engine::open(&dir).unwrap();
            assert_eq!(engine.total_records(), 100);

            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(25));
            let r = engine.point_query("data", &key).unwrap().unwrap();
            // Should have the overwritten value
            assert_eq!(r.get("val"), Some(&Value::Float(250.0)));
        }

        cleanup(&dir);
    }

    /// Engine: insert into nonexistent bundle returns error.
    #[test]
    fn engine_missing_bundle() {
        let dir = test_dir("missing");
        cleanup(&dir);

        let mut engine = Engine::open(&dir).unwrap();
        let mut rec = Record::new();
        rec.insert("id".into(), Value::Integer(1));
        let result = engine.insert("nonexistent", &rec);
        assert!(result.is_err());

        cleanup(&dir);
    }

    /// Engine: update a record with WAL persistence.
    #[test]
    fn engine_update() {
        let dir = test_dir("update");
        cleanup(&dir);

        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("users")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("name"))
                .fiber(FieldDef::numeric("salary").with_range(100_000.0));
            engine.create_bundle(schema).unwrap();

            let mut rec = Record::new();
            rec.insert("id".into(), Value::Integer(1));
            rec.insert("name".into(), Value::Text("Alice".into()));
            rec.insert("salary".into(), Value::Float(75000.0));
            engine.insert("users", &rec).unwrap();

            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(1));
            let mut patches = Record::new();
            patches.insert("salary".into(), Value::Float(95000.0));
            assert!(engine.update("users", &key, &patches).unwrap());

            let result = engine.point_query("users", &key).unwrap().unwrap();
            assert_eq!(result.get("salary"), Some(&Value::Float(95000.0)));
            assert_eq!(result.get("name"), Some(&Value::Text("Alice".into())));
            engine.checkpoint().unwrap();
        }

        // WAL replay
        {
            let engine = Engine::open(&dir).unwrap();
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(1));
            let result = engine.point_query("users", &key).unwrap().unwrap();
            assert_eq!(result.get("salary"), Some(&Value::Float(95000.0)));
        }

        cleanup(&dir);
    }

    /// Engine: delete a record with WAL persistence.
    #[test]
    fn engine_delete() {
        let dir = test_dir("delete");
        cleanup(&dir);

        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("items")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("name"));
            engine.create_bundle(schema).unwrap();

            let mut rec = Record::new();
            rec.insert("id".into(), Value::Integer(1));
            rec.insert("name".into(), Value::Text("Item1".into()));
            engine.insert("items", &rec).unwrap();

            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(1));
            assert!(engine.delete("items", &key).unwrap());
            assert!(engine.point_query("items", &key).unwrap().is_none());
            engine.checkpoint().unwrap();
        }

        // WAL replay
        {
            let engine = Engine::open(&dir).unwrap();
            assert_eq!(engine.total_records(), 0);
        }

        cleanup(&dir);
    }

    // ── Tests for the persistence upgrade ────────────────────────────────────

    /// DHOOM snapshot: data survives snapshot() + reopen with no WAL inserts.
    ///
    /// This tests the primary post-deploy recovery path: WAL is schema-only
    /// after snapshot(), so reopen must load data from the DHOOM file.
    #[test]
    fn snapshot_survives_wal_compact() {
        let dir = test_dir("snap_wal_compact");
        cleanup(&dir);

        // Insert data and snapshot
        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("drugs")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("name"))
                .fiber(FieldDef::numeric("mw").with_range(1000.0));
            engine.create_bundle(schema).unwrap();

            for i in 0..1000i64 {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("name".into(), Value::Text(format!("Drug_{i}")));
                rec.insert("mw".into(), Value::Float(200.0 + i as f64 * 0.1));
                engine.insert("drugs", &rec).unwrap();
            }
            let snapped = engine.snapshot().unwrap();
            assert_eq!(snapped, 1000);

            // Verify WAL is now schema-only (no insert entries)
            let wal_path = dir.join("gigi.wal");
            let wal_size = fs::metadata(&wal_path).unwrap().len();
            // A WAL with 1000 inserts would be >>1 KB; schema-only should be tiny
            assert!(
                wal_size < 4096,
                "WAL should be schema-only after snapshot, was {wal_size}B"
            );
        }

        // Reopen — must load from DHOOM snapshot, not WAL inserts
        {
            let engine = Engine::open(&dir).unwrap();
            assert_eq!(
                engine.total_records(),
                1000,
                "records must survive snapshot+reopen"
            );

            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(500));
            let r = engine.point_query("drugs", &key).unwrap().unwrap();
            assert_eq!(r.get("name"), Some(&Value::Text("Drug_500".into())));
            let mw = r.get("mw").unwrap();
            // DHOOM encodes whole-number floats as integers in JSON (250.0 → 250).
            // Accept both Float(250.0) and Integer(250) as correct.
            let mw_f = match mw {
                Value::Float(f) => *f,
                Value::Integer(i) => *i as f64,
                _ => panic!("mw not numeric: {mw:?}"),
            };
            assert!((mw_f - 250.0).abs() < 0.01, "mw mismatch: {mw_f}");
        }

        cleanup(&dir);
    }

    /// Snapshot + new inserts: post-snapshot WAL inserts are not lost on reopen.
    ///
    /// Simulates: ingest → snapshot → more ingest → crash/restart.
    /// All records (pre- and post-snapshot) must be present.
    #[test]
    fn snapshot_then_new_inserts_survive_reopen() {
        let dir = test_dir("snap_then_insert");
        cleanup(&dir);

        // Phase 1: insert + snapshot
        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("compounds")
                .base(FieldDef::numeric("chembl_id"))
                .fiber(FieldDef::categorical("smiles"));
            engine.create_bundle(schema).unwrap();

            for i in 0..500i64 {
                let mut rec = Record::new();
                rec.insert("chembl_id".into(), Value::Integer(i));
                rec.insert("smiles".into(), Value::Text(format!("C{i}H{}", i * 2)));
                engine.insert("compounds", &rec).unwrap();
            }
            engine.snapshot().unwrap();
        }

        // Phase 2: reopen, add more records (post-snapshot WAL inserts)
        {
            let mut engine = Engine::open(&dir).unwrap();
            assert_eq!(engine.total_records(), 500);

            for i in 500..600i64 {
                let mut rec = Record::new();
                rec.insert("chembl_id".into(), Value::Integer(i));
                rec.insert("smiles".into(), Value::Text(format!("C{i}H{}", i * 2)));
                engine.insert("compounds", &rec).unwrap();
            }
            engine.checkpoint().unwrap();
        }

        // Phase 3: reopen — must have all 600 records (500 from DHOOM + 100 from WAL)
        {
            let engine = Engine::open(&dir).unwrap();
            assert_eq!(
                engine.total_records(),
                600,
                "pre-snapshot + post-snapshot records must all survive"
            );

            // Spot-check pre-snapshot record
            let mut key = Record::new();
            key.insert("chembl_id".into(), Value::Integer(100));
            assert!(engine.point_query("compounds", &key).unwrap().is_some());

            // Spot-check post-snapshot record
            let mut key2 = Record::new();
            key2.insert("chembl_id".into(), Value::Integer(550));
            assert!(engine.point_query("compounds", &key2).unwrap().is_some());
        }

        cleanup(&dir);
    }

    /// batch_insert goes through the WAL (regression for the WAL bypass bug).
    ///
    /// Directly tests the `Engine::batch_insert` path — if records are
    /// not WAL-logged they won't survive reopen.
    #[test]
    fn batch_insert_is_wal_logged() {
        let dir = test_dir("batch_wal");
        cleanup(&dir);

        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("activities")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::numeric("ic50").with_range(100_000.0));
            engine.create_bundle(schema).unwrap();

            let records: Vec<Record> = (0..200i64)
                .map(|i| {
                    let mut r = Record::new();
                    r.insert("id".into(), Value::Integer(i));
                    r.insert("ic50".into(), Value::Float(i as f64 * 0.5));
                    r
                })
                .collect();

            let inserted = engine.batch_insert("activities", &records).unwrap();
            assert_eq!(inserted, 200);
            // Do NOT call checkpoint — tests that WAL sync in batch_insert is sufficient
        }

        // Reopen without snapshot — data must be in WAL
        {
            let engine = Engine::open(&dir).unwrap();
            assert_eq!(
                engine.total_records(),
                200,
                "batch_insert records must be WAL-logged even without explicit checkpoint"
            );
        }

        cleanup(&dir);
    }

    /// Streaming WAL replay handles large entry counts without buffering the whole file.
    ///
    /// Not directly observable, but we can verify the replay() closure
    /// receives the correct entry count — ensuring the streaming path is exercised.
    #[test]
    fn streaming_wal_replay_correct_count() {
        let dir = test_dir("stream_replay");
        cleanup(&dir);

        let n = 5_000usize;

        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("data")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("label"));
            engine.create_bundle(schema).unwrap();

            for i in 0..n as i64 {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("label".into(), Value::Text(format!("L{i}")));
                engine.insert("data", &rec).unwrap();
            }
            engine.checkpoint().unwrap();
        }

        // Count entries via the new streaming replay API directly
        let wal_path = dir.join("gigi.wal");
        let mut reader = crate::wal::WalReader::open(&wal_path).unwrap();
        let mut insert_count = 0usize;
        reader
            .replay(|entry| {
                if matches!(entry, crate::wal::WalEntry::Insert { .. }) {
                    insert_count += 1;
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(insert_count, n);

        // Engine reopen via streaming path also correct
        let engine = Engine::open(&dir).unwrap();
        assert_eq!(engine.total_records(), n);

        cleanup(&dir);
    }
}
