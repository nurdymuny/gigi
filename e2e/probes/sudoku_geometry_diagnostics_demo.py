"""SUDOKU geometry diagnostics demo — waves 5, 6.1, 6.2 across four domains.

Demonstrates three capabilities introduced in waves 5-6:

  W5  Pareto-optimal multi-violation near-misses
      Records non-dominated on (n_violations, total_relaxation_cost).
      Not just "I violated one rule" — "here's every record worth
      knowing about, ranked by how far it sits from feasibility."

  W6.1  Per-constraint raw curvature K_c
      K_c = fraction of bundle that fails this constraint, regardless
      of others. High K_c + high marginal = the deal-breaker.
      High K_c + zero marginal = constraint redundant with a sibling.
      Low K_c + high marginal = loose but uniquely distinguishing.

  W6.2  Čech pre-flight contradiction detection
      O(C²) pairwise check catches trivially contradictory constraint
      pairs (Eq+Eq different values, Le+Ge with inverted interval, etc.)
      BEFORE any bundle IO. n_records_considered = 0. The reason string
      names the field and both constraint indices.

All four domains run on the same endpoint, the same primitive, the
same math. Only the schema and the human question change.

Run against a local gigi-stream on port 3143.
"""
import http.client
import json
import random
import time

HOST, PORT = "localhost", 3143
CONN = http.client.HTTPConnection(HOST, PORT, timeout=30)
HEADERS = {"Content-Type": "application/json", "Connection": "keep-alive"}


def post(path, body):
    payload = json.dumps(body)
    try:
        CONN.request("POST", path, payload, HEADERS)
        resp = CONN.getresponse()
        data = resp.read()
        return resp.status, json.loads(data) if data else {}
    except (http.client.BadStatusLine, ConnectionResetError, OSError):
        CONN.close()
        CONN.connect()
        CONN.request("POST", path, payload, HEADERS)
        resp = CONN.getresponse()
        data = resp.read()
        return resp.status, json.loads(data) if data else {}


def setup_bundle(name, fields, records):
    post("/v1/bundles", {
        "name": name,
        "schema": {"fields": fields, "keys": list(fields.keys())[:1]},
    })
    for i in range(0, len(records), 200):
        post(f"/v1/bundles/{name}/insert", {"records": records[i:i+200]})


def banner(title, wave_tags=""):
    line = "=" * 76
    print(f"\n+{line}+")
    print(f"| {title:<74} |")
    if wave_tags:
        print(f"| {('tags: ' + wave_tags):<74} |")
    print(f"+{line}+")


def section(label):
    print(f"\n  ── {label} {'─' * (68 - len(label))}")


