# KRAKEN × GIGI Integration Specification

**Version:** 1.0  
**Author:** Davis Geometric  
**Target:** Wire the KRAKEN blind-test JSX demonstrator to GIGI Stream as the persistence, query, and real-time event layer.

## 0  Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                        KRAKEN Pipeline                           │
│  ┌──────────┐   ┌───────────┐   ┌───────┐   ┌───────────────┐  │
│  │  Scenario │──▶│ CUDA Hot  │──▶│ Fuse  │──▶│ Decision      │  │
│  │  Engine   │   │ Path      │   │ CVaR  │   │ + Merkle Sign │  │
│  │  (replay) │   │ (Select,  │   │       │   │               │  │
│  │           │   │ Translate,│   │       │   │               │  │
│  │           │   │ Geom)     │   │       │   │               │  │
│  └─────┬─────┘   └─────┬─────┘   └───┬───┘   └───────┬───────┘  │
│        │               │             │               │          │
│        ▼               ▼             ▼               ▼          │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                     GIGI Stream (port 3142)                 │ │
│  │                                                             │ │
│  │  Bundles:                                                   │ │
│  │    sensor_das       sensor_sonar      sensor_sat            │ │
│  │    sensor_sigint    embeddings        translations          │ │
│  │    genome_bank      genome_scores     cusum_state           │ │
│  │    fusion_state     trust_state       decisions             │ │
│  │    audit_log        operator_judgments session_state         │ │
│  │                                                             │ │
│  │  WebSockets:                                                │ │
│  │    /v1/ws/dashboard               → global health           │ │
│  │    /v1/ws/sensor_das/dashboard    → Panel 1 feed            │ │
│  │    /v1/ws/fusion_state/dashboard  → Panel 4 feed            │ │
│  │    /v1/ws/trust_state/dashboard   → Panel 6 feed            │ │
│  │    /ws SUBSCRIBE ...              → anomaly/drift alerts    │ │
│  └───────────────────────────┬─────────────────────────────────┘ │
└──────────────────────────────┼───────────────────────────────────┘
                               │
                    WebSocket + REST
                               │
                               ▼
              ┌────────────────────────────────┐
              │     KRAKEN JSX Demonstrator     │
              │                                │
              │  Panel 1: Waterfall            │
              │  Panel 2: Translation Graph    │
              │  Panel 3: Threat Manifold      │
              │  Panel 4: Fusion Cockpit       │
              │  Panel 5: Genome Dashboard     │
              │  Panel 6: Trust Dashboard      │
              │  Footer:  Timeline + Alerts    │
              └────────────────────────────────┘
```

Principle: CUDA produces. GIGI stores, indexes, watches, and pushes. The JSX consumes via WebSocket subscriptions and on-demand REST queries. No custom pub/sub. No custom drift detection. No custom anomaly thresholds. GIGI already does all of that because the data model IS fiber bundles.

## 1  GIGI Bundle Schemas

### 1.1  Sensor Bundles

One bundle per modality. Base = (channel, timestamp). Fiber = raw + processed values. The fiber bundle structure is literal: each base point (a channel at a time) has a fiber (the measurement).

```
BUNDLE sensor_das
  BASE (channel_id NUMERIC, timestamp_us NUMERIC)
  FIBER (
    strain       NUMERIC RANGE 1.0,
    amplitude_db NUMERIC RANGE 60,
    format_id    CATEGORICAL INDEX,
    scenario_id  CATEGORICAL INDEX
  )
  OPTIONS (WAL ENABLED);

BUNDLE sensor_sonar
  BASE (beam_id NUMERIC, timestamp_us NUMERIC)
  FIBER (
    power_db     NUMERIC RANGE 80,
    bearing_deg  NUMERIC RANGE 360,
    frequency_hz NUMERIC RANGE 20000,
    format_id    CATEGORICAL INDEX,
    scenario_id  CATEGORICAL INDEX
  )
  OPTIONS (WAL ENABLED);

BUNDLE sensor_sat
  BASE (patch_id NUMERIC, timestamp_us NUMERIC)
  FIBER (
    sar_db       NUMERIC RANGE 40,
    pixel_data   TEXT,
    scenario_id  CATEGORICAL INDEX
  );

BUNDLE sensor_sigint
  BASE (hit_id NUMERIC AUTO)
  FIBER (
    timestamp_us   NUMERIC,
    freq_ghz       NUMERIC RANGE 100,
    power_dbm      NUMERIC RANGE 120,
    doa_deg        NUMERIC RANGE 360,
    modulation     CATEGORICAL INDEX,
    scenario_id    CATEGORICAL INDEX
  );
```

**Why separate bundles:** Each modality has distinct fiber dimension and sample rate. Curvature K is computed per-bundle, so DAS curvature spikes are not diluted by SONAR's different noise profile. The drift subscription `SUBSCRIBE CURVATURE sensor_das DRIFT > δ` fires only on DAS-specific distribution shifts.

### 1.2  Pipeline State Bundles

```
BUNDLE embeddings
  BASE (modality CATEGORICAL, window_id NUMERIC)
  FIBER (
    vector         TEXT,
    format_used    CATEGORICAL INDEX,
    recon_loss     NUMERIC RANGE 1.0,
    is_probation   CATEGORICAL DEFAULT 'false',
    timestamp_us   NUMERIC,
    scenario_id    CATEGORICAL INDEX
  );

BUNDLE translations
  BASE (edge_id CATEGORICAL, window_id NUMERIC)
  FIBER (
    src_modality   CATEGORICAL INDEX,
    dst_modality   CATEGORICAL INDEX,
    residual       NUMERIC RANGE 1.0,
    epsilon_cal    NUMERIC,
    edge_state     CATEGORICAL INDEX,
    violations     NUMERIC DEFAULT 0,
    timestamp_us   NUMERIC,
    scenario_id    CATEGORICAL INDEX
  );

