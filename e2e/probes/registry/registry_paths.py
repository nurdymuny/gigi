"""Path traversal and structured contradiction detection for the registry.

Two rules govern everything here.

A path is returned as a path. It is never collapsed into a derived relation, no
aggregate altitude is computed for it, and the edges' assumptions are never
merged into one list -- one edge may require a thing the next destroys, and a bag
of strings cannot say so.

A machine may only ever assign `composition_unchecked`. `compatible_under` and
`incompatible` are reviewer rulings, stored in research_path_reviews and found
again by path signature.
"""

from collections import defaultdict
from decimal import Decimal, InvalidOperation
import hashlib
from itertools import combinations

from registry_bootstrap import eq, query, write_verified
from registry_review import nonblank, relation, view

UNCHECKED = "composition_unchecked"


def _assumptions(relation_id):
    return [a["assumption"] for a in
            query("research_relation_assumptions", [eq("relation_id", relation_id)])]


def hydrate(edge):
    """One edge with everything it owns. Nothing here is shared or summarised."""
    assessment = view(edge["id"])
    return {
        "relation_id": edge["id"],
        "subject": edge["subject"],
        "object": edge["object"],
        "claimed_relation": edge["claimed_relation"],
        "direction": edge["direction"],
        "established_altitude": edge["established_altitude"],
        "altitude_scope": edge["altitude_scope"],
        "lifecycle": edge["lifecycle"],
        "assumptions": _assumptions(edge["id"]),
        "evidence": [dict(s, source_id=s["id"], evidence_kind=s["kind"])
                     for s in assessment["sources"]],
        "needs_review": assessment["needs_review"],
        "flags": assessment["flags"],
    }


def edges_from(concept_id):
    """Edges leaving a concept.

    Directed edges are walked subject -> object only. Walking one backwards would
    manufacture a converse the asserter never claimed. Symmetric edges may be
    entered from either end, which is what symmetric means.
    """
    out = query("research_relations", [eq("subject", concept_id)])
    back = [r for r in query("research_relations", [eq("object", concept_id)])
            if r["direction"] == "symmetric"]
    return out + back


def signature(edges):
    return ">".join(e["relation_id"] for e in edges)


def review_id(sig):
    """Stable across processes, machines and Python hash seeds."""
    return "PR-" + hashlib.sha256(sig.encode("utf-8")).hexdigest()


def _review(sig):
    rows = query("research_path_reviews", [eq("path_signature", sig)])
    canonical = [r for r in rows if r["id"] == review_id(sig)]
    # A new explicit ruling supersedes legacy random-ID rows. Without it,
    # conflicting legacy records have no reliable chronology: refuse to guess.
    candidates = canonical or rows
    if len(candidates) > 1:
        raise ValueError("ambiguous legacy path reviews; record an explicit replacement ruling")
    if not candidates:
        return None
    result = candidates[0]
    if result.get("reviewed_by_is_agent") not in {"true", "false"}:
        raise ValueError("stored reviewer_is_agent must be true or false")
    _validate_ruling(result["composition_state"], result["reason"],
                     result["conditions"], result["reviewed_by"],
                     result["reviewed_by_is_agent"] == "true")
    return result


def as_path(edges):
    """Assemble a path result.

    Deliberately absent, and asserted absent by gate 9: a derived relation, an
    aggregate altitude, and a merged assumption list.
    """
    sig = signature(edges)
    r = _review(sig)
    return {
        "path_signature": sig,
        "length": len(edges),
        "edges": edges,
        "composition_state": r["composition_state"] if r else UNCHECKED,
        "composition_reason": r["reason"] if r else "",
        "composition_conditions": r["conditions"] if r else "",
        "reviewed_by": r["reviewed_by"] if r else "",
        "reviewed_by_is_agent": r["reviewed_by_is_agent"] if r else "",
        "needs_review": any(e["needs_review"] for e in edges),
    }


def traverse(start, max_depth=3):
    """All paths leaving `start`, each returned whole."""
    found = []

    def walk(node, chain, used):
        if len(chain) >= max_depth:
            return
        for e in edges_from(node):
            if e["id"] in used:
                continue
            nxt = e["object"] if e["subject"] == node else e["subject"]
            step = chain + [hydrate(e)]
            found.append(as_path(step))
            walk(nxt, step, used | {e["id"]})

    walk(start, [], frozenset())
    return found