def run_sudoku(bundle_name, n_records, problem, constraints,
               max_options=5, max_near_misses=5):
    print(f"\n  Bundle  : {bundle_name}  ({n_records} records)")
    print(f"  Problem : {problem}")
    print(f"  Constraints ({len(constraints)}):")
    for i, c in enumerate(constraints):
        if c.get("type") == "field":
            print(f"    [{i}] {c['field']} {c['op']} {c['value']!r}  "
                  f"({'hard' if c.get('hard', True) else 'soft'})")

    t0 = time.perf_counter()
    status, body = post(
        f"/v1/bundles/{bundle_name}/brain/sudoku",
        {"constraints": constraints,
         "max_options": max_options,
         "max_near_misses": max_near_misses},
    )
    elapsed_ms = (time.perf_counter() - t0) * 1000

    if status != 200:
        print(f"  ERROR HTTP {status}: {body.get('error', body)}")
        return body

    verdict = body.get("verdict", "?")
    n_walked = body.get("n_records_considered", 0)
    badge = {"sat": "  [SAT]", "unsat": "[UNSAT]", "unknown": "[  ?  ]"}.get(
        verdict, "[     ]"
    )

    print(f"\n  {badge}  verdict={verdict}  coverage={body.get('coverage', 0):.0%}"
          f"  records_walked={n_walked}  elapsed={elapsed_ms:.1f}ms")

    # ── Pre-flight contradiction (W6.2) ─────────────────────────────
    reason = body.get("pre_flight_unsat_reason")
    if reason:
        print(f"\n  PRE-FLIGHT UNSAT — no records walked")
        print(f"  Contradiction: {reason}")

    # ── Solutions ───────────────────────────────────────────────────
    sols = body.get("solutions") or []
    if sols:
        print(f"\n  Solutions ({len(sols)}):")
        for s in sols:
            q = s.get("quality_score")
            q_str = f"  quality={q:.3f}" if q is not None else ""
            mass_pct = s["stated_prior_mass"] * 100
            print(f"    * {_fmt(s['record'])}"
                  f"  [mass={mass_pct:.1f}%{q_str}]")
    elif not reason:
        print("  Solutions: (none)")

    # ── Near-misses ─────────────────────────────────────────────────
    nms = body.get("near_misses") or []
    if nms:
        print(f"\n  Near-misses (single violation):")
        for nm in nms:
            viol = nm["violations"][0] if nm.get("violations") else {}
            cost = viol.get("relaxation_cost", "?")
            raw = viol.get("raw_delta")
            raw_str = f", raw_delta={raw:g}" if raw is not None else ""
            print(f"    * {_fmt(nm['record'])}"
                  f"  [cost={cost:.2f}σ{raw_str}]")
            print(f"      violates [{viol.get('constraint_idx')}] "
                  f"{viol.get('field')}: {viol.get('violation', '?')}")

    # ── Pareto multi-violation near-misses (W5) ──────────────────────
    pareto = body.get("pareto_near_misses") or []
    multi = [p for p in pareto if len(p.get("violations", [])) >= 2]
    if multi:
        print(f"\n  Pareto multi-relax frontier (W5)  [{len(multi)} entries]:")
        for p in multi[:4]:
            n_v = len(p["violations"])
            tc = p["total_relaxation_cost"]
            fields_hit = ", ".join(v["field"] for v in p["violations"])
            print(f"    * {_fmt(p['record'])}")
            print(f"      bend {n_v} constraints ({fields_hit})"
                  f"  total_cost={tc:.2f}σ")

    # ── Selectivity + raw_curvature (W3/W6.1) ───────────────────────
    sel = body.get("selectivity") or []
    if sel:
        print(f"\n  Constraint geometry (W3 selectivity + W6.1 curvature):")
        print(f"    {'idx':<4} {'field':<18} {'K_c (tight→)':>14}"
              f"  {'marginal':>10}  {'binding':>7}  {'diagnosis'}")
        print(f"    {'-'*4} {'-'*18} {'-'*14}  {'-'*10}  {'-'*7}  {'-'*20}")
        for s in sel:
            kc = s.get("raw_curvature", 0.0)
            mg = s.get("marginal_filter_count", 0)
            bnd = "★" if s.get("binding") else " "
            # Diagnosis from the (K_c, marginal) pair
            if kc > 0.5 and mg > 0:
                diag = "deal-breaker"
            elif kc > 0.5 and mg == 0:
                diag = "redundant (covered)"
            elif kc <= 0.2 and mg > 0:
                diag = "loose but unique"
            else:
                diag = "moderate"
            print(f"    [{s['constraint_idx']}]  {s['field']:<18} "
                  f"{kc:>13.1%}  {mg:>10}  {bnd:>7}  {diag}")

    # ── Relaxation menu (W3) ─────────────────────────────────────────
    relax = body.get("relaxations") or []
    if relax:
        print(f"\n  Relaxation menu (top 3, sorted by gain/cost):")
        for r in relax[:3]:
            gain_str = f"+{r['gain']} record{'s' if r['gain'] != 1 else ''}"
            print(f"    * [{r['constraint_idx']}] {r['description']:<40}"
                  f"  {gain_str}  cost={r['relaxation_cost']:.2f}σ")

    return body


