# TDD-IDX — The index set is state, and it is not durable

**Status:** implementation spec. Analysis was read-only; nothing in the tree was
edited to produce it.
**Ground truth:** every line reference below was read against `main` @ `dd92ef8`
on 2026-08-15. Where the E17 verb audit cites a line number, ours is exactly
+20 — the audit was measured one commit earlier in the same file. That offset is
uniform across all five of its mutator citations, which corroborates its
measurements rather than undermining them.
**Provenance:** three of these defects were surfaced by the E22 review of the
E17 verb audit (`helicity/integration/daily/VERB_AUDIT.md`). Two were not — they
are visible only from inside `gigi-stream` and are the reason this is a
durability spec rather than a five-line cache patch.

---

## 1. THE INVARIANT

INV-D, from `TDD_DUR_wal_truncation_invariant.md`, says the WAL may be replaced
by a shorter one only when the state reconstructible from disk alone equals what
the engine serves from RAM. That invariant is correct. Its **domain was too
narrow**: it was written, and implemented, as if "state" meant "records."

> **INV-I.** For every bundle `B`, the pair `(records(B), index_set(B))`
> reconstructible from disk alone after any restart must equal the pair the
> engine served from RAM before it, **restricted to acknowledged state** — that
> is, to mutations whose originating call had already returned success.

The acknowledgement scope is not a weakening for convenience; it is what makes
the invariant achievable at all. Without it INV-I would forbid losing a write
that was still in flight when the process died, which no journalled system
provides. It is the same scope INV-D carries, and §4's F-0 states the exact
crash window it leaves open.

Two consequences, both load-bearing:

1. `index_set` must be journalled, exactly like records are. It is not.
2. Any quantity derived from `index_set` must be keyed on `index_set`, exactly
   as record-derived quantities are keyed on record mutations. It is not.

The reason this is a durability invariant and not a configuration nicety is
Section 2: **`index_set` is not a performance setting. It is the definition of
the object the λ-verbs measure.** Two engines holding identical records and
different index sets are not the same database returning the same answers
faster and slower. They are returning answers about different graphs.

---

## 2. WHY THE INDEX SET IS SEMANTIC, NOT A PERFORMANCE KNOB

### 2.1 The graph under measurement

`field_index_graph` (`spectral.rs:280-304`) constructs, from a store `B` and its
`schema.indexed_fields` set `F`:

```
V = records(B)
p ~ q   iff   there exists f in F with  p[f] = q[f],  both non-null,  p != q
```

Equivalently: the union, over `f in F` and each distinct value `v` of `f`, of
the complete graph on that bucket. This is the codebase's own Def 3.9 graph. The
λ-verbs (`DEPTH`, `HORIZON`, `SPECTRAL_GAP`, `BETTI`) all key off its spectrum.

The operator is the **normalized** Laplacian, Def 3.10 (`spectral.rs:595, 639`):

```
L = I − D^(−1/2) W D^(−1/2)
```

computed by sparse power iteration with deflation against the dominant
eigenvector `u = D^(1/2) · 1` (`spectral.rs:717-721`). Isolated vertices
contribute a zero row (`spectral.rs:2234`).

### 2.2 The theorem

> **Theorem (spectral graph theory, standard; Fiedler 1973 for the connectivity
> reading).** Let `G = (V, E)` be a finite, undirected graph with non-negative
> edge weights. Define `L` on **all** of `V` by
>
> ```
> L[u][v] = 1                      if u = v and deg(u) > 0
>         = −W[u][v]/sqrt(d_u d_v) if u ~ v
>         = 0                      otherwise    (so an isolated vertex is a zero row)
> ```
>
> Then `dim ker L` = the number of connected components of `G`, **counting each
> isolated vertex as its own component**.
>
> **Hypotheses, all required:** finite vertex set; undirected (symmetric `W`);
> non-negative weights; and the zero-row convention above. The Def 3.9 graph
> satisfies all four by construction — it is a union of cliques on a finite
> record set, unweighted (see §2.4), with the isolated-vertex convention fixed
> at `spectral.rs:2234`.
>
> **Corrected 2026-08-15 (Hallie, review of v1).** v1 stated this for `L`
> *restricted* to positive-degree vertices, and simultaneously cited the
> zero-row convention the code implements. Those are two different operators
> with two different kernels: under restriction, `dim ker L` counts only
> components containing an edge. The spec then asserted V-4 (`components = n` on
> an empty index set) which is **false under the stated theorem** — with no
> indexed fields there are no positive-degree vertices, so the restricted
> operator is `0×0` and its kernel is trivial. The theorem is now stated for the
> operator the engine actually builds, and V-4 follows from it.

`spectral_gap_explicit` does not merely inherit this result — it **implements**
it, short-circuiting to `0.0` whenever the component count exceeds one
(`spectral.rs:358-360`), before any iteration runs.

### 2.3 The three regimes, and the correction the audit's phrasing needs

Let `F` be the indexed-field set and `v(f)` the number of distinct non-null
values of `f`.

Assume for this table that every record has a non-null, non-NaN value in each
indexed field; §2.7 removes that assumption and it changes the component counts.

The two right-hand columns are **different quantities** and v1 conflated them,
which is the same conflation §2.6 exists to attack. `true λ₁` is a property of
the operator; `engine returns` is what `spectral_gap_explicit` hands back after
its guard.

| `|F|` | graph | components | true λ₁ | engine returns |
|---|---|---|---|---|
| 0 | no edges, `n` isolated vertices | `n` | **undefined** — `L` is the zero matrix, no non-zero eigenvalue exists | `0.0` (guard) |
| 1, `v(f) = 1` | `K_n` | 1 | `n/(n−1)` | `n/(n−1)` |
| 1, `v(f) ≥ 2`, buckets of size `g` | `v(f)` disjoint `K_g` | `v(f)` | `g/(g−1)` | `0.0` (guard) |
| ≥ 2 | union of clique covers | 1 iff the record-bucket incidence is connected | as computed | equal to true λ₁ iff connected, else `0.0` (guard) |

Row 3 is the one worth staring at: the operator's smallest non-zero eigenvalue
on five disjoint pairs is `2.0`, and the engine returns `0.0`. The returned
value is not an approximation of the real one. It is a different kind of thing.

The E17 audit states the rule as *"λ-driven verbs need at least two indexed
low-cardinality fields."* That is the right operational recipe and the wrong
**precondition**, and the difference matters because the recipe is about to be
compiled into a refusal gate. Field count is neither necessary nor sufficient:

- **Not necessary.** One indexed field with a single distinct value yields `K_n`,
  which is connected, with `λ₁ = n/(n−1)`. A gate keyed on `|F| ≥ 2` refuses a
  bundle whose spectrum is perfectly well-defined.
- **Not sufficient.** Two indexed fields whose buckets do not bridge leave the
  graph disconnected and `λ₁ = 0`. A gate keyed on `|F| ≥ 2` accepts it and the
  verb answers from a degenerate graph — which is the exact hazard the audit
  raised.

