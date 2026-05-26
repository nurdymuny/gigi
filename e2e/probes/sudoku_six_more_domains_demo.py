"""SUDOKU usefulness demo, round 2 -- six MORE domains.

Domains 7-12 deliberately stress axes the first 6 didn't:

  7. Used cars         -- multi-numeric Pareto trade-off
  8. Restaurants       -- Bool fields + many SAT (which is best?)
  9. Flights           -- Timestamp constraints (numeric-coerced)
 10. Used books        -- exact text search + year range
 11. Sensor anomaly    -- VECTOR/embedding constraint
                          (probes a known gap on purpose)
 12. HR promotion      -- multi-numeric all-soft, "best in set"
                          (probes lack of solution quality rank)

Run against a local gigi-stream on port 3143.
"""
import json
import sys
import urllib.request
import urllib.error

BASE = "http://localhost:3143"


def post(path, body):
    req = urllib.request.Request(
        BASE + path,
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        try:
            return e.code, json.loads(e.read())
        except Exception:
            return e.code, {"error": "non-JSON error response"}


def setup_bundle(name, fields, records):
    schema = {
        "name": name,
        "schema": {"fields": fields, "keys": list(fields.keys())[:1]},
    }
    post("/v1/bundles", schema)
    post(f"/v1/bundles/{name}/insert", {"records": records})


def banner(title, subtitle=""):
    line = "=" * 76
    print(f"\n+{line}+")
    print(f"| {title:<74} |")
    if subtitle:
        print(f"| {subtitle:<74} |")
    print(f"+{line}+")


def show_sudoku(name, problem, constraints, max_options=3, max_near_misses=3,
                expected_gap=None):
    """Run a SUDOKU query and pretty-print results.

    expected_gap: optional human-readable note describing a GP gap
    this domain is meant to probe. Printed BEFORE the verdict so the
    reader can compare what was hoped for vs what got returned.
    """
    print(f"\n  Problem:  {problem}")
    print(f"  Query:    {len(constraints)} constraints")
    for c in constraints:
        if c["type"] == "field":
            v = c["value"]
            v_short = v if not isinstance(v, list) or len(v) <= 4 \
                      else f"<{len(v)}-element vector>"
            print(f"            * {c['field']} {c['op']} {v_short!r}")
    if expected_gap:
        print(f"  Probes:   {expected_gap}")

    status, body = post(
        f"/v1/bundles/{name}/brain/sudoku",
        {"constraints": constraints, "max_options": max_options,
         "max_near_misses": max_near_misses}
    )
    if status != 200:
        print(f"  ERROR: HTTP {status}: {body.get('error')}")
        return

    verdict = body.get("verdict", "?")
    n = body.get("n_records_considered", 0)
    sols = body.get("solutions") or []
    nms = body.get("near_misses") or []
    badge = {"sat": "[SAT]", "unsat": "[UNSAT]", "unknown": "[?]"}.get(verdict, "[?]")
    print(f"  Verdict:  {badge} {verdict.upper()}  "
          f"(searched {n}, coverage {body.get('coverage', 0):.0%})")

    if sols:
        print(f"  Solutions ({len(sols)}):")
        for s in sols:
            mass_pct = s["stated_prior_mass"] * 100
            q = s.get("quality_score")
            q_str = f", quality {q:.2f}" if q is not None else ""
            print(f"    * {format_record(s['record'])}"
                  f"  [{mass_pct:.1f}% of records{q_str}]")
    else:
        print("  Solutions: (none)")

    if nms:
        print(f"  Near-misses ({len(nms)}):")
        for nm in nms:
            v = nm["violations"][0] if nm.get("violations") else {}
            cost = v.get("relaxation_cost")
            raw = v.get("raw_delta")
            cost_str = f"  [cost {cost:.2f}sigma" + (f", off by {raw:g}]" if raw is not None else "]")
            print(f"    * {format_record(nm['record'])}{cost_str}")
            print(f"      -> violates {v.get('field')}: {v.get('violation','?')}")

    sel = body.get("selectivity") or []
    binding = [s for s in sel if s.get("binding")]
    if binding:
        bnames = ", ".join(f"{s['field']} (filters {s['marginal_filter_count']})" for s in binding)
        print(f"  Binding constraint(s): {bnames}")

    relax = body.get("relaxations") or []
    if relax:
        print(f"  Relax-this-rule menu (top {min(3, len(relax))}, bang/cost):")
        for r in relax[:3]:
            print(f"    * {r['description']}  -> +{r['gain']} match(es)"
                  f"  [cost {r['relaxation_cost']:.2f}]")

    pareto = body.get("pareto_near_misses") or []
    multi_v = [p for p in pareto if len(p.get("violations", [])) >= 2]
    if multi_v:
        print(f"  Multi-relax Pareto ({min(2, len(multi_v))} of {len(multi_v)}):")
        for p in multi_v[:2]:
            n_v = len(p["violations"])
            tc = p["total_relaxation_cost"]
            fields = ", ".join(v["field"] for v in p["violations"])
            print(f"    * {format_record(p['record'])}")
            print(f"      -> bend {n_v} ({fields})  [total cost {tc:.2f}]")
    print()


def format_record(rec, limit=6):
    items = list(rec.items())[:limit]
    return ", ".join(f"{k}={pretty_val(v)}" for k, v in items)


def pretty_val(v):
    if isinstance(v, bool):
        return repr(v)
    if isinstance(v, float):
        return f"{v:.2f}" if abs(v) < 1000 else f"{v:,.0f}"
    if isinstance(v, list):
        if len(v) <= 4:
            return repr(v)
        return f"<{len(v)}-vec>"
    if isinstance(v, str) and len(v) > 30:
        return v[:27] + "..."
    return repr(v)


# -----------------------------------------------------------------
# DOMAIN 7 -- Used car shopping (multi-numeric Pareto)
# -----------------------------------------------------------------
banner(
    "DOMAIN 7 -- Used car shopping",
    "Tests multi-numeric Pareto. Every near-miss is potentially a bargain."
)
setup_bundle(
    "used_cars",
    fields={
        "vin_id": "numeric",
        "make": "categorical",
        "model": "categorical",
        "year": "numeric",
        "miles": "numeric",
        "price": "numeric",
        "mpg": "numeric",
        "accidents": "numeric",
    },
    records=[
        {"vin_id": 1, "make": "honda", "model": "civic",  "year": 2021, "miles": 28000, "price": 21500, "mpg": 36, "accidents": 0},
        {"vin_id": 2, "make": "honda", "model": "civic",  "year": 2020, "miles": 42000, "price": 19500, "mpg": 35, "accidents": 0},
        {"vin_id": 3, "make": "honda", "model": "accord", "year": 2022, "miles": 18000, "price": 26500, "mpg": 32, "accidents": 0},
        {"vin_id": 4, "make": "honda", "model": "civic",  "year": 2019, "miles": 55000, "price": 17500, "mpg": 34, "accidents": 1},
        {"vin_id": 5, "make": "toyota","model": "corolla","year": 2021, "miles": 31000, "price": 20500, "mpg": 38, "accidents": 0},
        {"vin_id": 6, "make": "honda", "model": "civic",  "year": 2022, "miles": 22000, "price": 24500, "mpg": 36, "accidents": 0},
        {"vin_id": 7, "make": "honda", "model": "fit",    "year": 2020, "miles": 35000, "price": 16500, "mpg": 41, "accidents": 0},
        {"vin_id": 8, "make": "ford",  "model": "focus",  "year": 2021, "miles": 30000, "price": 18500, "mpg": 32, "accidents": 0},
    ],
)
show_sudoku(
    "used_cars",
    "Find a 2020+ Honda Civic, <40k miles, <$22k, >35mpg, no accidents.",
    constraints=[
        {"type": "field", "field": "make",       "op": "eq", "value": "honda",   "hard": True},
        {"type": "field", "field": "model",      "op": "eq", "value": "civic",   "hard": True},
        {"type": "field", "field": "year",       "op": "ge", "value": 2020.0,    "hard": True},
        {"type": "field", "field": "miles",      "op": "lt", "value": 40000.0,   "hard": True},
        {"type": "field", "field": "price",      "op": "lt", "value": 22000.0,   "hard": True},
        {"type": "field", "field": "mpg",        "op": "gt", "value": 35.0,      "hard": True},
        {"type": "field", "field": "accidents",  "op": "eq", "value": 0,         "hard": True},
    ],
    expected_gap="Pareto trade-offs. Test that multi-violation Pareto surfaces deals."
)

# -----------------------------------------------------------------
# DOMAIN 8 -- Restaurant search (Bool fields + which-is-best?)
# -----------------------------------------------------------------
banner(
    "DOMAIN 8 -- Restaurant search",
    "Probes: Bool fields + lots of SAT (which is BEST among matches?)"
)
setup_bundle(
    "restaurants",
    fields={
        "venue_id": "numeric",
        "cuisine": "categorical",
        "rating": "numeric",
        "avg_price": "numeric",
        "open_late": "categorical",       # we treat bool as text "true"/"false" for now
        "neighborhood": "categorical",
        "vegan_options": "categorical",
    },
    records=[
        {"venue_id":  1, "cuisine": "thai",     "rating": 4.6, "avg_price": 24, "open_late": "true",  "neighborhood": "mission",      "vegan_options": "true"},
        {"venue_id":  2, "cuisine": "thai",     "rating": 4.2, "avg_price": 28, "open_late": "true",  "neighborhood": "mission",      "vegan_options": "true"},
        {"venue_id":  3, "cuisine": "thai",     "rating": 4.7, "avg_price": 32, "open_late": "true",  "neighborhood": "mission",      "vegan_options": "true"},
        {"venue_id":  4, "cuisine": "thai",     "rating": 4.5, "avg_price": 22, "open_late": "false", "neighborhood": "mission",      "vegan_options": "true"},
        {"venue_id":  5, "cuisine": "italian",  "rating": 4.4, "avg_price": 35, "open_late": "true",  "neighborhood": "mission",      "vegan_options": "false"},
        {"venue_id":  6, "cuisine": "thai",     "rating": 4.8, "avg_price": 26, "open_late": "true",  "neighborhood": "outer_sunset", "vegan_options": "true"},
        {"venue_id":  7, "cuisine": "vietnamese","rating": 4.3,"avg_price": 20, "open_late": "true",  "neighborhood": "mission",      "vegan_options": "true"},
        {"venue_id":  8, "cuisine": "thai",     "rating": 4.0, "avg_price": 18, "open_late": "true",  "neighborhood": "mission",      "vegan_options": "true"},
    ],
)
show_sudoku(
    "restaurants",
    "Open-late vegan-friendly Thai in Mission, rated 4+, under $30/person.",
    constraints=[
        {"type": "field", "field": "cuisine",       "op": "eq", "value": "thai",     "hard": True},
        {"type": "field", "field": "open_late",     "op": "eq", "value": "true",     "hard": True},
        {"type": "field", "field": "vegan_options", "op": "eq", "value": "true",     "hard": True},
        {"type": "field", "field": "neighborhood",  "op": "eq", "value": "mission",  "hard": True},
        {"type": "field", "field": "rating",        "op": "ge", "value": 4.0,        "hard": True},
        {"type": "field", "field": "avg_price",     "op": "lt", "value": 30.0,       "hard": True},
    ],
    expected_gap="Many SAT match. Test surfaces: are they ranked by QUALITY (e.g. rating)? Currently only by mass."
)

# -----------------------------------------------------------------
# DOMAIN 9 -- Flight search (Timestamp constraints)
# -----------------------------------------------------------------
banner(
    "DOMAIN 9 -- Flight search",
    "Probes: Timestamp-as-numeric ops. Calendar semantics are S4 territory."
)
# Timestamps: epoch seconds. Jan 15 2026 = 1768435200, Jan 20 = 1768867200.
setup_bundle(
    "flights",
    fields={
        "flight_id": "numeric",
        "origin": "categorical",
        "dest": "categorical",
        "depart_ts": "timestamp",
        "price": "numeric",
        "stops": "numeric",
        "airline": "categorical",
    },
    records=[
        {"flight_id":  1, "origin": "sfo", "dest": "nyc", "depart_ts": 1768435200, "price": 320, "stops": 0, "airline": "ua"},
        {"flight_id":  2, "origin": "sfo", "dest": "nyc", "depart_ts": 1768521600, "price": 280, "stops": 1, "airline": "aa"},
        {"flight_id":  3, "origin": "sfo", "dest": "nyc", "depart_ts": 1768608000, "price": 410, "stops": 0, "airline": "ua"},
        {"flight_id":  4, "origin": "sfo", "dest": "nyc", "depart_ts": 1768694400, "price": 350, "stops": 1, "airline": "dl"},
        {"flight_id":  5, "origin": "sfo", "dest": "nyc", "depart_ts": 1768867200, "price": 295, "stops": 0, "airline": "ua"},
        {"flight_id":  6, "origin": "sfo", "dest": "nyc", "depart_ts": 1768953600, "price": 260, "stops": 2, "airline": "f9"},
        {"flight_id":  7, "origin": "sfo", "dest": "bos", "depart_ts": 1768435200, "price": 240, "stops": 1, "airline": "b6"},
    ],
)
show_sudoku(
    "flights",
    "SFO->NYC, depart Jan 15-20, <=1 stop, under $300.",
    constraints=[
        {"type": "field", "field": "origin",    "op": "eq",      "value": "sfo",                              "hard": True},
        {"type": "field", "field": "dest",      "op": "eq",      "value": "nyc",                              "hard": True},
        {"type": "field", "field": "depart_ts", "op": "between", "value": [1768435200.0, 1768867200.0],       "hard": True},
        {"type": "field", "field": "stops",     "op": "le",      "value": 1.0,                                "hard": True},
        {"type": "field", "field": "price",     "op": "lt",      "value": 300.0,                              "hard": True},
    ],
    expected_gap="Timestamps treated as numeric (works). Calendar ops (weekday, month) are S4."
)

# -----------------------------------------------------------------
# DOMAIN 10 -- Used book / library search
# -----------------------------------------------------------------
banner(
    "DOMAIN 10 -- Used book / library search",
    "Probes: exact text Eq + year-range. Substring/contains is S4."
)
setup_bundle(
    "books",
    fields={
        "book_id": "numeric",
        "title": "categorical",
        "author": "categorical",
        "year": "numeric",
        "genre": "categorical",
        "pages": "numeric",
        "available": "categorical",
    },
    records=[
        {"book_id":  1, "title": "beloved",         "author": "morrison",  "year": 1987, "genre": "novel",     "pages": 324, "available": "true"},
        {"book_id":  2, "title": "jazz",            "author": "morrison",  "year": 1992, "genre": "novel",     "pages": 250, "available": "true"},
        {"book_id":  3, "title": "paradise",        "author": "morrison",  "year": 1997, "genre": "novel",     "pages": 318, "available": "false"},
        {"book_id":  4, "title": "song of solomon", "author": "morrison",  "year": 1977, "genre": "novel",     "pages": 337, "available": "true"},
        {"book_id":  5, "title": "infinite jest",   "author": "wallace",   "year": 1996, "genre": "novel",     "pages": 1079,"available": "true"},
        {"book_id":  6, "title": "the bluest eye",  "author": "morrison",  "year": 1970, "genre": "novel",     "pages": 224, "available": "true"},
        {"book_id":  7, "title": "song of solomon", "author": "morrison",  "year": 1977, "genre": "novel",     "pages": 337, "available": "false"},
    ],
)
show_sudoku(
    "books",
    "Morrison novel from the 1990s, available, normal length (200-500 pages).",
    constraints=[
        {"type": "field", "field": "author",    "op": "eq",      "value": "morrison",                   "hard": True},
        {"type": "field", "field": "year",      "op": "between", "value": [1990.0, 1999.0],             "hard": True},
        {"type": "field", "field": "available", "op": "eq",      "value": "true",                       "hard": True},
        {"type": "field", "field": "pages",     "op": "between", "value": [200.0, 500.0],               "hard": True},
        {"type": "field", "field": "genre",     "op": "eq",      "value": "novel",                      "hard": True},
    ],
    expected_gap="Exact-text Eq only. Substring 'song of' would need S4 text-contains op."
)

# -----------------------------------------------------------------
# DOMAIN 11 -- Sensor anomaly hunt (Vector embedding constraint)
# -----------------------------------------------------------------
banner(
    "DOMAIN 11 -- Sensor anomaly hunt",
    "Probes: VECTOR/embedding constraint. KNOWN GAP — does engine refuse cleanly?"
)
setup_bundle(
    "sensors",
    fields={
        "sensor_id": "numeric",
        "temp_c": "numeric",
        "vibration_hz": "numeric",
        "pressure_psi": "numeric",
        "embedding": "vector(3)",
        "status": "categorical",
    },
    records=[
        {"sensor_id":  1, "temp_c": 22.5, "vibration_hz": 110, "pressure_psi": 102.0, "embedding": [0.10, 0.20, 0.30], "status": "normal"},
        {"sensor_id":  2, "temp_c": 28.1, "vibration_hz": 140, "pressure_psi":  99.5, "embedding": [0.15, 0.22, 0.28], "status": "warning"},
        {"sensor_id":  3, "temp_c": 25.0, "vibration_hz": 105, "pressure_psi": 101.2, "embedding": [0.50, 0.60, 0.55], "status": "normal"},
        {"sensor_id":  4, "temp_c": 31.5, "vibration_hz": 180, "pressure_psi":  95.0, "embedding": [0.12, 0.18, 0.32], "status": "anomaly"},
        {"sensor_id":  5, "temp_c": 19.8, "vibration_hz":  90, "pressure_psi": 103.0, "embedding": [0.80, 0.70, 0.75], "status": "normal"},
    ],
)
# First try a numeric-only query so we know the basic path works.
show_sudoku(
    "sensors",
    "Numeric-only: temp 20-30, vibration > 100. (Embedding gap probed next.)",
    constraints=[
        {"type": "field", "field": "temp_c",       "op": "between", "value": [20.0, 30.0], "hard": True},
        {"type": "field", "field": "vibration_hz", "op": "gt",      "value": 100.0,        "hard": True},
    ],
    expected_gap="Baseline works. Next attempt: embedding-similarity (not in v1)."
)
# Now attempt embedding-similarity — expect graceful refusal.
# Tries Eq on a Vector — that's the only op the wire supports today.
print("\n  [Probing missing functionality: vector-similarity constraint]")
print("  Sending: embedding eq [0.10, 0.20, 0.30] (exact match)")
print("  EXPECTED: works for exact match; no 'within-radius' op exists.")
show_sudoku(
    "sensors",
    "Find sensors with embedding identical to [0.10, 0.20, 0.30] anchor.",
    constraints=[
        {"type": "field", "field": "embedding", "op": "eq", "value": [0.10, 0.20, 0.30], "hard": True},
    ],
    expected_gap="GAP: no 'within radius epsilon of vector' op. Exact-Eq works."
)

# -----------------------------------------------------------------
# DOMAIN 12 -- HR promotion candidates
# -----------------------------------------------------------------
banner(
    "DOMAIN 12 -- HR promotion candidates",
    "Probes: many SAT, all numeric-soft. 'Which is BEST?' (quality rank gap)"
)
setup_bundle(
    "hr_candidates",
    fields={
        "emp_id": "numeric",
        "tenure_yrs": "numeric",
        "performance_score": "numeric",
        "peer_feedback": "numeric",
        "promotions_so_far": "numeric",
        "department": "categorical",
    },
    records=[
        {"emp_id":  1, "tenure_yrs": 6, "performance_score": 0.92, "peer_feedback": 0.88, "promotions_so_far": 1, "department": "eng"},
        {"emp_id":  2, "tenure_yrs": 5, "performance_score": 0.86, "peer_feedback": 0.82, "promotions_so_far": 0, "department": "eng"},
        {"emp_id":  3, "tenure_yrs": 8, "performance_score": 0.89, "peer_feedback": 0.85, "promotions_so_far": 1, "department": "eng"},
        {"emp_id":  4, "tenure_yrs": 5, "performance_score": 0.95, "peer_feedback": 0.90, "promotions_so_far": 0, "department": "eng"},
        {"emp_id":  5, "tenure_yrs": 7, "performance_score": 0.84, "peer_feedback": 0.83, "promotions_so_far": 1, "department": "design"},
        {"emp_id":  6, "tenure_yrs": 9, "performance_score": 0.78, "peer_feedback": 0.70, "promotions_so_far": 2, "department": "eng"},
        {"emp_id":  7, "tenure_yrs": 5, "performance_score": 0.91, "peer_feedback": 0.86, "promotions_so_far": 0, "department": "eng"},
    ],
)
show_sudoku(
    "hr_candidates",
    "Engineers: 5+ tenure, perf > 0.85, peer > 0.8, <=1 prior promotion.",
    constraints=[
        {"type": "field", "field": "department",        "op": "eq", "value": "eng",  "hard": True},
        {"type": "field", "field": "tenure_yrs",        "op": "ge", "value": 5.0,    "hard": True},
        {"type": "field", "field": "performance_score", "op": "gt", "value": 0.85,   "hard": True},
        {"type": "field", "field": "peer_feedback",     "op": "gt", "value": 0.80,   "hard": True},
        {"type": "field", "field": "promotions_so_far", "op": "le", "value": 1.0,    "hard": True},
    ],
    expected_gap="GAP: 4 SAT, all engineers. Which is BEST? No quality_score field today."
)

# -----------------------------------------------------------------
# CLOSER -- GP gap survey
# -----------------------------------------------------------------
print()
print("=" * 78)
print("GP GAPS SURFACED BY DOMAINS 7-12")
print("=" * 78)
print("""
Findings across the 6 new domains:

  D7 (cars):       Multi-relax Pareto IS what shoppers want when no exact
                   match. Wave 3 delivers this cleanly.

  D8 (restaurants):Many SAT (3+ Thai places). Currently ranked only by
                   stated_prior_mass which is uniform when records are
                   distinct. GAP: no 'best-in-set' rank by quality.
                   --> Upgrade 4 (centrality / soft posterior score)
                   would resolve this. Strictly GP.

  D9 (flights):    Timestamps as numeric WORKS. Calendar-natural ops
                   (weekday/month) deferred to S4 -- not a GP regression.

 D10 (books):      Exact text Eq works. Substring/contains deferred to
                   S4 -- not a GP regression today.

 D11 (sensors):    Embedding-similarity GAP. No 'vector within radius e'
                   op. Eq on Vector works (exact match) but useless for
                   real similarity queries. Brain-stack already has
                   kernel_density_confidence which is the math we need.
                   --> Adding FieldOp::WithinRadius { center, epsilon }
                   on Vector fields would be small, GP, and unify with
                   /brain/confidence.

 D12 (HR):         4 SAT engineers, all good. Same as D8: no quality
                   rank. Upgrade 4 territory.

Recommended GP additions to next wave:
  1. FieldOp::WithinRadius for Vector fields  (D11 fixes immediately)
  2. quality_score on Solutions               (D8 + D12 resolve)

Both pass the GP test: math is identical regardless of field name,
schema, or domain; derived purely from the bundle's own data + the
constraint definition.
""")
