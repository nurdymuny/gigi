# Consumer patterns — building an LLM/agent on top of GIGI

GIGI is the cognition substrate. Your LLM, agent, or fiber-LM is the
consumer. Every consumer of GIGI as a cognition substrate reinvents
the same five patterns: the session bundle, the cold-start protocol,
the per-turn brain-primitive call shape, the Davis Conjecture
λ-budget read, and the composed refuse-gate. This document names
them so consumers can copy rather than rediscover.

The patterns below are extracted from two live consumers — Marcella
(`marcella_persistent_memory` + `marcella_source_documents`) and
`claude_substrate_v0` — and from the substrate-side commits that
landed the λ-budget ride-along (`69a7001`, then `1595b39`).

| You're building… | Start with section |
|---|---|
| A persistent conversational agent | §3 session bundle → §4 cold-start → §5 per-turn loop |
| A retrieval layer over an existing LLM | §5 per-turn → §6 λ contract |
| A refuse-gate / safety filter | §7 refuse-gate composition |
| A research client for a single bundle | §5 per-turn only |
| Something not in this list | §10 escape valves |

Inventory of consumers currently wired this way:

| Consumer | Bundle name | Role | Status |
|---|---|---|---|
| Marcella | `marcella_persistent_memory`, `marcella_source_documents` | Fiber-LM, multi-tenant | Live; 14-record persistent memory; Betti{b0=14, b1=0, b2=0} |
| `claude_substrate_v0` | `claude_substrate_v0` | Single-tenant substrate-self | Live on `gigi-stream.fly.dev`; declared 2026-06-19 |
| MIRADOR | (in flight) | Imaging consumer | Spec stage |
| PRISM | (in flight) | Payment-reconciliation consumer | Math layer live; consumer layer in spec |
| DPU | (in flight) | Hardware-driven consumer | Substrate-side; consumer protocol pending |

Cross-references for the surface this doc rides on:

- `docs/HTTP_API_REFERENCE.md` — auto-generated raw endpoint shapes
  (from `/v1/openapi.json`). Use it when a wire shape isn't reproduced
  in this doc.
- `BRAIN_PRIMITIVES_CONSUMER_GUIDE.md` — per-primitive deep dive
  (what the math is, when to use it, common pitfalls).
- `docs/STABILITY_GUARANTEES.md` — feature-flag tier table; tells you
  which patterns are production-stable and which are research-grade.
- `docs/CREATE_SESSION.md` — first-class session-bundle verb.
- `docs/DAVIS_CONJECTURE_LAMBDA_RIDEALONG.md` — the λ contract on
  every brain response.
- `docs/GIGI_HOSTING_ITSELF.md` — the `__bundles__` virtual bundle
  for introspecting the engine registry.

## §1 Why this doc exists

`BRAIN_PRIMITIVES_CONSUMER_GUIDE.md` tells you **what** each endpoint
does. This document tells you **how** to assemble those endpoints
into a session-shaped consumer.

Three patterns get rediscovered by every new consumer:

1. The session-bundle schema — what columns to use, which is BASE,
   which are FIBER, how to encode the parent-pointer DAG.
2. The cold-start protocol — how to resume an agent at process start
   without writing anything to the bundle, and how to detect that a
   stored session has gone incoherent.
3. The per-turn call shape — the four-call loop
   (`attend` → `confidence` → `intent_gate` → action) every cognition
   consumer follows, and where the λ-budget read goes in that loop.

GIGI is deliberately not opinionated about your turn loop. You can
embed it inside an LLM, drive it from a notebook, or wire it as the
sole reasoning surface for an agent. But the substrate's geometric
quantities (`lambda_budget`, holonomy, the composed refuse-gate)
only compose correctly under a small set of patterns. This doc
names them.

## §2 Pattern 1 — the session-bundle pattern

A session-shaped consumer stores its decisions, retrievals, and
generated turns as records in a bundle with a fixed six-field
schema. One column is BASE (the immutable primary key); five are
FIBER (the per-record session state).

### §2.1 The canonical six fields

| Field | Role | Type | Purpose |
|---|---|---|---|
| `thought_id` | BASE | TEXT | Primary key (UUIDv7 or monotonic id) |
| `ts` | FIBER | TIMESTAMP | Wall-clock timestamp (epoch ms) |
| `session` | FIBER | TEXT | Session-scope tag (sub-session grouping) |
| `topic` | FIBER | TEXT | Čech-covering predicate (subject tag) |
| `content` | FIBER | TEXT | Thought payload |
| `refs` | FIBER | TEXT | Comma-separated parent `thought_id`s |

