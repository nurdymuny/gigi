# Getting started with GIGI

GIGI (Geometric Intrinsic Global Index) is a geometric database engine.
Every record is a section of a fiber bundle. Keys live on the base
space, values live on the fiber, and the bundle carries curvature,
spectral gap, holonomy, and confidence as first-class state that
updates on every insert and rides along on every query response.

This doc gets you running in three different shapes — pick the one
that matches what you came here to do:

- **Track 1** — query the public read-only instance with `curl`. No
  install. 10 minutes.
- **Track 2** — replicate the Halcyon SU(2) buckyball thermalization
  receipt against the same public instance. 15 minutes.
- **Track 3** — run your own GIGI server locally, with your own data
  directory and your own API key. 30 minutes.

You do not need to read the math to use the database. The geometric
quantities come back in every response whether you asked for them or
not. Read `theory/` after you've made a record stick.

---

## Before you start

A few facts that apply to all three tracks:

- The HTTP API uses `X-API-Key` for auth, not `Authorization: Bearer`.
  On the public read-only instance this header is not required for
  read endpoints. On your own deployment you set the key via the
  `GIGI_API_KEY` environment variable.
- Every error response has the shape `{"error": "<message>"}`.
- The public read-only instance is `https://gigi-stream.fly.dev`. Only
  read endpoints work there without auth — writes against the public
  instance will refuse. To insert your own data, run Track 3.
- The full OpenAPI document is served at `/v1/openapi.json` on any
  GIGI instance. The HTTP reference doc (`HTTP_API_REFERENCE.md`) is
  rendered from it.

---

## Track 1: Use GIGI as a local database (10 minutes, no install)

Goal: list bundles on the public instance, run a point query, run a
filtered query, get curvature and spectral statistics back, and call
one brain primitive (DREAM) to see anisotropic trajectory generation.

Everything here is read-only against `gigi-stream.fly.dev`. You do not
need an API key. Every command is one line.

### 1. Confirm the server is up

```bash
curl https://gigi-stream.fly.dev/v1/health
```

Expected response shape:

```json
{
  "status": "ok",
  "engine": "gigi",
  "version": "0.4.x",
  "bundles": <int>,
  "total_records": <int>
}
```

If `status` is `"ok"` you're done with step 1.

### 2. List the bundles currently published on the instance

```bash
curl https://gigi-stream.fly.dev/v1/bundles
```

You get back `{"data": [...], "meta": {...}}` where each entry has a
`name`, a `records` count, and a `fields` count. Note one bundle name
to use in the next steps — for the examples below, substitute its name
for `<bundle>`.

### 3. Look at the schema for one bundle

```bash
curl https://gigi-stream.fly.dev/v1/bundles/<bundle>/schema
```

Response carries the bundle name, an array of `base_fields` (the keys
that live on the base space), and an array of `fiber_fields` (the
values that live on the fiber). The split matters — base fields are
what you query *by*, fiber fields are what GIGI puts geometry *on*.

### 4. Read the geometric stats for the whole bundle

```bash
curl https://gigi-stream.fly.dev/v1/bundles/<bundle>/stats
```

You get `record_count`, `field_stats` (Welford-maintained mean, var,
range per fiber field), an overall `curvature` scalar for the bundle,
and the storage mode. These numbers update incrementally on every
insert — they are never "stale" the way a precomputed summary table
would be. The lookup itself is O(1).

### 5. Read the curvature report

```bash
curl https://gigi-stream.fly.dev/v1/bundles/<bundle>/curvature
```

Response shape:

```json
{
  "K": <number>,
  "confidence": <number>,
  "capacity": <number>,
  "per_field": [
    { "field": "<name>", "variance": <n>, "range": <n>, "k": <n> },
    ...
  ]
}
```

`confidence = 1 / (1 + K)` is the bundle-wide trust score that rides
along on every query response.

### 6. Read the spectral report

```bash
curl https://gigi-stream.fly.dev/v1/bundles/<bundle>/spectral
```

Response gives `lambda1` (the Fiedler value of the index Laplacian —
how clumpy or smooth the data is), the graph `diameter`, and a
`spectral_capacity` summary. This is the geometric answer to *"is my
data clustered?"* — no separate k-means run.