def _fmt(rec, n=5):
    items = list(rec.items())[:n]
    return "  ".join(f"{k}={_pv(v)}" for k, v in items)


def _pv(v):
    if isinstance(v, float):
        return f"{v:.2f}" if abs(v) < 10_000 else f"{v:,.0f}"
    if isinstance(v, str) and len(v) > 25:
        return v[:22] + "..."
    return repr(v)


# =======================================================================
# DOMAIN 1 — NYC Apartment Search
# Focus: W6.1 constraint geometry (tight vs loose vs redundant)
#        W5 Pareto (records that violate both budget AND size)
#
# Schema: 120 listings, 6 fields.
# Problem: 2BR in Manhattan, ≤ $3500/mo, ≤ 30min commute to Midtown,
#          pet-friendly.
# =======================================================================
banner(
    "DOMAIN 1 — NYC Apartment Search",
    "W6.1 (constraint geometry), W5 (Pareto multi-violation)"
)

random.seed(42)
apt_records = []
for i in range(120):
    borough = random.choices(
        ["Manhattan", "Brooklyn", "Queens", "Bronx"],
        weights=[25, 40, 25, 10]
    )[0]
    bedrooms = random.choices([0, 1, 2, 3], weights=[20, 35, 30, 15])[0]
    # Rent correlates with borough + bedrooms
    base = {"Manhattan": 3800, "Brooklyn": 2600, "Queens": 2200, "Bronx": 1800}[borough]
    rent = base + bedrooms * 600 + random.gauss(0, 300)
    commute = (
        random.randint(8, 25) if borough == "Manhattan"
        else random.randint(20, 55)
    )
    pets = random.choices(["yes", "no"], weights=[35, 65])[0]
    apt_records.append({
        "listing_id": i + 1,
        "borough": borough,
        "bedrooms": bedrooms,
        "rent_mo": round(rent, 0),
        "commute_min": commute,
        "pet_friendly": pets,
    })

setup_bundle("nyc_apts", {
    "listing_id": "numeric",
    "borough": "categorical",
    "bedrooms": "numeric",
    "rent_mo": "numeric",
    "commute_min": "numeric",
    "pet_friendly": "categorical",
}, apt_records)

run_sudoku(
    "nyc_apts", len(apt_records),
    "2BR in Manhattan, ≤$3500/mo rent, ≤30min commute, pet-friendly",
    constraints=[
        {"type": "field", "field": "borough", "op": "eq",
         "value": "Manhattan", "hard": True},
        {"type": "field", "field": "bedrooms", "op": "ge",
         "value": 2, "hard": True},
        {"type": "field", "field": "rent_mo", "op": "le",
         "value": 3500, "hard": True},
        {"type": "field", "field": "commute_min", "op": "le",
         "value": 30, "hard": True},
        {"type": "field", "field": "pet_friendly", "op": "eq",
         "value": "yes", "hard": True},
    ],
    max_options=5,
    max_near_misses=4,
)

section("What the geometry tells us (W6.1)")
print("""  K_c (raw_curvature) measures each constraint's ABSOLUTE tightness
  independent of what other constraints are doing.

  Expected: rent_mo and borough will have high K_c (most of the
  120 records fail either rent≤3500 or borough=Manhattan alone).
  commute_min in Manhattan is actually loose (Manhattan listings
  mostly have short commutes). pet_friendly is moderately tight.

  The (K_c, marginal) pair tells you WHERE to negotiate first:
    - high K_c + high marginal = this is your bottleneck
    - high K_c + zero marginal = redundant with another constraint
    - low K_c + high marginal = cheap rule doing unique filtering
""")

section("What Pareto tells us (W5)")
print("""  Single-violation near-misses: listings that just missed one rule.
  Pareto multi-violation near-misses: listings that violate >=2 rules
  but are non-dominated on (n_violations, total_relaxation_cost).

  A listing at rent=$3600, 35min commute is Pareto: it's 1σ over on
  rent and only 0.3σ over on commute — that's closer to feasibility
  than one that's 0.5σ over on each. Domination is strict on BOTH
  axes: a record is dropped only if something else is at least as good
  on n_violations AND strictly cheaper on cost (or vice versa).
""")


