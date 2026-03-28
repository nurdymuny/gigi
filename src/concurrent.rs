//! Thread-safe concurrent access to the GIGI engine.
//!
//! Provides `ConcurrentEngine` — a wrapper around `Engine` with
//! read-write locking for safe multi-threaded access.
//!
//! Read operations (point_query, range_query, bundle access) acquire
//! shared read locks. Write operations (insert, create_bundle) acquire
//! exclusive write locks.

use std::path::Path;
use std::sync::{Arc, RwLock};

use crate::engine::Engine;
use crate::types::{BundleSchema, Record, Value};

/// Thread-safe concurrent engine.
///
/// Wraps `Engine` in an `Arc<RwLock<Engine>>` for safe multi-reader,
/// single-writer access.
#[derive(Clone)]
pub struct ConcurrentEngine {
    inner: Arc<RwLock<Engine>>,
}

impl ConcurrentEngine {
    /// Open or create a concurrent database.
    pub fn open(data_dir: &Path) -> std::io::Result<Self> {
        let engine = Engine::open(data_dir)?;
        Ok(Self {
            inner: Arc::new(RwLock::new(engine)),
        })
    }

    /// Create a new bundle (write lock).
    pub fn create_bundle(&self, schema: BundleSchema) -> std::io::Result<()> {
        let mut engine = self
            .inner
            .write()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Lock poisoned"))?;
        engine.create_bundle(schema)
    }

    /// Insert a record (write lock).
    pub fn insert(&self, bundle_name: &str, record: &Record) -> std::io::Result<()> {
        let mut engine = self
            .inner
            .write()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Lock poisoned"))?;
        engine.insert(bundle_name, record)
    }

    /// Point query (read lock).
    pub fn point_query(&self, bundle_name: &str, key: &Record) -> std::io::Result<Option<Record>> {
        let engine = self
            .inner
            .read()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Lock poisoned"))?;
        engine.point_query(bundle_name, key)
    }

    /// Range query (read lock).
    pub fn range_query(
        &self,
        bundle_name: &str,
        field: &str,
        values: &[Value],
    ) -> std::io::Result<Vec<Record>> {
        let engine = self
            .inner
            .read()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Lock poisoned"))?;
        engine.range_query(bundle_name, field, values)
    }

    /// Read-locked access for arbitrary operations.
    pub fn read<F, R>(&self, f: F) -> std::io::Result<R>
    where
        F: FnOnce(&Engine) -> R,
    {
        let engine = self
            .inner
            .read()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Lock poisoned"))?;
        Ok(f(&engine))
    }

    /// Write-locked access for arbitrary operations.
    pub fn write<F, R>(&self, f: F) -> std::io::Result<R>
    where
        F: FnOnce(&mut Engine) -> R,
    {
        let mut engine = self
            .inner
            .write()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Lock poisoned"))?;
        Ok(f(&mut engine))
    }

    /// Checkpoint (write lock).
    pub fn checkpoint(&self) -> std::io::Result<()> {
        let mut engine = self
            .inner
            .write()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Lock poisoned"))?;
        engine.checkpoint()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use std::thread;

    #[test]
    fn concurrent_reads_and_writes() {
        let dir = std::env::temp_dir().join("gigi_concurrent_test");
        let _ = std::fs::remove_dir_all(&dir);
        let engine = ConcurrentEngine::open(&dir).unwrap();

        // Create bundle
        let schema = BundleSchema::new("items")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("name"))
            .fiber(FieldDef::numeric("price").with_range(1000.0))
            .index("name");
        engine.create_bundle(schema).unwrap();

        // Spawn 4 writer threads, each inserting 100 records
        let n_writers = 4;
        let per_writer = 100;
        let mut handles = Vec::new();

        for w in 0..n_writers {
            let eng = engine.clone();
            handles.push(thread::spawn(move || {
                for i in 0..per_writer {
                    let id = w * per_writer + i;
                    let mut rec = Record::new();
                    rec.insert("id".into(), Value::Integer(id as i64));
                    rec.insert("name".into(), Value::Text(format!("item_{id}")));
                    rec.insert("price".into(), Value::Float(id as f64 * 10.0));
                    eng.insert("items", &rec).unwrap();
                }
            }));
        }

        // Spawn 4 reader threads, each doing random queries
        for _ in 0..4 {
            let eng = engine.clone();
            handles.push(thread::spawn(move || {
                for i in 0..50 {
                    let mut key = Record::new();
                    key.insert("id".into(), Value::Integer(i));
                    // May or may not find it depending on writer progress
                    let _ = eng.point_query("items", &key);
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // Verify all records were inserted
        let total = engine.read(|e| e.bundle("items").unwrap().len()).unwrap();
        assert_eq!(total, n_writers * per_writer);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn concurrent_readers_dont_block() {
        let dir = std::env::temp_dir().join("gigi_concurrent_readers");
        let _ = std::fs::remove_dir_all(&dir);
        let engine = ConcurrentEngine::open(&dir).unwrap();

        let schema = BundleSchema::new("data")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val").with_range(100.0));
        engine.create_bundle(schema).unwrap();

        for i in 0..100 {
            let mut rec = Record::new();
            rec.insert("id".into(), Value::Integer(i));
            rec.insert("val".into(), Value::Float(i as f64));
            engine.insert("data", &rec).unwrap();
        }

        // 8 concurrent readers
        let mut handles = Vec::new();
        for _ in 0..8 {
            let eng = engine.clone();
            handles.push(thread::spawn(move || {
                let count = eng.read(|e| e.bundle("data").unwrap().len()).unwrap();
                assert_eq!(count, 100);
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}
