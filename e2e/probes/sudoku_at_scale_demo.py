"""SUDOKU at realistic scale -- bundles of 100 to 1000+ records.

The earlier 6-domain demos used 7-8 hand-crafted records each (toy
size). This one synthesizes realistic distributions at scale:

  A. NYC apartments     -- 500 listings, 9 fields
  B. Drug discovery     -- 1000 compounds across 3 targets
  C. SP500 screening    -- 500 tickers, sector + numeric financials
  D. Restaurants city   -- 300 venues, multi-categorical filter
  E. Sensor fleet       -- 200 sensors with 8-D embedding (vector
                           field stress test)

Each runs a realistic multi-constraint SUDOKU query and reports:
  * verdict, n_solutions, n_near_misses
  * binding constraint
  * top relaxation menu entry
  * quality score range (does it differentiate at scale?)
  * solve time

Run against a local gigi-stream on port 3143.
"""
import http.client
import json
import random
import time

HOST, PORT = "localhost", 3143
SEED = 42

# ─────────────────────────────────────────────────────────────────
# HTTP plumbing -- keep-alive connection (4000x faster than
# urllib.urlopen on Windows because urllib creates a fresh socket
# per call and Windows TCP teardown is slow).
# ─────────────────────────────────────────────────────────────────

CONN = http.client.HTTPConnection(HOST, PORT, timeout=60)
HEADERS = {"Content-Type": "application/json", "Connection": "keep-alive"}

def post(path, body):
    payload = json.dumps(body)
    try:
        CONN.request("POST", path, payload, HEADERS)
        resp = CONN.getresponse()
        data = resp.read()
        return resp.status, json.loads(data) if data else {}
    except (http.client.BadStatusLine, ConnectionResetError):
        CONN.close()
        CONN.connect()
        CONN.request("POST", path, payload, HEADERS)
        resp = CONN.getresponse()
        data = resp.read()
        return resp.status, json.loads(data) if data else {}


def setup_bundle(name, fields, records):
    schema = {
        "name": name,
        "schema": {"fields": fields, "keys": list(fields.keys())[:1]},
    }
    post("/v1/bundles", schema)
    # Insert in batches of 200 to avoid huge JSON payloads.
    batch = 200
    for i in range(0, len(records), batch):
        post(f"/v1/bundles/{name}/insert", {"records": records[i:i+batch]})


def banner(title, subtitle=""):
    line = "=" * 76
    print(f"\n+{line}+")
    print(f"| {title:<74} |")
    if subtitle:
        print(f"| {subtitle:<74} |")
    print(f"+{line}+")


def run_query(name, n_records, problem, constraints, max_options=5,
              max_near_misses=5):
    print(f"\n  Bundle:  {name}  ({n_records} records)")
    print(f"  Problem: {problem}")
    print(f"  Query:   {len(constraints)} constraints")
    started = time.time()
    status, body = post(
        f"/v1/bundles/{name}/brain/sudoku",
        {"constraints": constraints,
         "max_options": max_options,
         "max_near_misses": max_near_misses}
    )
    elapsed_ms = (time.time() - started) * 1000.0
    if status != 200:
        print(f"  ERROR: HTTP {status}: {body.get('error')}")
        return
    verdict = body.get("verdict", "?")
    sols = body.get("solutions") or []
    nms = body.get("near_misses") or []
    pareto = body.get("pareto_near_misses") or []
    sel = body.get("selectivity") or []
    relax = body.get("relaxations") or []

    badge = {"sat": "[SAT]", "unsat": "[UNSAT]", "unknown": "[?]"}.get(verdict, "[?]")
    print(f"  Result:  {badge} {verdict.upper()} "
          f"({elapsed_ms:.0f} ms total)")
    print(f"           {len(sols)} solutions, {len(nms)} near-misses, "
          f"{len(pareto)} Pareto entries")

    if sols:
        # Quality range -- is it actually differentiating at scale?
        qs = [s.get("quality_score", 0.0) for s in sols]
        print(f"           quality score range: "
              f"{min(qs):.3f} .. {max(qs):.3f}  "
              f"(diff {max(qs)-min(qs):.3f})")
        if qs and max(qs) - min(qs) > 0.01:
            print(f"           TOP solution (quality {qs[0]:.3f}): "
                  f"{compact_record(sols[0]['record'])}")
        else:
            print(f"           [WARN] quality range collapsed; top by mass: "
                  f"{compact_record(sols[0]['record'])}")

    binding = [s for s in sel if s.get("binding")]
    if binding:
        bnames = ", ".join(
            f"{s['field']} (filters {s['marginal_filter_count']})"
            for s in binding)
        print(f"  Binding: {bnames}")

    # W6.1 result gate: per-constraint K_c, sorted descending
    if sel:
        sorted_sel = sorted(sel, key=lambda s: s.get("raw_curvature", 0),
                            reverse=True)
        print("  Constraint K_c (sudoky-style curvature):")
        for s in sorted_sel:
            kc = s.get("raw_curvature", 0.0)
            marg = s.get("marginal_filter_count", 0)
            # Tag the redundancy signature: high K_c + zero marginal.
            tag = ""
            if kc > 0.5 and marg == 0:
                tag = "  [REDUNDANT]"
            elif kc > 0.5 and marg > 0:
                tag = "  [BINDING]"
            print(f"    {s['field']:<25} K_c={kc:.2f}  marginal={marg}{tag}")

    if relax:
        r = relax[0]
        print(f"  Best relax: {r['description']} "
              f"-> +{r['gain']} match(es) [cost {r['relaxation_cost']:.2f}]")

    # Scale-specific: warn if relaxation menu is suspicious.
    if len(relax) > 30:
        print(f"  [WARN] relaxation menu has {len(relax)} entries "
              f"(may explode at scale)")
    if len(pareto) > 50:
        print(f"  [WARN] Pareto frontier has {len(pareto)} entries "
              f"(consider stricter dominance)")


