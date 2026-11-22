# GIGI v2 Feature Specification

## Eleven Features for the Next-Generation Fiber Bundle Database

**Authors:** Davis Geometric Intelligence  
**Date:** 2025-01  
**Revised:** 2026-03 (MIRADOR team review)  
**Status:** Draft v2  
**Audience:** Engine developers, MIRADOR/PRISM integrators  

---

> **Revision Note (v2):** Incorporates field feedback from the MIRADOR integration
> team. Key changes: reordered build priorities around production-fire risks,
> downscoped MVCC to pragmatic CoW snapshots, replaced full materialized views
> with TTL query cache, deferred three features (#5 Incremental COMPLETE,
> #7 Temporal Queries, #8 Computed Fields) based on real-world usage analysis,
> and added Feature 11 (Memory-Mapped Bundles) which was identified as the
> single biggest infrastructure win for reducing resident memory.

---

## Priority Tiers

| Tier | Features | Rationale |
|------|----------|-----------|
| **Tier 1 — Stop the bleeding** | #2 Streaming DHOOM → #1 Auto-Compaction → #3 CoW Snapshots | Production OOM, health-check kills, 13GB WAL |
| **Tier 2 — Performance** | #4 Hash Indexes → #10 Query Cost Planner → #11 Memory-Mapped Bundles | 11M-record scans, 32GB RAM requirement |
| **Tier 3 — Features** | #6 TTL Query Cache → #9 Pub/Sub Triggers | Analytical query latency, real-time notifications |
| **Deferred** | #5 Incremental COMPLETE, #7 Temporal Queries, #8 Computed Fields | See deferral rationale in each section |

**Critical dependency:** #2 (Streaming DHOOM) **blocks** #1 (Auto-Compaction).
Auto-compaction is useless if the snapshot itself OOMs. Fix the encoder first.

---

## Table of Contents

### Tier 1 — Stop the Bleeding
1. [Auto-Compaction](#1-auto-compaction)
2. [Streaming DHOOM Encoder](#2-streaming-dhoom-encoder) *(build first — blocks #1)*
3. [Read-Write Separation (CoW Snapshots)](#3-read-write-separation-cow-snapshots)

### Tier 2 — Performance
4. [Secondary Indexes](#4-secondary-indexes)
10. [Query Cost Planner](#10-query-cost-planner)
11. [Memory-Mapped Bundles](#11-memory-mapped-bundles) *(new)*

### Tier 3 — Features
6. [Query Cache with TTL](#6-query-cache-with-ttl)
9. [Pub/Sub with Sheaf Triggers](#9-pubsub-with-sheaf-triggers)

### Deferred
5. [Incremental COMPLETE](#5-incremental-complete) *(deferred)*
7. [Temporal Queries](#7-temporal-queries) *(deferred)*
8. [Computed Fields](#8-computed-fields) *(deferred)*

Each feature section contains:
- **Motivation** — why the feature matters (grounded in MIRADOR/PRISM usage)
- **Mathematical Foundation** — rigorous definitions and theorems
- **Design** — API surface, data structures, algorithms
- **Implementation Notes** — integration with existing engine code
- **Math-Based TDD** — test cases derived from mathematical invariants
- **MIRADOR Review Notes** — feedback from the integration team (where applicable)

---

## Notation Conventions

| Symbol | Meaning |
|--------|---------|
| $E = (B, F, \pi)$ | Fiber bundle: base space $B$, fiber $F$, projection $\pi$ |
| $s : B \to E$ | Section (a record maps base point to fiber values) |
| $\sigma(b)$ | The fiber value at base point $b$ |
| $K$ | Sectional curvature of base space |
| $\nabla$ | Connection on the bundle (parallel transport) |
| $\Delta$ | Laplacian on the neighborhood graph |
| $H^1$ | First cohomology (obstruction to global triviality) |
| $\mathcal{W}$ | WAL entry sequence |
| $\mathcal{S}_t$ | Engine state at logical time $t$ |

---

## 1. Auto-Compaction

### Motivation

GIGI's WAL grows without bound until an explicit `SNAPSHOT` command is issued. In
production (Fly.io, 11M records), the WAL reached 13 GB and caused OOM on restart.
Auto-compaction applies a policy $\pi_{\text{compact}}$ that triggers compaction
automatically, keeping WAL size bounded.

> **Dependency:** Requires Feature #2 (Streaming DHOOM) first. Auto-compaction triggers
> `snapshot()`, and if the snapshot itself OOMs on large bundles, auto-compaction
> makes the problem worse, not better.

### Mathematical Foundation

**Definition 1.1 (WAL Entropy).** Let $\mathcal{W} = (w_1, w_2, \ldots, w_N)$ be the
WAL entry sequence. Each entry $w_i$ is one of $\{I, U, D, C, S\}$ (Insert, Update,
Delete, Checkpoint, Schema). Define the *effective cardinality*

$$
N_{\mathrm{eff}} = |\{b \in B : \exists\, w_i = I(b) \text{ and } \nexists\, w_j = D(b),\, j > i\}|
$$

as the number of live records. The *WAL amplification factor* is

$$
A = \frac{|\mathcal{W}|}{N_{\mathrm{eff}}}
$$

When $A > \alpha_{\text{thresh}}$, the WAL contains significant dead weight (superseded
updates, deleted records). Compaction drives $A \to 1$.

**Theorem 1.1 (Bounded WAL Growth).** Under an auto-compaction policy that triggers
when $|\mathcal{W}| \geq \alpha_{\text{thresh}} \cdot N_{\mathrm{eff}} + \beta$, the
WAL size is bounded:

$$
|\mathcal{W}| \leq \alpha_{\text{thresh}} \cdot N_{\mathrm{eff}} + \beta + R_{\text{batch}}
$$

where $R_{\text{batch}}$ is the maximum batch size between compaction checks.

*Proof.* Compaction rewrites $\mathcal{W}$ to exactly $N_{\mathrm{eff}} + |\text{schemas}|$
entries. Between compaction and the next trigger, at most
$\alpha_{\text{thresh}} \cdot N_{\mathrm{eff}} + \beta$ additional entries accumulate.
Adding the maximum batch overshoot $R_{\text{batch}}$ gives the bound. $\square$

**Definition 1.2 (Compaction Policy).** A compaction policy
$\pi : (\mathbb{N}, \mathbb{N}, \mathbb{R}) \to \{0,1\}$ takes
$(|\mathcal{W}|, N_{\mathrm{eff}}, t_{\text{last}})$ and returns 1 iff compaction
should fire. The default policy is:

$$
\pi_{\text{default}}(|\mathcal{W}|, N_{\mathrm{eff}}, t_{\text{last}}, S_{\text{wal}}) = 
\begin{cases}
1 & \text{if } A > \alpha \text{ and } t - t_{\text{last}} > \delta_{\min} \\
1 & \text{if } |\mathcal{W}| > W_{\max} \\
1 & \text{if } S_{\text{wal}} > S_{\max} \\
0 & \text{otherwise}
\end{cases}
$$

with defaults $\alpha = 3$, $\delta_{\min} = 300\text{s}$, $W_{\max} = 10^7$,
$S_{\max} = 2\text{ GB}$.

> **MIRADOR Review:** Don't just trigger on `ops_since_checkpoint` — also
> trigger on **WAL file size**. A pathological workload of tiny ops could hit
> 500K quickly at low memory cost, while a bulk ingest of fat records could
> OOM before hitting the count threshold. The $S_{\max}$ (file size) trigger
> catches the latter case.

### Design

**New `Engine` fields:**
```rust
/// Auto-compaction configuration.
pub struct CompactionPolicy {
    /// WAL amplification threshold (default: 3.0).
    pub amplification_threshold: f64,
    /// Minimum seconds between compactions (default: 300).
    pub min_interval_secs: u64,
    /// Absolute WAL entry limit (default: 10_000_000).
    pub max_wal_entries: u64,
    /// WAL file size limit in bytes (default: 2 * 1024^3 = 2 GB).
    pub max_wal_bytes: u64,
    /// Use DHOOM snapshot (true) or WAL compact (false). Default: true.
    pub use_snapshot: bool,
    /// Disabled flag. Default: false.
    pub disabled: bool,
}
```

```rust
// Added to Engine struct:
compaction_policy: CompactionPolicy,
last_compaction: std::time::Instant,
wal_entry_count: u64,
wal_byte_count: u64,
```

**New method:**
```rust
impl Engine {
    fn maybe_auto_compact(&mut self) -> io::Result<()> {
        if self.compaction_policy.disabled { return Ok(()); }
        self.wal_entry_count += 1;
        let n_eff = self.total_records() as f64;
        let a = self.wal_entry_count as f64 / n_eff.max(1.0);
        let elapsed = self.last_compaction.elapsed().as_secs();
        
        let should_compact = 
            (a > self.compaction_policy.amplification_threshold 
             && elapsed > self.compaction_policy.min_interval_secs)
            || self.wal_entry_count > self.compaction_policy.max_wal_entries
            || self.wal_byte_count > self.compaction_policy.max_wal_bytes;
        
        if should_compact {
            if self.compaction_policy.use_snapshot {
                self.snapshot()?;
            } else {
                self.compact()?;
            }
            self.wal_entry_count = self.schemas.len() as u64 + 1; // schema + checkpoint
            self.last_compaction = std::time::Instant::now();
        }
        Ok(())
    }
}
```

**GQL Surface:**
```sql
SET COMPACTION AMPLIFICATION 5.0;
SET COMPACTION INTERVAL 600;
SET COMPACTION MAX_ENTRIES 20000000;
SET COMPACTION MAX_BYTES 4294967296;  -- 4 GB
SET COMPACTION OFF;
SET COMPACTION ON;
```

### Implementation Notes

- Hook `maybe_auto_compact()` into `maybe_checkpoint()` — checks run every checkpoint.
- `wal_entry_count` is recovered during replay by counting entries in `do_replay()`.
- `wal_byte_count` is initialized from `fs::metadata(wal_path)?.len()` on open and
  incremented per WAL write (entry size is known at write time).
- Compaction runs in the same thread as the write path; for the HTTP server, the
  write lock is already held, so no extra synchronization needed.
- For the streaming server, run compaction in `spawn_blocking` to avoid starving
  the Tokio runtime (same pattern as WAL replay).

### Math-Based TDD

```
Test 1.1 — Amplification trigger:
  Given: N_eff = 100, WAL entries = 500 (A = 5.0 > 3.0), elapsed > 300s
  Assert: auto_compact fires
  Assert: post-compaction A ≈ 1.0

Test 1.2 — Cooldown prevents rapid re-compaction:
  Given: A > 3.0 but elapsed < 300s
  Assert: auto_compact does NOT fire

Test 1.3 — Absolute limit overrides cooldown:
  Given: WAL entries = 10_000_001, elapsed = 0
  Assert: auto_compact fires regardless of cooldown

Test 1.4 — Disabled policy:
  Given: policy.disabled = true, A = 100.0
  Assert: auto_compact never fires

Test 1.5 — Post-compaction WAL invariant:
  Given: engine with 1000 records, run snapshot()
  Assert: WAL entry count = |schemas| + 1 (Checkpoint)
  Assert: A = (|schemas| + 1) / 1000 < 1.0

Test 1.6 — Amplification monotone under inserts:
  Given: fresh engine, A_0 = 0
  Insert N records without compaction
  Assert: A_i = (i + |schemas|) / i → monotonically decreasing toward 1.0

Test 1.7 — Amplification increases under updates:
  Given: engine with 100 records, A ≈ 1.0
  Execute 200 updates to same records
  Assert: A ≈ 3.0 (300 WAL entries / 100 records)
  Assert: auto_compact triggers on next check

Test 1.8 — WAL file size trigger:
  Given: WAL file size = 2.1 GB (> 2 GB default), A = 1.5 (below amplification threshold)
  Assert: auto_compact fires (file size overrides amplification check)

Test 1.9 — Fat-record pathology:
  Given: 100 inserts of 20 MB records each (WAL = 2 GB), entry count = 100
  Assert: file size trigger fires at ~100 entries, NOT waiting for 10M entries
  (This is the pathological case the MIRADOR team flagged)
```

---

## 2. Streaming DHOOM Encoder

> **BUILD THIS FIRST.** This feature blocks #1 (Auto-Compaction). Auto-compaction
> triggers `snapshot()`, and the current `snapshot()` OOMs on large bundles.
> Without a streaming encoder, enabling auto-compaction would cause periodic
> OOM crashes instead of preventing them.

### Motivation

The current `snapshot()` collects **all** records into `Vec<serde_json::Value>` before
encoding. For the chembl bundle (11M records), this materializes ~30 GB in memory,
causing OOM. A streaming encoder processes records in constant memory.

### Mathematical Foundation

**Definition 2.1 (DHOOM Encoding as a Map).** DHOOM encoding is a map
$\phi : \text{Record}^* \to \Sigma^*$ from a sequence of records to a byte string.
The current implementation requires the full sequence in memory:

$$
\phi(\{r_1, \ldots, r_N\}) = \text{encode\_json}([r_1, \ldots, r_N])
$$

**Definition 2.2 (Streaming Encoding).** A streaming encoder decomposes $\phi$ into
a monoid homomorphism. Define the *chunk encoder* for chunk size $C$:

$$
\phi_{\text{stream}}(\{r_1, \ldots, r_N\}) = 
\phi_{\text{header}}(N) \;\|\; 
\bigoplus_{k=0}^{\lceil N/C \rceil - 1} \phi_{\text{chunk}}(r_{kC+1}, \ldots, r_{\min((k+1)C, N)})
$$

where $\|$ is concatenation and $\oplus$ is the chunk-join operator.

**Theorem 2.1 (Constant Memory Streaming).** The streaming encoder uses memory

$$
M = O(C \cdot |r|_{\max})
$$

independent of $N$, where $|r|_{\max}$ is the maximum record size.

*Proof.* At any point, only one chunk of $C$ records is materialized. The header
is $O(1)$. Each chunk is encoded and flushed before the next is loaded. $\square$

**Definition 2.3 (Chunk Boundary Alignment).** For DHOOM's dictionary-based columnar
encoding, each chunk computes column dictionaries independently. Cross-chunk
deduplication is not required for correctness but improves compression. The *global
dictionary* variant maintains a running dictionary $D_k = D_{k-1} \cup D_{\text{new}}$
with memory $O(|D|)$ where $|D|$ is the number of distinct field values.

### Design

**New API:**
```rust
/// Streaming DHOOM encoder — writes to any `io::Write` sink.
pub struct StreamingDhoomEncoder<W: io::Write> {
    writer: W,
    collection: String,
    chunk_size: usize,
    buffer: Vec<serde_json::Value>,
    records_written: usize,
}

impl<W: io::Write> StreamingDhoomEncoder<W> {
    pub fn new(writer: W, collection: &str, chunk_size: usize) -> Self;
    
    /// Feed one record. Flushes when buffer reaches chunk_size.
    pub fn push(&mut self, record: serde_json::Value) -> io::Result<()>;
    
    /// Finalize — flush remaining buffer, write footer.
    pub fn finish(self) -> io::Result<usize>;
}
```

**Modified `snapshot()`:**
```rust
pub fn snapshot(&mut self) -> io::Result<usize> {
    // ...
    for (name, store) in &self.bundles {
        let snap_path = snapshots_dir.join(format!("{name}.dhoom"));
        let tmp_path = snapshots_dir.join(format!("{name}.dhoom.tmp"));
        let count = store.len();
        if count == 0 { continue; }

        let file = fs::File::create(&tmp_path)?;
        let buf = io::BufWriter::new(file);
        let mut encoder = StreamingDhoomEncoder::new(buf, name, 50_000);
        
        for rec in store.records() {
            encoder.push(record_to_serde_json(&rec))?;
        }
        encoder.finish()?;
        fs::rename(&tmp_path, &snap_path)?;
    }
    // ... compact WAL ...
}
```

### Implementation Notes

- `BundleStore::records()` already returns an iterator — no code change needed.
- The DHOOM format header declares record count; with streaming, we either
  (a) write count in footer, (b) pre-compute via `store.len()`, or (c) use
  a seekable writer to backpatch. Option (b) is simplest — count is already known.
- Chunk size of 50K records keeps memory under ~200 MB for typical records.
- For decode, the existing `decode_to_json` works on the concatenated output
  as long as chunk boundaries use the DHOOM collection separator `---`.

### Math-Based TDD

```
Test 2.1 — Roundtrip fidelity:
  Given: bundle B with N records {r_1, ..., r_N}
  Encode with streaming encoder (chunk_size = 100)
  Decode result
  Assert: decoded == {r_1, ..., r_N} (exact field-by-field equality)

Test 2.2 — Memory bound:
  Given: N = 1_000_000 records, chunk_size C = 10_000
  Track peak allocation during encode
  Assert: peak_memory < C * max_record_size * 3  (buffer + encode + write)
  Assert: peak_memory is O(C), NOT O(N)

Test 2.3 — Chunk boundary correctness:
  Given: N = 250, chunk_size = 100
  Assert: 3 chunks emitted (100 + 100 + 50)
  Assert: decode produces exactly 250 records in original order

Test 2.4 — Empty bundle:
  Given: bundle with 0 records
  Assert: streaming encoder produces valid empty DHOOM (header only)

Test 2.5 — Single record:
  Given: 1 record, chunk_size = 1000
  Assert: 1 chunk emitted, roundtrip correct

Test 2.6 — Equivalence with batch encoder:
  Given: bundle B with N records
  batch_output = encode_json(all_records, name)
  stream_output = StreamingDhoomEncoder(chunk_size=N).push_all().finish()
  Assert: decode(batch_output) == decode(stream_output)
  (Note: byte-level equality not required due to dictionary ordering)

Test 2.7 — Idempotent snapshot:
  Given: engine with data, run snapshot() twice
  Assert: second snapshot produces identical DHOOM output
  Assert: WAL contains only schema entries after each
```

---

## 3. Read-Write Separation (CoW Snapshots)

### Motivation

GIGI wraps `Engine` in `RwLock<Engine>`, meaning every write (insert, update, snapshot)
blocks all reads. The snapshot operation holding a write lock caused Fly.io health
checks to timeout, which made Fly think the machine was dead — nearly losing the
volume. This wasn't an inconvenience; it was a data loss risk.

> **MIRADOR Review:** Full MVCC is the correct long-term answer but it's a major
> rewrite. Pragmatic middle ground: **copy-on-write snapshots**. Fork the in-memory
> state to a background thread, snapshot from the copy. 80% of the benefit, 10%
> of the MVCC complexity. Full MVCC is a Phase 2 stretch goal.

### Mathematical Foundation

**Definition 3.1 (Version Space).** Define the *version space* as a totally ordered set
$(\mathcal{V}, \leq)$ where each write transaction produces a new version $v \in \mathcal{V}$.
The engine state at version $v$ is $\mathcal{S}_v$.

**Definition 3.2 (Snapshot Isolation).** A reader starting at version $v$ sees
$\mathcal{S}_v$ throughout its execution, regardless of concurrent writes advancing
the version to $v' > v$. Formally, for read transaction $T_r$ starting at version $v$:

$$
\forall\, \text{read}(b) \in T_r: \quad \text{result}(b) = \sigma_v(b)
$$

even if a write $T_w$ commits $\sigma_{v'}(b) \neq \sigma_v(b)$ during $T_r$'s execution.

**Theorem 3.1 (Serializability of Disjoint Writes).** If write transactions $T_1, T_2$
modify disjoint base points $B_1 \cap B_2 = \emptyset$, then the committed state is
independent of serialization order:

$$
\mathcal{S}_{T_1 \circ T_2} = \mathcal{S}_{T_2 \circ T_1}
$$

*Proof.* Each $T_i$ only modifies fibers over $B_i$. Since $B_1 \cap B_2 = \emptyset$,
the patch sets commute. $\square$

**Definition 3.3 (Epoch-Based Reclamation).** Old versions are garbage-collected when
no active reader references them. Define the *minimum active epoch*:

$$
e_{\min} = \min_{T_r \in \text{active}} v(T_r)
$$

All versions $v < e_{\min}$ can be reclaimed.

**Theorem 3.2 (Memory Bound).** Let $W$ be the write rate (records/sec) and $L$ be the
maximum read transaction duration. The MVCC overhead is bounded by:

$$
M_{\text{MVCC}} \leq W \cdot L \cdot |r|_{\max}
$$

*Proof.* In the worst case, a reader holds open a snapshot for $L$ seconds during which
$W \cdot L$ records are written. Each old version of a modified record must be retained
until the reader completes. $\square$

### Design

**Phase 1: CoW Snapshots (ship first).**

Keep `Arc<RwLock<Engine>>`. For long-running operations (snapshot, COMPLETE, large
queries), clone the needed state under a brief read lock, then release the lock
and operate on the clone.

```rust
impl Engine {
    /// Clone bundle data for background snapshot — holds read lock only
    /// long enough to Arc::clone the bundle stores.
    pub fn clone_for_snapshot(&self) -> HashMap<String, BundleStoreSnapshot> {
        // This takes the read lock for O(bundles) time (microseconds),
        // NOT O(records) time.
        self.bundles.iter().map(|(name, store)| {
            (name.clone(), store.cow_snapshot())
        }).collect()
    }
}

impl BundleStore {
    /// Cheap CoW snapshot: Arc-clone the storage vecs/maps.
    /// No deep copy — shared until mutation.
    pub fn cow_snapshot(&self) -> BundleStoreSnapshot {
        BundleStoreSnapshot {
            schema: self.schema.clone(),
            storage: self.storage.arc_clone(),  // Arc<Vec> or Arc<HashMap>
            field_index: Arc::clone(&self.field_index_arc),
            len: self.len(),
        }
    }
}
```

**Snapshot from clone (background thread):**
```rust
// In gigi_stream.rs:
async fn handle_snapshot(state: Arc<StreamState>) -> impl IntoResponse {
    let cloned = {
        let engine = state.engine.read().unwrap();
        engine.clone_for_snapshot()
    };
    // Write lock released. Reads/writes proceed normally.
    
    tokio::task::spawn_blocking(move || {
        // Encode DHOOM from cloned data — no lock held.
        for (name, snap) in &cloned {
            let encoder = StreamingDhoomEncoder::new(file, name, 50_000);
            for rec in snap.records() {
                encoder.push(record_to_serde_json(&rec))?;
            }
            encoder.finish()?;
        }
        // Brief write lock only for WAL compaction:
        let mut engine = state.engine.write().unwrap();
        engine.compact_wal_to_schemas()?;
    }).await
}
```

**Phase 2: Full MVCC (stretch goal).**

Replace `RwLock<Engine>` with epoch-based MVCC using persistent data structures:

```rust
pub struct MvccEngine {
    /// Current mutable state — only the writer touches this.
    current: Engine,
    /// Epoch counter, incremented on each write batch.
    epoch: AtomicU64,
    /// Immutable snapshots held by active readers.
    /// Map from epoch → Arc<EngineSnapshot>.
    snapshots: DashMap<u64, Arc<EngineSnapshot>>,
    /// Reader epoch tracker for GC.
    active_readers: DashMap<u64, u64>, // reader_id → epoch
}

/// A read-only, point-in-time view of the engine.
pub struct EngineSnapshot {
    epoch: u64,
    bundles: HashMap<String, BundleSnapshot>,
}

/// Read-only bundle view using CoW (copy-on-write) sections.
pub struct BundleSnapshot {
    schema: Arc<BundleSchema>,
    sections: im::HashMap<BasePoint, Arc<Vec<Value>>>,
    field_index: Arc<HashMap<String, HashMap<Value, RoaringBitmap>>>,
}
```

**Reader path:**
```rust
impl MvccEngine {
    /// Acquire a consistent read snapshot.
    pub fn read_snapshot(&self) -> ReadGuard {
        let epoch = self.epoch.load(Ordering::Acquire);
        let reader_id = generate_reader_id();
        self.active_readers.insert(reader_id, epoch);
        ReadGuard { engine: self, reader_id, epoch, snapshot: self.get_or_create_snapshot(epoch) }
    }
}

impl Drop for ReadGuard {
    fn drop(&mut self) {
        self.engine.active_readers.remove(&self.reader_id);
        self.engine.gc_old_snapshots();
    }
}
```

**Writer path:**
```rust
impl MvccEngine {
    pub fn insert(&mut self, bundle_name: &str, record: &Record) -> io::Result<()> {
        self.current.insert(bundle_name, record)?;
        self.epoch.fetch_add(1, Ordering::Release);
        Ok(())
    }
    
    pub fn batch_insert(&mut self, bundle_name: &str, records: &[Record]) -> io::Result<usize> {
        let n = self.current.batch_insert(bundle_name, records)?;
        self.epoch.fetch_add(1, Ordering::Release);
        Ok(n)
    }
}
```

### Implementation Notes

- **Phase 1 (CoW Snapshots — ship this):** Keep `RwLock<Engine>`. Wrap bundle
  storage internals in `Arc` so `cow_snapshot()` is O(1) per bundle (just
  Arc::clone). The snapshot thread operates on shared-but-immutable data while
  the write path continues. Write lock is only needed briefly for WAL compaction
  at the end. This approach already fixes the health-check kill that nearly lost
  the Fly.io volume.
- **Phase 2 (Full MVCC — stretch goal):** Introduce `im::HashMap` (persistent
  data structure) for `BundleStore::sections`. Writers clone-on-write; readers
  hold `Arc` refs. This eliminates the RwLock entirely for reads.
- WAL writes remain serialized (single writer) — this is correct since WAL is
  append-only and already serialized.
- `gigi_stream.rs` changes for Phase 1: snapshot handler clones state, releases
  lock, encodes in background. Read endpoints keep using `read().unwrap()` —
  they're fast enough not to contend.

> **MIRADOR Review:** Phase 1 is ~200 lines of code. Phase 2 is a major rewrite
> touching every BundleStore method. Ship Phase 1, measure contention in production,
> only build Phase 2 if reads are actually blocked by writes at user-noticeable
> latency.

### Math-Based TDD

```
Test 3.1 — Snapshot isolation:
  Given: engine with record r(b=1, x=10)
  Reader R starts, sees x=10
  Writer W updates r(b=1, x=20)
  Assert: R still sees x=10
  Assert: new reader R' sees x=20

Test 3.2 — Monotonic epochs:
  Given: sequence of writes w_1, w_2, ..., w_n
  Assert: epoch(w_i) < epoch(w_{i+1}) for all i

Test 3.3 — Disjoint write commutativity:
  Given: T_1 writes to b=1, T_2 writes to b=2
  Execute in order T_1, T_2 → state S_a
  Execute in order T_2, T_1 → state S_b
  Assert: S_a == S_b (Theorem 3.1)

Test 3.4 — Epoch-based GC:
  Given: snapshot at epoch 5, all readers finished
  Assert: snapshot at epoch 5 is reclaimed
  Assert: snapshots at epoch > 5 retained if readers active

Test 3.5 — Memory bound under concurrent load:
  Given: writer inserting W=1000 rec/s, reader holds snapshot for L=10s
  Assert: MVCC overhead ≤ W * L * max_record_size = 10000 * max_record_size

Test 3.6 — Writer serialization:
  Given: concurrent writes from multiple HTTP handlers
  Assert: all WAL entries are sequentially ordered (no interleaving)
  Assert: replay(WAL) produces identical state

Test 3.7 — Read-during-snapshot (CoW):
  Given: snapshot() running on cloned state in background thread
  Assert: read() returns immediately with current state
  Assert: write() succeeds (no lock contention from snapshot)
  Assert: health endpoint returns 200 throughout

Test 3.8 — CoW snapshot correctness:
  Given: engine with records {r_1, ..., r_N}
  Clone state for snapshot
  Insert r_{N+1} while snapshot runs
  Assert: snapshot contains exactly {r_1, ..., r_N} (not r_{N+1})
  Assert: engine contains {r_1, ..., r_{N+1}} after snapshot completes
```

---

## 4. Secondary Indexes

### Motivation

GIGI has RoaringBitmap field indexes (`field_index` in `BundleStore`) but only uses
them in `range_query()`. The `filtered_query()` and `filtered_query_ex()` methods
do full table scans, checking `QueryCondition::matches()` per record. For MIRADOR
queries like `WHERE drug_name = 'rifampin'`, an 11M-record scan is
$O(N)$ when it could be $O(|result|)$ with index lookup.

> **MIRADOR Review:** Be specific about the implementation. MIRADOR's queries are
> almost all exact-match lookups (`WHERE drug_name = "rifampin"`). Ship a **hash
> index first** (O(1) exact match, trivial to implement using the existing
> RoaringBitmap infrastructure), B-tree later for range queries. Days of work,
> not weeks.

### Mathematical Foundation

**Definition 4.1 (Bitmap Index).** For field $f$ in bundle $B$, the bitmap index
$\mathcal{I}_f$ is a map:

$$
\mathcal{I}_f : \text{Dom}(f) \to 2^{B}
$$

where $\mathcal{I}_f(v)$ is the set of base points $b$ where $\sigma(b)[f] = v$,
represented as a RoaringBitmap for $O(1)$ set operations.

**Definition 4.2 (Index Intersection).** For a conjunction of equality predicates
$f_1 = v_1 \land f_2 = v_2 \land \cdots \land f_k = v_k$, the result set is:

$$
R = \mathcal{I}_{f_1}(v_1) \cap \mathcal{I}_{f_2}(v_2) \cap \cdots \cap \mathcal{I}_{f_k}(v_k)
$$

This is computed in $O(k \cdot |R|)$ via RoaringBitmap AND operations, vs. $O(N)$ for a scan.

**Theorem 4.1 (Selectivity-Ordered Intersection).** If predicates are ordered by
selectivity (smallest bitmap first), the intersection cost is:

$$
T = O\left(\sum_{i=1}^{k} \min\left(|R_{i-1}|, |\mathcal{I}_{f_i}(v_i)|\right)\right)
$$

where $R_0 = \mathcal{I}_{f_1}(v_1)$ and $R_i = R_{i-1} \cap \mathcal{I}_{f_i}(v_i)$.

*Proof.* RoaringBitmap AND of sets $A, B$ runs in $O(\min(|A|, |B|))$. By processing
the smallest set first, each subsequent intersection operates on a set no larger than
the running result. $\square$

**Definition 4.3 (Range Index via Sorted Bitmap Merge — Phase 2).** For a range
predicate $a \leq f \leq b$, the result is the union of bitmaps for all indexed
values in range:

$$
R = \bigcup_{v \in \text{Dom}(f),\, a \leq v \leq b} \mathcal{I}_f(v)
$$

For numeric fields with high cardinality, this degenerates. A *bucketed range index*
partitions $\text{Dom}(f)$ into $\sqrt{N}$ equal-width buckets. Each bucket bitmap
covers all records in that range. Range queries touch at most $O(\sqrt{N})$ bitmaps.

> **Deferred to Phase 2.** The existing hash map index handles Eq, In, IsNull which
> covers ~95% of MIRADOR's query patterns. Range predicates (Lt, Gt, Between) remain
> full-scan with index-narrowed candidate sets from conjunction with equality predicates.

### Design

**Modified `filtered_query()`:**
```rust
impl BundleStore {
    pub fn filtered_query_ex(
        &self,
        conditions: &[QueryCondition],
        or_conditions: Option<&[Vec<QueryCondition>]>,
        select: Option<&[String]>,
        limit: Option<usize>,
        offset: usize,
        order_by: Option<&str>,
        order_desc: bool,
    ) -> Vec<Record> {
        // Phase 1: Extract indexable predicates from AND conditions
        let (indexed, residual) = self.partition_conditions(conditions);
        
        // Phase 2: Bitmap intersection for indexed predicates
        let candidate_bps = if indexed.is_empty() {
            None // full scan
        } else {
            Some(self.intersect_bitmaps(&indexed))
        };
        
        // Phase 3: Fetch records for candidate base points
        let iter: Box<dyn Iterator<Item = Record>> = match candidate_bps {
            Some(bps) => Box::new(bps.iter().filter_map(|bp| self.get_full_record(bp as u64))),
            None => Box::new(self.records()),
        };
        
        // Phase 4: Apply residual predicates (non-indexed)
        let results: Vec<Record> = iter
            .filter(|r| residual.iter().all(|c| c.matches(r)))
            .filter(|r| matches_or_filter(r, or_conditions))
            .collect();
        
        // Phase 5: Order, offset, limit, project
        // ... existing logic ...
    }
    
    fn partition_conditions(&self, conds: &[QueryCondition]) 
        -> (Vec<&QueryCondition>, Vec<&QueryCondition>) 
    {
        let mut indexed = Vec::new();
        let mut residual = Vec::new();
        for c in conds {
            if self.can_use_index(c) {
                indexed.push(c);
            } else {
                residual.push(c);
            }
        }
        // Sort indexed by estimated selectivity (smallest bitmap first)
        indexed.sort_by_key(|c| self.estimate_selectivity(c));
        (indexed, residual)
    }
    
    fn intersect_bitmaps(&self, conditions: &[&QueryCondition]) -> RoaringBitmap {
        let mut result: Option<RoaringBitmap> = None;
        for cond in conditions {
            let bm = self.condition_bitmap(cond);
            result = Some(match result {
                None => bm,
                Some(r) => r & bm,
            });
        }
        result.unwrap_or_default()
    }
}
```

**GQL Surface:**
```sql
-- Existing (schema-level):
CREATE BUNDLE drugs SCHEMA (name TEXT INDEX, organism TEXT INDEX, mic NUMERIC);

-- New (post-hoc):
ADD INDEX ON drugs(target_class);
DROP INDEX ON drugs(target_class);
EXPLAIN QUERY ON drugs WHERE organism = 'S. aureus';  -- shows index usage
```

### Implementation Notes

- `field_index` already exists and is maintained on insert/update/delete.
- The key change is in `filtered_query_ex()`: check indexes *before* iterating.
- `can_use_index(cond)` returns true for `Eq`, `In`, `IsNull` on indexed fields.
  Range predicates (Lt, Gt, Between) are *not* index-accelerated in Phase 1 —
  they become residual predicates applied after index narrowing.
- `estimate_selectivity(cond)` returns bitmap cardinality (or N for non-indexed).
- `get_full_record(bp)` reconstructs a Record from base values + fiber values.
- `ADD INDEX` requires a new GQL statement and WAL entry for persistence.

> **MIRADOR Review:** This is days of work, not weeks. The RoaringBitmap
> infrastructure already exists. The only new code is (a) plumbing
> `filtered_query_ex()` to check indexes, and (b) the `ADD INDEX` GQL statement.

### Math-Based TDD

```
Test 4.1 — Equality index acceleration:
  Given: bundle with N=1M records, field organism with 100 distinct values
  Index on organism
  Query: WHERE organism = 'S. aureus'  (~10K records)
  Assert: result set identical to full scan
  Assert: records_examined ≤ |result| (no false positives from index)

Test 4.2 — Conjunction intersection:
  Given: N=1M, indexed fields {organism, target_class}
  Query: WHERE organism = 'S. aureus' AND target_class = 'PBP'
  Let I_1 = I_organism('S. aureus'), I_2 = I_target('PBP')
  Assert: result == records at base points (I_1 ∩ I_2)
  Assert: |examined| = |I_1 ∩ I_2| (no extra scan)

Test 4.3 — Selectivity ordering:
  Given: I_organism has 10K entries, I_target has 500 entries
  Assert: optimizer processes I_target first (smaller set)
  Assert: intersection cost ≤ |I_target| + |I_target ∩ I_organism|

Test 4.4 — Mixed indexed + residual:
  Given: index on organism, NO index on mic
  Query: WHERE organism = 'S. aureus' AND mic < 4.0
  Assert: index narrows to I_organism, then full scan of I_organism for mic < 4.0
  Assert: correct results

Test 4.5 — Index maintenance on insert:
  Given: index on field f, insert record with f=v
  Assert: I_f(v) now contains new base point
  Assert: query WHERE f=v returns new record

Test 4.6 — Index maintenance on delete:
  Given: index on f, record at bp with f=v
  Delete record at bp
  Assert: I_f(v) no longer contains bp

Test 4.7 — Index maintenance on update:
  Given: index on f, record at bp with f=v_old
  Update f to v_new
  Assert: I_f(v_old) no longer contains bp
  Assert: I_f(v_new) contains bp

Test 4.8 — IN predicate uses index:
  Given: index on organism
  Query: WHERE organism IN ['S. aureus', 'E. coli']
  Assert: result == I_organism('S. aureus') ∪ I_organism('E. coli')
```

---

## 5. Incremental COMPLETE *(Deferred)*

> **Deferral Rationale (MIRADOR Review):** MIRADOR's sheaves are small (40-cell
> matrices). The expensive part of COMPLETE isn't recomputation — it's the full
> scan to find which records are affected. The dirty-set tracking adds complexity
> to every write path (insert/update/delete). Unless there's a consumer doing
> COMPLETE on million-row bundles, this is premature optimization. **Feature #4
> (Secondary Indexes) solves the actual bottleneck** (finding records for the
> neighborhood) without touching the completion algorithm.
>
> Revisit when: a consumer needs COMPLETE on bundles with >100K records AND
> sub-second latency requirements.

### Motivation *(retained for future reference)*

GIGI's sheaf `complete()` recomputes the entire Laplacian and Schur complement system
from scratch for every call. With 11M records and dense adjacency structures, this is
$O(N^2)$ for neighborhood construction and $O(N_b^3)$ for per-neighborhood solves
(where $N_b$ is the neighborhood size). Incremental completion tracks *dirty* neighborhoods
and only recomputes affected regions when new data arrives.

### Mathematical Foundation

**Definition 5.1 (Dirty Set).** Let $\mathcal{D} \subseteq B$ be the set of base points
whose fibers have changed since the last COMPLETE. For a new insertion at $b$, the dirty
set is the 1-ring neighborhood:

$$
\mathcal{D}(b) = \{b\} \cup \{b' \in B : (b, b') \in E_{\text{adj}}\}
$$

where $E_{\text{adj}}$ is the adjacency edge set determined by the bundle's adjacency
definitions.

**Theorem 5.1 (Local Sufficiency).** If a record changes at base point $b$, only
neighborhoods $\mathcal{N}(b')$ for $b' \in \mathcal{D}(b)$ need recomputation. All
other neighborhoods produce identical completions.

*Proof.* The sheaf completion at $b'$ depends only on the Laplacian restricted to
$\mathcal{N}(b')$. If no record in $\mathcal{N}(b')$ has changed, the restricted
Laplacian $\Delta|_{\mathcal{N}(b')}$ is unchanged, so the Schur complement solution
is unchanged. For $b' \in \mathcal{D}(b)$, the neighborhood $\mathcal{N}(b')$ contains
$b$ (by definition of 1-ring), so $\Delta|_{\mathcal{N}(b')}$ may differ. $\square$

**Definition 5.2 (Incremental Laplacian Update).** When a single row $b$ changes value
from $\sigma_{\text{old}}(b)$ to $\sigma_{\text{new}}(b)$, the Laplacian update is a
rank-1 correction:

$$
\Delta' = \Delta + \sum_{b' \sim b} \delta w_{bb'} \cdot (e_b - e_{b'})(e_b - e_{b'})^T
$$

where $\delta w_{bb'}$ is the change in edge weight due to the new fiber value.
The Schur complement can be updated via the Woodbury identity:

$$
(\Delta' + U V^T)^{-1} = \Delta'^{-1} - \Delta'^{-1} U (I + V^T \Delta'^{-1} U)^{-1} V^T \Delta'^{-1}
$$

This reduces per-neighborhood update cost from $O(N_b^3)$ to $O(N_b^2)$ for rank-1
updates (or $O(N_b^2 \cdot k)$ for rank-$k$ batch updates).

**Definition 5.3 (H¹ Sensitivity).** The change in cohomology obstruction $H^1$ at
$b$ is bounded by:

$$
|\delta H^1(b)| \leq \|\Delta^{-1}\|_2 \cdot \|\delta\sigma(b)\|_2 \cdot \max_{b' \sim b} w_{bb'}
$$

If $|\delta H^1(b)| < \epsilon$, the completion values change negligibly and
recomputation can be skipped.

### Design

**New `BundleStore` fields:**
```rust
/// Dirty set: base points needing sheaf recomputation.
dirty_set: HashSet<BasePoint>,

/// Cached completion results per base point.
completion_cache: HashMap<BasePoint, CompletionEntry>,

struct CompletionEntry {
    /// Completed fiber values.
    values: Vec<(String, f64)>,
    /// Confidence per field.
    confidence: Vec<f64>,
    /// Epoch when computed.
    epoch: u64,
    /// Cached local Laplacian (for Woodbury updates).
    laplacian: Option<Vec<Vec<f64>>>,
}
```

**Modified operations:**
```rust
impl BundleStore {
    pub fn insert(&mut self, record: &Record) -> BasePoint {
        let bp = /* ... existing insert ... */;
        self.mark_dirty(bp);
        bp
    }
    
    pub fn update(&mut self, key: &Record, patches: &Record) -> bool {
        let updated = /* ... existing update ... */;
        if updated {
            let bp = self.compute_base_point(key);
            self.mark_dirty(bp);
        }
        updated
    }
    
    fn mark_dirty(&mut self, bp: BasePoint) {
        // Mark bp and its 1-ring neighbors
        self.dirty_set.insert(bp);
        if let Some(neighbors) = self.find_neighbors_cached(bp) {
            for n in neighbors {
                self.dirty_set.insert(n.bp);
            }
        }
    }
}
```

**New incremental COMPLETE:**
```rust
pub fn complete_incremental(
    store: &mut BundleStore,
    fields: &[String],
) -> Vec<Record> {
    let dirty: Vec<BasePoint> = store.dirty_set.drain().collect();
    let mut results = Vec::new();
    
    for bp in &dirty {
        let neighborhood = find_neighbors(store, *bp);
        // Build local Laplacian for this neighborhood
        let laplacian = build_local_laplacian(store, &neighborhood);
        
        for field in fields {
            if record_has_value(store, *bp, field) {
                continue; // Already measured
            }
            // Solve: Laplacian * x = rhs (Schur complement)
            let (value, confidence) = solve_local_schur(
                &laplacian, &neighborhood, field, store
            );
            results.push(make_completion_record(*bp, field, value, confidence));
        }
        
        // Cache result
        store.completion_cache.insert(*bp, CompletionEntry { /* ... */ });
    }
    
    results
}
```

**GQL Surface:**
```sql
-- Full recompute (existing):
COMPLETE drugs ON [mic, auc24] USING [organism, target];

-- Incremental (new — only dirty neighborhoods):
COMPLETE INCREMENTAL drugs ON [mic, auc24] USING [organism, target];

-- Force full recompute:
COMPLETE FULL drugs ON [mic, auc24] USING [organism, target];
```

### Implementation Notes

- Dirty tracking adds $O(1)$ per write (insert/update/delete) and $O(d)$ per
  dirty-mark where $d$ is the average vertex degree.
- `completion_cache` is invalidated on compaction / snapshot → rebuild from scratch
  on next COMPLETE after restart.
- For the first COMPLETE after bulk load, there is no cache → falls through to
  full recompute (equivalent to current behavior).
- Woodbury rank-1 updates are optional (Phase 2); Phase 1 just re-solves dirty
  neighborhoods from scratch, which is still $O(|\mathcal{D}| \cdot N_b^3)$
  instead of $O(N \cdot N_b^3)$.

### Math-Based TDD

```
Test 5.1 — Dirty set is 1-ring closed:
  Given: bundle with known adjacency graph
  Insert record at base point b
  Assert: dirty_set == {b} ∪ {b' : (b,b') ∈ E_adj}

Test 5.2 — Incremental equals full (single insert):
  Given: bundle with 100 records, run COMPLETE → result_full
  Insert 1 record at b_new
  Run COMPLETE INCREMENTAL → result_incr
  Run COMPLETE FULL → result_full2
  Assert: for all b ∈ dirty_set: result_incr[b] == result_full2[b]

Test 5.3 — Non-dirty neighborhoods unchanged:
  Given: bundle with cached completions at {b_1, ..., b_100}
  Insert record at b_new, dirty_set = {b_new, b_5, b_12}
  Run COMPLETE INCREMENTAL
  Assert: completion_cache[b_1] unchanged (same epoch)
  Assert: completion_cache[b_5] updated (new epoch)

Test 5.4 — Batch dirty accumulation:
  Given: empty dirty_set
  Insert N records at b_1, ..., b_N
  Assert: dirty_set = ∪_i D(b_i)  (union of all 1-rings)

Test 5.5 — H¹ sensitivity bound:
  Given: completion at b with H¹ = h₀
  Perturb fiber value by δ at neighbor b'
  Run incremental COMPLETE
  Assert: |H¹_new - h₀| ≤ ||Δ⁻¹|| * ||δ|| * max_weight  (Defn 5.3)

Test 5.6 — Confidence monotonicity:
  Given: neighborhood with N_eff measured values
  Add one more measurement
  Assert: confidence_new ≥ confidence_old
  (Because N_eff/(N_eff+1) is monotone increasing)

Test 5.7 — Cache invalidation on snapshot:
  Given: populated completion_cache
  Run snapshot() + reload engine
  Assert: dirty_set == all base points (full recompute needed)

Test 5.8 — Empty dirty set is no-op:
  Given: dirty_set is empty
  Run COMPLETE INCREMENTAL
  Assert: returns empty result (no work done)
  Assert: completion_cache unchanged
```

---

## 6. Query Cache with TTL

> **MIRADOR Review:** The original spec proposed full materialized views with automatic
> invalidation on insert for arbitrary GQL queries. That's essentially the
> view-maintenance problem, which is research-grade hard. Simpler version that gets
> 90% of the value: **cached query results with TTL + explicit invalidation**.
> Much easier, actually shippable.

### Motivation

MIRADOR repeatedly runs the same aggregation queries (mean MIC by organism, total
AUC24 by drug class) across 11M records. Each query scans the entire bundle. A query
cache stores results with a time-to-live, returning cached results for identical
queries within the TTL window. Explicit invalidation handles writes.

### Mathematical Foundation

**Definition 6.1 (Query Fingerprint).** A query $Q$ is identified by its canonical
fingerprint $\mathcal{F}(Q) \in \Sigma^*$, a deterministic hash of the normalized
query AST (bundle name, conditions, aggregations, group-by, ordering). Two queries
$Q_1, Q_2$ are cache-equivalent iff $\mathcal{F}(Q_1) = \mathcal{F}(Q_2)$.

**Definition 6.2 (Cache Entry).** A cache entry is a tuple
$(f, R, t_{\text{created}}, \tau)$ where $f$ is the fingerprint, $R$ is the
result set, $t_{\text{created}}$ is the wall-clock creation time, and $\tau$ is
the TTL in seconds. The entry is *valid* at time $t$ iff:

$$
t - t_{\text{created}} < \tau
$$

**Theorem 6.1 (Cache Staleness Bound).** With TTL $\tau$ and write rate $W$ (records/sec),
the maximum number of writes not reflected in a cached result is:

$$
\Delta_{\max} = W \cdot \tau
$$

For MIRADOR's typical workload ($W \approx 10$ rec/s during bulk ingest, $\tau = 60$s),
at most 600 records may be stale. During quiescent periods ($W = 0$), staleness is zero.

*Proof.* The worst case is a cache entry created at $t_0$ and read at $t_0 + \tau - \epsilon$.
In that interval, $W \cdot \tau$ writes may have occurred. $\square$

**Definition 6.3 (Invalidation Modes).**

**(a) Time-based:** Entry expires when $t > t_{\text{created}} + \tau$.

**(b) Write-count:** Entry expires after $\Delta_w$ writes to the source bundle.
More precisely: track a per-bundle *generation counter* $g_b$ incremented on each
write. Cache entry stores $g_{\text{created}}$. Valid iff $g_b - g_{\text{created}} < \Delta_w$.

**(c) Explicit:** `INVALIDATE CACHE` or `INVALIDATE CACHE ON <bundle>` clears entries.

**Theorem 6.2 (Cache Hit Cost).** A cache hit returns in $O(1)$ (fingerprint lookup +
validity check). A cache miss incurs the full query cost $C_Q$ plus $O(|R|)$ for
storing the result. The amortized cost for a query repeated $k$ times within TTL is:

$$
C_{\text{amortized}} = \frac{C_Q + k \cdot O(1)}{k} = \frac{C_Q}{k} + O(1) \xrightarrow{k \to \infty} O(1)
$$

### Design

**Data structures:**
```rust
pub struct QueryCache {
    entries: HashMap<u64, CacheEntry>,  // fingerprint → entry
    /// Per-bundle generation counters (incremented on write).
    generations: HashMap<String, u64>,
    /// Maximum cache size (entries). LRU eviction when full.
    max_entries: usize,
    /// Default TTL for new entries (seconds).
    default_ttl_secs: u64,
}

pub struct CacheEntry {
    fingerprint: u64,
    bundle_name: String,
    result: Vec<Record>,
    created_at: std::time::Instant,
    generation_at_creation: u64,
    ttl_secs: u64,
}

impl QueryCache {
    /// Look up a cached result. Returns None if miss or expired.
    pub fn get(&self, fingerprint: u64) -> Option<&[Record]> {
        let entry = self.entries.get(&fingerprint)?;
        if entry.created_at.elapsed().as_secs() >= entry.ttl_secs {
            return None;  // TTL expired
        }
        let current_gen = self.generations.get(&entry.bundle_name).copied().unwrap_or(0);
        if current_gen != entry.generation_at_creation {
            return None;  // Bundle has been written to since cache
        }
        Some(&entry.result)
    }
    
    /// Insert a query result into the cache.
    pub fn put(&mut self, fingerprint: u64, bundle: &str, result: Vec<Record>, ttl: u64);
    
    /// Invalidate all entries for a bundle.
    pub fn invalidate_bundle(&mut self, bundle: &str);
    
    /// Invalidate everything.
    pub fn invalidate_all(&mut self);
    
    /// Called on every write to bump the generation counter.
    pub fn on_write(&mut self, bundle: &str) {
        *self.generations.entry(bundle.to_string()).or_default() += 1;
    }
}
```

**GQL Surface:**
```sql
-- Query with explicit cache TTL:
CACHE 60 QUERY drugs AGGREGATE avg(mic) GROUP BY organism;

-- Invalidate cache for a bundle:
INVALIDATE CACHE ON drugs;

-- Invalidate all caches:
INVALIDATE CACHE;

-- Disable caching for a query:
NOCACHE QUERY drugs WHERE organism = 'S. aureus';
```

**Engine integration:**
```rust
impl Engine {
    pub fn insert(&mut self, bundle_name: &str, record: &Record) -> io::Result<()> {
        self.wal.log_insert(bundle_name, record)?;
        if let Some(store) = self.bundles.get_mut(bundle_name) {
            store.insert(record);
        }
        // Bump generation counter — invalidates stale cache entries on next read
        self.query_cache.on_write(bundle_name);
        self.maybe_checkpoint()?;
        Ok(())
    }
}
```

### Implementation Notes

- `QueryCache` lives in `Engine` (not persisted — purely in-memory).
- Generation counters are cheap: one `u64` increment per write, one comparison per
  cache read. No WAL entry needed.
- Cache is bounded by `max_entries` (default: 1000). LRU eviction when full.
- The fingerprint is computed by hashing the normalized query AST (after parsing,
  before execution). This means `WHERE a = 1 AND b = 2` and `WHERE b = 2 AND a = 1`
  produce the same fingerprint.
- Cache is *not* persisted across restarts — this is intentional. On restart, the
  first query populates the cache. No stale-on-boot risk.
- For the streaming server, the cache is behind the write lock (same as Engine).
  Reads that hit the cache release the read lock faster, improving overall throughput.

### Math-Based TDD

```
Test 6.1 — Cache hit returns correct result:
  Given: query Q returns result R, cached with TTL=60s
  Re-execute Q within 60s
  Assert: returns R from cache (no bundle scan)

Test 6.2 — Cache miss on TTL expiry:
  Given: cached result with TTL=1s
  Wait 2s, re-execute query
  Assert: cache miss, full scan executed, new result cached

Test 6.3 — Write invalidation via generation counter:
  Given: cached result for query on bundle B
  Insert 1 record into B (generation bumps)
  Re-execute query
  Assert: cache miss (generation mismatch), fresh result returned

Test 6.4 — Explicit invalidation:
  Given: 5 cached queries on bundle B, 3 on bundle C
  INVALIDATE CACHE ON B
  Assert: all 5 B entries gone, 3 C entries retained

Test 6.5 — INVALIDATE CACHE clears everything:
  Given: cached entries on bundles B, C, D
  INVALIDATE CACHE
  Assert: all entries cleared

Test 6.6 — Fingerprint equivalence:
  Given: Q1 = "WHERE a = 1 AND b = 2", Q2 = "WHERE b = 2 AND a = 1"
  Assert: fingerprint(Q1) == fingerprint(Q2)
  Assert: Q2 returns cached result from Q1

Test 6.7 — LRU eviction:
  Given: max_entries = 3, insert queries Q1, Q2, Q3, Q4
  Assert: Q1 evicted (oldest), Q2-Q4 retained

Test 6.8 — Staleness bound:
  Given: TTL = τ, write rate W
  Cache query, then insert W*τ records within TTL window
  Assert: cached result is at most W*τ records stale
  Assert: after TTL expires, fresh query returns all records
```

---

## 7. Temporal Queries *(Deferred)*

> **MIRADOR Review — Deferred.**
> The WAL has *operation* ordering, not *domain-time* ordering. MIRADOR's time
> dimension is *application-level* time (when was the patient's barrier state
> measured, when was the MIC assayed). `AS OF <timestamp>` on the WAL gives you
> "what did the database look like at write-time T" — not "what was the patient's
> barrier state at clinical-time T". These are fundamentally different questions.
>
> Shipping this without the distinction would create a foot-gun: users would
> assume temporal queries answer domain-time questions when they only answer
> write-time questions. This needs a proper application-level temporal model
> (explicit `valid_time` fields, bitemporal design) before it's useful.
>
> **Revisit after**: Phase 2 features are stable and MIRADOR has a concrete
> temporal use case with defined `valid_time` semantics.

### Motivation

MIRADOR tracks drug sensitivity data over time — MIC values evolve as new clinical
trials report results, and knowing *when* a value was observed is as important as
the value itself. Currently, GIGI stores only latest state. Temporal queries expose
the WAL's implicit time dimension, allowing `AS OF` and `BETWEEN` queries over
the full history.

### Mathematical Foundation

**Definition 7.1 (Temporal Bundle).** Extend the fiber bundle $E = (B, F, \pi)$ to a
temporal bundle $E_T = (B \times T, F, \pi_T)$ where $T = (\mathbb{Z}^+, \leq)$ is
discrete logical time. Each WAL entry $w_t$ produces a state transition
$\mathcal{S}_{t-1} \to \mathcal{S}_t$.

**Definition 7.2 (Temporal Section).** The temporal section at base point $b$ is the
time series of fiber values:

$$
\sigma_T(b) = \{(t, \sigma_t(b)) : t \in T,\, \sigma_t(b) \text{ differs from } \sigma_{t-1}(b)\}
$$

This is a *change log* — only transitions are stored, not repeated values.

**Theorem 7.1 (Point-in-Time Reconstruction).** The state at any time $t$ is
reconstructable from the initial state and the WAL prefix:

$$
\mathcal{S}_t = \text{replay}(\mathcal{W}[1:t])
$$

For arbitrary $t$, this is $O(t)$. With periodic *temporal checkpoints* at interval
$\Delta T$, reconstruction cost is:

$$
O(N_{\mathrm{eff}} + (t \bmod \Delta T))
$$

Loading the nearest checkpoint $\lfloor t / \Delta T \rfloor$ then replaying the suffix.

**Definition 7.3 (Temporal Window Query).** For a time range $[t_1, t_2]$ and base
point $b$:

$$
\text{HISTORY}(b, t_1, t_2) = \{(t, \sigma_t(b)) : t_1 \leq t \leq t_2,\, (t, \sigma_t(b)) \in \sigma_T(b)\}
$$

This returns all state changes for $b$ within the window.

**Theorem 7.2 (Temporal Aggregation).** For a decomposable aggregate $\alpha$ over a
temporal window $[t_1, t_2]$:

$$
\alpha_{[t_1,t_2]}(B) = \alpha\left(\bigcup_{b \in B} \{(b, \sigma_t(b)) : t_1 \leq t \leq t_2\}\right)
$$

This can be computed incrementally by maintaining aggregate state per WAL entry and
providing a *subtract* operation for the sliding window.

### Design

**WAL extension — timestamp every entry:**
```rust
pub struct WalEntry {
    pub timestamp: u64,  // microseconds since epoch
    pub sequence: u64,   // monotonic sequence number
    pub payload: WalPayload,
}

pub enum WalPayload {
    CreateBundle(BundleSchema),
    Insert { bundle_name: String, record: Record },
    Update { bundle_name: String, key: Record, patches: Record },
    Delete { bundle_name: String, key: Record },
    // ... existing variants ...
}
```

**Temporal index (in-memory):**
```rust
/// Maps (bundle, base_point) → sorted list of (timestamp, WAL_offset).
pub struct TemporalIndex {
    index: HashMap<(String, BasePoint), Vec<(u64, u64)>>,
}

impl TemporalIndex {
    /// Binary search for the state at time t.
    pub fn state_at(&self, bundle: &str, bp: BasePoint, t: u64) 
        -> Option<u64>  // WAL offset of most recent entry ≤ t
    
    /// All changes in [t1, t2].
    pub fn changes_between(&self, bundle: &str, bp: BasePoint, t1: u64, t2: u64)
        -> Vec<(u64, u64)>  // (timestamp, WAL_offset) pairs
}
```

**GQL Surface:**
```sql
-- Point-in-time query:
QUERY drugs WHERE organism = 'S. aureus' AS OF '2024-06-15T00:00:00Z';

-- History of a specific record:
HISTORY drugs WHERE compound_id = 'CID-12345' 
  BETWEEN '2024-01-01' AND '2024-12-31';

-- Temporal aggregation:
QUERY drugs AGGREGATE avg(mic) GROUP BY organism
  AS OF '2024-06-15T00:00:00Z';

-- Diff between two timestamps:
DIFF drugs WHERE compound_id = 'CID-12345'
  AT '2024-01-01' AND '2024-07-01';
```

### Implementation Notes

- The WAL already has implicit ordering (sequential entries). Adding an explicit
  timestamp requires a WAL format version bump (add 8-byte timestamp prefix).
- Backward compatibility: entries without timestamps get `timestamp = 0` (unknown).
- The temporal index is built during WAL replay and maintained on new writes.
- For `AS OF` queries: instead of reading current state, binary-search the temporal
  index and replay from the nearest checkpoint.
- Memory cost of temporal index: $O(W)$ where $W$ is total WAL entries. For 36M
  entries, this is ~576 MB at 16 bytes per entry. Could be disk-backed with mmap.
- Temporal checkpoints: DHOOM snapshots already serve as checkpoints. Auto-compaction
  (Feature 1) provides periodic checkpoints for efficient temporal reconstruction.

### Math-Based TDD

```
Test 7.1 — Point-in-time reconstruction:
  Given: insert r(b=1, x=10) at t=100, update r(b=1, x=20) at t=200
  Assert: state_at(b=1, t=150) → x=10
  Assert: state_at(b=1, t=250) → x=20

Test 7.2 — History query:
  Given: insert at t=100, update at t=200, update at t=300
  HISTORY(b=1, t1=100, t2=300) returns 3 entries
  HISTORY(b=1, t1=150, t2=250) returns 1 entry (t=200)

Test 7.3 — Temporal aggregation:
  Given: 100 records inserted at t=100 with mic values
  50 updated at t=200 with new mic values
  AVG(mic) AS OF t=150 uses original values for all 100
  AVG(mic) AS OF t=250 uses updated values for 50, original for other 50

Test 7.4 — Delete is visible in history:
  Given: insert at t=100, delete at t=200
  state_at(b, t=150) → record exists
  state_at(b, t=250) → record does not exist
  HISTORY returns both insert and delete events

Test 7.5 — Timestamp monotonicity:
  Given: sequence of writes w_1, w_2, ..., w_n
  Assert: timestamp(w_i) ≤ timestamp(w_{i+1}) for all i

Test 7.6 — DIFF shows field-level changes:
  Given: r(b=1, x=10, y=20) at t=100, update(x=15) at t=200
  DIFF(b=1, t1=100, t2=200) → {x: {old: 10, new: 15}, y: unchanged}

Test 7.7 — Temporal index binary search correctness:
  Given: entries at timestamps [10, 20, 30, 40, 50]
  Assert: state_at(t=25) → returns entry at t=20
  Assert: state_at(t=10) → returns entry at t=10
  Assert: state_at(t=5) → returns None (before first entry)

Test 7.8 — Backward compatibility:
  Given: WAL with old-format (no timestamp) entries
  Assert: all entries get timestamp = 0
  Assert: AS OF queries with t > 0 return current state
```

---

## 8. Computed Fields *(Deferred)*

> **MIRADOR Review — Deferred.**
> This is pushing application logic into the database — the exact mistake that
> made stored procedures in SQL Server unmaintainable for 20 years. The
> therapeutic coherence formula $C = \tau / K$ belongs in the MIRADOR Rust crate,
> not in the GIGI engine. GIGI's job is storage and retrieval, not arithmetic.
>
> If every team defines their own computed fields, the engine becomes a grab-bag
> of domain-specific formulas with no clear ownership. Keep computation
> client-side (or in dedicated Rust crates linked against GIGI's SDK).
>
> **Revisit if**: Multiple production teams independently request the same
> computed field pattern, proving it's a genuine storage-layer concern.

### Motivation

MIRADOR's therapeutic coherence metric $C = \tau / K$ requires computing derived
quantities server-side. Currently, clients compute these in application code (or
worse, in JSX). Computed fields define server-side expressions that are evaluated
on read, ensuring consistency and keeping math in Rust.

### Mathematical Foundation

**Definition 8.1 (Computed Section).** A computed field $f_c$ is a derived section
defined by a map:

$$
f_c(b) = \phi(f_1(b), f_2(b), \ldots, f_n(b))
$$

where $\phi : F_1 \times \cdots \times F_n \to F_c$ is a smooth map between fiber
spaces and $f_1, \ldots, f_n$ are stored (or other computed) fields.

**Theorem 8.1 (Computed Field Consistency).** A computed field $f_c$ defined as
$\phi(f_1, \ldots, f_n)$ is consistent if $\phi$ is deterministic: for any base point
$b$, $f_c(b)$ depends only on the current fiber values at $b$.

*Proof.* By definition, $f_c(b)$ is a pure function of stored fields at $b$. Since
stored fields are determined by the engine state $\mathcal{S}$, and $\phi$ is
deterministic, $f_c(b)$ is uniquely determined by $\mathcal{S}$. $\square$

**Definition 8.2 (Expression Language).** Computed field expressions are built from:

| Operator | Semantics | Domain |
|----------|-----------|--------|
| $a + b$ | Addition | $\mathbb{R} \times \mathbb{R} \to \mathbb{R}$ |
| $a - b$ | Subtraction | $\mathbb{R} \times \mathbb{R} \to \mathbb{R}$ |
| $a * b$ | Multiplication | $\mathbb{R} \times \mathbb{R} \to \mathbb{R}$ |
| $a / b$ | Division (NaN if $b=0$) | $\mathbb{R} \times \mathbb{R}^* \to \mathbb{R}$ |
| $\log_{10}(a)$ | Decadic logarithm | $\mathbb{R}^+ \to \mathbb{R}$ |
| $\ln(a)$ | Natural logarithm | $\mathbb{R}^+ \to \mathbb{R}$ |
| $\exp(a)$ | Exponential | $\mathbb{R} \to \mathbb{R}^+$ |
| $a^b$ | Power | $\mathbb{R} \times \mathbb{R} \to \mathbb{R}$ |
| $|a|$ | Absolute value | $\mathbb{R} \to \mathbb{R}^+$ |
| $\sqrt{a}$ | Square root | $\mathbb{R}^+ \to \mathbb{R}^+$ |
| $\text{IF}(p, a, b)$ | Conditional | $\{0,1\} \times T \times T \to T$ |
| $\text{COALESCE}(a, b)$ | Null fallback | $T? \times T \to T$ |

**Definition 8.3 (Expression DAG).** Computed fields form a directed acyclic graph
where an edge $f_c \to f_d$ exists if $f_d$'s expression references $f_c$. The DAG
must be acyclic (no circular dependencies). Evaluation order is any topological sort.

**Theorem 8.2 (Evaluation Cost).** For a record with $k$ computed fields, each with
expression of size $s_i$, the evaluation cost is:

$$
T_{\text{eval}} = \sum_{i \in \text{topo-order}} s_i = O(S)
$$

where $S = \sum s_i$ is the total expression size. This is $O(1)$ per record
(expressions are fixed at schema-definition time).

### Design

**Expression AST:**
```rust
#[derive(Debug, Clone)]
pub enum Expr {
    Field(String),           // reference to stored field
    Literal(f64),            // numeric constant
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Pow(Box<Expr>, Box<Expr>),
    Log10(Box<Expr>),
    Ln(Box<Expr>),
    Exp(Box<Expr>),
    Abs(Box<Expr>),
    Sqrt(Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),  // condition, then, else
    Coalesce(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),  // comparison → 1.0 or 0.0
    Lt(Box<Expr>, Box<Expr>),
    Eq(Box<Expr>, Box<Expr>),
}

impl Expr {
    /// Evaluate against a record.
    pub fn eval(&self, record: &Record) -> Value {
        match self {
            Expr::Field(name) => record.get(name).cloned().unwrap_or(Value::Null),
            Expr::Literal(v) => Value::Float(*v),
            Expr::Add(a, b) => binop(a, b, record, |x, y| x + y),
            Expr::Log10(a) => unop(a, record, |x| x.log10()),
            // ... etc
        }
    }
    
    /// Validate: check all referenced fields exist in schema.
    pub fn validate(&self, schema: &BundleSchema) -> Result<(), String>;
    
    /// Check for cycles in the computed field DAG.
    pub fn check_dag(computed_fields: &[(String, Expr)]) -> Result<(), String>;
}
```

**Schema extension:**
```rust
pub struct FieldDef {
    pub name: String,
    pub field_type: FieldType,
    pub default: Value,
    pub range: Option<f64>,
    pub weight: f64,
    pub computed: Option<Expr>,  // NEW: if Some, field is computed on read
}
```

**GQL Surface:**
```sql
-- Add computed field to existing bundle:
ALTER BUNDLE drugs ADD FIELD therapeutic_coherence 
  COMPUTED "log10(auc24 / mic)";

ALTER BUNDLE drugs ADD FIELD potency_index 
  COMPUTED "IF(mic > 0, -log10(mic), 0)";

-- Computed fields appear automatically in query results:
QUERY drugs WHERE organism = 'S. aureus' SELECT [name, mic, therapeutic_coherence];

-- Computed fields can reference other computed fields:
ALTER BUNDLE drugs ADD FIELD normalized_tc 
  COMPUTED "therapeutic_coherence / 2.5";
```

### Implementation Notes

- Computed fields are *virtual* — not stored in the bundle, not in the WAL.
  They are evaluated on read, during record reconstruction.
- The `Expr` type is stored in `FieldDef.computed` and serialized in the schema
  (WAL `CreateBundle` entry). This means schema changes require a WAL entry.
- Expression parsing reuses the existing `parser.rs` infrastructure with a new
  `parse_expr()` function.
- Performance: for 11M records, evaluating $\log_{10}(\text{auc24}/\text{mic})$
  adds ~50ns per record → ~550ms for a full-bundle scan. Acceptable for analytical
  queries; hot-path queries should SELECT only stored fields.
- Null propagation: any operation involving `Value::Null` returns `Value::Null`
  (three-valued logic, consistent with SQL).

### Math-Based TDD

```
Test 8.1 — Simple arithmetic:
  Given: record {auc24: 100.0, mic: 2.0}
  Expr: "auc24 / mic"
  Assert: eval == 50.0

Test 8.2 — Logarithmic computed field:
  Given: record {auc24: 1000.0, mic: 10.0}
  Expr: "log10(auc24 / mic)"
  Assert: eval == 2.0 (log₁₀(100) = 2)

Test 8.3 — Null propagation:
  Given: record {auc24: Null, mic: 10.0}
  Expr: "log10(auc24 / mic)"
  Assert: eval == Null

Test 8.4 — Division by zero:
  Given: record {auc24: 100.0, mic: 0.0}
  Expr: "auc24 / mic"
  Assert: eval == NaN or Null (implementation-defined, but not panic)

Test 8.5 — Conditional expression:
  Given: record {mic: 0.5}
  Expr: "IF(mic > 0, -log10(mic), 0)"
  Assert: eval == 0.301... (-log₁₀(0.5))

  Given: record {mic: 0.0}
  Expr: "IF(mic > 0, -log10(mic), 0)"
  Assert: eval == 0.0 (else branch)

Test 8.6 — DAG cycle detection:
  Given: field A computed as "B + 1", field B computed as "A - 1"
  Assert: check_dag() returns Err("circular dependency: A → B → A")

Test 8.7 — Chained computed fields:
  Given: field tc = "log10(auc24 / mic)", field ntc = "tc / 2.5"
  Record: {auc24: 1000, mic: 10}
  Assert: tc = 2.0, ntc = 0.8
  Assert: evaluation follows topological order (tc before ntc)

Test 8.8 — Computed fields in query filter:
  Given: computed field tc = "log10(auc24 / mic)"
  Query: WHERE tc > 1.5
  Assert: only returns records where log₁₀(auc24/mic) > 1.5

Test 8.9 — COALESCE with null:
  Given: record {a: Null, b: 42.0}
  Expr: "COALESCE(a, b)"
  Assert: eval == 42.0

Test 8.10 — Expression validation:
  Given: schema with fields [mic, auc24], no field named "bogus"
  Expr: "bogus + 1"
  Assert: validate() returns Err("unknown field: bogus")
```

---

## 9. Pub/Sub with Sheaf Triggers

> **MIRADOR Review — Dependency: #3 (CoW Snapshots / MVCC) must ship first.**
> Without read-write separation, trigger evaluation during writes adds latency
> to the write path with no way to defer it. CoW snapshots let triggers evaluate
> against a consistent snapshot on a background thread while the writer continues.
> Without this, every INSERT pays the cost of every active trigger synchronously.

### Motivation

MIRADOR and PRISM need real-time notifications when:
- A sheaf COMPLETE fills a previously-null value (predicted MIC now available)
- An H¹ anomaly is detected (data inconsistency in a therapeutic model)
- A materialized view crosses a threshold (mean MIC for an organism exceeds clinical breakpoint)

Currently, clients must poll. Pub/Sub with sheaf-aware triggers enables push-based
reactive pipelines.

### Mathematical Foundation

**Definition 9.1 (Event Stream).** The event stream $\mathcal{E}$ is the sequence of
state transitions:

$$
\mathcal{E} = \{(t, \Delta\mathcal{S}_t) : t \in T\}
$$

where $\Delta\mathcal{S}_t = \mathcal{S}_t \setminus \mathcal{S}_{t-1}$ is the diff.

**Definition 9.2 (Trigger Predicate).** A trigger $\tau$ is a pair $(\pi, a)$ where
$\pi : \Delta\mathcal{S} \to \{0, 1\}$ is a predicate on state changes and $a$ is
the action (notification payload).

**Definition 9.3 (Sheaf Trigger Classes).**

**(a) Completion Trigger:** Fires when COMPLETE fills a previously-null value:
$$
\pi_{\text{complete}}(\Delta) = \exists\, b \in B,\, f : \sigma_{t-1}(b)[f] = \text{Null} \land \sigma_t(b)[f] \neq \text{Null}
$$

**(b) Anomaly Trigger:** Fires when $H^1$ exceeds threshold:
$$
\pi_{\text{anomaly}}(\Delta) = \exists\, b \in B : H^1_t(b) > \theta \land H^1_{t-1}(b) \leq \theta
$$

**(c) Threshold Trigger:** Fires when an aggregate crosses a boundary value:
$$
\pi_{\text{threshold}}(\Delta, f, v_{\text{thresh}}) = \alpha_t(f) > v_{\text{thresh}} \land \alpha_{t-1}(f) \leq v_{\text{thresh}}
$$

**(d) Insert/Update/Delete Trigger:** Fires on any mutation matching a filter:
$$
\pi_{\text{mutation}}(\Delta, \text{filter}) = \exists\, r \in \Delta : \text{filter}(r)
$$

**Theorem 9.1 (Trigger Evaluation Cost).** For a trigger with predicate complexity
$O(p)$ evaluated per state change, and $M$ active triggers, the overhead per write is
$O(M \cdot p)$. With indexed triggers (hash by bundle name), this reduces to
$O(M_b \cdot p)$ where $M_b$ is the number of triggers on the affected bundle.

### Design

**Trigger definitions:**
```rust
pub enum TriggerKind {
    /// Fires when COMPLETE fills a null field.
    OnComplete {
        bundle: String,
        fields: Vec<String>,
    },
    /// Fires when H¹ exceeds threshold at any base point.
    OnAnomaly {
        bundle: String,
        h1_threshold: f64,
    },
    /// Fires when a materialized view aggregate crosses a threshold.
    OnThreshold {
        view_name: String,
        aggregate_field: String,
        threshold: f64,
        direction: ThresholdDirection, // Rising, Falling, Both
    },
    /// Fires on matching mutations.
    OnMutation {
        bundle: String,
        operation: MutationOp, // Insert, Update, Delete, Any
        filter: Option<Vec<QueryCondition>>,
    },
}

pub enum ThresholdDirection { Rising, Falling, Both }
pub enum MutationOp { Insert, Update, Delete, Any }

pub struct TriggerDef {
    pub name: String,
    pub kind: TriggerKind,
    pub channel: String,  // notification channel name
}
```

**Notification delivery via WebSocket channels:**
```rust
pub struct PubSubManager {
    triggers: Vec<TriggerDef>,
    /// Per-channel subscriber lists.
    channels: HashMap<String, Vec<tokio::sync::broadcast::Sender<Notification>>>,
}

pub struct Notification {
    pub trigger_name: String,
    pub timestamp: u64,
    pub bundle: String,
    pub payload: serde_json::Value, // affected records, aggregate values, etc.
}
```

**GQL Surface:**
```sql
-- Create triggers:
CREATE TRIGGER mic_filled ON COMPLETE drugs FIELDS [mic] 
  NOTIFY 'completions';

CREATE TRIGGER anomaly_alert ON ANOMALY drugs THRESHOLD 3.0 
  NOTIFY 'anomalies';

CREATE TRIGGER breakpoint_crossed ON THRESHOLD drug_stats.mean_mic 
  ABOVE 4.0 NOTIFY 'breakpoints';

CREATE TRIGGER new_compounds ON INSERT drugs 
  WHERE target_class = 'PBP' NOTIFY 'pbp_inserts';

-- Subscribe (WebSocket):
SUBSCRIBE 'completions';
SUBSCRIBE 'anomalies';

-- List triggers:
LIST TRIGGERS;

-- Drop trigger:
DROP TRIGGER mic_filled;
```

### Implementation Notes

- The WebSocket infrastructure already exists in `gigi_stream.rs` (channels,
  broadcast). Triggers add a layer on top of the existing broadcast mechanism.
- Triggers are evaluated synchronously after each write operation. For
  `OnComplete` and `OnAnomaly`, triggers fire after the COMPLETE/CONSISTENCY
  operation completes (these are explicit user commands, not per-insert).
- `OnMutation` triggers fire per-insert/update/delete, so performance matters.
  Use a HashMap indexed by `(bundle_name, operation)` for O(1) lookup.
- `OnThreshold` triggers integrate with materialized views (Feature 6) — the
  view update also checks threshold crossings.
- Trigger definitions are persisted in the WAL: `WalEntry::CreateTrigger(TriggerDef)`
  and `WalEntry::DropTrigger(String)`.
- Notification ordering: notifications are emitted in WAL order (same order as
  writes). This preserves causal ordering for subscribers.

### Math-Based TDD

```
Test 9.1 — Completion trigger fires:
  Given: bundle with record r(b=1, mic=Null)
  Trigger: ON COMPLETE drugs FIELDS [mic]
  Run COMPLETE → mic filled to 2.5
  Assert: notification received with payload {bp: 1, field: "mic", value: 2.5}

Test 9.2 — Completion trigger does NOT fire for already-filled:
  Given: record r(b=1, mic=4.0) (already has value)
  Run COMPLETE
  Assert: no notification (mic was not null before)

Test 9.3 — Anomaly trigger fires on H¹ crossing:
  Given: trigger threshold θ = 3.0
  Insert data producing H¹(b=5) = 3.5
  Run CONSISTENCY → detects anomaly
  Assert: notification with {bp: 5, h1: 3.5}

Test 9.4 — Anomaly trigger does NOT fire below threshold:
  Given: H¹(b) = 2.0 < θ = 3.0
  Assert: no notification

Test 9.5 — Threshold trigger on rising edge:
  Given: view drug_stats.mean_mic = 3.8, threshold = 4.0
  Insert records pushing mean_mic to 4.2
  Assert: notification fires (rising crossing: 3.8 → 4.2)
  Insert more records pushing mean_mic to 4.5
  Assert: NO additional notification (already above threshold)

Test 9.6 — Mutation trigger with filter:
  Given: trigger ON INSERT drugs WHERE target_class = 'PBP'
  Insert record {target_class: 'PBP', name: 'ceftaroline'}
  Assert: notification fires
  Insert record {target_class: 'ribosome', name: 'azithromycin'}
  Assert: no notification (filter not matched)

Test 9.7 — Multiple triggers on same bundle:
  Given: 3 triggers on bundle drugs
  Insert 1 record matching all 3
  Assert: 3 notifications emitted, one per trigger

Test 9.8 — Trigger survives restart:
  Given: active triggers, snapshot(), close, reopen
  Assert: trigger definitions restored from WAL
  Assert: triggers fire correctly after restart
```

---

## 10. Query Cost Planner

> **MIRADOR Review — Dependency: #4 (Secondary Indexes) must ship first.**
> Without indexes, there is exactly one execution strategy: full scan. A planner
> with one option is not a planner — it's dead code. Build this only after hash
> indexes exist and there's a genuine choice between access paths.

### Motivation

As GIGI gains secondary indexes, materialized views, temporal indexes, and computed
fields, the query executor must choose between multiple access paths. Currently, every
query does a full scan. A cost-based planner selects the optimal strategy, reducing
query latency by orders of magnitude for selective queries on indexed bundles.

### Mathematical Foundation

**Definition 10.1 (Query Plan).** A query plan $P$ is a tree of physical operators.
Each operator has an estimated cost:

$$
\text{cost}(P) = \sum_{op \in P} \text{cost}(op)
$$

**Definition 10.2 (Access Path Costs).** For a query on bundle $B$ with $N$ records:

**(a) Full Scan:**
$$
C_{\text{scan}}(N) = N \cdot c_{\text{row}}
$$

where $c_{\text{row}}$ is the per-record evaluation cost (including predicate check).

**(b) Index Lookup (Equality):**
$$
C_{\text{index}}(f, v) = c_{\text{lookup}} + |\mathcal{I}_f(v)| \cdot c_{\text{fetch}}
$$

where $c_{\text{lookup}}$ is the HashMap lookup cost and $c_{\text{fetch}}$ is the
record fetch cost from base point.

**(c) Index Intersection:**
$$
C_{\text{intersect}}(f_1, v_1, \ldots, f_k, v_k) = \sum_{i=1}^{k} c_{\text{lookup}} + \text{min-card} \cdot k \cdot c_{\text{bitmap}}
$$

where $\text{min-card} = \min_i |\mathcal{I}_{f_i}(v_i)|$ and $c_{\text{bitmap}}$ is
the per-element bitmap AND cost.

**(d) View Lookup:**
$$
C_{\text{view}}(v, g) = c_{\text{lookup}}
$$

Constant time for a grouped materialized view lookup.

**Theorem 10.1 (Plan Optimality).** Given a set of candidate plans $\{P_1, \ldots, P_m\}$,
the optimizer selects:

$$
P^* = \arg\min_{P_i} \text{cost}(P_i)
$$

For GIGI's current feature set ($m \leq 5$ plan types), exhaustive enumeration is
optimal and runs in $O(m)$.

**Definition 10.3 (Selectivity Estimation).** The selectivity of predicate $p$ on field
$f$ is estimated from `FieldStats`:

**(a) Equality:** $\text{sel}(f = v) = 1 / |\text{distinct}(f)|$ (uniform assumption)
or $|\mathcal{I}_f(v)| / N$ if indexed.

**(b) Range:** $\text{sel}(a \leq f \leq b) = (b - a) / (\text{max}(f) - \text{min}(f))$
(uniform assumption using `FieldStats.min` and `FieldStats.max`).

**(c) Conjunction:** $\text{sel}(p_1 \land p_2) = \text{sel}(p_1) \cdot \text{sel}(p_2)$
(independence assumption).

**Theorem 10.2 (Cost-Benefit Threshold).** An index scan is preferred over a full scan
when:

$$
|\mathcal{I}_f(v)| \cdot c_{\text{fetch}} < N \cdot c_{\text{row}}
$$

i.e., when selectivity $< c_{\text{row}} / c_{\text{fetch}}$. For typical in-memory
engines, $c_{\text{fetch}} \approx 2 \cdot c_{\text{row}}$ (random access overhead),
so the threshold is $\text{sel} < 0.5$.

### Design

**Planner pipeline:**
```
Parse → Analyze → Plan → Execute
                   ↓
            CostEstimator
                   ↓
            PlanEnumerator
                   ↓
            PlanSelector (min cost)
```

```rust
pub struct QueryPlan {
    pub access: AccessMethod,
    pub residual_predicates: Vec<QueryCondition>,
    pub estimated_cost: f64,
    pub estimated_rows: usize,
}

pub enum AccessMethod {
    /// Full sequential scan of all records.
    FullScan,
    /// Single-field index lookup.
    IndexLookup { field: String, values: Vec<Value> },
    /// Multi-field index intersection.
    IndexIntersection { lookups: Vec<(String, Vec<Value>)> },
    /// Materialized view lookup (for aggregate queries).
    ViewLookup { view_name: String, group_key: Vec<Value> },
    /// Temporal index scan (for AS OF queries).
    TemporalScan { timestamp: u64 },
}

pub struct CostEstimator {
    /// Cost coefficients (tunable).
    row_cost: f64,      // default: 1.0
    fetch_cost: f64,    // default: 2.0
    lookup_cost: f64,   // default: 10.0
    bitmap_cost: f64,   // default: 0.1
}

impl CostEstimator {
    pub fn estimate(&self, plan: &QueryPlan, stats: &BundleStats) -> f64;
    
    pub fn selectivity(&self, cond: &QueryCondition, stats: &BundleStats) -> f64;
}

pub struct PlanEnumerator;

impl PlanEnumerator {
    /// Generate all candidate plans for a query.
    pub fn enumerate(
        &self,
        conditions: &[QueryCondition],
        bundle: &BundleStore,
        views: &[MaterializedView],
    ) -> Vec<QueryPlan>;
}
```

**EXPLAIN command:**
```rust
// Returns the chosen plan without executing the query.
pub fn explain(plan: &QueryPlan) -> String {
    format!(
        "Access: {:?}\nEstimated rows: {}\nEstimated cost: {:.2}\nResidual predicates: {:?}",
        plan.access, plan.estimated_rows, plan.estimated_cost, plan.residual_predicates
    )
}
```

**GQL Surface:**
```sql
EXPLAIN QUERY drugs WHERE organism = 'S. aureus' AND mic < 4.0;

-- Output:
-- Plan: IndexLookup(organism = 'S. aureus') + residual(mic < 4.0)
-- Estimated rows: 10,000 (of 11,000,000)
-- Estimated cost: 20,010.0
-- Alternative: FullScan cost: 11,000,000.0
```

### Implementation Notes

- The planner is inserted between parsing and execution in `gigi_stream.rs`
  query handlers. Currently these call `filtered_query_ex()` directly; with the
  planner, they call `plan()` then `execute(plan)`.
- `BundleStats` already exists as `FieldStats` (count, sum, sum_sq, min, max).
  The planner needs `distinct_count` per field — add this to `FieldStats` or
  derive from `field_index.len()` for indexed fields.
- The planner is deterministic and stateless — no learning or adaptation needed
  for Phase 1. Cardinality estimation from uniform distribution is sufficient
  given GIGI's use case (analytical queries, not OLTP).
- For Phase 2: collect actual vs. estimated cardinalities and adjust assumptions.

### Math-Based TDD

```
Test 10.1 — Full scan cost is O(N):
  Given: N = 1_000_000, no indexes
  Assert: plan == FullScan
  Assert: cost == N * c_row = 1_000_000

Test 10.2 — Index lookup beats scan:
  Given: N = 1_000_000, index on organism, |I_organism('S. aureus')| = 1000
  Assert: plan == IndexLookup
  Assert: cost = c_lookup + 1000 * c_fetch = 2010
  Assert: cost < N * c_row = 1_000_000

Test 10.3 — Index intersection:
  Given: N = 1M, index on [organism, target]
  |I_organism| = 10K, |I_target| = 500
  Assert: plan == IndexIntersection, target first (smaller)
  Assert: cost ≈ 2 * c_lookup + 500 * 2 * c_bitmap + |result| * c_fetch

Test 10.4 — Full scan when non-selective:
  Given: N = 100, index on gender, |I_gender('M')| = 50
  Assert: plan == FullScan (selectivity 50% → scan is cheaper)

Test 10.5 — Selectivity estimation (equality):
  Given: field with 100 distinct values in index
  Assert: sel(f = v) = 1/100 = 0.01

Test 10.6 — Selectivity estimation (range):
  Given: FieldStats { min: 0.0, max: 100.0 }
  Assert: sel(f BETWEEN 20 AND 30) = 10/100 = 0.1

Test 10.7 — Conjunction selectivity:
  Given: sel(p1) = 0.01, sel(p2) = 0.05
  Assert: sel(p1 AND p2) = 0.0005 (independence)

Test 10.8 — EXPLAIN output matches execution:
  Given: query Q, planner chooses plan P with estimated_rows = E
  Execute Q → actual_rows = A
  Assert: A ≤ 10 * E or A ≥ E / 10 (within 10x, log-scale accuracy)

Test 10.9 — View lookup for aggregate queries:
  Given: materialized view V on bundle B, query = aggregate on V's group
  Assert: plan == ViewLookup
  Assert: cost = c_lookup (constant, independent of N)

Test 10.10 — Planner determinism:
  Given: same query, same stats
  Call plan() 100 times
  Assert: all 100 produce identical plans
```

---

## 11. Memory-Mapped Bundles

> **MIRADOR Review — Biggest infrastructure win for memory.**
> Everything lives in heap memory, which is why 11M records need a 32 GB VM.
> Memory-mapping the DHOOM files would let the OS page in/out as needed, dropping
> the resident memory requirement by 5–10×. This is what would let GIGI run on a
> 2× machine instead of a 4× machine — an immediate cost savings.

### Motivation

GIGI currently deserializes every bundle fully into heap-allocated `Vec<Value>`
structures on startup. For 11M records across ~10 bundles, this consumes the
majority of the 32 GB Fly.io VM. Most queries touch a small fraction of records
at a time, yet the entire dataset sits in resident memory. Memory-mapped files
let the OS virtual memory subsystem act as an LRU cache over the DHOOM data,
paging in only the regions actively being read and evicting cold pages under
memory pressure.

### Mathematical Foundation

**Definition 11.1 (Working Set).** For a query workload $\mathcal{Q}$ over time
window $\Delta t$, the working set $W(\Delta t)$ is the set of distinct base points
accessed:

$$
W(\Delta t) = \bigcup_{q \in \mathcal{Q}_{\Delta t}} \text{touched}(q)
$$

**Theorem 11.1 (Resident Memory Bound).** With memory-mapped bundles, the resident
set size (RSS) is bounded by:

$$
\text{RSS} \leq |W(\Delta t)| \cdot \bar{r} + M_{\text{index}} + M_{\text{overhead}}
$$

where $\bar{r}$ is the mean serialized record size, $M_{\text{index}}$ is the
in-memory index footprint (field indexes, temporal indexes), and $M_{\text{overhead}}$
is engine/runtime overhead. Compare to the current model:

$$
\text{RSS}_{\text{current}} = N \cdot \bar{r}_{\text{heap}} + M_{\text{index}} + M_{\text{overhead}}
$$

where $\bar{r}_{\text{heap}} \gg \bar{r}$ due to heap allocation overhead (Box, Vec,
HashMap entry cost). The ratio:

$$
\frac{\text{RSS}_{\text{mmap}}}{\text{RSS}_{\text{current}}} \approx \frac{|W|}{N} \cdot \frac{\bar{r}}{\bar{r}_{\text{heap}}}
$$

For typical MIRADOR workloads where $|W|/N \approx 0.1$ and
$\bar{r}/\bar{r}_{\text{heap}} \approx 0.5$, this gives a **~20× reduction** in
resident memory — from 32 GB to ~1.5 GB.

**Definition 11.2 (Page Fault Cost).** Accessing a non-resident page incurs a page
fault with cost $c_{\text{fault}}$ (typically 1–10 µs for SSD-backed storage). A
query touching $k$ non-resident pages costs:

$$
T_{\text{query}} = T_{\text{compute}} + k \cdot c_{\text{fault}}
$$

For sequential scans, the kernel's readahead prefetcher reduces effective $k$ by
batching contiguous pages. For random-access (point queries on hashed bundles),
every lookup may trigger a fault.

**Theorem 11.2 (Amortized Scan Cost).** With readahead of $R$ pages and a sequential
scan of $P$ pages:

$$
T_{\text{scan}} = P \cdot c_{\text{row}} + \lceil P/R \rceil \cdot c_{\text{fault}}
$$

For $R = 32$ (128 KB readahead, typical Linux default) and 4 KB pages:

$$
T_{\text{scan}} \approx P \cdot c_{\text{row}} + \frac{P}{32} \cdot c_{\text{fault}}
$$

The fault overhead is $\leq 3\%$ of total scan time for in-memory $c_{\text{row}}$.

**Definition 11.3 (DHOOM Region Layout).** A memory-mapped DHOOM file is partitioned
into regions:

$$
\text{File} = \underbrace{[\text{Header}]}_{H} \| \underbrace{[\text{Schema}]}_{S} \| \underbrace{[\text{Records}]}_{R_1 \| R_2 \| \cdots \| R_N}
$$

Each record $R_i$ at file offset $o_i$ is accessible via:

$$
\text{ptr}_i = \text{mmap\_base} + o_i
$$

Zero-copy access: the record bytes are read directly from the kernel page cache
without deserialization into heap objects for fields that support zero-copy reading
(integers, floats, fixed-size types).

### Design

**Mmap-backed bundle storage:**
```rust
use memmap2::Mmap;

pub struct MmapBundle {
    /// Memory-mapped DHOOM file.
    mmap: Mmap,
    /// Record offset table: base_point → file offset.
    offsets: HashMap<BasePoint, u64>,
    /// Total records in file.
    record_count: usize,
    /// Schema (always in memory — small).
    schema: BundleSchema,
}

impl MmapBundle {
    /// Open a DHOOM file as memory-mapped.
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        let (schema, offsets) = Self::parse_header_and_index(&mmap)?;
        Ok(Self { mmap, offsets, record_count: offsets.len(), schema })
    }

    /// Read a single record (zero-copy where possible).
    pub fn get(&self, bp: &BasePoint) -> Option<Record> {
        let offset = *self.offsets.get(bp)? as usize;
        Some(Record::deserialize_from_slice(&self.mmap[offset..]))
    }

    /// Sequential scan with kernel readahead.
    pub fn scan(&self) -> MmapScanIter<'_> {
        MmapScanIter::new(&self.mmap, &self.schema)
    }

    /// Advise kernel on access pattern.
    pub fn advise_sequential(&self) {
        #[cfg(unix)]
        self.mmap.advise(memmap2::Advice::Sequential).ok();
    }

    pub fn advise_random(&self) {
        #[cfg(unix)]
        self.mmap.advise(memmap2::Advice::Random).ok();
    }
}
```

**Integration with Engine:**
```rust
pub enum BundleBackend {
    /// Current: fully deserialized in heap memory.
    InMemory(BundleStore),
    /// New: memory-mapped DHOOM file, records read on demand.
    Mmap(MmapBundle),
}

impl Engine {
    /// Open bundles from DHOOM snapshots as mmap instead of deserializing.
    pub fn open_mmap(data_dir: &Path) -> Result<Self, Error> {
        let mut engine = Engine::open_empty();
        // For each .dhoom file, open as MmapBundle
        for dhoom_file in fs::read_dir(data_dir)?.filter(|f| is_dhoom(f)) {
            let bundle = MmapBundle::open(&dhoom_file.path())?;
            engine.bundles.insert(bundle.name().into(), BundleBackend::Mmap(bundle));
        }
        // WAL entries after snapshot go into InMemory overlay
        engine.replay_wal_overlay(data_dir)?;
        Ok(engine)
    }
}
```

**WAL overlay for recent writes:**
```rust
/// Records written since the last DHOOM snapshot live in an in-memory overlay.
/// Point queries check overlay first, then fall through to mmap.
pub struct OverlayBundle {
    /// Recent inserts/updates (since last snapshot).
    overlay: BundleStore,
    /// Deleted base points (tombstones).
    tombstones: HashSet<BasePoint>,
    /// Mmap-backed snapshot data.
    base: MmapBundle,
}

impl OverlayBundle {
    pub fn get(&self, bp: &BasePoint) -> Option<Record> {
        if self.tombstones.contains(bp) { return None; }
        self.overlay.get(bp).or_else(|| self.base.get(bp))
    }

    pub fn scan(&self) -> impl Iterator<Item = Record> + '_ {
        let base_iter = self.base.scan()
            .filter(|r| !self.tombstones.contains(&r.base_point()))
            .filter(|r| !self.overlay.contains(&r.base_point()));
        let overlay_iter = self.overlay.scan();
        base_iter.chain(overlay_iter)
    }

    /// After snapshot, promote overlay to new mmap and clear.
    pub fn compact(&mut self, new_dhoom: &Path) -> io::Result<()> {
        self.base = MmapBundle::open(new_dhoom)?;
        self.overlay.clear();
        self.tombstones.clear();
        Ok(())
    }
}
```

**GQL Surface:**
```sql
-- Engine configuration (in gigi_stream startup or config):
SET STORAGE MODE MMAP;         -- default: HEAP
SET STORAGE MODE HEAP;         -- revert to current behavior

-- Runtime stats:
SHOW STORAGE;
-- Output:
--   bundle 'drugs': mmap, 4.2 GB file, 1.1 GB resident, 11M records
--   bundle 'drugs' overlay: heap, 12K records (since last snapshot)
--   bundle 'trials': mmap, 800 MB file, 200 MB resident, 2M records
```

### Implementation Notes

- Use the `memmap2` crate (maintained fork of `memmap`, already widely used in
  the Rust ecosystem). Add `memmap2 = "0.9"` to `Cargo.toml`.
- The DHOOM file format already has a header + record stream layout. The only
  addition is a **record offset index** appended at the end of the file (or
  stored in a sidecar `.idx` file) for O(1) point queries.
- On Windows (`Fly.io` is Linux, but local dev is Windows): `memmap2` supports
  both platforms. `Mmap::advise()` is a no-op on Windows.
- The overlay pattern (mmap base + in-memory delta) is the same architecture
  used by SQLite WAL mode and RocksDB. Auto-compaction (Feature #1) periodically
  flushes the overlay into a new DHOOM snapshot + mmap.
- Field indexes remain in-memory (they're $O(N)$ pointers, not $O(N \cdot \bar{r})$
  data). For 11M records with 5 indexed fields, index memory is ~500 MB vs
  ~25 GB for full record data.
- **Risk**: Zero-copy deserialization requires the DHOOM record format to be
  alignment-friendly. Current DHOOM uses variable-length encoding — may need a
  "flat" DHOOM variant for mmap (Phase 2 optimization).

### Math-Based TDD

```
Test 11.1 — Mmap bundle reads correct data:
  Given: DHOOM file with 1000 records
  Open as MmapBundle
  For each record: assert mmap.get(bp) == original_record

Test 11.2 — Point query O(1):
  Given: MmapBundle with 1M records
  Lookup single base point
  Assert: completes in < 1ms (no full scan)

Test 11.3 — Sequential scan correctness:
  Given: MmapBundle with N records
  scan() → collect all records
  Assert: count == N, all records match original data

Test 11.4 — Overlay masks mmap for updates:
  Given: MmapBundle with record r(bp=1, x=10)
  Overlay: insert r(bp=1, x=20)
  get(bp=1) → x=20 (overlay wins)

Test 11.5 — Tombstones hide mmap records:
  Given: MmapBundle with record r(bp=1)
  Overlay: delete bp=1
  get(bp=1) → None

Test 11.6 — Scan merges overlay and mmap:
  Given: MmapBundle with records bp ∈ {1,2,3}
  Overlay: insert bp=4, delete bp=2
  scan() → returns {1, 3, 4}

Test 11.7 — Compact promotes overlay to new mmap:
  Given: OverlayBundle with base (1000 records) + overlay (50 inserts)
  Snapshot + compact
  Assert: new mmap has 1050 records, overlay is empty

Test 11.8 — RSS reduction under working set:
  Given: MmapBundle with 1M records (500 MB file)
  Access only 1000 records
  Assert: RSS < 50 MB (not 500 MB)
  (Measured via /proc/self/statm on Linux, approximated on other platforms)

Test 11.9 — Concurrent read safety:
  Given: MmapBundle shared across N reader threads
  All threads read different base points simultaneously
  Assert: no data corruption, no panics

Test 11.10 — Mmap survives engine restart:
  Given: MmapBundle open, process restarts
  Re-open same DHOOM file → same data accessible
  Assert: no file corruption (mmap is read-only on DHOOM files)
```

---

## Cross-Feature Integration Matrix

| Feature | Tier | Depends On | Enhances |
|---------|------|-----------|----------|
| 1. Auto-Compaction | 1 | 2 (streaming encoder) | 11 (flush overlay → new mmap) |
| 2. Streaming DHOOM | 1 | — | 1 (low-memory snapshots), 11 (generate mmap-friendly files) |
| 3. CoW Snapshots | 1 | — | 9 (non-blocking trigger eval), 10 (concurrent reads) |
| 4. Hash Indexes | 2 | — | 10 (index access plans), 6 (group lookups) |
| 10. Query Planner | 2 | 4 (indexes must exist) | All query paths |
| 11. Mmap Bundles | 2 | 2 (DHOOM files as source) | 1 (compact = new mmap), memory budget |
| 6. TTL Query Cache | 3 | — | 10 (cache as access path) |
| 9. Pub/Sub Triggers | 3 | 3 (CoW for async eval) | — |
| 5. Incr. COMPLETE | Defer | 4 (neighbor lookup) | 9 (completion triggers) |
| 7. Temporal Queries | Defer | 1 (checkpoints) | — |
| 8. Computed Fields | Defer | — | — |

## Suggested Implementation Order

> **Revised per MIRADOR team review (2026-03).**
>
> Previous order optimized for feature completeness. New order optimizes for
> **production stability and cost reduction** — the things that actually matter
> when 11M records are running on a 32 GB VM.

```
Phase 1 — Stop the bleeding:
  #2 Streaming DHOOM     →  #1 Auto-Compaction  →  #3 CoW Snapshots
  (unblock snapshots)       (prevent WAL bloat)     (safe background ops)

Phase 2 — Performance:
  #4 Hash Indexes        →  #10 Query Planner   →  #11 Mmap Bundles
  (days not weeks)          (only after indexes)    (biggest memory win)

Phase 3 — Features:
  #6 TTL Query Cache     →  #9 Pub/Sub Triggers
  (lightweight, high ROI)   (after CoW ships)

Deferred:
  #5 Incremental COMPLETE  — sheaves are small, indexes solve the bottleneck
  #7 Temporal Queries      — WAL-time ≠ domain-time, needs proper model
  #8 Computed Fields       — stored procedure anti-pattern, keep client-side
```

Each phase ships and stabilizes before the next begins. Features within a phase
can be developed in parallel but should be merged and tested sequentially.

---

## Appendix A: Fiber Bundle Geometry Reference

For readers unfamiliar with the fiber bundle formalism, the key geometric concepts
used throughout this spec:

**Fiber Bundle** $E = (B, F, \pi)$:
- $B$ = base space (key fields: e.g., compound_id, organism)
- $F$ = fiber (data fields: e.g., mic, auc24)
- $\pi : E \to B$ = projection (extracting the key)
- A *section* $s : B \to E$ with $\pi \circ s = \text{id}_B$ is a record

**Connection** $\nabla$: Defines "parallel transport" — how to compare fibers at
different base points. In GIGI, adjacency definitions serve as the connection.

**Curvature** $K$: Measures how much parallel transport depends on the path. $K = 0$
(flat) means sequential storage works. $K > 0$ means hashed storage is needed.

**Cohomology** $H^1$: Obstruction to finding a globally consistent section. High $H^1$
indicates anomalous data (fiber values that cannot be explained by the local geometry).

**Sheaf Completion**: Uses the Laplacian $\Delta$ on the neighborhood graph to "fill in"
missing fiber values by minimizing the energy functional:

$$
E[\sigma] = \frac{1}{2} \sum_{(b, b') \in E_{\text{adj}}} w_{bb'} \|\sigma(b) - \Gamma_{bb'}\sigma(b')\|^2
$$

where $\Gamma_{bb'}$ is the parallel transport map along edge $(b, b')$.

---

*End of specification.*