BUNDLE genome_scores
  BASE (window_id NUMERIC, class_id CATEGORICAL)
  FIBER (
    mahalanobis_d  NUMERIC,
    cusum_s        NUMERIC,
    cusum_alerted  CATEGORICAL DEFAULT 'false',
    p_value        NUMERIC RANGE 1.0,
    timestamp_us   NUMERIC,
    scenario_id    CATEGORICAL INDEX
  );

BUNDLE fusion_state
  BASE (window_id NUMERIC)
  FIBER (
    threat_score   NUMERIC RANGE 1.0,
    class_label    CATEGORICAL INDEX,
    cvar_value     NUMERIC,
    alpha          NUMERIC,
    source_weights TEXT,
    decision_tier  CATEGORICAL INDEX,
    timestamp_us   NUMERIC,
    scenario_id    CATEGORICAL INDEX
  );

BUNDLE trust_state
  BASE (sensor_id CATEGORICAL, window_id NUMERIC)
  FIBER (
    trust_score    NUMERIC RANGE 1.0,
    consistency    NUMERIC RANGE 1.0,
    epsilon_track  NUMERIC RANGE 1.0,
    uniqueness     NUMERIC RANGE 1.0,
    quality        NUMERIC RANGE 1.0,
    state          CATEGORICAL INDEX,
    timestamp_us   NUMERIC,
    scenario_id    CATEGORICAL INDEX
  );
```

### 1.3  Reference Bundles

```
BUNDLE genome_bank
  BASE (class_id CATEGORICAL)
  FIBER (
    class_name     CATEGORICAL INDEX,
    mean_vector    TEXT,
    cov_inv        TEXT,
    lambda_shrink  NUMERIC RANGE 1.0,
    tau_threshold  NUMERIC,
    alpha_class    NUMERIC RANGE 1.0,
    sample_count   NUMERIC,
    color_hex      CATEGORICAL
  );
```

Pre-loaded at startup. Immutable during a session. The genome bank's curvature K should be near-zero (it's reference data with fixed structure). If K drifts, something is wrong.

**Invariant G-REF-1:** `CURVATURE genome_bank` returns K < 0.001 at all times during a session. Violation → integrity failure.

### 1.4  Decision and Audit Bundles

```
BUNDLE decisions
  BASE (decision_id CATEGORICAL)
  FIBER (
    scenario_id    CATEGORICAL INDEX,
    timestamp_us   NUMERIC,
    tier           CATEGORICAL INDEX,
    class_label    CATEGORICAL INDEX,
    threat_score   NUMERIC RANGE 1.0,
    merkle_root    TEXT,
    signature      TEXT,
    sensor_cids    TEXT,
    embed_cids     TEXT,
    translation_cids TEXT,
    geometry_cid   TEXT,
    genome_cids    TEXT,
    fusion_cid     TEXT,
    trust_cids     TEXT
  );

BUNDLE audit_log
  BASE (seq NUMERIC AUTO)
  FIBER (
    timestamp_us   NUMERIC,
    run_id         CATEGORICAL INDEX,
    stage          CATEGORICAL INDEX,
    decision_id    CATEGORICAL INDEX,
    input_cids     TEXT,
    output_cids    TEXT,
    outputs        TEXT,
    model_versions TEXT,
    policy_id      CATEGORICAL,
    latency_us     NUMERIC
  );

BUNDLE operator_judgments
  BASE (judgment_id NUMERIC AUTO)
  FIBER (
    scenario_id    CATEGORICAL INDEX,
    judgment       CATEGORICAL INDEX,
    class_guess    CATEGORICAL,
    timestamp_us   NUMERIC,
    pipeline_hash  TEXT
  );
```

### 1.5  Session Bundle

```
BUNDLE session_state
  BASE (session_id CATEGORICAL)
  FIBER (
    scenario_idx   NUMERIC,
    phase          CATEGORICAL INDEX,
    alpha          NUMERIC RANGE 1.0,
    speed          NUMERIC,
    playing        CATEGORICAL DEFAULT 'true',
    elapsed_secs   NUMERIC,
    created_at     TIMESTAMP,
    updated_at     TIMESTAMP
  );
```


## 2  Data Flow: Pipeline → GIGI → JSX

### 2.1  Write Path (Pipeline → GIGI)

The CUDA pipeline writes to GIGI via NDJSON streaming for high-throughput sensor data and batch REST inserts for pipeline state. Every write returns curvature and confidence from GIGI.

| Source | GIGI Endpoint | Rate | Bundle |
|---|---|---|---|
| DAS 1kHz × 200ch | `POST /v1/bundles/sensor_das/stream` | Batched per window (100ms) | sensor_das |
| SONAR 100Hz × 64 beams | `POST /v1/bundles/sensor_sonar/stream` | Batched per window (100ms) | sensor_sonar |
| SAT 1/360Hz | `POST /v1/bundles/sensor_sat/insert` | Per patch arrival | sensor_sat |
| SIGINT events | `POST /v1/bundles/sensor_sigint/insert` | Per hit | sensor_sigint |
| Embeddings | `POST /v1/bundles/embeddings/insert` | Per window per modality | embeddings |
| Translation results | `POST /v1/bundles/translations/insert` | Per edge per window | translations |
| Genome scores | `POST /v1/bundles/genome_scores/insert` | Per class per window | genome_scores |
| Fusion output | `POST /v1/bundles/fusion_state/insert` | Per window | fusion_state |
| Trust updates | `POST /v1/bundles/trust_state/upsert` | Per sensor per window | trust_state |
| Decisions | `POST /v1/bundles/decisions/insert` | Per alert | decisions |
| Audit deltas | `POST /v1/bundles/audit_log/stream` | Continuous | audit_log |

The pipeline checks the returned `curvature` and `confidence` on every write. This is not overhead — it IS the drift monitor.

### 2.2  Read Path (JSX → GIGI)

The JSX frontend connects via two channels:

**Channel A — WebSocket subscriptions (real-time push):**

```
ws://localhost:3142/v1/ws/sensor_das/dashboard     → waterfall energy
ws://localhost:3142/v1/ws/fusion_state/dashboard    → threat gauge
ws://localhost:3142/v1/ws/trust_state/dashboard     → trust bars
ws://localhost:3142/v1/ws/genome_scores/dashboard   → genome bars + CUSUM
ws://localhost:3142/v1/ws/dashboard                 → global health (header p99)
```

Plus targeted subscriptions via the command WebSocket:

```
ws://localhost:3142/ws

