# GIGI Automatic Geometric Analytics API

**The geometry drops out for free. The user never asks for it.**

## Core Principle

When you insert data into GIGI, you're defining a section on a fiber bundle.
The curvature, spectral gap, confidence, and anomaly scores are PROPERTIES
of the bundle that update incrementally with every insert. The user doesn't
run analytics — the analytics are the database response.

```
Traditional DB:  INSERT → store → [later] run analytics query → wait → results
GIGI:            INSERT → store + update K → done (analytics already computed)

Traditional DB:  SELECT → scan → return rows
GIGI:            SELECT → evaluate σ(x) → return rows + confidence + curvature
```

## What Every Response Includes (Free)

Every single GIGI query response is annotated with geometric metadata:

```json
{
  "records": [...],
  "meta": {
    "confidence": 0.9772,
    "curvature": 0.0233,
    "capacity": 4.29,
    "records_returned": 366,
    "query_time_ns": 1033
  }
}
```

The user didn't ask for confidence. They didn't run an analytics job.
They did a SELECT and the bundle told them how much to trust it.

---

## API Level 1: Automatic (Zero Configuration)

These require NO setup, NO configuration, NO separate analytics step.
They exist because the data is stored on a fiber bundle.

### Every INSERT enriches the geometry

```javascript
// User does this:
await db.bundle('sensors').insert({
  sensor_id: 'T-001',
  timestamp: 1710000000,
  temperature: 22.4,
  humidity: 48.3,
  status: 'normal',
});

// Behind the scenes, GIGI automatically:
// 1. Computes GIGI hash → base point p
// 2. Stores section σ(p)
// 3. Updates field_index topology
// 4. Incrementally updates curvature K for affected neighborhoods
// 5. Incrementally updates spectral connectivity if topology changed
// 6. Checks if new record is an anomaly (high local K)
//
// Cost: O(1) for all of this. The geometry is LOCAL.
```

### Every QUERY returns confidence

```javascript
// User does this:
const reading = await db.bundle('sensors').get({
  sensor_id: 'T-001',
  timestamp: 1710000000,
});

// They get back:
{
  data: { sensor_id: 'T-001', temperature: 22.4, ... },
  confidence: 0.9988,        // ← FREE. Came from curvature.
  curvature: 0.0012,         // ← FREE. Already computed at insert time.
  is_anomaly: false,         // ← FREE. K < threshold.
  query_time_ns: 1033,
}
```

### Every RANGE QUERY returns aggregate health

```javascript
// User does this:
const alerts = await db.bundle('sensors').where('status', 'alert');

// They get back:
{
  records: [...],
  meta: {
    confidence: 0.9215,         // ← aggregate confidence for this slice
    curvature: 0.0854,          // ← aggregate K for this slice
    anomaly_count: 3,           // ← records where local K > threshold
    homogeneity: 0.92,         // ← how uniform is this slice (1 = identical)
    records_returned: 47,
  }
}
```

### Every GROUP BY returns per-group curvature

```javascript
// User does this:
const by_city = await db.bundle('sensors').aggregate({
  groupBy: 'city',
  compute: { avg_temp: { fn: 'avg', field: 'temperature' } },
});

// They get back:
{
  groups: [
    {
      city: 'Moscow',
      avg_temp: -5.2,
      meta: {
        curvature: 0.0233,     // ← Moscow is variable
        confidence: 0.9772,
        anomaly_rate: 0.041,   // ← 4.1% of Moscow records are anomalous
        records: 366,
      }
    },
    {
      city: 'Singapore',
      avg_temp: 27.8,
      meta: {
        curvature: 0.0001,     // ← Singapore is ultra-stable
        confidence: 0.9999,
        anomaly_rate: 0.000,   // ← zero anomalies
        records: 366,
      }
    },
    // ...
  ]
}
```

The user asked for GROUP BY avg temperature.
They got anomaly rates per city for free.

---

## API Level 2: One-Call Analytics (No Pipeline)

