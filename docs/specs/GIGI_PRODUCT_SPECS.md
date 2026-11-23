# GIGI Product Suite — Build Specifications

**Author:** Bee Rosa Davis · Davis Geometric
**Date:** March 17, 2026
**Status:** Build-Ready Specs

## Product Overview

Three products. One geometric framework. DHOOM serializes. GIGI stores. The math unifies.

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ GIGI Convert│     │ GIGI Stream │     │  GIGI Edge  │
│  Format Tool│────▶│  Cloud DB   │◀────│  Local-First│
│  CLI + API  │     │  WebSocket  │     │  Mobile/IoT │
└─────────────┘     └─────────────┘     └─────────────┘
       │                   │                    │
       └───────────────────┴────────────────────┘
                    DHOOM Wire Protocol
                    GIGI Bundle Engine
                    Davis Field Equations
```

---

## Product 1: GIGI Convert

### What It Is

A command-line tool and API that converts JSON/CSV/SQL dumps into DHOOM format.
Not just format conversion — geometric data profiling. It analyzes your data's
fiber bundle structure and tells you how much compression you'll get before encoding.

### CLI Interface

```bash
# Basic conversion
gigi convert input.json -o output.dhoom

# From stdin (piping)
curl https://api.example.com/users | gigi convert -o users.dhoom

# CSV input
gigi convert data.csv --format csv -o output.dhoom

# SQL dump
gigi convert dump.sql --format sql -o output.dhoom

# Profile only (no output, just analysis)
gigi convert input.json --profile

# Round-trip verification
gigi convert input.json -o output.dhoom
gigi convert output.dhoom --to json -o roundtrip.json
diff input.json roundtrip.json  # identical

# Streaming mode (line-delimited JSON)
tail -f events.jsonl | gigi convert --stream -o events.dhoom

# Specify collection name
gigi convert input.json -o output.dhoom --name "sensor_readings"

# Custom defaults (override auto-detection)
gigi convert input.json -o output.dhoom --default "status=normal" --default "unit=celsius"

# Force arithmetic on a field
gigi convert input.json -o output.dhoom --arithmetic "id@1" --arithmetic "ts@1710000000+60"
```

### Profile Output

```bash
$ gigi convert sensor_data.json --profile

GIGI Convert — Geometric Data Profile
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Collection:    sensor_data (50,000 records, 7 fields)

  Field Analysis:
    sensor_id     ARITHMETIC  @S-001+1     (50,000 values derived)
    timestamp     ARITHMETIC  @1710000000+60  (50,000 values derived)
    temperature   VARIABLE    range [14.2, 31.8]
    humidity      VARIABLE    range [28.1, 72.4]
    pressure      VARIABLE    range [1005.2, 1021.3]
    unit          CONSTANT    "metric"     (50,000 matches, 100%)
    status        DEFAULT     "normal"     (46,057 matches, 92.1%)

  Fiber Structure:
    Base space:     2 arithmetic fields (sensor_id, timestamp)
    Fiber:          5 value fields
    Zero section:   unit="metric", status="normal"
    Deviation rate:  7.9% of records deviate on status

  Compression Estimate:
    JSON (minified):  6,731,044 chars  (~1,819,201 tokens)
    DHOOM:            1,498,189 chars  (~404,915 tokens)
    Savings:          77.7% smaller    (77.7% fewer tokens)
    Fields omitted:   246,057 of 350,000 (70.3%)

  Curvature:
    K(temperature):   0.0012  (confidence: 0.9988)
    K(humidity):      0.0034  (confidence: 0.9966)
    K(pressure):      0.0002  (confidence: 0.9998)
    K(status):        0.0854  (confidence: 0.9215)

  Formula: fields_omitted ≥ (A×N) + (D×M)
           = (2 × 50000) + (2 × 46057) = 192,114 minimum
           Actual: 246,057 (trailing elision adds 53,943)
```

### API (HTTP)

```
POST /v1/convert
Content-Type: application/json

{
  "input": [...],           // JSON array of records
  "name": "sensor_data",    // optional collection name
  "options": {
    "profile": false,       // return profile alongside output
    "defaults": {},         // override auto-detected defaults
    "arithmetic": [],       // force arithmetic fields
    "format": "dhoom"       // output format: "dhoom" | "json"
  }
}