SUBSCRIBE fusion_state WHERE threat_score >= 0.5     → alert events
SUBSCRIBE ANOMALIES sensor_das ON strain              → DAS anomaly spikes
SUBSCRIBE ANOMALIES sensor_sonar ON power_db          → SONAR anomaly spikes
SUBSCRIBE CURVATURE sensor_das DRIFT > 0.02           → DAS drift (format switch signal)
SUBSCRIBE CURVATURE trust_state DRIFT > 0.05          → trust degradation
SUBSCRIBE PHASE genome_scores ON mahalanobis_d        → genome phase transitions
```

**Channel B — REST queries (on-demand, user-triggered):**

| User action | GIGI query | Panel |
|---|---|---|
| Panel load / scenario start | `POST /v1/bundles/genome_bank/query` (all genomes) | 3, 5 |
| Manifold render | `POST /v1/bundles/embeddings/query` (current window ± 128) | 3 |
| Translation graph render | `POST /v1/bundles/translations/query` (latest per edge) | 2 |
| Expand panel (double-click) | `GET /v1/bundles/{bundle}/health` | any |
| Replay drill-down | `POST /v1/bundles/audit_log/query` (by decision_id) | replay |
| Replay causal cone | `POST /v1/bundles/audit_log/join` (pullback along input_cids) | replay |
| Scorecard generation | `POST /v1/bundles/operator_judgments/join` with decisions | scorecard |
| Genome geodesic distance | `POST /v1/bundles/genome_bank/geodesic` (class A → class B) | 3 |
| Vector similarity (embedding) | `POST /v1/bundles/embeddings/vector-search` | 3 |

### 2.3  React State Management

The JSX app maintains a state tree driven entirely by GIGI events. No polling.

```
KrakenState {
  session: {
    id, scenario_idx, phase, alpha, speed, playing, elapsed
  }
  
  panels: {
    waterfall: {
      das_buffer: Float32Array[200 × 256],   ← WS sensor_das/dashboard
      sonar_buffer: Float32Array[64 × 256],  ← WS sensor_sonar/dashboard
      format_id: string,                      ← WS embeddings/dashboard
      probation: bool
    }
    
    translation: {
      edges: [{                               ← REST translations/query
        src, dst, residual, epsilon, state, trust_adjusted_eps
      }]
    }
    
    manifold: {
      genome_ellipsoids: [{                   ← REST genome_bank/query (once)
        class_id, mean, tau, color
      }],
      observations: [{                        ← WS embeddings/dashboard
        window_id, vector, nearest_class, anomaly_score
      }],
      drift_arrow: [dx, dy, dz]              ← SUBSCRIBE CURVATURE DRIFT
    }
    
    fusion: {
      threat_score: f32,                      ← WS fusion_state/dashboard
      class_label: string,
      cvar_value: f32,
      source_weights: {DAS, SONAR, SAT, SIGINT},
      decision_tier: string
    }
    
    genome: {
      distances: [{                           ← WS genome_scores/dashboard
        class_id, d_c, tau_c, is_nearest
      }],
      cusum: [{
        class_id, s_t, h_c, alerted
      }],
      anomaly_score: f32,
      fdr_discoveries: u32
    }
    
    trust: {
      sensors: [{                             ← WS trust_state/dashboard
        sensor_id, score, state, c, e, u, q
      }],
      groups: [{
        group_id, median_trust
      }],
      event_log: [{ timestamp, sensor, transition }]
    }
  }
  
  timeline: {
    alerts: [{ t, class, score }],            ← SUBSCRIBE fusion_state
    das_energy: Float32Array[timeline_bins],
    sonar_energy: Float32Array[timeline_bins],
    cusum_ribbons: { class_id → Float32Array }
  }
  
  judgment: { value, class_guess, submitted_at }
  revealed: bool
  scorecard: null | ScorecardData
}
```

Each WebSocket message triggers a targeted state update. React re-renders only the affected panel. No full-tree diffs.


## 3  WebSocket Wiring Protocol

### 3.1  Connection Lifecycle

```
[App Mount]
  │
  ├─ Connect ws://localhost:3142/ws                    (command channel)
  ├─ Connect ws://localhost:3142/v1/ws/dashboard       (global health)
  │
  │  On scenario start:
  ├─ Connect ws://localhost:3142/v1/ws/sensor_das/dashboard
  ├─ Connect ws://localhost:3142/v1/ws/sensor_sonar/dashboard
  ├─ Connect ws://localhost:3142/v1/ws/fusion_state/dashboard
  ├─ Connect ws://localhost:3142/v1/ws/genome_scores/dashboard
  ├─ Connect ws://localhost:3142/v1/ws/trust_state/dashboard
  │
  │  Via command channel:
  ├─ SUBSCRIBE fusion_state WHERE threat_score >= 0.5
  ├─ SUBSCRIBE ANOMALIES sensor_das ON strain
  ├─ SUBSCRIBE ANOMALIES sensor_sonar ON power_db
  ├─ SUBSCRIBE CURVATURE sensor_das DRIFT > 0.02
  ├─ SUBSCRIBE CURVATURE trust_state DRIFT > 0.05
  ├─ SUBSCRIBE PHASE genome_scores ON mahalanobis_d
  │
  │  On scenario end:
  ├─ UNSUBSCRIBE all scenario-scoped subscriptions
  │
  │  On session end:
  └─ Close all WebSocket connections
