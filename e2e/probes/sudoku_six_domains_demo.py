"""SUDOKU usefulness demo — 6 domains, one primitive.

Showing that the same /v1/bundles/{name}/brain/sudoku endpoint
solves real problems across very different domains. The math is
identical; the schema is what changes.

What's load-bearing per domain:
  1. **Solutions ranked by frequency** — common cases first
  2. **Near-misses with relaxation guidance** — "if you raised the
     ceiling $50, 3 more match" — the differentiator vs plain WHERE
  3. **Honest tristate verdict** — distinguish "no matches exist"
     from "I gave up exploring"

Run against a local gigi-stream on port 3143.
"""
import json
import sys
import urllib.request

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
        return e.code, json.loads(e.read())


def setup_bundle(name, fields, records):
    """Create + populate a bundle. Idempotent."""
    schema = {
        "name": name,
        "schema": {"fields": fields, "keys": list(fields.keys())[:1]},
    }
    post("/v1/bundles", schema)  # idempotent (409 ok)
    post(f"/v1/bundles/{name}/insert", {"records": records})


def banner(title, subtitle=""):
    line = "=" * 76
    print(f"\n+{line}+")
    print(f"| {title:<74} |")
    if subtitle:
        print(f"| {subtitle:<74} |")
    print(f"+{line}+")


def show_sudoku(name, problem, constraints, max_options=3, max_near_misses=2):
    print(f"\n  Problem:  {problem}")
    print(f"  Query:    {len(constraints)} constraints")
    for c in constraints:
        if c["type"] == "field":
            print(f"            * {c['field']} {c['op']} {c['value']!r}")
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
          f"(searched {n} records, "
          f"coverage {body.get('coverage', 0):.0%})")
    if sols:
        print(f"  Solutions ({len(sols)}):")
        for s in sols:
            mass_pct = s["stated_prior_mass"] * 100
            rec = s["record"]
            q = s.get("quality_score")
            q_str = f", quality {q:.2f}" if q is not None else ""
            print(f"    * {format_record(rec)}  [{mass_pct:.1f}% of records{q_str}]")
    else:
        print("  Solutions: (none)")
    if nms:
        print(f"  Near-misses ({len(nms)}):")
        for nm in nms:
            rec = nm["record"]
            v = nm["violations"][0] if nm.get("violations") else {}
            cost = v.get("relaxation_cost")
            raw = v.get("raw_delta")
            cost_str = f"  [cost {cost:.2f}sigma" + (f", off by {raw:g}]" if raw is not None else "]")
            print(f"    * {format_record(rec)}{cost_str}")
            print(f"      -> violates {v.get('field')}: "
                  f"{v.get('violation', '?')}")
    elif sols:
        print("  Near-misses: (none -- all alternatives violate >=2 constraints)")

    # Wave 3 — Upgrade 2: selectivity report
    sel = body.get("selectivity") or []
    if sel:
        binding_idxs = [s["constraint_idx"] for s in sel if s.get("binding")]
        if binding_idxs:
            binding_fields = ", ".join(
                f"{s['field']} (filters {s['marginal_filter_count']})"
                for s in sel if s.get("binding")
            )
            print(f"  Binding constraint(s): {binding_fields}")

    # Wave 3 — Upgrade 3: relaxation menu (top 3)
    relax = body.get("relaxations") or []
    if relax:
        print(f"  Relax-this-rule menu (top {min(3, len(relax))}, bang/cost):")
        for r in relax[:3]:
            print(f"    * {r['description']}  -> +{r['gain']} match(es)"
                  f"  [cost {r['relaxation_cost']:.2f}]")

    # Wave 3 — Upgrade 5: multi-violation Pareto (only show if it
    # adds something vs single-violation near_misses)
    pareto = body.get("pareto_near_misses") or []
    multi_v = [p for p in pareto if len(p.get("violations", [])) >= 2]
    if multi_v:
        print(f"  Multi-relax Pareto ({len(multi_v[:2])} of {len(multi_v)}):")
        for p in multi_v[:2]:
            rec = p["record"]
            n_v = len(p["violations"])
            tc = p["total_relaxation_cost"]
            fields = ", ".join(v["field"] for v in p["violations"])
            print(f"    * {format_record(rec)}")
            print(f"      -> bend {n_v} constraints ({fields})"
                  f"  [total cost {tc:.2f}]")
    print()