def compact_record(rec, limit=5):
    keep = list(rec.items())[:limit]
    parts = []
    for k, v in keep:
        if isinstance(v, float):
            parts.append(f"{k}={v:.2f}")
        elif isinstance(v, list):
            parts.append(f"{k}=<{len(v)}-vec>")
        elif isinstance(v, str) and len(v) > 20:
            parts.append(f"{k}={v[:18]}..")
        else:
            parts.append(f"{k}={v}")
    return ", ".join(parts)


# ─────────────────────────────────────────────────────────────────
# DOMAIN A -- NYC apartments at city scale (500 listings)
# ─────────────────────────────────────────────────────────────────
banner(
    "DOMAIN A -- NYC apartments at city scale",
    "500 listings synthesized across 6 neighborhoods + 3 bedroom counts"
)
rng = random.Random(SEED)
NEIGHBORHOODS = ["lower_east", "williamsburg", "park_slope",
                 "astoria", "long_island_city", "harlem"]
nbhd_means = {
    "lower_east": 4200, "williamsburg": 3900, "park_slope": 4500,
    "astoria": 3100, "long_island_city": 3600, "harlem": 2700,
}
apartments = []
for i in range(500):
    nb = rng.choice(NEIGHBORHOODS)
    bedrooms = rng.choices([1, 2, 3], weights=[3, 5, 2])[0]
    base = nbhd_means[nb] + (bedrooms - 2) * 700
    rent = int(rng.gauss(base, 350))
    sqft = int(rng.gauss(bedrooms * 380 + 100, 80))
    apartments.append({
        "listing_id": i + 1,
        "neighborhood": nb,
        "bedrooms": bedrooms,
        "rent": rent,
        "sqft": max(sqft, 200),
        "pets_ok": rng.choice(["true", "false", "true"]),
        "doorman": rng.choice(["true", "false"]),
        "year_built": rng.randint(1920, 2020),
        "broker_fee_pct": rng.choice([0.0, 0.0, 8.33, 15.0]),
    })
setup_bundle(
    "nyc_apts_500",
    fields={
        "listing_id": "numeric", "neighborhood": "categorical",
        "bedrooms": "numeric", "rent": "numeric", "sqft": "numeric",
        "pets_ok": "categorical", "doorman": "categorical",
        "year_built": "numeric", "broker_fee_pct": "numeric",
    },
    records=apartments,
)
run_query(
    "nyc_apts_500", 500,
    "2BR in Williamsburg/Park Slope, pet-friendly, rent <= $4000, "
    "no broker fee, >=600 sqft.",
    constraints=[
        {"type": "field", "field": "bedrooms", "op": "eq", "value": 2, "hard": True},
        {"type": "field", "field": "neighborhood", "op": "is_in",
         "value": ["williamsburg", "park_slope"], "hard": True},
        {"type": "field", "field": "pets_ok", "op": "eq", "value": "true", "hard": True},
        {"type": "field", "field": "rent", "op": "le", "value": 4000.0, "hard": True},
        {"type": "field", "field": "broker_fee_pct", "op": "eq", "value": 0.0, "hard": True},
        {"type": "field", "field": "sqft", "op": "ge", "value": 600.0, "hard": True},
    ],
    max_options=5, max_near_misses=5,
)

