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
> engine served from RAM before it.

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
> edge weights, and let `L` be its normalized Laplacian restricted to the
> vertices of positive degree. Then
>
> ```
> dim ker L  =  number of connected components of G
> ```
>
> **Hypotheses, all required:** finite vertex set; undirected (symmetric `W`);
> non-negative weights; and the restriction to positive-degree vertices, since
> `D^(−1/2)` is otherwise undefined. The Def 3.9 graph satisfies all four by
> construction — it is a union of cliques on a finite record set, unweighted,
> and the isolated-vertex convention is fixed at `spectral.rs:2234`.

`spectral_gap_explicit` does not merely inherit this result — it **implements**
it, short-circuiting to `0.0` whenever the component count exceeds one
(`spectral.rs:358-360`), before any iteration runs.

### 2.3 The three regimes, and the correction the audit's phrasing needs

Let `F` be the indexed-field set and `v(f)` the number of distinct non-null
values of `f`.

| `|F|` | graph | components | λ₁ |
|---|---|---|---|
| 0 | no edges | `n` | 0 |
| 1, `v(f) = 1` | `K_n` | 1 | `n/(n−1)` |
| 1, `v(f) ≥ 2` | `v(f)` disjoint cliques | `v(f)` | 0 |
| ≥ 2 | union of clique covers | 1 iff the record-bucket incidence is connected | > 0 iff connected |

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

> **Lemma.** If every bucket of an indexed field `f` contains exactly two
> records, the clique `f` contributes is a perfect matching on the records it
> covers. Hence `k` indexed fields, each of whose buckets are pairs covering all
> `n` records, induce a union of `k` perfect matchings — a `k`-regular graph.
>
> **Corollary.** For a `k`-regular graph, `D = kI`, so
> `L_norm = L_comb / k`, and every closed-form combinatorial spectrum in the
> literature transfers by dividing by `k`.

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

---

## 3. WHAT IS BROKEN

Five defects. Each was verified today; D-2 and D-4 were verified by execution,
not by reading.

### D-1 · `add_index` writes no WAL entry
`POST /v1/bundles/{name}/add-index` (route `gigi_stream.rs:16603`, handler
`gigi_stream.rs:11051-11073`) calls `store.add_index(&req.field)` and returns
`{"status":"index_added"}`. No `wal.log_*` call on any path. The index exists
only in RAM.

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

### F-1 · `mark_mutated()` in the five mutators
One line each, at `bundle.rs` 4196, 4110, 4143, 3465, 3437, matching the
`batch_insert` precedent (`bundle.rs:1470`). Closes invalidation. Cheapest item
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
index would not be rebuilt on replay. The arm becomes: if the bundle is absent,
create it; if present, apply the schema delta by calling `add_index` for each
field in the new `indexed_fields` not already present. `BundleStore::add_index`
already rebuilds from existing records (`bundle.rs:4202-4212`), so replay order
does not matter.

**Idempotence:** `add_index` returns early when the field is already indexed
(`bundle.rs:4197-4199`), so N replays of the same entry are equivalent to one.

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

### F-6 · Refusal gate on a degenerate graph
`DEPTH` currently answers `level IV — topological encoding, infinite erasure
energy, the manifold topology has changed` with full confidence on a graph with
no edges. Per §2.3 the gate condition is:

```
components_from_index(store).len() == 1
```

and the refusal must **name the condition** — "insufficient structure: the
field-index graph has k components; λ is defined only on a connected graph" —
not return a maximally alarming interpretation of a zero.

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
| **T-IDX-2** | after `add_index`, `spectral_gap_cached()` returns a value differing from the pre-index value on a bundle whose graph actually changed | F-1 | as T-IDX-1 |
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

T-IDX-11 and T-IDX-12 are a matched pair and must be written together. T-IDX-12
is the one that catches the plausible-but-wrong implementation of F-6, and it is
the reason §2.3 exists.

---

## 6. MATH VALIDATION BATTERY

The tests in §5 prove the plumbing. These prove the engine computes the right
number, against closed forms rather than against its own prior output. All
fixtures are ordinary bundles driven through the real index mechanism, per the
Lemma in §2.4 — no test-only graph injection.

Throughout: `n` = record count, `L` = normalized Laplacian, `λ₁` = smallest
non-zero eigenvalue.

### V-1 · Complete graph `K_n` — must be **exact**
One indexed field, one distinct value, `n` records. Every record in one bucket.

```
L(K_n) spectrum = { 0,  n/(n−1) with multiplicity n−1 }
λ₁ = n/(n−1)
```

| `n` | expected `λ₁` |
|---|---|
| 3 | 1.5 |
| 4 | 1.333333… |
| 5 | 1.25 |
| 10 | 1.111111… |

**Assert bitwise equality, not tolerance.** This case takes the closed-form fast
path at `spectral.rs:362-364`, which returns `n as f64 / (n as f64 - 1.0)`
directly without iterating. A tolerance here would hide a regression that
replaced the constant with an iteration.

### V-2 · Disjoint cliques — the theorem, and the sentinel
One indexed field with `v` distinct values, buckets of size `g`, `n = v·g`.

```
components                        = v            (Theorem, §2.2)
dim ker L                         = v
smallest non-zero eigenvalue of L = g/(g−1)      (each block is K_g)
what the engine returns           = 0.0          (sentinel, spectral.rs:358)
```