def format_record(rec):
    # Pretty print: pick the 3-4 most interesting fields per domain.
    interesting = list(rec.items())[:6]
    return ", ".join(f"{k}={pretty_val(v)}" for k, v in interesting)


def pretty_val(v):
    if isinstance(v, float):
        return f"{v:.2f}" if abs(v) < 1000 else f"{v:,.0f}"
    if isinstance(v, str) and len(v) > 30:
        return v[:27] + "..."
    return repr(v)


# -----------------------------------------------------------------
# DOMAIN 1 -- Drug discovery (binding affinity lookup)
# -----------------------------------------------------------------
banner(
    "DOMAIN 1 -- Drug discovery: finding compounds that bind a target",
    "Real biomedical query. SUDOKU shape == bindingdb lookup."
)
setup_bundle(
    "drug_binding",
    fields={
        "ligand_id": "numeric",
        "target": "categorical",
        "compound_class": "categorical",
        "pki": "numeric",
        "ki_nm": "numeric",
        "fda_approved": "categorical",
    },
    records=[
        {"ligand_id": 1, "target": "EGFR", "compound_class": "tki",
         "pki": 9.2, "ki_nm": 0.6, "fda_approved": "yes"},
        {"ligand_id": 2, "target": "EGFR", "compound_class": "tki",
         "pki": 8.7, "ki_nm": 2.0, "fda_approved": "yes"},
        {"ligand_id": 3, "target": "EGFR", "compound_class": "tki",
         "pki": 8.1, "ki_nm": 7.9, "fda_approved": "no"},
        {"ligand_id": 4, "target": "EGFR", "compound_class": "antibody",
         "pki": 7.5, "ki_nm": 31.6, "fda_approved": "yes"},
        {"ligand_id": 5, "target": "EGFR", "compound_class": "tki",
         "pki": 6.8, "ki_nm": 158.5, "fda_approved": "no"},
        {"ligand_id": 6, "target": "EGFR", "compound_class": "tki",
         "pki": 6.3, "ki_nm": 501.2, "fda_approved": "no"},
        {"ligand_id": 7, "target": "HER2", "compound_class": "tki",
         "pki": 8.9, "ki_nm": 1.3, "fda_approved": "yes"},
        {"ligand_id": 8, "target": "VEGFR2", "compound_class": "tki",
         "pki": 8.4, "ki_nm": 4.0, "fda_approved": "yes"},
    ],
)
show_sudoku(
    "drug_binding",
    "Find FDA-approved EGFR inhibitors with sub-nanomolar binding (pki >= 9).",
    constraints=[
        {"type": "field", "field": "target", "op": "eq",
         "value": "EGFR", "hard": True},
        {"type": "field", "field": "pki", "op": "ge",
         "value": 9.0, "hard": True},
        {"type": "field", "field": "fda_approved", "op": "eq",
         "value": "yes", "hard": True},
    ],
)

