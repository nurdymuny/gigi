"""Post-Kähler Phase 1 — live examples on Fisher's iris data.

Ingests the real 150-row iris dataset into a local gigi-stream (branch build,
post_kahler_phase1) and exercises all four PK GQL verbs + the REST twins.
Captures every request/response to JSON for the artifact. Fittingly, the
dataset is R.A. Fisher's (1936) — and PK-2 is the Fisher information metric.
"""
import json, urllib.request, time, sys
from sklearn.datasets import load_iris

BASE = "http://127.0.0.1:3142"
OUT = sys.argv[1] if len(sys.argv) > 1 else "iris_pk_results.json"


def req(method, path, body=None):
    data = json.dumps(body).encode() if body is not None else None
    r = urllib.request.Request(BASE + path, data=data, method=method,
                               headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(r, timeout=60) as resp:
            return resp.status, json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode()[:400]


def gql(q):
    return req("POST", "/v1/gql", {"query": q})


def wait_ready():
    for _ in range(60):
        try:
            s, _ = req("GET", "/v1/metrics")
            if s == 200:
                return True
        except Exception:
            pass
        time.sleep(1)
    return False


iris = load_iris()
names = ["sepal_length", "sepal_width", "petal_length", "petal_width"]
species = iris.target  # 0 setosa, 1 versicolor, 2 virginica

print("waiting for server...")
if not wait_ready():
    print("server did not become ready"); sys.exit(1)

captured = {"dataset": "sklearn iris (Fisher 1936), 150 rows, 3 species",
            "base": BASE, "steps": []}


def record(label, kind, query_or_path, status, body):
    captured["steps"].append({"label": label, "kind": kind,
                              "call": query_or_path, "status": status, "response": body})
    print(f"[{status}] {label}: {query_or_path}")


# ── ingest ──
gql("DROP BUNDLE iris;")
s, b = gql("CREATE BUNDLE iris (id INT BASE, sepal_length FLOAT FIBER, "
           "sepal_width FLOAT FIBER, petal_length FLOAT FIBER, "
           "petal_width FLOAT FIBER, species FLOAT FIBER);")
record("create bundle", "gql", "CREATE BUNDLE iris (...)", s, b)

records = []
for i, (row, sp) in enumerate(zip(iris.data, species)):
    rec = {"id": i, "species": float(sp)}
    for k, v in zip(names, row):
        rec[k] = float(v)
    records.append(rec)
s, b = req("POST", "/v1/bundles/iris/insert", {"records": records})
record(f"ingest {len(records)} iris measurements", "rest",
       "POST /v1/bundles/iris/insert", s, {"inserted": b if isinstance(b, dict) else b})

# ── PK-2 FISHER (GQL) — the Fisher metric of each measurement ──
s, b = gql("FISHER iris ON (sepal_length, sepal_width, petal_length, petal_width);")
record("PK-2 Fisher metric per measurement", "gql",
       "FISHER iris ON (sepal_length, sepal_width, petal_length, petal_width);", s, b)

# ── PK-4 PERSISTENCE (GQL) — H0 clusters in petal space ──
s, b = gql("PERSISTENCE iris ON (petal_length, petal_width) GAP 2;")
record("PK-4 persistence — clusters in petal space", "gql",
       "PERSISTENCE iris ON (petal_length, petal_width) GAP 2;", s, b)

# sepal space is famously more tangled — show the honest contrast
s, b = gql("PERSISTENCE iris ON (sepal_length, sepal_width) GAP 2;")
record("PK-4 persistence — sepal space (tangled, honest contrast)", "gql",
       "PERSISTENCE iris ON (sepal_length, sepal_width) GAP 2;", s, b)

# ── PK-3 WASSERSTEIN (GQL) — setosa vs virginica petal length ──
s, b = gql("WASSERSTEIN iris ON petal_length BETWEEN species = 0 AND 2;")
record("PK-3 Wasserstein — setosa vs virginica petal length", "gql",
       "WASSERSTEIN iris ON petal_length BETWEEN species = 0 AND 2;", s, b)
s, b = gql("WASSERSTEIN iris ON petal_length BETWEEN species = 1 AND 2;")
record("PK-3 Wasserstein — versicolor vs virginica (closer)", "gql",
       "WASSERSTEIN iris ON petal_length BETWEEN species = 1 AND 2;", s, b)

# ── PK-1 REEB (GQL) — contact structure on three measurements ──
s, b = gql("REEB iris ON (petal_length, petal_width, sepal_length);")
record("PK-1 Reeb — contact structure on 3 measurements", "gql",
       "REEB iris ON (petal_length, petal_width, sepal_length);", s, b)

# ── REST twins ──
s, b = req("GET", "/v1/bundles/iris/fisher_metric?fields=petal_length,petal_width")
record("PK-2 REST twin", "rest",
       "GET /v1/bundles/iris/fisher_metric?fields=petal_length,petal_width", s, b)
s, b = req("GET", "/v1/bundles/iris/topology/persistence?fields=petal_length,petal_width")
record("PK-4 REST twin", "rest",
       "GET /v1/bundles/iris/topology/persistence?fields=petal_length,petal_width", s, b)

with open(OUT, "w") as f:
    json.dump(captured, f, indent=2)
print(f"\nwrote {OUT} with {len(captured['steps'])} steps")