**The engine's return value and the operator's smallest non-zero eigenvalue are
different numbers here, and the spec's first draft conflated them.** `0.0` is
not a measurement of this graph; it is the guard firing. The real spectrum has
no small eigenvalue at all — it has a `v`-fold kernel and then a jump to
`g/(g−1)`.

Assert all three: engine returns `0.0`; `components_from_index` returns exactly
`v`; and `dim ker L = v`. Asserting only the zero would pass for the wrong
reason, since a graph with no edges also returns zero.

This makes V-2 the sharpest adversarial fixture in the battery, because removing
the guard produces a **specific predicted wrong answer** rather than vague
breakage:

| `v` | `g` | engine (guard on) | engine (guard deleted) = `g/(g−1)` |
|---|---|---|---|
| 2 | 3 | 0.0 | 1.5 |
| 5 | 2 | 0.0 | 2.0 |
| 3 | 4 | 0.0 | 1.333333… |

All three verified numerically. A test that asserts `0.0` and the exact
guard-deleted value is a two-sided gate in one fixture.

### V-3 · Cycle `C_n` — the two-field bridging case, under tolerance
Two indexed fields, all buckets of size 2, `n` even:

- field `a`: buckets `{0,1}, {2,3}, …, {n−2,n−1}`
- field `b`: buckets `{1,2}, {3,4}, …, {n−1,0}`

By the Lemma each field contributes a perfect matching; their union is `C_n`,
which is 2-regular, so `L_norm = L_comb / 2`:

```
L_comb(C_n) eigenvalues = 2 − 2cos(2πk/n),  k = 0..n−1
L_norm      eigenvalues = 1 − cos(2πk/n)
λ₁ = 1 − cos(2π/n)
```

| `n` | expected `λ₁` |
|---|---|
| 4 | 1.0 |
| 6 | 0.5 |
| 8 | 0.292893218813… |
| 12 | 0.133974596216… |

This is the load-bearing validation: it is the only one that exercises the
two-field bridging mechanism the audit's fix depends on, and it is exact enough
to catch a sign or normalisation error that `K_n` would not.

**Tolerance discipline.** `λ₁` here is produced by power iteration
(`spectral.rs:721`), so it is approximate. The tolerance must be **derived, not
chosen**: record the iteration's residual and assert
`|λ_measured − λ_closed_form| <= max(10 · residual, 1e-9)`. A hard-coded `1e-6`
that nobody can justify is how a slowly-degrading eigensolver ships.

### V-4 · Empty index set
Zero indexed fields, `n` records.

```
E = empty,  components = n,  λ₁ = 0
```

Assert `λ₁ == 0.0`, `components == n`, **and** that `DEPTH` refuses (F-6). This
is the fixture the audit's hazard lives on.

### V-5 · Reproducibility across restart — INV-I, measured
For each of V-1, V-3: compute `λ₁`, restart the engine, recompute.

```
λ₁(before)  ==  λ₁(after)     bitwise
index_set(before) == index_set(after)
```

This is the fixture that fails today, and it is the whole point of the spec. It
should be run on both boot paths — `Engine::open` and `Engine::open_mmap` —
because they reconstruct schemas through different code
(`engine.rs:911` vs `engine.rs:648`).

### V-6 · Adversarial pass
For each of V-1 through V-5, remove the mechanism under test and confirm the
validation goes red. Specifically:

- replace the `K_n` fast-path constant with `1.0` → V-1 red
- delete the component short-circuit at `spectral.rs:358` → V-2 red
- swap `L_norm` for `L_comb` (drop the `/k`) → V-3 red, V-1 **still green**
  (`K_n` returns a constant), which is itself the argument for V-3 existing
- revert F-2/F-3 → V-5 red

The third bullet is the one worth reading twice: a normalisation error is
invisible to the complete-graph case and visible to the cycle case. A battery
containing only `K_n` would certify a wrong Laplacian.

---

## 7. ORDER OF WORK

**W-IDX-0 — F-1, the five `mark_mutated()` calls. Land alone, first.**
Five lines, no behaviour change on any correct path, ships with T-IDX-1/2/3. It
is what unblocks the E17 indexing work, and it is the only item here that has to
land before that work starts rather than alongside it.

**W-IDX-1 — F-2 + F-3, index durability.** The `do_replay` arm, the
`Engine::schemas` write, the `CreateBundle` re-emit. Ships with T-IDX-4 through
T-IDX-8 and V-5. This is the item that makes INV-I true.

**W-IDX-2 — audit the metadata door.** The 17 `bundle_mut` sites, against the
same two questions: is the mutation journalled, and does it invalidate. Output
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

**The other four λ-verbs' refusal behaviour.** F-6 specifies the gate for
`DEPTH` because that is where the audit measured the hazard. `HORIZON`,
`BETTI` and `SPECTRAL_GAP` need the same treatment and are not specified here;
the E22 review's refusal-battery recommendation is the general form, and at 307
verbs it wants its own scoping pass rather than a clause in this document.

**Records.** Nothing in this spec touches the record write path. That path was
hardened separately and is gated by `tests/durability_wal_truncation.rs`; the
two specs are independent and INV-I extends INV-D's domain without weakening it.