> **The precondition is the measured component count, not the field count.**
> `F-6` below states it as `components_from_index(store).len() == 1`, which is
> the quantity the theorem is about and the quantity `spectral.rs:358` already
> computes.

### 2.4 Expressiveness lemma — why closed-form validation is reachable at all

A validation battery is only worth writing if the engine can be driven to graphs
whose spectra are known in closed form, **through the index mechanism itself**
rather than through a test-only back door. It can:

> **Lemma.** Let `f_1 … f_k` be indexed fields such that (i) every bucket of
> every `f_i` contains exactly two records, (ii) each `f_i`'s buckets cover all
> `n` records, and (iii) **the `k` matchings are pairwise edge-disjoint.** Then
> the induced Def 3.9 graph is the union of `k` perfect matchings, hence
> `k`-regular.
>
> **Corollary.** For a `k`-regular graph, `D = kI`, so `L_norm = L_comb / k`,
> and every closed-form combinatorial spectrum transfers by dividing by `k`.

**Hypothesis (iii) is required and was missing in v1 (Hallie).** Adjacency in
§2.1 is a *boolean* relation — "there exists `f in F` with `p[f] = q[f]`" — so
two fields that pair the same two records contribute one edge, not two. Without
disjointness the union of `k` matchings is `j`-regular for some `j ≤ k`, and the
Corollary's `D = kI` silently fails along with every closed form built on it.

**Verified, not assumed: the graph is unweighted.** `field_index_graph` accumulates
into `HashMap<BasePoint, HashSet<BasePoint>>` and inserts neighbours into that
set (`spectral.rs:281, 290`). A `HashSet` insert of an existing element is a
no-op, so coincident edges from different fields collapse to one and no weight
is accumulated. Had it been a `Vec` or a counter, `W` would be weighted, `D ≠ kI`,
and V-3's closed form would be wrong with it. This is the check hypothesis (iii)
makes necessary and it passes.

V-3's construction satisfies (iii) for `n ≥ 4`. At `n = 2` field `a` gives
`{0,1}` and field `b` gives `{1,0}` — the same edge — so the construction
degenerates to `K_2`, which is the second reason the V-3 table starts at 4.

This is what makes Section 6 possible. The test fixtures are ordinary bundles
with ordinary indexed fields; the graphs they induce happen to be `K_n` and
`C_n`, whose normalized spectra are exact.

### 2.5 Invalidation and identity are different problems

The audit's fix — call `mark_mutated()` in the five mutators that skip it — is
correct and sufficient **for invalidation**. `mark_mutated` (`bundle.rs:1356-1370`)
bumps `mutation_counter` and clears `spectral_gap_cache`, so any consumer that
re-reads will recompute.

It is not sufficient for **identity**. `mutation_counter` answers "has anything
changed since you last looked." It cannot answer "is this the same geometry as
the number in my report," because it is a counter, not a content hash, and
because it conflates a record change with an index change. Anything that
persists a λ value alongside a bundle reference — a receipt, a report, a
cached attribution — needs

```
bundle_version = H(records, index_set)
```

not `H(records)`, and today needs neither because no `bundle_version` exists
anywhere in the tree (`grep 'bundle_version' src/` → zero hits).

Both are in scope. `F-1` closes invalidation cheaply. `F-5` closes identity and
is the one that matters for a diagnosis product whose outputs get quoted.

### 2.6 `λ₁` names two different things, and that is why `DEPTH` is confident

`spectral_gap` is documented as "the smallest nonzero eigenvalue of the
normalized Laplacian" (`spectral.rs:637`). On a connected graph that is what it
returns. On a disconnected one it returns `0.0` from the guard at
`spectral.rs:358-360`, **before any eigenvalue is computed**.

Those are not the same quantity. §V-2 measures the gap concretely: for five
cliques of size two, the operator's smallest non-zero eigenvalue is `2.0`, and
the engine returns `0.0`. The returned `0.0` does not approximate `2.0`; it
means "undefined," encoded as a legal value of the same type.

That encoding is the root of the audit's `DEPTH` hazard, and it is worth
separating from the missing gate. `DEPTH` is not failing to check a
precondition it could have checked — it is reading a sentinel as a measurement,
and there is nothing in the value's type that distinguishes them. A caller doing
everything right still cannot tell "λ₁ is genuinely near zero, this manifold is
degenerate" from "λ₁ is undefined here."

This is the same defect shape catalogued in `TDD_DUR` §6 — a success value
computed from reaching the end of the function rather than from the result — and
it is the fifth instance in this codebase in a fortnight. Which is why F-6 is
specified as a refusal that **names the component count** rather than as a
boolean guard: the fix is to stop returning "undefined" in the same channel as
"measured," not merely to check more carefully before returning it.

### 2.7 Nulls and NaNs are isolated vertices — and `Value` has two disagreeing definitions of equality

§2.1 requires both endpoints non-null. v1 stated that and then ignored its
consequence in the §2.3 table (Hallie). The consequence is that the gate's
behaviour on real data is decided by missingness, not by field count.

**Nulls.** `add_index` skips `Value::Null` (`bundle.rs:4206-4209`), so a null
record joins no bucket and is an isolated vertex. One indexed field with a
single distinct value over `n` records of which `m` are null gives `K_{n−m}`
plus `m` singletons — `components = 1 + m`, so F-6 refuses for any `m ≥ 1`.

**The bucket-keying defect, restated after review 2.** v2 wrote this up as "NaN
breaks `Eq`" and concluded that "every `HashMap<Value, _>` inherits the defect
and every `BTreeMap<Value, _>` does not." The first half is right, the second is
**false**, and the generalisation was wrong in a way that would have sent the
follow-up fix in one direction when the defect points in two.

The actual defect: **`Ord` and `PartialEq` are two independent, disagreeing
definitions of equality on `Value`.**

- `PartialEq` is derived (`types.rs:24`), so it is IEEE on floats and
  false across variants.
- `Hash` (`types.rs:88-107`) hashes the discriminant, then `to_bits()` for floats.
- `Ord` (`types.rs:46-86`) is hand-written: `total_cmp` for floats, cross-type
  numeric arms at 68-69, and a `_ => type_order(self).cmp(&type_order(other))`
  fallthrough at 82 that catches every pair with no explicit arm.
- `impl Eq for Value {}` (`types.rs:38`) asserts that `PartialEq` is an
  equivalence relation. It is not reflexive on `Float(NaN)`.

Three known instances, pointing in **two** directions. All measured
(`tests/tmp_nan_value_contract.rs`):

| case | `PartialEq` | `Ord` | broken container | symptom |
|---|---|---|---|---|
| `Float(NaN)` vs itself | not equal | `Equal` | `HashMap` | leaks an unreachable entry per insert |
| `Integer(1)` vs `Float(1.0)` | not equal | `Equal` | `BTreeMap` | silently **overwrites** |
| `Binary(a)` vs `Binary(b)` | by content | **always `Equal`** | `BTreeMap` | **all** `Binary` keys collapse to one |

