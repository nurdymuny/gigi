"""Bootstrap the research relationship registry against a live GIGI engine.

Implements the first-release slice of docs/RESEARCH_RELATIONSHIP_REGISTRY_PROPOSAL.md:
save, inspect evidence, and the integrity gates that do not need path traversal.

Per AGENTS.md: 127.0.0.1 not localhost; REST not /v1/gql (which can return
{"status":"ok"} having executed nothing); planted answers chosen in advance.

Arrays are normalised into join bundles rather than packed into strings, because
gate 3 must find every relation resting on a source. GIGI has no boolean field
type (unknown types fall back to Categorical), so flags are stored as text.
"""

import json
import sys
import urllib.error
import urllib.request

BASE = "http://127.0.0.1:3143"


def call(path, payload=None, method=None):
    url = BASE + path
    data = json.dumps(payload).encode() if payload is not None else None
    req = urllib.request.Request(
        url, data=data, method=method or ("POST" if data else "GET"),
        headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            return r.status, json.loads(r.read().decode() or "{}")
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read().decode() or "{}")


def create_bundle(name, fields, keys):
    status, body = call("/v1/bundles", {"name": name,
                                        "schema": {"fields": fields, "keys": keys}})
    if status not in (200, 201):
        raise SystemExit(f"create {name} failed: {status} {body}")
    return body


def insert(name, records):
    status, body = call(f"/v1/bundles/{name}/insert", {"records": records})
    if status not in (200, 201):
        raise SystemExit(f"insert into {name} failed: {status} {body}")
    # AGENTS.md: a reported count is not proof. Confirm by reading back.
    return body


def query(name, conditions=None, limit=1000):
    payload = {"limit": limit}
    if conditions:
        payload["conditions"] = conditions
    status, body = call(f"/v1/bundles/{name}/query", payload)
    if status != 200:
        raise SystemExit(f"query {name} failed: {status} {body}")
    return body.get("data", [])


def eq(field, value):
    return {"field": field, "op": "eq", "value": value}


# ── Schemas ──────────────────────────────────────────────────────────────────

SCHEMAS = {
    "research_sources": (
        {"id": "text", "author": "text", "title": "text", "uri": "text",
         "version_hash": "text", "locator": "text", "access_scope": "text",
         "captured_date": "text", "availability": "text"},
        ["id"],
    ),
    "research_concepts": (
        {"id": "text", "name": "text", "definition": "text", "domain": "text"},
        ["id"],
    ),
    "research_relations": (
        {"id": "text", "subject": "text", "object": "text",
         "claimed_relation": "text", "direction": "text", "statement": "text",
         "scope": "text", "established_altitude": "text",
         "altitude_basis": "text", "altitude_scope": "text",
         "lifecycle": "text", "lifecycle_note": "text",
         "asserted_by": "text", "asserted_by_is_agent": "text",
         "reviewed_by": "text", "reviewed_by_is_agent": "text",
         "reviewer_decision": "text"},
        ["id"],
    ),
    # Join bundles. Arrays are not packed into strings: gate 3 has to find every
    # relation resting on a source, which a substring search cannot do honestly.
    "research_relation_evidence": (
        {"id": "text", "relation_id": "text", "source_id": "text",
         "evidence_kind": "text"},
        ["id"],
    ),
    "research_relation_assumptions": (
        {"id": "text", "relation_id": "text", "assumption": "text"},
        ["id"],
    ),
}

# ── Content ──────────────────────────────────────────────────────────────────

SOURCES = [
    {"id": "DG-REF-COORDINATE-SAMENESS-001",
     "author": "Davis Geometric research note",
     "title": "Coordinate translations, Jacobians, and the Geometry of Sameness",
     "uri": "docs/research/coordinate-translations-and-sameness.md",
     "version_hash": "", "locator": "whole note",
     "access_scope": "collaborators", "captured_date": "2026-10-24",
     "availability": "available"},
    {"id": "SRC-MIT-JACOBIANS",
     "author": "MIT 18.02 course notes",
     "title": "Jacobians and curvilinear coordinates",
     "uri": "https://web.mit.edu/8.Math/www/lectures/lec14/lec14.pdf",
     "version_hash": "", "locator": "lecture 14",
     "access_scope": "public", "captured_date": "2026-10-24",
     "availability": "available"},
]

CONCEPTS = [
    {"id": "davis-manifold-metric-construction", "name": "Davis Manifold metric construction",
     "definition": "Metric built by pullback along a differentiable translator.",
     "domain": "Davis Geometric"},
    {"id": "classical-pullback-metric-JtJ", "name": "Classical pullback metric J^T h J",
     "definition": "Pullback of a target metric along a map; f*h = J^T h J.",
     "domain": "differential geometry"},
    {"id": "classical-polar-coordinate-map", "name": "Polar to Cartesian map",
     "definition": "F(r,theta) = (r cos theta, r sin theta) on r > 0.",
     "domain": "differential geometry"},
    {"id": "geometry-of-sameness-translation-geometry-relationship",
     "name": "Geometry of Sameness translation/geometry question",
     "definition": "When do translator-based and manifold-based systems realise the same semantic sameness structure.",
     "domain": "Davis Geometric"},
    {"id": "gigi-pullback-join", "name": "GIGI pullback join",
     "definition": "Fiber product over a key. src/join.rs:17, POST /v1/bundles/{name}/join.",
     "domain": "GIGI engine"},
    {"id": "differential-geometric-pullback-metric", "name": "Differential-geometric pullback metric",
     "definition": "Precomposition along a map, f*h = J^T h J.",
     "domain": "differential geometry"},
]