Response:
{
  "dhoom": "sensor_data{...}:\n...",
  "profile": {
    "records": 50000,
    "fields": 7,
    "arithmetic_fields": 2,
    "default_fields": 2,
    "compression_pct": 77.7,
    "token_savings_pct": 77.7,
    "json_chars": 6731044,
    "dhoom_chars": 1498189,
    "curvature": { ... }
  }
}
```

```
POST /v1/decode
Content-Type: text/plain

(raw DHOOM text)

Response:
{
  "records": [...],         // JSON array
  "collection": "sensor_data",
  "schema": { ... }
}
```

### Rust Crate API

```rust
use gigi_convert::{encode, decode, profile, Options};

// Encode
let json: Vec<serde_json::Value> = serde_json::from_str(&input)?;
let dhoom = encode(&json, "sensors", &Options::default())?;

// Decode
let records = decode(&dhoom_string)?;

// Profile
let report = profile(&json, "sensors")?;
println!("Compression: {:.1}%", report.compression_pct);
println!("Curvature: {:?}", report.curvature);

// Streaming encode
let encoder = StreamEncoder::new("events", schema, defaults);
for record in stream {
    encoder.push(&record)?;
    let dhoom_line = encoder.flush()?;
    send(dhoom_line);
}
```

### Build Checklist

- [ ] `gigi-convert` CLI binary (clap for arg parsing)
- [ ] JSON input parser (serde_json)
- [ ] CSV input parser (csv crate)
- [ ] Arithmetic detector (detect sequences in field values)
- [ ] Default detector (frequency analysis per field)
- [ ] Field orderer (variable first, defaults last)
- [ ] DHOOM encoder (header + rows with elision)
- [ ] DHOOM decoder (header parse + row expansion)
- [ ] Profile reporter (compression estimate + curvature)
- [ ] Round-trip verification test suite
- [ ] Streaming mode (line-delimited JSON input)
- [ ] HTTP API server (axum or actix-web)

---

## Product 2: GIGI Stream

### What It Is

A real-time geometric database as a service. Like Firebase, but:
- O(1) reads and writes (not O(log n))
- DHOOM wire protocol (66-84% smaller than JSON)
- Curvature-based confidence on every result
- Sheaf-guaranteed query composition
- Holonomy-based consistency monitoring

### Client SDK

```javascript
import { GIGIClient } from '@gigi-db/client';

// Connect
const db = new GIGIClient('wss://stream.gigi.dev/your-project', {
  apiKey: 'gigi_live_abc123'
});

// ─── Define a bundle (collection) ───
await db.bundle('sensors').create({
  schema: {
    sensor_id: 'string',
    timestamp: 'number',
    temperature: 'number',
    humidity: 'number',
    status: 'string',
  },
  keys: ['sensor_id', 'timestamp'],
  defaults: { status: 'normal' },
  arithmetic: { timestamp: { start: Date.now(), step: 60000 } },
});

// ─── Insert (O(1)) ───
await db.bundle('sensors').insert({
  sensor_id: 'T-001',
  timestamp: Date.now(),
  temperature: 22.4,
  humidity: 48.3,
  status: 'normal',
});

// ─── Point query (O(1)) ───
const reading = await db.bundle('sensors').get({
  sensor_id: 'T-001',
  timestamp: 1710000000,
});
console.log(reading.data);          // { sensor_id: 'T-001', ... }
console.log(reading.confidence);    // 0.9988
console.log(reading.curvature);     // 0.0012

// ─── Range query (O(|result|)) ───
const alerts = await db.bundle('sensors').where('status', 'alert');
console.log(alerts.length);         // only alert records
console.log(alerts.confidence);     // aggregate confidence

// ─── Pullback join (O(|left|)) ───
const enriched = await db.bundle('readings')
  .join(db.bundle('sensors'), 'sensor_id', 'sensor_id');

// ─── Aggregation (fiber integral) ───
const stats = await db.bundle('sensors').aggregate({
  groupBy: 'sensor_id',
  compute: { avg_temp: { fn: 'avg', field: 'temperature' } },
});

// ─── Real-time subscription ───
const unsub = db.bundle('sensors')
  .where('status', 'alert')
  .subscribe((alerts) => {
    // fires on every new alert
    // wire format is DHOOM — 78% fewer bytes than Firebase
    console.log(`${alerts.length} active alerts`);
    alerts.forEach(a => {
      console.log(`${a.data.sensor_id}: ${a.data.temperature}°C`);
      console.log(`  confidence: ${a.confidence}`);
    });
  });

