//! Persistent storage engine — ties BundleStore + WAL together.
//!
//! Provides crash-safe, disk-backed bundle management.
//! On startup, replays the WAL to reconstruct in-memory state,
//! then loads DHOOM snapshots for any bundle whose snapshot predates the WAL.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::bundle::{BundleStore, QueryCondition};
use crate::mmap_bundle::{MmapBundle, OverlayBundle};
use crate::types::{BundleSchema, Record, Value};
use crate::wal::{WalEntry, WalReader, WalWriter};

/// Auto-compaction policy — controls when the engine automatically snapshots
/// to keep WAL size bounded. See spec Definition 1.2.
pub struct CompactionPolicy {
    /// WAL amplification threshold α (default: 3.0).
    /// Compaction fires when WAL_entries / N_eff > α.
    pub amplification_threshold: f64,
    /// Minimum seconds between compactions (default: 300).
    pub min_interval_secs: u64,
    /// Absolute WAL entry limit (default: 10_000_000).
    pub max_wal_entries: u64,
    /// WAL file size limit in bytes (default: 2 GiB).
    pub max_wal_bytes: u64,
    /// Disabled flag — when true, auto-compaction never fires.
    pub disabled: bool,
}

impl Default for CompactionPolicy {
    fn default() -> Self {
        Self {
            amplification_threshold: 3.0,
            min_interval_secs: 300,
            max_wal_entries: 10_000_000,
            max_wal_bytes: 2 * 1024 * 1024 * 1024, // 2 GiB
            disabled: false,
        }
    }
}

/// Snapshot of bundle data: name, schema, and collected records.
/// Point-in-time clone that can be encoded to DHOOM without holding the engine lock.
pub struct BundleDataClone {
    pub name: String,
    pub schema: BundleSchema,
    pub records: Vec<serde_json::Value>,
}

/// Storage mode: controls whether bundles are heap-resident or memory-mapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageMode {
    /// All records deserialized into heap memory (current default).
    Heap,
    /// DHOOM snapshots opened via mmap; WAL overlay in BundleStore.
    Mmap,
}

/// The persistent database engine.
pub struct Engine {
    /// Data directory for WAL and data files.
    data_dir: PathBuf,
    /// In-memory bundle stores keyed by bundle name (Heap mode).
    bundles: HashMap<String, BundleStore>,
    /// Mmap-backed bundles with BundleStore overlay (Mmap mode).
    mmap_bundles: HashMap<String, OverlayBundle>,
    /// Active storage mode.
    storage_mode: StorageMode,
    /// Schemas stored separately for WAL replay.
    schemas: HashMap<String, BundleSchema>,
    /// Write-ahead log.
    wal: WalWriter,
    /// Count of ops since last checkpoint.
    ops_since_checkpoint: u64,
    /// Checkpoint interval (number of ops between auto-checkpoints).
    checkpoint_interval: u64,
    /// Auto-compaction policy.
    compaction_policy: CompactionPolicy,
    /// Timestamp of last compaction.
    last_compaction: std::time::Instant,
    /// Number of WAL entries since last compaction/snapshot.
    wal_entry_count: u64,
    /// WAL file size in bytes (tracked incrementally).
    wal_byte_count: u64,
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
        let mut replay_entry_count: u64 = 0;

        if replay && wal_path.exists() {
            replay_entry_count =
                Self::do_replay(&wal_path, data_dir, &mut bundles, &mut schemas)?;
        }