def _validate_ruling(state, reason, conditions, reviewer, reviewer_is_agent):
    if state not in {"compatible_under", "incompatible"}:
        raise ValueError("ruling must be compatible_under or incompatible; unchecked is the default")
    if not nonblank(reason) or not nonblank(reviewer):
        raise ValueError("a ruling requires a nonblank reason and named reviewer")
    if not isinstance(conditions, str):
        raise ValueError("conditions must be text")
    if state == "compatible_under" and not nonblank(conditions):
        raise ValueError("compatible_under requires explicit conditions")
    if type(reviewer_is_agent) is not bool:
        raise ValueError("reviewer_is_agent must be a boolean")


def _validate_path(sig):
    if not nonblank(sig):
        raise ValueError("a ruling requires an existing ordered path")
    ids = sig.split(">")
    if any(not nonblank(rid) for rid in ids) or len(set(ids)) != len(ids):
        raise ValueError("path must contain nonblank, distinct relation IDs")
    endpoints = None
    for rid in ids:
        edge = relation(rid)
        if edge is None:
            raise ValueError(f"path names missing relation {rid}")
        if edge["direction"] not in {"directed", "symmetric"}:
            raise ValueError(f"unknown direction on relation {rid}")
        orientations = [(edge["subject"], edge["object"])]
        if edge["direction"] == "symmetric":
            orientations.append((edge["object"], edge["subject"]))
        endpoints = {end for start, end in orientations
                     if endpoints is None or start in endpoints}
        if not endpoints:
            raise ValueError("path is not contiguous in a permitted direction")


def record_composition_review(sig, state, reason, conditions, reviewer,
                              reviewer_is_agent):
    """Replace the current ruling after validation and verify its stored content.

    Reviewer names are declarations, not authenticated identities. This is not
    an access-control boundary. Historical reviews are not an audit log.
    """
    _validate_ruling(state, reason, conditions, reviewer, reviewer_is_agent)
    _validate_path(sig)
    rid = review_id(sig)
    write_verified("research_path_reviews", {
        "id": rid, "path_signature": sig, "composition_state": state,
        "reason": reason, "conditions": conditions, "reviewed_by": reviewer,
        "reviewed_by_is_agent": "true" if reviewer_is_agent else "false"})
    return rid


# ── Structured contradiction detection ───────────────────────────────────────

COORDS = ("quantity", "object_id", "domain", "conditions")


def _typed_value(claim):
    kind, value = claim.get("value_kind"), claim["value"]
    # This schema stores exact asserted values as text. A legacy numeric JSON
    # value may already have lost spelling/precision; do not silently repair it.
    if not nonblank(value):
        return None
    if kind == "numeric":
        try:
            number = Decimal(value)
        except (InvalidOperation, ValueError):
            return None
        return (kind, number) if number.is_finite() else None
    if kind == "invariance" and value in {"invariant", "changes"}:
        return kind, value
    return None


def _comparison(left, right):
    """Compare only exact numbers and an explicit two-value property enum.

    Numeric means an exact asserted value, not a measured estimate with error
    bars. Coordinates must already match, including the quantity's units.
    Missing/unknown types and symbolic expressions require human review.
    """
    a, b = _typed_value(left), _typed_value(right)
    if a is None or b is None or a[0] != b[0]:
        return "unresolved"
    return "equal" if a[1] == b[1] else "contradiction"


def contradictions():
    """Incompatible typed assertions at identical coordinates; no symbolic algebra."""
    groups = defaultdict(list)
    for c in query("research_claims"):
        groups[tuple(c[k] for k in COORDS)].append(c)

    out = []
    for key, rows in groups.items():
        for a, b in combinations(rows, 2):
            if _comparison(a, b) == "contradiction":
                out.append({"coordinates": dict(zip(COORDS, key)),
                            "values": sorted({a["value"], b["value"]}),
                            "claim_ids": sorted([a["id"], b["id"]])})
    return sorted(out, key=lambda item: item["claim_ids"])


def possible_conflicts():
    """Context differences and unparsed assertions go to review, not proof."""
    groups = defaultdict(list)
    for c in query("research_claims"):
        groups[(c["quantity"], c["object_id"])].append(c)

    out = []
    for (q, obj), rows in groups.items():
        for a, b in combinations(rows, 2):
            contexts = {(r["domain"], r["conditions"]) for r in (a, b)}
            if len(contexts) > 1 or _comparison(a, b) == "unresolved":
                out.append({"quantity": q, "object_id": obj,
                            "contexts": sorted(contexts),
                            "reason": "different contexts" if len(contexts) > 1
                                      else "values require interpretation",
                            "claim_ids": sorted([a["id"], b["id"]])})
    return sorted(out, key=lambda item: item["claim_ids"])
