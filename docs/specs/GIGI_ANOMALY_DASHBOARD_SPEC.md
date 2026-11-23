# GIGI Anomaly Detection API + Live Dashboard

## Geometric Curvature Intelligence — Specification & TDD

**Author:** Bee Rosa Davis · Davis Geometric  
**Date:** March 20, 2026  
**Version:** 0.1  
**Features:** `anomaly-detection-api` + `gigi-live-dashboard`

---

## Why B and D Are the Same Feature

Option B (Anomaly Detection API) and Option D (GIGI Live Dashboard) are not
two separate features — they are the **same geometric event seen at two
different timescales**.

Every GIGI insert computes local curvature K(p). When K(p) exceeds the
adaptive threshold, the record is geometrically anomalous. That fact has two
natural consumers:

1. **The API consumer** (Option B) — querying for anomalies after the fact.  
   "Show me every record where local K exceeded 2σ."

2. **The dashboard consumer** (Option D) — watching anomalies materialize in
   real time.  
   "Show me the curvature spike that just happened on the `sensors` bundle."

The REST endpoints power the dashboard. The WebSocket stream powers live
updates. The math is identical — one derivation, two surfaces.

```
                   ┌─────────────────────┐
                   │   INSERT σ(p)       │
                   │   compute K(p)      │
                   └────────┬────────────┘
                            │
              K(p) > μ + nσ ?
              ┌─────────────┴─────────────┐
              YES                        NO
              │                           │
     ┌────────▼────────┐        ┌─────────▼────────┐
     │ Anomaly Event   │        │ Normal Insert     │
     │ emitted to WS   │        │ confidence ≈ 1    │
     └────────┬────────┘        └──────────────────┘
              │
    ┌──────────┴──────────┐
    │                     │
    ▼                     ▼
REST API              WebSocket
POST /anomalies   /v1/ws/dashboard
(query)           (real-time stream)
```

---

## Mathematical Foundation

All anomaly detection in GIGI is derived from the fiber bundle geometry
described in GIGI_SPEC_v0.1. This section states the precise definitions
used by both the API and the dashboard.

### M.1  Local Curvature K(p)

Per **Definition 3.3** of the spec, the local curvature at base point p is:

```
K(p) = w_σ · σ̃(p) + w_ρ · ρ(p) + w_κ · κ(p)
```

where:

| Component | Symbol | Meaning |
|-----------|--------|---------|
| Saturation | σ̃(p) | fraction of neighbourhood N(p) that has been populated |
| Scarcity   | ρ(p)  | 1 − (distinct values in N(p)) / |F_max| |
| Coupling norm | κ(p) | mean fiber overlap between p and its neighbours |

Equal weighting: w_σ = w_ρ = w_κ = 1/3.

K(p) ∈ [0, 1] by construction.  
K(p) ≈ 0 → uniform neighbourhood → high confidence.  
K(p) >> 0 → variable neighbourhood → anomaly candidate.

### M.2  Deviation from the Zero Section

Per **Definition 1.4**, the deviation at point p is:

```
δ(p) = σ(p) − σ₀(p)
```

The deviation norm counts how many fields are non-default:

```
‖δ(p)‖ = #{i : δᵢ(p) ≠ 0}
```

A high deviation norm signals a record that is "far" from the modal pattern,
regardless of the magnitude of individual field differences.

### M.3  Deviation Magnitude (Fiber Metric Distance)

Per **Definition 1.7**, the fiber metric distance from the zero section is:

```
d_F(σ(p), σ₀(p)) = √( Σᵢ ωᵢ · gᵢ(σᵢ(p), σ₀ᵢ)² )
```

For numeric fields: gᵢ(a, b) = |a − b| / range(Fᵢ) (normalised).  
For categorical fields: gᵢ(a, b) = 0 if a = b, else 1.

`d_F` is the geometric distance between a record and its default — a
scale-invariant measure of "how far out" the record sits.

### M.4  Confidence Score

Per **Corollary 3.3**:

```
confidence(p) = 1 / (1 + K(p))
```

Confidence ∈ (0, 1]. It requires no calibration — it falls out of the
curvature computation that was already paid for on insert.

### M.5  Adaptive Anomaly Threshold

The global curvature distribution over all populated base points has an
empirical mean μ_K and standard deviation σ_K. An anomaly is any record
whose local curvature exceeds:

```
K_threshold(n) = μ_K + n · σ_K
```

The default is n = 2 (2-sigma), matching the convention in classical
statistical process control.

The **z-score** of a record is:

```
z(p) = (K(p) − μ_K) / σ_K
```

