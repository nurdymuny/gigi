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

from registry_bootstrap import call, eq, query

UNCHECKED = "composition_unchecked"


def _assumptions(relation_id):
    return [a["assumption"] for a in
            query("research_relation_assumptions", [eq("relation_id", relation_id)])]


def _evidence(relation_id):
    rows = query("research_relation_evidence", [eq("relation_id", relation_id)])
    return [{"source_id": r["source_id"], "evidence_kind": r["evidence_kind"]}
            for r in rows]


def hydrate(edge):
    """One edge with everything it owns. Nothing here is shared or summarised."""
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
        "evidence": _evidence(edge["id"]),
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


def _review(sig):
    rows = query("research_path_reviews", [eq("path_signature", sig)])
    return rows[0] if rows else None


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


def record_composition_review(sig, state, reason, conditions, reviewer,
                              reviewer_is_agent):
    """Persist a reviewer's ruling. Only a human-or-named reviewer calls this."""
    if state == UNCHECKED:
        raise ValueError("unchecked is the machine default, not a ruling")
    rid = "PR-" + str(abs(hash(sig)) % 10**8)
    call("/v1/bundles/research_path_reviews/insert", {"records": [{
        "id": rid, "path_signature": sig, "composition_state": state,
        "reason": reason, "conditions": conditions, "reviewed_by": reviewer,
        "reviewed_by_is_agent": "true" if reviewer_is_agent else "false"}]})
    return rid


# ── Structured contradiction detection ───────────────────────────────────────

COORDS = ("quantity", "object_id", "domain", "conditions")


def contradictions():
    """Version one: four coordinates must match exactly and values must differ.

    Prose is not examined and is never auto-labelled `contradicts_under`. Two
    claims about the same quantity on different domains are not in conflict, and
    this returns nothing for them.
    """
    groups = defaultdict(list)
    for c in query("research_claims"):
        groups[tuple(c[k] for k in COORDS)].append(c)

    out = []
    for key, rows in groups.items():
        values = sorted({r["value"] for r in rows})
        if len(values) > 1:
            out.append({"coordinates": dict(zip(COORDS, key)),
                        "values": values,
                        "claim_ids": sorted(r["id"] for r in rows)})
    return out


def possible_conflicts():
    """Same quantity and object, differing domain or conditions.

    These are routed for review and are explicitly NOT contradictions. Returned
    separately so the two can never be confused at the call site.
    """
    groups = defaultdict(list)
    for c in query("research_claims"):
        groups[(c["quantity"], c["object_id"])].append(c)

    out = []
    for (q, obj), rows in groups.items():
        contexts = {(r["domain"], r["conditions"]) for r in rows}
        if len(contexts) > 1:
            out.append({"quantity": q, "object_id": obj,
                        "contexts": sorted(contexts),
                        "claim_ids": sorted(r["id"] for r in rows)})
    return out