# =======================================================================
# DOMAIN 2 — Clinical Trial Eligibility
# Focus: W6.2 pre-flight contradiction detection
#        Two scenarios: (a) overt contradiction caught before any walk,
#        (b) subtle contradiction that requires the data.
# =======================================================================
banner(
    "DOMAIN 2 — Clinical Trial Eligibility Screening",
    "W6.2 (Čech pre-flight contradiction)"
)

# 300 synthetic patient records
patient_records = []
random.seed(99)
conditions = ["T2D", "HTN", "COPD", "CKD", "none"]
for i in range(300):
    age = random.randint(18, 80)
    bmi = round(random.gauss(28, 6), 1)
    a1c = round(random.gauss(7.5, 1.8), 1) if random.random() < 0.6 else None
    egfr = random.randint(20, 120)
    cond = random.choices(conditions, weights=[30, 30, 15, 10, 15])[0]
    patient_records.append({
        "patient_id": i + 1,
        "age": age,
        "bmi": round(bmi, 1),
        "a1c": a1c if a1c is not None else 0.0,
        "egfr": egfr,
        "primary_condition": cond,
    })

setup_bundle("patients", {
    "patient_id": "numeric",
    "age": "numeric",
    "bmi": "numeric",
    "a1c": "numeric",
    "egfr": "numeric",
    "primary_condition": "categorical",
}, patient_records)

section("2a — Overt age contradiction (pre-flight fires, 0 records walked)")
print("  Query: pediatric (age < 18) AND geriatric (age >= 65) — impossible.\n")
run_sudoku(
    "patients", len(patient_records),
    "Find patients who are both pediatric AND geriatric",
    constraints=[
        {"type": "field", "field": "age", "op": "lt",
         "value": 18, "hard": True},
        {"type": "field", "field": "age", "op": "ge",
         "value": 65, "hard": True},
    ],
)

section("2b — Disjoint condition sets (pre-flight fires, 0 records walked)")
print("  Query: primary_condition in {T2D} AND primary_condition in {COPD} — no overlap.\n")
run_sudoku(
    "patients", len(patient_records),
    "Find patients whose primary condition is simultaneously T2D and COPD",
    constraints=[
        {"type": "field", "field": "primary_condition", "op": "is_in",
         "value": ["T2D"], "hard": True},
        {"type": "field", "field": "primary_condition", "op": "is_in",
         "value": ["COPD"], "hard": True},
    ],
)

section("2c — Tight but valid query (bundle walk happens, real UNSAT possible)")
print("  Query: eGFR ≥ 90 AND BMI ≥ 35 AND A1c ≥ 10 — valid but rare.\n")
run_sudoku(
    "patients", len(patient_records),
    "Obese patients with preserved kidney function and poor glycemic control",
    constraints=[
        {"type": "field", "field": "egfr", "op": "ge",
         "value": 90, "hard": True},
        {"type": "field", "field": "bmi", "op": "ge",
         "value": 35, "hard": True},
        {"type": "field", "field": "a1c", "op": "ge",
         "value": 10.0, "hard": True},
    ],
    max_near_misses=4,
)

section("What W6.2 proves (Čech pre-flight)")
print("""  For queries 2a and 2b: n_records_considered = 0.
  The contradiction is detected in O(C²) constraint-pair scan —
  no data needed. The reason string names the field and tells the
  consumer exactly which two constraints clash.

  For query 2c: the constraints are not trivially contradictory (eGFR,
  BMI, and A1c are independent fields). The bundle walk happens.
  If no patients match, that's real UNSAT derived from the data —
  not a constraint contradiction. The two cases produce the same
  verdict label ("unsat") but a completely different mechanism and
  a completely different remediation. W6.2 makes that visible.
""")


