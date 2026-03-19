//! Persistent storage engine — ties BundleStore + WAL together.
//!
//! Provides crash-safe, disk-backed bundle management.
//! On startup, replays the WAL to reconstruct in-memory state.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::bundle::BundleStore;
use crate::types::{BundleSchema, Record};
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
    pub fn open(data_dir: &Path) -> io::Result<Self> {
        fs::create_dir_all(data_dir)?;

        // Set restrictive permissions on data directory (Unix only: owner rwx only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o700);
            fs::set_permissions(data_dir, perms)?;
        }

        let wal_path = data_dir.join("gigi.wal");

        // Replay WAL if it exists
        let mut bundles = HashMap::new();
        let mut schemas = HashMap::new();

        if wal_path.exists() {
            let mut reader = WalReader::open(&wal_path)?;
            let entries = reader.read_all()?;
            for entry in entries {
                match entry {
                    WalEntry::CreateBundle(schema) => {
                        let store = BundleStore::new(schema.clone());
                        bundles.insert(schema.name.clone(), store);
                        schemas.insert(schema.name.clone(), schema);
                    }
                    WalEntry::Insert { bundle_name, record } => {
                        if let Some(store) = bundles.get_mut(&bundle_name) {
                            store.insert(&record);
                        }
                    }
                    WalEntry::Update { bundle_name, key, patches } => {
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
                    WalEntry::Checkpoint => {
                        // Checkpoint: all prior entries are committed
                    }
                }
            }
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
    pub fn update(&mut self, bundle_name: &str, key: &Record, patches: &Record) -> io::Result<bool> {
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
            io::Error::new(io::ErrorKind::NotFound, format!("Bundle '{}' not found", bundle_name))
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

    /// Compact the WAL: write a fresh WAL from current state.
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

    fn maybe_checkpoint(&mut self) -> io::Result<()> {
        self.ops_since_checkpoint += 1;
        if self.ops_since_checkpoint >= self.checkpoint_interval {
            self.checkpoint()?;
        }
        Ok(())
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
            assert!(size_after < size_before, "compact: {size_after} >= {size_before}");
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
}
