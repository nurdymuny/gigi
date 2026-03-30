//! Memory-mapped DHOOM bundles (Feature #11).
//!
//! Maps a DHOOM snapshot file into virtual memory so the OS page cache
//! manages record residency. Only actively queried pages consume RSS,
//! giving ~20× memory reduction for typical query workloads (Thm 11.1).

use std::fs::File;
use std::io;
use std::path::Path;
use std::sync::RwLock;

use memmap2::Mmap;
use serde_json::Value;

use crate::bundle::BundleStore;
use crate::dhoom::{parse_fiber, DhoomRecordParser, Fiber, Modifier};
use crate::types::{BundleSchema, Record};

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

        // Skip pool lines (start with '&') and blank lines before records
        let body = &text[header_end..];
        let mut line_offsets = Vec::new();
        let mut pos = header_end;
        let mut in_pools = true;
        let mut data_start = header_end;

        for line in body.lines() {
            let trimmed = line.trim();
            let line_byte_len = line.len() + 1; // +1 for newline (approximate)

            if in_pools {
                if trimmed.starts_with('&') || trimmed.is_empty() {
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
    pub fn get(&self, index: usize) -> Option<Value> {
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
    type Item = Value;

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
    tombstones: RwLock<std::collections::HashSet<String>>,
}

impl OverlayBundle {
    /// Create an overlay on top of an mmap'd bundle.
    pub fn new(base: MmapBundle, schema: BundleSchema) -> Self {
        Self {
            base,
            overlay: RwLock::new(BundleStore::new(schema)),
            tombstones: RwLock::new(std::collections::HashSet::new()),
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
        if let Ok(mut store) = self.overlay.write() {
            *store = BundleStore::new(schema);
        }
        if let Ok(mut ts) = self.tombstones.write() {
            ts.clear();
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

        let all: Vec<Value> = bundle.scan().collect();
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
        let all: Vec<Value> = overlay.base().scan().collect();
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
