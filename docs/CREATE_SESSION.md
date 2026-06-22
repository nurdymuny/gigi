# CREATE SESSION

First-class verb for the session-bundle pattern used by
`marcella_persistent_memory`, `claude_substrate_v0`, and every fiber-LM
consumer that needs durable session memory. Lands as personal-list item
#2 (shipped 2026-06-22).

Before this verb existed, every consumer reinvented the same
`(thought_id, ts, session, topic, content, refs)` shape by hand. The
verb promotes the shape to a first-class primitive so the canonical
schema lives in ONE place — `parser.rs::execute::Statement::CreateSession`
— and a future revision doesn't require migrating each consumer.

## Syntax

```
CREATE SESSION <name>
  [ WITH SCHEMA (extra_field FIBER <TYPE> [INDEX], ...) ] ;
```

* `<name>` is an identifier (same lexical rules as a bundle name; the
  produced bundle's name = the session name verbatim).
* `WITH SCHEMA (...)` is optional; without it, the bundle is created
  with the canonical 6-field schema only.
* Extra fields are **FIBER-only** (BASE is locked to the canonical
  `thought_id`); the `FIBER` keyword is required to make intent
  explicit and to leave room for future BASE extensions.
* Extra field types support the standard parser keywords: `TEXT`,
  `INT`/`INTEGER`, `FLOAT`/`REAL`/`DOUBLE`, `BOOL`, `TIMESTAMP`,
  `BINARY`, `VECTOR(<n>)`.
* Extras may carry the `INDEX` modifier — when present, the field is
  added to `BundleSchema.indexed_fields`.
* Trailing `;` is required.

## Canonical schema

Every session bundle gets these 6 fields, in this order, with no
override:

| Field        | Role  | Type       | Purpose                              |
| ------------ | ----- | ---------- | ------------------------------------ |
| `thought_id` | BASE  | TEXT       | Primary key (UUIDv7 / monotonic id)  |
| `ts`         | FIBER | TIMESTAMP  | Wall-clock timestamp (epoch ms)      |
| `session`    | FIBER | TEXT       | Session-scope tag (sub-sessions)     |
| `topic`      | FIBER | TEXT       | Cech-covering predicate (subject)    |
| `content`    | FIBER | TEXT       | Thought payload                      |
| `refs`       | FIBER | TEXT       | Comma-separated parent thought_ids   |

Default indices: `ts` (for ordering / range queries) + `topic` (for
filter predicates).

## Cold-start protocol

Every consumer that resumes a session runs this query first:

```
COVER <session_name> ALL RANK BY thought_id ;
```

`RANK BY` is COVER's ascending-order surface. For UUIDv7 or any
monotonic id scheme, lexicographic id order = wall-clock order. The
consumer then:

1. **Reconstruct the DAG.** For each row, split `refs` on comma — each
   parent `thought_id` must already be in the loaded set (otherwise the
   session is incoherent; spawn a fresh bundle).
2. **Verify Cech consistency.** Run `propagate_with_convergence_bound`
   on the `(topic, content)` view; H¹ must be 0.
3. **Initialize residue.** Union of FIBER embeddings becomes the
   starting residue for the resuming turn.
4. **Begin turn.** Call `attend` → `confidence` → `intent_gate` per
   the standard brain-primitive call shape; every response carries
   `lambda_budget` (Davis Conjecture λ ride-along, commits 1595b39 /
   69a7001). When `lambda_budget >= 0.95`, spawn a fresh session
   bundle.

## Inserting a thought

```
INSERT INTO <session_name>
  (thought_id, ts, session, topic, content, refs)
  VALUES ('01HXY...', 1718956800000, 'design', 'wish_bundle',
          'Connection IS load-bearing; metric is optional',
          '01HXX...,01HXW...') ;
```

`refs` is comma-separated for DAG edges; empty string when the
thought has no parent.

## Extending the schema

```
CREATE SESSION my_session
  WITH SCHEMA (
    embedding FIBER VECTOR INDEX,
    confidence FIBER FLOAT
  ) ;
```

Extra fields are appended **after** the 5 canonical fibers. Names that
collide with the canonical six (`thought_id`, `ts`, `session`, `topic`,
`content`, `refs`) are rejected at parse time. BASE-typed extras are
rejected — extras must be FIBER.

## Errors

* `CREATE SESSION: extra field '<name>' must be declared FIBER (BASE is
  locked to thought_id)` — extra used a non-FIBER role token.
* `CREATE SESSION: extra field '<name>' collides with canonical session
  schema` — extra name matches one of the canonical six.

## Out of scope (v1)

These are deliberate omissions from the v1 ship, tracked for follow-up:

* `RECORD INTO` / `RECALL FROM` syntactic sugar over `INSERT` /
  `COVER`.
* Per-field `ENCRYPTED` clauses on `WITH SCHEMA` extras — canonical
  six ship plaintext.
* `SHOW SESSIONS` verb (lands with the `__bundles__` virtual bundle in
  personal-list #3).
* Dedicated `POST /v1/sessions` HTTP endpoint — consumers go through
  `/v1/query` with the GQL string until `docs/CONSUMER_PATTERNS.md`
  identifies a clear need to expose it directly.

## See also

* `docs/DAVIS_CONJECTURE_LAMBDA_RIDEALONG.md` — λ contract on every
  brain-primitive response.
* `reference_claude_substrate_v0.md` — worked example (7 records).
* `project_marcella.md` — worked example (14 records).