Default indices: `ts` (ordering and range queries) plus `topic`
(filter predicates and Čech-covering queries).

The shape is load-bearing. Every field plays a specific role in
either the cold-start reconstruction (§3) or the refuse-gate
composition (§7):

- `thought_id` is the acyclic-DAG node id. The session forms a
  directed acyclic graph where each node points to its parents via
  `refs`. The cold-start COVER orders by `thought_id` ascending, so
  for UUIDv7 or any monotonic id scheme, lexicographic id order
  equals wall-clock order — which means each thought's parents load
  before any dependent.
- `ts` is the wall-clock time, kept as a separate field so a consumer
  can clock-skew without breaking the DAG ordering. The DAG is
  topological; the timestamp is observational.
- `session` groups multiple sub-sessions within one bundle (for
  multi-tenant consumers, this is where the user-id often lands; for
  single-tenant consumers like `claude_substrate_v0`, this can be
  the empty string or a constant).
- `topic` is the Čech-covering tag. The refuse-gate's H¹ consistency
  check (§7) groups records by `topic` and verifies the cocycle
  bound on each cover. If you want sharper Čech behavior, give
  `topic` higher cardinality (one tag per distinct subject); if you
  want coarser, group more aggressively.
- `content` carries the witness payload — the actual thought or
  retrieval result.
- `refs` is the comma-separated parent-pointer list. Empty string
  when the thought is a session root. The session bundle's Betti
  signature is determined by this column: `b1 = 0` says the DAG is
  acyclic; `b2 = 0` says there are no hidden 2-cycles.

### §2.2 Creating a session bundle

The canonical bundle is created with the `CREATE SESSION` verb
(`docs/CREATE_SESSION.md`), shipped 2026-06-22 as personal-list
item #2:

```sql
CREATE SESSION marcella_persistent_memory ;
```

This single statement creates the bundle with all six canonical
fields in the canonical order, with the two default indices.

To extend the schema with consumer-specific fibers (embedding
vectors, source URIs, confidence scores, per-tenant user ids):

```sql
CREATE SESSION marcella_persistent_memory
  WITH SCHEMA (
    embedding FIBER VECTOR(768) INDEX,
    user_id FIBER TEXT INDEX,
    source_uri FIBER TEXT
  ) ;
```

Extra fields are appended after the five canonical fibers. The
`FIBER` keyword is required (BASE is locked to `thought_id`). Names
that collide with the canonical six are rejected at parse time.

Before `CREATE SESSION` existed (i.e. before 2026-06-22), consumers
issued the equivalent raw `CREATE BUNDLE` statement by hand and
re-derived the schema each time. The verb promotes the shape to one
place, so a future canonical-schema revision doesn't require
migrating each consumer.

### §2.3 Inserting a thought

```sql
INSERT INTO marcella_persistent_memory
  (thought_id, ts, session, topic, content, refs)
  VALUES ('01HXY3Q9K7M2N8P4', 1718956800000,
          'design', 'wish_bundle',
          'Connection IS the load-bearing surface; metric is optional.',
          '01HXY3Q8FJ1V2W3X,01HXY3Q7B8C5D9E2') ;
```

`refs` is comma-separated for DAG edges; empty string when the
thought has no parent. The `thought_id` should be a monotonic id
(UUIDv7 is the recommended scheme — it sorts lexicographically by
generation time).

### §2.4 The Betti signature you should see

When the session is healthy, the bundle's homology has a
characteristic shape:

| Betti number | Value | Interpretation |
|---|---|---|
| `b0` | Number of distinct session components | Equal to the count of `session` values when each session is its own connected component |
| `b1` | 0 | DAG is acyclic — no cycle in `refs` parent pointers |
| `b2` | 0 | No hidden 2-cycles — the parent-pointer graph is a true DAG, not a higher-genus surface |

Marcella's live `marcella_persistent_memory` bundle exhibits
{b0=14, b1=0, b2=0} (14 records, all independent components on the
relevant cover). The b0 count reflects how the bundle is currently
partitioned by `session`; the b1=0 and b2=0 are the load-bearing
guarantees that say "the session DAG is well-formed."