High z-score = geometric outlier. The threshold determines which z-scores
are reported.

### M.6  Contributing Fields

For a record flagged as an anomaly, the contributing fields are those whose
component of the curvature `κ(p)` is above average. Specifically, field f
is a **contributing field** for record p if:

```
gᵢ(σᵢ(p), mode_f) > μ_g(field f)
```

where mode_f is the most common value of field f across N(p), and μ_g is
the mean per-field distance in the neighbourhood.

In plain English: a field "contributes" to an anomaly when the record's
value for that field is further from the neighbourhood's modal value than
most other records are.

### M.7  Bundle Curvature Moments

The global curvature moments are tracked incrementally:

```
μ_K = (1/N) Σ_{p} K(p)          [mean]
σ_K = √( (1/N) Σ_{p} (K(p)−μ_K)² )  [std dev]
Ĥ¹  = dim of Čech 1-cohomology    [consistency]
λ₁   = spectral gap of field-index Laplacian  [connectivity]
C    = τ/K                          [Davis capacity]
```

All five are updated on every insert in O(1) amortized time.

---

## Part I — Anomaly Detection REST API

### Endpoint 1.1 — Query Anomalies

```
POST /v1/bundles/{name}/anomalies
```

Returns every record in the bundle whose local curvature exceeds the
adaptive threshold, ranked by z-score descending.

**Request body:**
```json
{
  "threshold_sigma": 2.0,
  "filters": [
    { "field": "city", "op": "eq", "value": "Moscow" }
  ],
  "fields": ["temperature", "humidity"],
  "limit": 100,
  "include_scores": true
}
```

| Field | Type | Default | Notes |
|-------|------|---------|-------|
| `threshold_sigma` | float | 2.0 | number of σ above μ_K to flag |
| `filters` | ConditionSpec[] | [] | pre-filter records before scoring (uses existing sheaf query) |
| `fields` | string[] | all | restrict contributing-field analysis to these fields |
| `limit` | int | 100 | max anomalies returned |
| `include_scores` | bool | true | include z_score, curvature, confidence in each record |

**Response:**
```json
{
  "bundle": "sensors",
  "anomalies": [
    {
      "record": { "city": "Moscow", "date": "2026-01-04", "temperature": -31.9 },
      "local_curvature": 0.089,
      "z_score": 5.30,
      "confidence": 0.918,
      "deviation_norm": 2,
      "deviation_distance": 0.73,
      "contributing_fields": ["temperature"],
      "neighbourhood_size": 366
    }
  ],
  "total_anomalies": 47,
  "anomaly_rate": 0.0064,
  "threshold_used": 0.071,
  "threshold_sigma": 2.0,
  "bundle_stats": {
    "global_k_mean": 0.031,
    "global_k_std": 0.020,
    "global_confidence": 0.969,
    "total_records": 7320,
    "cech_h1": 0,
    "spectral_gap": 0.0,
    "capacity": 2.89
  }
}
```

### Endpoint 1.2 — Bundle Health

```
GET /v1/bundles/{name}/health
```

Full geometric diagnostics for a bundle. No body required.

**Response:**
```json
{
  "bundle": "sensors",
  "records": 7320,
  "global_curvature": 0.0346,
  "global_confidence": 0.9665,
  "global_k_mean": 0.031,
  "global_k_std": 0.020,
  "capacity": 2.89,
  "spectral_gap": 0.0,
  "connected_components": 20,
  "cech_h1": 0,
  "anomaly_count": 47,
  "anomaly_rate": 0.0064,
  "anomaly_threshold": 0.071,
  "fields": {
    "temperature": {
      "curvature": 0.0087,
      "confidence": 0.9913,
      "anomaly_count": 43,
      "range": [-31.9, 32.5],
      "mode": 22.4
    },
    "humidity": {
      "curvature": 0.0034,
      "confidence": 0.9966,
      "anomaly_count": 11,
      "range": [10.0, 99.0],
      "mode": 48.3
    }
  },
  "top_anomalies": [
    {
      "record": { "city": "Moscow", "date": "2026-01-04", "temperature": -31.9 },
      "z_score": 5.30,
      "contributing_fields": ["temperature"]
    }
  ],
  "computed_in_ms": 1.2
}
```

### Endpoint 1.3 — Predict Volatility

```
POST /v1/bundles/{name}/predict
```

Uses the curvature K(p) of each GROUP BY partition to predict which groups
are stable vs. volatile. Implements the statistical mechanics from
**Definition 3.7** — each group's curvature determines its effective
temperature and expected anomaly rate.

**Request:**
```json
{
  "group_by": "city",
  "field": "temperature"
}
```