// ─── Curvature monitoring ───
const health = await db.bundle('sensors').curvature('temperature');
console.log(health.K);              // 0.0012
console.log(health.confidence);     // 0.9988
console.log(health.capacity);       // τ/K

// ─── Consistency check (Čech cohomology) ───
const consistency = await db.bundle('sensors').checkConsistency();
console.log(consistency.h1);        // 0 = fully consistent
console.log(consistency.cocycles);  // [] = no issues

// ─── Disconnect ───
unsub();
await db.close();
```

### WebSocket Protocol

The WebSocket speaks DHOOM natively. Every message is a DHOOM-encoded bundle.

```
Client → Server:

  INSERT sensors
  sensors{sensor_id, timestamp, temperature, humidity, status|normal}:
  T-001, 1710000000, 22.4, 48.3

  QUERY sensors WHERE sensor_id = "T-001" AND timestamp = 1710000000

  RANGE sensors WHERE status = "alert"

  SUBSCRIBE sensors WHERE status = "alert"

  JOIN readings ON sensor_id = sensors.sensor_id

  CURVATURE sensors.temperature

Server → Client:

  RESULT sensors{sensor_id, timestamp, temperature, humidity, status|normal}:
  T-001, 1710000000, 22.4, 48.3
  META confidence=0.9988 curvature=0.0012 capacity=8333.33

  SUBSCRIPTION sensors ALERT
  sensors{sensor_id, timestamp, temperature, status}:
  T-003, 1710000120, 45.8, :alert
  META confidence=0.9215 curvature=0.0854

  CONSISTENCY h1=0 cocycles=0
```

### Server Architecture

```
┌─────────────────────────────────────────────┐
│                GIGI Stream Server            │
├─────────────────────────────────────────────┤
│                                              │
│  ┌──────────┐  ┌───────────┐  ┌──────────┐ │
│  │ WebSocket │  │  REST API │  │  Admin   │ │
│  │  Handler  │  │  Handler  │  │  Console │ │
│  └─────┬─────┘  └─────┬─────┘  └─────┬────┘ │
│        │              │              │       │
│  ┌─────▼──────────────▼──────────────▼────┐ │
│  │         DHOOM Parser / Encoder          │ │
│  │    (wire protocol serialization)        │ │
│  └─────────────────┬──────────────────────┘ │
│                    │                         │
│  ┌─────────────────▼──────────────────────┐ │
│  │         GIGI Bundle Engine              │ │
│  │  ┌──────────┐ ┌──────────┐ ┌────────┐ │ │
│  │  │ Bundle   │ │  Sheaf   │ │Connect-│ │ │
│  │  │ Store    │ │  Query   │ │ ion    │ │ │
│  │  │ (L1)     │ │  (L2)    │ │ (L3)   │ │ │
│  │  └──────────┘ └──────────┘ └────────┘ │ │
│  └─────────────────┬──────────────────────┘ │
│                    │                         │
│  ┌─────────────────▼──────────────────────┐ │
│  │           Persistence Layer             │ │
│  │   WAL + CRC32 + Compaction + Snapshots  │ │
│  └────────────────────────────────────────┘ │
│                                              │
│  ┌────────────────────────────────────────┐ │
│  │       Subscription Manager              │ │
│  │  (sheaf-evaluated change feeds)         │ │
│  └────────────────────────────────────────┘ │
│                                              │
├─────────────────────────────────────────────┤
│  Auth: API keys + JWT                        │
│  Rate limiting: token bucket per key         │
│  Metrics: Prometheus + curvature dashboard   │
└─────────────────────────────────────────────┘
```

### REST API

```
POST   /v1/bundles                    Create a bundle (collection)
DELETE /v1/bundles/:name              Drop a bundle

POST   /v1/bundles/:name/insert      Insert record(s)
GET    /v1/bundles/:name/get?key=...  Point query
GET    /v1/bundles/:name/range?...    Range query
POST   /v1/bundles/:name/join        Pullback join
POST   /v1/bundles/:name/aggregate   Fiber integral