These are explicit calls, but they're ONE call that returns immediately
because the data is already on a fiber bundle. No ETL. No pipeline.
No Spark job. One function call.

### Health Check (full bundle diagnostics)

```javascript
const health = await db.bundle('sensors').health();

// Returns:
{
  bundle: 'sensors',
  records: 7320,
  
  // Curvature (anomaly detection)
  global_curvature: 0.0346,
  global_confidence: 0.9665,
  
  // Spectral (connectivity)
  spectral_gap: 0.0,            // 0 = disconnected clusters
  connected_components: 20,      // 20 isolated city clusters
  
  // Consistency (data quality)  
  cech_h1: 0,                   // 0 = fully consistent
  inconsistencies: [],           // no cocycle violations
  
  // Capacity
  capacity: 2.89,               // C = τ/K
  
  // Per-field analysis
  fields: {
    temperature: { curvature: 0.0087, anomaly_threshold: 2.5 },
    humidity:    { curvature: 0.0034, anomaly_threshold: 1.8 },
    wind:        { curvature: 0.0019, anomaly_threshold: 3.2 },
    pressure:    { curvature: 0.0002, anomaly_threshold: 0.9 },
  },
  
  // Top anomalies (precomputed)
  top_anomalies: [
    { record_id: 4521, city: 'Moscow', date: '2024-01-04', z_score: 5.30 },
    { record_id: 4520, city: 'Moscow', date: '2024-01-03', z_score: 5.26 },
    // ...
  ],
  
  computed_in_ms: 1.2,
}
```

One call. Full diagnostic. No pipeline.

### Anomalies (curvature spike detection)

```javascript
const anomalies = await db.bundle('sensors').anomalies();

// Returns every record where local K exceeds the adaptive threshold:
{
  anomalies: [
    {
      data: { city: 'Moscow', date: '2024-01-04', temperature: -31.9, ... },
      local_curvature: 0.089,
      z_score: 5.30,
      contributing_fields: ['temperature'],    // which field(s) drove K up
      neighborhood_size: 366,
    },
    // ...
  ],
  threshold: 'adaptive',      // based on global K distribution
  total_anomalies: 47,
  anomaly_rate: 0.0064,       // 0.64% of records
}
```

No configuration. No threshold tuning. The curvature distribution
determines the threshold automatically. High K = anomaly. Period.

### Anomalies by Field

```javascript
const temp_anomalies = await db.bundle('sensors').anomalies({
  field: 'temperature',
});

// Returns anomalies driven specifically by temperature curvature:
{
  field: 'temperature',
  anomalies: [
    { city: 'Moscow', date: '2024-01-04', value: -31.9, z_score: 5.30 },
    { city: 'Moscow', date: '2024-01-03', value: -31.6, z_score: 5.26 },
    // ...
  ],
  field_curvature: 0.0087,
  field_confidence: 0.9913,
}
```

### Predict (curvature-based forecasting)

```javascript
const forecast = await db.bundle('sensors').predict({
  field: 'temperature',
  groupBy: 'city',
});

// Returns per-group volatility prediction:
{
  predictions: [
    {
      city: 'Moscow',
      curvature: 0.0233,
      prediction: 'HIGH_VOLATILITY',
      confidence: 0.9772,
      expected_anomaly_rate: 0.041,
    },
    {
      city: 'Singapore',
      curvature: 0.0001,
      prediction: 'STABLE',
      confidence: 0.9999,
      expected_anomaly_rate: 0.000,
    },
    // ...
  ],
  method: 'curvature_ranking',
  accuracy_estimate: '55% on historical backtest',
}
```

### Connectivity (spectral analysis)

```javascript
const connectivity = await db.bundle('sensors').connectivity();

// Returns spectral analysis of the field index graph:
{
  spectral_gap: 0.0,
  connected_components: 20,
  components: [
    { field_value: 'Moscow', size: 366, internal_lambda1: 1.003 },
    { field_value: 'Singapore', size: 366, internal_lambda1: 1.003 },
    // ...
  ],
  bottlenecks: [],             // field values that bridge components
  mixing_time: Infinity,       // inf because disconnected
  computed_in_ms: 1.03,
}
```