**Response:**
```json
{
  "bundle": "sensors",
  "group_by": "city",
  "field": "temperature",
  "predictions": [
    {
      "group": "Moscow",
      "curvature": 0.0233,
      "confidence": 0.9772,
      "z_score_from_global": 1.88,
      "prediction": "HIGH_VOLATILITY",
      "expected_anomaly_rate": 0.041,
      "partition_function_Z": 14.3,
      "records": 366
    },
    {
      "group": "Singapore",
      "curvature": 0.0001,
      "confidence": 0.9999,
      "z_score_from_global": -1.55,
      "prediction": "STABLE",
      "expected_anomaly_rate": 0.000,
      "partition_function_Z": 1.0,
      "records": 366
    }
  ],
  "method": "curvature_ranking",
  "partition_function_formula": "Z(β,p) = Σ_{q∈N(p)} exp(-β·d(p,q))"
}
```

The `prediction` label is assigned by curvature z-score:

| z_score_from_global | Label |
|---------------------|-------|
| > +2σ | `HIGH_VOLATILITY` |
| +1σ to +2σ | `ELEVATED` |
| −1σ to +1σ | `NORMAL` |
| < −1σ | `STABLE` |

### Endpoint 1.4 — Field Anomalies

```
POST /v1/bundles/{name}/anomalies/field
```

Anomaly detection scoped to a single field. Returns records whose
per-field curvature contribution is highest.

**Request:**
```json
{
  "field": "temperature",
  "threshold_sigma": 2.0,
  "limit": 50
}
```

**Response:**
```json
{
  "field": "temperature",
  "field_curvature": 0.0087,
  "field_confidence": 0.9913,
  "anomalies": [
    {
      "record": { "city": "Moscow", "date": "2026-01-04", "temperature": -31.9 },
      "field_value": -31.9,
      "deviation_from_mode": 54.3,
      "z_score": 5.30
    }
  ]
}
```

---

## Part I — TDD: Anomaly Detection API

All test IDs are prefixed `AD-` for Anomaly Detection.

### AD-1  Curvature Math

**AD-1.1** — Insert 100 identical records (same field values). Query anomalies.
Verify: `total_anomalies = 0`. K(p) for each record < μ_K + 2σ_K.

**AD-1.2** — Insert 99 identical records. Insert 1 record with all fields at
extreme values. Query anomalies with `threshold_sigma: 2.0`.
Verify: the 1 outlier is returned. Its `z_score > 2.0`.

**AD-1.3** — For AD-1.2 outlier, verify `confidence < 0.5` and
`confidence = 1 / (1 + K(p))` to within float tolerance.

**AD-1.4** — Verify `deviation_norm` for the AD-1.2 outlier equals the number
of fields that differ from the default values.

**AD-1.5** — Verify `deviation_distance` satisfies:
`0.0 ≤ deviation_distance ≤ √(k)` where k = number of numeric fields.

**AD-1.6** — Insert records with 1, 2, 3, all fields deviating from defaults.
Verify `deviation_norm` = 1, 2, 3, k respectively.

### AD-2  Adaptive Threshold

**AD-2.1** — Insert a Gaussian-distributed dataset (1000 records, normal field
values, mean μ, std σ). Query with `threshold_sigma: 2.0`.
Verify: `anomaly_rate` ≈ 0.045 (matches Gaussian 2σ tail).

**AD-2.2** — Query same dataset with `threshold_sigma: 3.0`.
Verify: `anomaly_rate` ≈ 0.003.

**AD-2.3** — Query same dataset with `threshold_sigma: 1.0`.
Verify: `anomaly_rate > 0.15`.

**AD-2.4** — Verify `threshold_used = global_k_mean + threshold_sigma * global_k_std`.

**AD-2.5** — Insert 10 identical records then 1 outlier. Verify that the
`global_k_std` increases after the outlier insert and the adaptive
threshold adjusts accordingly.

### AD-3  Contributing Fields

**AD-3.1** — Insert a record that deviates only on field `temperature`.
Verify: `contributing_fields = ["temperature"]`.

**AD-3.2** — Insert a record that deviates on both `temperature` and `humidity`.
Verify: both appear in `contributing_fields`.

**AD-3.3** — Request anomalies with `fields: ["temperature"]` (field restriction).
Verify: `contributing_fields` only contains `temperature`, never other fields.

### AD-4  Filters (Sheaf Pre-filter)

**AD-4.1** — Insert anomalies in two cities: Moscow and Singapore.
Query with `filters: [{ field: city, op: eq, value: Moscow }]`.
Verify: only Moscow records appear in anomalies.
Verify: Singapore anomaly count is not in response.