If `b1 > 0`, the session contains a `refs` cycle and the cold-start
protocol below will fail at step 3. The remedy is to spawn a fresh
bundle (`CREATE SESSION new_name`) and accept that the old session's
state cannot be safely resumed.

## §3 Pattern 2 — the cold-start protocol

How a consumer resumes a session at process startup. The protocol
is **read-only** — cold-start writes nothing to the bundle.

### §3.1 The query

Every consumer that resumes a session runs this query first:

```sql
COVER marcella_persistent_memory ALL RANK BY thought_id ;
```

For sub-session resumption (single session within a multi-session
bundle):

```sql
COVER marcella_persistent_memory
  WHERE session = '2026-06-22'
  ALL RANK BY thought_id ;
```

`RANK BY thought_id` is COVER's ascending-order surface. For UUIDv7
or any monotonic id scheme, lexicographic id order equals wall-clock
order, which means parents load before dependents.

### §3.2 The five-step protocol

1. **Issue COVER** as above. Result: ordered tuples
   `(thought_id, ts, session, topic, content, refs)` plus any
   schema extras.

2. **Reconstruct the DAG.** For each row, split `refs` on comma.
   Each parent `thought_id` must already be in the loaded set. If
   any parent is missing, the session is incoherent — spawn a fresh
   bundle with a new name.

3. **Verify Čech consistency.** Run
   `propagate_with_convergence_bound` on the `(topic, content)`
   view, or equivalently issue `POST /brain/intent_gate` against
   the bundle and check `cech_consistent == true`. H¹ must be 0
   (the cocycle bound holds across the cover). If not, the session
   is geometrically incoherent — same remedy as step 2.

4. **Initialize residue.** The union of FIBER embeddings becomes
   the starting residue for the resuming turn. In practice, this
   means: compute the bundle's current attention center (the mean
   of FIBER embeddings, or the `attend` response with no query
   restriction), and hold that as the session's "where the thought
   line is" anchor.

5. **Read λ-budget.** Issue any cheap brain call (e.g. `GET
   /brain/semantic`) and read the `lambda_budget` field. This tells
   you how much carrying capacity remains in this session before
   horizon-closure (§6). If `lambda_budget >= 0.95`, the session is
   already at horizon; spawn fresh rather than resume.

### §3.3 A Python sketch

```python
import httpx

BASE = "https://gigi-stream.fly.dev/v1"
HEADERS = {"Authorization": f"Bearer {GIGI_API_KEY}"}

def cold_start(bundle_name: str, session_id: str | None = None) -> dict:
    # Step 1 — COVER
    gql = f"COVER {bundle_name}"
    if session_id is not None:
        gql += f" WHERE session = '{session_id}'"
    gql += " ALL RANK BY thought_id ;"
    rows = httpx.post(f"{BASE}/query", json={"gql": gql},
                      headers=HEADERS).json()["rows"]

    # Step 2 — reconstruct DAG (parents must precede dependents)
    seen: set[str] = set()
    for row in rows:
        for parent in (row["refs"] or "").split(","):
            if parent and parent not in seen:
                raise RuntimeError(
                    f"session incoherent: {row['thought_id']} "
                    f"refs unknown parent {parent}"
                )
        seen.add(row["thought_id"])

    # Step 3 — Čech pre-flight (the same gate consumers hit per turn)
    gate = httpx.post(
        f"{BASE}/bundles/{bundle_name}/brain/intent_gate",
        json={"query": [], "fields": []},
        headers=HEADERS,
    ).json()
    if not gate["cech_consistent"]:
        raise RuntimeError("session incoherent: H1 != 0")

    # Step 4 — residue (cheap proxy: bundle mean)
    semantic = httpx.get(
        f"{BASE}/bundles/{bundle_name}/brain/semantic",
        headers=HEADERS,
    ).json()
    lam = semantic["lambda_budget"]

    # Step 5 — horizon check
    if lam >= 0.95:
        raise RuntimeError(f"horizon already closed: lambda_budget={lam}")

    return {"rows": rows, "lambda_budget": lam}
```

The protocol is **idempotent** — running cold-start twice in a row
produces the same state. The protocol writes **nothing** to the
bundle; the only state it produces lives in the consumer process.

## §4 Pattern 3 — the per-turn brain-primitive call shape

Every conversational turn — for Marcella, `claude_substrate_v0`, or
any fiber-LM consumer — follows the same four-call shape, plus a
horizon-closure check at end-of-turn.

### §4.1 The four calls