# -----------------------------------------------------------------
# DOMAIN 2 -- Real estate
# -----------------------------------------------------------------
banner(
    "DOMAIN 2 -- Apartment search with budget + pet constraint",
    "The near-miss IS the value: 'if you went up $200, 3 more match'."
)
setup_bundle(
    "apartments",
    fields={
        "listing_id": "numeric",
        "neighborhood": "categorical",
        "bedrooms": "numeric",
        "rent": "numeric",
        "pets_ok": "categorical",
        "sqft": "numeric",
    },
    records=[
        {"listing_id": 1, "neighborhood": "mission", "bedrooms": 2,
         "rent": 3800, "pets_ok": "yes", "sqft": 900},
        {"listing_id": 2, "neighborhood": "mission", "bedrooms": 2,
         "rent": 4200, "pets_ok": "yes", "sqft": 1000},
        {"listing_id": 3, "neighborhood": "soma", "bedrooms": 2,
         "rent": 4500, "pets_ok": "yes", "sqft": 1100},
        {"listing_id": 4, "neighborhood": "mission", "bedrooms": 2,
         "rent": 3950, "pets_ok": "no", "sqft": 850},
        {"listing_id": 5, "neighborhood": "mission", "bedrooms": 1,
         "rent": 2900, "pets_ok": "yes", "sqft": 600},
        {"listing_id": 6, "neighborhood": "mission", "bedrooms": 2,
         "rent": 3700, "pets_ok": "yes", "sqft": 880},
        {"listing_id": 7, "neighborhood": "outer_sunset", "bedrooms": 2,
         "rent": 3500, "pets_ok": "yes", "sqft": 950},
        {"listing_id": 8, "neighborhood": "mission", "bedrooms": 3,
         "rent": 4800, "pets_ok": "yes", "sqft": 1300},
    ],
)
show_sudoku(
    "apartments",
    "Find 2BR in Mission, pet-friendly, under $4000/mo.",
    constraints=[
        {"type": "field", "field": "neighborhood", "op": "eq",
         "value": "mission", "hard": True},
        {"type": "field", "field": "bedrooms", "op": "eq",
         "value": 2, "hard": True},
        {"type": "field", "field": "rent", "op": "le",
         "value": 4000.0, "hard": True},
        {"type": "field", "field": "pets_ok", "op": "eq",
         "value": "yes", "hard": True},
    ],
    max_options=3,
    max_near_misses=3,
)

# -----------------------------------------------------------------
# DOMAIN 3 -- Recipe / cooking
# -----------------------------------------------------------------
banner(
    "DOMAIN 3 -- Recipe search with dietary + time constraint",
    "Allergies + time pressure = real shopping list problem."
)
setup_bundle(
    "recipes",
    fields={
        "recipe_id": "numeric",
        "name": "categorical",
        "cuisine": "categorical",
        "prep_min": "numeric",
        "vegetarian": "categorical",
        "contains_nuts": "categorical",
        "difficulty": "categorical",
    },
    records=[
        {"recipe_id": 1, "name": "pasta_pomodoro", "cuisine": "italian",
         "prep_min": 20, "vegetarian": "yes", "contains_nuts": "no",
         "difficulty": "easy"},
        {"recipe_id": 2, "name": "pasta_pesto", "cuisine": "italian",
         "prep_min": 15, "vegetarian": "yes", "contains_nuts": "yes",
         "difficulty": "easy"},
        {"recipe_id": 3, "name": "lentil_dal", "cuisine": "indian",
         "prep_min": 35, "vegetarian": "yes", "contains_nuts": "no",
         "difficulty": "medium"},
        {"recipe_id": 4, "name": "stir_fry_tofu", "cuisine": "asian",
         "prep_min": 25, "vegetarian": "yes", "contains_nuts": "no",
         "difficulty": "easy"},
        {"recipe_id": 5, "name": "chicken_curry", "cuisine": "indian",
         "prep_min": 40, "vegetarian": "no", "contains_nuts": "no",
         "difficulty": "medium"},
        {"recipe_id": 6, "name": "caprese_salad", "cuisine": "italian",
         "prep_min": 10, "vegetarian": "yes", "contains_nuts": "no",
         "difficulty": "easy"},
        {"recipe_id": 7, "name": "thai_pad_thai", "cuisine": "asian",
         "prep_min": 30, "vegetarian": "yes", "contains_nuts": "yes",
         "difficulty": "medium"},
    ],
)
show_sudoku(
    "recipes",
    "Find vegetarian recipes, nut-free (allergy), under 30 min prep.",
    constraints=[
        {"type": "field", "field": "vegetarian", "op": "eq",
         "value": "yes", "hard": True},
        {"type": "field", "field": "contains_nuts", "op": "eq",
         "value": "no", "hard": True},
        {"type": "field", "field": "prep_min", "op": "le",
         "value": 30.0, "hard": True},
    ],
    max_options=3,
    max_near_misses=2,
)