**AD-4.2** — Verify filtered anomalies still have valid `z_score` and
`local_curvature` fields.

**AD-4.3** — Query with empty `filters`. Verify all anomalies returned.

### AD-5  Bundle Health Endpoint

**AD-5.1** — GET /health on a fresh empty bundle.
Verify: `records = 0`, `anomaly_count = 0`,
`global_curvature = 0.0`, `global_confidence = 1.0`.

**AD-5.2** — Insert 1000 records. GET /health.
Verify: `records = 1000`, all stats present and non-null.

**AD-5.3** — Verify `fields` map contains one entry per schema field.

**AD-5.4** — Verify `top_anomalies` list is sorted descending by `z_score`.

**AD-5.5** — Verify `capacity = τ / global_curvature` where τ is the
configured tolerance budget (default τ = 0.1).

**AD-5.6** — Insert consistent data with no conflicts. Verify `cech_h1 = 0`.

### AD-6  Predict Endpoint

**AD-6.1** — Predict on a bundle with 3 cities: uniform city A, variable city B,
extreme outlier city C. Verify: A is "STABLE", B is "NORMAL" or "ELEVATED",
C is "HIGH_VOLATILITY".

**AD-6.2** — Verify `partition_function_Z` for the uniform group ≈ 1.0
(when d(p,q) ≈ 0 for all q ∈ N(p), Z = |N| · exp(0) / normaliser,
but in the extreme stable case, all distances = 0 so Z collapses to 1.0).

**AD-6.3** — Verify `expected_anomaly_rate` for "STABLE" group < 0.01.

**AD-6.4** — Verify `expected_anomaly_rate` for "HIGH_VOLATILITY" group > 0.02.

**AD-6.5** — Predict with `group_by` on a field with a single value.
Verify: one prediction group returned, labelled generically.

### AD-7  End-to-End Timing

**AD-7.1** — POST /anomalies on a 100K-record bundle.
Verify: response time < 100ms (anomaly scoring is O(N_anomalies), not O(N)).

**AD-7.2** — GET /health on a 100K-record bundle.
Verify: `computed_in_ms < 10` (all moments already tracked incrementally).

---

## Part II — GIGI Live Dashboard

### Architecture Overview

The dashboard consists of two layers:

1. **WebSocket event stream** — a server-side push mechanism built into
   `gigi_stream.rs`. Every anomaly, curvature update, and insert event is
   fanned out to connected subscribers.

2. **Dashboard UI** — a minimal single-page HTML/JS application served by
   `gigi_stream.rs` itself at `GET /dashboard`. No build step required.
   React is loaded from a CDN. The charts are rendered with Canvas API.

The two layers are connected by the same mathematical event objects. There
is no "dashboard schema" separate from the geometric event schema.

```
gigi_stream.rs
├── GET  /dashboard          → serves dashboard.html (embedded)
├── GET  /v1/ws/dashboard    → WebSocket subscription (all bundles)
├── GET  /v1/ws/{bundle}     → WebSocket subscription (one bundle)
├── POST /v1/bundles/{name}/anomalies   → Endpoint 1.1
├── GET  /v1/bundles/{name}/health      → Endpoint 1.2
├── POST /v1/bundles/{name}/predict     → Endpoint 1.3
└── POST /v1/bundles/{name}/anomalies/field → Endpoint 1.4
```

The WebSocket endpoints share the same `DashboardBroadcaster` — an async
broadcast channel (tokio::sync::broadcast) owned by the AppState. Every
insert, delete, and anomaly detection writes to the broadcaster. Connected
WebSocket clients receive the events.

### WebSocket Subscription

```
GET /v1/ws/dashboard
Upgrade: websocket
```

Subscribe to all bundles. No auth in v0.1. Future: bearer token in header.

```
GET /v1/ws/{bundle}
Upgrade: websocket
```

Subscribe to a single bundle only.

After upgrade, the server sends a **welcome frame** immediately:

```json
{
  "type": "welcome",
  "bundles": ["sensors", "orders", "users"],
  "server_version": "0.7.0",
  "ts": 1742400000000
}
```

Then it streams events as they occur. Clients can also send a **ping** to
keep the connection alive:

```json
{ "type": "ping" }
```

Server replies:

```json
{ "type": "pong", "ts": 1742400000000 }
```

### Event Schema

All events share a common envelope:

```json
{
  "type": "<event_type>",
  "bundle": "<bundle_name>",
  "ts": 1742400000123
}
```

The `ts` is Unix epoch in milliseconds (u64).