```
Float(NaN)  vs itself : cmp=Equal eq=false
  HashMap  1 NaN key  -> len=1, get(NaN) -> None
  HashMap  2 NaN keys -> len=2
  BTreeMap 2 NaN keys -> len=1

Integer(1) vs Float(1.0) : cmp=Equal eq=false
  BTreeMap both -> len=1, surviving value "float"
  HashMap  both -> len=2

Binary([1,2,3]) vs Binary([9,9,9]) : cmp=Equal eq=false
  BTreeMap 2 distinct Binary keys -> len=1
```

The `Binary` row is the worst of the three and has nothing to do with floats:
there is no `(Binary, Binary)` arm, so the fallthrough compares `7` against `7`
and every pair of `Binary` values is `Equal` under `Ord` regardless of content.
Any `BTreeMap` keyed on `Binary`, and any sort over `Binary`, treats them all as
one.

**For the field index specifically.** `field_index` is
`HashMap<String, HashMap<Value, RoaringBitmap>>` (`bundle.rs:3298`), so it is on
the `HashMap` side: each NaN-valued record becomes its own singleton bucket,
hence an isolated vertex, hence `components = n` for a NaN-heavy field, and the
index leaks an entry per record. Cross-type numerics bucket *separately* there
(`HashMap` keeps them apart), which is probably the intended semantics — but it
is the opposite of what a `BTreeMap`-backed range index would do, and
`types.rs:417` documents `indexed_fields` as "indexed for **range queries**."
V-8's sibling fixture pins which semantics the index actually has, because the
doc comment and the container disagree about it.

**Out of scope for this spec, and the defect record must carry the general form,
not the NaN row.** The NaN case is the least likely of the three to occur; the
cross-type and `Binary` cases need no special float values at all. V-7 and V-8
in §6 pin the indexing symptoms so the numbers are on record; they do not repair
the cause.


---

## 3. WHAT IS BROKEN

Five defects. Each was verified today; D-2 and D-4 were verified by execution,
not by reading.

### D-1 · **No schema mutation writes a WAL entry**
**Rescoped after Hallie's second review; v1 and v2 both titled this
"`add_index` writes no WAL entry," which is true and under-scoped.** All three
schema mutators journal nothing:

| handler | lines | logs? |
|---|---|---|
| `add_index` | `gigi_stream.rs:11051-11073` | no |
| `add_field` | `gigi_stream.rs:11010-11048` | no |
| `drop_field` | `gigi_stream.rs:10978-11007` | no |

Measured: `grep -c 'wal\.'` over `gigi_stream.rs:10970-11075` returns **0**.
Each takes `engine_write()`, mutates the store, and returns success.

The under-scoping was not cosmetic — it put a permanently-red test in §5. See
F-2's "every schema mutation logs" note and T-IDX-15.

`indexed_fields` **is** serialised — but only as a member of a schema payload
(`wal.rs:1529-1530` encode, `wal.rs:1639` decode), which is written by
`CreateBundle` and never again.

### D-2 · The engine's schema map never learns about the index
`create_bundle` (`engine.rs:1325-1327`):

```rust
let store = BundleStore::new(schema.clone());
self.bundles.insert(schema.name.clone(), store);
self.schemas.insert(schema.name.clone(), schema);
```

The store holds a **clone**; `Engine::schemas` (`engine.rs:384`) holds the
original. Two independently owned values, no `Arc`. `bundle_mut`
(`engine.rs:1770-1779`) hands back the store, so `add_index` mutates the store's
copy and `Engine::schemas` is untouched.

This is worse than D-1 alone, because `compact_wal_to_schemas` re-emits
`CreateBundle` from `Engine::schemas`. A compaction therefore writes the **stale,
pre-index schema** back over the WAL, making the loss permanent rather than
merely un-replayed.

**Verified by execution** (`tests/tmp_index_persistence.rs`, uncommitted):

```
after add_index, store schema : ["tag"]
after restart,  store schema  : []
```

— with an explicit `snapshot()` between the two.

### D-3 · `add_index` does not invalidate the geometry cache
`spectral_gap_cached` (`bundle.rs:4714-4730`) memoises into
`Mutex<Option<SpectralGapSnapshot>>` with **no key at all**. `mark_mutated`
clears it; `add_index` (`bundle.rs:4196`) never calls `mark_mutated`.

Four sites call `mark_mutated` today (`bundle.rs:1385, 1485, 1861, 1972`). Five
mutators do not:

| mutator | line @ `dd92ef8` | audit's line |
|---|---|---|
| `add_index` | 4196 | 4176 |
| `add_field` | 4110 | 4090 |
| `drop_field` | 4143 | 4123 |
| `truncate` | 3465 | 3445 |
| `bulk_delete` | 3437 | 3417 |

The audit's observable — `/depth` reporting `λ₁ = 0.0342` while `/spectral_gap`
reports `λ₂ = 0.0` at an unchanged `cached_at` — is this defect. Note the shape:
**the one mutation that changes the graph the eigenvalue is defined on is the
one mutation that does not clear the eigenvalue.**

### D-4 · On mmap bundles, `add_index` reaches only the overlay
`OverlayBundle::add_index` (`mmap_bundle.rs:1348-1352`) takes the overlay write
lock and calls `add_index` on the overlay's `BundleStore`. The mmap base is not
touched. Every production bundle is mmap-resident, so an index added there
covers only records written since the last snapshot.

Scope qualifier, stated so this is not over-claimed: `/spectral_gap` currently
returns `501 NOT_IMPLEMENTED` for mmap-resident bundles
(`gigi_stream.rs:3956-3971`), so this defect is not *today* a stale-geometry
path on that endpoint. It is a correctness hole in the index itself, and it
becomes a geometry path the moment the polymorphic-`BundleRef` follow-up lands.

### D-5 · No `bundle_version`, so nothing downstream can detect the change
Per §2.5. Not a bug in isolation; it is the missing mechanism that makes D-1
through D-4 undetectable from outside.

### The class
`add_index` is not special. It is one of **17** `bundle_mut` call sites in
`gigi_stream.rs` (2162, 10554, 10601, 10657, 10702, 10827, 10863, 10949, 10984,
11032, 11057, 11416, 11553, 11753, 12351, 13094, 16440), which mutate bundle
state through a door with none of the discipline the record path now has.
`TDD_DUR` §5 already flagged two of them (`truncate_bundle`, `ttl_eviction_task`)
as WAL-bypass mutations. `W-IDX-2` audits the rest.

---

## 4. THE FIX

### F-0 · Write ordering, atomicity, and the crash window

**Missing entirely from v1 (Hallie), and it is the one omission that would have
produced a wrong implementation from a spec that reads correct.** `add_index`
after F-2/F-3 is a three-part update — WAL append, `Engine::schemas` write,
store mutation — and v1 specified all three parts and none of their ordering.

**Ordering: log before apply.** The WAL append must complete before either
in-memory structure is touched:

```
1.  wal.log_create_bundle(updated_schema)      append
2.  wal.sync()                                 fsync boundary
3.  self.schemas.insert(name, updated_schema)  engine map
4.  store.add_index(field)                     store schema + index build
5.  store.mark_mutated()                       cache invalidation (F-1)
```

The direction matters and is not symmetric. INV-I is written as "what disk
reconstructs must equal what RAM served," so the survivable failure is **disk
ahead of RAM** — a crash between 2 and 4 leaves a journalled index that replay
rebuilds, which is correct. The unsurvivable one is **RAM ahead of disk**: apply
first and a crash between 4 and 1 leaves an engine that served an indexed
geometry whose index no longer exists. That is a direct INV-I violation and it
is what the obvious implementation produces, because `store.add_index` is the
line that already exists and the WAL call is the line being added.

**fsync boundary.** `Engine::insert` does not sync per-op; `sync` fires from
`checkpoint` every `checkpoint_interval` ops and from `batch_insert`
(`TDD_DUR` §5). An index declaration is not a high-rate operation and its loss
window should not be `checkpoint_interval` records wide, so step 2 syncs
unconditionally. This is a deliberate asymmetry with the record path and the
cost is one fsync per `add-index` call, which is a route no production traffic
currently exercises at all.

**Atomicity.** Steps 1–5 must hold the engine write lock for their whole extent.
`compact_wal_to_schemas` reads `Engine::schemas` and re-emits from it; a
compaction interleaved between 3 and 4 would emit a schema whose index the store
does not yet have — harmless in that direction, but the reverse interleaving is
not, and the invariant should not depend on which way the race falls. The HTTP
handler already takes `state.engine_write()` (`gigi_stream.rs:11056`), so this
is a constraint to preserve rather than to add, and `T-IDX-13` pins it.

**The one non-crash failure.** Step 4 (`store.add_index`) can fail on its own —
an allocation failure during the index build, say — after step 2 has already
synced. That leaves disk ahead of RAM, which is the survivable direction: the
next restart replays the entry and builds the index. The call should still
return an error to its caller, so the declaration is unacknowledged and INV-I's
scope covers the discrepancy until the next boot resolves it.

**Crash-window statement.** After F-0 the only window is between 1 and 2, and it
loses an unacknowledged index declaration — which is why INV-I is scoped to
acknowledged state in §1.

### F-1 · `mark_mutated()` in the five mutators — but they are two classes

All five need the invalidation call. Only three of them need journalling, and v2
obscured that by treating the five as one group for F-1 and then naming only
`add_index` in F-2 (Hallie):

| class | mutators | needs |
|---|---|---|
| **schema** | `add_index`, `add_field`, `drop_field` | F-1 **+ F-2 journalling + F-0 ordering** |
| **record** | `truncate`, `bulk_delete` | F-1 only; their WAL story is `TDD_DUR` §5's WAL-bypass item, not this spec's |

One line each, at `bundle.rs` 4196, 4110, 4143, 3465, 3437, matching the
`batch_insert` precedent, whose call sits at `bundle.rs:1485` inside the
function beginning at 1470. (v1 cited 1470 here and 1485 in D-3's list of
existing call sites; both are correct and the inconsistency was noise in a
document that opens by claiming every reference was verified. Call sites are
cited throughout; function starts are named as such.) Closes invalidation.
Cheapest item
here and the only one the audit needs to unblock its indexing work.

### F-2 · Journal the index change by re-emitting `CreateBundle`
**Design decision, and the reason this fix is small.** A new WAL op code
(`0x06`) would require a format-compatibility story. It is not needed:
`CreateBundle` replay is already last-write-wins into `schemas`
(`engine.rs:915`, `engine.rs:649`), and `compact_wal_to_schemas` already
re-emits one per schema. So `add_index` logs a `CreateBundle` carrying the
updated schema. No new op, no format version bump, no reader change.

**The one arm that must change.** `do_replay` (`engine.rs:911-916`):

```rust
WalEntry::CreateBundle(schema) => {
    bundles
        .entry(schema.name.clone())
        .or_insert_with(|| BundleStore::new(schema.clone()));
    schemas.insert(schema.name.clone(), schema);
}
```

`or_insert_with` is what makes the re-emit safe — an existing store keeps its
records, so a second `CreateBundle` cannot wipe data. It is also what makes the
re-emit insufficient as-is: the existing store's schema is never updated, so the
index would not be rebuilt on replay.

