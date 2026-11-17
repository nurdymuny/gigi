"""Fail-closed verifier for the isolated MONODROMY_GROUP_ORDER live run."""
from pathlib import Path
import hashlib
import json
import math
import os
import tempfile

from monodromy_exact_oracle import oracle


# Independent known group invariants, not values learned from an HTTP response.
CLOSED = {
    "icosahedral": (120, 120, {1: 1, 2: 1, 3: 20, 4: 30, 5: 24, 6: 20, 10: 24}),
    "identity": (1, 1, {1: 1}),
    "central": (2, 1, {1: 1, 2: 1}),
    "cyclic5": (5, 1, {1: 1, 5: 4}),
    "quaternion": (8, 2, {1: 1, 2: 1, 4: 6}),
    "tetrahedral": (24, 8, {1: 1, 2: 1, 3: 8, 4: 6, 6: 8}),
    "s_only": (6, 1, {1: 1, 2: 1, 3: 2, 6: 2}),
    "t_only": (10, 1, {1: 1, 2: 1, 5: 4, 10: 4}),
}
for alias in ["reordered", "conjugated", "redundant", "inverse"]:
    CLOSED[alias] = CLOSED["icosahedral"]
UNRESOLVED = {"cap119": 119, "irrational": 32}
REFUSED = {"missing": "not declared", "bad_edge": "out of range", "wrong_group": "SU(2)"}


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def require(ok, message):
    if not ok:
        raise ValueError(message)


def strict_json(text):
    def reject_constant(x):
        raise ValueError(f"nonfinite JSON: {x}")
    def unique_keys(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, f"duplicate JSON key: {key}")
            result[key] = value
        return result
    return json.loads(text, parse_constant=reject_constant, object_pairs_hook=unique_keys)


def checked_row(record, name):
    require(record["status"] == 200, f"{name}: HTTP status")
    rows = record["response"].get("rows")
    require(isinstance(rows, list) and len(rows) == 1, f"{name}: actual computed row required")
    row = rows[0]
    require(row["scope"] == "supplied_generator_subgroup", f"{name}: scope")
    require(row["group_used"] == "SU(2)", f"{name}: group")
    require(row["numerical_policy"] == "su2_closure_v1", f"{name}: policy")
    require(row["exact_subgroup_certified"] is False, f"{name}: false exactness claim")
    require(row["tolerance"] == 1e-8, f"{name}: tolerance")
    for key in ["max_input_norm_error", "max_closure_residual"]:
        require(isinstance(row[key], (int, float)) and math.isfinite(row[key]), f"{name}: {key}")
        require(0 <= row[key] <= (1e-10 if key == "max_input_norm_error" else 1e-8), f"{name}: {key} bound")
    return row


def verify(path: Path):
    path = Path(path)
    receipt = path.with_suffix(".PASS.json")
    # Invalidate before reading anything, including a missing or corrupt input.
    receipt.unlink(missing_ok=True)
    raw = path.read_bytes()
    data = strict_json(raw.decode("utf-8"))
    require(data.get("schema") == "monodromy_live_v1", "missing evidence schema")
    require(len(data.get("head", "")) == 40, "missing source revision")
    require(sha(data["binary"]["path"]) == data["binary"]["sha256"], "binary changed")
    sources = data["sources"]
    require(any(p.endswith("/src/monodromy.rs") for p in sources), "missing kernel hash")
    require(any(p.endswith("/src/bin/gigi_stream.rs") for p in sources), "missing HTTP hash")
    require(any(p.endswith("/monodromy_exact_oracle.py") for p in sources), "missing oracle hash")
    for source, digest in sources.items():
        require(sha(source) == digest, f"source changed: {source}")
    exact = oracle()
    require(data["oracle"] == json.loads(json.dumps(exact)), "oracle mismatch")
    require(exact["group_order"] == 120 and exact["commutator_subgroup_order"] == 120, "oracle failed")
    requests = data["requests"]
    by_name = {}
    for record in requests:
        name = record["label"]
        require(strict_json(record["raw_response"]) == record["response"], f"{name}: raw/parsed response mismatch")
        require(name not in by_name, f"duplicate request label: {name}")
        by_name[name] = record
    required = set(CLOSED) | set(UNRESOLVED) | set(REFUSED)
    actual = {r["label"] for r in requests if r["request"].get("query", "").startswith("MONODROMY_GROUP_ORDER ")}
    require(actual == required, "missing or unexpected monodromy cases")
    for name in required:
        record = by_name[name]
        require(record["path"] == "/v1/gql", f"{name}: not native GQL")
        require(" COMMUTATOR " in record["request"]["query"], f"{name}: missing commutator request")
    for name, (order, derived, histogram) in CLOSED.items():
        row = checked_row(by_name[name], name)
        require(row["status"] == "numerically_closed" and row["table_verified"] is True, f"{name}: not verified")
        require(row["reason"] is None, f"{name}: conflicting failure")
        require(row["group_order"] == order and row["n_elements_observed"] == order, f"{name}: order")
        require(row["commutator_subgroup_order"] == derived, f"{name}: derived subgroup")
        require(row["is_perfect"] is (order == derived), f"{name}: perfection")
        require(row["abelianization_order"] == order // derived, f"{name}: abelianization")
        require(row["element_orders"] == list(histogram), f"{name}: element orders")
        require(row["element_order_counts"] == list(histogram.values()), f"{name}: histogram counts")
        require(row["associativity_checks"] == order ** 3, f"{name}: incomplete table verification")
        require(row["products_evaluated"] >= order ** 2, f"{name}: incomplete product table")
        if order == 1:
            require(row["min_element_separation"] is None, f"{name}: identity separation")
        else:
            require(math.isfinite(row["min_element_separation"]) and row["min_element_separation"] > 8e-8, f"{name}: separation")
    for name, cap in UNRESOLVED.items():
        row = checked_row(by_name[name], name)
        require(row["status"] == "unresolved" and row["table_verified"] is False, f"{name}: false closure")
        require(row["cap"] == cap and row["n_elements_observed"] == cap, f"{name}: ignored cap")
        for key in ["group_order", "commutator_subgroup_order", "is_perfect", "abelianization_order"]:
            require(row[key] is None, f"{name}: invented {key}")
        require(row["element_orders"] == [] and row["element_order_counts"] == [], f"{name}: invented histogram")
        require("exhausted" in row["reason"], f"{name}: missing actionable reason")
    for name, fragment in REFUSED.items():
        record = by_name[name]
        require(record["status"] >= 400 and fragment in record["response"].get("error", ""), f"{name}: refusal missing")
    result = {"status": "PASS", "scope": "isolated live demonstration, not overall release gate",
              "evidence_sha256": hashlib.sha256(raw).hexdigest(), "cases_verified": len(required),
              "binary_sha256": data["binary"]["sha256"], "exact_subgroup_certified": False}
    with tempfile.NamedTemporaryFile("w", dir=receipt.parent, delete=False, encoding="utf-8") as stream:
        tmp = Path(stream.name)
        json.dump(result, stream, indent=2, allow_nan=False)
        stream.flush()
        os.fsync(stream.fileno())
    try:
        os.replace(tmp, receipt)
    finally:
        tmp.unlink(missing_ok=True)
    return result


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("evidence", type=Path)
    print(json.dumps(verify(parser.parse_args().evidence), indent=2))