# =======================================================================
# DOMAIN 3 — Supply Chain Routing (4-constraint Pareto)
# Focus: W5 Pareto at higher constraint count.
#        A supplier that misses lead_time AND cost is worse than one
#        that only misses one of them — but if it misses two at low
#        individual cost it may still Pareto-dominate a record that
#        misses one at very high cost.
# =======================================================================
banner(
    "DOMAIN 3 — Supply Chain Routing",
    "W5 (Pareto at 4 constraints), W6.1 (curvature)"
)

random.seed(7)
regions = ["APAC", "EMEA", "AMER", "LATAM"]
supplier_records = []
for i in range(80):
    region = random.choice(regions)
    lead_days = random.randint(
        4 if region == "AMER" else 8,
        12 if region == "AMER" else 25,
    )
    cost_unit = round(random.gauss(
        12 if region == "AMER" else 18, 4
    ), 2)
    reliability = round(random.uniform(0.82, 0.995), 3)
    certifications = random.choices(
        ["ISO9001", "AS9100", "none"], weights=[50, 20, 30]
    )[0]
    supplier_records.append({
        "supplier_id": i + 1,
        "region": region,
        "lead_days": lead_days,
        "cost_per_unit": max(5.0, cost_unit),
        "reliability": reliability,
        "cert": certifications,
    })

setup_bundle("suppliers", {
    "supplier_id": "numeric",
    "region": "categorical",
    "lead_days": "numeric",
    "cost_per_unit": "numeric",
    "reliability": "numeric",
    "cert": "categorical",
}, supplier_records)

run_sudoku(
    "suppliers", len(supplier_records),
    "AMER supplier: lead≤7d, cost≤$14/unit, reliability≥0.97, ISO9001",
    constraints=[
        {"type": "field", "field": "region", "op": "eq",
         "value": "AMER", "hard": True},
        {"type": "field", "field": "lead_days", "op": "le",
         "value": 7, "hard": True},
        {"type": "field", "field": "cost_per_unit", "op": "le",
         "value": 14.0, "hard": True},
        {"type": "field", "field": "reliability", "op": "ge",
         "value": 0.97, "hard": True},
        {"type": "field", "field": "cert", "op": "eq",
         "value": "ISO9001", "hard": True},
    ],
    max_options=5,
    max_near_misses=5,
)

section("Reading the Pareto table (W5)")
print("""  At 5 constraints the Pareto frontier can have multi-violation entries
  that are genuinely useful: a supplier with lead_days=8 (1 day over,
  cost=0.07σ) AND cost=$14.50 (cost=0.12σ over) may have a lower
  total_relaxation_cost than one with just lead_days=10 (cost=0.50σ).

  The Pareto table surfaces both — without domination filtering, the
  consumer would see a huge list. With it, they see only the records
  where relaxing ANY subset of violated constraints produces a net gain.
  Multi-violation Pareto entries are negotiation starting points, not
  hard rejections.
""")


# =======================================================================
# DOMAIN 4 — Menu Engineering (restaurant recipe selection)
# Focus: overt categorical contradiction + curvature shape
#        A menu-planning query where the chef's criteria are
#        self-contradictory: must be vegan AND contain dairy.
# =======================================================================
banner(
    "DOMAIN 4 — Menu Engineering / Recipe Selection",
    "W6.2 (categorical contradiction), W6.1 (curvature)"
)

random.seed(13)
cuisines = ["Italian", "Japanese", "Mexican", "Indian", "American"]
dish_records = []
for i in range(60):
    cuisine = random.choice(cuisines)
    is_vegan = random.choices(["yes", "no"], weights=[20, 80])[0]
    has_dairy = random.choices(["yes", "no"], weights=[55, 45])[0]
    if is_vegan == "yes":
        has_dairy = "no"   # vegan dishes don't have dairy
    prep_min = random.randint(10, 60)
    cost_plate = round(random.uniform(4.0, 22.0), 2)
    dish_records.append({
        "dish_id": i + 1,
        "cuisine": cuisine,
        "vegan": is_vegan,
        "has_dairy": has_dairy,
        "prep_min": prep_min,
        "cost_plate": cost_plate,
    })