**Every schema mutation logs, not just `add_index` (Hallie, review 2).** v2
specified journalling for `add_index` alone while simultaneously adding
T-IDX-15 ("an index removed through `drop_field` stays removed across a
restart"). Those two cannot both hold. `drop_field` journals nothing (D-1), so
the newest `CreateBundle` on disk still carries the dropped index; replay
computes `new − old` against that stale payload, sees the index present, and
**restores it**. T-IDX-15 would have been red with F-2 applied *and* red with
the mechanism removed — permanently red, for a reason F-2 did not address,
which is precisely the "passes/fails for the wrong reason" failure §5's preamble
is written against. It was caught in review rather than in implementation only
because someone read the test against the fix.

So F-2 covers `add_index`, `add_field` and `drop_field`, each emitting the
updated schema, and F-0's ordering applies to all three.

**The delta must be symmetric, not add-only (Hallie).** v1 specified "call
`add_index` for each field in the new `indexed_fields` not already present,"
which is monotone. `indexed_fields` is **not** monotone: `drop_field` removes
from it (`bundle.rs:4156`, `indexed_fields.retain(|f| f != field_name)`). Under
add-only replay a dropped index is journalled in the schema payload and never
applied, so it **returns after a restart** — the mirror image of D-2, and a
defect the fix would have introduced. The arm is:

```
absent  -> create from schema
present -> for f in new.indexed_fields - old.indexed_fields:  store.add_index(f)
           for f in old.indexed_fields - new.indexed_fields:  store.drop_index(f)
```

`drop_index` does not currently exist as a public operation (`grep 'fn drop_index'`
→ zero hits); only `drop_field` removes from `indexed_fields`, and it removes the
field too. F-2 therefore needs an index-only removal path, which is new surface
and is why `W-IDX-1` is not a one-line change.

**Ordering within replay is not the same problem as F-0.** Replay is
single-threaded and rebuilds from a known-durable log, so the log-before-apply
constraint does not apply to it; what matters is that `add_index` rebuilds from
whatever records the store holds at that moment (`bundle.rs:4202-4212`).
Verified sufficient on the heap path: `do_replay` sets the schema at
`CreateBundle` (911-916) and loads snapshot records through `store.batch_insert`
at the `Checkpoint` arm (`engine.rs:931`), which maintains declared indexes. So
schema-then-records is the order that already happens.

**Idempotence:** `add_index` returns early when the field is already indexed
(`bundle.rs:4197-4199`), so N replays of the same entry are equivalent to one.
Hallie flagged that this early return is only load-bearing if `indexed_fields`
is a `Vec` rather than a set. Checked: `pub indexed_fields: Vec<String>`
(`types.rs:418`), and the guard is `Vec::contains`. T-IDX-7 is valid as written.
The adjacent risk she names is real and separate — unconditional WAL logging on
a repeated `add_index` would grow the log without changing state, so F-2 must
log only on an actual delta.

**`CreateBundle` becomes semantically overloaded, and that should be said out
loud (Hallie).** After F-2 the op no longer means "a bundle was created"; it
means "assert that this is the schema." That is a defensible design — it is what
makes the no-format-bump argument work, and `compact_wal_to_schemas` already
emits it that way — but anything that counts or timestamps bundle-creation
events by scanning the WAL will now see duplicates and infer re-creations that
did not happen. Nothing in the tree does that today (`grep` for consumers of
`WalEntry::CreateBundle` outside replay and compaction returns none), so this is
a note for future readers rather than a defect. If a creation-time audit is ever
wanted, it needs its own op or a flag on this one.

**The mmap boot path needs its own answer, and v1 did not give one.**
`grep 'add_index\|rebuild_index\|build_index' src/engine.rs` returns **zero
hits**: no boot path rebuilds an index explicitly. The heap path gets away with
it because `batch_insert` maintains indexes incrementally. `open_mmap` does not
route base records through `batch_insert` — they stay in the mmap — so a
declared index over an mmap-resident bundle has nothing that builds it over the
base. F-4 and F-2 meet here, and the honest statement is that **index
persistence for mmap-resident bundles is unspecified in this document**; it is
`W-IDX-5`, it is the case that covers all of production, and it should not be
described as fixed by F-2.

### F-3 · Keep `Engine::schemas` coherent
`add_index` must write the updated schema into `Engine::schemas`, not only the
store's clone. Without this, F-2's re-emit reads a stale schema and compaction
still buries the index (D-2).

The durable fix for the class is to remove the divergence rather than patch each
writer: make the schema single-owned and have the store borrow it. That is a
larger change than this spec funds; it is named in `W-IDX-4` and the interim is
an explicit write plus `T-IDX-5`, which fails if the two copies disagree.

### F-4 · Make `add_index` cover the mmap base
`OverlayBundle::add_index` must index base records, not only the overlay. The
cheap correct form is to route the index build through the same merged view the
read paths use rather than the overlay's own store.

### F-5 · `bundle_version = H(records, index_set)`
Introduce a content hash over the pair, exposed on the endpoints that return
geometry, and stamped into anything that persists a λ value. This is the piece
that makes a stale answer *detectable* rather than merely *prevented*, which is
what a diagnosis product needs when its outputs get quoted in a report.

### F-6 · Make "undefined" unrepresentable, then gate one caller on it

**Rewritten after review.** v1 diagnosed a type problem in §2.6 and then
prescribed a lint: gate `DEPTH`, while `spectral_gap` kept handing `0.0` to
everyone else as a legal `f64`, and §8 conceded `HORIZON`/`BETTI`/`SPECTRAL_GAP`
were unaddressed. Hallie is right that this does not follow from the diagnosis.
If the defect is that "undefined" travels in the same channel as "measured,"
the fix is the channel.

```rust
pub enum SpectralGap {
    Measured(f64),
    Undefined { components: usize, records: usize },
}
```

`spectral_gap` returns this. The guard at `spectral.rs:358-360` becomes the
`Undefined` constructor rather than an early `return 0.0`, carrying the count it
already computed. Every caller then has to say what it does with `Undefined`,
and the compiler enumerates them — which is the same reason `TDD_DUR`'s W3
chose a type over a review checklist.

`DEPTH` is then one consumer: it refuses, and the refusal **names the
condition**. The message must be accurate about the mathematics, which v1's was
not: λ₁ is defined on any graph with at least one edge. What is true is that it
equals the algebraic connectivity only on a connected graph, and is identically
zero otherwise, so on a disconnected graph it carries no information about
connectivity at all. Wording:

> insufficient structure: the field-index graph has `k` components over `n`
> records, so λ₁ is identically 0 and is not a measurement of this bundle

**Cost note.** The gate condition is `components_from_index(store).len() == 1`.
Hallie raised the possibility that this is quadratic in bucket size, since a
clique on a bucket of size `b` has `b²/2` edges. Checked: `components_from_index`
(`spectral.rs:211-269`) is union-find over `(record, bucket)` incidences —
it unions each bucket's members to that bucket's first member and never
materialises an edge, so it is `O(n·|F|·α)`. The cheap form she asks for is
already what is there. The quadratic risk is real but lives in
`field_index_graph`, on the eigenvalue path, which `TDD-SBF` already replaced
with an implicit clique operator (`spectral.rs:306+`) for exactly this reason.
F-6 adds no asymptotic cost.

---

## 5. TDD PLAN

Every test below is written before its fix and must be observed **red**. Every
test also carries a mechanism-removal step: revert the named fix, re-run, and
confirm the test goes red again. A test that passes both with and without its
fix tests nothing, and this is the discipline that caught the two live gates in
`TDD_DUR` — it is not ceremony.

| id | asserts | red without | mechanism-removal |
|---|---|---|---|
| **T-IDX-1** | after `add_index`, `/depth` and `/spectral_gap` agree on the spectrum at one instant | F-1 | drop `mark_mutated()` from `add_index` → red |
| **T-IDX-2a** | after `add_index`, the cache is **invalidated** — `spectral_gap_cache` is empty and `mutation_counter` advanced | F-1 | as T-IDX-1 |
| **T-IDX-2b** | on a fixture where the value is *proven* to change (0 fields → 1 field, `v = 1`, so `undefined → n/(n−1)`), the recomputed value differs | F-1 | as T-IDX-1 |
| **T-IDX-3** | the other four mutators (`add_field`, `drop_field`, `truncate`, `bulk_delete`) each invalidate | F-1 | drop each call individually → four separate reds |
| **T-IDX-4** | an index added through `bundle_mut` survives `Engine::open` — **this is the currently-failing probe** | F-2 + F-3 | revert `do_replay` arm → red |
| **T-IDX-5** | `Engine::schemas[name].indexed_fields == store.schema().indexed_fields` after `add_index` | F-3 | revert the `schemas` write → red |
| **T-IDX-6** | an index survives a **compaction**, not merely a restart — the D-2 path | F-2 + F-3 | revert F-3 → red (compaction re-emits stale) |
| **T-IDX-7** | replaying the same `CreateBundle` N times leaves records and `indexed_fields` unchanged (idempotence) | F-2 | remove the early-return at `bundle.rs:4197` → red |
| **T-IDX-8** | a re-emitted `CreateBundle` for a populated bundle does **not** drop records | F-2 | change `or_insert_with` to `insert` → red |
| **T-IDX-9** | on an mmap-backed bundle, `add_index` covers base records, not only overlay | F-4 | revert F-4 → red |
| **T-IDX-10** | `bundle_version` changes when `index_set` changes and records do not | F-5 | key the hash on records alone → red |
| **T-IDX-11** | `DEPTH` refuses on a 0-indexed-field bundle, and the refusal names the component count | F-6 | remove the gate → red |
| **T-IDX-12** | `DEPTH` **answers** on a 1-indexed-field bundle whose field has a single distinct value (`K_n`, connected) | F-6 | implement the gate as `|F| >= 2` → red |

| **T-IDX-13** | `add_index` holds the engine write lock across WAL append, `schemas` write, and store mutation — a compaction cannot interleave | F-0 | release the lock between steps 2 and 4 → red under a concurrent compaction |
| **T-IDX-14** | killing the process between the WAL append and the store mutation yields, on restart, an engine **with** the index — disk ahead of RAM is survivable | F-0 | reorder to apply-before-log → red |
| **T-IDX-15** | an index **removed** through `drop_field` stays removed across a restart | F-2 **journalling for `drop_field`** + symmetric delta | implement the replay delta as add-only → red |
| **T-IDX-16** | a repeated `add_index` for an already-indexed field appends **no** WAL entry | F-2 | log unconditionally → red (WAL grows without a state change) |

**Notes on three of these.**

T-IDX-2 was split after review. v1 asserted only that the cached value *differs*
after `add_index`, which is fixture-dependent in a way that hides the bug: going
from 0 indexed fields to 1 field with `v ≥ 2` takes the engine's return from
`0.0` (no edges) to `0.0` (disconnected cliques) — same number, changed graph,
so the test passes or fails on the fixture rather than on the mechanism.
Invalidation and value-change are now separate assertions, and the value-change
one runs only on a fixture where §2.3 proves the value moves.

T-IDX-7's mechanism-removal is valid, but only because `indexed_fields` is a
`Vec<String>` (`types.rs:418`) and the guard is `Vec::contains`. Had it been a
`HashSet`, adding a duplicate would be idempotent regardless of the early return
and the test would stay green with the mechanism removed — the exact failure this
section's preamble warns about. Checked rather than assumed, on review.

T-IDX-11 and T-IDX-12 are a matched pair and must be written together. T-IDX-12
is the one that catches the plausible-but-wrong implementation of F-6, and it is
the reason §2.3 exists.

T-IDX-15 was **permanently red as written in v2** and the review caught it
before implementation did. v2 paired it with the symmetric-delta fix alone,
while F-2 journalled only `add_index` — so `drop_field` would log nothing, the
newest `CreateBundle` on disk would still carry the dropped index, and replay
would restore it no matter how the delta was computed. Red with the fix, red
without it: not a mechanism-removal pair, just a broken test. It passes only
once F-2 covers all three schema mutators. The general lesson is worth keeping
alongside the preamble: a test can fail the two-sided check by being red on
both sides as easily as by being green on both.

T-IDX-14 is the only test here that requires killing a process mid-operation. If
that is impractical in the harness, the fallback is a seam: a test hook that
returns after the WAL sync, exercised by a unit test that then opens a fresh
engine over the same directory. A seam is weaker evidence than a real kill and
should be labelled as such in the test name.

---

## 6. MATH VALIDATION BATTERY

The tests in §5 prove the plumbing. These prove the engine computes the right
number, against closed forms rather than against its own prior output. All
fixtures are ordinary bundles driven through the real index mechanism, per the
Lemma in §2.4 — no test-only graph injection.

Throughout: `n` = record count, `L` = normalized Laplacian. Where v1 wrote "λ₁"
this revision distinguishes **true λ₁** (the operator's smallest non-zero
eigenvalue, which may not exist) from **what the engine returns**, per §2.3.

### V-1 · Complete graph `K_n` — exact
One indexed field, one distinct value, `n` records, no nulls. Every record in
one bucket.

```
L(K_n) spectrum = { 0,  n/(n−1) with multiplicity n−1 }
true λ₁ = n/(n−1)
```

| `n` | expected |
|---|---|
| 3 | `3.0 / 2.0` |
| 4 | `4.0 / 3.0` |
| 5 | `5.0 / 4.0` |
| 10 | `10.0 / 9.0` |

**Compare against the expression, not a decimal literal (Hallie).** `4/3` and
`10/9` are not representable in binary floating point, so a bitwise test against
`1.333333` fails and one against `1.3333333333333333` is guessing at the
rounding. Compute `(n as f64) / ((n - 1) as f64)` exactly as the fast path does
(`spectral.rs:363`) and compare to that. Bitwise is correct **here** because
that path returns a constant without iterating; a tolerance would hide a
regression that replaced the constant with a solve.

### V-2 · Disjoint cliques — the theorem, and the sentinel
One indexed field with `v` distinct values, buckets of size `g`, `n = v·g`.

```
components                        = v          (Theorem, §2.2)
dim ker L                         = v
smallest non-zero eigenvalue of L = g/(g−1)    (each block is K_g)
what the engine returns           = 0.0        (guard, spectral.rs:358)
```

The engine's return value and the operator's smallest non-zero eigenvalue are
different numbers here. `0.0` is not a measurement of this graph; it is the
guard firing. The real spectrum has a `v`-fold kernel and then a jump to
`g/(g−1)`.

Assert all three: the engine returns `0.0`; `components_from_index` returns `v`;
`dim ker L = v`.

**Mechanism-removal — corrected 2026-08-15 (Hallie).** v1 claimed that deleting
the guard yields `g/(g−1)` — 1.5, 2.0, 1.333 — and called this "a two-sided gate
in one fixture." That is wrong, and how it was wrong is worth recording: those
are the *operator's* eigenvalues, computed in numpy. They are not what the
engine returns with the guard deleted, and v1 asserted them as though the second
thing had been measured. It had not.

Re-measured by simulating the engine's own solver — power iteration deflating
the **single** vector `u = D^(1/2)·1` (`spectral.rs:717-721`) against a
`v`-dimensional kernel:

| `v` | `g` | operator's smallest non-zero | engine, guard deleted |
|---|---|---|---|
| 2 | 3 | 1.500000000000 | **0.000000000000** |
| 5 | 2 | 2.000000000000 | **1.615361810194** |
| 3 | 4 | 1.333333333333 | **0.000000000000** |

Deflating one direction out of a `v`-dimensional kernel leaves `v−1` kernel
vectors in play, so the iteration falls into the kernel and returns ≈0 — or,
when the random start happens to carry little mass there, stops at an arbitrary
intermediate value. The `1.615` is not an eigenvalue of anything; it is where
that run happened to stop.

The guard-deleted output is therefore **unstable, not merely different**, and no
numeric assertion on it is sound — including `≠ 0.0`, which the `v = 2` row
already falsifies. V-2's mechanism-removal asserts on
`components_from_index(store).len() == v`, which is deterministic, union-find,
and independent of the solver.

This is also the strongest argument for F-6 being a type change rather than a
gate: with the guard removed the function returns a plausible float derived from
nothing, and nothing in its type says so.

### V-3 · Cycle `C_n` — the bridged case, under tolerance
Two indexed fields, all buckets of size 2, `n` even, `n ≥ 4`:

- field `a`: buckets `{0,1}, {2,3}, …, {n−2,n−1}`
- field `b`: buckets `{1,2}, {3,4}, …, {n−1,0}`

Edge-disjoint for `n ≥ 4` (§2.4 hypothesis (iii)), so the union is 2-regular and
`L_norm = L_comb / 2`:

```
L_comb(C_n) eigenvalues = 2 − 2cos(2πk/n)
L_norm      eigenvalues = 1 − cos(2πk/n)
true λ₁ = 1 − cos(2π/n)
```

| `n` | expected | multiplicity of λ₁ | gap to next distinct |
|---|---|---|---|
| 4 | 1.0 | 2 | 1.0 |
| 6 | 0.5 | 2 | 1.0 |
| 8 | 0.292893218813 | 2 | 0.707106781187 |
| 12 | 0.133974596216 | 2 | 0.366025403784 |

This is the fixture that catches a normalisation error (V-6).

**Tolerance, actually derived.** v1 wrote `max(10 · residual, 1e-9)` and called
it derived; both constants were chosen, which is the thing that paragraph
attacks. The correct statement uses the symmetric perturbation bound:

> For symmetric `L`, unit `v`, and Rayleigh quotient `μ = vᵀ L v`, let
> `r = ‖L v − μ v‖`. Then **some** eigenvalue of `L` lies within `r` of `μ`
> (Weyl; Bauer–Fike in the symmetric case). To conclude it is the *intended*
> eigenvalue, `r` must be less than half the gap to the nearest other
> **distinct** eigenvalue.

Stated as a chain rather than a pair, since v2's phrasing read circularly
(Hallie): measure `μ` and `r`; the bound puts **some** eigenvalue within `r` of
`μ`; if `r < gap/2` that eigenvalue is the unique one in reach, hence
identified; therefore `|μ − λ_closed| ≤ r` confirms it is the intended one.

```
1.  r < gap/2                          precondition, gap from the table
2.  |λ_measured − λ_closed| <= r        conclusion
```

If (1) fails, the test fails as **inconclusive**, not as wrong: the solver did
not converge far enough to identify which eigenvalue it found, which is a
different defect from computing the wrong one and must not be reported as the
same thing.

**On degeneracy.** Hallie notes `C_n`'s λ₁ is a double eigenvalue (`k` and
`n−k` coincide) and reads that as unsoundness. Partly accepted. Multiplicity of
λ₁ does **not** threaten the asserted value — power iteration converges to the
correct eigenvalue regardless; only the eigenvector is ambiguous within its
eigenspace. What governs convergence, and what the bound above requires, is the
gap to the next **distinct** eigenvalue, which is why that is the column in the
table rather than the multiplicity. Degeneracy would bite an eigenvector
assertion, and this battery makes none.

