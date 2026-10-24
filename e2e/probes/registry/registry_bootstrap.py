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
         "evidence_kind": "text", "locator": "text",
         # "subject", "object" or "" -- which side of the relation this evidence
         # locates the shared mechanism on. RESONANCE needs both sides.
         "locates_mechanism_for": "text"},
        ["id"],
    ),
    "research_checks": (
        {"id": "text", "relation_id": "text", "method": "text",
         "expected": "text", "actual": "text", "result": "text",
         "evidence_kind": "text", "control_ran": "text",
         "control_failed_as_required": "text", "environment": "text",
         "date": "text"},
        ["id"],
    ),
    "research_relation_assumptions": (
        {"id": "text", "relation_id": "text", "assumption": "text"},
        ["id"],
    ),
    # A reviewer's composition ruling on one ordered chain of edges. Keyed by the
    # path signature so the ruling is found again on the next traversal.
    "research_path_reviews": (
        {"id": "text", "path_signature": "text", "composition_state": "text",
         "reason": "text", "conditions": "text", "reviewed_by": "text",
         "reviewed_by_is_agent": "text"},
        ["id"],
    ),
    # Structured claims. Contradiction detection in version one compares these
    # four coordinates and nothing else; prose is never auto-labelled.
    "research_claims": (
        {"id": "text", "quantity": "text", "object_id": "text", "domain": "text",
         "conditions": "text", "value": "text", "source_id": "text"},
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
    # Gate 9 fixture. The blocked step is the double cover S^1 -> RP^1: the 2:1
    # map admits no continuous section, so the orientation is absent, not merely
    # degraded. "Invertible then lossy" would NOT work here -- that composes
    # fine and only fails to be invertible.
    {"id": "principal-axis-extraction", "name": "Principal axis extraction",
     "definition": "Returns the dominant axis of a symmetric form.",
     "domain": "estimation"},
    {"id": "unoriented-axis-RP1", "name": "Unoriented axis (element of RP^1)",
     "definition": "A line through the origin. Determined up to sign; carries no orientation.",
     "domain": "estimation"},
    {"id": "oriented-heading-S1", "name": "Oriented heading (element of S^1)",
     "definition": "A uniquely determined directed arrow.",
     "domain": "estimation"},
    # Gives the graph a SECOND multi-edge path, one that is genuinely composable.
    # Without it the "machine never self-assigns a ruling" control is vacuous:
    # all() over an empty list is true.
    {"id": "jacobian-of-a-differentiable-map", "name": "Jacobian of a differentiable map",
     "definition": "The matrix of first partial derivatives; the local linear part of the map.",
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

    # The composable counterpart: the pullback metric is built from the Jacobian.
    # Chains with the Davis Manifold edge to give a second two-edge path.
    {"id": "DG-REL-PULLBACK-JACOBIAN-001",
     "subject": "classical-pullback-metric-JtJ",
     "object": "jacobian-of-a-differentiable-map",
     "claimed_relation": "uses_definition", "direction": "directed",
     "statement": "The pullback metric J^T h J is built from the Jacobian of the map.",
     "scope": "Definitional.",
     "established_altitude": "INCIDENCE",
     "altitude_basis": "Standard construction, located in the cited reference.",
     "altitude_scope": "Differentiable maps of full column rank.",
     "lifecycle": "proposed", "lifecycle_note": "",
     "asserted_by": "fixture", "asserted_by_is_agent": "true",
     "reviewed_by": "", "reviewed_by_is_agent": "", "reviewer_decision": "unreviewed"},

    # Gate 9: a two-edge chain whose composition cannot be formed.
    {"id": "DG-REL-AXIS-FROM-EXTRACTION-001",
     "subject": "unoriented-axis-RP1", "object": "principal-axis-extraction",
     "claimed_relation": "derived_from", "direction": "directed",
     "statement": "The unoriented axis is the output of principal axis extraction.",
     "scope": "Output type only.",
     "established_altitude": "INCIDENCE",
     "altitude_basis": "Definitional, stated by the estimator.",
     "altitude_scope": "The estimator as defined.",
     "lifecycle": "proposed", "lifecycle_note": "",
     "asserted_by": "fixture", "asserted_by_is_agent": "true",
     "reviewed_by": "", "reviewed_by_is_agent": "", "reviewer_decision": "unreviewed"},
    {"id": "DG-REL-HEADING-FROM-AXIS-001",
     "subject": "oriented-heading-S1", "object": "unoriented-axis-RP1",
     "claimed_relation": "derived_from", "direction": "directed",
     "statement": "ASSERTED, NOT ESTABLISHED. An oriented heading is obtained from the unoriented axis.",
     "scope": "Blocked step. Recovering S^1 from RP^1 needs orientation information the axis does not carry.",
     "established_altitude": "NONE",
     "altitude_basis": "No evidence. The 2:1 cover S^1 -> RP^1 admits no continuous section.",
     "altitude_scope": "",
     "lifecycle": "proposed", "lifecycle_note": "",
     "asserted_by": "fixture", "asserted_by_is_agent": "true",
     "reviewed_by": "", "reviewed_by_is_agent": "", "reviewer_decision": "unreviewed"},
]

# Gate 10 fixtures. Real arithmetic, not invented: det J = r for the polar map,
# which is r on r > 0 and 0 at the origin. Same quantity and object; the r = 0
# row differs only by DOMAIN and so is not a contradiction.
CLAIMS = [
    {"id": "CLM-001", "quantity": "det J", "object_id": "classical-polar-coordinate-map",
     "domain": "r > 0", "conditions": "standard chart", "value": "r",
     "source_id": "SRC-MIT-JACOBIANS"},
    {"id": "CLM-002", "quantity": "det J", "object_id": "classical-polar-coordinate-map",
     "domain": "r > 0", "conditions": "standard chart", "value": "1",
     "source_id": "DG-REF-COORDINATE-SAMENESS-001"},
    {"id": "CLM-003", "quantity": "det J", "object_id": "classical-polar-coordinate-map",
     "domain": "r = 0", "conditions": "standard chart", "value": "0",
     "source_id": "SRC-MIT-JACOBIANS"},

    # Second contradiction fixture: a property rather than a value, and again
    # real. Arc length ds^2 = dr^2 + r^2 dtheta^2 is invariant under rotation of
    # the plane; a claim that a rotation changes it conflicts directly. The third
    # row is about a NON-rotation (a dilation), which genuinely does change it,
    # so it must not be flagged.
    {"id": "CLM-004", "quantity": "induced arc length", "object_id": "classical-polar-coordinate-map",
     "domain": "r > 0", "conditions": "rotation about the origin",
     "value": "invariant", "source_id": "SRC-MIT-JACOBIANS"},
    {"id": "CLM-005", "quantity": "induced arc length", "object_id": "classical-polar-coordinate-map",
     "domain": "r > 0", "conditions": "rotation about the origin",
     "value": "changes", "source_id": "DG-REF-COORDINATE-SAMENESS-001"},
    {"id": "CLM-006", "quantity": "induced arc length", "object_id": "classical-polar-coordinate-map",
     "domain": "r > 0", "conditions": "dilation by k != 1",
     "value": "changes", "source_id": "SRC-MIT-JACOBIANS"},
]

EVIDENCE = [
    # Mechanism located on BOTH sides, which is what RESONANCE requires.
    {"id": "EV-001", "relation_id": "DG-REL-DAVISMANIFOLD-PULLBACK-001",
     "source_id": "DG-REF-COORDINATE-SAMENESS-001", "evidence_kind": "source",
     "locator": "manuscript near line 445", "locates_mechanism_for": "subject"},
    {"id": "EV-002", "relation_id": "DG-REL-DAVISMANIFOLD-PULLBACK-001",
     "source_id": "SRC-MIT-JACOBIANS", "evidence_kind": "source",
     "locator": "lecture 14, pullback metric", "locates_mechanism_for": "object"},
    # One side only: enough for INCIDENCE, not for RESONANCE.
    {"id": "EV-003", "relation_id": "DG-REL-POLAR-SAMENESS-001",
     "source_id": "DG-REF-COORDINATE-SAMENESS-001", "evidence_kind": "source",
     "locator": "worked mathematical anchor", "locates_mechanism_for": "subject"},
    # DG-REL-PULLBACK-HOMONYM-001 deliberately has none.
]

CHECKS = []

ASSUMPTIONS = [
    {"id": "AS-001", "relation_id": "DG-REL-DAVISMANIFOLD-PULLBACK-001", "assumption": "full column rank J"},
    {"id": "AS-002", "relation_id": "DG-REL-DAVISMANIFOLD-PULLBACK-001", "assumption": "target metric h positive definite"},
    {"id": "AS-003", "relation_id": "DG-REL-POLAR-SAMENESS-001", "assumption": "r > 0"},
    {"id": "AS-004", "relation_id": "DG-REL-POLAR-SAMENESS-001", "assumption": "local injective angular chart"},
    {"id": "AS-005", "relation_id": "DG-REL-POLAR-SAMENESS-001", "assumption": "Euclidean target metric"},
    {"id": "AS-006", "relation_id": "DG-REL-AXIS-FROM-EXTRACTION-001",
     "assumption": "output is determined only up to sign"},
    {"id": "AS-007", "relation_id": "DG-REL-HEADING-FROM-AXIS-001",
     "assumption": "requires a uniquely determined orientation"},
    {"id": "AS-008", "relation_id": "DG-REL-HEADING-FROM-AXIS-001",
     "assumption": "no further orientation information is supplied"},
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
                       ("research_relation_assumptions", ASSUMPTIONS),
                       ("research_claims", CLAIMS),
                       ("research_checks", CHECKS)]:
        if not rows:
            print(f"  {name}: 0 rows (empty by design)")
            continue
        insert(name, rows)
        back = query(name)
        if len(back) != len(rows):
            raise SystemExit(f"{name}: sent {len(rows)}, engine holds {len(back)}")
        print(f"  {name}: {len(rows)} sent, {len(back)} read back")

    print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