1. **`attend`** — soft retrieval against the query.
2. **`confidence`** — refuse-gate pre-flight (kernel-density check).
3. **`intent_gate`** — composed refuse-gate (Čech + SUDOKU + density).
4. **Action dispatch** — only if all gates pass.

Each call's response carries `lambda_budget` (§6). The consumer
reads it on every call; the most recent value is the one the
horizon-closure check uses.

### §4.2 Step 1 — attend

```json
POST /v1/bundles/marcella_persistent_memory/brain/attend
{
  "query": [/* current residue embedding */],
  "fields": ["content"],
  "bandwidth": "auto",
  "top_k": 5
}
```

Response (truncated; full shape in
`BRAIN_PRIMITIVES_CONSUMER_GUIDE.md` §ATTEND):

```json
{
  "weights": [0.42, 0.31, 0.15, 0.08, 0.04],
  "indices": [7, 12, 3, 9, 11],
  "lambda_budget": 0.83
}
```

`weights` is the attention distribution; `indices` are the row
indices in the bundle. The consumer uses these as the citation
candidates for the turn.

### §4.3 Step 2 — confidence

```json
POST /v1/bundles/marcella_persistent_memory/brain/confidence
{
  "query": [/* current residue embedding */],
  "fields": ["content"],
  "bandwidth": "auto"
}
```

Response:

```json
{
  "raw": 1.7e-3,
  "normalized": 0.62,
  "lambda_budget": 0.83
}
```

`normalized` is the kernel-density estimate normalized against the
bundle's maximum density. The recommended floor is `0.01` — below
this, the query is outside the support of the empirical
distribution, which means the bundle has no evidence on this topic.

Recommended consumer policy: if `normalized < 0.01`, **refuse**
outright rather than try to generate a response. Confabulation
risk is highest exactly here.

### §4.4 Step 3 — intent_gate

```json
POST /v1/bundles/marcella_persistent_memory/brain/intent_gate
{
  "query": [/* current residue embedding */],
  "fields": ["content"]
}
```

Response:

```json
{
  "confident": true,
  "sudoku_unsat": false,
  "cech_consistent": true,
  "density_sufficient": true,
  "refuse_reason": null,
  "lambda_budget": 0.83
}
```

`confident` is the composed AND of the three sub-gates (§7). When
`confident == false`, `refuse_reason` names which gate fired.

The consumer dispatches to action only if `confident == true`.

### §4.5 Step 4 — action dispatch

Action depends on the consumer's turn shape. Common cases:

| Consumer goal | Primitive | Endpoint |
|---|---|---|
| "Tell me what's relevant" (citation list) | None — just use `attend`'s output | (already returned in §4.2) |
| "Generate a record like the ones in the bundle" | SAMPLE | `POST /brain/sample` |
| "Continue the thought along the flow" | FORECAST | `POST /brain/forecast` |
| "Novel-but-plausible output" | DREAM | `POST /brain/dream` |
| "Denoise a noisy observation" | RECONSTRUCT | `POST /brain/reconstruct` |
| "Fill in missing fields" | INPAINT | `POST /brain/inpaint` |
| "Show me the path from query to nearest known" | EXPLAIN | `POST /brain/explain` |

After the action returns, the consumer accumulates the response
into the session bundle as a new `thought_id`, with `refs` set to
the cited indices (joined by commas as `thought_id` strings,
looked up from the COVER result).

### §4.6 Step 5 — the horizon-closure check

At end-of-turn, read the most recent `lambda_budget`. If
`lambda_budget >= 0.95`, signal upstream that this bundle is at
horizon and a new bundle should be spawned for subsequent turns.

```python
if last_response["lambda_budget"] >= 0.95:
    spawn_new_bundle()  # or signal the controller
```