### V-4 · Empty index set
Zero indexed fields, `n` records.

```
E = ∅,  every vertex isolated,  dim ker L = n   (zero-row convention, §2.2)
engine returns 0.0 by guard
true λ₁ is UNDEFINED — L is the zero matrix and has no non-zero eigenvalue
```

Assert `components == n`, that the engine returns `0.0`, and that `DEPTH`
refuses (F-6). Do **not** assert "λ₁ = 0" as a mathematical claim; there is no
non-zero eigenvalue for it to be the smallest of.

### V-5 · Reproducibility across restart — INV-I, measured
Split into two assertions of different strength, which v1 conflated (Hallie).

**V-5a — discrete state, bitwise.** This is INV-I proper:

```
records(B)   before == after      exact
index_set(B) before == after      exact
```

**V-5b — the derived value, under V-3's tolerance.** Bitwise equality of λ₁
across a restart would require an identical starting vector, identical iteration
count, and identical floating-point summation order — hence identical record
ordering out of the store — and the two boot paths reconstruct through different
code (`engine.rs:911` vs `engine.rs:648`). One reordered record changes the last
ulp and the test fails spuriously.

V-5b therefore asserts `|λ₁(before) − λ₁(after)| ≤ r` under V-3's precondition.
V-1's fixture is the exception and may be compared bitwise, because its value is
a returned constant rather than a solve.