# ─────────────────────────────────────────────────────────────────
# DOMAIN B -- Drug discovery at chembl-fragment scale (1000 compounds)
# ─────────────────────────────────────────────────────────────────
banner(
    "DOMAIN B -- Drug discovery (1000 compounds, 3 targets)",
    "Realistic pki/ki distributions; FDA status weighted toward 'no'."
)
TARGETS = ["EGFR", "HER2", "VEGFR2"]
CLASSES = ["tki", "antibody", "small_molecule", "macrocycle"]
rng = random.Random(SEED + 1)
compounds = []
for i in range(1000):
    target = rng.choice(TARGETS)
    klass = rng.choices(CLASSES, weights=[5, 2, 6, 1])[0]
    pki = max(rng.gauss(7.0, 1.4), 3.0)        # 3..11 typical range
    ki_nm = 10 ** (9 - pki)                    # ki = 10^(-pki) M -> nM
    compounds.append({
        "ligand_id": i + 1, "target": target, "compound_class": klass,
        "pki": round(pki, 2), "ki_nm": round(ki_nm, 3),
        "mw": round(rng.gauss(420, 100), 1),
        "fda_approved": "true" if rng.random() < 0.06 else "false",
        "phase": rng.choices(["preclin", "1", "2", "3"], weights=[7, 1, 1, 1])[0],
    })
setup_bundle(
    "drug_1000",
    fields={
        "ligand_id": "numeric", "target": "categorical",
        "compound_class": "categorical", "pki": "numeric",
        "ki_nm": "numeric", "mw": "numeric",
        "fda_approved": "categorical", "phase": "categorical",
    },
    records=compounds,
)
run_query(
    "drug_1000", 1000,
    "FDA-approved EGFR TKIs, sub-nM (pki >= 9), MW < 500 Da.",
    constraints=[
        {"type": "field", "field": "target", "op": "eq", "value": "EGFR", "hard": True},
        {"type": "field", "field": "compound_class", "op": "eq", "value": "tki", "hard": True},
        {"type": "field", "field": "pki", "op": "ge", "value": 9.0, "hard": True},
        {"type": "field", "field": "mw", "op": "lt", "value": 500.0, "hard": True},
        {"type": "field", "field": "fda_approved", "op": "eq", "value": "true", "hard": True},
    ],
    max_options=5, max_near_misses=5,
)

# ─────────────────────────────────────────────────────────────────
# DOMAIN C -- SP500-sized stock screening (500 tickers)
# ─────────────────────────────────────────────────────────────────
banner(
    "DOMAIN C -- SP500-sized stock screen (500 tickers)",
    "Realistic sector mix, P/E + dividend + market cap distributions."
)
SECTORS = ["tech", "finance", "health", "consumer", "energy", "industrial", "utilities"]
SECTOR_WEIGHTS = [25, 15, 15, 12, 8, 15, 10]
rng = random.Random(SEED + 2)
stocks = []
for i in range(500):
    sec = rng.choices(SECTORS, weights=SECTOR_WEIGHTS)[0]
    # Sector-conditioned distributions.
    pe = max(rng.gauss(22 if sec == "tech" else 17, 8), 3)
    div_yield = max(rng.gauss(1.5 if sec == "tech" else 3.2, 1.2), 0)
    mcap = max(rng.lognormvariate(4, 1.2), 1.0)   # $B, lognormal-ish
    stocks.append({
        "ticker_id": i + 1, "ticker": f"T{i:03d}",
        "sector": sec, "pe_ratio": round(pe, 1),
        "dividend_yield_pct": round(div_yield, 2),
        "market_cap_b": round(mcap, 1),
        "beta": round(rng.gauss(1.0, 0.4), 2),
        "rev_growth_pct": round(rng.gauss(8, 12), 1),
    })
setup_bundle(
    "stocks_500",
    fields={
        "ticker_id": "numeric", "ticker": "categorical",
        "sector": "categorical", "pe_ratio": "numeric",
        "dividend_yield_pct": "numeric", "market_cap_b": "numeric",
        "beta": "numeric", "rev_growth_pct": "numeric",
    },
    records=stocks,
)
run_query(
    "stocks_500", 500,
    "Tech value screen: P/E <= 20, div >= 2%, mcap >= $50B, "
    "rev growth >= 5%, beta <= 1.3.",
    constraints=[
        {"type": "field", "field": "sector", "op": "eq", "value": "tech", "hard": True},
        {"type": "field", "field": "pe_ratio", "op": "le", "value": 20.0, "hard": True},
        {"type": "field", "field": "dividend_yield_pct", "op": "ge", "value": 2.0, "hard": True},
        {"type": "field", "field": "market_cap_b", "op": "ge", "value": 50.0, "hard": True},
        {"type": "field", "field": "rev_growth_pct", "op": "ge", "value": 5.0, "hard": True},
        {"type": "field", "field": "beta", "op": "le", "value": 1.3, "hard": True},
    ],
    max_options=5, max_near_misses=5,
)

# ─────────────────────────────────────────────────────────────────
# DOMAIN D -- Restaurants citywide (300 venues, many categorical)
# ─────────────────────────────────────────────────────────────────
banner(
    "DOMAIN D -- Restaurants citywide (300 venues)",
    "Heavy categorical filter; tests selectivity at scale."
)
CUISINES = ["thai", "italian", "japanese", "korean", "indian",
            "mexican", "ethiopian", "vietnamese", "french", "american"]
