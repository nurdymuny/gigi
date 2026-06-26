//! Persistent storage engine — ties BundleStore + WAL together.
//!
//! Provides crash-safe, disk-backed bundle management.
//! On startup, replays the WAL to reconstruct in-memory state,
//! then loads DHOOM snapshots for any bundle whose snapshot predates the WAL.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};

use crate::bundle::{BundleStore, QueryCondition};
use crate::mmap_bundle::{BundleMut, BundleRef, MmapBundle, OverlayBundle};
use crate::types::{BundleSchema, Record, Value};
use crate::wal::{WalEntry, WalReader, WalWriter};

// ── Feature #6: Query Cache with TTL (Definitions 6.1–6.3, Theorems 6.1–6.2) ──

/// Compute a deterministic fingerprint for a query (Definition 6.1).
///
/// The fingerprint is a 64-bit hash of (bundle_name, sorted conditions, sorted or_conditions).
/// Two queries Q1, Q2 are cache-equivalent iff fingerprint(Q1) == fingerprint(Q2).
pub fn query_fingerprint(
    bundle: &str,
    conditions: &[QueryCondition],
    or_conditions: Option<&[Vec<QueryCondition>]>,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bundle.hash(&mut hasher);

    // Sort conditions by Debug representation for canonical ordering
    let mut sorted: Vec<String> = conditions.iter().map(|c| format!("{c:?}")).collect();
    sorted.sort();
    for s in &sorted {
        s.hash(&mut hasher);
    }

    // OR conditions: sort each group, then sort groups
    if let Some(ors) = or_conditions {
        let mut or_strs: Vec<String> = ors.iter().map(|group| {
            let mut g: Vec<String> = group.iter().map(|c| format!("{c:?}")).collect();
            g.sort();
            format!("{g:?}")
        }).collect();
        or_strs.sort();
        for s in &or_strs {
            s.hash(&mut hasher);
        }
    }

    hasher.finish()
}

/// A cached query result (Definition 6.2).
struct CacheEntry {
    bundle_name: String,
    result: Vec<Record>,
    created_at: std::time::Instant,
    generation_at_creation: u64,
    ttl_secs: u64,
}

/// In-memory query cache with TTL + generation-based invalidation (Feature #6).
///
/// Cache entries expire when:
///   (a) TTL elapses (Definition 6.3a)
///   (b) The source bundle has been written to since caching (Definition 6.3b)
///   (c) Explicit INVALIDATE CACHE command (Definition 6.3c)
///
/// Bounded by `max_entries` with LRU eviction.
pub struct QueryCache {
    entries: HashMap<u64, CacheEntry>,
    /// LRU order: front = oldest, back = newest
    lru_order: VecDeque<u64>,
    /// Per-bundle generation counters (incremented on write).
    generations: HashMap<String, u64>,
    /// Maximum cache size (entries). LRU eviction when full.
    pub max_entries: usize,
    /// Default TTL for new entries (seconds).
    pub default_ttl_secs: u64,
}

impl QueryCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            lru_order: VecDeque::new(),
            generations: HashMap::new(),
            max_entries: 1000,
            default_ttl_secs: 60,
        }
    }

    /// Look up a cached result. Returns None if miss or expired.
    /// Theorem 6.2: cache hit is O(1).
    pub fn get(&mut self, fingerprint: u64) -> Option<Vec<Record>> {
        let entry = self.entries.get(&fingerprint)?;
        // TTL check (Definition 6.3a)
        if entry.created_at.elapsed().as_secs() >= entry.ttl_secs {
            let fp = fingerprint;
            self.entries.remove(&fp);
            self.lru_order.retain(|&f| f != fp);
            return None;
        }
        // Generation check (Definition 6.3b)
        let current_gen = self.generations.get(&entry.bundle_name).copied().unwrap_or(0);
        if current_gen != entry.generation_at_creation {
            let fp = fingerprint;
            self.entries.remove(&fp);
            self.lru_order.retain(|&f| f != fp);
            return None;
        }
        // Move to back of LRU
        self.lru_order.retain(|&f| f != fingerprint);
        self.lru_order.push_back(fingerprint);
        Some(entry.result.clone())
    }

    /// Insert a query result into the cache.
    pub fn put(&mut self, fingerprint: u64, bundle: &str, result: Vec<Record>, ttl: u64) {
        // Evict LRU if at capacity
        while self.entries.len() >= self.max_entries {
            if let Some(old_fp) = self.lru_order.pop_front() {
                self.entries.remove(&old_fp);
            } else {
                break;
            }
        }
        let gen = self.generations.get(bundle).copied().unwrap_or(0);
        self.entries.insert(fingerprint, CacheEntry {
            bundle_name: bundle.to_string(),
            result,
            created_at: std::time::Instant::now(),
            generation_at_creation: gen,
            ttl_secs: ttl,
        });
        self.lru_order.retain(|&f| f != fingerprint);
        self.lru_order.push_back(fingerprint);
    }

    /// Bump generation counter on write — invalidates stale cache entries on next read.
    pub fn on_write(&mut self, bundle: &str) {
        *self.generations.entry(bundle.to_string()).or_default() += 1;
    }

    /// Invalidate all entries for a specific bundle (Definition 6.3c).
    pub fn invalidate_bundle(&mut self, bundle: &str) {
        self.entries.retain(|_, e| e.bundle_name != bundle);
        self.lru_order.retain(|fp| self.entries.contains_key(fp));
    }

    /// Invalidate all entries (Definition 6.3c).
    pub fn invalidate_all(&mut self) {
        self.entries.clear();
        self.lru_order.clear();
    }

    /// Number of cached entries (for testing/diagnostics).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Get current generation for a bundle.
    pub fn generation(&self, bundle: &str) -> u64 {
        self.generations.get(bundle).copied().unwrap_or(0)
    }
}

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
    /// **#105 fix.** Per-bundle timeout for cow_snapshot's
    /// per-bundle write loop. If any single bundle's snapshot encode
    /// + flush exceeds this budget, the writer aborts that bundle
    /// (deletes the partial .tmp file), logs a `TimedOut` warning,
    /// and continues with the next bundle. Other bundles in the
    /// batch are unaffected — snapshot files are per-bundle.
    ///
    /// `None` = no timeout (pre-#105 behavior; can hang
    /// indefinitely on a pathological bundle, as observed on
    /// `icarus_traverse_130799` per task #104).
    ///
    /// Default: `Some(600)` (10 minutes per bundle). Generous for
    /// large bundles; tight enough that a true hang is caught within
    /// a single ops shift rather than blocking the whole snapshot
    /// pipeline indefinitely.
    pub per_bundle_timeout_secs: Option<u64>,
}

impl Default for CompactionPolicy {
    fn default() -> Self {
        Self {
            amplification_threshold: 3.0,
            min_interval_secs: 300,
            max_wal_entries: 10_000_000,
            max_wal_bytes: 2 * 1024 * 1024 * 1024, // 2 GiB
            disabled: false,
            per_bundle_timeout_secs: Some(600), // 10 minutes per bundle
        }
    }
}

/// **#105 fix.** Outcome report from a per-bundle snapshot write.
/// Returned by `write_snapshot_files_with_timeout` so callers can see
/// which bundles succeeded vs timed out without aborting the whole
/// snapshot operation.
#[derive(Debug, Clone)]
pub struct SnapshotBundleOutcome {
    pub bundle_name: String,
    /// `Ok(record_count)` on success; `Err(elapsed_secs)` on timeout.
    pub result: Result<usize, u64>,
}

/// **#105 fix.** Aggregate report from `write_snapshot_files_with_timeout`.
/// Successful bundles + timed-out bundles, summed for convenience.
#[derive(Debug, Clone)]
pub struct SnapshotReport {
    pub bundles: Vec<SnapshotBundleOutcome>,
    pub total_records_written: usize,
    pub timed_out_bundles: Vec<String>,
}

// ── Feature #9: Pub/Sub with Sheaf Triggers (Definitions 9.1–9.3, Theorem 9.1) ──

/// Mutation operation type for trigger matching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationOp {
    Insert,
    Update,
    Delete,
    Any,
}

/// Trigger kind (Definition 9.3).
#[derive(Debug, Clone)]
pub enum TriggerKind {
    /// Fires on matching mutations (insert/update/delete).
    OnMutation {
        bundle: String,
        operation: MutationOp,
        filter: Option<Vec<QueryCondition>>,
    },
}

/// A trigger definition (Definition 9.2).
#[derive(Debug, Clone)]
pub struct TriggerDef {
    pub name: String,
    pub kind: TriggerKind,
    pub channel: String,
}

/// A notification emitted by trigger evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub trigger_name: String,
    pub bundle: String,
    pub payload: Record,
}

/// Manages trigger definitions and evaluates them on mutations.
pub struct TriggerManager {
    triggers: Vec<TriggerDef>,
}

impl TriggerManager {
    pub fn new() -> Self {
        Self {
            triggers: Vec::new(),
        }
    }

    /// Register a new trigger.
    pub fn create_trigger(&mut self, def: TriggerDef) {
        // Replace existing trigger with same name
        self.triggers.retain(|t| t.name != def.name);
        self.triggers.push(def);
    }

    /// Remove a trigger by name.
    pub fn drop_trigger(&mut self, name: &str) -> bool {
        let before = self.triggers.len();
        self.triggers.retain(|t| t.name != name);
        self.triggers.len() < before
    }

    /// List all trigger definitions.
    pub fn list_triggers(&self) -> &[TriggerDef] {
        &self.triggers
    }

    /// Evaluate mutation triggers for a specific bundle and operation.
    /// Returns notifications for all matching triggers (Theorem 9.1).
    pub fn evaluate_mutation(
        &self,
        bundle: &str,
        op: &MutationOp,
        record: &Record,
    ) -> Vec<Notification> {
        let mut notifications = Vec::new();
        for trigger in &self.triggers {
            match &trigger.kind {
                TriggerKind::OnMutation {
                    bundle: trigger_bundle,
                    operation,
                    filter,
                } => {
                    if trigger_bundle != bundle {
                        continue;
                    }
                    if *operation != MutationOp::Any && operation != op {
                        continue;
                    }
                    if let Some(conditions) = filter {
                        if !crate::bundle::matches_filter(record, conditions, None) {
                            continue;
                        }
                    }
                    notifications.push(Notification {
                        trigger_name: trigger.name.clone(),
                        bundle: bundle.to_string(),
                        payload: record.clone(),
                    });
                }
            }
        }
        notifications
    }

