# GIGI hosting itself — the `__bundles__` virtual bundle

Status: shipped 2026-06-22 (personal-list #3).
Module: `src/virtual_bundles.rs`.
Tests: `tests/gigi_hosting_itself.rs` (8 cases).

## What this is

`__bundles__` is a virtual bundle whose rows are the engine's live
bundle registry. It is queried with the same `COVER` verb that
queries every other bundle, but it is never persisted, never
written to the WAL, and never appears in `self.bundles` /
`self.mmap_bundles`. The row set is re-materialized from the engine
registry on every `COVER __bundles__` call, so the answer is always
honest about current engine state.

```text
COVER __bundles__ ;
COVER __bundles__ WHERE type='heap' ;
COVER __bundles__ RANK BY n_records DESC FIRST 10 ;
COVER __bundles__ PROJECT (name, type) ;
```

The full `COVER` clause vocabulary works for free — `WHERE`, `OR`
groups, `RANK BY`, `FIRST`, `SKIP`, `PROJECT`, `DISTINCT` — because
the virtual-bundle short-circuit reuses the same `QueryCondition`
matcher that real bundles use, applied to an in-memory `Vec<Record>`.

## Schema

| field        | kind  | type      | semantics                                                                 |
|--------------|-------|-----------|---------------------------------------------------------------------------|
| `name`       | BASE  | TEXT      | bundle key as returned by `engine.bundle_names()`                          |
| `type`       | FIBER | TEXT      | `"heap"` / `"overlay"` / `"virtual"`                                       |
| `n_records`  | FIBER | INT       | `BundleRef::len()` (overlay: `base.len() + overlay_len()`)                 |
| `created_ts` | FIBER | TIMESTAMP | best-effort creation timestamp in seconds since the UNIX epoch (see below) |

The `__bundles__` row appears in its own output. Querying the
registry that contains the registry returns a row for the registry.
The self-row is classified as `type='virtual'` so a downstream
consumer can filter it out with `WHERE type='heap'` when only the
real bundles are wanted.

### `created_ts` honesty

The WAL `CreateBundle` entry does not carry a wall-clock timestamp,
so `created_ts` is the closest honest proxy available without
breaking WAL forward-compat:

- **overlay bundles** → mtime of `snapshots/<name>.dhoom` when the
  file exists on disk
- **heap bundles** → mtime of `gigi.wal` (proxy for "when this
  engine started serving this bundle")
- **in-memory engines** before the first WAL write — current
  wall-clock time, honest about the fact that nothing is durable yet
- **`__bundles__` self-row** → query time

A future revision can promote `created_ts` to a tracked field by
extending the WAL `CreateBundle` entry with a wall-clock timestamp;
the schema field is already in place. Consumers who only need a
strict ordering should `RANK BY name` instead.

## Read-only enforcement

`__bundles__` is a reserved name. Every write-shape executor arm in
`src/parser.rs` calls `virtual_bundles::reject_virtual_write(name,
verb)` before mutating any state, so `INSERT`, `BATCH INSERT`,
`UPSERT`, `BATCH UPSERT`, `REDEFINE`, `BULK REDEFINE`, `RETRACT`,
`BULK RETRACT`, `CREATE BUNDLE`, `CREATE SESSION`, `COLLAPSE`,
`INGEST`, `TRANSPLANT`, `GENERATE BASE`, `FILL`, and maintenance
verbs (`COMPACT` / `ANALYZE` / `VACUUM` / `REBUILD INDEX` /
`CHECK INTEGRITY` / `REPAIR`) all fail with a clear error before
any state mutation. A rejected write is byte-identical to no write
at all — no WAL entry, no notification, no cache invalidation.

```text
> SECTION __bundles__ (name='nope', type='heap', n_records=0, created_ts=0)
ERROR: INSERT on '__bundles__' rejected: '__bundles__' is a virtual
bundle (read-only); use COVER __bundles__ to query the live registry
```

The reserved-name set lives in
`virtual_bundles::reserved_names()`. Future virtual bundles
(`__lattices__`, `__gauges__`, `__sessions__`) plug into the same
guard by extending that slice — every existing write arm picks
them up automatically.

## Motivation — structure as its own datum

The honest framing of `__bundles__` is the pyramid paper's Theorem 7
lifted into the runtime. Single-chain sequential composition cannot
hit Old Kingdom precision; the only construction that meets the
2.87mm translation / 8.62" yaw / 0.5mm bias budget over N≈203 levels
is a network adjustment that includes the network's own structure
as a datum.

A database engine that hides its registry behind a bespoke
`SHOW BUNDLES` verb is the single-chain construction — the structure
is queryable only through one purpose-built path, and consumers who
want to compose `SHOW BUNDLES` with `RANK BY n_records DESC FIRST 10`
have to reimplement the clause vocabulary or post-process the
result. A database engine that treats its registry as a queryable
bundle is the network-adjustment construction — the structure is
data the engine can be queried about with the same vocabulary that
queries every other bundle, and `COVER __bundles__ WHERE
type='overlay' RANK BY n_records DESC FIRST 10` composes for free.

The pattern scales. Anywhere GIGI maintains a private registry —
lattices, gauge fields, sessions, prepared statements, triggers,
patterns, brain-primitive caches — the same virtual-bundle
construction exposes it as a first-class queryable surface. The
hosting moves the structure into the same vocabulary as the data,
which is the architectural commitment of every Davis Geometric
construction Bee has shipped since the foundational trio: bring the
inaccessible thing (curvature, holonomy, registry) onto the
substrate that already has the operators (cosine, transport,
COVER).

## Reserved-name protocol for future virtual bundles

When adding `__lattices__` / `__gauges__` / `__sessions__` /
similar:

1. Add the name to `virtual_bundles::reserved_names()`.
2. Add a `materialize_<thing>_rows(engine: &Engine) -> Vec<Record>`
   helper that walks the relevant registry.
3. Add a matching short-circuit branch in the `Statement::Cover`
   executor arm (next to the existing `__bundles__` branch).
4. Document the new schema in this file.
5. No write-guard changes needed — the existing
   `reject_virtual_write` calls already cover every write verb.

The pattern is intentionally repetitive: each new virtual bundle is
its own `materialize_X` + dispatch branch, not a generic registry
abstraction. The repetition keeps the surface honest about what
each virtual bundle returns; a generic abstraction would force
every new virtual bundle to fit the same row shape, which is
exactly the constraint Theorem 7 says we should not impose.

## See also

- `src/virtual_bundles.rs` — module surface + 5 unit tests
- `tests/gigi_hosting_itself.rs` — 8 integration tests
- `docs/CREATE_SESSION.md` — personal-list #2; will share the
  reserved-name protocol when canonical session bundles get a
  reserved prefix
- `theory/davis_geometric/pyramid_paper_v8.pdf` — Theorem 7, the
  structure-as-own-datum result