### 7. Run a filtered query

Replace `<field>` and `<value>` with names from step 3:

```bash
curl -X POST https://gigi-stream.fly.dev/v1/bundles/<bundle>/query \
  -H "Content-Type: application/json" \
  -d '{
    "conditions": [{"field": "<field>", "op": "eq", "value": "<value>"}],
    "limit": 5
  }'
```

Response is `{"data": [...records...], "meta": {curvature, confidence, count}}`.
The records look like ordinary JSON rows. The `meta` block is the
geometric annotation that came along for free.

### 8. Run a range query

```bash
curl "https://gigi-stream.fly.dev/v1/bundles/<bundle>/range?field=<numeric_field>&min=0&max=100"
```

Same shape — `{"data": [...], "meta": {...}}` — over records whose
`<numeric_field>` value sits in `[0, 100]`.

### 9. Call a brain primitive (DREAM)

DREAM walks a trajectory on the bundle's anisotropic fiber. It is a
*generative* read — it produces a synthesized record-shaped trajectory
that flows along the learned geometry rather than retrieving a stored
record. The endpoint is read-only in the sense that it does not mutate
the bundle.

```bash
curl -X POST https://gigi-stream.fly.dev/v1/bundles/<bundle>/brain/dream \
  -H "Content-Type: application/json" \
  -d '{"steps": 16, "temperature": 0.5}'
```

You get back a trajectory of fiber-valued points and per-step
curvature. If you see numbers, the brain layer is responding.

### What just happened

You hit a real geometric database from a cold start. The numbers that
came back — curvature, confidence, spectral gap, the DREAM trajectory
— were not computed on demand by a separate analytics layer. They are
the substrate. Every other query you make against any GIGI instance
will return the same shape.

To put your own records in, run Track 3. To go straight to the
Halcyon Yang-Mills receipt, do Track 2 next.

---

## Track 2: Replicate the Halcyon buckyball receipt (15 minutes)

Goal: declare a truncated-icosahedron lattice (the buckyball), put an
SU(2) gauge field on it, thermalize it with Gibbs sampling at the
canonical inverse temperature, snapshot the resulting field, and
verify the snapshot's SHA-256 matches the pinned receipt.

All five statements use the unified `/v1/gql` endpoint. The verbs are
parsed by `src/parser.rs` and dispatched by `src/halcyon_gql_dispatch.rs`.
Read endpoints work against the public instance without auth; the
write endpoints (`GAUGE_FIELD ... INIT ... PERSIST`, `GIBBS_SAMPLE`,
`SNAPSHOT ... PERSIST`) require running your own instance, so for the
declaration steps below run Track 3 first and replace the URL with
your local server.

### The canonical chain

The receipt is five GQL statements, executed in order against
`/v1/gql`. Each is a single POST.

```
LATTICE bb FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';
GAUGE_FIELD U ON LATTICE bb GROUP SU(2) INIT IDENTITY PERSIST;
GIBBS_SAMPLE U BETA 2.5 N_SWEEPS 200 SEED 20260616;
SNAPSHOT GAUGE_FIELD U PERSIST;
```

The `SEED` is load-bearing. Changing the seed changes every byte of
the resulting snapshot. The pinned seed is `20260616`.

### Step 1: declare the lattice

```bash
curl -X POST http://localhost:3142/v1/gql \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $GIGI_API_KEY" \
  -d '{"query": "LATTICE bb FROM TRUNCATED_ICOSAHEDRON TOPOLOGY '\''S2'\'';"}'
```

The truncated icosahedron has 60 vertices, 90 edges, and 32 faces
(12 pentagons + 20 hexagons). The `TOPOLOGY 'S2'` clause marks it as
the 2-sphere face-cycle table — closes Euler χ = 2.

### Step 2: declare the gauge field on the lattice

```bash
curl -X POST http://localhost:3142/v1/gql \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $GIGI_API_KEY" \
  -d '{"query": "GAUGE_FIELD U ON LATTICE bb GROUP SU(2) INIT IDENTITY PERSIST;"}'
```