# -----------------------------------------------------------------
# DOMAIN 4 -- Hiring / candidate matching
# -----------------------------------------------------------------
banner(
    "DOMAIN 4 -- Engineering candidate matching",
    "is_in for skill sets; near-miss surfaces 'almost qualified'."
)
setup_bundle(
    "candidates",
    fields={
        "candidate_id": "numeric",
        "primary_lang": "categorical",
        "years_exp": "numeric",
        "remote_ok": "categorical",
        "timezone": "categorical",
        "expected_salary_k": "numeric",
    },
    records=[
        {"candidate_id": 1, "primary_lang": "rust", "years_exp": 6,
         "remote_ok": "yes", "timezone": "PT", "expected_salary_k": 220},
        {"candidate_id": 2, "primary_lang": "rust", "years_exp": 8,
         "remote_ok": "yes", "timezone": "ET", "expected_salary_k": 260},
        {"candidate_id": 3, "primary_lang": "python", "years_exp": 5,
         "remote_ok": "yes", "timezone": "ET", "expected_salary_k": 180},
        {"candidate_id": 4, "primary_lang": "rust", "years_exp": 4,
         "remote_ok": "yes", "timezone": "ET", "expected_salary_k": 195},
        {"candidate_id": 5, "primary_lang": "go", "years_exp": 7,
         "remote_ok": "no", "timezone": "PT", "expected_salary_k": 240},
        {"candidate_id": 6, "primary_lang": "rust", "years_exp": 9,
         "remote_ok": "yes", "timezone": "CT", "expected_salary_k": 280},
        {"candidate_id": 7, "primary_lang": "python", "years_exp": 6,
         "remote_ok": "yes", "timezone": "ET", "expected_salary_k": 200},
    ],
)
show_sudoku(
    "candidates",
    "Find senior Rust engineers (5+ yrs) -- remote-OK, in PT or ET, <= $250k.",
    constraints=[
        {"type": "field", "field": "primary_lang", "op": "eq",
         "value": "rust", "hard": True},
        {"type": "field", "field": "years_exp", "op": "ge",
         "value": 5.0, "hard": True},
        {"type": "field", "field": "remote_ok", "op": "eq",
         "value": "yes", "hard": True},
        {"type": "field", "field": "timezone", "op": "is_in",
         "value": ["PT", "ET"], "hard": True},
        {"type": "field", "field": "expected_salary_k", "op": "le",
         "value": 250.0, "hard": True},
    ],
    max_options=3,
    max_near_misses=2,
)

# -----------------------------------------------------------------
# DOMAIN 5 -- Stock screening
# -----------------------------------------------------------------
banner(
    "DOMAIN 5 -- Stock screening (value-investing filter)",
    "PRISM-shaped query: hard financial criteria + tristate verdict."
)
setup_bundle(
    "stocks",
    fields={
        "ticker_id": "numeric",
        "ticker": "categorical",
        "sector": "categorical",
        "pe_ratio": "numeric",
        "dividend_yield_pct": "numeric",
        "market_cap_b": "numeric",
    },
    records=[
        {"ticker_id": 1, "ticker": "GOOD_A", "sector": "tech",
         "pe_ratio": 18.5, "dividend_yield_pct": 3.2,
         "market_cap_b": 250.0},
        {"ticker_id": 2, "ticker": "GOOD_B", "sector": "tech",
         "pe_ratio": 19.8, "dividend_yield_pct": 3.5,
         "market_cap_b": 180.0},
        {"ticker_id": 3, "ticker": "FAIR_A", "sector": "tech",
         "pe_ratio": 22.0, "dividend_yield_pct": 2.8,
         "market_cap_b": 120.0},
        {"ticker_id": 4, "ticker": "EXP_A", "sector": "tech",
         "pe_ratio": 45.0, "dividend_yield_pct": 0.0,
         "market_cap_b": 1500.0},
        {"ticker_id": 5, "ticker": "FIN_A", "sector": "finance",
         "pe_ratio": 12.0, "dividend_yield_pct": 4.5,
         "market_cap_b": 90.0},
        {"ticker_id": 6, "ticker": "TECH_C", "sector": "tech",
         "pe_ratio": 19.5, "dividend_yield_pct": 2.9,
         "market_cap_b": 50.0},
        {"ticker_id": 7, "ticker": "TECH_D", "sector": "tech",
         "pe_ratio": 16.0, "dividend_yield_pct": 3.8,
         "market_cap_b": 300.0},
    ],
)
show_sudoku(
    "stocks",
    "Value-investing screen: tech, P/E <= 20, dividend >= 3%, large-cap (>= $100B).",
    constraints=[
        {"type": "field", "field": "sector", "op": "eq",
         "value": "tech", "hard": True},
        {"type": "field", "field": "pe_ratio", "op": "le",
         "value": 20.0, "hard": True},
        {"type": "field", "field": "dividend_yield_pct", "op": "ge",
         "value": 3.0, "hard": True},
        {"type": "field", "field": "market_cap_b", "op": "ge",
         "value": 100.0, "hard": True},
    ],
    max_options=3,
    max_near_misses=2,
)