### Consistency (Čech cohomology)

```javascript
const consistency = await db.bundle('sensors').consistency();

// Returns:
{
  h1: 0,                        // 0 = fully consistent
  inconsistencies: 0,
  cocycles: [],                 // no violations
  status: 'CONSISTENT',
  
  // If h1 != 0:
  // cocycles: [
  //   {
  //     neighborhoods: ['city=Moscow', 'region=EU'],
  //     overlap_records: 366,
  //     disagreements: 2,
  //     fields: ['temperature'],
  //     severity: 0.003,
  //   }
  // ]
}
```

---

## API Level 3: Real-Time Subscriptions (Geometry as Events)

The geometry updates live. Subscribe to geometric events.

### Subscribe to Anomalies

```javascript
// Fire an event whenever a new record has high curvature
db.bundle('sensors').onAnomaly((event) => {
  console.log(`ANOMALY: ${event.data.city} at ${event.data.date}`);
  console.log(`  temperature: ${event.data.temperature}°C`);
  console.log(`  z-score: ${event.z_score}`);
  console.log(`  local K: ${event.curvature}`);
  console.log(`  confidence: ${event.confidence}`);
});

// How it works:
// 1. New record inserted
// 2. GIGI computes local curvature K at the new base point
// 3. If K > adaptive threshold → fire anomaly event
// 4. Cost: O(1) per insert (curvature is local)
```

### Subscribe to Curvature Drift

```javascript
// Fire when a region's curvature changes significantly
db.bundle('sensors').onCurvatureDrift({
  groupBy: 'city',
  threshold: 0.01,       // fire if K changes by more than 0.01
}, (event) => {
  console.log(`DRIFT: ${event.city} K changed ${event.k_before} → ${event.k_after}`);
  console.log(`  direction: ${event.direction}`);  // 'increasing' or 'decreasing'
  console.log(`  interpretation: ${event.interpretation}`);
  // "Moscow temperature variability increasing — possible weather pattern shift"
});
```

### Subscribe to Consistency Violations

```javascript
// Fire when Čech H¹ goes nonzero (data becomes inconsistent)
db.bundle('sensors').onInconsistency((event) => {
  console.log(`INCONSISTENCY DETECTED: H¹ = ${event.h1}`);
  console.log(`  location: ${event.cocycle.neighborhoods}`);
  console.log(`  disagreements: ${event.cocycle.disagreements}`);
  console.log(`  affected fields: ${event.cocycle.fields}`);
});
```

### Subscribe to Spectral Changes

```javascript
// Fire when the spectral gap changes (topology shift)
db.bundle('sensors').onSpectralShift((event) => {
  console.log(`TOPOLOGY CHANGE: λ₁ = ${event.lambda1_before} → ${event.lambda1_after}`);
  console.log(`  components: ${event.components_before} → ${event.components_after}`);
  // "New city added — spectral gap decreased, graph is now more fragmented"
});
```

---

## API Level 4: DHOOM Wire Integration

All of the above is transmitted over DHOOM, so the geometric metadata
is compressed alongside the data.

### Query Response on the Wire

```
RESULT sensors{sensor_id, timestamp, temperature, humidity, status|normal}:
T-001, 1710000000, 22.4, 48.3
T-002, 1710000060, 23.1, 50.1
T-003, 1710000120, 45.8, 42.7, :alert
META confidence=0.9665 curvature=0.0346 anomalies=1 capacity=2.89
ANOMALY T-003 z=3.21 k=0.089 fields=temperature
```

The META and ANOMALY lines are part of the DHOOM response.
Zero extra bytes for the geometry — it rides the same wire.

### Health Response on the Wire

```
HEALTH sensors
META records=7320 global_k=0.0346 confidence=0.9665
META spectral_gap=0.0 components=20
META cech_h1=0 consistent=true
FIELD temperature k=0.0087 threshold=2.5
FIELD humidity k=0.0034 threshold=1.8
FIELD wind k=0.0019 threshold=3.2
TOP_ANOMALY city=Moscow date=20240104 z=5.30
TOP_ANOMALY city=Moscow date=20240103 z=5.26
```

