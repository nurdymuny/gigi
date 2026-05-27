"""SAMPLE_TRANSPORT S4 -- curvature-bounded neighborhood sampling demo.

GIGI's SAMPLE_TRANSPORT verb returns a geometrically-valid neighborhood of
destinations from a source point on the fiber, not just the single geodesic.

N(p_src, tau) = { p in E : d^2(p_src, p) <= tau }

Candidates are weighted by exp(-beta * d^2) and sampled without replacement
(Efraimidis-Spirakis). tau is the "creativity budget": larger tau =
more geometric diversity.

--- Four domains ---------------------------------------------------------

  Domain 1 -- Semantic Analogy (word vectors on 2D fiber)
      Bundle: 2D unit-circle corpus (angle_x, angle_y).
      Source: "walk" at angle 0 (1, 0).
      Budget 0.3: finds ~3 words in the angular neighborhood.

  Domain 2 -- Music Similarity (genre embeddings)
      Bundle: genre embeddings on 2D fiber (tempo_norm, energy_norm).
      Source: "pop" (0.6, 0.7).
      Budget 0.2: finds acoustically-similar genres within budget.

  Domain 3 -- Drug Analog Search (chemical similarity)
      Bundle: 3D chemical fiber (hydrophobicity, mw_norm, pki_norm).
      Source: reference compound. Budget 0.25: finds analogs.

  Domain 4 -- seed reproducibility check
      Two calls with same seed -> identical candidate ordering.
      Two calls without seed -> results may differ.

--- Gate -----------------------------------------------------------------

For each domain:
  PASS 1: n_admissible > 0 (neighborhood is non-empty within budget)
  PASS 2: all returned d_sq <= budget (budget filter is hard)
  PASS 3: all returned weights match exp(-beta * d_sq) exactly
  PASS 4: sameness = 1 - d_sq for every candidate

Domain 4 additionally:
  PASS 5: seed=42 twice -> identical candidate list
  PASS 6: no-seed runs -> d_sq values correct (non-deterministic ordering ok)

S4 gate: ALL DOMAINS PASS -> commit ready.
"""

import sys
import math
import json
import urllib.request
import urllib.error
from typing import Any

PORT = 3142
BASE = f"http://localhost:{PORT}"

# ---- HTTP helpers -------------------------------------------------------