`INIT IDENTITY` seeds every link with the SU(2) identity element.
`PERSIST` writes the gauge field to the lattice registry so subsequent
verbs can find it by name. Without `PERSIST` the field would live only
in the dispatch frame for this one statement.

### Step 3: thermalize with Gibbs sampling

```bash
curl -X POST http://localhost:3142/v1/gql \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $GIGI_API_KEY" \
  -d '{"query": "GIBBS_SAMPLE U BETA 2.5 N_SWEEPS 200 SEED 20260616;"}'
```

This runs 200 heatbath sweeps at β = 2.5. Each sweep visits every link
and resamples it from the conditional Boltzmann distribution given its
staples. The seed forces a bit-deterministic trajectory.

Response carries the sweep count and (if you add a `MEASURE` clause)
the per-sweep mean plaquette trajectory.

### Step 4: snapshot the field

```bash
curl -X POST http://localhost:3142/v1/gql \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $GIGI_API_KEY" \
  -d '{"query": "SNAPSHOT GAUGE_FIELD U PERSIST;"}'
```

`SNAPSHOT GAUGE_FIELD U` without `PERSIST` will parse-error per the
Part V locked decision — snapshots are reproducibility receipts, so
they must commit to disk to be valid.

### Step 5: verify the receipt

The response from step 4 includes the SHA-256 of the snapshot bytes.
Compare it against the pinned canonical hash:

```
ea7b934ca3fbe9897e9f11851647388972004a2ca025100179a92dd966516591
```

If the two hex strings match, you have bit-identity with the Halcyon
canonical receipt. Bit-identity means: same lattice topology, same
gauge group representation, same RNG stream, same heatbath kernel,
same serialization. A single drift anywhere in that pipeline changes
the hash.

### What this proves

Reproducibility at the byte level for a lattice gauge theory
computation, declared in four lines of GQL on top of a database
substrate. Halcyon Parts I–V live on production. Per-part gold gates
are pinned in `theory/halcyon/`.

---

## Track 3: Run your own GIGI instance (30 minutes)

Goal: build GIGI from source, start the HTTP server, write data, and
keep it across restarts.

### Prerequisites

- Rust 2021 edition, stable toolchain. `rustc --version` should print
  1.75 or newer.
- About 1 GB of disk for the build directory.
- A free TCP port. Default is 3142.

### 1. Build

```bash
git clone https://github.com/davis-geometric/gigi.git
cd gigi
cargo build --release
```

For the full surface (Kähler brain primitives, Halcyon lattice/gauge,
IMAGINE/WISH, transactions) use:

```bash
cargo build --release --features "kahler halcyon imagine wish transactions"
```

The base build with no features is a complete CRUD engine on its own.
Features are additive — see `STABILITY_GUARANTEES.md` for which
features are production-stable vs research-stage.

### 2. Configure via environment variables

The minimum a fresh instance needs is a data directory and a secret
seed for app-bundle encryption. Both have safe defaults but you should
set them explicitly before persisting anything you care about.

```bash
export PORT=3142
export GIGI_DATA_DIR=$HOME/gigi_data
export JG_KV_ENCRYPTION_SEED=$(openssl rand -hex 32)
```

Optional, recommended for any deployment that anyone else can reach:

```bash
export GIGI_API_KEY=$(openssl rand -hex 32)
```

When `GIGI_API_KEY` is set, every endpoint except `/v1/health` and
`/v1/openapi.json` requires the matching `X-API-Key` header. When it
is unset, all endpoints are open.

Other env vars: `GIGI_JWT_SECRET` (JWT verification for multi-tenant),
`GIGI_APP_BUNDLES` (JSON manifest of bundles to create on startup),
`GIGI_QUERY_MAX_ROWS` (per-query row cap), `GIGI_CORS_ORIGIN` (set to
`*` for dev or a domain for production). The S3 sync block
(`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_ENDPOINT_URL_S3`,
`AWS_REGION`, `BUCKET_NAME`) is only needed for Fly.io-style Tigris
snapshot sync at startup. Skip it for local runs.

### 3. Start the server