rng = random.Random(SEED + 3)
restaurants = []
for i in range(300):
    cuis = rng.choice(CUISINES)
    nb = rng.choice(NEIGHBORHOODS)
    restaurants.append({
        "venue_id": i + 1,
        "cuisine": cuis, "neighborhood": nb,
        "rating": round(min(5.0, max(2.5, rng.gauss(4.0, 0.5))), 2),
        "avg_price": int(max(10, rng.gauss(35, 18))),
        "open_late": "true" if rng.random() < 0.45 else "false",
        "vegan_options": "true" if rng.random() < 0.55 else "false",
        "noise_level": rng.choice(["quiet", "moderate", "loud"]),
        "reservation_required": "true" if rng.random() < 0.3 else "false",
    })
setup_bundle(
    "restaurants_300",
    fields={
        "venue_id": "numeric", "cuisine": "categorical",
        "neighborhood": "categorical", "rating": "numeric",
        "avg_price": "numeric", "open_late": "categorical",
        "vegan_options": "categorical", "noise_level": "categorical",
        "reservation_required": "categorical",
    },
    records=restaurants,
)
run_query(
    "restaurants_300", 300,
    "Late-night vegan Thai/Japanese/Korean in WBurg/Park Slope, "
    "rated 4+, <=$40, quiet, no resv needed.",
    constraints=[
        {"type": "field", "field": "cuisine", "op": "is_in",
         "value": ["thai", "japanese", "korean"], "hard": True},
        {"type": "field", "field": "neighborhood", "op": "is_in",
         "value": ["williamsburg", "park_slope"], "hard": True},
        {"type": "field", "field": "open_late", "op": "eq", "value": "true", "hard": True},
        {"type": "field", "field": "vegan_options", "op": "eq", "value": "true", "hard": True},
        {"type": "field", "field": "rating", "op": "ge", "value": 4.0, "hard": True},
        {"type": "field", "field": "avg_price", "op": "le", "value": 40.0, "hard": True},
        {"type": "field", "field": "noise_level", "op": "eq", "value": "quiet", "hard": True},
        {"type": "field", "field": "reservation_required", "op": "eq", "value": "false", "hard": True},
    ],
    max_options=5, max_near_misses=5,
)

# ─────────────────────────────────────────────────────────────────
# DOMAIN E -- Sensor fleet (200 sensors, 8-D embedding)
# ─────────────────────────────────────────────────────────────────
banner(
    "DOMAIN E -- Sensor fleet anomaly hunt (200 sensors, 8D embedding)",
    "Vector stress test at scale + numeric filters."
)
rng = random.Random(SEED + 4)
sensors = []
anomaly_anchor = [0.5] * 8
for i in range(200):
    # Bimodal: 90% normal cluster near origin, 10% near anomaly anchor.
    if rng.random() < 0.1:
        center = anomaly_anchor
        status = "anomaly"
    else:
        center = [0.1] * 8
        status = rng.choice(["normal", "normal", "normal", "warning"])
    emb = [round(c + rng.gauss(0, 0.08), 4) for c in center]
    sensors.append({
        "sensor_id": i + 1,
        "temp_c": round(rng.gauss(25, 4), 1),
        "vibration_hz": round(rng.gauss(110, 20), 0),
        "pressure_psi": round(rng.gauss(100, 5), 1),
        "embedding": emb,
        "status": status,
    })
setup_bundle(
    "sensors_200",
    fields={
        "sensor_id": "numeric", "temp_c": "numeric",
        "vibration_hz": "numeric", "pressure_psi": "numeric",
        "embedding": "vector(8)", "status": "categorical",
    },
    records=sensors,
)
# Numeric-only query: find sensors in normal temp range w/ high vibration.
run_query(
    "sensors_200", 200,
    "Sensors with temp 20-30, vibration > 130, pressure 95-105.",
    constraints=[
        {"type": "field", "field": "temp_c", "op": "between", "value": [20.0, 30.0], "hard": True},
        {"type": "field", "field": "vibration_hz", "op": "gt", "value": 130.0, "hard": True},
        {"type": "field", "field": "pressure_psi", "op": "between", "value": [95.0, 105.0], "hard": True},
    ],
    max_options=5, max_near_misses=5,
)
# Vector-similarity probe.
run_query(
    "sensors_200", 200,
    "Find sensors with embedding == anomaly anchor (exact-Eq probe).",
    constraints=[
        {"type": "field", "field": "embedding", "op": "eq",
         "value": anomaly_anchor, "hard": True},
    ],
    max_options=5, max_near_misses=8,
)

print()
print("=" * 78)
print("DONE -- scale stress test complete")
print("=" * 78)