Solver determinism across boot paths may be worth having, but it is a
**separate invariant** with its own cost, and INV-I does not depend on it.

Run both on `Engine::open` and `Engine::open_mmap`. **Expect V-5 to stay red on
the mmap path until `W-IDX-5`** — see F-2's closing note; F-2 does not fix that
path and this spec should not imply otherwise.

### V-6 · Adversarial pass
Remove each mechanism, confirm the matching validation goes red.

- replace the `K_n` fast-path constant with `1.0` → V-1 red
- delete the component short-circuit at `spectral.rs:358` → V-2's **component**
  assertion red; its numeric assertion is deliberately absent, see V-2
- swap `L_norm` for `L_comb` (drop the `/k`) → V-3 red, V-1 **still green**
- make V-3's two matchings non-disjoint (pair the same records in both fields)
  → the union is 1-regular, `D ≠ 2I`, V-3 red. This is the fixture for §2.4
  hypothesis (iii), and without it the Lemma's added hypothesis is untested
- revert F-2/F-3 → V-5a red
- drop the null accounting → V-7 red

The third bullet is load-bearing: a normalisation error is invisible to `K_n`,
which returns a constant, and visible to `C_n`. A battery containing only `K_n`
would certify a wrong Laplacian.

### V-7 · Nulls are isolated vertices
One indexed field, one distinct value, `n` records of which `m` hold
`Value::Null` in that field.

```
graph      = K_{n−m}  plus  m isolated vertices
components = 1 + m
F-6 refuses for every m >= 1
```

Assert the component count as a function of `m`, for `m ∈ {0, 1, 3}`. `m = 0`
must reduce exactly to V-1 — that reduction is what makes this a null-accounting
test rather than a second copy of V-1.

### V-8 · NaN buckets are singletons, and the index leaks one entry each
Same shape as V-7 with `Value::Float(f64::NAN)` in place of the nulls.

```
components = 1 + m        each NaN record is its own bucket
field_index gains one UNREACHABLE entry per NaN record
```

Assert both the component count and that the field's index-map entry count is
`1 + m` rather than `1`. The second assertion is what separates "NaN is treated
as its own value" from "NaN leaks a bucket per record"; per §2.7 the tree does
the latter, and `HashMap::get` on a NaN key returns `None` immediately after
insertion.

This fixture fails until the `Value` `Eq`/`Hash` contract is repaired, which is
**out of scope here**. It is included so the failure is pinned to a number now
rather than discovered from a memory graph later.

### V-9 · Additional exact fixtures — coverage
V-3 was the only fixture exercising a bridged graph, on one family, so V-6's
third bullet rests on a single test. Two more are reachable through the same
index mechanism, both regular, both verified numerically:

| graph | construction | `k` | true λ₁ | multiplicity |
|---|---|---|---|---|
| `Q₃` (3-cube) | 3 edge-disjoint perfect matchings on 8 records | 3 | `2/3` | **3** |
| **prism `K₃ □ K₂`** | 3 edge-disjoint perfect matchings on 6 records | 3 | `2/3` | **1** |
| `K₃,₃` | 3 edge-disjoint perfect matchings on 6 records | 3 | `1.0` | 4 |
| `K₄,₄` | 4 edge-disjoint perfect matchings on 8 records | 4 | `1.0` | 6 |