```

### 3.2  Message Dispatch

Each dashboard WebSocket pushes JSON objects as data changes. The JSX app maps each message to a panel updater:

```javascript
// React hook: useGigiDashboard(bundle_name) → state
function useGigiDashboard(bundle) {
  const [state, setState] = useState(null);
  useEffect(() => {
    const ws = new WebSocket(
      `ws://localhost:3142/v1/ws/${bundle}/dashboard`
    );
    ws.onmessage = (evt) => {
      const msg = JSON.parse(evt.data);
      setState(msg);
    };
    return () => ws.close();
  }, [bundle]);
  return state;
}

// React hook: useGigiSubscription(channel, filter) → events[]
function useGigiSubscription(filter) {
  const [events, setEvents] = useState([]);
  useEffect(() => {
    const ws = new WebSocket('ws://localhost:3142/ws');
    ws.onopen = () => ws.send(`SUBSCRIBE ${filter}`);
    ws.onmessage = (evt) => {
      const msg = evt.data;
      if (msg.startsWith('RESULT') || msg.startsWith('META')) {
        setEvents(prev => [...prev.slice(-500), parseEvent(msg)]);
      }
    };
    return () => ws.close();
  }, [filter]);
  return events;
}
```

### 3.3  Backpressure

DAS at 1kHz × 200 channels = 200K samples/sec. The pipeline batches into 100ms windows before writing to GIGI. The dashboard WebSocket pushes at the GIGI internal rate (post-compaction), which is naturally throttled. The JSX app further throttles renders to 30fps via `requestAnimationFrame` — it reads the latest state, not every intermediate push.

If the WebSocket buffer exceeds 50 messages, the client drops oldest-first. This is acceptable because GIGI's dashboard feed represents current state, not event history. Missed intermediate states are invisible to the operator.


## 4  Panel-to-Bundle Mapping

| Panel | Primary Bundle(s) | Read Mode | GIGI Feature Used |
|---|---|---|---|
| 1 — Waterfall | sensor_das, sensor_sonar | WS dashboard | Streaming curvature on insert |
| 2 — Translation Graph | translations, trust_state | REST query (latest per edge) | Pullback join (translations ALONG src onto trust) |
| 3 — Threat Manifold | embeddings, genome_bank | REST query + WS dashboard | Vector-search for nearest genome; geodesic between classes |
| 4 — Fusion Cockpit | fusion_state | WS dashboard | Anomaly detection on threat_score |
| 5 — Genome Dashboard | genome_scores, genome_bank | WS dashboard + REST | Curvature on mahalanobis_d; phase detection |
| 6 — Trust Dashboard | trust_state | WS dashboard | Consistency check (H¹); curvature drift subscription |
| Timeline footer | fusion_state, sensor_das, sensor_sonar | WS subscription | SUBSCRIBE WHERE threat_score >= threshold |
| Replay viewer | audit_log, decisions | REST query + pullback join | Causal cone via pullback along input_cids |
| Scorecard | operator_judgments, decisions | REST join | Pullback operator_judgments ALONG scenario_id ONTO decisions |


## 5  Strict TDD: Math-Grounded Test Suite

Every test is derived from a mathematical invariant of the system. Tests are organized by the property they verify, not the component they exercise. Each test states the theorem, the GIGI query that witnesses it, and the assertion.

### 5.0  Notation

```
K(B)          = scalar curvature of bundle B
C(B)          = confidence of bundle B = 1 / (1 + K(B))
H¹(B)         = first Čech cohomology (consistency)
β₀(B)         = zeroth Betti number (connected components)
D_c(y)        = Mahalanobis distance of observation y from genome class c
S_{c,t}       = CUSUM statistic for class c at time t
T_j(t)        = trust score of sensor j at time t
ε_{m→m'}      = calibrated epsilon bound for translation edge m→m'
CVaR_α(L)     = conditional value-at-risk of loss L at confidence α
d_g(p, q)     = geodesic distance between points p, q
g_{ij}        = metric tensor components
```

### 5.1  Curvature-Confidence Duality

**Theorem (GIGI Fundamental).** For any bundle B with K ≥ 0:

$$C(B) = \frac{1}{1 + K(B)}, \qquad K(B) = \frac{1}{C(B)} - 1$$

C is monotone decreasing in K. C = 1 iff K = 0. C → 0 as K → ∞.

```
TEST curvature_confidence_inverse
  FOR EACH bundle IN [sensor_das, sensor_sonar, embeddings,
                       genome_scores, fusion_state, trust_state]:
    k ← GET /v1/bundles/{bundle}/curvature → .K
    c ← GET /v1/bundles/{bundle}/curvature → .confidence
    ASSERT |c - 1/(1 + k)| < 1e-6
    ASSERT 0 ≤ c ≤ 1
    ASSERT k ≥ 0
```

```
TEST curvature_monotonicity_under_injection
  -- Inject increasingly anomalous data, verify K rises and C falls
  baseline_k ← CURVATURE sensor_das → .K
  baseline_c ← CURVATURE sensor_das → .confidence
  
  FOR sigma IN [1, 2, 5, 10]:
    INSERT anomalous record with strain = mean + sigma * std
    k_new ← CURVATURE sensor_das → .K
    c_new ← CURVATURE sensor_das → .confidence
    ASSERT k_new ≥ baseline_k
    ASSERT c_new ≤ baseline_c
    baseline_k ← k_new
    baseline_c ← c_new
```

### 5.2  Gauge Invariance of Schema Evolution

**Theorem (GIGI Gauge).** For any gauge transformation G (add/rename/drop fiber field):

$$K(B) = K(G(B))$$

Curvature is invariant under fiber reparameterization.

```
TEST gauge_invariance_schema_migration
  k_before ← CURVATURE sensor_das → .K
  
  GAUGE sensor_das TRANSFORM (ADD quality NUMERIC RANGE 1.0 DEFAULT 0)
  k_after ← CURVATURE sensor_das → .K
  ASSERT |k_before - k_after| < 1e-6
  
  GAUGE sensor_das TRANSFORM (RENAME quality TO signal_quality)
  k_renamed ← CURVATURE sensor_das → .K
  ASSERT |k_before - k_renamed| < 1e-6
  
  GAUGE sensor_das TRANSFORM (DROP signal_quality)
  k_dropped ← CURVATURE sensor_das → .K
  ASSERT |k_before - k_dropped| < 1e-6
```

### 5.3  ε-Bound Translation Verification

**Theorem (KRAKEN §4).** For every translation edge m → m' with calibrated bound ε_{m→m'}, the residual satisfies:

$$\|T_{m \to m'}(z^{(m)}_t) - z^{(m')}_t\|_2 \leq \varepsilon_{m \to m'}$$

with violation rate < 5% on held-out data.

```
TEST epsilon_bound_violation_rate
  edges ← POST /v1/bundles/translations/query
    { conditions: [{ field: "scenario_id", op: "eq", value: current }] }
  
  FOR EACH edge_id IN DISTINCT(edges → edge_id):
    records ← FILTER edges WHERE edge_id = edge_id AND edge_state ≠ 'Banned'
    violations ← COUNT(records WHERE residual > epsilon_cal)
    rate ← violations / COUNT(records)
    ASSERT rate < 0.05
    -- Gate G-TRN-1 from spec §9.1
```

```
TEST epsilon_curvature_correlation
  -- High translation residual should correlate with high curvature
  -- in the target embedding bundle
  FOR EACH window IN recent(128):
    r ← SECTION translations AT edge_id='DAS→SONAR', window_id=window → residual
    k ← CURVATURE embeddings ON vector
      RESTRICT TO (COVER embeddings WHERE window_id = window)
    -- Spearman rank correlation over the 128-window set
  ASSERT spearman(residuals, curvatures) > 0.3
```

```
TEST banned_edge_curvature_spike
  -- When an edge gets banned, the curvature of translations bundle
  -- should spike (distribution shift from losing an edge)
  INSERT translations record with edge_state='Banned'
  k_after ← CURVATURE translations → .K
  ASSERT k_after > k_before
  -- GIGI's SUBSCRIBE CURVATURE translations DRIFT > 0.02 should fire
```

### 5.4  Mahalanobis Non-Negativity and Genome Separation

**Theorem.** For any observation y and genome class c with positive-definite Σ_c:

$$D_c(y) = (y - g_c)^\top \Sigma_c^{-1} (y - g_c) \geq 0$$

with equality iff y = g_c.

```
TEST mahalanobis_non_negative
  scores ← POST /v1/bundles/genome_scores/query
    { conditions: [{ field: "scenario_id", op: "eq", value: current }] }
  FOR EACH record IN scores:
    ASSERT record.mahalanobis_d ≥ 0
```

**Theorem (Class Separation).** For distinct genome classes c₁ ≠ c₂ with centroids g_{c₁}, g_{c₂}, the geodesic distance in GIGI satisfies:

$$d_g(c_1, c_2) > 0$$

```
TEST genome_class_separation
  classes ← POST /v1/bundles/genome_bank/query { conditions: [] }
  FOR EACH pair (c1, c2) IN classes × classes WHERE c1 ≠ c2:
    d ← POST /v1/bundles/genome_bank/geodesic
      { from: { class_id: c1 }, to: { class_id: c2 } }
    ASSERT d.distance > 0
    ASSERT d.path_found = true
```

**Theorem (AUC Gate).** Mahalanobis D_c correctly separates known classes with AUC ≥ 0.95 on validation data.

```
TEST genome_auc_gate
  -- Gate G-GEN-1 from spec §9.1
  -- For each true-class scenario, check that the correct genome
  -- class has the minimum Mahalanobis distance
  FOR EACH scenario IN threat_scenarios:
    true_class ← scenario.ground_truth.class
    scores ← POST /v1/bundles/genome_scores/query
      { conditions: [
          { field: "scenario_id", op: "eq", value: scenario.id },
          { field: "cusum_alerted", op: "eq", value: "true" }
      ]}
    FOR EACH window_group IN scores GROUPED BY window_id:
      nearest ← argmin(window_group, by: mahalanobis_d)
      hits += (nearest.class_id == true_class) ? 1 : 0
      total += 1
  ASSERT hits / total ≥ 0.95
```

### 5.5  CUSUM Recurrence and Persistence

**Theorem (CUSUM).** The cumulative sum statistic satisfies the recurrence:

$$S_{c,t} = \max\{0,\; S_{c,t-1} + D_c(y_t) - \kappa_c\}$$

S is non-negative, and an alert fires iff S > h_c.

```
TEST cusum_recurrence
  state_history ← POST /v1/bundles/genome_scores/query
    { conditions: [
        { field: "class_id", op: "eq", value: "Kilo" },
        { field: "scenario_id", op: "eq", value: current }
      ],
      sort: [{ field: "window_id", desc: false }]
    }
  
  kappa ← genome_bank.Kilo.kappa  -- pre-loaded from genome_bank
  
  s_prev ← 0
  FOR EACH record IN state_history:
    s_expected ← max(0, s_prev + record.mahalanobis_d - kappa)
    ASSERT |record.cusum_s - s_expected| < 1e-4
    ASSERT record.cusum_s ≥ 0
    s_prev ← record.cusum_s

TEST cusum_spike_rejection
  -- Gate G-GEN-2: Single-frame spike must not cross BH FDR threshold
  -- Inject one anomalous window, verify CUSUM does not alert
  INSERT genome_scores (window_id: W, class_id: 'Kilo',
    mahalanobis_d: 200, cusum_s: 200 - kappa, cusum_alerted: 'false')
  -- Next window is normal
  INSERT genome_scores (window_id: W+1, class_id: 'Kilo',
    mahalanobis_d: 50, cusum_s: max(0, prev_s + 50 - kappa))
  -- CUSUM should decay, not persist
  ASSERT latest_cusum_s < h_kilo
```

### 5.6  CVaR Monotonicity

**Theorem (KRAKEN §8).** For α₁ < α₂ (tighter tail):

$$\text{CVaR}_{\alpha_1}(L) \geq \text{CVaR}_{\alpha_2}(L)$$

A tighter α never reduces the detection count.

```
TEST cvar_monotonicity
  -- Gate G-FUS-1 from spec §9.1
  FOR EACH scenario:
    -- Run pipeline at three alpha levels
    FOR alpha IN [0.01, 0.05, 0.10]:
      UPDATE session_state SET alpha = alpha
      -- Collect detection count
      detections[alpha] ← COUNT(
        POST /v1/bundles/fusion_state/query
          { conditions: [
              { field: "scenario_id", op: "eq", value: scenario.id },
              { field: "threat_score", op: "gte", value: 0.5 }
          ]}
      )
    ASSERT detections[0.01] ≥ detections[0.05]
    ASSERT detections[0.05] ≥ detections[0.10]

TEST cvar_value_ordering
  -- For a single window, CVaR at tighter alpha is always ≥ looser alpha
  fusion ← SECTION fusion_state AT window_id = W
  cvar_001 ← compute_cvar(fusion, alpha=0.01)
  cvar_005 ← compute_cvar(fusion, alpha=0.05)
  cvar_010 ← compute_cvar(fusion, alpha=0.10)
  ASSERT cvar_001 ≥ cvar_005
  ASSERT cvar_005 ≥ cvar_010
```

### 5.7  Trust Score Bounds and EMA Convergence

**Theorem.** Trust scores T_j(t) ∈ [0, 1] for all sensors j, all times t. The EMA update preserves this bound.

```
TEST trust_bounded
  all_trust ← POST /v1/bundles/trust_state/query { conditions: [] }
  FOR EACH record IN all_trust:
    ASSERT 0 ≤ record.trust_score ≤ 1
    ASSERT 0 ≤ record.consistency ≤ 1
    ASSERT 0 ≤ record.epsilon_track ≤ 1
    ASSERT 0 ≤ record.uniqueness ≤ 1
    ASSERT 0 ≤ record.quality ≤ 1
```

**Theorem (Quarantine).** A compromised sensor reaches Quarantined within 15 EMA windows.

```
TEST quarantine_convergence
  -- Gate G-TRU-1 from spec §9.1
  -- Inject adversarial trust degradation
  FOR window IN 1..20:
    UPSERT trust_state (sensor_id='SONAR-2', window_id=window,
      trust_score=0.9 * exp(-0.15 * window), state='Trusted', ...)
  
  final ← SECTION trust_state AT sensor_id='SONAR-2', window_id=20
  ASSERT final.trust_score < 0.40
  ASSERT final.state = 'Quarantined'
  
  -- Verify the curvature of trust_state spiked during degradation
  k ← CURVATURE trust_state → .K
  ASSERT k > baseline_k
```

### 5.8  Consistency (Čech Cohomology)

**Theorem (GIGI).** A bundle with H¹ = 0 has no data conflicts. Overlapping sections agree on shared fibers.

```
TEST bundle_consistency
  FOR EACH bundle IN [sensor_das, sensor_sonar, embeddings,
                       genome_bank, fusion_state, trust_state,
                       decisions, audit_log]:
    h1 ← GET /v1/bundles/{bundle}/consistency → .h1
    ASSERT h1 = 0
    -- Any nonzero H¹ means two writes disagreed on a shared fiber value
    -- This should never happen in KRAKEN's append-only pipeline
```

```
TEST consistency_after_replay
  -- Replay a decision and verify consistency is preserved
  decision ← SECTION decisions AT decision_id = D
  replay_result ← replay_decision(D, pipeline)
  
  -- The audit_log should have zero cocycles after replay
  h1 ← GET /v1/bundles/audit_log/consistency → .h1
  ASSERT h1 = 0
```

### 5.9  Merkle Integrity via GIGI Content Addressing

**Theorem (KRAKEN §10).** For a decision bundle d, the Merkle root commits to all child CIDs:

$$M(d) = \text{MerkleRoot}(\text{SensorCIDs}, \text{EmbedCIDs}, \ldots, \text{TrustCIDs})$$

Replaying the causal cone and recomputing the root must yield the same hash.

```
TEST merkle_root_determinism
  -- Gate G-AUD-1 from spec §9.1
  FOR EACH decision IN POST /v1/bundles/decisions/query
    { conditions: [{ field: "scenario_id", op: "eq", value: current }] }:
    
    -- Reconstruct causal cone via pullback
    cone ← POST /v1/bundles/audit_log/query
      { conditions: [
          { field: "decision_id", op: "eq", value: decision.decision_id }
      ]}
    
    -- Recompute Merkle root from cone's output CIDs
    recomputed_root ← merkle_root(
      cone.map(delta → delta.output_cids).flatten()
    )
    
    ASSERT recomputed_root = decision.merkle_root
```

```
TEST audit_log_append_only
  -- The audit_log seq numbers are strictly monotonic
  log ← POST /v1/bundles/audit_log/query
    { sort: [{ field: "seq", desc: false }], limit: 1000 }
  FOR i IN 1..len(log):
    ASSERT log[i].seq > log[i-1].seq
  
  -- Attempting to INSERT with a lower seq is a semantic error
  -- (GIGI AUTO base field guarantees monotonicity)
```

### 5.10  Metric Tensor Positive-Definiteness

**Theorem.** The GIGI metric tensor g_{ij} of any bundle with ≥ 2 distinct records is positive semi-definite. Eigenvalues λ_i ≥ 0.

```
TEST metric_positive_definite
  FOR EACH bundle IN [sensor_das, embeddings, genome_scores,
                       fusion_state, trust_state]:
    metric ← GET /v1/bundles/{bundle}/metric
    FOR EACH eigenvalue IN metric.eigenvalues:
      ASSERT eigenvalue ≥ -1e-10  -- numerical tolerance
    ASSERT metric.condition_number ≥ 1.0
    ASSERT metric.effective_dimension > 0
```

### 5.11  Spectral Gap and Connectivity

**Theorem.** If β₀(B) = 1 (bundle is connected), then the first non-zero Laplacian eigenvalue λ₁ > 0. The mixing time is O(1/λ₁).

```
TEST spectral_connectivity
  FOR EACH bundle IN [sensor_das, sensor_sonar, genome_bank]:
    betti ← GET /v1/bundles/{bundle}/betti
    spectral ← GET /v1/bundles/{bundle}/spectral
    
    IF betti.beta_0 = 1:
      ASSERT spectral.lambda1 > 0
      ASSERT spectral.spectral_capacity = 1.0 / spectral.lambda1
    ELSE:
      ASSERT spectral.lambda1 = 0  -- disconnected → zero spectral gap
```

### 5.12  Double Cover (S + d² = 1)

**Theorem (Davis).** For any query Q on bundle B, the retrieval balance satisfies:

$$S + d^2 = 1$$

where S is the selectivity (fraction retrieved) and d² is the squared distortion.

```
TEST double_cover_identity
  FOR EACH bundle IN [sensor_das, genome_scores, fusion_state]:
    -- Run a variety of queries with different selectivities
    FOR threshold IN [0.1, 0.3, 0.5, 0.7, 0.9]:
      query ← COVER {bundle} WHERE some_numeric_field > threshold
      total ← COUNT(COVER {bundle} ALL)
      retrieved ← COUNT(query)
      
      S ← retrieved / total
      -- d² computed from curvature of the restricted cover
      k_restricted ← CURVATURE of restricted result
      k_full ← CURVATURE of full bundle
      d_sq ← (k_restricted / k_full) * (1 - S)  -- normalized distortion
      
      ASSERT |S + d_sq - 1| < 0.05  -- tolerance for finite-sample effects
```

### 5.13  Anomaly Detection Threshold Consistency

**Theorem (GIGI).** An anomaly at σ-threshold n has z-score ≥ n, and its local curvature exceeds μ_K + n·σ_K.

```
TEST anomaly_threshold_consistency
  FOR EACH bundle IN [sensor_das, sensor_sonar]:
    health ← GET /v1/bundles/{bundle}/health
    anomalies ← POST /v1/bundles/{bundle}/anomalies
      { threshold_sigma: 2.0, include_scores: true }
    
    FOR EACH a IN anomalies.anomalies:
      ASSERT a.z_score ≥ 2.0
      ASSERT a.local_curvature > health.k_threshold_2s
      ASSERT a.confidence < health.confidence
```

### 5.14  End-to-End Detection Quality

**Theorem (KRAKEN §9.2).** Across the 20-scenario corpus, TPR ≥ 0.90 at FPR ≤ 0.02.

```
TEST end_to_end_detection
  decisions ← POST /v1/bundles/decisions/query { conditions: [] }
  scenarios ← all 20 scenarios with ground truth
  
  tp, fp, tn, fn ← 0
  FOR EACH scenario:
    kraken_said_threat ← EXISTS(decisions WHERE scenario_id = scenario.id
                                AND threat_score ≥ 0.5)
    actual_threat ← scenario.ground_truth.is_threat
    
    IF actual_threat AND kraken_said_threat: tp += 1
    IF actual_threat AND NOT kraken_said_threat: fn += 1
    IF NOT actual_threat AND kraken_said_threat: fp += 1
    IF NOT actual_threat AND NOT kraken_said_threat: tn += 1
  
  tpr ← tp / (tp + fn)
  fpr ← fp / (fp + tn)
  ASSERT tpr ≥ 0.90
  ASSERT fpr ≤ 0.02

TEST degraded_detection
  -- Under 30% sensor outage, TPR ≥ 0.80 at FPR ≤ 0.05
  -- The 5 degraded scenarios (ids: 4, 5, 13, 17, 18)
  -- Same structure as above, filtered to degraded set
  ASSERT tpr_degraded ≥ 0.80
  ASSERT fpr_degraded ≤ 0.05
```

### 5.15  Latency via GIGI Curvature-Aware Monitoring

```
TEST pipeline_latency_from_audit
  -- p99 end-to-end < 100ms
  latencies ← POST /v1/bundles/audit_log/query
    { conditions: [{ field: "stage", op: "eq", value: "fusion" }],
      fields: ["latency_us"] }
  
  p99 ← percentile(latencies.map(r → r.latency_us), 0.99)
  ASSERT p99 < 100_000  -- 100ms in microseconds
  
  -- Verify curvature of latency distribution is stable
  k_latency ← CURVATURE audit_log ON latency_us
  ASSERT k_latency < 0.5  -- latency distribution should be tight
```

### 5.16  Operator Scorecard Correctness

```
TEST scorecard_join_integrity
  -- The scorecard is a pullback join: operator_judgments × decisions × ground_truth
  joined ← POST /v1/bundles/operator_judgments/join
    { right_bundle: "decisions",
      left_field: "scenario_id",
      right_field: "scenario_id" }
  
  ASSERT COUNT(joined) = COUNT(scenarios_completed)
  
  FOR EACH row IN joined:
    ASSERT row.left IS NOT NULL   -- operator always has a judgment
    ASSERT row.right IS NOT NULL  -- KRAKEN always has a decision
    -- Both sides reference the same scenario
    ASSERT row.left.scenario_id = row.right.scenario_id
```

### 5.17  Reference Bundle Immutability

```
TEST genome_bank_immutable
  k_start ← CURVATURE genome_bank → .K
  hash_start ← GET /v1/bundles/genome_bank/consistency → .curvature
  
  -- Run full 20-scenario session
  run_all_scenarios()
  
  k_end ← CURVATURE genome_bank → .K
  hash_end ← GET /v1/bundles/genome_bank/consistency → .curvature
  
  ASSERT |k_start - k_end| < 1e-10
  ASSERT hash_start = hash_end
  -- Genome bank curvature must not change during a session
```

### 5.18  Free Energy Thermodynamic Consistency

**Theorem.** Helmholtz free energy F = −τ ln Z. For fixed data, F is monotone decreasing in τ (higher temperature → lower free energy).

```
TEST free_energy_monotone_in_tau
  FOR EACH bundle IN [sensor_das, genome_scores]:
    f_prev ← +∞
    FOR tau IN [0.01, 0.1, 0.5, 1.0, 5.0, 10.0]:
      f ← GET /v1/bundles/{bundle}/free-energy?tau={tau} → .free_energy
      ASSERT f ≤ f_prev  -- monotone decreasing
      f_prev ← f
```


## 6  Test Execution Matrix

| Test ID | Gate | Invariant | GIGI Endpoint | Frequency |
|---|---|---|---|---|
| 5.1a | — | C = 1/(1+K) | curvature | Per-bundle, per-scenario |
| 5.1b | — | K monotone under injection | curvature, insert | Per-scenario |
| 5.2 | — | Gauge invariance | curvature, GAUGE | Once at startup |
| 5.3a | G-TRN-1 | ε violation rate < 5% | query translations | Per-scenario |
| 5.3b | — | ε ↔ K correlation | curvature, query | Per-scenario |
| 5.3c | — | Banned edge → K spike | curvature, insert | On edge ban event |
| 5.4a | — | D_c(y) ≥ 0 | query genome_scores | Per-window |
| 5.4b | — | d_g(c₁,c₂) > 0 | geodesic | Once at startup |
| 5.4c | G-GEN-1 | AUC ≥ 0.95 | query genome_scores | Post-session |
| 5.5a | — | CUSUM recurrence | query genome_scores | Per-scenario |
| 5.5b | G-GEN-2 | Spike rejection | insert, query | Per-scenario |
| 5.6a | G-FUS-1 | CVaR monotonicity | query fusion_state | Per-scenario × 3 alphas |
| 5.6b | — | CVaR value ordering | query fusion_state | Per-window |
| 5.7a | — | T ∈ [0,1] | query trust_state | Per-scenario |
| 5.7b | G-TRU-1 | Quarantine in ≤ 15 windows | upsert, query trust | Per-adversarial scenario |
| 5.8a | — | H¹ = 0 | consistency | Per-bundle, per-scenario |
| 5.8b | — | Consistency after replay | consistency | Post-replay |
| 5.9a | G-AUD-1 | Merkle root determinism | query audit_log, decisions | Per-decision |
| 5.9b | — | Append-only monotonicity | query audit_log | Per-scenario |
| 5.10 | — | g_{ij} positive semi-definite | metric | Per-bundle |
| 5.11 | — | λ₁ > 0 iff β₀ = 1 | spectral, betti | Per-bundle |
| 5.12 | — | S + d² = 1 | curvature, query | Per-bundle × 5 thresholds |
| 5.13 | — | z-score ≥ nσ | anomalies, health | Per-bundle |
| 5.14a | §9.2 | TPR ≥ 0.90, FPR ≤ 0.02 | query decisions | Post-session |
| 5.14b | §9.2 | Degraded TPR ≥ 0.80 | query decisions | Post-session |
| 5.15 | §9.2 | p99 < 100ms | query audit_log | Post-session |
| 5.16 | — | Scorecard join integrity | join | Post-session |
| 5.17 | G-REF-1 | Genome bank immutable | curvature, consistency | Pre/post session |
| 5.18 | — | F monotone in τ | free-energy | Per-bundle |

Total: **28 tests** covering **14 mathematical theorems** and **10 spec gate conditions**.


## 7  Implementation Order

### Phase A — Schema Bootstrap (Day 1)

Create all 14 GIGI bundles. Load genome_bank reference data. Verify G-REF-1, tests 5.2, 5.4b, 5.10, 5.11.

### Phase B — Write Path (Days 2-3)

Wire pipeline stages to GIGI insert/stream/upsert endpoints. Run tests 5.1a, 5.1b, 5.4a, 5.7a, 5.8a after each stage is wired.

### Phase C — WebSocket Layer (Days 4-5)

Connect JSX dashboard hooks to GIGI WebSocket endpoints. Wire subscription filters. Verify real-time push reaches React state within 100ms of GIGI write.

### Phase D — REST Query Layer (Days 6-7)

Wire on-demand queries for manifold, translation graph, replay viewer, scorecard. Run tests 5.3a, 5.5a, 5.6a, 5.9a, 5.16.

### Phase E — Subscription Events (Day 8)

Wire anomaly, drift, and phase subscriptions. Verify tests 5.3c, 5.7b, 5.13.

### Phase F — Full Integration (Days 9-10)

Run all 20 scenarios end-to-end. Execute the complete test matrix. Verify gates 5.14a, 5.14b, 5.15, 5.17.
