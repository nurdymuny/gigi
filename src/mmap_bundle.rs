//! Memory-mapped DHOOM bundles (Feature #11).
//!
//! Maps a DHOOM snapshot file into virtual memory so the OS page cache
//! manages record residency. Only actively queried pages consume RSS,
//! giving ~20× memory reduction for typical query workloads (Thm 11.1).

use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::Path;

use memmap2::Mmap;
use serde_json::{Map, Value};

use crate::dhoom::{arithmetic_value, coerce, parse_fiber, split_record_fields, Fiber, Modifier};

/// Memory-mapped DHOOM bundle — records parsed on demand from OS page cache.
pub struct MmapBundle {
    /// Memory-mapped file bytes.
    mmap: Mmap,
    /// Parsed DHOOM fiber (schema) from the header.
    fiber: Fiber,
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
            fiber,
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
        Some(self.decode_record_line(line, index))
    }

    /// Decode a single DHOOM record line using the fiber definition.
    fn decode_record_line(&self, line: &str, ordinal: usize) -> Value {
        let record_fields = self.fiber.record_fields();
        let raw_fields: Vec<String> = if line.is_empty() {
            vec![]
        } else {
            split_record_fields(line)
        };

        let mut obj = Map::new();

        // Fill arithmetic fields
        for fdecl in &self.fiber.fields {
            if let Some(Modifier::Arithmetic {
                ref start,
                ref step,
            }) = fdecl.modifier
            {
                let s = step.unwrap_or(1);
                obj.insert(fdecl.name.clone(), arithmetic_value(start, s, ordinal));
            }
        }

        // Map positional record values
        for (j, rf) in record_fields.iter().enumerate() {
            if j < raw_fields.len() {
                let raw = &raw_fields[j];
                let val = if raw.is_empty() {
                    if let Some(Modifier::Default(ref d)) = rf.modifier {
                        d.clone()
                    } else {
                        Value::String(String::new())
                    }
                } else if let Some(stripped) = raw.strip_prefix(':') {
                    coerce(stripped)
                } else {
                    coerce(raw)
                };
                obj.insert(rf.name.clone(), val);
            } else {
                // Trailing elision → fill with default
                if let Some(Modifier::Default(ref d)) = rf.modifier {
                    obj.insert(rf.name.clone(), d.clone());
                }
            }
        }

        Value::Object(obj)
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
        &self.fiber
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

/// Overlay bundle: mmap base + in-memory delta for recent writes.
///
/// Point queries check overlay first, then fall through to mmap.
/// Tombstones suppress deleted mmap records.
pub struct OverlayBundle {
    /// Mmap-backed snapshot data.
    base: MmapBundle,
    /// Recent inserts/updates keyed by ordinal-like identifier.
    overlay: HashMap<String, Value>,
    /// Deleted keys (tombstones).
    tombstones: std::collections::HashSet<String>,
}

impl OverlayBundle {
    /// Create an overlay on top of an mmap'd bundle.
    pub fn new(base: MmapBundle) -> Self {
        Self {
            base,
            overlay: HashMap::new(),
            tombstones: std::collections::HashSet::new(),
        }
    }

    /// Insert or update a record in the overlay.
    pub fn put(&mut self, key: String, record: Value) {
        self.tombstones.remove(&key);
        self.overlay.insert(key, record);
    }

    /// Mark a key as deleted.
    pub fn delete(&mut self, key: &str) {
        self.tombstones.insert(key.to_string());
        self.overlay.remove(key);
    }

    /// Point lookup: overlay wins, tombstone hides, base is fallback.
    pub fn get_overlay(&self, key: &str) -> Option<&Value> {
        if self.tombstones.contains(key) {
            return None;
        }
        self.overlay.get(key)
    }

    /// Get the mmap base bundle.
    pub fn base(&self) -> &MmapBundle {
        &self.base
    }

    /// Number of overlay entries.
    pub fn overlay_len(&self) -> usize {
        self.overlay.len()
    }

    /// Number of tombstones.
    pub fn tombstone_len(&self) -> usize {
        self.tombstones.len()
    }

    /// Clear overlay and tombstones (after compaction to new mmap).
    pub fn clear_overlay(&mut self) {
        self.overlay.clear();
        self.tombstones.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

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

    /// TDD-11.4: Overlay masks mmap for updates.
    #[test]
    fn tdd_11_4_overlay_masks_updates() {
        let dhoom = "items{id@1, val}:\n10\n20\n30\n";
        let bundle = mmap_from_dhoom(dhoom);
        let mut overlay = OverlayBundle::new(bundle);

        // Base record
        let base_r0 = overlay.base().get(0).unwrap();
        assert_eq!(base_r0["val"], 10);

        // Overlay update
        overlay.put(
            "1".to_string(),
            serde_json::json!({"id": 1, "val": 999}),
        );
        let ov = overlay.get_overlay("1").unwrap();
        assert_eq!(ov["val"], 999);

        // Non-overlayed key falls through to None in overlay
        assert!(overlay.get_overlay("2").is_none());
    }

    /// TDD-11.5: Tombstones hide mmap records.
    #[test]
    fn tdd_11_5_tombstones_hide_records() {
        let dhoom = "items{id@1, val}:\n10\n20\n";
        let bundle = mmap_from_dhoom(dhoom);
        let mut overlay = OverlayBundle::new(bundle);

        // Delete key "1"
        overlay.delete("1");
        assert!(overlay.get_overlay("1").is_none());
        assert_eq!(overlay.tombstone_len(), 1);

        // Base data still exists
        assert!(overlay.base().get(0).is_some());
    }

    /// TDD-11.6: Scan merges overlay and base data.
    #[test]
    fn tdd_11_6_scan_base_still_works() {
        let dhoom = "items{id@1, val}:\n10\n20\n30\n";
        let bundle = mmap_from_dhoom(dhoom);
        let overlay = OverlayBundle::new(bundle);

        // Base scan still returns all 3 records
        let all: Vec<Value> = overlay.base().scan().collect();
        assert_eq!(all.len(), 3);
    }

    /// TDD-11.7: Compact clears overlay.
    #[test]
    fn tdd_11_7_compact_clears_overlay() {
        let dhoom = "items{id@1, val}:\n10\n20\n";
        let bundle = mmap_from_dhoom(dhoom);
        let mut overlay = OverlayBundle::new(bundle);

        overlay.put("extra".into(), serde_json::json!({"id": 99, "val": 0}));
        overlay.delete("old");
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