RELATIONS = [
    {"id": "DG-REL-DAVISMANIFOLD-PULLBACK-001",
     "subject": "davis-manifold-metric-construction",
     "object": "classical-pullback-metric-JtJ",
     "claimed_relation": "uses_definition", "direction": "directed",
     "statement": "The Davis Manifold metric construction uses the classical pullback metric J^T h J.",
     "scope": "The shared construction only. Not a claim about the manuscript's detection or error-budget results.",
     "established_altitude": "RESONANCE",
     "altitude_basis": "Mechanism named and located on both sides.",
     "altitude_scope": "Holds for the metric construction as written in that passage. No numerical or proof check performed.",
     "lifecycle": "proposed",
     "lifecycle_note": "Manuscript locator is a local path. A hash fixes the version; it does not make it readable by a collaborator.",
     "asserted_by": "Astra", "asserted_by_is_agent": "true",
     "reviewed_by": "", "reviewed_by_is_agent": "", "reviewer_decision": "unreviewed"},
    {"id": "DG-REL-POLAR-SAMENESS-001",
     "subject": "classical-polar-coordinate-map",
     "object": "geometry-of-sameness-translation-geometry-relationship",
     "claimed_relation": "illustrates", "direction": "directed",
     "statement": "An exact coordinate transformation illustrates how translated descriptions can share a consistent geometric measurement rule.",
     "scope": "Explanatory example. Does not establish the manuscript's general equivalence or error-transfer claims.",
     "established_altitude": "INCIDENCE",
     "altitude_basis": "A source with a precise locator shows the example illustrates the question. No mechanism located on the manuscript side.",
     "altitude_scope": "The polar chart on r > 0 only.",
     "lifecycle": "proposed", "lifecycle_note": "",
     "asserted_by": "Astra", "asserted_by_is_agent": "true",
     "reviewed_by": "", "reviewed_by_is_agent": "", "reviewer_decision": "unreviewed"},
    {"id": "DG-REL-PULLBACK-HOMONYM-001",
     "subject": "gigi-pullback-join",
     "object": "differential-geometric-pullback-metric",
     "claimed_relation": "uses_same_mechanism", "direction": "symmetric",
     "statement": "ASSERTED, NOT ESTABLISHED. Both are called 'pullback'.",
     "scope": "Adversarial fixture. A fiber product over a key is not precomposition along a map.",
     "established_altitude": "NONE",
     "altitude_basis": "No mechanism located on both sides. A shared term is not a mechanism.",
     "altitude_scope": "",
     "lifecycle": "proposed", "lifecycle_note": "",
     "asserted_by": "fixture", "asserted_by_is_agent": "true",
     "reviewed_by": "", "reviewed_by_is_agent": "", "reviewer_decision": "unreviewed"},
]

EVIDENCE = [
    {"id": "EV-001", "relation_id": "DG-REL-DAVISMANIFOLD-PULLBACK-001",
     "source_id": "DG-REF-COORDINATE-SAMENESS-001", "evidence_kind": "source"},
    {"id": "EV-002", "relation_id": "DG-REL-DAVISMANIFOLD-PULLBACK-001",
     "source_id": "SRC-MIT-JACOBIANS", "evidence_kind": "source"},
    {"id": "EV-003", "relation_id": "DG-REL-POLAR-SAMENESS-001",
     "source_id": "DG-REF-COORDINATE-SAMENESS-001", "evidence_kind": "source"},
    # DG-REL-PULLBACK-HOMONYM-001 deliberately has none.
]

ASSUMPTIONS = [
    {"id": "AS-001", "relation_id": "DG-REL-DAVISMANIFOLD-PULLBACK-001", "assumption": "full column rank J"},
    {"id": "AS-002", "relation_id": "DG-REL-DAVISMANIFOLD-PULLBACK-001", "assumption": "target metric h positive definite"},
    {"id": "AS-003", "relation_id": "DG-REL-POLAR-SAMENESS-001", "assumption": "r > 0"},
    {"id": "AS-004", "relation_id": "DG-REL-POLAR-SAMENESS-001", "assumption": "local injective angular chart"},
    {"id": "AS-005", "relation_id": "DG-REL-POLAR-SAMENESS-001", "assumption": "Euclidean target metric"},
]


def main():
    print("=" * 78)
    print("Registry bootstrap against a live engine at", BASE)
    print("=" * 78)

    for name, (fields, keys) in SCHEMAS.items():
        create_bundle(name, fields, keys)
        print(f"  created {name} ({len(fields)} fields, key={keys})")

    for name, rows in [("research_sources", SOURCES), ("research_concepts", CONCEPTS),
                       ("research_relations", RELATIONS),
                       ("research_relation_evidence", EVIDENCE),
                       ("research_relation_assumptions", ASSUMPTIONS)]:
        insert(name, rows)
        back = query(name)
        if len(back) != len(rows):
            raise SystemExit(f"{name}: sent {len(rows)}, engine holds {len(back)}")
        print(f"  {name}: {len(rows)} sent, {len(back)} read back")

    print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