#### Event: `insert`

Emitted after every successful record insert. Carries the updated bundle
curvature moments and whether the inserted record is anomalous.

```json
{
  "type": "insert",
  "bundle": "sensors",
  "ts": 1742400000123,
  "record_count": 7321,
  "is_anomaly": false,
  "local_curvature": 0.008,
  "confidence": 0.992,
  "global_k_mean": 0.031,
  "global_k_std": 0.020,
  "global_confidence": 0.969
}
```

#### Event: `anomaly`

Emitted when an inserted record's K(p) exceeds the 2σ threshold.
`insert` is also emitted — `anomaly` is the additional notification.

```json
{
  "type": "anomaly",
  "bundle": "sensors",
  "ts": 1742400000124,
  "record": {
    "city": "Moscow",
    "date": "2026-01-04",
    "temperature": -31.9
  },
  "local_curvature": 0.089,
  "z_score": 5.30,
  "confidence": 0.918,
  "deviation_norm": 2,
  "contributing_fields": ["temperature"]
}
```

#### Event: `bundle_health`

Emitted periodically (every 5 seconds) or on demand (client sends
`{ "type": "subscribe_health", "bundle": "sensors" }`). Contains the
same payload as `GET /v1/bundles/{name}/health`.

```json
{
  "type": "bundle_health",
  "bundle": "sensors",
  "ts": 1742400005000,
  "records": 7321,
  "global_curvature": 0.0347,
  "global_confidence": 0.9664,
  "anomaly_count": 47,
  "anomaly_rate": 0.0064,
  "capacity": 2.88,
  "cech_h1": 0,
  "spectral_gap": 0.0
}
```

#### Event: `curvature_update`

Emitted when the global curvature distribution shifts by > 5% (i.e.,
`|μ_K_new - μ_K_old| / μ_K_old > 0.05`). This prevents flooding the
stream with trivial updates while ensuring the dashboard tracks real shifts.

```json
{
  "type": "curvature_update",
  "bundle": "sensors",
  "ts": 1742400000200,
  "old_k_mean": 0.030,
  "new_k_mean": 0.032,
  "delta_pct": 6.7,
  "new_threshold": 0.072,
  "new_confidence": 0.968
}
```

#### Event: `delete`

Emitted after a record deletion. Includes updated moments.

```json
{
  "type": "delete",
  "bundle": "sensors",
  "ts": 1742400000999,
  "record_count": 7320,
  "global_k_mean": 0.031,
  "global_confidence": 0.969
}
```

#### Event: `consistency_alert`

Emitted when Čech H¹ becomes nonzero after an operation that creates a
data consistency violation. This is the geometric early-warning system.

```json
{
  "type": "consistency_alert",
  "bundle": "sensors",
  "ts": 1742400001500,
  "cech_h1": 1,
  "cech_h1_prev": 0,
  "message": "Consistency violation detected: 1 independent inconsistency introduced"
}
```

### Dashboard UI (`GET /dashboard`)

The HTML/JS dashboard is embedded as a string constant in `gigi_stream.rs`
and served at `GET /dashboard` with `Content-Type: text/html`. No external
file I/O at runtime.

**URL:** http://localhost:7327/dashboard

#### Layout

```
┌─────────────────────────────────────────────────────────────────┐
│   GIGI Live Dashboard        [bundle selector: ▼ sensors]       │
├──────────────┬──────────────┬──────────────┬────────────────────┤
│  Records     │  Anomalies   │  Confidence  │  Curvature K       │
│  7,321       │  47  (0.64%) │  96.64%      │  0.0346            │
├──────────────┴──────────────┴──────────────┴────────────────────┤
│                                                                 │
│         Curvature over time (rolling 60s)                       │
│  0.12 ┤                                          ▆              │
│  0.08 ┤              ▃                    ▄  ▆▅ ██              │
│  0.04 ┤  ▂▁▂▁▂▁▂▁▂▁▂▂█▂▁▂▁▂▁▂▁▂▁▂▃▂▁▂▁▂▁▂▃▂▂▁██ ██              │
│  0.00 └──────────────────────────────────────────────           │
│                                                                 │
├──────────────────────────────────┬──────────────────────────────┤
│  Recent Anomalies (live)         │  Per-Field Curvature         │
│  ──────────────────────────────  │  temperature  ████░░░░ 0.009 │
│  ⚠ Moscow -31.9°C z=5.30        │  humidity     ███░░░░░ 0.003 │
│  ⚠ Moscow -31.6°C z=5.26        │  wind         ██░░░░░░ 0.002 │
│  ⚠ Moscow -29.4°C z=4.87        │  pressure     █░░░░░░░ 0.001 │
│  [Load more...]                  │                              │
└──────────────────────────────────┴──────────────────────────────┘
```