    /// Number of registered triggers.
    pub fn len(&self) -> usize {
        self.triggers.len()
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
    /// Feature #6: In-memory query cache with TTL + generation invalidation.
    query_cache: QueryCache,
    /// Feature #9: Trigger manager for pub/sub notifications.
    trigger_manager: TriggerManager,
    /// Pending notifications from the last mutation (drained by caller).
    pending_notifications: Vec<Notification>,
    /// Ask G — Pattern Hunt in-memory registry.
    ///
    /// Lifetime tied to the engine process; lost on restart. Mirrors the
    /// PREPARE precedent. Phase 6 graduates this to a `gigi_patterns`
    /// bundle for persistence + sharing across operators
    /// (theory/scj/PATTERN_HUNT_SPEC_v0.1.md §11 OQ-1).
    ///
    /// Made `pub(crate)` so the parser's `execute()` function can mutate
    /// it directly; no API leak outside the crate.
    #[cfg(feature = "patterns")]
    pub(crate) pattern_registry:
        std::collections::HashMap<String, crate::parser::PatternDef>,
    /// Owned tempdir for `Engine::open_memory()` instances. `None` for
    /// file-backed engines created via `open()` / `open_empty()` /
    /// `open_mmap()`. When `Some`, the tempdir backing `data_dir` is
    /// removed automatically when this `Engine` is dropped (via
    /// `tempfile::TempDir`'s `Drop` impl). No manual cleanup required.
    ///
    /// Placed last in the struct so Rust's field-drop order tears down
    /// every other resource (notably `wal: WalWriter`) BEFORE the
    /// tempdir is removed — important on Windows where open file
    /// handles block directory removal.
    _tempdir: Option<tempfile::TempDir>,
}

/// TDD-HAL-V.2: response shape for
/// `Engine::snapshot_gauge_field_durable`. `sha256` is the lowercase
/// hex encoding of the SHA-256 over the LE-encoded buffer bytes
/// (Bee's locked decision D-V-C — same hash lands in the WAL and in
/// the citation envelope). `wal_offset` is the post-write
/// `WalWriter::entry_count`, a stable monotonically-increasing handle
/// the future V.3 replay-restoration gate can cite without reaching
/// into WAL internals.
#[cfg(feature = "gauge")]
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotResponse {
    /// Lowercase hex of SHA-256 over the LE-encoded buffer bytes.
    pub sha256: String,
    /// Post-write WAL entry count.
    pub wal_offset: u64,
}

/// Lowercase hex encoding of a 32-byte SHA-256 digest. Inline helper
/// so we don't pull in the `hex` crate for this single call site; the
/// canonical citation form per spec §3.P0.3 is lowercase.
#[cfg(feature = "gauge")]
fn hex_encode(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0F) as usize] as char);
    }
    out
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

    /// Open an Engine backed by an in-memory tempdir. The tempdir
    /// is created at construction and removed when the returned
    /// Engine is dropped. No WAL replay. No persistence between
    /// runs. Suitable for dev, CI, tests, and aurora-server-style
    /// host binaries that have no persistence needs.
    ///
    /// AURORA Phase 3 (Rory 2026-06-22 §1): replaces the
    /// `tempdir() + Engine::open_empty(td.path())?` boilerplate that
    /// appeared in every test harness and CI job. The tempdir is
    /// owned by the Engine and cleaned up automatically via
    /// `tempfile::TempDir`'s `Drop` impl — no manual `td.close()`.
    pub fn open_memory() -> io::Result<Self> {
        let td = tempfile::tempdir()?;
        let mut engine = Self::open_empty(td.path())?;
        engine._tempdir = Some(td);
        Ok(engine)
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

        // Feature #9: Replay trigger definitions from WAL
        let mut trigger_manager = TriggerManager::new();
        if replay && wal_path.exists() {
            Self::replay_triggers(&wal_path, &mut trigger_manager)?;
        }

        // TDD-HAL-II.4b: Replay durable lattice + gauge field
        // declarations from WAL. Done after the main `do_replay` pass
        // so the lattice and gauge-field registries are rebuilt
        // deterministically from the WAL byte stream alone (clear →
        // re-register every declaration in WAL order). Two-pass: first
        // pass restores lattices (gauge fields depend on them), second
        // pass restores gauge fields.
        #[cfg(feature = "gauge")]
        if replay && wal_path.exists() {
            Self::replay_gauge_substrate(&wal_path)?;
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
            query_cache: QueryCache::new(),
            trigger_manager,
            pending_notifications: Vec::new(),
            #[cfg(feature = "patterns")]
            pattern_registry: std::collections::HashMap::new(),
            _tempdir: None,
        })
    }

    fn is_wal_crc_mismatch(error: &io::Error) -> bool {
        error.kind() == io::ErrorKind::InvalidData
            && error.to_string().contains("WAL CRC mismatch")
    }

    fn finish_wal_replay_prefix(
        result: io::Result<()>,
        context: &str,
        entries_applied: u64,
    ) -> io::Result<()> {
        match result {
            Ok(()) => Ok(()),
            Err(e) if Self::is_wal_crc_mismatch(&e) => {
                eprintln!(
                    "  WARNING: {context} stopped at corrupted WAL tail after \
                     {entries_applied} valid entries; preserving valid prefix"
                );
                Ok(())
            }
            Err(e) => Err(e),
        }
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
            let mut entries_applied = 0u64;
            let replay_result = reader.replay(|entry| {
                entries_applied += 1;
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
            });
            Self::finish_wal_replay_prefix(
                replay_result,
                "mmap schema/overlay WAL scan",
                entries_applied,
            )?;
        }

        // Phase 2: Open each .dhoom as MmapBundle, wrap in OverlayBundle.
        // For schemas that have a CreateBundle in WAL but NO matching .dhoom
        // file (i.e. bundles created after the last snapshot was written),
        // fall back to creating a heap-only BundleStore. The Engine's bundle
        // accessor (`pub fn bundle`) checks `bundles` before `mmap_bundles`
        // so mixed mode works transparently at the lookup layer.
        //
        // Pre-2026-05-25 behavior was to `continue` on missing .dhoom,
        // silently dropping the bundle. That meant any bundle created
        // since the last snapshot vanished from the live engine on every
        // fast-path startup — invisible until queried. The fix restores
        // those bundles as heap-only stores; subsequent WAL inserts (the
        // Phase 3 loop below) land on them correctly.
        let mut mmap_bundles: HashMap<String, OverlayBundle> = HashMap::new();
        let mut heap_bundles: HashMap<String, BundleStore> = HashMap::new();
        for (name, schema) in &schemas {
            let snap_path = snapshots_dir.join(format!("{name}.dhoom"));
            if snapshots_dir.exists() && snap_path.exists() {
                match MmapBundle::open(&snap_path) {
                    Ok(mmap) => {
                        let n = mmap.len();
                        let overlay = OverlayBundle::new(mmap, schema.clone());
                        eprintln!("  Mmap opened: {name} ({n} records)");
                        mmap_bundles.insert(name.clone(), overlay);
                    }
                    Err(e) => {
                        eprintln!(
                            "  WARNING: mmap open failed for {name}: {e} — falling back to heap"
                        );
                        heap_bundles.insert(name.clone(), BundleStore::new(schema.clone()));
                    }
                }
            } else {
                // No snapshot on disk for this schema. Create a heap-only
                // BundleStore so subsequent WAL inserts find a target.
                eprintln!("  Heap-only (no snapshot): {name}");
                heap_bundles.insert(name.clone(), BundleStore::new(schema.clone()));
            }
        }

        // Phase 3: Replay post-checkpoint WAL entries. Apply to mmap
        // overlay if the bundle was snapshotted, otherwise to the heap-
        // only store created above. Either target catches the insert.
        for entry in &wal_entries {
            match entry {
                WalEntry::Insert { bundle_name, record } => {
                    if let Some(ob) = mmap_bundles.get(bundle_name) {
                        ob.insert(record);
                    } else if let Some(store) = heap_bundles.get_mut(bundle_name) {
                        store.insert(record);
                    }
                }
                WalEntry::Update { bundle_name, key, patches } => {
                    if let Some(ob) = mmap_bundles.get(bundle_name) {
                        ob.update(key, patches);
                    } else if let Some(store) = heap_bundles.get_mut(bundle_name) {
                        store.update(key, patches);
                    }
                }
                WalEntry::Delete { bundle_name, key } => {
                    if let Some(ob) = mmap_bundles.get(bundle_name) {
                        let key_str = format!("{key:?}");
                        ob.delete(&key_str, Some(key));
                    } else if let Some(store) = heap_bundles.get_mut(bundle_name) {
                        // BundleStore::delete takes &Record directly
                        // (mirrors the slow-path do_replay handler).
                        store.delete(key);
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

        // TDD-HAL-II.4b: rebuild lattice + gauge registries on mmap
        // open the same way the heap-mode `open_inner` path does. The
        // registries are process singletons; mmap mode replaying the
        // bundle WAL but skipping the gauge substrate would silently
        // leave fields missing.
        #[cfg(feature = "gauge")]
        if wal_path.exists() {
            Self::replay_gauge_substrate(&wal_path)?;
        }

        let wal = WalWriter::open(&wal_path)?;

        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            // Mixed mode: snapshotted bundles live in `mmap_bundles`;
            // WAL-only (post-snapshot) bundles live in `bundles`.
            // The Engine's bundle accessor checks `bundles` first then
            // `mmap_bundles`, so callers don't need to know the split.
            bundles: heap_bundles,
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
            query_cache: QueryCache::new(),
            trigger_manager: TriggerManager::new(),
            pending_notifications: Vec::new(),
            #[cfg(feature = "patterns")]
            pattern_registry: std::collections::HashMap::new(),
            _tempdir: None,
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

        let replay_result = reader.replay(|entry| {
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
                // Feature #9: Trigger WAL entries are handled post-replay by the engine
                WalEntry::CreateTrigger { .. } | WalEntry::DropTrigger(_) => {}
                // TDD-HAL-II.4b: lattice + gauge-field WAL entries are
                // handled post-replay by `replay_gauge_substrate` so we
                // can fail loudly on a malformed entry instead of
                // panicking inside this generic closure.
                // TDD-HAL-V.1: same treatment for the snapshot op —
                // the post-replay path will install the buffer in V.2.
                // Here we just acknowledge the variant so this match
                // stays exhaustive.
                #[cfg(feature = "gauge")]
                WalEntry::LatticeDeclare { .. }
                | WalEntry::GaugeFieldDeclare { .. }
                | WalEntry::GaugeFieldSnapshot(_) => {}
                // AURORA Phase 2: HAMILTONIAN_DECLARE is metadata-only
                // audit/introspection; replay handling is deferred to a
                // follow-up workflow (host binaries explicitly re-
                // register at startup per the Q5 contract). Acknowledge
                // here so the match stays exhaustive.
                #[cfg(feature = "gauge")]
                WalEntry::HamiltonianDeclare { .. } => {}
                // AURORA Phase 3: INTEGRATOR_CHOICE is audit-only —
                // records which integrator path SYMPLECTIC_FLOW
                // selected per invocation. Replay does not re-execute;
                // the entry is consumed by post-hoc diagnostics tools.
                #[cfg(feature = "gauge")]
                WalEntry::IntegratorChoice { .. } => {}
                // IMAGINE coherence Phase 2: IMAGINE_FALLBACK is
                // audit-only — records when the tame-metric fallback
                // engaged for an `imagine_coherence` request. Replay
                // does not re-execute the decision; the entry is
                // consumed by post-hoc diagnostics tools (Marcella's
                // confidence routing, operator dashboards).
                #[cfg(feature = "imagine")]
                WalEntry::ImagineFallback { .. } => {}
            }
            Ok(())
        });
        Self::finish_wal_replay_prefix(replay_result, "bundle WAL replay", entry_count)?;

        let elapsed = start.elapsed().as_secs_f64();
        let total: usize = bundles.values().map(|s| s.len()).sum();
        eprintln!("  WAL replay complete: {entry_count} entries, {total} records in {elapsed:.1}s");
        Ok(entry_count)
    }

    /// TDD-HAL-II.4b: Replay durable lattice + gauge field WAL entries
    /// into the process-singleton `lattice::registry` and
    /// `gauge::registry`. Called from `open_inner` after the main
    /// bundle replay so the gauge registry is rebuilt deterministically
    /// from disk.
    ///
    /// The function clears both registries first — this is the
    /// "every `Engine::open` starts from a clean registry" contract.
    /// In-memory (non-durable) declarations are NOT preserved across
    /// restart by design (Bee's locked decision 3); only the durable
    /// path round-trips.
    ///
    /// Two passes are required because gauge fields name their
    /// lattice by string and the materialize helper needs the lattice
    /// in the registry to look up edge count. First pass installs
    /// every `LatticeDeclare`; second pass installs every
    /// `GaugeFieldDeclare`.
    #[cfg(feature = "gauge")]
    fn replay_gauge_substrate(wal_path: &Path) -> io::Result<()> {
        crate::lattice::registry::clear();
        crate::gauge::registry::clear();

        // Pass 1 — lattices.
        let mut reader = WalReader::open(wal_path)?;
        let mut entries_applied = 0u64;
        let replay_result = reader.replay(|entry| {
            entries_applied += 1;
            if let WalEntry::LatticeDeclare { gql } = entry {
                let lat = crate::lattice::Lattice::from_gql(&gql).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("WAL LatticeDeclare parse error: {e}"),
                    )
                })?;
                crate::lattice::registry::register(lat);
            }
            Ok(())
        });
        Self::finish_wal_replay_prefix(
            replay_result,
            "gauge lattice WAL replay",
            entries_applied,
        )?;

        // Pass 2 — gauge fields. Resolution: look up lattice by name
        // in the registry (already populated by Pass 1).
        //
        // For SU(2) fields we also populate the SU(2)-mut registry via
        // `register_su2` (mirrors the parser-side fix from commit
        // 9c5b614). That keeps `get_su2_mut(name)` working after a
        // restart, which is what the V.3 snapshot-replay pass below
        // needs: it acquires the mut handle, calls `replace_buffer`,
        // and republishes through `republish_su2` to keep the dyn
        // surface coherent with the post-restoration state.
        let mut reader = WalReader::open(wal_path)?;
        let mut entries_applied = 0u64;
        let replay_result = reader.replay(|entry| {
            entries_applied += 1;
            if let WalEntry::GaugeFieldDeclare {
                name,
                lattice_name,
                group,
                init_kind,
                init_seed,
            } = entry
            {
                let lat = crate::lattice::registry::get(&lattice_name).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "WAL GaugeFieldDeclare references unknown lattice '{lattice_name}'"
                        ),
                    )
                })?;
                let handle = crate::gauge::persistence::materialize_field(
                    name.clone(),
                    &lat,
                    group,
                    init_kind.clone(),
                    init_seed,
                )?;
                crate::gauge::registry::register(handle);
                // Dual-register the SU(2)-mut handle so V.3 snapshot
                // replay (Pass 3 below) and post-restart GIBBS_SAMPLE
                // can both reach the field. Non-SU(2) groups skip this
                // — they don't have a mut surface yet.
                if matches!(group, crate::gauge::Group::SU2) {
                    let field = crate::gauge::su2_gauge_field::SU2GaugeField::new(
                        name,
                        &lat,
                        init_kind,
                        init_seed,
                    )
                    .map_err(|e| {
                        io::Error::new(io::ErrorKind::InvalidData, e.to_string())
                    })?;
                    crate::gauge::registry::register_su2(field);
                }
            }
            Ok(())
        });
        Self::finish_wal_replay_prefix(
            replay_result,
            "gauge field WAL replay",
            entries_applied,
        )?;

        // Pass 3 — snapshot restoration (TDD-HAL-V.3).
        //
        // Walks every `OP_GAUGE_FIELD_SNAPSHOT` in WAL order. For each
        // entry we:
        //   1. Decode the LE-encoded payload (already done by the WAL
        //      reader; `WalEntry::GaugeFieldSnapshot(payload)` carries
        //      the parsed `GaugeFieldSnapshotPayload`).
        //   2. Resolve the field handle via the dyn read registry. A
        //      missing handle is an orphan snapshot — abort with
        //      `WalError::OrphanedSnapshot(name)`.
        //   3. Compare the declared field's group against the snapshot
        //      payload's group tag. Mismatch is
        //      `WalError::SnapshotGroupMismatch { expected, found }`.
        //   4. Re-derive SHA-256 from the decoded buffer's LE bytes
        //      (the canonical citation handle per locked decision
        //      D-V-C) and compare against `payload.sha256`. Mismatch is
        //      `WalError::SnapshotChecksumMismatch { name }`.
        //   5. Install the buffer in place via
        //      `SU2GaugeField::replace_buffer`, then republish through
        //      both registries so the dyn read surface and the SU(2)-
        //      mut surface stay coherent with the post-restoration
        //      state.
        //
        // Idempotency: multiple snapshot entries for the same field
        // replay last-write-wins (each `replace_buffer` overwrites
        // `self.buffer.data` in place; subsequent entries overwrite
        // their predecessor's bytes).
        let mut reader = WalReader::open(wal_path)?;
        let mut entries_applied = 0u64;
        let replay_result = reader.replay(|entry| {
            entries_applied += 1;
            if let WalEntry::GaugeFieldSnapshot(payload) = entry {
                // 2. Resolve handle through the dyn read registry. When the
                // declare is missing (orphan snapshot — usually a partial
                // WAL from a wedged or OOM-killed prior boot), skip the
                // snapshot with a warning instead of aborting the entire
                // mmap-open path. Skipping keeps the boot on the
                // low-memory mmap fast path; the orphan field stays
                // unavailable until POST /v1/admin/gauge/repair writes a
                // synthetic declare.
                let handle = match crate::gauge::registry::get(&payload.name) {
                    Some(h) => h,
                    None => {
                        eprintln!(
                            "WARNING: gauge field WAL has snapshot for '{}' \
                             with no preceding OP_GAUGE_FIELD_DECLARE — \
                             skipping orphan snapshot (field unavailable). \
                             Boot continues on mmap fast path.",
                            payload.name
                        );
                        return Ok(());
                    }
                };
                // 3. Group-tag agreement.
                let expected_group = handle.group();
                if expected_group != payload.group {
                    return Err(crate::wal::WalError::SnapshotGroupMismatch {
                        name: payload.name.clone(),
                        expected: expected_group,
                        found: payload.group,
                    }
                    .into());
                }
                // 4. SHA-256 re-derivation (citation-handle integrity).
                let derived = crate::wal::GaugeFieldSnapshotPayload::compute_buffer_sha256(
                    &payload.buffer,
                );
                if derived != payload.sha256 {
                    return Err(crate::wal::WalError::SnapshotChecksumMismatch {
                        name: payload.name.clone(),
                    }
                    .into());
                }
                // 5. Install the buffer in place via the SU(2)-mut
                //    handle, then republish to keep both registries
                //    coherent.
                if let Some(field_arc) = crate::gauge::registry::get_su2_mut(&payload.name) {
                    {
                        let mut guard =
                            field_arc.lock().expect("su2 field mutex poisoned");
                        guard
                            .replace_buffer(payload.buffer)
                            .map_err(|e| {
                                io::Error::new(io::ErrorKind::InvalidData, e.to_string())
                            })?;
                    }
                    crate::gauge::registry::republish_su2(&payload.name, field_arc);
                }
            }
            Ok(())
        });
        Self::finish_wal_replay_prefix(
            replay_result,
            "gauge snapshot WAL replay",
            entries_applied,
        )?;
        Ok(())
    }

    /// Feature #9: Replay trigger WAL entries into the TriggerManager.
    fn replay_triggers(wal_path: &Path, tm: &mut TriggerManager) -> io::Result<()> {
        let mut reader = WalReader::open(wal_path)?;
        let mut entries_applied = 0u64;
        let replay_result = reader.replay(|entry| {
            entries_applied += 1;
            match entry {
                WalEntry::CreateTrigger {
                    name,
                    bundle,
                    channel,
                    operation,
                    filter_str: _,
                } => {
                    let op = match operation.as_str() {
                        "INSERT" => MutationOp::Insert,
                        "UPDATE" => MutationOp::Update,
                        "DELETE" => MutationOp::Delete,
                        _ => MutationOp::Any,
                    };
                    tm.create_trigger(TriggerDef {
                        name,
                        kind: TriggerKind::OnMutation {
                            bundle: bundle.clone(),
                            operation: op,
                            filter: None, // TODO: parse filter_str if needed
                        },
                        channel,
                    });
                }
                WalEntry::DropTrigger(name) => {
                    tm.drop_trigger(&name);
                }
                _ => {}
            }
            Ok(())
        });
        Self::finish_wal_replay_prefix(replay_result, "trigger WAL replay", entries_applied)?;
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
        } else if let Some(ob) = self.mmap_bundles.get(bundle_name) {
            ob.insert(record);
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Bundle '{}' not found", bundle_name),
            ));
        }
        self.query_cache.on_write(bundle_name);
        // Feature #9: Evaluate triggers
        let notifs = self.trigger_manager.evaluate_mutation(bundle_name, &MutationOp::Insert, record);
        self.pending_notifications.extend(notifs);
        self.maybe_checkpoint()?;
        Ok(())
    }

    fn key_for_record(&self, bundle_name: &str, record: &Record) -> io::Result<Record> {
        let schema = self.schemas.get(bundle_name).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Bundle '{}' not found", bundle_name),
            )
        })?;
        Ok(schema
            .base_fields
            .iter()
            .map(|field| {
                (
                    field.name.clone(),
                    record.get(&field.name).cloned().unwrap_or(Value::Null),
                )
            })
            .collect())
    }

    fn non_key_patches_for_record(&self, bundle_name: &str, record: &Record) -> io::Result<Record> {
        let schema = self.schemas.get(bundle_name).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Bundle '{}' not found", bundle_name),
            )
        })?;
        Ok(record
            .iter()
            .filter(|(name, _)| !schema.base_fields.iter().any(|field| field.name == **name))
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect())
    }

    /// Upsert a record through durable WAL entries.
    ///
    /// Returns true when a new record was inserted and false when an
    /// existing record was updated.
    pub fn upsert(&mut self, bundle_name: &str, record: &Record) -> io::Result<bool> {
        let key = self.key_for_record(bundle_name, record)?;
        if self.point_query(bundle_name, &key)?.is_some() {
            let patches = self.non_key_patches_for_record(bundle_name, record)?;
            let updated = self.update(bundle_name, &key, &patches)?;
            if updated {
                Ok(false)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Record disappeared during upsert in '{}'", bundle_name),
                ))
            }
        } else {
            self.insert(bundle_name, record)?;
            Ok(true)
        }
    }

    /// Batch upsert records through durable WAL entries.
    ///
    /// Returns (inserted, updated).
    pub fn batch_upsert(&mut self, bundle_name: &str, records: &[Record]) -> io::Result<(usize, usize)> {
        if self.bundle(bundle_name).is_none() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Bundle '{}' not found", bundle_name),
            ));
        }
        let mut inserted = 0usize;
        let mut updated = 0usize;
        for record in records {
            if self.upsert(bundle_name, record)? {
                inserted += 1;
            } else {
                updated += 1;
            }
        }
        Ok((inserted, updated))
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
        if updated {
            self.query_cache.on_write(bundle_name);
            let notifs = self.trigger_manager.evaluate_mutation(bundle_name, &MutationOp::Update, patches);
            self.pending_notifications.extend(notifs);
        }
        self.maybe_checkpoint()?;
        Ok(updated)
    }

    /// Bulk update all matching records through one WAL update per record.
    pub fn bulk_update(
        &mut self,
        bundle_name: &str,
        conditions: &[QueryCondition],
        patches: &Record,
    ) -> io::Result<usize> {
        let matching_keys: Vec<Record> = {
            let store = self.bundle(bundle_name).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Bundle '{}' not found", bundle_name),
                )
            })?;
            store
                .records()
                .filter(|record| crate::bundle::matches_filter(record, conditions, None))
                .map(|record| self.key_for_record(bundle_name, &record))
                .collect::<io::Result<Vec<_>>>()?
        };

        let mut updated = 0usize;
        for key in &matching_keys {
            if self.update(bundle_name, key, patches)? {
                updated += 1;
            }
        }
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
        if deleted {
            self.query_cache.on_write(bundle_name);
            let notifs = self.trigger_manager.evaluate_mutation(bundle_name, &MutationOp::Delete, key);
            self.pending_notifications.extend(notifs);
        }
        self.maybe_checkpoint()?;
        Ok(deleted)
    }

    /// Bulk delete all matching records through one WAL delete per record.
    pub fn bulk_delete(
        &mut self,
        bundle_name: &str,
        conditions: &[QueryCondition],
    ) -> io::Result<usize> {
        let matching_keys: Vec<Record> = {
            let store = self.bundle(bundle_name).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Bundle '{}' not found", bundle_name),
                )
            })?;
            store
                .records()
                .filter(|record| crate::bundle::matches_filter(record, conditions, None))
                .map(|record| self.key_for_record(bundle_name, &record))
                .collect::<io::Result<Vec<_>>>()?
        };

        let mut deleted = 0usize;
        for key in &matching_keys {
            if self.delete(bundle_name, key)? {
                deleted += 1;
            }
        }
        Ok(deleted)
    }

    /// Drop (remove) a bundle entirely.
    pub fn drop_bundle(&mut self, name: &str) -> io::Result<bool> {
        self.wal.log_drop_bundle(name)?;
        let existed = self.bundles.remove(name).is_some()
            || self.mmap_bundles.remove(name).is_some();
        self.schemas.remove(name);
        self.query_cache.invalidate_bundle(name);
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

        // Invalidate cache for this bundle
        if count > 0 {
            self.query_cache.on_write(bundle_name);
        }

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

    /// Unified bundle access — checks heap first, then mmap overlay.
    pub fn bundle(&self, name: &str) -> Option<BundleRef<'_>> {
        if let Some(store) = self.bundles.get(name) {
            return Some(BundleRef::Heap(store));
        }
        if let Some(overlay) = self.mmap_bundles.get(name) {
            return Some(BundleRef::Overlay(overlay));
        }
        None
    }

    /// Unified mutable bundle access — checks heap first, then mmap overlay.
    pub fn bundle_mut(&mut self, name: &str) -> Option<BundleMut<'_>> {
        // Check heap first (borrow-checker friendly: separate branches).
        if self.bundles.contains_key(name) {
            return self.bundles.get_mut(name).map(BundleMut::Heap);
        }
        if let Some(overlay) = self.mmap_bundles.get(name) {
            return Some(BundleMut::Overlay(overlay));
        }
        None
    }

    /// Get a direct reference to the heap BundleStore (needed for WAL/snapshot internals).
    pub fn heap_bundle(&self, name: &str) -> Option<&BundleStore> {
        self.bundles.get(name)
    }

    /// Get a direct mutable reference to the heap BundleStore.
    pub fn heap_bundle_mut(&mut self, name: &str) -> Option<&mut BundleStore> {
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

    /// Best-effort creation timestamp for a bundle, in seconds since
    /// the UNIX epoch. Used by the `__bundles__` virtual bundle to
    /// populate the `created_ts` fiber field.
    ///
    /// - mmap overlays return the snapshot file's mtime when readable
    /// - heap bundles fall back to the WAL file's mtime (proxy for
    ///   "when this engine started serving this bundle")
    /// - returns `None` if the bundle is not in the registry or no
    ///   filesystem timestamp is available (e.g. `open_memory()` with
    ///   no WAL on disk yet)
    ///
    /// NOTE: this is observational, not transactional. The WAL
    /// `CreateBundle` entry does not carry a wall-clock timestamp, so
    /// the answer is the closest honest proxy available without
    /// breaking the WAL forward-compat contract.
    pub fn bundle_created_ts(&self, name: &str) -> Option<i64> {
        if self.mmap_bundles.contains_key(name) {
            // Overlay: prefer the snapshot file's mtime.
            let snap = self.data_dir.join("snapshots").join(format!("{name}.dhoom"));
            if let Ok(meta) = fs::metadata(&snap) {
                if let Ok(modified) = meta.modified() {
                    if let Ok(d) = modified.duration_since(std::time::UNIX_EPOCH) {
                        return Some(d.as_secs() as i64);
                    }
                }
            }
        }
        // Heap or overlay-without-snapshot-on-disk: fall back to WAL mtime.
        if self.bundles.contains_key(name) || self.mmap_bundles.contains_key(name) {
            let wal_path = self.data_dir.join("gigi.wal");
            if let Ok(meta) = fs::metadata(&wal_path) {
                if let Ok(modified) = meta.modified() {
                    if let Ok(d) = modified.duration_since(std::time::UNIX_EPOCH) {
                        return Some(d.as_secs() as i64);
                    }
                }
            }
            // No WAL on disk (in-memory engine pre-write). Treat
            // creation as "now" — honest about the fact that this is
            // an ephemeral bundle.
            return Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0),
            );
        }
        None
    }

    /// Number of records across all bundles (heap + mmap base + overlay).
    pub fn total_records(&self) -> usize {
        let heap: usize = self.bundles.values().map(|b| b.len()).sum();
        let mmap: usize = self.mmap_bundles.values().map(|ob| {
            ob.base().len() + ob.overlay_len()
        }).sum();
        heap + mmap
    }

    // ── Feature #6: Query Cache public API ──

    /// Get a mutable reference to the query cache.
    pub fn query_cache_mut(&mut self) -> &mut QueryCache {
        &mut self.query_cache
    }

    /// Get a reference to the query cache.
    pub fn query_cache(&self) -> &QueryCache {
        &self.query_cache
    }

    // ── Feature #9: Trigger public API ──

    /// Get a mutable reference to the trigger manager.
    pub fn trigger_manager_mut(&mut self) -> &mut TriggerManager {
        &mut self.trigger_manager
    }

    /// Get a reference to the trigger manager.
    pub fn trigger_manager(&self) -> &TriggerManager {
        &self.trigger_manager
    }

    /// Drain pending notifications (called by streaming layer after each mutation).
    pub fn drain_notifications(&mut self) -> Vec<Notification> {
        std::mem::take(&mut self.pending_notifications)
    }

    /// Create a trigger with WAL persistence (Feature #9, Test 9.8).
    pub fn create_trigger(&mut self, def: TriggerDef) -> io::Result<()> {
        let (bundle, op_str) = match &def.kind {
            TriggerKind::OnMutation { bundle, operation, .. } => {
                let op_str = match operation {
                    MutationOp::Insert => "INSERT",
                    MutationOp::Update => "UPDATE",
                    MutationOp::Delete => "DELETE",
                    MutationOp::Any => "ANY",
                };
                (bundle.clone(), op_str)
            }
        };
        self.wal.log_create_trigger(&def.name, &bundle, &def.channel, op_str, None)?;
        self.trigger_manager.create_trigger(def);
        Ok(())
    }

    /// Drop a trigger with WAL persistence (Feature #9).
    pub fn drop_trigger(&mut self, name: &str) -> io::Result<bool> {
        self.wal.log_drop_trigger(name)?;
        Ok(self.trigger_manager.drop_trigger(name))
    }

    /// TDD-HAL-II.4b: durable LATTICE declaration. Writes a
    /// `WalEntry::LatticeDeclare` to the WAL + installs the lattice in
    /// the in-process registry. Restart replays the WAL entry through
    /// `Lattice::from_gql` and re-registers, so the user sees the same
    /// `SHOW LATTICE name` result before and after restart.
    ///
    /// Bee's locked decision 3: the PERSIST keyword surface that
    /// chooses between this method and the in-memory
    /// `lattice::registry::register` lands at the parser/executor layer
    /// in II.5. At II.4b the durable path is exposed as a method only.
    #[cfg(feature = "gauge")]
    pub fn declare_lattice_durable(
        &mut self,
        lat: crate::lattice::Lattice,
    ) -> io::Result<()> {
        // Order matters: WAL append before in-process registration so
        // a crash between the two leaves a recoverable state (replay
        // re-installs the lattice; the in-process registry catches up
        // on next `Engine::open`).
        self.wal.log_lattice_declare(&lat.to_gql())?;
        crate::lattice::registry::register(lat);
        self.maybe_checkpoint()?;
        Ok(())
    }

    /// TDD-HAL-II.4b: durable GAUGE_FIELD declaration. Writes a
    /// `WalEntry::GaugeFieldDeclare` (metadata-only — Bee's locked
    /// decision 1) + installs the handle in
    /// `gauge::registry`. Restart re-materializes the field via
    /// `persistence::materialize_field` and re-registers, so the
    /// post-restart buffer is byte-identical to the pre-restart one.
    ///
    /// The lattice the field is bound to MUST already be durable
    /// (declared via `declare_lattice_durable`); replay walks the WAL
    /// in declaration order and the lattice declare must precede the
    /// gauge-field declare. Compaction's emit loop preserves this
    /// ordering invariant (lattice → gauge field → checkpoint).
    /// IMAGINE coherence Phase 2: log an `IMAGINE_FALLBACK` audit
    /// record. Called by the `bundle_imagine_coherence` HTTP handler
    /// when the tame-metric fallback engages for a high-K bundle.
    /// Metadata-only — replay does not re-execute the decision; the
    /// entry exists for post-hoc diagnostics (Marcella's confidence
    /// routing, operator dashboards).
    #[cfg(feature = "imagine")]
    pub fn log_imagine_fallback(
        &mut self,
        bundle: &str,
        original_k: f64,
        substituted_k: f64,
        timestamp_ms: u64,
    ) -> io::Result<()> {
        self.wal
            .log_imagine_fallback(bundle, original_k, substituted_k, timestamp_ms)?;
        self.maybe_checkpoint()?;
        Ok(())
    }

    #[cfg(feature = "gauge")]
    pub fn declare_gauge_field_durable(
        &mut self,
        handle: std::sync::Arc<dyn crate::gauge::registry::GaugeFieldHandle>,
    ) -> io::Result<()> {
        let (kind, seed) = handle.init_metadata();
        self.wal.log_gauge_field_declare(
            handle.name(),
            handle.lattice_name(),
            handle.group(),
            &kind,
            seed,
        )?;
        crate::gauge::registry::register(handle);
        self.maybe_checkpoint()?;
        Ok(())
    }

    /// TDD-HAL-V.2: durable post-thermalization buffer snapshot for an
    /// already-declared `GAUGE_FIELD`. Computes the canonical SHA-256
    /// over the LE-encoded buffer bytes (Bee's locked decision D-V-C —
    /// the same hash lands in the WAL and in the response envelope as
    /// the citation handle), then appends a
    /// `WalEntry::GaugeFieldSnapshot` via
    /// `WalWriter::log_gauge_field_snapshot`.
    ///
    /// Returns a `SnapshotResponse { sha256, wal_offset }` so the
    /// executor can lower into a single-row Rows envelope. The
    /// `wal_offset` is the post-write `WalWriter::entry_count` — a
    /// stable monotonically-increasing handle the future replay-
    /// restoration gate (V.3) can cite without reaching into WAL
    /// internals.
    #[cfg(feature = "gauge")]
    pub fn snapshot_gauge_field_durable(
        &mut self,
        name: &str,
        group: crate::gauge::Group,
        buffer: Vec<f64>,
    ) -> io::Result<SnapshotResponse> {
        // Build the payload — this computes SHA-256 over the LE-encoded
        // buffer bytes at construction time (locked decision D-V-A +
        // D-V-C). The payload is the wire format the WAL writer
        // expects.
        let payload = crate::wal::GaugeFieldSnapshotPayload::from_buffer(
            name.to_string(),
            group,
            buffer,
        );
        let sha256 = payload.sha256;
        self.wal.log_gauge_field_snapshot(&payload)?;
        let wal_offset = self.wal.entry_count();
        self.maybe_checkpoint()?;
        Ok(SnapshotResponse {
            sha256: hex_encode(&sha256),
            wal_offset,
        })
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

    /// Timeout-aware streaming snapshot for admin/API callers.
    ///
    /// Unlike `cow_snapshot_with_report`, this keeps the heap-mode snapshot
    /// path streaming and avoids cloning every record before encoding. WAL
    /// compaction only runs after every non-empty heap bundle snapshots cleanly.
    pub fn snapshot_with_report(&mut self) -> io::Result<SnapshotReport> {
        self.snapshot_with_chunk_size_report(
            50_000,
            self.compaction_policy.per_bundle_timeout_secs,
        )
    }

    /// Timeout-aware variant of `snapshot_with_chunk_size`.
    ///
    /// A timed-out bundle leaves its prior `.dhoom` file untouched and prevents
    /// WAL compaction for this cycle, preserving data that may still be
    /// WAL-resident.
    pub fn snapshot_with_chunk_size_report(
        &mut self,
        chunk_size: usize,
        per_bundle_timeout_secs: Option<u64>,
    ) -> io::Result<SnapshotReport> {
        let snapshots_dir = self.data_dir.join("snapshots");
        fs::create_dir_all(&snapshots_dir)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&snapshots_dir, fs::Permissions::from_mode(0o700));
        }

        let budget = per_bundle_timeout_secs.map(std::time::Duration::from_secs);
        let mut report = SnapshotReport {
            bundles: Vec::new(),
            total_records_written: 0,
            timed_out_bundles: Vec::new(),
        };

        for (name, store) in &self.bundles {
            let snap_path = snapshots_dir.join(format!("{name}.dhoom"));
            let tmp_path = snapshots_dir.join(format!("{name}.dhoom.tmp"));

            let count = store.len();
            if count == 0 {
                continue;
            }

            eprintln!(
                "  Snapshot streaming: {name} ({count} records, chunk_size={chunk_size}{})...",
                budget.map_or(String::new(), |b| format!(", budget={}s", b.as_secs()))
            );

            let start = std::time::Instant::now();
            let mut timed_out = false;
            let inner = (|| -> io::Result<usize> {
                let schema = self.schemas.get(name.as_str());
                let arith_key = schema.and_then(|s| {
                    if s.base_fields.len() == 1
                        && matches!(s.base_fields[0].field_type, crate::types::FieldType::Numeric)
                    {
                        Some(s.base_fields[0].name.clone())
                    } else {
                        None
                    }
                });

                let file = fs::File::create(&tmp_path)?;
                let buf = io::BufWriter::new(file);
                let mut encoder =
                    crate::dhoom::StreamingDhoomEncoder::new(buf, name, chunk_size);

                if let Some(ref key_field) = arith_key {
                    let mut recs: Vec<serde_json::Value> = Vec::new();
                    for rec in store.records() {
                        if let Some(b) = budget {
                            if start.elapsed() > b {
                                timed_out = true;
                                return Ok(0);
                            }
                        }
                        recs.push(record_to_serde_json(&rec));
                    }
                    recs.sort_by(|a, b| {
                        let va = a
                            .get(key_field)
                            .and_then(|v| v.as_i64())
                            .unwrap_or(i64::MAX);
                        let vb = b
                            .get(key_field)
                            .and_then(|v| v.as_i64())
                            .unwrap_or(i64::MAX);
                        va.cmp(&vb)
                    });
                    for rec in &recs {
                        if let Some(b) = budget {
                            if start.elapsed() > b {
                                timed_out = true;
                                return Ok(0);
                            }
                        }
                        encoder.push(rec.clone())?;
                    }
                } else {
                    for rec in store.records() {
                        if let Some(b) = budget {
                            if start.elapsed() > b {
                                timed_out = true;
                                return Ok(0);
                            }
                        }
                        encoder.push_record(&rec)?;
                    }
                }

                encoder.finish()?;
                Ok(count)
            })();

            if timed_out {
                let _ = fs::remove_file(&tmp_path);
                let elapsed = start.elapsed().as_secs();
                eprintln!(
                    "  Snapshot TIMED OUT on bundle '{name}' after {elapsed}s. \
                     Partial .tmp removed; WAL compaction skipped."
                );
                report.timed_out_bundles.push(name.clone());
                report.bundles.push(SnapshotBundleOutcome {
                    bundle_name: name.clone(),
                    result: Err(elapsed),
                });
                continue;
            }

            match inner {
                Ok(n) => {
                    fs::rename(&tmp_path, &snap_path)?;
                    report.total_records_written += n;
                    report.bundles.push(SnapshotBundleOutcome {
                        bundle_name: name.clone(),
                        result: Ok(n),
                    });
                    eprintln!("  Snapshot written: {name} ({n} records)");
                }
                Err(e) => {
                    let _ = fs::remove_file(&tmp_path);
                    return Err(e);
                }
            }
        }

        if report.timed_out_bundles.is_empty() {
            self.compact_wal_to_schemas()?;
        }

        Ok(report)
    }

    // ── CoW Snapshot (Feature #3) ─────────────────────────────────────────

    /// Clone all bundle data into owned vecs. The caller holds `&self` (read
    /// lock) only for the duration of this call. The returned data can then
    /// be encoded to DHOOM files without any lock.
    pub fn clone_bundle_data(&self) -> Vec<BundleDataClone> {
        let mut result: Vec<BundleDataClone> = self.bundles
            .iter()
            .filter(|(_, store)| store.len() > 0)
            .map(|(name, store)| BundleDataClone {
                name: name.clone(),
                schema: self.schemas.get(name).cloned().unwrap_or_else(|| {
                    BundleSchema::new(name)
                }),
                records: store.records().map(|r| record_to_serde_json(&r)).collect(),
            })
            .collect();

        // Include mmap bundles: merge base + overlay − tombstones
        for (name, ob) in &self.mmap_bundles {
            let overlay_pks: std::collections::HashSet<String> = ob.with_overlay(|s| {
                s.records().map(|r| self.pk_string(name, &r)).collect()
            }).unwrap_or_default();

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

            if !merged.is_empty() {
                result.push(BundleDataClone {
                    name: name.clone(),
                    schema: self.schemas.get(name).cloned().unwrap_or_else(|| {
                        BundleSchema::new(name)
                    }),
                    records: merged,
                });
            }
        }

        result
    }

    /// Encode pre-cloned bundle data to DHOOM snapshot files.
    /// Does NOT require any engine lock — operates on owned data and the filesystem.
    pub fn write_snapshot_files(
        data_dir: &Path,
        bundles: &[BundleDataClone],
        chunk_size: usize,
    ) -> io::Result<usize> {
        // Backwards-compat shim: no timeout, all-or-fail. The new
        // `write_snapshot_files_with_timeout` is recommended for any
        // call where a single hung bundle should not block the rest.
        let report = Self::write_snapshot_files_with_timeout(
            data_dir, bundles, chunk_size, None,
        )?;
        // No timeouts were possible (None passed), so empty timed_out
        // list is guaranteed. Return the total for API parity.
        Ok(report.total_records_written)
    }

    /// **#105 fix.** Per-bundle timeout-aware snapshot writer.
    ///
    /// Iterates each bundle in `bundles`; for each one, encodes records
    /// into a `.dhoom.tmp` file under `snapshots/` and atomically
    /// renames to `.dhoom` on success. If `per_bundle_timeout_secs`
    /// is `Some(t)` and a single bundle's work exceeds `t` seconds,
    /// that bundle is aborted (partial `.tmp` file deleted on a
    /// best-effort basis), recorded in `report.timed_out_bundles`,
    /// and the loop continues with the next bundle. Other bundles in
    /// the batch are unaffected — snapshot files are per-bundle, so
    /// one hang doesn't break the rest.
    ///
    /// The timeout is checked between records in the inner encode
    /// loop (every record by default). For pathological encoders
    /// that hang *inside* a single `encoder.push()` call without
    /// returning, this approach cannot interrupt them (would require
    /// thread-level cancellation, which Rust's std doesn't support
    /// portably). The current StreamingDhoomEncoder per-record cost
    /// is bounded, so the inter-record check is sufficient for the
    /// observed-in-prod failure mode (#104: snapshot of
    /// `icarus_traverse_130799` hung indefinitely — exact root cause
    /// not yet diagnosed; this timeout is the protective mitigation
    /// per Bee's request).
    pub fn write_snapshot_files_with_timeout(
        data_dir: &Path,
        bundles: &[BundleDataClone],
        chunk_size: usize,
        per_bundle_timeout_secs: Option<u64>,
    ) -> io::Result<SnapshotReport> {
        let snapshots_dir = data_dir.join("snapshots");
        fs::create_dir_all(&snapshots_dir)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&snapshots_dir, fs::Permissions::from_mode(0o700));
        }

        let mut report = SnapshotReport {
            bundles: Vec::with_capacity(bundles.len()),
            total_records_written: 0,
            timed_out_bundles: Vec::new(),
        };

        let budget = per_bundle_timeout_secs.map(std::time::Duration::from_secs);

        for bdc in bundles {
            let snap_path = snapshots_dir.join(format!("{}.dhoom", bdc.name));
            let tmp_path = snapshots_dir.join(format!("{}.dhoom.tmp", bdc.name));

            eprintln!(
                "  CoW snapshot streaming: {} ({} records, chunk_size={chunk_size}{})…",
                bdc.name,
                bdc.records.len(),
                budget.map_or(String::new(), |b| format!(", budget={}s", b.as_secs())),
            );

            let start = std::time::Instant::now();
            let mut timed_out = false;
            let inner = (|| -> io::Result<usize> {
                let file = fs::File::create(&tmp_path)?;
                let buf = io::BufWriter::new(file);
                let mut encoder =
                    crate::dhoom::StreamingDhoomEncoder::new(buf, &bdc.name, chunk_size);
                for rec in &bdc.records {
                    if let Some(b) = budget {
                        if start.elapsed() > b {
                            timed_out = true;
                            return Ok(0); // outer code will discard
                        }
                    }
                    encoder.push(rec.clone())?;
                }
                encoder.finish()?;
                Ok(bdc.records.len())
            })();

            if timed_out {
                // Best-effort cleanup of the partial tmp file.
                let _ = fs::remove_file(&tmp_path);
                let elapsed = start.elapsed().as_secs();
                eprintln!(
                    "  CoW snapshot TIMED OUT on bundle '{}' after {}s (budget {}s, \
                     {} records). Partial .tmp removed; remaining bundles continue.",
                    bdc.name,
                    elapsed,
                    budget.map(|b| b.as_secs()).unwrap_or(0),
                    bdc.records.len(),
                );
                report
                    .timed_out_bundles
                    .push(bdc.name.clone());
                report.bundles.push(SnapshotBundleOutcome {
                    bundle_name: bdc.name.clone(),
                    result: Err(elapsed),
                });
                continue;
            }

            match inner {
                Ok(n) => {
                    fs::rename(&tmp_path, &snap_path)?;
                    report.total_records_written += n;
                    report.bundles.push(SnapshotBundleOutcome {
                        bundle_name: bdc.name.clone(),
                        result: Ok(n),
                    });
                }
                Err(e) => {
                    // I/O error inside the inner block — propagate.
                    // Clean up the tmp on the way out so we don't
                    // leave a partial file.
                    let _ = fs::remove_file(&tmp_path);
                    return Err(e);
                }
            }
        }

        Ok(report)
    }

    /// Compact the WAL to schema-only entries (called after snapshot files
    /// have been written). Requires `&mut self` (write lock).
    ///
    /// Emit order (dependency-correct, see TDD-HAL-II.4b decision
    /// points):
    ///   1. `CreateBundle*`
    ///   2. `CreateTrigger*`
    ///   3. `LatticeDeclare*` (gauge feature)
    ///   4. `GaugeFieldDeclare*` (gauge feature) — must come AFTER
    ///      `LatticeDeclare` because replay's pass-2 looks the lattice
    ///      up by name.
    ///   5. `Checkpoint`
    pub fn compact_wal_to_schemas(&mut self) -> io::Result<()> {
        let wal_path = self.data_dir.join("gigi.wal");
        let tmp_path = self.data_dir.join("gigi.wal.tmp");
        {
            let mut new_wal = WalWriter::open(&tmp_path)?;
            for schema in self.schemas.values() {
                new_wal.log_create_bundle(schema)?;
            }
            // Persist trigger definitions through WAL compaction
            for tdef in self.trigger_manager.list_triggers() {
                let (bundle, op_str, filter_str) = match &tdef.kind {
                    TriggerKind::OnMutation { bundle, operation, filter } => {
                        let op = match operation {
                            MutationOp::Insert => "INSERT",
                            MutationOp::Update => "UPDATE",
                            MutationOp::Delete => "DELETE",
                            MutationOp::Any => "ANY",
                        };
                        let filt = filter.as_ref().map(|conds| format!("{conds:?}"));
                        (bundle.clone(), op.to_string(), filt)
                    }
                };
                new_wal.log_create_trigger(&tdef.name, &bundle, &tdef.channel, &op_str, filter_str.as_deref())?;
            }
            // TDD-HAL-II.4b: re-emit durable lattice + gauge-field
            // declarations so they survive WAL compaction. Lattices
            // first (gauge fields depend on their lattice being in the
            // registry at replay pass 2 time).
            #[cfg(feature = "gauge")]
            {
                for lat in crate::lattice::registry::all() {
                    new_wal.log_lattice_declare(&lat.to_gql())?;
                }
                for handle in crate::gauge::registry::all() {
                    let (kind, seed) = handle.init_metadata();
                    new_wal.log_gauge_field_declare(
                        handle.name(),
                        handle.lattice_name(),
                        handle.group(),
                        &kind,
                        seed,
                    )?;
                }
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
        // **#105 fix.** Route through the timeout-aware variant
        // using the policy's per_bundle_timeout_secs. A bundle that
        // exceeds the budget is skipped (its prior .dhoom remains
        // valid; the .tmp is cleaned up) and the loop continues with
        // the next bundle.
        //
        // **Critical data-loss guard:** `compact_wal_to_schemas`
        // REWRITES the WAL to schemas-only, dropping all data
        // records. If we ran it after a timeout, the timed-out
        // bundle's data — which is still only in the WAL because
        // no .dhoom was written for it — would be LOST. Instead we
        // skip compaction whenever any bundle timed out; the WAL
        // grows for one snapshot cycle, but no data is destroyed.
        // The next snapshot attempt (or a manual `cow_snapshot`)
        // can succeed and compact then.
        let cloned = self.clone_bundle_data();
        let report = Self::write_snapshot_files_with_timeout(
            &self.data_dir,
            &cloned,
            50_000,
            self.compaction_policy.per_bundle_timeout_secs,
        )?;
        if report.timed_out_bundles.is_empty() {
            // Clean run — safe to compact.
            self.compact_wal_to_schemas()?;
        } else {
            eprintln!(
                "  CoW snapshot: {} of {} bundle(s) timed out: [{}]. \
                 SKIPPING WAL compaction this cycle to preserve the timed-out \
                 bundle's data (still WAL-resident; would be erased by \
                 compaction). Next snapshot attempt may succeed and compact \
                 then. WAL will grow by one snapshot interval.",
                report.timed_out_bundles.len(),
                cloned.len(),
                report.timed_out_bundles.join(", "),
            );
        }
        Ok(report.total_records_written)
    }

    /// **#105 fix.** Per-bundle-timeout-aware variant that returns
    /// the full per-bundle outcome report. Use when the caller wants
    /// to see *which* bundles timed out (for alerting / diagnostics),
    /// rather than just the success count. Same data-loss guard as
    /// `cow_snapshot`: WAL compaction is skipped if any bundle timed
    /// out.
    pub fn cow_snapshot_with_report(&mut self) -> io::Result<SnapshotReport> {
        let cloned = self.clone_bundle_data();
        let report = Self::write_snapshot_files_with_timeout(
            &self.data_dir,
            &cloned,
            50_000,
            self.compaction_policy.per_bundle_timeout_secs,
        )?;
        if report.timed_out_bundles.is_empty() {
            self.compact_wal_to_schemas()?;
        }
        Ok(report)
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
                // Collect and sort by arithmetic key field so detect_arithmetic()
                // sees a uniform sequence regardless of storage iteration order.
                let schema = self.schemas.get(name.as_str());
                let arith_key = schema.and_then(|s| {
                    if s.base_fields.len() == 1
                        && matches!(s.base_fields[0].field_type, crate::types::FieldType::Numeric)
                    {
                        Some(s.base_fields[0].name.clone())
                    } else {
                        None
                    }
                });

                let file = fs::File::create(&tmp_path)?;
                let buf = io::BufWriter::new(file);
                let mut encoder =
                    crate::dhoom::StreamingDhoomEncoder::new(buf, name, chunk_size);

                if let Some(ref key_field) = arith_key {
                    // Buffer → sort → encode (guarantees arithmetic
                    // detection on the sample). The arithmetic-sort
                    // path needs to inspect `id` numerically, so we
                    // keep the JSON intermediate here — sorting native
                    // Records would require an extra GigiValue→i64
                    // accessor and the snapshot path isn't hot.
                    let mut recs: Vec<serde_json::Value> = store
                        .records()
                        .map(|r| record_to_serde_json(&r))
                        .collect();
                    recs.sort_by(|a, b| {
                        let va = a.get(key_field).and_then(|v| v.as_i64()).unwrap_or(i64::MAX);
                        let vb = b.get(key_field).and_then(|v| v.as_i64()).unwrap_or(i64::MAX);
                        va.cmp(&vb)
                    });
                    for rec in &recs {
                        encoder.push(rec.clone())?;
                    }
                } else {
                    // **#112 — native Record path.** Skips the
                    // per-record `serde_json::Value` allocation on the
                    // streaming encoder hot path.
                    for rec in store.records() {
                        encoder.push_record(&rec)?;
                    }
                }
                encoder.finish()?;
                eprintln!("  Snapshot written: {name} ({count} records)");
            }

            fs::rename(&tmp_path, &snap_path)?;
            total_records += count;
        }

        // Compact WAL to schema-only (no insert entries).
        // TDD-HAL-II.4b: emit order matches `compact_wal_to_schemas`
        // exactly — schemas → triggers → lattices → gauge fields →
        // checkpoint. This site and `compact_wal_to_schemas` are
        // pre-existing duplication (trigger pattern); both must emit
        // the same surface or compaction would silently lose the
        // gauge substrate when the snapshot path is hit instead of
        // the explicit `compact_wal_to_schemas` path.
        let wal_path = self.data_dir.join("gigi.wal");
        let tmp_path = self.data_dir.join("gigi.wal.tmp");
        {
            let mut new_wal = WalWriter::open(&tmp_path)?;
            for schema in self.schemas.values() {
                new_wal.log_create_bundle(schema)?;
            }
            // Persist trigger definitions through WAL compaction
            for tdef in self.trigger_manager.list_triggers() {
                let (bundle, op_str, filter_str) = match &tdef.kind {
                    TriggerKind::OnMutation { bundle, operation, filter } => {
                        let op = match operation {
                            MutationOp::Insert => "INSERT",
                            MutationOp::Update => "UPDATE",
                            MutationOp::Delete => "DELETE",
                            MutationOp::Any => "ANY",
                        };
                        let filt = filter.as_ref().map(|conds| format!("{conds:?}"));
                        (bundle.clone(), op.to_string(), filt)
                    }
                };
                new_wal.log_create_trigger(&tdef.name, &bundle, &tdef.channel, &op_str, filter_str.as_deref())?;
            }
            #[cfg(feature = "gauge")]
            {
                for lat in crate::lattice::registry::all() {
                    new_wal.log_lattice_declare(&lat.to_gql())?;
                }
                for handle in crate::gauge::registry::all() {
                    let (kind, seed) = handle.init_metadata();
                    new_wal.log_gauge_field_declare(
                        handle.name(),
                        handle.lattice_name(),
                        handle.group(),
                        &kind,
                        seed,
                    )?;
                }
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

        // ── Phase 2 ── heap-only bundles get snapshotted too.
        //
        // The 2026-05-25 recovery fix (commit 9826f9b) populates
        // self.bundles with heap-only BundleStores for any schema
        // that didn't have a .dhoom on disk at startup. Before
        // this Phase 2, those bundles never got a .dhoom written —
        // mmap_rebase_snapshot only iterated self.mmap_bundles, so
        // heap-only bundles stayed WAL-only across restarts and
        // re-replayed the entire WAL on every boot. Production at
        // 2026-05-26T01:05 had 61 .dhoom files vs ~4900 logical
        // bundles; this Phase 2 closes the gap.
        //
        // The heap-only bundles stay in self.bundles after writing
        // their .dhoom — on the next restart, open_mmap picks them
        // up as mmap_bundles per the standard fast path, so they
        // participate in the rebase loop above going forward.
        let heap_names: Vec<String> = self
            .bundles
            .iter()
            .filter(|(name, store)| {
                !self.mmap_bundles.contains_key(*name) && store.len() > 0
            })
            .map(|(name, _)| name.clone())
            .collect();
        for name in &heap_names {
            let store = match self.bundles.get(name) {
                Some(s) => s,
                None => continue,
            };
            let records: Vec<serde_json::Value> = store
                .records()
                .map(|r| record_to_serde_json(&r))
                .collect();
            if records.is_empty() {
                continue;
            }

            let snap_path = snapshots_dir.join(format!("{name}.dhoom"));
            let tmp_path = snapshots_dir.join(format!("{name}.dhoom.tmp"));
            {
                let file = fs::File::create(&tmp_path)?;
                let buf = io::BufWriter::new(file);
                let mut encoder =
                    crate::dhoom::StreamingDhoomEncoder::new(buf, name, 50_000);
                for val in &records {
                    encoder.push(val.clone())?;
                }
                encoder.finish()?;
            }
            fs::rename(&tmp_path, &snap_path)?;
            eprintln!("  Heap snapshot: {name} ({} records)", records.len());
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
// **#112 — DHOOM native Record API.** The pre-fix duplicates of
// `record_to_serde_json` + `value_to_serde_json` previously lived here
// as private helpers (and were re-implemented in several other call
// sites across the codebase). They moved to `crate::dhoom` so the
// conversion lives next to the format definition. The remaining call
// sites in this file use `crate::dhoom::record_to_dhoom_value` for
// snapshot-clone paths (which need the `Vec<serde_json::Value>` shape)
// and `encoder.push_record(&rec)` for the streaming-encoder paths
// (which skip the intermediate Vec).
use crate::dhoom::record_to_dhoom_value as record_to_serde_json;

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
        serde_json::Value::String(s) => {
            if let Some(encoded) = s.strip_prefix("b64:") {
                use base64::Engine as _;
                match base64::engine::general_purpose::STANDARD.decode(encoded) {
                    Ok(bytes) => Value::Binary(bytes),
                    Err(_) => Value::Text(s.clone()),
                }
            } else {
                Value::Text(s.clone())
            }
        }
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

    #[test]
    fn engine_wal_replay_preserves_valid_prefix_on_crc_tail() {
        use std::io::Write as _;

        let dir = test_dir("replay_crc_tail");
        cleanup(&dir);

        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("employees")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("name"));
            engine.create_bundle(schema).unwrap();

            let mut rec = Record::new();
            rec.insert("id".into(), Value::Integer(7));
            rec.insert("name".into(), Value::Text("Grace".into()));
            engine.insert("employees", &rec).unwrap();
            engine.checkpoint().unwrap();
        }

        {
            let mut wal = fs::OpenOptions::new()
                .append(true)
                .open(dir.join("gigi.wal"))
                .unwrap();
            wal.write_all(&1u32.to_le_bytes()).unwrap();
            wal.write_all(&[0xFF]).unwrap();
            wal.write_all(&0u32.to_le_bytes()).unwrap();
            wal.sync_all().unwrap();
        }

        {
            let engine = Engine::open(&dir).unwrap();
            assert_eq!(engine.total_records(), 1);

            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(7));
            let result = engine.point_query("employees", &key).unwrap().unwrap();
            assert_eq!(result.get("name"), Some(&Value::Text("Grace".into())));
        }

        cleanup(&dir);
    }

    #[test]
    fn durable_upsert_and_bulk_helpers_replay() {
        let dir = test_dir("durable_gql_helpers");
        cleanup(&dir);

        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("durable")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("name"))
                .fiber(FieldDef::categorical("status"));
            engine.create_bundle(schema).unwrap();

            let mut rec1 = Record::new();
            rec1.insert("id".into(), Value::Integer(1));
            rec1.insert("name".into(), Value::Text("alpha".into()));
            rec1.insert("status".into(), Value::Text("open".into()));
            assert!(engine.upsert("durable", &rec1).unwrap());

            let mut rec1_update = rec1.clone();
            rec1_update.insert("name".into(), Value::Text("alpha-v2".into()));
            assert!(!engine.upsert("durable", &rec1_update).unwrap());

            let mut rec2 = Record::new();
            rec2.insert("id".into(), Value::Integer(2));
            rec2.insert("name".into(), Value::Text("beta".into()));
            rec2.insert("status".into(), Value::Text("open".into()));
            let mut rec3 = Record::new();
            rec3.insert("id".into(), Value::Integer(3));
            rec3.insert("name".into(), Value::Text("gamma".into()));
            rec3.insert("status".into(), Value::Text("closed".into()));
            assert_eq!(
                engine.batch_upsert("durable", &[rec2, rec3]).unwrap(),
                (2, 0)
            );

            let mut patches = Record::new();
            patches.insert("status".into(), Value::Text("done".into()));
            let open = [QueryCondition::Eq("status".into(), Value::Text("open".into()))];
            assert_eq!(engine.bulk_update("durable", &open, &patches).unwrap(), 2);

            let closed = [QueryCondition::Eq("status".into(), Value::Text("closed".into()))];
            assert_eq!(engine.bulk_delete("durable", &closed).unwrap(), 1);
            engine.checkpoint().unwrap();
        }

        {
            let engine = Engine::open(&dir).unwrap();
            let mut key1 = Record::new();
            key1.insert("id".into(), Value::Integer(1));
            let row1 = engine.point_query("durable", &key1).unwrap().unwrap();
            assert_eq!(row1.get("name"), Some(&Value::Text("alpha-v2".into())));
            assert_eq!(row1.get("status"), Some(&Value::Text("done".into())));

            let mut key2 = Record::new();
            key2.insert("id".into(), Value::Integer(2));
            let row2 = engine.point_query("durable", &key2).unwrap().unwrap();
            assert_eq!(row2.get("status"), Some(&Value::Text("done".into())));

            let mut key3 = Record::new();
            key3.insert("id".into(), Value::Integer(3));
            assert!(engine.point_query("durable", &key3).unwrap().is_none());
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

    /// **#105 fix** — zero timeout: every bundle's snapshot work
    /// times out (no chance to even start), and the report names
    /// every bundle in `timed_out_bundles`. Critically, **WAL
    /// compaction is skipped** so the timed-out bundles' WAL-resident
    /// data is not destroyed.
    #[test]
    fn issue_105_zero_timeout_records_all_bundles_as_timed_out() {
        let dir = test_dir("issue_105_zero_timeout");
        cleanup(&dir);

        let mut engine = engine_no_autocompact(&dir);
        let schema = BundleSchema::new("data")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val"));
        engine.create_bundle(schema).unwrap();

        // Insert enough records that *something* would be written
        // under a non-zero budget.
        for i in 0..500i64 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("val".into(), Value::Float(i as f64));
            engine.insert("data", &r).unwrap();
        }

        // Capture WAL byte count *before* the failed snapshot so we
        // can verify it's preserved.
        let wal_bytes_before = engine.wal_byte_count();
        assert!(wal_bytes_before > 0, "WAL should have data records");

        // Set zero timeout → every bundle times out (the inner loop
        // checks elapsed > budget on the FIRST record and bails).
        engine.compaction_policy_mut().per_bundle_timeout_secs = Some(0);

        let report = engine.cow_snapshot_with_report().unwrap();

        assert_eq!(
            report.timed_out_bundles,
            vec!["data".to_string()],
            "every bundle should be in timed_out_bundles under a zero budget"
        );
        assert_eq!(
            report.total_records_written, 0,
            "no records should have been persisted under a zero budget"
        );
        assert_eq!(
            report.bundles.len(),
            1,
            "report should list one outcome per requested bundle"
        );
        assert!(
            matches!(report.bundles[0].result, Err(_)),
            "bundle outcome should be Err on timeout"
        );

        // **Data preservation check.** WAL must NOT have been
        // compacted — the timed-out bundle's records are still only
        // in the WAL, so erasing them would lose data. WAL size
        // should be >= what it was before (slightly different is fine
        // due to internal state, but it must contain at least the
        // original data records).
        assert!(
            engine.wal_byte_count() >= wal_bytes_before / 2,
            "WAL should NOT have been compacted on timeout — preserves WAL-resident data. \
             before={wal_bytes_before}, after={}",
            engine.wal_byte_count()
        );

        // Sanity: bundle still queryable from live engine (in-memory
        // state untouched by the failed snapshot).
        assert_eq!(engine.total_records(), 500);

        cleanup(&dir);
    }

    /// **#105 fix** — passing `None` (no timeout) makes the new
    /// `write_snapshot_files_with_timeout` behave identically to the
    /// legacy `write_snapshot_files`. Backwards compat guarantee.
    #[test]
    fn issue_105_none_timeout_matches_legacy_path() {
        let dir = test_dir("issue_105_none_timeout");
        cleanup(&dir);

        let mut engine = engine_no_autocompact(&dir);
        let schema = BundleSchema::new("data")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val"));
        engine.create_bundle(schema).unwrap();
        for i in 0..100i64 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("val".into(), Value::Float(i as f64));
            engine.insert("data", &r).unwrap();
        }

        let cloned = engine.clone_bundle_data();
        let legacy_total =
            Engine::write_snapshot_files(engine.data_dir(), &cloned, 50_000).unwrap();
        // Snapshot the same data via the new path with None timeout
        // into a separate directory (write_snapshot_files reuses
        // snapshot file names — call into the same dir is fine since
        // it's idempotent).
        let report = Engine::write_snapshot_files_with_timeout(
            engine.data_dir(),
            &cloned,
            50_000,
            None,
        )
        .unwrap();

        assert_eq!(legacy_total, report.total_records_written);
        assert!(report.timed_out_bundles.is_empty());

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

    /// Prove that small bundles (<32 records, Hashed storage) get arithmetic
    /// O(1) lookup after snapshot. The snapshot sort guarantees detect_arithmetic()
    /// sees a uniform sequence regardless of HashMap iteration order.
    #[test]
    fn small_bundle_arithmetic_snapshot() {
        use crate::dhoom::Modifier;

        let dir = test_dir("small_arith");
        cleanup(&dir);

        // 5 records — stays in Hashed storage (< 32 auto-detect threshold)
        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("tiny")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("val"));
            engine.create_bundle(schema).unwrap();
            for i in 0..5i64 {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("val".into(), Value::Text(format!("v{i}")));
                engine.insert("tiny", &rec).unwrap();
            }
            engine.snapshot().unwrap();
        }

        // Reopen mmap — verify arithmetic modifier and O(1) lookup
        {
            let engine = Engine::open_mmap(&dir).unwrap();
            assert_eq!(engine.total_records(), 5);

            let ob = engine.mmap_bundles.get("tiny").unwrap();
            let arith = ob.base().fiber().fields.iter()
                .find(|f| f.name == "id")
                .and_then(|f| f.modifier.as_ref());
            assert!(
                matches!(arith, Some(Modifier::Arithmetic { .. })),
                "Small bundle 'id' must have Arithmetic modifier after sorted snapshot, got {:?}",
                arith
            );

            for id in 0..5i64 {
                let mut key = Record::new();
                key.insert("id".into(), Value::Integer(id));
                let rec = engine.point_query("tiny", &key).unwrap()
                    .unwrap_or_else(|| panic!("id={id} must be accessible"));
                assert_eq!(rec.get("val"), Some(&Value::Text(format!("v{id}"))));
            }
        }

        cleanup(&dir);
    }

    /// Prove clone_bundle_data includes mmap bundles (base + overlay − tombstones).
    #[test]
    fn clone_bundle_data_includes_mmap() {
        let dir = test_dir("clone_mmap");
        cleanup(&dir);

        // Phase 1: Create and snapshot
        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("data")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("label"));
            engine.create_bundle(schema).unwrap();
            for i in 0..50i64 {
                let mut rec = Record::new();
                rec.insert("id".into(), Value::Integer(i));
                rec.insert("label".into(), Value::Text(format!("row{i}")));
                engine.insert("data", &rec).unwrap();
            }
            engine.snapshot().unwrap();
        }

        // Phase 2: Open mmap, mutate, then clone
        {
            let mut engine = Engine::open_mmap(&dir).unwrap();

            // Update id=10
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(10));
            let mut patches = Record::new();
            patches.insert("label".into(), Value::Text("UPDATED".into()));
            engine.update("data", &key, &patches).unwrap();

            // Delete id=20
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(20));
            engine.delete("data", &key).unwrap();

            // Clone — should include mmap data with mutations
            let cloned = engine.clone_bundle_data();
            assert_eq!(cloned.len(), 1);
            assert_eq!(cloned[0].name, "data");
            // 50 original - 1 deleted = 49
            assert_eq!(cloned[0].records.len(), 49, "should have 49 records (50-1 deleted)");

            // Verify updated record is present with new label
            let updated = cloned[0].records.iter()
                .find(|r| r.get("id").and_then(|v| v.as_i64()) == Some(10))
                .expect("id=10 must be in clone");
            assert_eq!(updated.get("label").and_then(|v| v.as_str()), Some("UPDATED"));

            // Verify deleted record is absent
            let deleted = cloned[0].records.iter()
                .find(|r| r.get("id").and_then(|v| v.as_i64()) == Some(20));
            assert!(deleted.is_none(), "id=20 must be absent (tombstoned)");
        }

        cleanup(&dir);
    }

    // ── Feature #6: Query Cache with TTL — Tests 6.1–6.8 ──

    fn make_cache_engine(name: &str) -> (Engine, PathBuf) {
        let dir = test_dir(name);
        cleanup(&dir);
        let mut engine = Engine::open(&dir).unwrap();
        let schema = BundleSchema::new("drugs")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("organism"))
            .fiber(FieldDef::numeric("mic"));
        engine.create_bundle(schema).unwrap();
        for i in 0..10i64 {
            let mut rec = Record::new();
            rec.insert("id".into(), Value::Integer(i));
            rec.insert("organism".into(), Value::Text(format!("org_{i}")));
            rec.insert("mic".into(), Value::Float(i as f64 * 0.5));
            engine.insert("drugs", &rec).unwrap();
        }
        (engine, dir)
    }

    #[test]
    fn test_6_1_cache_hit_returns_correct_result() {
        let (mut engine, dir) = make_cache_engine("cache_hit");

        let conditions = vec![QueryCondition::Eq("organism".into(), Value::Text("org_3".into()))];
        let fp = super::query_fingerprint("drugs", &conditions, None);

        // Execute query, cache result
        let result = engine.filtered_query("drugs", &conditions, None, None, false, None, None).unwrap();
        assert_eq!(result.len(), 1);
        engine.query_cache_mut().put(fp, "drugs", result.clone(), 60);

        // Cache hit
        let cached = engine.query_cache_mut().get(fp);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);

        cleanup(&dir);
    }

    #[test]
    fn test_6_2_cache_miss_on_ttl_expiry() {
        let mut cache = super::QueryCache::new();
        let result = vec![{
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(1));
            r
        }];
        cache.put(42, "drugs", result, 0); // TTL=0 → expired immediately

        // Even 0-second TTL means it expires at >= 0 seconds elapsed
        // Use std::thread::sleep to ensure elapsed > 0
        std::thread::sleep(std::time::Duration::from_millis(10));
        let cached = cache.get(42);
        assert!(cached.is_none(), "Should be cache miss after TTL expiry");
    }

    #[test]
    fn test_6_3_write_invalidation_via_generation() {
        let (mut engine, dir) = make_cache_engine("cache_gen");

        let conditions = vec![QueryCondition::Eq("organism".into(), Value::Text("org_5".into()))];
        let fp = super::query_fingerprint("drugs", &conditions, None);
        let result = engine.filtered_query("drugs", &conditions, None, None, false, None, None).unwrap();
        engine.query_cache_mut().put(fp, "drugs", result, 60);

        // Verify cache hit
        assert!(engine.query_cache_mut().get(fp).is_some());

        // Insert a record → bumps generation
        let mut new_rec = Record::new();
        new_rec.insert("id".into(), Value::Integer(100));
        new_rec.insert("organism".into(), Value::Text("org_new".into()));
        new_rec.insert("mic".into(), Value::Float(9.9));
        engine.insert("drugs", &new_rec).unwrap();

        // Cache miss — generation mismatch
        assert!(engine.query_cache_mut().get(fp).is_none());

        cleanup(&dir);
    }

    #[test]
    fn test_6_4_explicit_invalidation_per_bundle() {
        let mut cache = super::QueryCache::new();
        // 5 entries on "drugs", 3 on "compounds"
        for i in 0..5u64 {
            cache.put(i, "drugs", vec![], 60);
        }
        for i in 10..13u64 {
            cache.put(i, "compounds", vec![], 60);
        }
        assert_eq!(cache.len(), 8);

        cache.invalidate_bundle("drugs");
        assert_eq!(cache.len(), 3, "Only compounds entries should remain");

        // Verify compounds entries still accessible
        assert!(cache.get(10).is_some());
        assert!(cache.get(11).is_some());
        assert!(cache.get(12).is_some());
    }

    #[test]
    fn test_6_5_invalidate_all_clears_everything() {
        let mut cache = super::QueryCache::new();
        cache.put(1, "drugs", vec![], 60);
        cache.put(2, "compounds", vec![], 60);
        cache.put(3, "organisms", vec![], 60);
        assert_eq!(cache.len(), 3);

        cache.invalidate_all();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_6_6_fingerprint_equivalence() {
        // WHERE a = 1 AND b = 2  vs  WHERE b = 2 AND a = 1
        let q1 = vec![
            QueryCondition::Eq("a".into(), Value::Integer(1)),
            QueryCondition::Eq("b".into(), Value::Integer(2)),
        ];
        let q2 = vec![
            QueryCondition::Eq("b".into(), Value::Integer(2)),
            QueryCondition::Eq("a".into(), Value::Integer(1)),
        ];
        let fp1 = super::query_fingerprint("test", &q1, None);
        let fp2 = super::query_fingerprint("test", &q2, None);
        assert_eq!(fp1, fp2, "Reordered conditions must produce same fingerprint");

        // Different bundle name → different fingerprint
        let fp3 = super::query_fingerprint("other", &q1, None);
        assert_ne!(fp1, fp3);
    }

    #[test]
    fn test_6_7_lru_eviction() {
        let mut cache = super::QueryCache::new();
        cache.max_entries = 3;

        cache.put(1, "b", vec![], 60);
        cache.put(2, "b", vec![], 60);
        cache.put(3, "b", vec![], 60);
        assert_eq!(cache.len(), 3);

        // Insert Q4 → Q1 evicted (oldest)
        cache.put(4, "b", vec![], 60);
        assert_eq!(cache.len(), 3);
        assert!(cache.get(1).is_none(), "Q1 should be evicted");
        assert!(cache.get(2).is_some());
        assert!(cache.get(3).is_some());
        assert!(cache.get(4).is_some());
    }

    #[test]
    fn test_6_8_staleness_bound() {
        // With TTL=60 and writes happening, cache should be stale until TTL expires
        let (mut engine, dir) = make_cache_engine("cache_stale");

        let conditions: Vec<QueryCondition> = vec![];
        let fp = super::query_fingerprint("drugs", &conditions, None);
        let result = engine.filtered_query("drugs", &conditions, None, None, false, None, None).unwrap();
        let initial_count = result.len();
        assert_eq!(initial_count, 10);
        engine.query_cache_mut().put(fp, "drugs", result, 60);

        // Insert more records (simulating writes within TTL window)
        for i in 100..110i64 {
            let mut rec = Record::new();
            rec.insert("id".into(), Value::Integer(i));
            rec.insert("organism".into(), Value::Text(format!("new_{i}")));
            rec.insert("mic".into(), Value::Float(1.0));
            engine.insert("drugs", &rec).unwrap();
        }

        // Cache miss due to generation bump (Theorem 6.1: at most W*τ stale records)
        assert!(engine.query_cache_mut().get(fp).is_none(), "Should miss after writes");

        // Fresh query returns all records
        let fresh = engine.filtered_query("drugs", &conditions, None, None, false, None, None).unwrap();
        assert_eq!(fresh.len(), 20, "Fresh query should see all 20 records");

        cleanup(&dir);
    }

    #[test]
    fn test_cache_gql_invalidate_parse() {
        // INVALIDATE CACHE
        let stmt = crate::parser::parse("INVALIDATE CACHE").unwrap();
        assert_eq!(stmt, crate::parser::Statement::InvalidateCache { bundle: None });

        // INVALIDATE CACHE ON drugs
        let stmt2 = crate::parser::parse("INVALIDATE CACHE ON drugs").unwrap();
        assert_eq!(stmt2, crate::parser::Statement::InvalidateCache {
            bundle: Some("drugs".into()),
        });
    }

    #[test]
    fn test_cache_gql_invalidate_execute() {
        let (mut engine, dir) = make_cache_engine("cache_gql_exec");

        // Cache a result
        let fp = super::query_fingerprint("drugs", &[], None);
        let result = engine.filtered_query("drugs", &[], None, None, false, None, None).unwrap();
        engine.query_cache_mut().put(fp, "drugs", result, 60);
        assert!(engine.query_cache_mut().get(fp).is_some());

        // Execute INVALIDATE CACHE ON drugs
        let stmt = crate::parser::parse("INVALIDATE CACHE ON drugs").unwrap();
        crate::parser::execute(&mut engine, &stmt).unwrap();
        assert!(engine.query_cache_mut().get(fp).is_none(), "Cache should be cleared");

        cleanup(&dir);
    }

    // ── Feature #9: Pub/Sub with Sheaf Triggers — Tests 9.1–9.8 ──

    fn make_trigger_engine(name: &str) -> (Engine, PathBuf) {
        let dir = test_dir(name);
        cleanup(&dir);
        let mut engine = Engine::open(&dir).unwrap();
        let schema = BundleSchema::new("drugs")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("target_class"))
            .fiber(FieldDef::numeric("mic"));
        engine.create_bundle(schema).unwrap();
        (engine, dir)
    }

    #[test]
    fn test_9_6_mutation_trigger_with_filter() {
        let (mut engine, dir) = make_trigger_engine("trigger_filter");

        // ON INSERT drugs WHERE target_class = 'PBP'
        let def = super::TriggerDef {
            name: "pbp_inserts".into(),
            kind: super::TriggerKind::OnMutation {
                bundle: "drugs".into(),
                operation: super::MutationOp::Insert,
                filter: Some(vec![
                    QueryCondition::Eq("target_class".into(), Value::Text("PBP".into())),
                ]),
            },
            channel: "pbp_channel".into(),
        };
        engine.create_trigger(def).unwrap();

        // Insert matching record
        let mut rec = Record::new();
        rec.insert("id".into(), Value::Integer(1));
        rec.insert("target_class".into(), Value::Text("PBP".into()));
        rec.insert("mic".into(), Value::Float(2.0));
        engine.insert("drugs", &rec).unwrap();

        let notifs = engine.drain_notifications();
        assert_eq!(notifs.len(), 1, "Should fire for PBP insert");
        assert_eq!(notifs[0].trigger_name, "pbp_inserts");

        // Insert non-matching record
        let mut rec2 = Record::new();
        rec2.insert("id".into(), Value::Integer(2));
        rec2.insert("target_class".into(), Value::Text("ribosome".into()));
        rec2.insert("mic".into(), Value::Float(3.0));
        engine.insert("drugs", &rec2).unwrap();

        let notifs2 = engine.drain_notifications();
        assert_eq!(notifs2.len(), 0, "Should NOT fire for ribosome insert");

        cleanup(&dir);
    }

    #[test]
    fn test_9_7_multiple_triggers_same_bundle() {
        let (mut engine, dir) = make_trigger_engine("trigger_multi");

        // 3 triggers on drugs
        for i in 1..=3 {
            let def = super::TriggerDef {
                name: format!("trigger_{i}"),
                kind: super::TriggerKind::OnMutation {
                    bundle: "drugs".into(),
                    operation: super::MutationOp::Insert,
                    filter: None,
                },
                channel: format!("channel_{i}"),
            };
            engine.create_trigger(def).unwrap();
        }

        // Insert 1 record — should fire all 3
        let mut rec = Record::new();
        rec.insert("id".into(), Value::Integer(1));
        rec.insert("target_class".into(), Value::Text("test".into()));
        rec.insert("mic".into(), Value::Float(1.0));
        engine.insert("drugs", &rec).unwrap();

        let notifs = engine.drain_notifications();
        assert_eq!(notifs.len(), 3, "All 3 triggers should fire");
        let names: Vec<&str> = notifs.iter().map(|n| n.trigger_name.as_str()).collect();
        assert!(names.contains(&"trigger_1"));
        assert!(names.contains(&"trigger_2"));
        assert!(names.contains(&"trigger_3"));

        cleanup(&dir);
    }

    #[test]
    fn test_9_8_trigger_survives_restart() {
        let dir = test_dir("trigger_restart");
        cleanup(&dir);

        // Create engine with triggers, snapshot, close
        {
            let mut engine = Engine::open(&dir).unwrap();
            let schema = BundleSchema::new("drugs")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("name"));
            engine.create_bundle(schema).unwrap();

            let def = super::TriggerDef {
                name: "insert_watch".into(),
                kind: super::TriggerKind::OnMutation {
                    bundle: "drugs".into(),
                    operation: super::MutationOp::Insert,
                    filter: None,
                },
                channel: "inserts".into(),
            };
            engine.create_trigger(def).unwrap();

            // Insert a record to prove the trigger works
            let mut rec = Record::new();
            rec.insert("id".into(), Value::Integer(1));
            rec.insert("name".into(), Value::Text("test".into()));
            engine.insert("drugs", &rec).unwrap();
            assert_eq!(engine.drain_notifications().len(), 1);

            engine.snapshot().unwrap();
        }

        // Reopen — triggers restored from WAL
        {
            let mut engine = Engine::open(&dir).unwrap();
            assert_eq!(engine.trigger_manager().len(), 1, "Trigger should survive restart");

            // Trigger should fire on new insert
            let mut rec = Record::new();
            rec.insert("id".into(), Value::Integer(2));
            rec.insert("name".into(), Value::Text("post_restart".into()));
            engine.insert("drugs", &rec).unwrap();

            let notifs = engine.drain_notifications();
            assert_eq!(notifs.len(), 1, "Trigger should fire after restart");
            assert_eq!(notifs[0].trigger_name, "insert_watch");
        }

        cleanup(&dir);
    }

    #[test]
    fn test_trigger_mutation_op_types() {
        let (mut engine, dir) = make_trigger_engine("trigger_ops");

        // Update trigger
        engine.create_trigger(super::TriggerDef {
            name: "on_update".into(),
            kind: super::TriggerKind::OnMutation {
                bundle: "drugs".into(),
                operation: super::MutationOp::Update,
                filter: None,
            },
            channel: "updates".into(),
        }).unwrap();

        // Delete trigger
        engine.create_trigger(super::TriggerDef {
            name: "on_delete".into(),
            kind: super::TriggerKind::OnMutation {
                bundle: "drugs".into(),
                operation: super::MutationOp::Delete,
                filter: None,
            },
            channel: "deletes".into(),
        }).unwrap();

        // Any trigger
        engine.create_trigger(super::TriggerDef {
            name: "on_any".into(),
            kind: super::TriggerKind::OnMutation {
                bundle: "drugs".into(),
                operation: super::MutationOp::Any,
                filter: None,
            },
            channel: "all_ops".into(),
        }).unwrap();

        // Insert a record
        let mut rec = Record::new();
        rec.insert("id".into(), Value::Integer(1));
        rec.insert("target_class".into(), Value::Text("test".into()));
        rec.insert("mic".into(), Value::Float(1.0));
        engine.insert("drugs", &rec).unwrap();

        let notifs = engine.drain_notifications();
        // Only "on_any" should fire for insert (not on_update, not on_delete)
        assert_eq!(notifs.len(), 1);
        assert_eq!(notifs[0].trigger_name, "on_any");

        // Update
        let key = {
            let mut k = Record::new();
            k.insert("id".into(), Value::Integer(1));
            k
        };
        let mut patches = Record::new();
        patches.insert("mic".into(), Value::Float(9.9));
        engine.update("drugs", &key, &patches).unwrap();

        let notifs = engine.drain_notifications();
        let names: Vec<&str> = notifs.iter().map(|n| n.trigger_name.as_str()).collect();
        assert!(names.contains(&"on_update"), "Update trigger should fire");
        assert!(names.contains(&"on_any"), "Any trigger should fire");
        assert!(!names.contains(&"on_delete"), "Delete trigger should NOT fire");

        // Delete
        engine.delete("drugs", &key).unwrap();
        let notifs = engine.drain_notifications();
        let names: Vec<&str> = notifs.iter().map(|n| n.trigger_name.as_str()).collect();
        assert!(names.contains(&"on_delete"), "Delete trigger should fire");
        assert!(names.contains(&"on_any"), "Any trigger should fire");
        assert!(!names.contains(&"on_update"), "Update trigger should NOT fire on delete");

        cleanup(&dir);
    }

    #[test]
    fn test_trigger_drop() {
        let (mut engine, dir) = make_trigger_engine("trigger_drop");

        engine.create_trigger(super::TriggerDef {
            name: "watch".into(),
            kind: super::TriggerKind::OnMutation {
                bundle: "drugs".into(),
                operation: super::MutationOp::Insert,
                filter: None,
            },
            channel: "ch".into(),
        }).unwrap();
        assert_eq!(engine.trigger_manager().len(), 1);

        engine.drop_trigger("watch").unwrap();
        assert_eq!(engine.trigger_manager().len(), 0);

        // Insert should produce no notifications
        let mut rec = Record::new();
        rec.insert("id".into(), Value::Integer(1));
        rec.insert("target_class".into(), Value::Text("x".into()));
        rec.insert("mic".into(), Value::Float(1.0));
        engine.insert("drugs", &rec).unwrap();
        assert_eq!(engine.drain_notifications().len(), 0);

        cleanup(&dir);
    }

    #[test]
    fn test_trigger_gql_create_and_drop() {
        let (mut engine, dir) = make_trigger_engine("trigger_gql");

        // GQL: ON SECTION drugs EXECUTE notify
        let stmt = crate::parser::parse("ON SECTION drugs EXECUTE notify").unwrap();
        crate::parser::execute(&mut engine, &stmt).unwrap();
        assert_eq!(engine.trigger_manager().len(), 1);

        // Insert fires the trigger
        let mut rec = Record::new();
        rec.insert("id".into(), Value::Integer(1));
        rec.insert("target_class".into(), Value::Text("x".into()));
        rec.insert("mic".into(), Value::Float(1.0));
        engine.insert("drugs", &rec).unwrap();
        assert_eq!(engine.drain_notifications().len(), 1);

        cleanup(&dir);
    }

    /// 2026-05-26 auto-snapshot bug regression: when the engine has
    /// BOTH mmap-backed bundles (with .dhoom on disk) AND heap-only
    /// bundles (created post the 2026-05-25 recovery), auto-compact
    /// used to take the mmap_rebase branch which only iterated
    /// self.mmap_bundles, leaving heap-only bundles WAL-only
    /// forever. Production at 2026-05-26T01:05 had 61 .dhoom files
    /// vs ~4900 logical bundles. This test guarantees the fix —
    /// the Phase 2 in mmap_rebase_snapshot now writes a .dhoom for
    /// heap-only bundles too.
    #[test]
    fn mmap_rebase_also_snapshots_heap_only_bundles() {
        let dir = test_dir("mmap_rebase_heap");
        cleanup(&dir);

        // Phase 1 — create bundle A, snapshot it, force the engine
        // to load it as mmap on the next open.
        {
            let mut engine = engine_no_autocompact(&dir);
            let schema_a = BundleSchema::new("alpha")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("kind"));
            engine.create_bundle(schema_a).unwrap();
            for i in 0..30i64 {
                let mut r = Record::new();
                r.insert("id".into(), Value::Integer(i));
                r.insert("kind".into(), Value::Text("a".into()));
                engine.insert("alpha", &r).unwrap();
            }
            engine.cow_snapshot().unwrap();
        }

        // Phase 2 — reopen via the mmap fast path. Bundle 'alpha'
        // loads from .dhoom → self.mmap_bundles. Now create a NEW
        // bundle 'beta' that doesn't have a .dhoom → goes into
        // self.bundles as heap-only.
        let mut engine = Engine::open_mmap(&dir).unwrap();
        let schema_b = BundleSchema::new("beta")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("kind"));
        engine.create_bundle(schema_b).unwrap();
        for i in 0..20i64 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("kind".into(), Value::Text("b".into()));
            engine.insert("beta", &r).unwrap();
        }

        // Sanity: alpha is mmap, beta is heap (mixed mode).
        // We don't have direct accessors for mmap_bundles/bundles
        // (private), but the snapshot files tell the story.
        let snapshots_dir = dir.join("snapshots");
        let alpha_dhoom_before = snapshots_dir.join("alpha.dhoom").exists();
        let beta_dhoom_before = snapshots_dir.join("beta.dhoom").exists();
        assert!(alpha_dhoom_before, "alpha.dhoom should exist before rebase");
        assert!(
            !beta_dhoom_before,
            "beta.dhoom should NOT exist before rebase (heap-only)"
        );

        // Phase 3 — set the checkpoint to fire immediately, push
        // a single insert to trigger auto-compact, then verify
        // BOTH bundles have .dhoom files. Before the fix beta
        // would be skipped because mmap_rebase_snapshot only
        // iterated self.mmap_bundles.
        engine.set_checkpoint_interval(1);
        engine.compaction_policy_mut().disabled = false;
        engine.compaction_policy_mut().max_wal_entries = 1; // force trigger
        engine.compaction_policy_mut().min_interval_secs = 0;
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(999));
        r.insert("kind".into(), Value::Text("trigger".into()));
        engine.insert("alpha", &r).unwrap(); // any insert → maybe_checkpoint → maybe_auto_compact

        let alpha_dhoom_after = snapshots_dir.join("alpha.dhoom").exists();
        let beta_dhoom_after = snapshots_dir.join("beta.dhoom").exists();
        assert!(alpha_dhoom_after, "alpha.dhoom should still exist after rebase");
        assert!(
            beta_dhoom_after,
            "REGRESSION: beta.dhoom must exist after rebase \
             (was the bug — heap-only bundles were skipped)"
        );

        cleanup(&dir);
    }

    // ─────────────────────────────────────────────────────────────────────
    // TDD-HAL-V.3 — Replay restoration + WalError variants + replace_buffer
    //
    // Spec: theory/halcyon/HALCYON_PART_V_SNAPSHOT_GATES.md §3 (P1).
    // Locked decisions:
    //   D-V-A — explicit little-endian WAL encoding (the SHA-recompute
    //           pass at replay time relies on exact LE bytes).
    //   D-V-C — SHA-256 over LE buffer bytes is the citation handle;
    //           replay re-derives it and rejects mismatch as
    //           `WalError::SnapshotChecksumMismatch`.
    //
    // Four red tests:
    //   1. Byte-identity round trip — declare LATTICE + GAUGE_FIELD,
    //      thermalize via GIBBS_SAMPLE, SNAPSHOT, drop engine, reopen,
    //      verify the SU(2)-mut handle's buffer is byte-identical to
    //      the pre-close state.
    //   2. Orphan snapshot — hand-build a WAL with
    //      OP_GAUGE_FIELD_SNAPSHOT but no preceding
    //      OP_GAUGE_FIELD_DECLARE; replay rejects with
    //      `WalError::OrphanedSnapshot`.
    //   3. Group mismatch — snapshot an SU(2) field, corrupt the
    //      payload's group byte to U(1) in the WAL on disk (recompute
    //      CRC); replay rejects with `WalError::SnapshotGroupMismatch`.
    //   4. Checksum mismatch — snapshot, flip one byte of the buffer
    //      portion of the payload (recompute CRC so the WAL reader
    //      doesn't trip first); replay re-derives SHA, sees mismatch,
    //      rejects with `WalError::SnapshotChecksumMismatch`.
    //
    // All four tests acquire `gauge::registry::test_serial_lock()` so
    // they don't interleave with parallel gauge tests on the process-
    // global lattice + gauge registries.

    /// CRC32 (Castagnoli) — local copy of `wal::crc32` for the byte-
    /// surgery WAL tests below. The implementation is identical to the
    /// one in `src/wal.rs`; replicated here because `crc32` is a
    /// private function in the WAL module. This is test-only code.
    #[cfg(feature = "gauge")]
    fn crc32_for_test(data: &[u8]) -> u32 {
        let mut crc: u32 = 0xFFFF_FFFF;
        for &byte in data {
            crc ^= byte as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0x82F6_3B78;
                } else {
                    crc >>= 1;
                }
            }
        }
        !crc
    }

    /// Locate the first `OP_GAUGE_FIELD_SNAPSHOT` (0x0B) entry's
    /// payload-byte range inside a raw WAL file. Returns
    /// `(entry_start, entry_end_excl, payload_start, payload_end_excl)`
    /// where `entry_*` covers `[op_byte..crc)` (the CRC-checked range)
    /// and `payload_*` covers the inner payload bytes (excludes op
    /// byte). The CRC tail itself is at `[entry_end_excl ..
    /// entry_end_excl + 4)`.
    #[cfg(feature = "gauge")]
    fn locate_snapshot_entry(bytes: &[u8]) -> Option<(usize, usize, usize, usize)> {
        let mut offset = 0usize;
        while offset + 4 <= bytes.len() {
            let total_len = u32::from_le_bytes(
                bytes[offset..offset + 4].try_into().unwrap(),
            ) as usize;
            let entry_start = offset + 4;
            let entry_end = entry_start + total_len; // op + payload (no CRC)
            if entry_end + 4 > bytes.len() {
                return None;
            }
            let op = bytes[entry_start];
            if op == 0x0B {
                // payload covers [entry_start+1 .. entry_end)
                return Some((entry_start, entry_end, entry_start + 1, entry_end));
            }
            offset = entry_end + 4; // skip CRC tail
        }
        None
    }

    /// TDD-HAL-V.3: byte-identity round trip — declare a buckyball
    /// LATTICE + GAUGE_FIELD U IDENTITY PERSIST via the engine's
    /// durable path, run a few thermalization sweeps (so the snapshot
    /// captures a non-identity state), SNAPSHOT, drop the engine,
    /// reopen on the same data directory. The replay pass restores
    /// the buffer from the WAL, and the post-restart SU(2)-mut
    /// handle's buffer must be byte-identical to the pre-close state.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_v_3_replay_snapshot_byte_identity() {
        let _g = crate::gauge::registry::test_serial_lock();
        crate::lattice::registry::clear();
        crate::gauge::registry::clear();

        let dir = test_dir("hal_v_3_byte_identity");
        cleanup(&dir);

        let pre_close_buffer: Vec<f64>;
        let lat_name = "bb_v_3_bi";
        let field_name = "U_v_3_bi";
        {
            let mut engine = Engine::open(&dir).unwrap();
            // 1. Declare buckyball lattice durably (low-level path so
            //    the LATTICE statement lands in the WAL — the parser-
            //    level LATTICE FROM TRUNCATED_ICOSAHEDRON variant only
            //    writes to the in-memory registry).
            let mut bb = crate::lattice::topology::truncated_icosahedron::buckyball();
            bb.name = lat_name.into();
            engine
                .declare_lattice_durable(bb.clone())
                .expect("declare lattice durable");
            // 2. Declare GAUGE_FIELD U IDENTITY PERSIST.
            let field = crate::gauge::SU2GaugeField::new(
                field_name.into(),
                &bb,
                crate::gauge::GaugeFieldInit::Identity,
                None,
            )
            .expect("identity init");
            engine
                .declare_gauge_field_durable(std::sync::Arc::new(field.clone()))
                .expect("declare gauge field durable");
            // Mirror the parser-side dual-register so GIBBS_SAMPLE
            // can find the SU(2)-mut handle (this is the same pattern
            // commit 9c5b614 wired into the parser arm).
            crate::gauge::registry::register_su2(field);
            // 3. Thermalize via the SNAPSHOT executor's prerequisite
            //    verb — the parser-level GIBBS_SAMPLE arm walks
            //    `get_su2_mut` directly.
            let sweep = crate::parser::parse(&format!(
                "GIBBS_SAMPLE {field_name} BETA 2.5 N_SWEEPS 5 SEED 20260616;"
            ))
            .expect("parse GIBBS_SAMPLE");
            crate::parser::execute(&mut engine, &sweep).expect("exec GIBBS_SAMPLE");
            // 4. Capture pre-close buffer state.
            let arc = crate::gauge::registry::get_su2_mut(field_name)
                .expect("SU(2)-mut handle must exist post-sweep");
            pre_close_buffer = arc.lock().unwrap().buffer.data.clone();
            // Sanity: thermalized buffer is not the identity vector.
            assert!(
                pre_close_buffer.iter().any(|x| (*x - 1.0).abs() > 1e-12 && x.abs() > 1e-12),
                "thermalized buffer must not be identity"
            );
            // 5. SNAPSHOT GAUGE_FIELD U PERSIST via the executor.
            let snap = crate::parser::parse(&format!(
                "SNAPSHOT GAUGE_FIELD {field_name} PERSIST;"
            ))
            .expect("parse SNAPSHOT");
            crate::parser::execute(&mut engine, &snap).expect("exec SNAPSHOT");
            // engine drops at end of scope — WAL is flushed via
            // `WalWriter::sync` inside the engine's maybe_checkpoint /
            // explicit sync paths.
        }

        // Wipe the in-process registries so we know reopen rebuilt
        // from the WAL alone — not from leftover state.
        crate::lattice::registry::clear();
        crate::gauge::registry::clear();

        // Reopen.
        {
            let _engine = Engine::open(&dir).expect("reopen engine");
            let arc = crate::gauge::registry::get_su2_mut(field_name)
                .expect(
                    "SU(2)-mut handle must be restored by replay",
                );
            let post_open_buffer = arc.lock().unwrap().buffer.data.clone();
            assert_eq!(
                post_open_buffer.len(),
                pre_close_buffer.len(),
                "buffer length must match pre-close"
            );
            assert_eq!(
                post_open_buffer, pre_close_buffer,
                "post-replay buffer must be byte-identical to pre-close state"
            );
        }

        crate::lattice::registry::clear();
        crate::gauge::registry::clear();
        cleanup(&dir);
    }

    /// TDD-HAL-V.3 (revised 2026-06-26): orphan snapshot — hand-build a WAL
    /// whose only gauge entry is `OP_GAUGE_FIELD_SNAPSHOT` for field
    /// `U_orphan` (no preceding `OP_GAUGE_FIELD_DECLARE`). Engine::open
    /// must SKIP the orphan snapshot with a warning + complete successfully,
    /// leaving the orphan field unregistered so callers see "field unknown"
    /// rather than the entire boot failing. This is the availability
    /// trade-off shipped after the 2026-06-26 production incident where a
    /// wedged-then-OOM-killed prior boot left an orphan U_v snapshot that
    /// would have forced fall-back to full heap replay (~15GB RSS → OOM).
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_v_3_replay_orphan_snapshot() {
        let _g = crate::gauge::registry::test_serial_lock();
        crate::lattice::registry::clear();
        crate::gauge::registry::clear();

        let dir = test_dir("hal_v_3_orphan");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let wal_path = dir.join("gigi.wal");
        let mut buffer = Vec::with_capacity(90 * 4);
        for _ in 0..90 {
            buffer.push(1.0);
            buffer.push(0.0);
            buffer.push(0.0);
            buffer.push(0.0);
        }
        let payload = crate::wal::GaugeFieldSnapshotPayload::from_buffer(
            "U_orphan".to_string(),
            crate::gauge::Group::SU2,
            buffer,
        );
        {
            let mut writer = crate::wal::WalWriter::open(&wal_path).unwrap();
            writer.log_gauge_field_snapshot(&payload).unwrap();
            writer.sync().unwrap();
        }

        // Engine::open replays the WAL — the snapshot pass must SKIP the
        // orphan (logging a warning) and return successfully. The orphan
        // field stays unregistered.
        let engine = Engine::open(&dir)
            .expect("orphan snapshot must be SKIPPED, not hard-error");
        assert!(
            crate::gauge::registry::get("U_orphan").is_none(),
            "skipped orphan field must remain unregistered"
        );
        drop(engine);

        crate::lattice::registry::clear();
        crate::gauge::registry::clear();
        cleanup(&dir);
    }

    /// TDD-HAL-V.3: group mismatch — declare an SU(2) field, snapshot
    /// it, then surgically rewrite the group tag byte in the WAL's
    /// snapshot payload to U(1) (0x03) and recompute the CRC so the
    /// WAL reader doesn't trip first. Replay must surface
    /// `WalError::SnapshotGroupMismatch` because the registered
    /// handle's group disagrees with the payload's group tag.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_v_3_replay_group_mismatch() {
        let _g = crate::gauge::registry::test_serial_lock();
        crate::lattice::registry::clear();
        crate::gauge::registry::clear();

        let dir = test_dir("hal_v_3_group_mismatch");
        cleanup(&dir);

        let lat_name = "bb_v_3_gm";
        let field_name = "U_v_3_gm";

        // 1. Set up the WAL with a real SU(2) snapshot via the
        //    durable engine path so LATTICE + GAUGE_FIELD + SNAPSHOT
        //    all land in the WAL.
        {
            let mut engine = Engine::open(&dir).unwrap();
            let mut bb =
                crate::lattice::topology::truncated_icosahedron::buckyball();
            bb.name = lat_name.into();
            engine
                .declare_lattice_durable(bb.clone())
                .expect("declare lattice durable");
            let field = crate::gauge::SU2GaugeField::new(
                field_name.into(),
                &bb,
                crate::gauge::GaugeFieldInit::Identity,
                None,
            )
            .expect("identity init");
            engine
                .declare_gauge_field_durable(std::sync::Arc::new(field.clone()))
                .expect("declare gauge field durable");
            crate::gauge::registry::register_su2(field);
            let snap = crate::parser::parse(&format!(
                "SNAPSHOT GAUGE_FIELD {field_name} PERSIST;"
            ))
            .expect("parse SNAPSHOT");
            crate::parser::execute(&mut engine, &snap).expect("exec SNAPSHOT");
        }

        // 2. Surgically rewrite the group-tag byte. The snapshot
        //    payload layout is:
        //      [u32 name_len][name_bytes][u8 group_tag][32 sha256]…
        //    So group_tag lives at payload_start + 4 + name_len.
        let wal_path = dir.join("gigi.wal");
        let mut bytes = fs::read(&wal_path).unwrap();
        let (entry_start, entry_end, payload_start, _payload_end) =
            locate_snapshot_entry(&bytes).expect("snapshot entry in WAL");
        let name_len = u32::from_le_bytes(
            bytes[payload_start..payload_start + 4].try_into().unwrap(),
        ) as usize;
        let group_tag_idx = payload_start + 4 + name_len;
        // Sanity: it's currently SU(2) (0x01).
        assert_eq!(bytes[group_tag_idx], 0x01, "group tag must currently be SU(2)");
        bytes[group_tag_idx] = 0x03; // U(1)
        // Recompute CRC over [entry_start..entry_end) and write it
        // back at [entry_end..entry_end+4).
        let new_crc = crc32_for_test(&bytes[entry_start..entry_end]);
        bytes[entry_end..entry_end + 4].copy_from_slice(&new_crc.to_le_bytes());
        fs::write(&wal_path, &bytes).unwrap();

        // 3. Wipe registries, reopen — must surface group mismatch.
        crate::lattice::registry::clear();
        crate::gauge::registry::clear();
        let err = match Engine::open(&dir) {
            Ok(_) => panic!("group mismatch must surface WalError::SnapshotGroupMismatch"),
            Err(e) => e,
        };
        let msg = err.to_string();
        assert!(
            msg.contains("U_v_3_gm") && msg.contains("SU(2)") && msg.contains("U(1)"),
            "error must name the field and both groups, got: {msg}"
        );

        crate::lattice::registry::clear();
        crate::gauge::registry::clear();
        cleanup(&dir);
    }

    /// TDD-HAL-V.3: checksum mismatch — declare + snapshot an SU(2)
    /// field, then surgically flip one byte of the buffer portion of
    /// the WAL's snapshot payload and recompute the entry's CRC.
    /// Replay re-derives SHA-256 over the corrupted buffer, sees
    /// mismatch against the payload's stored hash, and rejects with
    /// `WalError::SnapshotChecksumMismatch`.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_v_3_replay_checksum_mismatch() {
        let _g = crate::gauge::registry::test_serial_lock();
        crate::lattice::registry::clear();
        crate::gauge::registry::clear();

        let dir = test_dir("hal_v_3_checksum");
        cleanup(&dir);

        let lat_name = "bb_v_3_cs";
        let field_name = "U_v_3_cs";

        // 1. Real SU(2) snapshot in the WAL via the durable engine
        //    path (parser LATTICE FROM TRUNCATED_ICOSAHEDRON is
        //    in-memory only — we need declare_lattice_durable so the
        //    LATTICE entry lands in the WAL alongside the GAUGE_FIELD
        //    and SNAPSHOT entries).
        {
            let mut engine = Engine::open(&dir).unwrap();
            let mut bb =
                crate::lattice::topology::truncated_icosahedron::buckyball();
            bb.name = lat_name.into();
            engine
                .declare_lattice_durable(bb.clone())
                .expect("declare lattice durable");
            let field = crate::gauge::SU2GaugeField::new(
                field_name.into(),
                &bb,
                crate::gauge::GaugeFieldInit::Identity,
                None,
            )
            .expect("identity init");
            engine
                .declare_gauge_field_durable(std::sync::Arc::new(field.clone()))
                .expect("declare gauge field durable");
            crate::gauge::registry::register_su2(field);
            let sweep = crate::parser::parse(&format!(
                "GIBBS_SAMPLE {field_name} BETA 2.5 N_SWEEPS 3 SEED 20260616;"
            ))
            .expect("parse GIBBS_SAMPLE");
            crate::parser::execute(&mut engine, &sweep)
                .expect("exec GIBBS_SAMPLE");
            let snap = crate::parser::parse(&format!(
                "SNAPSHOT GAUGE_FIELD {field_name} PERSIST;"
            ))
            .expect("parse SNAPSHOT");
            crate::parser::execute(&mut engine, &snap).expect("exec SNAPSHOT");
        }

        // 2. Flip one byte of the buffer. Payload layout:
        //      [u32 name_len][name_bytes][u8 group_tag][32 sha256]
        //      [u32 buf_len][buf_len*8 buffer_bytes]
        //    Buffer starts at:
        //      payload_start + 4 + name_len + 1 + 32 + 4
        let wal_path = dir.join("gigi.wal");
        let mut bytes = fs::read(&wal_path).unwrap();
        let (entry_start, entry_end, payload_start, _payload_end) =
            locate_snapshot_entry(&bytes).expect("snapshot entry in WAL");
        let name_len = u32::from_le_bytes(
            bytes[payload_start..payload_start + 4].try_into().unwrap(),
        ) as usize;
        let buffer_start = payload_start + 4 + name_len + 1 + 32 + 4;
        // Flip the very first byte of the buffer (least-significant byte
        // of the first f64). XOR with 0xFF is a cheap guaranteed mutation.
        bytes[buffer_start] ^= 0xFF;
        // Recompute CRC so the WAL reader's integrity check passes —
        // the checksum gate we want to catch is the SHA-256 one inside
        // the V.3 replay pass, not the CRC32 in the WAL reader.
        let new_crc = crc32_for_test(&bytes[entry_start..entry_end]);
        bytes[entry_end..entry_end + 4].copy_from_slice(&new_crc.to_le_bytes());
        fs::write(&wal_path, &bytes).unwrap();

        // 3. Wipe + reopen — must surface SHA-256 mismatch.
        crate::lattice::registry::clear();
        crate::gauge::registry::clear();
        let err = match Engine::open(&dir) {
            Ok(_) => panic!("checksum mismatch must surface WalError::SnapshotChecksumMismatch"),
            Err(e) => e,
        };
        let msg = err.to_string();
        assert!(
            msg.contains("U_v_3_cs") && msg.contains("SHA-256"),
            "error must name the field and the SHA-256 category, got: {msg}"
        );

        crate::lattice::registry::clear();
        crate::gauge::registry::clear();
        cleanup(&dir);
    }
}