setup_bundle("menu", {
    "dish_id": "numeric",
    "cuisine": "categorical",
    "vegan": "categorical",
    "has_dairy": "categorical",
    "prep_min": "numeric",
    "cost_plate": "numeric",
}, dish_records)

section("4a — Vegan AND dairy-containing (data contradiction, not pre-flight)")
print("  These are DIFFERENT fields — pre-flight can't catch it. Bundle walk required.\n")
run_sudoku(
    "menu", len(dish_records),
    "Find vegan dishes that contain dairy (impossible by construction)",
    constraints=[
        {"type": "field", "field": "vegan", "op": "eq",
         "value": "yes", "hard": True},
        {"type": "field", "field": "has_dairy", "op": "eq",
         "value": "yes", "hard": True},
    ],
    max_near_misses=4,
)

section("4b — Same field: cuisine must be Eq(Italian) AND Eq(Japanese)")
print("  Pre-flight catches this — same field, different values.\n")
run_sudoku(
    "menu", len(dish_records),
    "Find dishes that are simultaneously Italian and Japanese",
    constraints=[
        {"type": "field", "field": "cuisine", "op": "eq",
         "value": "Italian", "hard": True},
        {"type": "field", "field": "cuisine", "op": "eq",
         "value": "Japanese", "hard": True},
    ],
)

section("4c — Valid query: fast, affordable, vegan Italian")
run_sudoku(
    "menu", len(dish_records),
    "Vegan Italian under 25 min and $10/plate",
    constraints=[
        {"type": "field", "field": "cuisine", "op": "eq",
         "value": "Italian", "hard": True},
        {"type": "field", "field": "vegan", "op": "eq",
         "value": "yes", "hard": True},
        {"type": "field", "field": "prep_min", "op": "le",
         "value": 25, "hard": True},
        {"type": "field", "field": "cost_plate", "op": "le",
         "value": 10.0, "hard": True},
    ],
    max_near_misses=4,
)

section("Key distinction: pre-flight vs data UNSAT (W6.2)")
print("""  4a is UNSAT — but only because the bundle was constructed so that
  vegan=yes implies has_dairy=no (a domain invariant baked into the
  data). The constraint set is not trivially contradictory on its face:
  two different fields with compatible-looking values. The bundle walk
  is required to discover this.

  4b is UNSAT — and detected BEFORE any walk. Same field, different
  values → structurally impossible regardless of the data.

  Same verdict label, fundamentally different meaning:
    4a: the world doesn't have what you want
    4b: you asked for something that can't exist
  W6.2 makes the distinction visible in the response.
""")

print("\n" + "=" * 78)
print("  SUMMARY")
print("=" * 78)
print("""
  W5  Pareto multi-violation near-misses
      Every domain above produces Pareto entries when constraints are
      tight enough that no single record satisfies all of them.
      The frontier is the minimal set of records worth negotiating over.

  W6.1  Constraint curvature K_c
      Printed as a table for domains 1 and 3. K_c diagnoses the
      constraint-interaction geometry: which rule is the real barrier
      (high K_c + high marginal), which is redundant (high K_c + zero
      marginal), and which is a cheap differentiator (low K_c + non-zero
      marginal). Same math as sudoky-energy's K_loc scheduling signal.

  W6.2  Čech pre-flight
      Domains 2a, 2b, 4b: contradictory constraint pairs caught in O(C²)
      before any bundle IO. n_records_considered = 0 is the observable
      proof. The reason string names the field and constraint indices.
      Domains 2c, 4a: valid constraint pairs that happen to have no
      matching records — the bundle walk is required and does happen.

  Four domains. One primitive. Same math. The schema is what changes.
""")