#### Dashboard Panels

| Panel | Data Source | Update Trigger |
|-------|-------------|----------------|
| Records counter | `insert` / `delete` events | every insert |
| Anomalies counter | `anomaly` events accumulated | every anomaly |
| Confidence gauge | `insert` events `.global_confidence` | every insert |
| Curvature K gauge | `insert` events `.global_k_mean` | every insert |
| Curvature time-series | ring buffer of last 3600 `insert` events | every insert |
| Anomaly feed | `anomaly` events, newest-first, max 50 | every anomaly |
| Per-field curvature bars | `bundle_health` events `.fields` | every 5s |

The curvature time-series is a Canvas 2D chart: no charting library.
It is drawn by `renderCurvatureChart(ctx, ringBuffer)` — a pure function
operating on the ring buffer of floats.

The colour scheme:
- **Green** (`#00ff88`) — confidence > 0.95 (low curvature, stable)
- **Yellow** (`#ffcc00`) — confidence 0.80–0.95 (moderate curvature)
- **Orange** (`#ff8800`) — confidence 0.60–0.80 (elevated curvature)
- **Red** (`#ff2244`) — confidence < 0.60 (high curvature, alert state)

The gauge backgrounds (record count, confidence, K) animate their colour
through the same scale, giving an at-a-glance "traffic light" health
signal.

#### Client-Side WebSocket Logic