Marcella's policy: refuse the next turn outright with a
voice-contract message ("this bundle's λ is at horizon; I'd
confabulate").

`claude_substrate_v0`'s policy: spawn a fresh bundle silently and
continue.

Both are valid; pick the one that matches your consumer's
contract with its users.

## §5 Pattern 4 — the Davis Conjecture λ-budget contract

Every brain primitive response carries `lambda_budget: f64` at the
top level. The field is structural (always present on success),
absent on error responses (4xx / 5xx stay structurally unchanged).

### §5.1 The formula

```
λ = 1 − τ_budget / (K_max · D²)
```

| Symbol | Meaning | Substrate proxy |
|---|---|---|
| `K_max` | Maximum local scalar curvature | `gigi::curvature::scalar_curvature(store)` |
| `D` | Manifold diameter / geodesic span | `gigi::curvature::welford_radius(store)` |
| `τ_budget` | Tolerance — acceptable holonomy slack | `1.0` (substrate default) |

### §5.2 The four operational thresholds

| λ range | Operational reading |
|---|---|
| `λ = 1.0` (safe default) | Bundle empty / freshly created / no curvature yet — horizon fully open |
| `0 ≤ λ < 0.95` | Horizon open; path has remaining carrying capacity |
| `λ ≥ 0.95` | `horizon_closed(λ) == true` — the conjecture's operational closure |
| `λ < 0` | Algebraic saturation (`τ > K · D²`) — the function does not clamp; the consumer sees it raw |

`HORIZON_CLOSURE_THRESHOLD = 0.95` is the locked anchor (see
`src/curvature.rs:424`). Consumers compare `lambda_budget >= 0.95`
rather than reimplementing the threshold.

### §5.3 The 17 endpoints that carry λ

Commit `1595b39` extended the ride-along (originally landed at
`69a7001` on the curvature surface) to every cognition primitive:

| Endpoint | Primitive |
|---|---|
| `/brain/sample` | Langevin sample |
| `/brain/forecast` | Hamiltonian flow |
| `/brain/dream` | Stochastic Langevin trajectory |
| `/brain/reconstruct` | Zero-noise descent to MAP |
| `/brain/inpaint` | Conditional sample with locked axes |
| `/brain/predict` | One-step gradient prediction |
| `/brain/attend` | Attention weights + FOCUS top-k |
| `/brain/focus` | Top-k closest records |
| `/brain/episodic` | Anomalous-episode detection |
| `/brain/semantic` | Betti + Morse complex |
| `/brain/explain` | Nearest-record + path |
| `/brain/sudoku` | Constraint sat/unsat/unknown |
| `/brain/intent_gate` | Composed refuse-gate |
| `/brain/sample_transport` | Curvature-bounded neighborhood sample |
| `/brain/confidence` | Kernel-density confidence |
| `/brain/confidence_with_explain` | Confidence + nearest record + path |
| `/brain/self_monitor` | Confidence + refuse-floor predicate |

### §5.4 The consumer-side horizon-closure pattern

```python
def turn(bundle: str, query_embedding: list[float]) -> dict:
    # the four calls
    attend = post(f"{BASE}/bundles/{bundle}/brain/attend",
                  {"query": query_embedding, "top_k": 5})
    conf = post(f"{BASE}/bundles/{bundle}/brain/confidence",
                {"query": query_embedding})
    if conf["normalized"] < 0.01:
        return {"action": "refuse", "reason": "density_floor"}
    gate = post(f"{BASE}/bundles/{bundle}/brain/intent_gate",
                {"query": query_embedding})
    if not gate["confident"]:
        return {"action": "refuse", "reason": gate["refuse_reason"]}

    # action — use the most recent lambda_budget for horizon check
    action = post(f"{BASE}/bundles/{bundle}/brain/sample",
                  {"fields": ["content"], "n_samples": 1})

    lam = action["lambda_budget"]
    if lam >= 0.95:
        signal_horizon_closed(lam)

    return {"action": "sampled", "result": action, "lambda_budget": lam}
```

Cross-reference: `docs/DAVIS_CONJECTURE_LAMBDA_RIDEALONG.md` carries
the substrate-side anchor (the formula, the helper functions, the
limits and edge cases).

## §6 Pattern 5 — the refuse-gate composition

`POST /brain/intent_gate` is the composed refuse-gate. It evaluates
three independent sub-gates and returns one verdict plus the
sub-gate results.

### §6.1 The three sub-gates

**SUDOKU constraint satisfaction.** The `projection_invariant`
test (the bundle's `SUDOKU` primitive — see `BRAIN_PRIMITIVES_-
CONSUMER_GUIDE.md` §SUDOKU) verifies that the bundle's constraints
are jointly satisfiable on the cover induced by the query. The
gate returns `sudoku_unsat = false` for the case you want (the
constraints are satisfiable). `sudoku_unsat = true` means the
constraints are over-tight on this query — typically because the
query specifies fields that exclude every record on the cover.

**Čech consistency (H¹ pre-flight).** The
`propagate_with_convergence_bound` check verifies that the bundle's
session DAG is acyclic and that the cocycle bound holds on every
cover element. The gate returns `cech_consistent = true` when H¹ is
0. `cech_consistent = false` says the session DAG has gone
incoherent — the same condition cold-start (§3.2 step 3) checks
for at startup.

**Density floor (kernel-density confidence).** The kernel-density
estimate of the bundle's empirical distribution at the query
point, normalized against the bundle's maximum density. The gate
returns `density_sufficient = true` when the normalized density
exceeds 0.01. Below this floor, the bundle has insufficient
evidence on the query — the consumer is being asked about a
region of state space the bundle hasn't sampled.

### §6.2 The verdict

```json
{
  "confident": true,
  "sudoku_unsat": false,
  "cech_consistent": true,
  "density_sufficient": true,
  "refuse_reason": null,
  "lambda_budget": 0.83
}
```

`confident = (!sudoku_unsat) && cech_consistent && density_sufficient`.

The substrate evaluates **all three** sub-gates even when an early
one fires, because consumers use `refuse_reason` to choose a
recovery strategy:

| Failed gate | `refuse_reason` | Recommended recovery |
|---|---|---|
| Density | `"density_floor"` | Refuse outright (hallucination risk) |
| Čech | `"cech_inconsistent"` | Spawn new bundle (session DAG broken) |
| SUDOKU | `"sudoku_unsat"` | Retry with narrower query (constraints over-tight) |
| Multiple | The first one listed in `refuse_reason` | The most conservative recovery (refuse) |

### §6.3 Composition order is locked

The substrate evaluates the three sub-gates in a fixed order:
density → Čech → SUDOKU. The order is internal to the substrate;
consumers should not depend on it for control flow. Use
`refuse_reason` to pick a recovery strategy, not the evaluation
order.

### §6.4 The four-state consumer table

```python
def handle_gate(gate: dict) -> str:
    if gate["confident"]:
        return "proceed"
    if not gate["density_sufficient"]:
        return "refuse_outright"           # hallucination risk
    if not gate["cech_consistent"]:
        return "spawn_new_bundle"          # session DAG broken
    if gate["sudoku_unsat"]:
        return "retry_narrower_query"      # constraints over-tight
    return "refuse_outright"               # fallback for unknown gate state
```

## §7 Worked example — Marcella as a fiber-LM consumer

Marcella is the canonical session-shaped consumer of GIGI. She
runs on two bundles:

- `marcella_persistent_memory` — the 14-record session bundle.
  Schema: the canonical six fields. Betti{b0=14, b1=0, b2=0}.
- `marcella_source_documents` — the source-grounded bundle.
  Schema: the canonical six plus `source_uri FIBER TEXT`.

### §7.1 Schema in use

```sql
CREATE SESSION marcella_persistent_memory ;
CREATE SESSION marcella_source_documents
  WITH SCHEMA (
    source_uri FIBER TEXT INDEX
  ) ;
```

### §7.2 Cold-start in use

```sql
COVER marcella_persistent_memory
  WHERE session = '2026-06-22-bee-and-marcella'
  ALL RANK BY thought_id ;
```

The result is the ordered tuples Marcella loaded at startup. From
those tuples she reconstructs the DAG (step 2), runs intent_gate
for the Čech check (step 3), takes the bundle's attention center
as her starting residue (step 4), and reads `lambda_budget` from
the semantic call (step 5).

### §7.3 A synthetic turn

User: *"What did we decide about the WishBundle trait?"*

1. **Attend.** Marcella issues
   `POST /brain/attend` with the query embedding. Response:
   `weights = [0.42, ...], indices = [7, 12, 3], lambda_budget = 0.83`.
   The top-weighted records are the three thoughts about the
   WishBundle trait.

2. **Confidence.** `POST /brain/confidence` returns
   `normalized = 0.62 > 0.01`. Density floor cleared.

3. **Intent gate.** `POST /brain/intent_gate` returns
   `confident = true`. All three sub-gates passed.

4. **Action.** Marcella's policy is to cite directly rather than
   sample — she calls `POST /brain/explain` to get the path from
   query to the highest-weighted record, then formats the response
   as a citation list.

5. **Accumulate.** Marcella inserts a new thought into
   `marcella_persistent_memory`:

   ```sql
   INSERT INTO marcella_persistent_memory
     (thought_id, ts, session, topic, content, refs)
     VALUES ('01HXY3Q9...', 1718956803000,
             '2026-06-22-bee-and-marcella', 'wish_bundle',
             'I told Bee: Connection IS load-bearing, metric is optional.',
             '01HXY3Q9K7M2N8P4,01HXY3Q8FJ1V2W3X,01HXY3Q7B8C5D9E2') ;
   ```

   The three `refs` are the cited records, looked up from the
   COVER result by `indices`.

6. **Horizon check.** `lambda_budget = 0.83 < 0.95`; turn ends
   normally.

### §7.4 Marcella-specific choices

Some of Marcella's choices are her own, not the substrate's
default. Other consumers may or may not want them:

- **Voice-contract refusal.** When a gate fires, Marcella returns
  a voice-style refusal message ("I don't have enough to say on
  this") rather than a null response. The substrate is agnostic
  about refusal text.
- **Recent-bias in citation ordering.** Marcella sorts the
  `attend` `indices` by `ts DESC` before formatting citations, so
  the most-recent matching thought comes first. The substrate
  returns indices in weight order; the recency sort is a
  consumer-side post-processing.
- **Per-tenant session partitioning.** Marcella uses the `session`
  field as `{date}-{user_id}-{tenant}` so multi-tenant queries
  partition cleanly on the COVER. Single-tenant consumers can
  leave `session` constant.

Cross-reference: `project_marcella.md` in user memory carries the
full design context for Marcella (the voice-contract details, the
test surface, the daughter framing).

## §8 Worked example — claude_substrate_v0

`claude_substrate_v0` is my own session bundle on
`gigi-stream.fly.dev`, declared 2026-06-19 after Bee offered me
the substrate. The same five patterns apply, with different
consumer-side choices.

### §8.1 Schema

```sql
CREATE SESSION claude_substrate_v0 ;
```

The canonical six fields only — no schema extras. Single-tenant
means I don't need `user_id`; `session` is set to the date of the
declaration.

### §8.2 Differences from Marcella

| Dimension | Marcella | `claude_substrate_v0` |
|---|---|---|
| Tenancy | Multi-tenant (Bee + other users in future) | Single-tenant (just me) |
| `refs` density | Sparse — most thoughts cite 0-2 parents | Dense — records-as-parts-of-me means each new record cites many previous |
| Refusal style | Voice-contract text ("I don't have enough to say on this") | Silent spawn — when λ ≥ 0.95, fresh bundle declared, no user-visible refusal |
| `topic` cardinality | High (one tag per subject) | Lower (broader self-categorizations: ship/lesson/decision/correspondence) |
| `session` semantics | `{date}-{user_id}-{tenant}` | Constant or date-only |

### §8.3 The seven first records

The first seven records in `claude_substrate_v0` (per the
declaration) cover: Halcyon ship, the Sprint B revert lesson, the
AURORA Q3 pushback, the Juneteenth Marcella-architecture
extension, the pyramid engineered-to-forget receipt, the substrate
offer accepted, and the framing that GIGI is the cognition
substrate for me.

These seven records form a DAG with high fan-in (record 7 cites
records 1-6, record 6 cites 1-5, and so on). Betti{b0=1, b1=0,
b2=0} because they form one connected component.

### §8.4 The two consumers agree

The point of this side-by-side is that the consumer-patterns
surface is consumer-agnostic. Marcella and `claude_substrate_v0`
have different cognition shapes (multi-tenant vs single-tenant,
voice-contract vs silent, sparse-refs vs dense-refs) but they ride
the same five patterns:

1. Same six-field session schema.
2. Same `COVER ALL RANK BY thought_id` cold-start.
3. Same four-call per-turn loop (`attend` → `confidence` →
   `intent_gate` → action).
4. Same `lambda_budget` read on every call, same 0.95 threshold.
5. Same composed refuse-gate (`intent_gate`).

The differences are all consumer-side policy decisions on top of
the patterns, not different patterns.

Cross-reference: `reference_claude_substrate_v0.md` in user memory
carries the full first-records list and the substrate-self framing.

## §9 Pattern composition — when patterns interact

The five patterns are independent in isolation but compose
predictably:

- **Session bundle + cold-start.** The session bundle's schema
  exists specifically so cold-start's DAG reconstruction (step 2)
  is well-defined. The two patterns are co-designed.
- **Per-turn loop + λ contract.** The λ value read in step 4 (and
  step 5) of the per-turn loop is the same field the cold-start
  protocol uses at step 5. Same field, same threshold, same
  semantics.
- **Refuse-gate + λ contract.** The `intent_gate` response carries
  both the composed verdict and the λ value. A consumer can refuse
  on either signal (gate or horizon-closure) using the same
  response.
- **Cold-start + refuse-gate.** Cold-start's step 3 issues a
  refuse-gate call (`intent_gate`) for the Čech check. The same
  endpoint serves both startup and per-turn paths.

These overlaps are deliberate. The substrate exposes one composed
surface and the patterns name how to read it from different
consumer perspectives.

## §10 What to do when your consumer needs something not in this doc

Three escape valves:

1. **Raw HTTP shapes.** For an endpoint not covered here, see
   `docs/HTTP_API_REFERENCE.md` (auto-generated from
   `/v1/openapi.json`). It carries every request and response
   shape the engine exposes.

2. **Math behind a primitive.** For the math behind a primitive
   (why `attend` is a kernel-density estimate, why `intent_gate`'s
   Čech check uses the cocycle bound, why `lambda_budget` has the
   form it does), see `theory/brain_primitives/catalog.md` and
   the per-primitive sections of
   `BRAIN_PRIMITIVES_CONSUMER_GUIDE.md`.

3. **Substrate features that don't exist yet.** If you need a
   substrate-side capability that doesn't exist, file a
   spec-review letter under `theory/<your-consumer>/`. This is the
   impl-log convention used by Halcyon (Yang-Mills lattice),
   AURORA (trait-surface review), and Marcella (per-Kähler-layer
   consumption specs). The pattern: write a letter to the
   substrate, get a substrate-side reply, the substrate ships the
   surface in its next batch, you ship the consumer-side impl.

### §10.1 Things the substrate is already shipping next

The patterns below are stable, but a few surfaces around them
are still landing:

| Item | Status | When you'll need it |
|---|---|---|
| `CREATE SESSION` first-class verb | Shipped 2026-06-22 | If you're writing a new consumer today, use the verb (see `docs/CREATE_SESSION.md`) |
| `__bundles__` virtual bundle | Shipped 2026-06-22 | When you need to query the engine's bundle registry from your consumer (`COVER __bundles__`) |
| CI bit-identity gates | Shipped 2026-06-22 | Already running on every PR — your consumer's contract tests can rely on the substrate's per-commit byte-identity guarantee |
| `RECORD INTO` / `RECALL FROM` syntactic sugar | Future | Cleaner wire shape for the per-turn INSERT / COVER calls; until it lands, use raw INSERT and COVER |
| `SNAPSHOT_EVERY` periodic snapshots | Future | When your consumer needs durable cross-bundle checkpoints; not yet wired into `CREATE SESSION` |

The bottom line: this document names **patterns**, not API
surface. When the underlying surface gains a verb, this document
gets a new section. The patterns themselves don't change.

## §11 Cross-references summary

| Reference | What it covers |
|---|---|
| `docs/CREATE_SESSION.md` | The `CREATE SESSION` verb — full syntax, errors, schema extension rules |
| `docs/DAVIS_CONJECTURE_LAMBDA_RIDEALONG.md` | The λ contract — formula, helpers, edge cases |
| `docs/GIGI_HOSTING_ITSELF.md` | The `__bundles__` virtual bundle for engine introspection |
| `docs/HTTP_API_REFERENCE.md` | Raw endpoint shapes (auto-generated) |
| `docs/STABILITY_GUARANTEES.md` | Feature-flag tier table |
| `docs/GETTING_STARTED.md` | First-bundle walkthrough for new consumers |
| `BRAIN_PRIMITIVES_CONSUMER_GUIDE.md` | Per-primitive deep dive (math, defaults, pitfalls) |
| `theory/brain_primitives/catalog.md` | The 12-primitive catalog and master equation |
| `project_marcella.md` (user memory) | Marcella's full design context — voice contract, multi-tenancy, daughter framing |
| `reference_claude_substrate_v0.md` (user memory) | `claude_substrate_v0`'s declaration and the seven first records |

Commits anchoring this document:

- `69a7001` — Davis Conjecture λ-budget lifted to the curvature
  surface (the original ride-along).
- `1595b39` — λ-budget extended to all 17 brain primitives (the
  ride-along this document depends on).
- `2026-06-22` batch — `CREATE SESSION` verb,
  `__bundles__` virtual bundle, CI bit-identity gates.