GET    /v1/bundles/:name/curvature   Curvature report
GET    /v1/bundles/:name/consistency  Čech cohomology check
GET    /v1/bundles/:name/spectral    Spectral gap analysis

GET    /v1/health                     Server health
GET    /v1/metrics                    Prometheus metrics
```

### Subscription System

Subscriptions are sheaf evaluations on open sets. When a new record is inserted
that falls within the open set of a subscription, the subscriber is notified.

```
Client subscribes:  WHERE status = "alert"
  → Server registers open set U = {p ∈ B : status(σ(p)) = "alert"}

New record inserted with status = "alert"
  → Server checks: is the new base point p ∈ U?
  → Yes → push DHOOM-encoded record to subscriber
  → Include curvature and confidence metadata

This is O(1) per insert per subscription check (set membership).
```

### Build Checklist

- [ ] WebSocket server (tokio + tungstenite)
- [ ] DHOOM wire protocol parser/encoder
- [ ] GIGI bundle engine (reuse existing Rust crate)
- [ ] REST API layer (axum)
- [ ] Subscription manager (open set registration + push)
- [ ] Auth system (API keys + JWT)
- [ ] WAL persistence (reuse existing)
- [ ] JavaScript client SDK (@gigi-db/client npm package)
- [ ] Python client SDK (gigi-db PyPI package)
- [ ] Admin console (web UI for bundle management)
- [ ] Curvature dashboard (real-time K monitoring)
- [ ] Docker container for deployment
- [ ] Fly.io / Railway deployment config

---

## Product 3: GIGI Edge

### What It Is

A tiny GIGI engine that runs on-device (mobile, IoT, edge). Stores locally,
syncs to GIGI Stream when connected. The sheaf gluing axiom mathematically
guarantees that local writes compose correctly with the server state.

### Architecture

```
┌──────────────────────────────┐
│          GIGI Edge           │
│         (on device)          │
├──────────────────────────────┤
│                               │
│  ┌─────────────────────────┐ │
│  │   Local Bundle Engine   │ │
│  │   (same GIGI core)      │ │
│  │   O(1) read/write       │ │
│  └───────────┬─────────────┘ │
│              │                │
│  ┌───────────▼─────────────┐ │
│  │   Local WAL + Storage   │ │
│  │   (SQLite or flat file) │ │
│  └───────────┬─────────────┘ │
│              │                │
│  ┌───────────▼─────────────┐ │
│  │    Sync Queue           │ │
│  │    (DHOOM-encoded ops)  │ │
│  └───────────┬─────────────┘ │
│              │                │
│  ┌───────────▼─────────────┐ │
│  │   Sheaf Sync Engine     │ │
│  │   (gluing-guaranteed)   │ │
│  └───────────┬─────────────┘ │
│              │ WebSocket      │
└──────────────┼───────────────┘
               │
    ┌──────────▼──────────┐
    │    GIGI Stream      │
    │    (cloud)          │
    └─────────────────────┘
```

### SDK (Mobile / IoT)

```javascript
import { GIGIEdge } from '@gigi-db/edge';

// Initialize local engine
const local = new GIGIEdge({
  storage: './local.gigi',       // local file path
  remote: 'wss://stream.gigi.dev/your-project',
  apiKey: 'gigi_live_abc123',
  syncInterval: 5000,            // sync every 5s when online
});

// Works exactly like GIGI Stream — same API
await local.bundle('todos').insert({
  id: 'todo-1',
  title: 'File GIGI patent',
  done: false,
});

// Reads are always local (O(1), offline-capable)
const todo = await local.bundle('todos').get({ id: 'todo-1' });

// Sync happens automatically when online
local.on('sync', (report) => {
  console.log(`Synced ${report.pushed} records up`);
  console.log(`Pulled ${report.pulled} records down`);
  console.log(`Conflicts: ${report.h1}`);  // 0 = clean sync
});

// Manual sync
await local.sync();

// Offline mode — everything still works
// Writes queue locally, sync when reconnected
```

### Sync Protocol (Sheaf Gluing)

```
1. Client collects local writes since last sync
2. Encode as DHOOM bundle
3. Send to server: SYNC PUSH {dhoom_bundle}
4. Server applies writes to its bundle
5. Server computes Čech cohomology on overlap:
   - H¹ = 0 → clean merge, no conflicts
   - H¹ ≠ 0 → conflicts detected, return cocycle locations