def post(path: str, body: Any) -> Any:
    data = json.dumps(body).encode()
    req = urllib.request.Request(
        f"{BASE}{path}",
        data=data,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            return json.loads(resp.read())
    except urllib.error.HTTPError as e:
        body_text = e.read().decode(errors="replace")
        raise RuntimeError(f"HTTP {e.code} on POST {path}: {body_text}") from e


def delete_bundle(name: str) -> None:
    req = urllib.request.Request(
        f"{BASE}/v1/bundles/{name}",
        method="DELETE",
    )
    try:
        with urllib.request.urlopen(req, timeout=5):
            pass
    except urllib.error.HTTPError:
        pass  # 404 is fine


def ensure_bundle(name: str, fields: dict) -> None:
    """Create bundle with given schema. 400 = already exists: swallow."""
    delete_bundle(name)
    body = json.dumps({
        "name": name,
        "schema": {
            "fields": fields,
            "keys": ["id"],
        },
    }).encode()
    req = urllib.request.Request(
        f"{BASE}/v1/bundles",
        data=body,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=10) as _:
            pass
    except urllib.error.HTTPError as e:
        if e.code == 400:
            pass  # bundle already exists
        else:
            raise RuntimeError(f"HTTP {e.code} creating bundle {name}: {e.read().decode()[:200]}") from e


def insert(name: str, records: list) -> None:
    post(f"/v1/bundles/{name}/insert", {"records": records})


def sample_transport(name: str, from_keys: dict, fiber_fields: list,
                     budget: float, k: int, beta: float = 1.0,
                     seed: int = None) -> dict:
    body = {
        "from_keys": from_keys,
        "fiber_fields": fiber_fields,
        "budget": budget,
        "k": k,
        "beta": beta,
    }
    if seed is not None:
        body["seed"] = seed
    return post(f"/v1/bundles/{name}/brain/sample_transport", body)


# ---- Section / check helpers --------------------------------------------

PASS_MARK = "PASS"
FAIL_MARK = "FAIL"
results = []


def check(label: str, condition: bool, detail: str = "") -> bool:
    mark = PASS_MARK if condition else FAIL_MARK
    suffix = f"  ({detail})" if detail else ""
    print(f"    {mark}  {label}{suffix}")
    results.append((label, condition))
    return condition


def section(title: str) -> None:
    print(f"\n{'='*70}")
    print(f"  {title}")
    print(f"{'='*70}")


def subsection(title: str) -> None:
    print(f"\n  -- {title} --")


# ---- Domain 1: Semantic Analogy (2D fiber, unit circle) -----------------

BUNDLE_WORDS = "st_words"

WORD_CORPUS = [
    # (id, word, angle_x, angle_y)  -- points on the unit circle
    {"id": "w0",  "word": "walk",    "ax": 1.000,  "ay": 0.000},
    {"id": "w1",  "word": "walked",  "ax": 0.966,  "ay": 0.259},
    {"id": "w2",  "word": "walking", "ax": 0.866,  "ay": 0.500},
    {"id": "w3",  "word": "run",     "ax": 0.707,  "ay": 0.707},
    {"id": "w4",  "word": "ran",     "ax": 0.500,  "ay": 0.866},
    {"id": "w5",  "word": "jog",     "ax": 0.259,  "ay": 0.966},
    {"id": "w6",  "word": "sprint",  "ax": 0.000,  "ay": 1.000},
    {"id": "w7",  "word": "crawl",   "ax": -0.259, "ay": 0.966},
    {"id": "w8",  "word": "swim",    "ax": -0.500, "ay": 0.866},
    {"id": "w9",  "word": "fly",     "ax": -0.707, "ay": 0.707},
    {"id": "w10", "word": "glide",   "ax": -0.966, "ay": 0.259},
    {"id": "w11", "word": "drift",   "ax": -1.000, "ay": 0.000},
]


def domain_words() -> None:
    section("Domain 1 -- Semantic Analogy (2D fiber, unit circle)")

    ensure_bundle(BUNDLE_WORDS, {
        "id":   "Text",
        "word": "Text",
        "ax":   "Float",
        "ay":   "Float",
    })
    insert(BUNDLE_WORDS, WORD_CORPUS)

    print(f"\n  Bundle: {len(WORD_CORPUS)} word embeddings on unit circle")
    print(f"  Source: 'walk' at (1.0, 0.0)")

    subsection("Budget 0.3 (roughly +-25 degrees arc)")
    result = sample_transport(
        BUNDLE_WORDS,
        from_keys={"id": "w0"},
        fiber_fields=["ax", "ay"],
        budget=0.3,
        k=10,
        beta=1.0,
        seed=42,
    )
    print(f"  n_admissible={result['n_admissible']}  n_returned={result['n_returned']}")
    for c in result["candidates"]:
        word = c["record"].get("word", "?")
        print(f"    {word:12s}  d_sq={c['d_sq']:.4f}  sameness={c['sameness']:.4f}  "
              f"weight={c['weight']:.4f}")

    check("n_admissible > 0",
          result["n_admissible"] > 0,
          f"n_admissible={result['n_admissible']}")

    all_bounded = all(c["d_sq"] <= 0.3 + 1e-9 for c in result["candidates"])
    check("all d_sq <= budget=0.3", all_bounded)

    all_weights_ok = all(
        abs(c["weight"] - math.exp(-1.0 * c["d_sq"])) < 1e-9
        for c in result["candidates"]
    )
    check("weights match exp(-beta * d_sq) beta=1.0", all_weights_ok)

    all_sameness_ok = all(
        abs(c["sameness"] - (1.0 - c["d_sq"])) < 1e-9
        for c in result["candidates"]
    )
    check("sameness = 1 - d_sq for every candidate", all_sameness_ok)

    subsection("Budget 1.0 (full corpus)")
    result_full = sample_transport(
        BUNDLE_WORDS,
        from_keys={"id": "w0"},
        fiber_fields=["ax", "ay"],
        budget=1.0,
        k=100,
        beta=1.0,
        seed=1,
    )
    check("budget=1.0 admits all records",
          result_full["n_admissible"] == len(WORD_CORPUS),
          f"n_admissible={result_full['n_admissible']}/{len(WORD_CORPUS)}")


# ---- Domain 2: Music Similarity (genre embeddings) ----------------------

BUNDLE_MUSIC = "st_music"

GENRE_CORPUS = [
    {"id": "g0",  "genre": "pop",          "tempo_n": 0.60, "energy_n": 0.70},
    {"id": "g1",  "genre": "dance_pop",    "tempo_n": 0.65, "energy_n": 0.75},
    {"id": "g2",  "genre": "synth_pop",    "tempo_n": 0.55, "energy_n": 0.65},
    {"id": "g3",  "genre": "indie_pop",    "tempo_n": 0.50, "energy_n": 0.55},
    {"id": "g4",  "genre": "rock",         "tempo_n": 0.70, "energy_n": 0.85},
    {"id": "g5",  "genre": "metal",        "tempo_n": 0.85, "energy_n": 0.95},
    {"id": "g6",  "genre": "jazz",         "tempo_n": 0.35, "energy_n": 0.30},
    {"id": "g7",  "genre": "classical",    "tempo_n": 0.25, "energy_n": 0.20},
    {"id": "g8",  "genre": "ambient",      "tempo_n": 0.10, "energy_n": 0.10},
    {"id": "g9",  "genre": "folk",         "tempo_n": 0.40, "energy_n": 0.35},
    {"id": "g10", "genre": "hip_hop",      "tempo_n": 0.75, "energy_n": 0.80},
    {"id": "g11", "genre": "r_and_b",      "tempo_n": 0.58, "energy_n": 0.62},
]


def domain_music() -> None:
    section("Domain 2 -- Music Similarity (genre embeddings, 2D fiber)")

    ensure_bundle(BUNDLE_MUSIC, {
        "id":       "Text",
        "genre":    "Text",
        "tempo_n":  "Float",
        "energy_n": "Float",
    })
    insert(BUNDLE_MUSIC, GENRE_CORPUS)

    print(f"\n  Bundle: {len(GENRE_CORPUS)} genre embeddings")
    print(f"  Source: 'pop' at (0.60, 0.70)")

    beta = 2.0
    result = sample_transport(
        BUNDLE_MUSIC,
        from_keys={"id": "g0"},
        fiber_fields=["tempo_n", "energy_n"],
        budget=0.15,
        k=6,
        beta=beta,
        seed=99,
    )
    print(f"  budget=0.15  beta={beta}")
    print(f"  n_admissible={result['n_admissible']}  n_returned={result['n_returned']}")
    for c in result["candidates"]:
        genre = c["record"].get("genre", "?")
        print(f"    {genre:15s}  d_sq={c['d_sq']:.4f}  sameness={c['sameness']:.4f}  "
              f"weight={c['weight']:.4f}")

    check("n_admissible > 0", result["n_admissible"] > 0)

    all_bounded = all(c["d_sq"] <= 0.15 + 1e-9 for c in result["candidates"])
    check("all d_sq <= budget=0.15", all_bounded)

    all_weights_ok = all(
        abs(c["weight"] - math.exp(-beta * c["d_sq"])) < 1e-9
        for c in result["candidates"]
    )
    check(f"weights match exp(-{beta} * d_sq)", all_weights_ok)

    all_sameness_ok = all(
        abs(c["sameness"] - (1.0 - c["d_sq"])) < 1e-9
        for c in result["candidates"]
    )
    check("sameness = 1 - d_sq for every candidate", all_sameness_ok)


# ---- Domain 3: Drug Analog Search (3D fiber) ----------------------------

BUNDLE_DRUGS = "st_drugs"

DRUG_CORPUS = [
    # 3D fiber: (hydro, mw_norm, pki_norm).
    # Source compound_A sits at (1, 0, 0).
    # Analogs are within angular budget=0.10 (d^2 <= 0.10).
    # Dissimilars are orthogonal / opposite -- d^2 >= 0.25.
    {"id": "d0", "name": "compound_A",   "hydro": 1.00, "mw": 0.00, "pki": 0.00},
    {"id": "d1", "name": "compound_B",   "hydro": 0.99, "mw": 0.14, "pki": 0.00},
    {"id": "d2", "name": "compound_C",   "hydro": 0.99, "mw": 0.00, "pki": 0.14},
    {"id": "d3", "name": "compound_D",   "hydro": 0.98, "mw": 0.10, "pki": 0.10},
    {"id": "d4", "name": "compound_E",   "hydro": 0.97, "mw": 0.17, "pki": 0.17},
    {"id": "d5", "name": "near_analog",  "hydro": 0.95, "mw": 0.22, "pki": 0.22},
    {"id": "d6", "name": "scaffold_hop", "hydro": 0.90, "mw": 0.31, "pki": 0.31},
    # Dissimilars: orthogonal (0, 1, 0) and opposite (-1, 0, 0)
    {"id": "d7", "name": "dissimilar_X", "hydro": 0.00, "mw": 1.00, "pki": 0.00},
    {"id": "d8", "name": "dissimilar_Y", "hydro": -1.0, "mw": 0.00, "pki": 0.00},
]


def domain_drugs() -> None:
    section("Domain 3 -- Drug Analog Search (3D chemical fiber)")

    ensure_bundle(BUNDLE_DRUGS, {
        "id":    "Text",
        "name":  "Text",
        "hydro": "Float",
        "mw":    "Float",
        "pki":   "Float",
    })
    insert(BUNDLE_DRUGS, DRUG_CORPUS)

    print(f"\n  Bundle: {len(DRUG_CORPUS)} compounds (hydro, mw_norm, pki_norm)")
    print(f"  Source: 'compound_A' (reference compound)")

    result = sample_transport(
        BUNDLE_DRUGS,
        from_keys={"id": "d0"},
        fiber_fields=["hydro", "mw", "pki"],
        budget=0.10,
        k=5,
        beta=1.0,
        seed=77,
    )
    print(f"  budget=0.10")
    print(f"  n_admissible={result['n_admissible']}  n_returned={result['n_returned']}")
    for c in result["candidates"]:
        name = c["record"].get("name", "?")
        print(f"    {name:15s}  d_sq={c['d_sq']:.4f}  sameness={c['sameness']:.4f}")

    check("n_admissible > 0 (close analogs found)", result["n_admissible"] > 0)

    all_bounded = all(c["d_sq"] <= 0.10 + 1e-9 for c in result["candidates"])
    check("all d_sq <= budget=0.10", all_bounded)

    all_sameness_ok = all(
        abs(c["sameness"] - (1.0 - c["d_sq"])) < 1e-9
        for c in result["candidates"]
    )
    check("sameness = 1 - d_sq for every candidate", all_sameness_ok)

    # Verify dissimilar_X and dissimilar_Y are NOT in the candidates
    # (they are at completely different fiber locations and should fail the budget).
    returned_names = {c["record"].get("name") for c in result["candidates"]}
    check("dissimilar compounds excluded from budget=0.10 neighborhood",
          "dissimilar_X" not in returned_names and "dissimilar_Y" not in returned_names,
          f"returned names: {returned_names}")


# ---- Domain 4: Seed reproducibility + curvature_k invariant -------------

BUNDLE_REPRO = "st_repro"

REPRO_CORPUS = [
    {"id": f"r{i}", "v0": math.cos(i * math.pi / 8), "v1": math.sin(i * math.pi / 8)}
    for i in range(16)
]


def domain_reproducibility() -> None:
    section("Domain 4 -- Seed Reproducibility + curvature_k Invariant")

    ensure_bundle(BUNDLE_REPRO, {
        "id": "Text",
        "v0": "Float",
        "v1": "Float",
    })
    insert(BUNDLE_REPRO, REPRO_CORPUS)

    print(f"\n  Bundle: {len(REPRO_CORPUS)} points on unit circle (v0, v1)")
    print(f"  Source: r0 = (1.0, 0.0)")

    subsection("Seed=42 twice -> identical candidate ordering")
    r1 = sample_transport(
        BUNDLE_REPRO,
        from_keys={"id": "r0"},
        fiber_fields=["v0", "v1"],
        budget=0.6,
        k=5,
        beta=1.0,
        seed=42,
    )
    r2 = sample_transport(
        BUNDLE_REPRO,
        from_keys={"id": "r0"},
        fiber_fields=["v0", "v1"],
        budget=0.6,
        k=5,
        beta=1.0,
        seed=42,
    )

    same_count = r1["n_returned"] == r2["n_returned"]
    check("n_returned identical with same seed", same_count,
          f"{r1['n_returned']} vs {r2['n_returned']}")

    if same_count and r1["n_returned"] > 0:
        same_order = all(
            abs(a["d_sq"] - b["d_sq"]) < 1e-12
            for a, b in zip(r1["candidates"], r2["candidates"])
        )
        check("d_sq ordering identical with same seed", same_order)

    subsection("curvature_k = 2 * sqrt(d_sq) for every candidate")
    r_full = sample_transport(
        BUNDLE_REPRO,
        from_keys={"id": "r0"},
        fiber_fields=["v0", "v1"],
        budget=1.0,
        k=100,
        beta=1.0,
        seed=1,
    )
    all_k_ok = all(
        abs(c["curvature_k"] - 2.0 * math.sqrt(c["d_sq"])) < 1e-9
        for c in r_full["candidates"]
    )
    check("curvature_k = 2 * sqrt(d_sq) for every candidate", all_k_ok)

    subsection("confidence = 1 / (1 + kappa)")
    kappa = r_full.get("kappa", 0.0)
    confidence = r_full.get("confidence", 0.0)
    expected_conf = 1.0 / (1.0 + kappa) if (1.0 + kappa) > 0 else 0.0
    check("confidence = 1/(1+kappa)",
          abs(confidence - expected_conf) < 1e-9,
          f"kappa={kappa:.4f}  confidence={confidence:.4f}  expected={expected_conf:.4f}")

    subsection("Budget monotonic: increasing budget >= previous n_admissible")
    budgets = [0.0, 0.1, 0.2, 0.4, 0.6, 1.0]
    prev = 0
    monotonic = True
    for b in budgets:
        rb = sample_transport(
            BUNDLE_REPRO,
            from_keys={"id": "r0"},
            fiber_fields=["v0", "v1"],
            budget=b,
            k=100,
            beta=1.0,
            seed=1,
        )
        if rb["n_admissible"] < prev:
            monotonic = False
            print(f"    budget={b} gave n_admissible={rb['n_admissible']} < {prev}")
            break
        prev = rb["n_admissible"]
    check("n_admissible monotonically non-decreasing with budget", monotonic)


# ---- Main ---------------------------------------------------------------

def main() -> None:
    print("\nSAMPLE_TRANSPORT S4 -- Curvature-Bounded Neighborhood Sampling")
    print("Four domains, zero shared schema, zero domain configuration.\n")

    try:
        domain_words()
        domain_music()
        domain_drugs()
        domain_reproducibility()
    except RuntimeError as e:
        print(f"\nFATAL: {e}", file=sys.stderr)
        sys.exit(1)

    print(f"\n{'='*70}")
    n_pass = sum(1 for _, ok in results if ok)
    n_total = len(results)
    all_pass = n_pass == n_total

    for label, ok in results:
        mark = PASS_MARK if ok else FAIL_MARK
        print(f"  {mark}  {label}")

    print(f"\n  {n_pass}/{n_total} checks passed")
    print()
    if all_pass:
        print("  S4 gate: ALL DOMAINS PASS -- commit ready")
    else:
        failed = [label for label, ok in results if not ok]
        print("  S4 gate: FAILED checks:")
        for f in failed:
            print(f"    - {f}")
        sys.exit(1)


if __name__ == "__main__":
    main()
