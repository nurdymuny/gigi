# gigi-client

Python client for **GIGI** — the Geometric Intrinsic Global Index database engine.

## Install

```bash
pip install gigi-client
# With pandas support:
pip install gigi-client[pandas]
```

## Quick start

```python
from gigi import GigiClient

db = GigiClient("http://localhost:3142")

# Create a bundle (table)
db.create_bundle("sensors", fields={
    "sensor_id": "categorical",
    "temp": "numeric",
    "humidity": "numeric",
}, keys=["sensor_id"])

# Insert records
db.insert("sensors", [
    {"sensor_id": "S-001", "temp": 22.5, "humidity": 60.1},
    {"sensor_id": "S-002", "temp": 19.3, "humidity": 71.4},
])

# Query
results = db.query("sensors", filters=[
    {"field": "temp", "op": "gt", "value": 20}
])
print(results)

# Get curvature (geometric health)
k = db.curvature("sensors")
print(f"K={k['K']:.4f}  confidence={k['confidence']:.4f}")

# As a pandas DataFrame
df = db.query_df("sensors")
print(df.head())
```

## Real-time subscriptions

```python
import asyncio
from gigi import GigiSubscriber

async def main():
    sub = GigiSubscriber("ws://localhost:3142/ws")
    await sub.connect()

    # Subscribe to all inserts on the 'sensors' bundle
    await sub.subscribe("sensors")

    # Subscribe with a filter: only temp > 30
    await sub.subscribe("alerts", where="temp > 30")

    async for event in sub.events():
        print(f"[{event.op}] {event.bundle}: {event.record}")

asyncio.run(main())
```

## GIGI Math

GIGI models data as **fiber bundles** over a base manifold.  
Each bundle's geometry is captured by scalar curvature $K$:

$$K = \frac{\text{Var}(F)}{R^2}$$

where $F$ is the fiber distribution and $R$ is the base range.

- $K \approx 0$: flat geometry, arithmetic-dominated (maximum compressibility)
- $K > 0$: positive curvature, bounded/categorical data
- $K < 0$: negative curvature (hyperbolic), heavy-tailed distributions

The **confidence** in the curvature estimate:

$$\text{conf}(K) = 1 - e^{-n/100}$$

where $n$ is the record count.

## API Reference

See [davisgeometric.com/gigi/docs](https://davisgeometric.com/gigi/docs).