# -----------------------------------------------------------------
# DOMAIN 6 -- Music playlist generation
# -----------------------------------------------------------------
banner(
    "DOMAIN 6 -- Music: build a focused-work playlist",
    "Between-range constraint + multi-field aesthetic filter."
)
setup_bundle(
    "tracks",
    fields={
        "track_id": "numeric",
        "title": "categorical",
        "genre": "categorical",
        "bpm": "numeric",
        "energy": "numeric",
        "instrumental": "categorical",
    },
    records=[
        {"track_id": 1, "title": "ambient_1", "genre": "electronic",
         "bpm": 95, "energy": 0.55, "instrumental": "yes"},
        {"track_id": 2, "title": "post_rock_a", "genre": "post_rock",
         "bpm": 110, "energy": 0.65, "instrumental": "yes"},
        {"track_id": 3, "title": "post_rock_b", "genre": "post_rock",
         "bpm": 120, "energy": 0.78, "instrumental": "yes"},
        {"track_id": 4, "title": "electronic_a", "genre": "electronic",
         "bpm": 125, "energy": 0.82, "instrumental": "yes"},
        {"track_id": 5, "title": "indie_pop", "genre": "indie",
         "bpm": 118, "energy": 0.7, "instrumental": "no"},
        {"track_id": 6, "title": "electronic_b", "genre": "electronic",
         "bpm": 135, "energy": 0.9, "instrumental": "yes"},
        {"track_id": 7, "title": "post_rock_c", "genre": "post_rock",
         "bpm": 90, "energy": 0.4, "instrumental": "yes"},
        {"track_id": 8, "title": "electronic_c", "genre": "electronic",
         "bpm": 122, "energy": 0.75, "instrumental": "yes"},
    ],
)
show_sudoku(
    "tracks",
    "Focused-work playlist: instrumental only, BPM 110-130, energy >= 0.7.",
    constraints=[
        {"type": "field", "field": "instrumental", "op": "eq",
         "value": "yes", "hard": True},
        {"type": "field", "field": "bpm", "op": "between",
         "value": [110.0, 130.0], "hard": True},
        {"type": "field", "field": "energy", "op": "ge",
         "value": 0.7, "hard": True},
    ],
    max_options=3,
    max_near_misses=2,
)

# -----------------------------------------------------------------
# CLOSER
# -----------------------------------------------------------------
print()
print("=" * 78)
print("WHAT THIS DEMONSTRATES")
print("=" * 78)
print(
    """
Same primitive, six domains. The math is identical; only the schema
changes. Each query gives back THREE things plain SQL doesn't:

  1. STATED-PRIOR MASS -- solutions ranked by how common they are in
     the underlying data (not just first-N-matches in insertion
     order). Tells the consumer which match is the "default" answer.

  2. NEAR-MISSES -- records that violate exactly one constraint, with
     the relaxation that would unlock them. This is the value-add
     over a WHERE clause: 'no exact match, but if you can stretch
     to $4200, listing #2 works'.

  3. HONEST TRISTATE VERDICT -- Sat / Unsat / Unknown. Plain SQL
     returns [] for both 'searched everything, nothing matched' AND
     'I gave up'. SUDOKU distinguishes -- Unsat is a guarantee,
     Unknown is a request for more time.

That's the difference between a query engine and a primitive that
helps consumers REASON about their search.
"""
)