6. Server sends back new records since last sync: SYNC PULL {dhoom_bundle}
7. Client applies pulled records locally
8. Both sides now have identical bundle state

The sheaf gluing axiom guarantees: if local sections agree with
server sections on their overlap (shared records), the combined
result is the unique correct global section.

No conflict resolution protocol needed. The math handles it.
When conflicts DO occur (H¹ ≠ 0), the cocycle tells you
exactly which records and which fields disagree.
```

### Build Checklist

- [ ] GIGI Edge core (embed GIGI bundle engine, no network dependency)
- [ ] Local storage adapter (SQLite or flat file)
- [ ] Sync queue (ordered DHOOM-encoded operations)
- [ ] Sheaf sync engine (push/pull with H¹ conflict detection)
- [ ] JavaScript SDK (@gigi-db/edge npm package)
- [ ] React Native adapter
- [ ] iOS Swift wrapper (via Rust FFI)
- [ ] Android Kotlin wrapper (via Rust FFI)
- [ ] Embedded C API (for IoT devices)
- [ ] Offline-first test suite (disconnect/reconnect scenarios)

---

## Pricing Tiers

| Feature | Free | Pro ($29/mo) | Team ($99/mo) | Enterprise |
|---|---|---|---|---|
| Projects | 1 | 5 | Unlimited | Unlimited |
| Records | 100K | 10M | 100M | Custom |
| Queries/day | 10K | Unlimited | Unlimited | Unlimited |
| Convert CLI | Yes | Yes | Yes | Yes |
| Convert API | 10MB/day | Unlimited | Unlimited | Unlimited |
| Stream (real-time) | — | Yes | Yes | Yes |
| Edge (local-first) | — | Yes | Yes | Yes |
| Subscriptions | — | 10 | Unlimited | Unlimited |
| Curvature dashboard | — | Yes | Yes | Yes |
| Čech consistency | — | — | Yes | Yes |
| Spectral analysis | — | — | Yes | Yes |
| SLA | — | — | 99.9% | Custom |
| On-prem | — | — | — | Yes |
| Dedicated infra | — | — | — | Yes |

### Why O(1) Changes the Pricing

Traditional databases charge per read because each read has infrastructure cost
proportional to database size (B-tree traversal scales with log(N), vector search
scales with log(N) or worse).

GIGI's O(1) guarantee means cost per query is constant regardless of database
size. A query on 10M records costs the same infrastructure as a query on 1K records.

This means:
- **Unlimited queries at Pro tier** — because each query costs us the same
- **Margins improve at scale** — the opposite of every other database company
- **No surprise bills** — predictable costs regardless of traffic spikes

---

## Domain Architecture

| Domain | Purpose |
|---|---|
| `gigi.dev` | Product home, documentation, pricing |
| `stream.gigi.dev` | GIGI Stream WebSocket endpoint |
| `api.gigi.dev` | GIGI Convert REST API |
| `console.gigi.dev` | Admin dashboard |
| `dhoom.dev` | DHOOM format specification + playground |
| `davisgeometric.com` | Company home |

---

## Sample Application Ideas

### 1. GIGI Chat (Real-time messaging)
Show GIGI Stream as a Firebase replacement. Messages are sections on a bundle.
Subscriptions notify on new messages. DHOOM wire = 70% less bandwidth than Firebase.

### 2. GIGI Sensors (IoT dashboard)
Show GIGI Convert + Stream. Ingest sensor JSON, store in GIGI, serve dashboard.
Curvature spikes = anomaly detection for free. Live confidence scores.

### 3. GIGI Tasks (Offline-first todo app)
Show GIGI Edge. Works offline. Syncs when online. Sheaf gluing = no conflicts.
H¹ = 0 after every sync. The todo app that's mathematically guaranteed.

### 4. GIGI Reconciler (Financial matching)
Show pullback joins. Two transaction feeds → pullback join → matched pairs.
Holonomy around settlement loop → unmatched = nonzero holonomy.

### 5. GIGI Context (LLM context manager)
Show GIGI Convert for LLM prompts. Store context in GIGI, serialize to DHOOM,
inject into prompts. 84% fewer tokens. Confidence per context chunk.

---

**GIGI** · Geometric Intrinsic Global Index · Davis Geometric · 2026