```bash
cargo run --release --bin gigi_stream
```

Or, if you built with features:

```bash
cargo run --release --features "kahler halcyon imagine wish transactions" \
  --bin gigi_stream
```

The server binds on `0.0.0.0:$PORT`. First-time startup creates
`$GIGI_DATA_DIR` if it does not exist.

### 4. Confirm it's listening

In another shell:

```bash
curl http://localhost:3142/v1/health
```

You should get the same `{"status": "ok", "engine": "gigi", ...}`
shape from Track 1.

### 5. Create a bundle

```bash
curl -X POST http://localhost:3142/v1/bundles \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $GIGI_API_KEY" \
  -d '{
    "name": "sensors",
    "schema": {
      "fields": {
        "id": "text",
        "ts": "timestamp",
        "temperature": "float",
        "humidity": "float"
      },
      "keys": ["id"],
      "indexed": ["ts"]
    }
  }'
```

Response: `{"status": "created", "bundle": "sensors"}` on success, or
`{"error": "..."}` with HTTP 409 if the bundle already exists.

### 6. Insert records

```bash
curl -X POST http://localhost:3142/v1/bundles/sensors/insert \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $GIGI_API_KEY" \
  -d '{
    "records": [
      {"id": "s001", "ts": "2026-06-20T10:00:00Z", "temperature": 21.4, "humidity": 0.55},
      {"id": "s002", "ts": "2026-06-20T10:05:00Z", "temperature": 21.6, "humidity": 0.56},
      {"id": "s003", "ts": "2026-06-20T10:10:00Z", "temperature": 99.9, "humidity": 0.04}
    ]
  }'
```

Response carries `inserted`, `total`, and the updated bundle-wide
`curvature` and `confidence`. The third record has anomalous values —
watch the `curvature` move.

### 7. Read it back

Point query:

```bash
curl "http://localhost:3142/v1/bundles/sensors/get?id=s001" \
  -H "X-API-Key: $GIGI_API_KEY"
```

Filtered query:

```bash
curl -X POST http://localhost:3142/v1/bundles/sensors/query \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $GIGI_API_KEY" \
  -d '{"conditions": [{"field": "temperature", "op": "gt", "value": 30}]}'
```

The anomalous record should come back, with the bundle's curvature
reflecting the spike.

### 8. Restart and verify persistence

Stop the server (`Ctrl+C`), restart with the same command, and rerun
step 7. The data should still be there. Snapshots in `$GIGI_DATA_DIR`
are mmap-loaded in seconds on restart.

### 9. Find the API reference for everything else

```bash
curl http://localhost:3142/v1/openapi.json
```

This is the source of truth for every endpoint, request body, and
response shape. `HTTP_API_REFERENCE.md` is rendered from it.

### Running in a container

A `Dockerfile` in the repo builds and runs the same binary. Set the
same environment variables on the container and mount a volume for
`GIGI_DATA_DIR`. The Fly.io configuration in `fly.toml` is a working
production template — it sets up the S3 sync block and a non-default
data dir.

### Embedding GIGI in a Rust application

If you want the engine in-process instead of behind HTTP, add `gigi`
as a dependency and use the engine API directly. The kitchen-sink
example is `examples/brain_tour_demo.rs` — every brain primitive on
one bundle in one run. To see it:

```bash
cargo run --release --features kahler --bin brain_tour_demo
```

---

## Where to go next

- **`STABILITY_GUARANTEES.md`** — which features are production-stable
  vs research-stage. Read this before depending on a feature in
  production code.
- **`HTTP_API_REFERENCE.md`** — every endpoint, request body, response
  shape, error code, and `curl` example. Rendered from `openapi.json`.
- **`theory/`** — the math. Start with `theory/SPECS_INDEX.md` for the
  per-feature spec map. Read after you've made a record stick.
- **`examples/`** — runnable examples. `brain_tour_demo.rs` covers
  every brain primitive. `causal_states_scan.rs` is a good entry to
  the commutator substrate.
- **`README.md`** — the long-form pitch, the side-by-side comparison
  with Postgres / MySQL / Mongo, and the licensing terms.