The bipartite three decompose by König's edge-colouring theorem (every
`k`-regular bipartite graph is a disjoint union of `k` perfect matchings). The
prism is not bipartite and needs its decomposition exhibited; Hallie supplied it
and it verifies — with vertices `a₀a₁a₂` (triangle), `b₀b₁b₂` (triangle), rungs
`aᵢbᵢ`:

```
M1 = {a0a1, b0b1, a2b2}
M2 = {a1a2, b1b2, a0b0}
M3 = {a2a0, b2b0, a1b1}
```

Each is perfect (covers all 6), the three are edge-disjoint, and their union is
exactly the prism's 9 edges. Three indexed fields, three buckets of two.

**`{Q₃, prism}` is the sharpest pair in the battery.** They share λ₁ = `2/3` and
differ only in eigenspace dimension — 3 against 1. A solver that reported
something about the eigenspace rather than the eigenvalue would separate them;
one that reports the eigenvalue cannot. No other pair here isolates that.

**Retracting a generalisation (Hallie, review 2).** v2 closed this section with
"they are all vertex-transitive, and vertex-transitivity forces multiplicity."
That is **false**, and the prism is the counterexample: 3-regular,
vertex-transitive (`|orbit(v₀)| = 6` of 6, verified by brute-force automorphism
search), λ₁ = `2/3` with multiplicity **1**. The claim came from noticing that
the three fixtures v2 happened to list were all degenerate and inventing a
reason. Vertex-transitivity constrains the *eigenspaces* to be modules over the
automorphism group; it does not force any of them to have dimension above one.

The half of v2's note that survives is the half that was actually argued:
multiplicity does not threaten the asserted **value** — power iteration converges
to the correct eigenvalue regardless, only the eigenvector is ambiguous — and
Hallie's original reason for proposing `Q₃` ("its λ₁ is simple") was itself
wrong, since `Q_d`'s eigenvalues are `2k/d` with multiplicity `C(d,k)`, giving
`C(3,1) = 3`. Both of us were wrong about `Q₃`, in opposite directions, and the
prism is what settles it.

## 7. ORDER OF WORK

**W-IDX-0 — F-1, the five `mark_mutated()` calls. Land alone, first.**
Five lines, no behaviour change on any correct path, ships with T-IDX-1/2/3. It
is what unblocks the E17 indexing work, and it is the only item here that has to
land before that work starts rather than alongside it.

**W-IDX-1 — F-2 + F-3, index durability.** The `do_replay` arm, the
`Engine::schemas` write, the `CreateBundle` re-emit. Ships with T-IDX-4 through
T-IDX-8 and V-5. This is the item that makes INV-I true.

**W-IDX-2 — audit the metadata door.** The 17 `bundle_mut` sites, against
**three** questions: is the mutation journalled, does it invalidate, and does it
diverge from `Engine::schemas`. v1 asked only the first two, which omits the
failure D-2 actually was — the store's schema clone drifting from the engine's
map — and that is the question whose answer sizes W-IDX-4, which is what this
item exists to determine. Output
is a table, not a fix — the point is to learn whether `add_index` was one bug or
a dozen before deciding how much machinery §4 deserves. Cheap, and it is the
input to W-IDX-4.

**W-IDX-3 — F-6, the refusal gate.** Ships with T-IDX-11 + T-IDX-12 + V-4.
Independent of W-IDX-1; can run in parallel.

**W-IDX-4 — F-5 and the schema-ownership fix.** `bundle_version`, plus removing
the store/engine schema divergence that F-3 works around rather than removes.
Sequenced last because W-IDX-2 determines its scope: if the divergence has one
victim, F-3 is the fix; if it has a dozen, single-ownership is.

**W-IDX-5 — F-4, mmap base coverage.** Sequenced with, or after, the
polymorphic-`BundleRef` follow-up, since that is what turns D-4 from an index
correctness hole into a geometry path.

W-IDX-0 removes the immediate blocker. W-IDX-1 establishes the invariant.
W-IDX-2 sizes everything after it.

---

## 8. WHAT THIS DOES NOT FIX

**The λ-verbs remain unavailable on mmap-resident bundles.** `/spectral_gap`
returns `501` for them (`gigi_stream.rs:3956-3971`), which is every production
bundle. Nothing here changes that; F-4 makes the index correct for the day it
does change.

**The E17 audit's statistical findings.** JACKKNIFE coverage, the Hotelling `T²`
conditioning collision, the depth-3 power question, the substrate-robustness
rebuild — none of those are engine defects and none are addressed here. They
belong to whoever owns `VERB_AUDIT.md`.

**Whether the day-shape bundle should have indexed fields at all.** The audit's
recommendation is quantised companion fields — a cluster letter, a volatility
tercile. That is a modelling decision about what relation deserves to be
measured, and §2.3's rule ("index a field when its induced adjacency *is* the
relation you want measured, never for convenience") is guidance, not a fix.

**The other λ-verbs' handling of `Undefined`.** This changed on review. v1 gated
`DEPTH` alone and listed `HORIZON` / `BETTI` / `SPECTRAL_GAP` here as
unaddressed — which Hallie correctly read as diagnosing a type problem and
prescribing a lint. F-6 is now the type change, so every consumer is forced by
the compiler to say what it does with `Undefined`, and none of them can silently
inherit a sentinel. What is still **not** specified here is what each of those
verbs should *decide* — refuse, degrade, or report separately — which is a
product question per verb and not one this document can answer. The compiler
will produce the list; someone has to walk it.

**The `Value` equality contract.** §2.7 measures the general defect: `Ord` and
`PartialEq` are two independent, disagreeing definitions of equality, with three
known instances pointing in **two** directions — `Float(NaN)` breaks `HashMap`,
while `Integer(1)` vs `Float(1.0)` and *any two* `Binary` values break
`BTreeMap`. v2 scoped this as "NaN breaks `Eq`" and asserted that `BTreeMap`
behaves correctly; both halves were wrong, and the second would have aimed the
follow-up fix at one container when the defect spans both.

It wants its own defect record carrying the general form. Two notes for whoever
takes it: the `Binary` case needs no unusual values at all — there is simply no
`(Binary, Binary)` arm — and `types.rs:417` documents `indexed_fields` as
"indexed for **range queries**," which is the ordered path, so the container the
index uses and the semantics its doc comment promises are not the same thing.
V-7 and V-8 pin the indexing symptoms; they do not repair the cause, and those
fixtures are expected red until someone does.

**Refusal batteries beyond this spec.** The E22 review's recommendation — build
the broken input, assert the guard fires, then delete the guard and assert the
test fails — is the general form, and at 307 verbs it wants its own scoping pass
rather than a clause here.

**Records.** Nothing in this spec touches the record write path. That path was
hardened separately and is gated by `tests/durability_wal_truncation.rs`; the
two specs are independent and INV-I extends INV-D's domain without weakening it.