---

## Implementation: How Curvature Updates are O(1)

The key insight: curvature is LOCAL. When you insert a record at base
point p, only the neighborhood N(p) is affected. The neighborhood is
determined by the field index — records sharing an indexed field value.

```rust
fn insert_with_geometry(&mut self, record: &Record) {
    // 1. Standard insert: O(1)
    let bp = self.gigi_hash(&record);
    self.sections.insert(bp, record.clone());
    
    // 2. Update field index: O(m) for m indexed fields
    for field in &self.schema.indexed_fields {
        let val = record.get(field);
        self.field_index[field][val].insert(bp);
    }
    
    // 3. Incremental curvature update: O(1) amortized
    // Only need to update running statistics for affected neighborhoods
    for field in &self.schema.indexed_fields {
        let val = record.get(field);
        let stats = &mut self.curvature_stats[field][val];
        
        // Welford's online algorithm for incremental variance
        stats.n += 1;
        for fiber_field in &self.schema.fiber_fields {
            if let Some(v) = record.get_f64(fiber_field) {
                let delta = v - stats.mean[fiber_field];
                stats.mean[fiber_field] += delta / stats.n as f64;
                let delta2 = v - stats.mean[fiber_field];
                stats.m2[fiber_field] += delta * delta2;
                // K = variance / (range² × n) — updated in O(1)
            }
        }
        
        // Check anomaly threshold
        let k = stats.curvature();
        if k > stats.adaptive_threshold() {
            self.emit_anomaly_event(bp, k);
        }
    }
    
    // 4. Spectral update: O(1) amortized
    // Union-find: just union the new point with its field value group
    for field in &self.schema.indexed_fields {
        let val = record.get(field);
        let group_rep = self.spectral_uf.find(val);
        self.spectral_uf.union(bp, group_rep);
    }
}
```

Total insert cost: O(1) amortized for data + geometry + spectral.
The user pays NOTHING extra for the analytics.

---

## What This Means for the User

### Before GIGI (traditional stack):
```
1. Store data in PostgreSQL
2. Set up Prometheus for monitoring
3. Build anomaly detection pipeline (Python + Spark + ML)
4. Set up Grafana dashboards
5. Write alert rules manually
6. Build data quality checks
7. Hire a data engineer to maintain all of this
```

### With GIGI:
```
1. Store data in GIGI
2. Done.
   - Anomaly detection: built-in (curvature)
   - Monitoring: built-in (confidence per query)
   - Data quality: built-in (Čech H¹)
   - Alerting: built-in (subscribe to curvature drift)
   - Prediction: built-in (curvature forecasting)
   - Connectivity analysis: built-in (spectral gap)
```

Seven tools replaced by one database.
The geometry gives you everything for free.

---

## GQL (GIGI Query Language) Commands

```sql
-- Standard queries (return confidence automatically)
SELECT * FROM sensors WHERE city = 'Moscow';
-- → rows + META confidence=0.9772 curvature=0.0233

-- Explicit geometric queries
SELECT CURVATURE(temperature) FROM sensors GROUP BY city;
SELECT CONFIDENCE(*) FROM sensors WHERE region = 'EU';
SELECT ANOMALIES(temperature, threshold=2.5) FROM sensors;
SELECT SPECTRAL_GAP FROM sensors;
SELECT CONSISTENCY FROM sensors;

-- Prediction
SELECT PREDICT(temperature, volatility) FROM sensors GROUP BY city;

-- Health
HEALTH sensors;

-- Subscribe
SUBSCRIBE sensors WHERE CURVATURE(temperature) > 0.05;
SUBSCRIBE sensors WHERE ANOMALY(z_score > 3.0);
SUBSCRIBE sensors WHERE CONSISTENCY != 0;
```

---

**GIGI** · Geometric Intrinsic Global Index · Davis Geometric · 2026