```javascript
const ws = new WebSocket(`ws://${location.host}/v1/ws/${bundleName}`);
const ring = new Float32Array(3600);   // 1 hour at 1 insert/sec
let ringHead = 0;
let state = { records: 0, anomalies: 0, k_mean: 0, confidence: 1.0 };

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);

  if (msg.type === "insert") {
    state.records    = msg.record_count;
    state.k_mean     = msg.global_k_mean;
    state.confidence = msg.global_confidence;
    ring[ringHead++ % 3600] = msg.global_k_mean;
    updateGauges(state);
    renderCurvatureChart(ctx, ring, ringHead);
  }

  if (msg.type === "anomaly") {
    state.anomalies++;
    prependAnomalyFeed(msg);
    updateGauges(state);
  }

  if (msg.type === "bundle_health") {
    renderFieldBars(msg.fields);
  }

  if (msg.type === "consistency_alert") {
    showConsistencyBanner(msg);
  }
};
```

---

## Part II — TDD: Live Dashboard

All test IDs are prefixed `WS-` for WebSocket / Dashboard.

### WS-1  WebSocket Handshake

**WS-1.1** — Connect to `/v1/ws/dashboard`.
Verify: connection established. First frame received is `{ type: "welcome" }`.
Verify: `bundles` field lists all existing bundles.

**WS-1.2** — Connect to `/v1/ws/{bundle}` where bundle exists.
Verify: `welcome` frame received with `bundles: ["{bundle}"]`.

**WS-1.3** — Connect to `/v1/ws/{bundle}` where bundle does not exist.
Verify: either connection rejected (HTTP 404 before upgrade) or
`welcome` with empty `bundles` list, then auto-subscribed when bundle is created.

**WS-1.4** — Send `{ type: "ping" }`.
Verify: `{ type: "pong", ts: <current_ms> }` received within 100ms.

**WS-1.5** — Connect 5 simultaneous clients to `/v1/ws/dashboard`.
Insert 1 record. Verify: all 5 clients receive the `insert` event.

### WS-2  Insert Events

**WS-2.1** — Connect WebSocket. Insert a record via REST. 
Verify: `insert` event received within 50ms.
Verify: `record_count` in event matches expected count.

**WS-2.2** — Verify `insert` event fields: `type`, `bundle`, `ts`, `record_count`,
`is_anomaly`, `local_curvature`, `confidence`, `global_k_mean`,
`global_k_std`, `global_confidence` — all present, all non-null.

**WS-2.3** — Insert 100 records sequentially.
Verify: `record_count` in events is monotonically increasing.

**WS-2.4** — For a non-anomalous insert, verify `is_anomaly = false`.

### WS-3  Anomaly Events

**WS-3.1** — Insert a record designed to be anomalous (all fields at extremes).
Verify: `anomaly` event received in addition to `insert` event.
Verify: `anomaly.z_score > 2.0`.

**WS-3.2** — Verify `anomaly` event contains: `record`, `local_curvature`,
`z_score`, `confidence`, `deviation_norm`, `contributing_fields`.

**WS-3.3** — Insert 99 normal records then 1 anomaly.
Verify: exactly 1 `anomaly` event received (not 100).

**WS-3.4** — Verify `insert` event for the anomalous record has `is_anomaly = true`.

### WS-4  Curvature Update Events

**WS-4.1** — Insert records until `global_k_mean` shifts by > 5%.
Verify: `curvature_update` event received.
Verify: `old_k_mean`, `new_k_mean`, `delta_pct`, `new_threshold`,
`new_confidence` are all present.

**WS-4.2** — Insert records that do not shift mean by > 5%.
Verify: no `curvature_update` events (suppression working).

**WS-4.3** — Verify `delta_pct = |new - old| / old * 100` to within 0.1%.

### WS-5  Health Events

**WS-5.1** — Connect WebSocket. Wait 5 seconds without inserting anything.
Verify: `bundle_health` event received within 5500ms.

**WS-5.2** — Send `{ type: "subscribe_health", bundle: "sensors" }`.
Verify: `bundle_health` event received immediately (on-demand).

**WS-5.3** — Verify `bundle_health` event contains same fields as
`GET /v1/bundles/{name}/health` response.

### WS-6  Consistency Alert Events

**WS-6.1** — (Future: when data integrity violations are detectable in
the bundle's Čech cohomology.) For now: mock a state where H¹ transitions
from 0 to 1. Verify `consistency_alert` event is emitted with
`cech_h1: 1` and `cech_h1_prev: 0`.

**WS-6.2** — Insert no data that triggers violations.
Verify: no `consistency_alert` events received.

### WS-7  Multi-Bundle Isolation

**WS-7.1** — Create bundles A and B. Connect `/v1/ws/A`.
Insert into B. Verify: client subscribed to A does NOT receive B's events.

**WS-7.2** — Connect `/v1/ws/dashboard` (global). Insert into A and B.
Verify: both events received.

**WS-7.3** — Drop bundle A. Verify: no more A events received by A-subscriber.

### WS-8  Dashboard HTTP Endpoint

**WS-8.1** — GET /dashboard. Verify: HTTP 200, `Content-Type: text/html`.

**WS-8.2** — Verify response body contains the string `GIGI Live Dashboard`.

**WS-8.3** — Verify response body contains a `<canvas>` element (chart).

**WS-8.4** — Verify no external script sources beyond CDN React + ReactDOM
(no custom build step at runtime).

### WS-9  Performance

**WS-9.1** — Connect 50 simultaneous WebSocket clients. Insert 1000 records
at 100/s. Verify: no client drops a message (all receive 1000 `insert` events).

**WS-9.2** — Verify insert latency (wall clock) does not increase by > 2ms
when 50 clients are subscribed vs. 0 clients (fan-out is non-blocking).

**WS-9.3** — Verify WebSocket message serialisation adds < 1ms to insert path.

---

## Part III — Implementation Plan

### Phase 1: Geometric State (no API yet)

Extend `BundleStore` to maintain incremental curvature moments:

```rust
pub struct BundleStats {
    pub k_sum: f64,
    pub k_sum_sq: f64,
    pub k_count: u64,
    // Derived: mu = k_sum/k_count, sigma = sqrt(k_sum_sq/k_count - mu²)
}
```

Update on every `insert_record` and `remove_record`. The curvature K(p)
for an inserted record is already computed by the existing
`local_curvature` plumbing — route that value into `BundleStats`.

**Prerequisite:** Confirm `BundleStore::insert_record` returns or can
return `K(p)` for the inserted record. If not, expose a
`curvature_at(&self, key: &[Value]) -> f64` method.

Deliverables:
- `BundleStats` struct in `bundle.rs`
- `BundleStore::stats() -> &BundleStats`
- `BundleStore::anomaly_threshold(n_sigma: f64) -> f64`
- All Phase 1 TDD tests passing (AD-1.*, AD-2.*, AD-3.*)

### Phase 2: REST Anomaly API

Wire the stats into `/anomalies`, `/health`, `/predict`, `/anomalies/field`.

Deliverables:
- `AnomalyRequest`, `AnomalyResponse`, `HealthResponse`, 
  `PredictRequest`, `PredictResponse` structs in `gigi_stream.rs`  
- Four new route handlers  
- Routes added to the Axum router  
- All AD-4.* through AD-7.* TDD tests passing

### Phase 3: WebSocket Broadcaster

Add `DashboardBroadcaster` to `AppState`:

```rust
pub struct AppState {
    // ... existing fields ...
    pub broadcaster: tokio::sync::broadcast::Sender<DashboardEvent>,
}
```

`DashboardEvent` is a serialisable enum mirroring the JSON event schema.
Every mutating handler (`insert_record`, `delete`, `drop_field`, etc.)
sends to the broadcaster after completing its operation.

Deliverables:
- `DashboardEvent` enum + `serde::Serialize` impl  
- `AppState.broadcaster` field  
- Broadcast call in every mutating handler  
- `/v1/ws/dashboard` and `/v1/ws/{bundle}` WebSocket handlers  
- Periodic `bundle_health` task (tokio::spawn, 5s interval)  
- All WS-1.* through WS-7.* tests passing

### Phase 4: Dashboard UI

Embed the dashboard HTML in `gigi_stream.rs` as a `const DASHBOARD_HTML: &str`:

```rust
const DASHBOARD_HTML: &str = include_str!("../../dashboard/index.html");

