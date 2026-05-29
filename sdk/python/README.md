# gigi-client

> **The Python SDK for [GIGI](https://davisgeometric.com)** — a geometric database engine that models your data as fiber bundles over a base manifold.

```python
from gigi import GigiClient

db = GigiClient("http://localhost:3142", api_key="dev-local")

# Create a bundle (think: a table, but with fiber-bundle structure)
db.create_bundle("sensors", fields={
    "sensor_id": "categorical",     # BASE: identifies a sensor
    "temp":      "numeric",         # FIBER: temperature reading
    "humidity":  "numeric",         # FIBER: humidity reading
}, keys=["sensor_id"])

# Insert
db.insert("sensors", [
    {"sensor_id": "S-001", "temp": 22.5, "humidity": 60.1},
    {"sensor_id": "S-002", "temp": 19.3, "humidity": 71.4},
])

# Query
hot = db.query("sensors", filters=[{"field": "temp", "op": "gt", "value": 20}])
print(hot)

# Compute the scalar curvature K — a single number that describes the
# geometry of your data. Flat (K≈0) = uniform. Positive = bounded.
# Negative = heavy-tailed.
k = db.curvature("sensors")
print(f"K = {k['K']:.4f}    confidence = {k['confidence']:.4f}")
```

That's the whole API surface for the common case. CRUD-style methods, plus geometric reasoning operators that you can't get from any other database.

---

## What this is, in plain words

You probably already have a database. PostgreSQL, MongoDB, DuckDB, whatever. They store your rows and serve your queries.

GIGI is a **different kind** of database. It still stores rows and serves queries — but it also gives you mathematical operators that treat your data as a **geometric object**. The headline operator is **scalar curvature K**, a single number that tells you the shape of your data's distribution. There are more: spectral gap, holonomy, parallel transport. Each one is a question about the *geometry* of your data that you literally couldn't ask of a regular database.

`gigi-client` is the Python SDK for talking to a GIGI engine. It's a thin, focused HTTP + WebSocket client. You install it, you point it at a GIGI URL, and you have programmatic access to the whole engine.

---

## Install

```bash
pip install gigi-client

# With pandas integration (.query_df() and similar):
pip install "gigi-client[pandas]"
```

That gets you the HTTP + WebSocket client. No GIGI engine needed for installation, only at runtime when you want to actually do something.

---

## See it work against the live demo

GIGI runs a **public read-only demo instance** at `https://gigi-stream.fly.dev`, currently hosting **4,961 bundles and 12.8 million records**. You can hit it right now from this SDK without any setup:

```python
from gigi import GigiClient

# Point at the public demo (read-only, no key required for /v1/health)
db = GigiClient("https://gigi-stream.fly.dev")

# Health check — confirms the engine is reachable
print(db.health())
# {'status': 'ok', 'engine': 'gigi-stream', 'version': '0.1.0',
#  'bundles': 4961, 'total_records': 12815841, 'uptime_secs': 43976}
```

If the bundle list endpoint is exposed on the instance you're hitting, you can also browse what's there:

```python
for bundle in db.list_bundles():
    print(f"  {bundle['name']}: {bundle.get('record_count', '?')} records")
```

---

## The data model: bundles, base, fiber

GIGI doesn't store "tables" — it stores **bundles**. A bundle is a fiber bundle over a base manifold, which is a math term but the practical idea is small:

| Term | What it means | Example |
|---|---|---|
| **Base** | The fields that *identify* a record. Usually static. | `sensor_id` |
| **Fiber** | The fields that *evolve* over the base. Usually dynamic. | `temp`, `humidity` at this moment |
| **Bundle** | The whole thing — the base manifold and the fiber attached at every point. | A bundle of sensors with their readings |

Concretely, in a GIGI bundle, when you ask "what's the temperature for sensor S-001?", you're asking for **the fiber value at the base point `sensor_id = "S-001"`**. The geometry of how fibers attach to bases is what gives GIGI its mathematical machinery.

You create bundles like this:

```python
db.create_bundle("orders", fields={
    "order_id":  "categorical",   # BASE
    "customer":  "categorical",   # BASE
    "amount":    "numeric",       # FIBER (evolves with each order)
    "ts":        "timestamp",     # FIBER
}, keys=["order_id"])
```

Behind the scenes, the SDK distinguishes which fields are base and which are fiber based on field types and keys. Categorical fields with `keys` declarations form the base; numeric/timestamp fields form the fiber.

---

## Operations the SDK exposes

### CRUD-style

```python
db.create_bundle(name, fields, keys=[...])           # Define a bundle
db.insert(name, records)                              # Append records
db.query(name, filters=[...], limit=...)              # Filtered SELECT
db.count(name, filters=[...])                         # COUNT(*) WHERE ...
db.query_df(name, filters=[...])                      # → pandas.DataFrame
db.delete_bundle(name)                                # DROP TABLE
```

Filters are JSON-shaped:

```python
db.query("sensors", filters=[
    {"field": "temp", "op": "gt", "value": 30},
    {"field": "sensor_id", "op": "in", "value": ["S-001", "S-007"]},
])
```

### GIGI Query Language (GQL)

For more expressive queries:

```python
results = db.gql("""
    COVER sensors WHERE temp > 30
    SCAN sensors LIMIT 100
""")
```

**`COVER`** filters; **`SCAN`** reads; **`TRANSPORT`** moves records between bundles geometrically; **`CREATE BUNDLE`** and **`INSERT INTO`** do what they look like. Full reference in the [GIGI engine docs](https://github.com/nurdymuny/gigi).

> **Note:** GIGI Query Language is GIGI's own SQL-flavored DSL. It is **not** GraphQL.

### Geometric operators

These are the part you can't get anywhere else. Every operation returns a **plain JSON dict** with the result plus a `confidence` field that scales with sample size.

```python
# Scalar curvature K — what shape is the data's distribution?
k = db.curvature("sensors")
# {'K': 0.0023, 'confidence': 0.9933, 'fiber_var': 12.4, 'base_range': 7.3}

# Spectral gap — how "connected" is the bundle structurally?
g = db.spectral_gap("sensors")

# Holonomy — does parallel transport around closed loops in the base
# produce non-trivial drift in the fiber?
h = db.holonomy("sensors", loop=[...])

# Transport — geodesically move records from one bundle to another
db.transport(src="staging", dst="production", where="quality > 0.9")
```

These four operations (`curvature`, `spectral`, `holonomy`, `transport`) are **patented commercial-tier operations**. Non-commercial use is free; commercial deployments are licensed. The SDK exposes the methods unconditionally — gating happens server-side in the GIGI execution layer, which returns `LICENSE_REQUIRED` for commercial callers without a license.

### Brain primitives

The twelve cognitive operations live behind the bundle interface, as `/v1/bundles/{name}/brain/{primitive}`:

```python
# Generate synthetic records (the DREAM primitive)
synth = db.brain.dream("sensors", n_samples=1000, temperature=2.0)

# Detect change-points (the EPISODIC primitive)
cps = db.brain.episodic("sensors", field="temp")

# Forecast future values
forecast = db.brain.forecast("sensors", field="temp", horizon=24)

# ... and nine more (SAMPLE, RECONSTRUCT, INPAINT, PREDICT, ATTEND,
# FOCUS, SEMANTIC, SELF_MONITOR, EXPLAIN)
```

These are also available as standalone PyPI packages if you only want one — see [Sibling packages](#sibling-packages) below.

### Real-time subscriptions

GIGI exposes a WebSocket subscription API. The SDK wraps it:

```python
import asyncio
from gigi import GigiSubscriber

async def main():
    sub = GigiSubscriber("ws://localhost:3142/ws")
    await sub.connect()

    # Subscribe to all inserts on the 'sensors' bundle
    await sub.subscribe("sensors")

    # Subscribe with a filter — only events matching the predicate
    await sub.subscribe("alerts", where="temp > 30")

    async for event in sub.events():
        print(f"[{event.op}] {event.bundle}: {event.record}")

asyncio.run(main())
```

`event.op` is one of `INSERT`, `UPDATE`, `DELETE`, or `MUTATION`. `event.record` is the affected record. Subscriptions survive engine restarts when the engine is in WAL mode.

---

## The math, explained

You'll see the SDK return things like `K = 0.0023` and `confidence = 0.99` — here's what they mean.

### Scalar curvature K

For each fiber-bundle bundle, the SDK can compute a single number called **scalar curvature K**:

$$K = \frac{\text{Var}(F)}{R^2}$$

where:

- $\text{Var}(F)$ is the variance of the fiber distribution (how spread out the fiber values are at each base point)
- $R$ is the base range (the "size" of the base manifold — number of unique base points, basically)

This is a wildly simplified version of the Ricci scalar curvature from differential geometry, specialized to the fiber-bundle data model. The interpretation:

| $K$ | What it tells you |
|---|---|
| $K \approx 0$ | **Flat geometry.** Arithmetic-dominated. Data is maximally compressible. |
| $K > 0$ | **Positive curvature.** Bounded, categorical-feeling data. Like a sphere. |
| $K < 0$ | **Negative curvature.** Heavy-tailed, hyperbolic-feeling data. Like a saddle. |

You can use $K$ as a **single-number health check** for any bundle. If $K$ suddenly shifts after an insert, something about the geometric structure of your data changed — maybe a new mode appeared, maybe an outlier ballooned the variance. It's like a metric of data shape that lives outside any specific query.

### Confidence

The **confidence** in any geometric estimate is:

$$\text{conf} = 1 - e^{-n/100}$$

where $n$ is the number of records contributing to the estimate. Practical reads:

| $n$ | Confidence |
|---|---|
| 10 | 0.10 |
| 50 | 0.39 |
| 100 | 0.63 |
| 300 | 0.95 |
| 500 | 0.99 |

So if you ask for curvature with 50 records, you get back $K$ with a confidence of 0.39 — meaning "this estimate has a lot of uncertainty, use it as a rough sketch, not a measurement." With 500 records, the confidence is 0.99 and you can take the number seriously.

### The Friston master equation

All twelve brain primitives are unified by the **Friston master equation** on a Kähler bundle:

$$\dot{x} = -\nabla H(x) \, dt + \sqrt{2T} \, dW \quad \text{(dissipative)}$$

$$\dot{x} = B^{-1} \nabla H(x) \quad \text{(conservative)}$$

The first form is "thermal" — the system minimizes a free energy $H$ while being kicked around by noise at temperature $T$. The second form is "geometric" — the system rolls along the gradient of $H$ as transported by the bundle's geometric structure $B$.

Different brain primitives instantiate this equation with different choices of:

- $H$ — the free energy / cost functional
- $T$ — the temperature (the `temperature` knob in `DREAM`, e.g.)
- $B$ — the bundle's metric tensor (the **Kähler form**, in GIGI's case)

This is the unifying mathematical story behind GIGI. If you read the math papers at [davisgeometric.com](https://davisgeometric.com), this is what they're building toward.

---

## Live demo proof point

You can verify all of this against the live demo. Here's a tiny script that hits the health endpoint:

```python
from gigi import GigiClient

db = GigiClient("https://gigi-stream.fly.dev")
print(db.health())
# {'status': 'ok', 'engine': 'gigi-stream', 'version': '0.1.0',
#  'bundles': 4961, 'total_records': 12815841, 'uptime_secs': 43976}
```

**4,961 bundles. 12.8 million records.** Running in production right now on Fly.io, serving requests from this SDK on every continent.

---

## About GIGI — the bigger picture

GIGI is more than the SDK. The full system includes:

- 🧠 **Persistent structured memory** with schema that survives serialization (via the DHOOM format)
- 🌐 **GIGI Query Language** for filtering, aggregating, transporting fiber-bundle data
- 📐 **Four geometric operators**: curvature, spectral gap, holonomy, transport
- 🌀 **Twelve brain primitives** (SAMPLE, FORECAST, DREAM, RECONSTRUCT, INPAINT, PREDICT, ATTEND, FOCUS, EPISODIC, SEMANTIC, SELF-MONITOR, EXPLAIN) all unified by the Friston master equation on a Kähler bundle
- 🔄 **Real-time WebSocket subscriptions** for live data
- 📊 **Live demo** at [gigi-stream.fly.dev](https://gigi-stream.fly.dev/v1/health)

### GIGI is free

Per [Davis Geometric's licensing philosophy](https://davisgeometric.com):

> *"Free for the people who use it to learn; supported by the companies that ship products with it."*

- 🆓 **Free for research, education, and non-commercial use.**
- 💼 **Commercial deployments are patent-protected** (US Provisional Patent 64/045,889) — contact for licensing.
- 🏛️ **Patented commercial-tier operations** (curvature, spectral, holonomy, transport) return `LICENSE_REQUIRED` for non-commercial callers — the **SDK itself is unconditionally MIT licensed**, so you can install and use it freely; only the engine-side operators are gated.

Read about the math: [davisgeometric.com](https://davisgeometric.com)
The engine: [github.com/nurdymuny/gigi](https://github.com/nurdymuny/gigi)
Full API reference: [davisgeometric.com/gigi/docs](https://davisgeometric.com/gigi/docs)

---

## Sibling packages

`gigi-client` is the **full SDK** for the engine. If you only want a single brain primitive without running a GIGI instance, three smaller packages are also published:

- [**`gigi-dream`**](https://pypi.org/project/gigi-dream/) — pure-numpy implementation of the `DREAM` primitive (synthetic data generation)
- [**`gigi-episodes`**](https://pypi.org/project/gigi-episodes/) — pure-numpy implementation of the `EPISODIC` primitive (change-point detection)
- [**`gigi-mcp`**](https://pypi.org/project/gigi-mcp/) — Model Context Protocol server letting Claude (or any MCP client) drive `gigi-client` from natural language

Each one stands alone. `gigi-client` is the SDK that all of them ultimately call when you want the full Kähler-aware engine instead of the standalone numpy fits.

---

## License

MIT for the SDK itself. See [LICENSE](LICENSE).

GIGI itself (the server you connect to) has the dual license described above. The SDK is unconditional — install and use freely.

---

## Status

**v0.8.0** — production-grade for the documented HTTP + WebSocket surface against gigi-stream. The geometric operators and brain primitives are exposed; gating happens server-side per the licensing model.

Issues & feedback: [github.com/nurdymuny/gigi/issues](https://github.com/nurdymuny/gigi/issues)

Built with care by [Bee Rosa Davis](https://davisgeometric.com) / [Davis Geometric](https://davisgeometric.com). 💛