        // WAL byte count from file metadata
        let wal_byte_count = if wal_path.exists() {
            fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        // Open WAL for appending new operations
        let wal = WalWriter::open(&wal_path)?;

        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            bundles,
            mmap_bundles: HashMap::new(),
            storage_mode: StorageMode::Heap,
            schemas,
            wal,
            ops_since_checkpoint: 0,
            checkpoint_interval: 10_000,
            compaction_policy: CompactionPolicy::default(),
            last_compaction: std::time::Instant::now(),
            wal_entry_count: replay_entry_count,
            wal_byte_count,
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
        let entry_count = Self::do_replay(
            &wal_path,
            &self.data_dir,
            &mut self.bundles,
            &mut self.schemas,
        )?;
        self.wal_entry_count = entry_count;
        self.wal_byte_count = fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0);
        Ok(())
    }

    /// Open in mmap mode: DHOOM snapshots are memory-mapped, WAL delta
    /// replays into BundleStore overlays (with full index support).
    ///
    /// This is the 32GB→2GB path: only actively queried pages are resident.
    pub fn open_mmap(data_dir: &Path) -> io::Result<Self> {
        fs::create_dir_all(data_dir)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(data_dir, fs::Permissions::from_mode(0o700));
        }

        let wal_path = data_dir.join("gigi.wal");
        let snapshots_dir = data_dir.join("snapshots");

        // Phase 1: Read WAL to get schemas (but don't load DHOOM into heap)
        let mut schemas: HashMap<String, BundleSchema> = HashMap::new();
        let mut wal_entries: Vec<WalEntry> = Vec::new();
        let mut saw_checkpoint = false;

        if wal_path.exists() {
            let mut reader = WalReader::open(&wal_path)?;
            reader.replay(|entry| {
                match &entry {
                    WalEntry::CreateBundle(schema) => {
                        schemas.insert(schema.name.clone(), schema.clone());
                    }
                    WalEntry::Checkpoint => {
                        saw_checkpoint = true;
                    }
                    _ => {}
                }
                // Collect post-checkpoint entries for replay into overlay
                if saw_checkpoint {
                    match &entry {
                        WalEntry::Checkpoint => {}
                        WalEntry::CreateBundle(_) => {}
                        other => wal_entries.push(other.clone()),
                    }
                }
                Ok(())
            })?;
        }

        // Phase 2: Open each .dhoom as MmapBundle, wrap in OverlayBundle
        let mut mmap_bundles: HashMap<String, OverlayBundle> = HashMap::new();
        if snapshots_dir.exists() {
            for (name, schema) in &schemas {
                let snap_path = snapshots_dir.join(format!("{name}.dhoom"));
                if !snap_path.exists() {
                    continue;
                }
                match MmapBundle::open(&snap_path) {
                    Ok(mmap) => {
                        let n = mmap.len();
                        let overlay = OverlayBundle::new(mmap, schema.clone());
                        eprintln!("  Mmap opened: {name} ({n} records)");
                        mmap_bundles.insert(name.clone(), overlay);
                    }
                    Err(e) => {
                        eprintln!("  WARNING: mmap open failed for {name}: {e}");
                    }
                }
            }
        }

        // Phase 3: Replay post-checkpoint WAL entries into overlay BundleStores
        for entry in &wal_entries {
            match entry {
                WalEntry::Insert { bundle_name, record } => {
                    if let Some(ob) = mmap_bundles.get(bundle_name) {
                        ob.insert(record);
                    }
                }
                WalEntry::Update { bundle_name, key, patches } => {
                    if let Some(ob) = mmap_bundles.get(bundle_name) {
                        ob.update(key, patches);
                    }
                }
                WalEntry::Delete { bundle_name, key } => {
                    if let Some(ob) = mmap_bundles.get(bundle_name) {
                        let key_str = format!("{key:?}");
                        ob.delete(&key_str, Some(key));
                    }
                }
                _ => {}
            }
        }

        let wal_byte_count = if wal_path.exists() {
            fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };
        let wal = WalWriter::open(&wal_path)?;

        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            bundles: HashMap::new(),
            mmap_bundles,
            storage_mode: StorageMode::Mmap,
            schemas,
            wal,
            ops_since_checkpoint: 0,
            checkpoint_interval: 10_000,
            compaction_policy: CompactionPolicy::default(),
            last_compaction: std::time::Instant::now(),
            wal_entry_count: wal_entries.len() as u64,
            wal_byte_count,
        })
    }

    /// Current storage mode.
    pub fn storage_mode(&self) -> StorageMode {
        self.storage_mode
    }

    fn do_replay(
        wal_path: &Path,
        data_dir: &Path,
        bundles: &mut HashMap<String, BundleStore>,
        schemas: &mut HashMap<String, BundleSchema>,
    ) -> io::Result<u64> {
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
                WalEntry::MeasurementOverride {
                    bundle_name,
                    field,
                    key,
                    new_measured_value,
                    ..
                } => {
                    // Apply the override: set the field to the measured value
                    if let Some(store) = bundles.get_mut(&bundle_name) {
                        let mut patches = Record::new();
                        patches.insert(field, Value::Float(new_measured_value));
                        store.update(&key, &patches);
                    }
                }
            }
            Ok(())
        })?;

        let elapsed = start.elapsed().as_secs_f64();
        let total: usize = bundles.values().map(|s| s.len()).sum();
        eprintln!("  WAL replay complete: {entry_count} entries, {total} records in {elapsed:.1}s");
        Ok(entry_count)
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
        } else if let Some(ob) = self.mmap_bundles.get(bundle_name) {
            ob.insert(record);
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Bundle '{}' not found", bundle_name),
            ));
        }
        self.maybe_checkpoint()?;
        Ok(())
    }

    /// Extract a deterministic primary-key string from a record for dedup/tombstone purposes.
    /// Uses schema base_fields; falls back to `format!("{rec:?}")` if no schema found.
    fn pk_string(&self, bundle_name: &str, rec: &Record) -> String {
        if let Some(schema) = self.schemas.get(bundle_name) {
            let mut parts: Vec<(&str, &Value)> = schema.base_fields.iter()
                .filter_map(|f| rec.get(&f.name).map(|v| (f.name.as_str(), v)))
                .collect();
            parts.sort_by_key(|(k, _)| *k);
            format!("{parts:?}")
        } else {
            format!("{rec:?}")
        }
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
        } else if let Some(ob) = self.mmap_bundles.get(bundle_name) {
            // Try overlay first; if not found, fetch from base, merge, insert to overlay
            if ob.update(key, patches) {
                true
            } else {
                // Record might be in base — try arithmetic O(1) path, then O(N) scan fallback
                let base_rec = self.mmap_arithmetic_lookup(ob, key)
                    .or_else(|| self.mmap_base_scan(ob, key));
                if let Some(mut base_rec) = base_rec {
                    for (k, v) in patches {
                        base_rec.insert(k.clone(), v.clone());
                    }
                    ob.insert(&base_rec);
                    true
                } else {
                    false
                }
            }
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
        } else if let Some(ob) = self.mmap_bundles.get(bundle_name) {
            let key_str = self.pk_string(bundle_name, key);
            ob.delete(&key_str, Some(key));
            true
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
        let existed = self.bundles.remove(name).is_some()
            || self.mmap_bundles.remove(name).is_some();
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

        // In-memory: dispatch to heap or mmap overlay
        let count = if let Some(store) = self.bundles.get_mut(bundle_name) {
            store.batch_insert(records)
        } else if let Some(ob) = self.mmap_bundles.get(bundle_name) {
            for record in records {
                ob.insert(record);
            }
            records.len()
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Bundle '{}' not found", bundle_name),
            ));
        };

        // Single checkpoint check for entire batch
        self.ops_since_checkpoint += count as u64;
        self.wal_entry_count += count as u64;
        if self.ops_since_checkpoint >= self.checkpoint_interval {
            self.checkpoint()?;
            let wal_path = self.data_dir.join("gigi.wal");
            self.wal_byte_count = fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0);
            self.maybe_auto_compact()?;
        }

        Ok(count)
    }

    /// Point query on a named bundle.
    pub fn point_query(&self, bundle_name: &str, key: &Record) -> io::Result<Option<Record>> {
        if let Some(store) = self.bundles.get(bundle_name) {
            Ok(store.point_query(key))
        } else if let Some(ob) = self.mmap_bundles.get(bundle_name) {
            // Check overlay first (indexed BundleStore)
            if let Some(rec) = ob.point_query_overlay(key) {
                return Ok(Some(rec));
            }
            // Check tombstones — if key was deleted, stop here
            let key_str = self.pk_string(bundle_name, key);
            if ob.is_tombstoned(&key_str) {
                return Ok(None);
            }
            // Arithmetic fast path: O(1) key → index resolution
            if let Some(rec) = self.mmap_arithmetic_lookup(ob, key) {
                return Ok(Some(rec));
            }
            // General fallback: O(N) scan (non-arithmetic keys only)
            for i in 0..ob.base().len() {
                if let Some(val) = ob.base().get(i) {
                    if let serde_json::Value::Object(map) = &val {
                        let mut matches = true;
                        for (k, v) in key {
                            match map.get(k) {
                                Some(jv) if serde_json_to_value(jv) == *v => {}
                                _ => { matches = false; break; }
                            }
                        }
                        if matches {
                            let rec: Record = map.iter()
                                .map(|(k, v)| (k.clone(), serde_json_to_value(v)))
                                .collect();
                            return Ok(Some(rec));
                        }
                    }
                }
            }
            Ok(None)
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Bundle '{}' not found", bundle_name),
            ))
        }
    }

    /// Range query on a named bundle.
    pub fn range_query(
        &self,
        bundle_name: &str,
        field: &str,
        values: &[crate::types::Value],
    ) -> io::Result<Vec<Record>> {
        if let Some(store) = self.bundles.get(bundle_name) {
            Ok(store.range_query(field, values))
        } else if let Some(ob) = self.mmap_bundles.get(bundle_name) {
            // Overlay range query
            let overlay_results = ob.with_overlay(|s| s.range_query(field, values))
                .unwrap_or_default();

            // Collect ALL overlay PKs for dedup (same pattern as filtered_query)
            let overlay_pks: std::collections::HashSet<String> = ob.with_overlay(|s| {
                s.records().map(|r| self.pk_string(bundle_name, &r)).collect()
            }).unwrap_or_default();

            // Base scan: match records where field value is in the value set
            let mut base_results = Vec::new();
            for i in 0..ob.base().len() {
                if let Some(val) = ob.base().get(i) {
                    if let serde_json::Value::Object(map) = &val {
                        let rec = serde_map_to_record(&map);
                        let rec_pk = self.pk_string(bundle_name, &rec);
                        if ob.is_tombstoned(&rec_pk) { continue; }
                        if overlay_pks.contains(&rec_pk) { continue; }
                        if let Some(fv) = rec.get(field) {
                            if values.contains(fv) {
                                base_results.push(rec);
                            }
                        }
                    }
                }
            }
            let mut combined = overlay_results;
            combined.extend(base_results);
            Ok(combined)
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Bundle '{}' not found", bundle_name),
            ))
        }
    }

    /// Get a reference to a bundle store for advanced operations.
    /// In mmap mode, returns the overlay BundleStore (which only has post-snapshot data).
    /// For full data access in mmap mode, use `mmap_bundle()`.
    pub fn bundle(&self, name: &str) -> Option<&BundleStore> {
        self.bundles.get(name)
    }

    /// Get a mutable reference to a bundle store.
    pub fn bundle_mut(&mut self, name: &str) -> Option<&mut BundleStore> {
        self.bundles.get_mut(name)
    }

    /// Get a reference to an mmap overlay bundle.
    pub fn mmap_bundle(&self, name: &str) -> Option<&OverlayBundle> {
        self.mmap_bundles.get(name)
    }

    /// List all bundle names (both heap and mmap).
    pub fn bundle_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.bundles.keys().map(|s| s.as_str()).collect();
        for k in self.mmap_bundles.keys() {
            if !names.contains(&k.as_str()) {
                names.push(k.as_str());
            }
        }
        names
    }

    /// Number of records across all bundles (heap + mmap base + overlay).
    pub fn total_records(&self) -> usize {
        let heap: usize = self.bundles.values().map(|b| b.len()).sum();
        let mmap: usize = self.mmap_bundles.values().map(|ob| {
            ob.base().len() + ob.overlay_len()
        }).sum();
        heap + mmap
    }

    /// Filtered query dispatching to both heap and mmap bundles.
    pub fn filtered_query(
        &self,
        bundle_name: &str,
        conditions: &[QueryCondition],
        or_conditions: Option<&[Vec<QueryCondition>]>,
        sort_by: Option<&str>,
        sort_desc: bool,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> io::Result<Vec<Record>> {
        if let Some(store) = self.bundles.get(bundle_name) {
            Ok(store.filtered_query_ex(conditions, or_conditions, sort_by, sort_desc, limit, offset))
        } else if let Some(ob) = self.mmap_bundles.get(bundle_name) {
            // Overlay query (uses hash index acceleration)
            let overlay_results = ob.with_overlay(|s| {
                s.filtered_query_ex(conditions, or_conditions, sort_by, sort_desc, None, None)
            }).unwrap_or_default();

            // Collect ALL overlay primary keys for dedup (not just filtered results,
            // because any overlay record supersedes its base counterpart regardless of filter)
            let overlay_keys: std::collections::HashSet<String> = ob.with_overlay(|s| {
                s.records().map(|r| self.pk_string(bundle_name, &r)).collect()
            }).unwrap_or_default();

            // Base scan with filter — skip records whose key is in overlay or tombstoned
            let mut base_results = Vec::new();
            for i in 0..ob.base().len() {
                if let Some(val) = ob.base().get(i) {
                    if let serde_json::Value::Object(map) = &val {
                        let rec = serde_map_to_record(&map);
                        let rec_pk = self.pk_string(bundle_name, &rec);
                        if ob.is_tombstoned(&rec_pk) { continue; }
                        if overlay_keys.contains(&rec_pk) { continue; }
                        if crate::bundle::matches_filter(&rec, conditions, or_conditions) {
                            base_results.push(rec);
                        }
                    }
                }
            }

            let mut combined = overlay_results;
            combined.extend(base_results);

            // Apply sort
            if let Some(field) = sort_by {
                let field = field.to_string();
                combined.sort_by(|a, b| {
                    let va = a.get(&field).unwrap_or(&Value::Null);
                    let vb = b.get(&field).unwrap_or(&Value::Null);
                    if sort_desc { vb.cmp(va) } else { va.cmp(vb) }
                });
            }

            // Apply offset + limit
            if let Some(off) = offset {
                if off < combined.len() {
                    combined = combined.split_off(off);
                } else {
                    combined.clear();
                }
            }
            if let Some(lim) = limit {
                combined.truncate(lim);
            }

            Ok(combined)
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Bundle '{}' not found", bundle_name),
            ))
        }
    }

    /// O(1) point query on mmap base using arithmetic key resolution.
    /// Returns None if the key field isn't arithmetic or doesn't match.
    fn mmap_arithmetic_lookup(&self, ob: &OverlayBundle, key: &Record) -> Option<Record> {
        let fiber = ob.base().fiber();
        for fdecl in &fiber.fields {
            if let Some(crate::dhoom::Modifier::Arithmetic { ref start, ref step }) = fdecl.modifier {
                let key_val = key.get(&fdecl.name)?;
                let s = step.unwrap_or(1);
                // Extract integer key value
                let key_i = match key_val {
                    Value::Integer(i) => *i,
                    Value::Float(f) => *f as i64,
                    _ => return None,
                };
                // Extract integer start value from serde_json::Value
                let start_i = match start {
                    serde_json::Value::Number(n) => n.as_i64()?,
                    _ => return None,
                };
                if s == 0 { return None; }
                let diff = key_i - start_i;
                if diff < 0 || diff % s != 0 { return None; }
                let idx = (diff / s) as usize;
                if idx >= ob.base().len() { return None; }

                let val = ob.base().get(idx)?;
                if let serde_json::Value::Object(map) = &val {
                    // Verify the key field matches (guards against hash collisions in
                    // non-contiguous arithmetic sequences)
                    let rec = serde_map_to_record(&map);
                    if rec.get(&fdecl.name) == Some(key_val) {
                        return Some(rec);
                    }
                }
                return None;
            }
        }
        None
    }

    /// O(N) fallback scan on mmap base — used when arithmetic lookup doesn't apply.
    fn mmap_base_scan(&self, ob: &OverlayBundle, key: &Record) -> Option<Record> {
        for i in 0..ob.base().len() {
            if let Some(val) = ob.base().get(i) {
                if let serde_json::Value::Object(map) = &val {
                    let mut matches = true;
                    for (k, v) in key {
                        match map.get(k) {
                            Some(jv) if serde_json_to_value(jv) == *v => {}
                            _ => { matches = false; break; }
                        }
                    }
                    if matches {
                        return Some(serde_map_to_record(&map));
                    }
                }
            }
        }
        None
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
        // Reset WAL tracking for auto-compaction
        self.wal_entry_count = self.schemas.len() as u64 + 1;
        self.wal_byte_count = fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0);
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
        self.snapshot_with_chunk_size(50_000)
    }

    // ── CoW Snapshot (Feature #3) ─────────────────────────────────────────

    /// Clone all bundle data into owned vecs. The caller holds `&self` (read
    /// lock) only for the duration of this call. The returned data can then
    /// be encoded to DHOOM files without any lock.
    pub fn clone_bundle_data(&self) -> Vec<BundleDataClone> {
        assert!(
            self.mmap_bundles.is_empty(),
            "clone_bundle_data() called with mmap bundles present — would silently drop mmap-resident data. \
             Use rebase() to drain overlays into new snapshots before snapshotting."
        );
        self.bundles
            .iter()
            .filter(|(_, store)| store.len() > 0)
            .map(|(name, store)| BundleDataClone {
                name: name.clone(),
                schema: self.schemas.get(name).cloned().unwrap_or_else(|| {
                    BundleSchema::new(name)
                }),
                records: store.records().map(|r| record_to_serde_json(&r)).collect(),
            })
            .collect()
    }

    /// Encode pre-cloned bundle data to DHOOM snapshot files.
    /// Does NOT require any engine lock — operates on owned data and the filesystem.
    pub fn write_snapshot_files(
        data_dir: &Path,
        bundles: &[BundleDataClone],
        chunk_size: usize,
    ) -> io::Result<usize> {
        let snapshots_dir = data_dir.join("snapshots");
        fs::create_dir_all(&snapshots_dir)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&snapshots_dir, fs::Permissions::from_mode(0o700));
        }

        let mut total = 0usize;
        for bdc in bundles {
            let snap_path = snapshots_dir.join(format!("{}.dhoom", bdc.name));
            let tmp_path = snapshots_dir.join(format!("{}.dhoom.tmp", bdc.name));

            eprintln!(
                "  CoW snapshot streaming: {} ({} records, chunk_size={chunk_size})…",
                bdc.name,
                bdc.records.len()
            );
            {
                let file = fs::File::create(&tmp_path)?;
                let buf = io::BufWriter::new(file);
                let mut encoder =
                    crate::dhoom::StreamingDhoomEncoder::new(buf, &bdc.name, chunk_size);
                for rec in &bdc.records {
                    encoder.push(rec.clone())?;
                }
                encoder.finish()?;
            }
            fs::rename(&tmp_path, &snap_path)?;
            total += bdc.records.len();
        }
        Ok(total)
    }

    /// Compact the WAL to schema-only entries (called after snapshot files
    /// have been written). Requires `&mut self` (write lock).
    pub fn compact_wal_to_schemas(&mut self) -> io::Result<()> {
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
        self.wal_entry_count = self.schemas.len() as u64 + 1;
        self.wal_byte_count = fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0);
        Ok(())
    }

    /// CoW snapshot: clone data (brief `&self`), encode to files (no lock),
    /// then compact WAL (`&mut self`). When called from single-threaded
    /// context, this is equivalent to `snapshot()`.
    pub fn cow_snapshot(&mut self) -> io::Result<usize> {
        let cloned = self.clone_bundle_data();
        let total = Self::write_snapshot_files(&self.data_dir, &cloned, 50_000)?;
        self.compact_wal_to_schemas()?;
        Ok(total)
    }

    /// Get the data directory (for external snapshot file writing).
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    // ── End CoW Snapshot ──────────────────────────────────────────────────

    /// Streaming snapshot — encodes bundles to DHOOM in constant memory.
    /// `chunk_size` controls how many records are buffered before flushing.
    pub fn snapshot_with_chunk_size(&mut self, chunk_size: usize) -> io::Result<usize> {
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

            let count = store.len();
            if count == 0 {
                continue;
            }

            eprintln!("  Snapshot streaming: {name} ({count} records, chunk_size={chunk_size})…");
            {
                let file = fs::File::create(&tmp_path)?;
                let buf = io::BufWriter::new(file);
                let mut encoder =
                    crate::dhoom::StreamingDhoomEncoder::new(buf, name, chunk_size);

                for rec in store.records() {
                    encoder.push(record_to_serde_json(&rec))?;
                }
                encoder.finish()?;
                eprintln!("  Snapshot written: {name} ({count} records)");
            }

            fs::rename(&tmp_path, &snap_path)?;
            total_records += count;
        }

        // Compact WAL to schema-only (no insert entries).
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
        // Reset WAL tracking for auto-compaction
        self.wal_entry_count = self.schemas.len() as u64 + 1;
        self.wal_byte_count = fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0);

        Ok(total_records)
    }

    fn maybe_checkpoint(&mut self) -> io::Result<()> {
        self.ops_since_checkpoint += 1;
        self.wal_entry_count += 1;
        if self.ops_since_checkpoint >= self.checkpoint_interval {
            self.checkpoint()?;
            // Refresh WAL byte count after flush (metadata is now accurate)
            let wal_path = self.data_dir.join("gigi.wal");
            self.wal_byte_count = fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0);
            self.maybe_auto_compact()?;
        }
        Ok(())
    }

    /// Check the compaction policy and run snapshot() if any trigger fires.
    /// Called at each checkpoint boundary (every checkpoint_interval ops).
    fn maybe_auto_compact(&mut self) -> io::Result<()> {
        if self.compaction_policy.disabled {
            return Ok(());
        }

        let n_eff = self.total_records().max(1) as f64;
        let a = self.wal_entry_count as f64 / n_eff;
        let elapsed = self.last_compaction.elapsed().as_secs();

        let should_compact = (a > self.compaction_policy.amplification_threshold
            && elapsed >= self.compaction_policy.min_interval_secs)
            || self.wal_entry_count > self.compaction_policy.max_wal_entries
            || self.wal_byte_count > self.compaction_policy.max_wal_bytes;

        if should_compact {
            if self.mmap_bundles.is_empty() {
                self.cow_snapshot()?;
            } else {
                self.mmap_rebase_snapshot()?;
            }
            // cow_snapshot / mmap_rebase_snapshot calls compact_wal_to_schemas()
            // which resets wal_entry_count and wal_byte_count. Update timestamp.
            self.last_compaction = std::time::Instant::now();
        }
        Ok(())
    }

    /// Rebase mmap bundles: merge base + overlay − tombstones into a fresh DHOOM
    /// snapshot, then swap the base and clear the overlay.
    /// This is the mmap-mode equivalent of cow_snapshot().
    fn mmap_rebase_snapshot(&mut self) -> io::Result<()> {
        let snapshots_dir = self.data_dir.join("snapshots");
        fs::create_dir_all(&snapshots_dir)?;

        let names: Vec<String> = self.mmap_bundles.keys().cloned().collect();
        for name in &names {
            let ob = self.mmap_bundles.get(name).unwrap();

            // Collect overlay PK set for dedup against base
            let overlay_pks: std::collections::HashSet<String> = ob.with_overlay(|s| {
                s.records().map(|r| self.pk_string(name, &r)).collect()
            }).unwrap_or_default();

            // Collect merged records: base (non-tombstoned, non-superseded) + overlay
            let mut merged: Vec<serde_json::Value> = Vec::new();

            for i in 0..ob.base().len() {
                if let Some(val) = ob.base().get(i) {
                    if let serde_json::Value::Object(ref map) = val {
                        let rec = serde_map_to_record(map);
                        let pk = self.pk_string(name, &rec);
                        if ob.is_tombstoned(&pk) { continue; }
                        if overlay_pks.contains(&pk) { continue; }
                        merged.push(record_to_serde_json(&rec));
                    }
                }
            }

            let overlay_recs: Vec<Record> = ob.with_overlay(|s| {
                s.records().collect()
            }).unwrap_or_default();
            for rec in &overlay_recs {
                merged.push(record_to_serde_json(rec));
            }

            // Sort by arithmetic key field (if any) to preserve O(1) lookup after rebase.
            // detect_arithmetic() in the encoder requires uniform consecutive diffs.
            let arith_field: Option<String> = ob.base().fiber().fields.iter()
                .find(|f| matches!(f.modifier, Some(crate::dhoom::Modifier::Arithmetic { .. })))
                .map(|f| f.name.clone());
            if let Some(ref af) = arith_field {
                merged.sort_by(|a, b| {
                    let va = a.get(af).and_then(|v| v.as_i64()).unwrap_or(i64::MAX);
                    let vb = b.get(af).and_then(|v| v.as_i64()).unwrap_or(i64::MAX);
                    va.cmp(&vb)
                });
            }

            // Encode to DHOOM
            let snap_path = snapshots_dir.join(format!("{name}.dhoom"));
            let tmp_path = snapshots_dir.join(format!("{name}.dhoom.tmp"));
            {
                let file = fs::File::create(&tmp_path)?;
                let buf = io::BufWriter::new(file);
                let mut encoder =
                    crate::dhoom::StreamingDhoomEncoder::new(buf, name, 50_000);
                for val in &merged {
                    encoder.push(val.clone())?;
                }
                encoder.finish()?;
            }

            fs::rename(&tmp_path, &snap_path)?;

            // Open new mmap base and rebase the overlay
            let new_base = MmapBundle::open(&snap_path)?;
            let schema = self.schemas.get(name).cloned().unwrap_or_else(|| {
                BundleSchema::new(name)
            });
            eprintln!("  Rebase: {name} ({} records)", new_base.len());
            self.mmap_bundles.get_mut(name).unwrap().rebase(new_base, schema);
        }

        self.compact_wal_to_schemas()?;
        Ok(())
    }

    /// Access the compaction policy for configuration.
    pub fn compaction_policy_mut(&mut self) -> &mut CompactionPolicy {
        &mut self.compaction_policy
    }

    /// Set the checkpoint interval (how many ops between checkpoints).
    /// Also controls how often auto-compaction is evaluated.
    pub fn set_checkpoint_interval(&mut self, interval: u64) {
        self.checkpoint_interval = interval;
    }

    /// Current WAL entry count (for testing / monitoring).
    pub fn wal_entry_count(&self) -> u64 {
        self.wal_entry_count
    }

    /// Current WAL byte count (for testing / monitoring).
    pub fn wal_byte_count(&self) -> u64 {
        self.wal_byte_count
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

/// Convert a serde_json Map to a GIGI Record.
fn serde_map_to_record(map: &serde_json::Map<String, serde_json::Value>) -> Record {
    map.iter()
        .map(|(k, v)| (k.clone(), serde_json_to_value(v)))
        .collect()
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

    // -------------------------------------------------------------------
    // Feature #2 — Streaming DHOOM Snapshot TDD
    // -------------------------------------------------------------------

    /// Test 2.1 / 2.7: Streaming snapshot roundtrip — data survives snapshot + reopen.
    /// Also tests idempotency: two successive snapshots produce same result.
    #[test]
    fn streaming_snapshot_roundtrip() {
        let dir = test_dir("stream_snap_rt");
        cleanup(&dir);

        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("drugs")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("name"))
                .fiber(FieldDef::numeric("mw").with_range(1000.0));
            engine.create_bundle(schema).unwrap();

            for i in 0..500i64 {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("name".into(), Value::Text(format!("Drug_{i}")));
                rec.insert("mw".into(), Value::Float(100.0 + i as f64 * 0.3));
                engine.insert("drugs", &rec).unwrap();
            }

            // Streaming snapshot with small chunk size to exercise chunking
            let n = engine.snapshot_with_chunk_size(100).unwrap();
            assert_eq!(n, 500);
        }

        // Reopen and verify all records survived
        {
            let engine = Engine::open(&dir).unwrap();
            assert_eq!(engine.total_records(), 500);

            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(250));
            let r = engine.point_query("drugs", &key).unwrap().unwrap();
            assert_eq!(r.get("name"), Some(&Value::Text("Drug_250".into())));
        }

        // Idempotency: snapshot again, reopen, same data
        {
            let mut engine = Engine::open(&dir).unwrap();
            let n = engine.snapshot_with_chunk_size(100).unwrap();
            assert_eq!(n, 500);
        }
        {
            let engine = Engine::open(&dir).unwrap();
            assert_eq!(engine.total_records(), 500);
        }

        cleanup(&dir);
    }

    /// Streaming snapshot with chunk_size=1 (extreme: every record is its own chunk).
    #[test]
    fn streaming_snapshot_chunk_size_one() {
        let dir = test_dir("stream_snap_c1");
        cleanup(&dir);

        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("items")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("label"));
            engine.create_bundle(schema).unwrap();

            for i in 0..50i64 {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("label".into(), Value::Text(format!("item_{i}")));
                engine.insert("items", &rec).unwrap();
            }
            engine.snapshot_with_chunk_size(1).unwrap();
        }

        let engine = Engine::open(&dir).unwrap();
        assert_eq!(engine.total_records(), 50);
        cleanup(&dir);
    }

    /// Streaming snapshot + post-snapshot inserts survive reopen.
    #[test]
    fn streaming_snapshot_then_new_inserts() {
        let dir = test_dir("stream_snap_post");
        cleanup(&dir);

        // snapshot 200 records
        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("compounds")
                .base(FieldDef::numeric("cid"))
                .fiber(FieldDef::categorical("smiles"));
            engine.create_bundle(schema).unwrap();

            for i in 0..200i64 {
                let mut rec = Record::new();
                rec.insert("cid".into(), Value::Integer(i));
                rec.insert("smiles".into(), Value::Text(format!("C{i}")));
                engine.insert("compounds", &rec).unwrap();
            }
            engine.snapshot_with_chunk_size(50).unwrap();
        }

        // add 50 more after snapshot
        {
            let mut engine = Engine::open(&dir).unwrap();
            assert_eq!(engine.total_records(), 200);
            for i in 200..250i64 {
                let mut rec = Record::new();
                rec.insert("cid".into(), Value::Integer(i));
                rec.insert("smiles".into(), Value::Text(format!("C{i}")));
                engine.insert("compounds", &rec).unwrap();
            }
            engine.checkpoint().unwrap();
        }

        // reopen: 250 total (200 DHOOM + 50 WAL)
        {
            let engine = Engine::open(&dir).unwrap();
            assert_eq!(engine.total_records(), 250);
        }
        cleanup(&dir);
    }

    /// Multiple bundles snapshot with streaming.
    #[test]
    fn streaming_snapshot_multiple_bundles() {
        let dir = test_dir("stream_snap_multi");
        cleanup(&dir);

        {
            let mut engine = Engine::open(&dir).unwrap();
            let s1 = BundleSchema::new("alpha")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("val"));
            let s2 = BundleSchema::new("beta")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::numeric("score").with_range(100.0));
            engine.create_bundle(s1).unwrap();
            engine.create_bundle(s2).unwrap();

            for i in 0..100i64 {
                let mut r1 = Record::new();
                r1.insert("id".into(), Value::Integer(i));
                r1.insert("val".into(), Value::Text(format!("a{i}")));
                engine.insert("alpha", &r1).unwrap();

                let mut r2 = Record::new();
                r2.insert("id".into(), Value::Integer(i));
                r2.insert("score".into(), Value::Float(i as f64 * 0.5));
                engine.insert("beta", &r2).unwrap();
            }
            engine.snapshot_with_chunk_size(30).unwrap();
        }

        let engine = Engine::open(&dir).unwrap();
        assert_eq!(engine.bundle_names().len(), 2);
        assert_eq!(engine.total_records(), 200);
        cleanup(&dir);
    }

    // -------------------------------------------------------------------
    // Feature #1 — Auto-Compaction TDD
    // -------------------------------------------------------------------

    /// Helper: create an engine with auto-compaction disabled by default
    /// (tests enable specific policies as needed).
    fn engine_no_autocompact(dir: &Path) -> Engine {
        let mut engine = Engine::open(dir).unwrap();
        engine.compaction_policy_mut().disabled = true;
        engine.set_checkpoint_interval(10); // check frequently in tests
        engine
    }

    fn insert_n(engine: &mut Engine, bundle: &str, n: usize) {
        for i in 0..n {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i as i64));
            r.insert("val".into(), Value::Text(format!("v{i}")));
            engine.insert(bundle, &r).unwrap();
        }
    }

    /// Test 1.1 — Amplification trigger: A > 3.0 with cooldown elapsed → fires.
    #[test]
    fn autocompact_amplification_trigger() {
        let dir = test_dir("ac_amp_trigger");
        cleanup(&dir);

        let mut engine = engine_no_autocompact(&dir);
        let schema = BundleSchema::new("data")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("val"));
        engine.create_bundle(schema).unwrap();

        // Insert 100 records (WAL entries ≈ 101 including schema)
        insert_n(&mut engine, "data", 100);
        assert_eq!(engine.total_records(), 100);

        // Now add 200 updates to same records → WAL entries ≈ 301
        // A = 301/100 = 3.01 > 3.0
        for i in 0..200 {
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(i % 100));
            let mut patches = Record::new();
            patches.insert("val".into(), Value::Text(format!("u{i}")));
            engine.update("data", &key, &patches).unwrap();
        }

        let pre_wal = engine.wal_entry_count();
        let a = pre_wal as f64 / engine.total_records().max(1) as f64;
        assert!(a > 3.0, "amplification should exceed threshold: A={a}");

        // Enable compaction with 0 cooldown so it fires immediately
        engine.compaction_policy_mut().disabled = false;
        engine.compaction_policy_mut().min_interval_secs = 0;

        // Insert updates (not new records) to cross a checkpoint boundary
        // without lowering A by increasing total_records.
        for i in 0..15 {
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(i % 100));
            let mut patches = Record::new();
            patches.insert("val".into(), Value::Text(format!("trig_{i}")));
            engine.update("data", &key, &patches).unwrap();
        }

        // Post-compaction: A should be near 1.0
        let post_a = engine.wal_entry_count() as f64 / engine.total_records().max(1) as f64;
        assert!(
            post_a < 1.5,
            "post-compaction A should be ~1.0, got {post_a}"
        );

        cleanup(&dir);
    }

    /// Test 1.2 — Cooldown prevents rapid re-compaction.
    #[test]
    fn autocompact_cooldown_prevents_refire() {
        let dir = test_dir("ac_cooldown");
        cleanup(&dir);

        let mut engine = engine_no_autocompact(&dir);
        let schema = BundleSchema::new("data")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("val"));
        engine.create_bundle(schema).unwrap();

        insert_n(&mut engine, "data", 100);

        // Push A > threshold
        for i in 0..250 {
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(i % 100));
            let mut patches = Record::new();
            patches.insert("val".into(), Value::Text(format!("u{i}")));
            engine.update("data", &key, &patches).unwrap();
        }

        // Enable with 9999s cooldown — should NOT fire
        engine.compaction_policy_mut().disabled = false;
        engine.compaction_policy_mut().min_interval_secs = 9999;

        let pre_wal = engine.wal_entry_count();

        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(888));
        r.insert("val".into(), Value::Text("nope".into()));
        engine.insert("data", &r).unwrap();

        // WAL should have grown, not shrunk
        assert!(
            engine.wal_entry_count() > pre_wal,
            "cooldown should prevent compaction"
        );

        cleanup(&dir);
    }

    /// Test 1.3 — Absolute WAL entry limit overrides cooldown.
    #[test]
    fn autocompact_absolute_limit_overrides_cooldown() {
        let dir = test_dir("ac_abs_limit");
        cleanup(&dir);

        let mut engine = Engine::open(&dir).unwrap();
        let schema = BundleSchema::new("data")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("val"));
        engine.create_bundle(schema).unwrap();
        engine.set_checkpoint_interval(10);

        // Set very low absolute limit
        engine.compaction_policy_mut().max_wal_entries = 50;
        engine.compaction_policy_mut().min_interval_secs = 999_999; // huge cooldown
        engine.compaction_policy_mut().amplification_threshold = 999.0; // disable amp trigger

        // Insert 60 records — should trigger at entry 51
        insert_n(&mut engine, "data", 60);

        // After compaction, WAL entry count should be small (schemas + checkpoint)
        assert!(
            engine.wal_entry_count() < 50,
            "absolute limit should have triggered compaction, got {}",
            engine.wal_entry_count()
        );
        assert_eq!(engine.total_records(), 60);

        cleanup(&dir);
    }

    /// Test 1.4 — Disabled policy never fires.
    #[test]
    fn autocompact_disabled_never_fires() {
        let dir = test_dir("ac_disabled");
        cleanup(&dir);

        let mut engine = engine_no_autocompact(&dir);
        let schema = BundleSchema::new("data")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("val"));
        engine.create_bundle(schema).unwrap();

        // Even with absurd amplification, disabled means no compaction
        engine.compaction_policy_mut().amplification_threshold = 0.001;
        engine.compaction_policy_mut().min_interval_secs = 0;

        insert_n(&mut engine, "data", 200);

        // WAL should have grown, never shrunk
        assert!(
            engine.wal_entry_count() >= 200,
            "disabled policy should not compact"
        );

        cleanup(&dir);
    }

    /// Test 1.5 — Post-compaction WAL invariant: entry count = |schemas| + 1.
    #[test]
    fn autocompact_post_compaction_wal_invariant() {
        let dir = test_dir("ac_post_invariant");
        cleanup(&dir);

        let mut engine = engine_no_autocompact(&dir);
        let s1 = BundleSchema::new("alpha")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("val"));
        let s2 = BundleSchema::new("beta")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("score").with_range(100.0));
        engine.create_bundle(s1).unwrap();
        engine.create_bundle(s2).unwrap();

        insert_n(&mut engine, "alpha", 500);
        for i in 0..500i64 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("score".into(), Value::Float(i as f64 * 0.1));
            engine.insert("beta", &r).unwrap();
        }

        engine.snapshot().unwrap();

        // WAL entry count should be |schemas| + 1 (checkpoint)
        let expected = engine.bundle_names().len() as u64 + 1;
        assert_eq!(
            engine.wal_entry_count(),
            expected,
            "post-snapshot WAL should have schema entries + checkpoint"
        );
        // A = (schemas+1)/1000 < 1.0
        let a = engine.wal_entry_count() as f64 / engine.total_records().max(1) as f64;
        assert!(a < 1.0, "post-compaction A should be < 1.0, got {a}");

        cleanup(&dir);
    }

    /// Test 1.6 — Amplification monotone decreasing under pure inserts.
    #[test]
    fn autocompact_amplification_monotone_inserts() {
        let dir = test_dir("ac_amp_monotone");
        cleanup(&dir);

        let mut engine = engine_no_autocompact(&dir);
        let schema = BundleSchema::new("data")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("val"));
        engine.create_bundle(schema).unwrap();

        let mut prev_a = f64::MAX;
        for batch in 1..=10 {
            for i in 0..50 {
                let idx = (batch - 1) * 50 + i;
                let mut r = Record::new();
                r.insert("id".into(), Value::Integer(idx as i64));
                r.insert("val".into(), Value::Text(format!("v{idx}")));
                engine.insert("data", &r).unwrap();
            }
            let n = engine.total_records().max(1) as f64;
            let a = engine.wal_entry_count() as f64 / n;
            // For pure inserts: A = (schema_entries + N) / N → 1.0 as N grows
            // Monotone decreasing (or equal) after the first few
            if batch > 1 {
                assert!(
                    a <= prev_a + 0.01, // small tolerance for rounding
                    "A should be monotone decreasing: batch {batch}, prev={prev_a}, cur={a}"
                );
            }
            prev_a = a;
        }
        // Final A should be close to 1.0
        assert!(prev_a < 1.5, "final A under pure inserts should approach 1.0");

        cleanup(&dir);
    }

    /// Test 1.7 — Amplification increases under updates, triggers compaction.
    #[test]
    fn autocompact_amplification_increases_under_updates() {
        let dir = test_dir("ac_amp_updates");
        cleanup(&dir);

        let mut engine = Engine::open(&dir).unwrap();
        let schema = BundleSchema::new("data")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("val"));
        engine.create_bundle(schema).unwrap();
        engine.set_checkpoint_interval(10);

        // Disable compaction while we set up
        engine.compaction_policy_mut().disabled = true;

        insert_n(&mut engine, "data", 100);

        // Snapshot to reset WAL
        engine.snapshot().unwrap();
        let a_after_snap = engine.wal_entry_count() as f64 / 100.0;
        assert!(a_after_snap < 1.0, "A after snapshot should be < 1.0");

        // Now do 350 updates to the same 100 records → A = (2 + 350)/100 = 3.52 > 3.0
        for i in 0..350 {
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(i % 100));
            let mut patches = Record::new();
            patches.insert("val".into(), Value::Text(format!("upd{i}")));
            engine.update("data", &key, &patches).unwrap();
        }

        let a = engine.wal_entry_count() as f64 / 100.0;
        assert!(a > 3.0, "A after 350 updates should be > 3.0, got {a}");

        // Enable compaction with 0 cooldown
        engine.compaction_policy_mut().disabled = false;
        engine.compaction_policy_mut().min_interval_secs = 0;

        // Updates (not new inserts) to cross checkpoint boundary without inflating total_records
        for i in 0..15 {
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(i % 100));
            let mut patches = Record::new();
            patches.insert("val".into(), Value::Text(format!("fire_{i}")));
            engine.update("data", &key, &patches).unwrap();
        }

        let post_a = engine.wal_entry_count() as f64 / engine.total_records().max(1) as f64;
        assert!(post_a < 1.5, "compaction should have fired, A={post_a}");

        cleanup(&dir);
    }

    /// Test 1.8 — WAL file size trigger overrides amplification.
    #[test]
    fn autocompact_wal_file_size_trigger() {
        let dir = test_dir("ac_filesize");
        cleanup(&dir);

        let mut engine = Engine::open(&dir).unwrap();
        let schema = BundleSchema::new("data")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("val"));
        engine.create_bundle(schema).unwrap();
        engine.set_checkpoint_interval(10);

        // Disable amp and entry triggers, set tiny file size trigger
        engine.compaction_policy_mut().amplification_threshold = 999.0;
        engine.compaction_policy_mut().max_wal_entries = u64::MAX;
        engine.compaction_policy_mut().min_interval_secs = 0;
        // Set max_wal_bytes very low so file size trigger fires
        engine.compaction_policy_mut().max_wal_bytes = 500;

        // Insert enough records to exceed 500 bytes WAL
        for i in 0..50 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert(
                "val".into(),
                Value::Text(format!("padding_{:0>20}", i)),
            );
            engine.insert("data", &r).unwrap();
        }

        // After file size trigger, WAL should be small
        // (compaction happened, WAL is schema-only)
        assert!(
            engine.wal_byte_count() < 500,
            "file size trigger should have compacted WAL, bytes={}",
            engine.wal_byte_count()
        );
        assert_eq!(engine.total_records(), 50);

        cleanup(&dir);
    }

    /// Test 1.9 — Data survives compaction cycle: insert → auto-compact → reopen.
    #[test]
    fn autocompact_data_survives_cycle() {
        let dir = test_dir("ac_survives");
        cleanup(&dir);

        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("drugs")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("name"))
                .fiber(FieldDef::numeric("mw").with_range(1000.0));
            engine.create_bundle(schema).unwrap();
            engine.set_checkpoint_interval(10);

            // Low limit to force auto-compaction during inserts
            engine.compaction_policy_mut().max_wal_entries = 100;
            engine.compaction_policy_mut().min_interval_secs = 0;
            engine.compaction_policy_mut().amplification_threshold = 999.0;

            for i in 0..250i64 {
                let mut r = Record::new();
                r.insert("id".into(), Value::Integer(i));
                r.insert("name".into(), Value::Text(format!("drug_{i}")));
                r.insert("mw".into(), Value::Float(100.0 + i as f64));
                engine.insert("drugs", &r).unwrap();
            }

            assert_eq!(engine.total_records(), 250);
        }

        // Reopen and verify all data survived
        let engine = Engine::open(&dir).unwrap();
        assert_eq!(engine.total_records(), 250);

        // Spot check
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(42));
        let found = engine.point_query("drugs", &key).unwrap().unwrap();
        assert_eq!(found.get("name"), Some(&Value::Text("drug_42".into())));

        cleanup(&dir);
    }

    // -------------------------------------------------------------------
    // Feature #3 — CoW Snapshots TDD
    // -------------------------------------------------------------------

    /// Test 3.7: clone_bundle_data captures point-in-time state, and
    /// write_snapshot_files encodes it correctly. Data survives reopen.
    #[test]
    fn cow_snapshot_roundtrip() {
        let dir = test_dir("cow_rt");
        cleanup(&dir);

        {
            let mut engine = engine_no_autocompact(&dir);
            let schema = BundleSchema::new("tissue")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("name"))
                .fiber(FieldDef::numeric("volume").with_range(500.0));
            engine.create_bundle(schema).unwrap();

            for i in 0..200i64 {
                let mut r = Record::new();
                r.insert("id".into(), Value::Integer(i));
                r.insert("name".into(), Value::Text(format!("t_{i}")));
                r.insert("volume".into(), Value::Float(i as f64 * 1.5));
                engine.insert("tissue", &r).unwrap();
            }

            // Use cow_snapshot (clone → encode → compact WAL)
            let n = engine.cow_snapshot().unwrap();
            assert_eq!(n, 200);
        }

        // Reopen and verify
        let engine = Engine::open(&dir).unwrap();
        assert_eq!(engine.total_records(), 200);

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(99));
        let found = engine.point_query("tissue", &key).unwrap().unwrap();
        assert_eq!(found.get("name"), Some(&Value::Text("t_99".into())));

        cleanup(&dir);
    }

    /// Test 3.8: CoW snapshot captures point-in-time — records inserted AFTER
    /// clone_bundle_data are NOT in the snapshot, but ARE in the live engine.
    #[test]
    fn cow_snapshot_point_in_time() {
        let dir = test_dir("cow_pit");
        cleanup(&dir);

        let mut engine = engine_no_autocompact(&dir);
        let schema = BundleSchema::new("drugs")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("name"));
        engine.create_bundle(schema).unwrap();

        for i in 0..100i64 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("name".into(), Value::Text(format!("d_{i}")));
            engine.insert("drugs", &r).unwrap();
        }

        // Step 1: clone data (simulating read lock capture)
        let cloned = engine.clone_bundle_data();
        assert_eq!(cloned.len(), 1);
        assert_eq!(cloned[0].records.len(), 100);

        // Step 2: insert more records AFTER clone (simulating writes during encoding)
        for i in 100..150i64 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("name".into(), Value::Text(format!("d_{i}")));
            engine.insert("drugs", &r).unwrap();
        }
        assert_eq!(engine.total_records(), 150);

        // Step 3: write snapshot files from the clone (should have 100, not 150)
        let n = Engine::write_snapshot_files(&engine.data_dir(), &cloned, 50_000).unwrap();
        assert_eq!(n, 100, "snapshot should contain exactly 100 records from clone");

        // Step 4: compact WAL
        engine.compact_wal_to_schemas().unwrap();

        // Step 5: reopen — snapshot has 100 records, but WAL had 150 records
        // before compaction. After compact_wal_to_schemas, WAL only has schemas.
        // The 50 post-snapshot records are lost (this is expected behavior —
        // the caller should only compact WAL after confirming all data is in snapshot).
        // In practice, the 50 new inserts would remain in WAL because
        // compact_wal_to_schemas only removes insert entries.
        drop(engine);
        let engine = Engine::open(&dir).unwrap();
        // Only the 100 snapshotted records survive (WAL was compacted)
        assert_eq!(engine.total_records(), 100);

        cleanup(&dir);
    }

    /// Test 3.7b: Reads continue working while cow_snapshot processes.
    /// Since we're single-threaded in tests, we simulate this by verifying
    /// that clone_bundle_data() doesn't mutate the engine and reads work
    /// before, during (after clone), and after snapshot.
    #[test]
    fn cow_snapshot_reads_unblocked() {
        let dir = test_dir("cow_reads");
        cleanup(&dir);

        let mut engine = engine_no_autocompact(&dir);
        let schema = BundleSchema::new("data")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("val"));
        engine.create_bundle(schema).unwrap();

        insert_n(&mut engine, "data", 100);

        // Read works before clone
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(50));
        assert!(engine.point_query("data", &key).unwrap().is_some());

        // Clone data (simulates moment read lock is held)
        let cloned = engine.clone_bundle_data();

        // Read still works after clone (engine is not modified)
        assert!(engine.point_query("data", &key).unwrap().is_some());
        assert_eq!(engine.total_records(), 100);

        // Write also works after clone
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(999));
        r.insert("val".into(), Value::Text("new".into()));
        engine.insert("data", &r).unwrap();
        assert_eq!(engine.total_records(), 101);

        // Write snapshot from clone
        let n = Engine::write_snapshot_files(&engine.data_dir(), &cloned, 50_000).unwrap();
        assert_eq!(n, 100);

        // Engine still fully functional
        assert_eq!(engine.total_records(), 101);

        cleanup(&dir);
    }

    /// Test 3.3: Disjoint write commutativity — inserting to different bundles
    /// in any order produces the same state.
    #[test]
    fn cow_disjoint_write_commutativity() {
        let dir_a = test_dir("cow_commute_a");
        let dir_b = test_dir("cow_commute_b");
        cleanup(&dir_a);
        cleanup(&dir_b);

        // Order A: bundle alpha first, then beta
        {
            let mut engine = engine_no_autocompact(&dir_a);
            let s1 = BundleSchema::new("alpha")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::numeric("x").with_range(100.0));
            let s2 = BundleSchema::new("beta")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::numeric("y").with_range(100.0));
            engine.create_bundle(s1).unwrap();
            engine.create_bundle(s2).unwrap();

            for i in 0..50i64 {
                let mut r = Record::new();
                r.insert("id".into(), Value::Integer(i));
                r.insert("x".into(), Value::Float(i as f64));
                engine.insert("alpha", &r).unwrap();
            }
            for i in 0..50i64 {
                let mut r = Record::new();
                r.insert("id".into(), Value::Integer(i));
                r.insert("y".into(), Value::Float(i as f64 * 2.0));
                engine.insert("beta", &r).unwrap();
            }
            engine.cow_snapshot().unwrap();
        }

        // Order B: bundle beta first, then alpha
        {
            let mut engine = engine_no_autocompact(&dir_b);
            let s1 = BundleSchema::new("alpha")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::numeric("x").with_range(100.0));
            let s2 = BundleSchema::new("beta")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::numeric("y").with_range(100.0));
            engine.create_bundle(s1).unwrap();
            engine.create_bundle(s2).unwrap();

            for i in 0..50i64 {
                let mut r = Record::new();
                r.insert("id".into(), Value::Integer(i));
                r.insert("y".into(), Value::Float(i as f64 * 2.0));
                engine.insert("beta", &r).unwrap();
            }
            for i in 0..50i64 {
                let mut r = Record::new();
                r.insert("id".into(), Value::Integer(i));
                r.insert("x".into(), Value::Float(i as f64));
                engine.insert("alpha", &r).unwrap();
            }
            engine.cow_snapshot().unwrap();
        }

        // Both should produce same state after reopen
        let eng_a = Engine::open(&dir_a).unwrap();
        let eng_b = Engine::open(&dir_b).unwrap();
        assert_eq!(eng_a.total_records(), eng_b.total_records());
        assert_eq!(eng_a.total_records(), 100);

        // Spot check: same values
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(25));
        let a_alpha = eng_a.point_query("alpha", &key).unwrap().unwrap();
        let b_alpha = eng_b.point_query("alpha", &key).unwrap().unwrap();
        assert_eq!(a_alpha.get("x"), b_alpha.get("x"));

        let a_beta = eng_a.point_query("beta", &key).unwrap().unwrap();
        let b_beta = eng_b.point_query("beta", &key).unwrap().unwrap();
        assert_eq!(a_beta.get("y"), b_beta.get("y"));

        cleanup(&dir_a);
        cleanup(&dir_b);
    }

    // ── Integration tests for open_mmap ──────────────────────────────────

    /// open_mmap: snapshot → reopen in mmap mode → queries work.
    #[test]
    fn open_mmap_basic() {
        let dir = test_dir("mmap_basic");
        cleanup(&dir);

        // Phase A: populate via heap engine, snapshot to DHOOM
        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("sensors")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("name"))
                .fiber(FieldDef::numeric("temp").with_range(200.0));
            engine.create_bundle(schema).unwrap();

            for i in 0..50i64 {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("name".into(), Value::Text(format!("Sensor_{i}")));
                rec.insert("temp".into(), Value::Float(20.0 + i as f64 * 0.5));
                engine.insert("sensors", &rec).unwrap();
            }
            let snapped = engine.snapshot().unwrap();
            assert_eq!(snapped, 50);
        }

        // Phase B: reopen in mmap mode
        {
            let engine = Engine::open_mmap(&dir).unwrap();
            assert!(matches!(engine.storage_mode(), StorageMode::Mmap));
            assert_eq!(engine.total_records(), 50);
            assert!(engine.bundle_names().contains(&"sensors"));

            // Point query
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(25));
            let result = engine.point_query("sensors", &key).unwrap().unwrap();
            assert_eq!(result.get("name"), Some(&Value::Text("Sensor_25".into())));
        }

        cleanup(&dir);
    }

    /// open_mmap: insert/update/delete dispatch to overlay.
    #[test]
    fn open_mmap_overlay_ops() {
        let dir = test_dir("mmap_overlay");
        cleanup(&dir);

        // Populate + snapshot
        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("items")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("label"));
            engine.create_bundle(schema).unwrap();

            for i in 0..10i64 {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("label".into(), Value::Text(format!("Item_{i}")));
                engine.insert("items", &rec).unwrap();
            }
            engine.snapshot().unwrap();
        }

        // Reopen mmap and do overlay operations
        {
            let mut engine = Engine::open_mmap(&dir).unwrap();
            assert_eq!(engine.total_records(), 10);

            // Insert into overlay
            let mut rec = Record::new();
            rec.insert("id".into(), Value::Integer(100));
            rec.insert("label".into(), Value::Text("NewItem".into()));
            engine.insert("items", &rec).unwrap();

            // Query the overlay insert
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(100));
            let result = engine.point_query("items", &key).unwrap().unwrap();
            assert_eq!(result.get("label"), Some(&Value::Text("NewItem".into())));

            // Update via overlay
            let mut patches = Record::new();
            patches.insert("label".into(), Value::Text("Updated".into()));
            engine.update("items", &key, &patches).unwrap();

            let result = engine.point_query("items", &key).unwrap().unwrap();
            assert_eq!(result.get("label"), Some(&Value::Text("Updated".into())));

            // Delete via overlay
            engine.delete("items", &key).unwrap();
            let result = engine.point_query("items", &key).unwrap();
            assert!(result.is_none());
        }

        cleanup(&dir);
    }

    // ── Phase 3a TDD: mmap ship-blocker tests ───────────────────────────

    /// TDD Phase 3: point_query at 10K scale uses arithmetic key resolution.
    #[test]
    fn mmap_point_query_10k_arithmetic() {
        let dir = test_dir("mmap_pq_10k");
        cleanup(&dir);

        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("big")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("label"));
            engine.create_bundle(schema).unwrap();

            for i in 0..10_000i64 {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("label".into(), Value::Text(format!("row_{i}")));
                engine.insert("big", &rec).unwrap();
            }
            engine.snapshot().unwrap();
        }

        {
            let engine = Engine::open_mmap(&dir).unwrap();
            assert_eq!(engine.total_records(), 10_000);

            for target in [0i64, 1, 500, 5000, 9999] {
                let mut key = Record::new();
                key.insert("id".into(), Value::Integer(target));
                let result = engine.point_query("big", &key).unwrap()
                    .unwrap_or_else(|| panic!("missing record id={target}"));
                assert_eq!(
                    result.get("label"),
                    Some(&Value::Text(format!("row_{target}")))
                );
            }
        }

        cleanup(&dir);
    }

    /// TDD Phase 3: range_query on mmap bundle returns base data.
    #[test]
    fn mmap_range_query_base_data() {
        let dir = test_dir("mmap_range");
        cleanup(&dir);

        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("drugs")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("name"));
            engine.create_bundle(schema).unwrap();

            for i in 0..5i64 {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("name".into(), Value::Text(format!("Drug_{i}")));
                engine.insert("drugs", &rec).unwrap();
            }
            engine.snapshot().unwrap();
        }

        {
            let engine = Engine::open_mmap(&dir).unwrap();
            let results = engine.range_query(
                "drugs", "name",
                &[Value::Text("Drug_2".into())],
            ).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].get("name"), Some(&Value::Text("Drug_2".into())));
        }

        cleanup(&dir);
    }

    /// TDD Phase 3: filtered_query on mmap returns base + overlay data.
    #[test]
    fn mmap_filtered_query_base_plus_overlay() {
        use crate::bundle::QueryCondition;

        let dir = test_dir("mmap_filtered");
        cleanup(&dir);

        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("compounds")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("name"))
                .fiber(FieldDef::numeric("mw").with_range(1000.0));
            engine.create_bundle(schema).unwrap();

            for i in 0..100i64 {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("name".into(), Value::Text(format!("Cpd_{i}")));
                rec.insert("mw".into(), Value::Float(100.0 + i as f64));
                engine.insert("compounds", &rec).unwrap();
            }
            engine.snapshot().unwrap();
        }

        {
            let mut engine = Engine::open_mmap(&dir).unwrap();

            // Insert one more into overlay
            let mut rec = Record::new();
            rec.insert("id".into(), Value::Integer(200));
            rec.insert("name".into(), Value::Text("Overlay_Cpd".into()));
            rec.insert("mw".into(), Value::Float(190.0));
            engine.insert("compounds", &rec).unwrap();

            // Query: name = "Cpd_50" — should find base record
            let results = engine.filtered_query(
                "compounds",
                &[QueryCondition::Eq("name".into(), Value::Text("Cpd_50".into()))],
                None, None, false, None, None,
            ).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].get("id"), Some(&Value::Integer(50)));

            // Query: mw >= 190.0 — should find base records 90-99 + overlay record 200
            let results = engine.filtered_query(
                "compounds",
                &[QueryCondition::Gte("mw".into(), Value::Float(190.0))],
                None, None, false, None, None,
            ).unwrap();
            assert_eq!(results.len(), 11); // base: 90..99 (10) + overlay: 200 (1)
        }

        cleanup(&dir);
    }

    /// TDD Phase 3: batch_insert into mmap bundle.
    #[test]
    fn mmap_batch_insert() {
        let dir = test_dir("mmap_batch");
        cleanup(&dir);

        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("items")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("val"));
            engine.create_bundle(schema).unwrap();

            let mut rec = Record::new();
            rec.insert("id".into(), Value::Integer(1));
            rec.insert("val".into(), Value::Text("seed".into()));
            engine.insert("items", &rec).unwrap();
            engine.snapshot().unwrap();
        }

        {
            let mut engine = Engine::open_mmap(&dir).unwrap();

            let batch: Vec<Record> = (100..110i64).map(|i| {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("val".into(), Value::Text(format!("batch_{i}")));
                rec
            }).collect();
            let count = engine.batch_insert("items", &batch).unwrap();
            assert_eq!(count, 10);

            // Verify batch records are queryable
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(105));
            let result = engine.point_query("items", &key).unwrap().unwrap();
            assert_eq!(result.get("val"), Some(&Value::Text("batch_105".into())));
        }

        cleanup(&dir);
    }

    /// Edge case: arithmetic lookup for a tombstoned base record returns None.
    #[test]
    fn mmap_arithmetic_lookup_tombstoned() {
        let dir = test_dir("mmap_tomb_arith");
        cleanup(&dir);

        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("data")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("val"));
            engine.create_bundle(schema).unwrap();

            for i in 0..10i64 {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("val".into(), Value::Text(format!("v{i}")));
                engine.insert("data", &rec).unwrap();
            }
            engine.snapshot().unwrap();
        }

        {
            let mut engine = Engine::open_mmap(&dir).unwrap();
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(5));
            // Exists before delete
            assert!(engine.point_query("data", &key).unwrap().is_some());
            // Delete it
            engine.delete("data", &key).unwrap();
            // Should be None now (tombstoned)
            assert!(engine.point_query("data", &key).unwrap().is_none());
        }

        cleanup(&dir);
    }

    /// Edge case: filtered query deduplicates overlay-updated records over stale base.
    #[test]
    fn mmap_filtered_overlay_precedence() {
        use crate::bundle::QueryCondition;

        let dir = test_dir("mmap_dedup");
        cleanup(&dir);

        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("items")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("status"));
            engine.create_bundle(schema).unwrap();

            for i in 0..10i64 {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("status".into(), Value::Text("draft".into()));
                engine.insert("items", &rec).unwrap();
            }
            engine.snapshot().unwrap();
        }

        {
            let mut engine = Engine::open_mmap(&dir).unwrap();
            // Update base record id=1 in overlay: draft -> published
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(1));
            let mut patches = Record::new();
            patches.insert("status".into(), Value::Text("published".into()));
            let updated = engine.update("items", &key, &patches).unwrap();
            assert!(updated, "update should succeed for mmap base record id=1");

            // Query for status = "published" should find exactly 1 (overlay version)
            let results = engine.filtered_query(
                "items",
                &[QueryCondition::Eq("status".into(), Value::Text("published".into()))],
                None, None, false, None, None,
            ).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].get("id"), Some(&Value::Integer(1)));

            // Query for status = "draft" — should find 9 (base 0,2..9), not 10
            let results = engine.filtered_query(
                "items",
                &[QueryCondition::Eq("status".into(), Value::Text("draft".into()))],
                None, None, false, None, None,
            ).unwrap();
            assert_eq!(results.len(), 9);
        }

        cleanup(&dir);
    }

    /// Edge case: arithmetic lookup for out-of-bounds key returns None.
    #[test]
    fn mmap_arithmetic_out_of_bounds() {
        let dir = test_dir("mmap_oob");
        cleanup(&dir);

        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("small")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("x"));
            engine.create_bundle(schema).unwrap();

            for i in 0..5i64 {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("x".into(), Value::Text(format!("r{i}")));
                engine.insert("small", &rec).unwrap();
            }
            engine.snapshot().unwrap();
        }

        {
            let engine = Engine::open_mmap(&dir).unwrap();
            // Key beyond range
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(999));
            assert!(engine.point_query("small", &key).unwrap().is_none());

            // Negative key
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(-1));
            assert!(engine.point_query("small", &key).unwrap().is_none());
        }

        cleanup(&dir);
    }

    /// Edge case: mmap rebase merges overlay into fresh base snapshot.
    #[test]
    fn mmap_rebase_snapshot_roundtrip() {
        let dir = test_dir("mmap_rebase");
        cleanup(&dir);

        // Phase 1: create bundle, insert 5 records, snapshot
        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("data")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("val"));
            engine.create_bundle(schema).unwrap();

            for i in 0..5i64 {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("val".into(), Value::Text(format!("v{i}")));
                engine.insert("data", &rec).unwrap();
            }
            engine.snapshot().unwrap();
        }

        // Phase 2: open mmap, update id=2, delete id=4, insert id=10, then rebase
        {
            let mut engine = Engine::open_mmap(&dir).unwrap();
            assert_eq!(engine.mmap_bundles.get("data").unwrap().base().len(), 5);

            // Update
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(2));
            let mut patches = Record::new();
            patches.insert("val".into(), Value::Text("UPDATED".into()));
            assert!(engine.update("data", &key, &patches).unwrap());

            // Delete
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(4));
            engine.delete("data", &key).unwrap();

            // Insert new
            let mut rec = Record::new();
            rec.insert("id".into(), Value::Integer(10));
            rec.insert("val".into(), Value::Text("new".into()));
            engine.insert("data", &rec).unwrap();

            // Overlay should have 2 records (updated id=2 + new id=10), 1 tombstone
            assert_eq!(engine.mmap_bundles.get("data").unwrap().overlay_len(), 2);
            assert_eq!(engine.mmap_bundles.get("data").unwrap().tombstone_len(), 1);

            // Rebase
            engine.mmap_rebase_snapshot().unwrap();

            // After rebase: overlay empty, new base has 5 records (0,1,2_updated,3,10)
            let ob = engine.mmap_bundles.get("data").unwrap();
            assert_eq!(ob.overlay_len(), 0, "overlay should be empty after rebase");
            assert_eq!(ob.tombstone_len(), 0, "tombstones should be empty after rebase");
            assert_eq!(ob.base().len(), 5, "base should have 5 records after rebase");

            // Verify updated record
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(2));
            let rec = engine.point_query("data", &key).unwrap().expect("id=2 should exist");
            assert_eq!(rec.get("val"), Some(&Value::Text("UPDATED".into())));

            // Verify deleted record gone
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(4));
            assert!(engine.point_query("data", &key).unwrap().is_none());

            // Verify new record
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(10));
            let rec = engine.point_query("data", &key).unwrap().expect("id=10 should exist");
            assert_eq!(rec.get("val"), Some(&Value::Text("new".into())));
        }

        cleanup(&dir);
    }

    /// Verify arithmetic O(1) lookup works after rebase (post-rebase modifier preservation).
    #[test]
    fn mmap_rebase_preserves_arithmetic_lookup() {
        let dir = test_dir("mmap_rebase_arith");
        cleanup(&dir);

        // Phase 1: 10 sequential records, snapshot
        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("seq")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("val"));
            engine.create_bundle(schema).unwrap();
            for i in 0..10i64 {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("val".into(), Value::Text(format!("v{i}")));
                engine.insert("seq", &rec).unwrap();
            }
            engine.snapshot().unwrap();
        }

        // Phase 2: open mmap, update id=3, delete id=7, insert id=10, rebase
        {
            let mut engine = Engine::open_mmap(&dir).unwrap();

            // Update id=3
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(3));
            let mut patches = Record::new();
            patches.insert("val".into(), Value::Text("UPDATED3".into()));
            engine.update("seq", &key, &patches).unwrap();

            // Delete id=7
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(7));
            engine.delete("seq", &key).unwrap();

            // Insert id=10 (extends the sequence)
            let mut rec = Record::new();
            rec.insert("id".into(), Value::Integer(10));
            rec.insert("val".into(), Value::Text("v10".into()));
            engine.insert("seq", &rec).unwrap();

            engine.mmap_rebase_snapshot().unwrap();

            // After rebase: 10 records (0..10 minus 7 = [0,1,2,3,4,5,6,8,9,10])
            let ob = engine.mmap_bundles.get("seq").unwrap();
            assert_eq!(ob.base().len(), 10);
            assert_eq!(ob.overlay_len(), 0);
            assert_eq!(ob.tombstone_len(), 0);

            // Verify arithmetic O(1) lookup still works for multiple keys
            for id in [0i64, 1, 2, 4, 5, 6, 8, 9, 10] {
                let mut key = Record::new();
                key.insert("id".into(), Value::Integer(id));
                let rec = engine.point_query("seq", &key).unwrap();
                assert!(rec.is_some(), "id={id} should be found after rebase");
            }

            // id=3 should have updated value
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(3));
            let rec = engine.point_query("seq", &key).unwrap().unwrap();
            assert_eq!(rec.get("val"), Some(&Value::Text("UPDATED3".into())));

            // id=7 should be gone
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(7));
            assert!(engine.point_query("seq", &key).unwrap().is_none());
        }

        cleanup(&dir);
    }

    /// Verify range_query respects tombstones and overlay precedence in mmap mode.
    #[test]
    fn mmap_range_query_dedup() {
        let dir = test_dir("mmap_range_dedup");
        cleanup(&dir);

        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("items")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("cat"))
                .index("cat");
            engine.create_bundle(schema).unwrap();
            for i in 0..5i64 {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("cat".into(), Value::Text("A".into()));
                engine.insert("items", &rec).unwrap();
            }
            engine.snapshot().unwrap();
        }

        {
            let mut engine = Engine::open_mmap(&dir).unwrap();
            // Update id=1: cat A -> B
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(1));
            let mut patches = Record::new();
            patches.insert("cat".into(), Value::Text("B".into()));
            engine.update("items", &key, &patches).unwrap();

            // Delete id=3
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(3));
            engine.delete("items", &key).unwrap();

            // Range query for cat IN ["A"] should return 3 records (0,2,4) — not 5
            // id=1 was updated to "B" (excluded), id=3 was deleted (excluded)
            let results = engine.range_query("items", "cat", &[Value::Text("A".into())]).unwrap();
            assert_eq!(results.len(), 3, "range_query should exclude updated and tombstoned records");

            // Range query for cat IN ["B"] should return 1 record (id=1 overlay version)
            let results = engine.range_query("items", "cat", &[Value::Text("B".into())]).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].get("id"), Some(&Value::Integer(1)));
        }

        cleanup(&dir);
    }

    /// Prove that O(1) arithmetic lookup survives a rebase cycle end-to-end.
    ///
    /// The arithmetic modifier @start+step is the engine's core O(1) guarantee:
    /// given key k, resolve index n = (k − start) / step in constant time.
    /// This test verifies that after rebase (overlay merge → new DHOOM → re-mmap),
    /// the encoder re-detects the arithmetic progression and the modifier is
    /// physically present in the rebased fiber. No interior deletions — only
    /// updates — so the sequence stays contiguous and detect_arithmetic() succeeds.
    ///
    /// Uses 100 records (≥32) so BundleStore auto-detects flat geometry and
    /// switches to Sequential storage, ensuring records serialize in arithmetic
    /// order for DHOOM's detect_arithmetic().
    #[test]
    fn mmap_rebase_arithmetic_o1_proven() {
        use crate::dhoom::Modifier;

        let n = 100i64; // ≥32 triggers auto-detection → Sequential storage
        let dir = test_dir("mmap_rebase_o1");
        cleanup(&dir);

        // Phase 1: N records, ids 0..N-1 (arithmetic @0+1)
        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("arith")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("val"));
            engine.create_bundle(schema).unwrap();
            for i in 0..n {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("val".into(), Value::Text(format!("v{i}")));
                engine.insert("arith", &rec).unwrap();
            }
            engine.snapshot().unwrap();
        }

        // Phase 2: open mmap, confirm arithmetic modifier on base, update records, rebase
        {
            let mut engine = Engine::open_mmap(&dir).unwrap();

            // ─── Verify arithmetic modifier exists BEFORE rebase ───
            let ob = engine.mmap_bundles.get("arith").unwrap();
            let pre_fields = &ob.base().fiber().fields;
            let pre_arith = pre_fields.iter().find(|f| f.name == "id")
                .and_then(|f| f.modifier.as_ref());
            assert!(
                matches!(pre_arith, Some(Modifier::Arithmetic { .. })),
                "Pre-rebase: 'id' must have Arithmetic modifier, got {:?}", pre_arith
            );

            // Verify start=0, step=1 on pre-rebase fiber
            if let Some(Modifier::Arithmetic { ref start, ref step }) = pre_arith {
                assert_eq!(start.as_i64().unwrap(), 0, "pre-rebase arithmetic start must be 0");
                assert_eq!(step.unwrap_or(1), 1, "pre-rebase arithmetic step must be 1");
            }

            // ─── Update 3 records (no deletes — sequence stays contiguous) ───
            for id in [1i64, 50, 98] {
                let mut key = Record::new();
                key.insert("id".into(), Value::Integer(id));
                let mut patches = Record::new();
                patches.insert("val".into(), Value::Text(format!("UPDATED{id}")));
                assert!(engine.update("arith", &key, &patches).unwrap());
            }
            assert_eq!(engine.mmap_bundles.get("arith").unwrap().overlay_len(), 3);

            // ─── Rebase ───
            engine.mmap_rebase_snapshot().unwrap();

            // ─── Verify arithmetic modifier exists AFTER rebase ───
            let ob = engine.mmap_bundles.get("arith").unwrap();
            assert_eq!(ob.overlay_len(), 0, "overlay must be empty after rebase");
            assert_eq!(ob.tombstone_len(), 0, "tombstones must be empty after rebase");
            assert_eq!(ob.base().len(), n as usize, "all records survive (no deletes)");

            let post_fields = &ob.base().fiber().fields;
            let post_arith = post_fields.iter().find(|f| f.name == "id")
                .and_then(|f| f.modifier.as_ref());
            assert!(
                matches!(post_arith, Some(Modifier::Arithmetic { .. })),
                "Post-rebase: 'id' MUST retain Arithmetic modifier — O(1) lookup depends on it. Got {:?}",
                post_arith
            );

            // Verify start=0, step=1 on post-rebase fiber
            if let Some(Modifier::Arithmetic { ref start, ref step }) = post_arith {
                let start_i = start.as_i64().expect("start must be integer");
                let s = step.unwrap_or(1);
                assert_eq!(start_i, 0, "arithmetic start must be 0");
                assert_eq!(s, 1, "arithmetic step must be 1");
            }

            // ─── Verify O(1) path returns correct data for ALL keys ───
            for id in 0..n {
                let mut key = Record::new();
                key.insert("id".into(), Value::Integer(id));
                let rec = engine.point_query("arith", &key).unwrap()
                    .unwrap_or_else(|| panic!("id={id} must be found via O(1) arithmetic lookup"));
                let expected_val = if [1, 50, 98].contains(&id) {
                    format!("UPDATED{id}")
                } else {
                    format!("v{id}")
                };
                assert_eq!(
                    rec.get("val"),
                    Some(&Value::Text(expected_val.clone())),
                    "id={id}: expected val={expected_val}"
                );
            }
        }

        cleanup(&dir);
    }
}