async fn serve_dashboard() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/html")], DASHBOARD_HTML)
}
```

The HTML lives at `dashboard/index.html` in the workspace. At build time,
`include_str!` bakes it in. No separate server process. The dashboard is
always at http://localhost:7327/dashboard.

Deliverables:
- `dashboard/index.html` — full self-contained page  
- `GET /dashboard` route in `gigi_stream.rs`  
- All WS-8.* tests passing  
- Manual smoke-test: open browser, insert data, watch curvature chart update

---

## Part IV — Python SDK Extensions (v0.8.0)

The Python SDK gains four new methods mirroring the REST endpoints:

```python
# Anomaly Detection
client.anomalies(bundle, threshold_sigma=2.0, filters=None, fields=None, limit=100)
client.health(bundle)
client.predict(bundle, group_by, field)
client.field_anomalies(bundle, field, threshold_sigma=2.0, limit=50)
```

SDK TDD (prefixed `SDK-`):

**SDK-1.1** — `client.health("sensors")` returns dict with keys:
`records`, `global_curvature`, `global_confidence`, `anomaly_count`,
`anomaly_rate`, `fields`.

**SDK-1.2** — `client.anomalies("sensors")` returns dict with key
`anomalies` as a list, `total_anomalies` as int.

**SDK-1.3** — `client.anomalies("sensors", threshold_sigma=999)` returns
`anomalies = []` (no record can exceed 999σ above mean).

**SDK-1.4** — `client.predict("sensors", group_by="city", field="temperature")`
returns dict with key `predictions` as list, each having `prediction` field.

**SDK-1.5** — `client.field_anomalies("sensors", "temperature")` returns
dict with key `field` = "temperature" and `anomalies` as list.

**SDK-1.6** — All methods raise `GigiError` on HTTP 4xx/5xx responses.

---

## Appendix A: Notation Reference

| Symbol | Meaning | Defined In |
|--------|---------|-----------|
| E, B, F, π | Total space, base space, fiber, projection | Def 1.1 |
| σ(p) | Section at base point p (= stored record) | Def 1.2 |
| σ₀(p) | Zero section (= defaults record) | Def 1.3 |
| δ(p) | Deviation from zero section | Def 1.4 |
| ‖δ(p)‖ | Deviation norm (field count) | Def 1.4 |
| d_F | Fiber metric distance | Def 1.7 |
| K(p) | Local curvature (3-component) | Def 3.3 |
| μ_K, σ_K | Global curvature mean, std dev | § M.7 |
| z(p) | Curvature z-score | § M.5 |
| confidence(p) | 1 / (1 + K(p)) | Corollary 3.3 |
| C = τ/K | Davis capacity equation | Theorem 3.2 |
| Z(β,p) | Query partition function | Def 3.7 |
| Ĥ¹ | Čech 1-cohomology (consistency) | Def 2.4 |
| λ₁ | Spectral gap of field-index Laplacian | Def 3.9–3.10 |

---

## Appendix B: Remark on No-Pipeline Philosophy

The anomaly detection and dashboard described here require **zero ETL, zero
pipeline, zero separate process**. The anomaly score for a record is the
local curvature K(p) — a geometric property of the bundle that is computed
as part of the insert, not as a separate analytics job.

The dashboard is not a separate service. It is served by the same
`gigi_stream.rs` binary. The WebSocket stream is not a message queue layer
— it is a direct broadcast of the same events that are already being
tracked inside `AppState`.

This is the "geometry drops out for free" principle from
`GIGI_AUTOMATIC_ANALYTICS_API.md`:

```
Traditional DB:  INSERT → store → [later] run analytics → wait → results
     GIGI:       INSERT → store + update K → anomaly? → broadcast event → done
```

One pass. One process. No waiting.
