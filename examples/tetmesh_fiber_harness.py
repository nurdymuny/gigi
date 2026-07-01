#!/usr/bin/env python3
"""Tetrahedral-mesh fiber-bundle harness for GIGI.

Companion to GIGI_TETMESH_SPEC_v0.6.md. Does two things:

1. Offline math validation (no server needed):
   - builds the Kuhn/Freudenthal triangulation of the unit cube
     (n^3 subcubes x 6 path simplices = 6n^3 tets; n=4 -> 384),
   - computes every fiber field the spec defines per tet: edge
     lengths, anchor edge vectors, edge-metric Gram, outward-face-
     normal angular Gram, dihedral vector, numeric PSD certificate,
     quality record (V, q_vol, min/max dihedral, d_bad),
   - checks the spec's identities numerically: rank-3 PSD angular
     Gram with positive kernel, facet-area closure sum(a_i n_i)=0,
     det G_edge = (6V)^2, one classifier cell modulo S4 on the
     structured mesh, Regge deficit kappa_e = 0 on interior edges,
   - runs naive uniform longest-edge bisection for LEVELS levels and
     prints the classifier-cell growth + min-q_vol descent telemetry
     (the spec's Experiment C instrument).

2. Live GIGI run (pass --server http://localhost:PORT):
   - creates a `tets` bundle (flat fiber schema; matrices ride as
     GIGI Vector fields), inserts every tet fiber,
   - creates a `tet_refine_events` bundle and inserts one typed event
     per bisection (parent, child, scheme, level, root),
   - runs the spec's queries in real GQL (COVER / SECTION AT /
     INTEGRATE) and prints the responses with their geometric
     ride-alongs.

Usage:
  python3 examples/tetmesh_fiber_harness.py            # math only
  python3 examples/tetmesh_fiber_harness.py --server http://localhost:3199
"""

import argparse
import itertools
import json
import sys
import urllib.request

import numpy as np

N = 4          # subdivisions per axis -> 6*N^3 tets
LEVELS = 3     # naive longest-edge bisection levels
EPS_MIN = 0.2  # sliver threshold on dihedrals (radians)
EPS_VOL = 0.1  # sliver threshold on q_vol
TOL = 1e-9

# ---------------------------------------------------------------- mesh

def kuhn_cube_mesh(n):
    """Kuhn/Freudenthal triangulation of [0,1]^3: 6*n^3 path simplices."""
    tets = []
    h = 1.0 / n
    for ix, iy, iz in itertools.product(range(n), repeat=3):
        base = np.array([ix, iy, iz], float) * h
        for perm in itertools.permutations(range(3)):
            v = [base.copy()]
            for axis in perm:
                p = v[-1].copy()
                p[axis] += h
                v.append(p)
            tets.append(np.array(v))
    return tets

# ---------------------------------------------------------------- fiber

def tet_fiber(verts):
    """Compute the full spec fiber record for one tet (4x3 vertex array)."""
    v = np.asarray(verts, float)
    # edges in a fixed order: (0,1),(0,2),(0,3),(1,2),(1,3),(2,3)
    pairs = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]
    edge_vecs = {p: v[p[1]] - v[p[0]] for p in pairs}
    lengths = np.array([np.linalg.norm(edge_vecs[p]) for p in pairs])

    # anchor edges from vertex 0
    E = np.array([v[1] - v[0], v[2] - v[0], v[3] - v[0]])
    G_edge = E @ E.T
    det_G = np.linalg.det(G_edge)
    vol = abs(np.linalg.det(E)) / 6.0

    # face i = face opposite vertex i; outward unit normal
    normals, areas = [], []
    for i in range(4):
        idx = [j for j in range(4) if j != i]
        a, b, c = v[idx[0]], v[idx[1]], v[idx[2]]
        nrm = np.cross(b - a, c - a)
        area = np.linalg.norm(nrm) / 2.0
        nrm = nrm / np.linalg.norm(nrm)
        if np.dot(nrm, v[i] - a) > 0:      # flip to point away from v_i
            nrm = -nrm
        normals.append(nrm)
        areas.append(area)
    normals = np.array(normals)
    areas = np.array(areas)

    # angular Gram A: A_ii = 1, A_ij = n_i . n_j = -cos(alpha_ij)
    A = normals @ normals.T
    np.fill_diagonal(A, 1.0)

    # dihedral along the edge shared by faces i,j (i<j): alpha = arccos(-n_i.n_j)
    dihedrals = {}
    for i in range(4):
        for j in range(i + 1, 4):
            dihedrals[(i, j)] = float(np.arccos(np.clip(-A[i, j], -1, 1)))
    alpha = np.array(sorted(dihedrals.values()))  # canonical (S4-invariant) order

    L = float(np.sqrt(np.mean(lengths ** 2)))
    q_vol = 6.0 * vol / L ** 3

    eigvals = np.linalg.eigvalsh(A)
    kernel = np.linalg.eigh(A)[1][:, 0]
    if kernel.sum() < 0:
        kernel = -kernel
    closure = np.linalg.norm((areas[:, None] * normals).sum(axis=0))

    d_bad = max(0.0, min(alpha.min() - EPS_MIN,
                         (np.pi - EPS_MIN) - alpha.max(),
                         q_vol - EPS_VOL))
    return dict(
        lengths=lengths, G_edge=G_edge, det_G=det_G, vol=vol,
        A=A, eigvals=eigvals, kernel=kernel, areas=areas,
        closure=closure, alpha=alpha, L=L, q_vol=q_vol, d_bad=d_bad,
    )

def classifier_key(alpha, digits=7):
    """S4-invariant proxy key: the sorted, rounded dihedral tuple."""
    return tuple(round(float(a), digits) for a in np.sort(alpha))

# ------------------------------------------------------- validation

def validate_mesh(tets):
    print(f"mesh: {len(tets)} tets (Kuhn n={N}; 6*n^3 = {6 * N ** 3})")
    keys, worst = set(), dict(psd=0.0, det0=0.0, closure=0.0, volid=0.0)
    for t in tets:
        f = tet_fiber(t)
        # PSD certificate, numeric mode
        assert f["eigvals"][0] > -1e-9, "angular Gram not PSD"
        assert (f["eigvals"] > 1e-9).sum() == 3, "numeric rank != 3"
        assert (f["kernel"] > 0).all(), "kernel vector not positive"
        # kernel is proportional to facet areas
        k_norm = f["kernel"] / f["kernel"].sum()
        a_norm = f["areas"] / f["areas"].sum()
        assert np.allclose(k_norm, a_norm, atol=1e-8), "kernel != facet areas"
        worst["psd"] = min(worst["psd"], f["eigvals"][0])
        worst["det0"] = max(worst["det0"], abs(f["eigvals"][0]))
        worst["closure"] = max(worst["closure"], f["closure"])
        worst["volid"] = max(worst["volid"], abs(f["det_G"] - (6 * f["vol"]) ** 2))
        keys.add(classifier_key(f["alpha"]))
    f0 = tet_fiber(tets[0])
    print(f"  PSD rank-3 + positive kernel:     all {len(tets)} pass "
          f"(min eig {worst['psd']:.1e}, |det-ish eig| <= {worst['det0']:.1e})")
    print(f"  facet-area closure |sum a_i n_i|: max {worst['closure']:.1e}")
    print(f"  det G_edge = (6V)^2:              max err {worst['volid']:.1e}")
    print(f"  distinct classifier cells mod S4: {len(keys)}  (expected 1)")
    print(f"  Kuhn tet dihedrals (rad):         {np.round(f0['alpha'], 6)}")
    print(f"  q_vol = {f0['q_vol']:.6f}, d_bad = {f0['d_bad']:.6f}")
    assert len(keys) == 1, "structured mesh must land in one classifier cell"
    return f0

def validate_regge(tets):
    """kappa_e = 2*pi - sum of dihedrals around interior edges == 0 (flat)."""
    star = {}
    for t in tets:
        f = tet_fiber(t)
        pairs = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]
        # dihedral along edge (a,b) is between the two faces NOT opposite a,b
        for (a, b) in pairs:
            faces = [i for i in range(4) if i not in (a, b)]
            i, j = faces
            n = tet_normals(t)
            ang = float(np.arccos(np.clip(-np.dot(n[i], n[j]), -1, 1)))
            key = tuple(sorted(map(tuple, np.round([t[a], t[b]], 9).tolist())))
            star.setdefault(key, 0.0)
            star[key] += ang
    interior = []
    for (p, q), total in star.items():
        on_bdry = any(p[k] == q[k] and p[k] in (0.0, 1.0) for k in range(3))
        if not on_bdry:
            interior.append(2 * np.pi - total)
    interior = np.array(interior)
    print(f"  Regge deficit on {len(interior)} interior edges: "
          f"max |kappa_e| = {np.abs(interior).max():.1e}  (expected 0: flat)")
    assert np.abs(interior).max() < 1e-8

def tet_normals(v):
    out = []
    for i in range(4):
        idx = [j for j in range(4) if j != i]
        a, b, c = v[idx[0]], v[idx[1]], v[idx[2]]
        n = np.cross(b - a, c - a)
        n = n / np.linalg.norm(n)
        if np.dot(n, v[i] - a) > 0:
            n = -n
        out.append(n)
    return out

# ------------------------------------------- naive longest-edge bisection

def bisect_longest(tet):
    pairs = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]
    lens = [np.linalg.norm(tet[b] - tet[a]) for a, b in pairs]
    a, b = pairs[int(np.argmax(lens))]
    m = (tet[a] + tet[b]) / 2.0
    child1, child2 = tet.copy(), tet.copy()
    child1[a] = m
    child2[b] = m
    return child1, child2

def refinement_telemetry(tets, levels):
    print(f"\nnaive uniform longest-edge bisection, {levels} levels "
          f"(Experiment C instrument):")
    current = list(tets)
    rows, events, snapshots = [], [], []
    ids = {id(t): f"t{i:05d}" for i, t in enumerate(current)}
    roots = {id(t): ids[id(t)] for t in current}
    counter = len(current)
    for lvl in range(levels + 1):
        if lvl > 0:
            snapshots.extend((t, lvl) for t in current)
        keys, qmin, dmin = set(), np.inf, np.inf
        for t in current:
            f = tet_fiber(t)
            keys.add(classifier_key(f["alpha"], digits=5))
            qmin, dmin = min(qmin, f["q_vol"]), min(dmin, f["d_bad"])
        rows.append((lvl, len(current), len(keys), qmin, dmin))
        print(f"  level {lvl}: {len(current):5d} tets, "
              f"{len(keys):3d} classifier cells, "
              f"min q_vol {qmin:.4f}, min d_bad {dmin:.4f}")
        if lvl == levels:
            break
        nxt = []
        for t in current:
            c1, c2 = bisect_longest(t)
            for c in (c1, c2):
                cid = f"t{counter:05d}"
                counter += 1
                ids[id(c)] = cid
                roots[id(c)] = roots[id(t)]
                events.append(dict(parent=ids[id(t)], child=cid,
                                   scheme="naive_leb", level=lvl + 1,
                                   root=roots[id(t)]))
                nxt.append(c)
        current = nxt
    return rows, snapshots, events, ids, roots

# ---------------------------------------------------------- GIGI client

def http(server, method, path, payload=None):
    req = urllib.request.Request(
        server + path,
        data=json.dumps(payload).encode() if payload is not None else None,
        headers={"Content-Type": "application/json"},
        method=method,
    )
    with urllib.request.urlopen(req) as r:
        return json.loads(r.read())

def gql(server, query):
    out = http(server, "POST", "/v1/gql", {"query": query})
    return out

def load_and_query(server, tets, refined, events, ids, roots):
    print(f"\nlive GIGI run against {server}")
    print("  health:", json.dumps(http(server, "GET", "/v1/health")))

    schema = {
        "name": "tets",
        "schema": {
            "fields": {
                "id": "text",
                "level": "float",
                "root": "text",
                "classifier_cell": "text",
                "min_dihedral": "float",
                "max_dihedral": "float",
                "q_vol": "float",
                "d_bad": "float",
                "volume": "float",
                "dihedrals": "vector(6)",
                "edge_lengths": "vector(6)",
                "face_normal_gram": "vector(16)",
                "edge_metric_gram": "vector(9)",
                "psd_mode": "text",
                "psd_numeric_rank": "float",
                "psd_min_eig": "float",
            },
            "keys": ["id"],
            "indexed": ["classifier_cell", "level", "root"],
        },
    }
    for b in ("tets", "tet_refine_events"):
        try:
            http(server, "DELETE", f"/v1/bundles/{b}")
        except Exception:
            pass
    http(server, "POST", "/v1/bundles", schema)
    http(server, "POST", "/v1/bundles", {
        "name": "tet_refine_events",
        "schema": {
            "fields": {"event_id": "text", "parent": "text", "child": "text",
                       "scheme": "text", "level": "float", "root": "text"},
            "keys": ["event_id"],
            "indexed": ["scheme", "level", "root"],
        },
    })

    # canonical classifier-cell registry (proxy: one label per distinct key)
    cells = {}

    def record(tet, tid, lvl, root):
        f = tet_fiber(tet)
        key = classifier_key(f["alpha"], digits=5)
        label = cells.setdefault(key, f"C{len(cells)}")
        return {
            "id": tid, "level": float(lvl), "root": root,
            "classifier_cell": label,
            "min_dihedral": float(f["alpha"].min()),
            "max_dihedral": float(f["alpha"].max()),
            "q_vol": float(f["q_vol"]), "d_bad": float(f["d_bad"]),
            "volume": float(f["vol"]),
            "dihedrals": [float(x) for x in f["alpha"]],
            "edge_lengths": [float(x) for x in f["lengths"]],
            "face_normal_gram": [float(x) for x in f["A"].ravel()],
            "edge_metric_gram": [float(x) for x in f["G_edge"].ravel()],
            "psd_mode": "numeric",
            "psd_numeric_rank": float((f["eigvals"] > 1e-9).sum()),
            "psd_min_eig": float(f["eigvals"][0]),
        }

    recs = [record(t, ids[id(t)], 0, roots[id(t)]) for t in tets]
    recs += [record(t, ids[id(t)], lvl, roots[id(t)])
             for t, lvl in refined]
    for i in range(0, len(recs), 500):
        http(server, "POST", "/v1/bundles/tets/insert",
             {"records": recs[i:i + 500]})
    ev = [dict(e, event_id=f"e{i:06d}", level=float(e["level"]))
          for i, e in enumerate(events)]
    for i in range(0, len(ev), 500):
        http(server, "POST", "/v1/bundles/tet_refine_events/insert",
             {"records": ev[i:i + 500]})
    print(f"  inserted {len(recs)} tet fibers + {len(ev)} refine events")

    queries = [
        # spec query 1: slivers by distance-to-badness
        "COVER tets WHERE d_bad < 0.05 FIRST 3;",
        # spec query 2: all tets in one classifier cell (indexed -> bitmap)
        "COVER tets ON classifier_cell = 'C0' FIRST 2;",
        # point query on the fiber: O(1)
        "SECTION tets AT id = 't00000';",
        # spec query 3: reachable classifier state per refinement level.
        # count(*) and multi-field min() in one statement exercise the
        # 2026-07-01 INTEGRATE fixes (group_by_measures): previously
        # count(*) silently returned empty and every min() aliased the
        # first field's value.
        "INTEGRATE tets OVER classifier_cell MEASURE count(*), "
        "min(min_dihedral), min(q_vol);",
        # spec query 4: shape-quality descent curve across levels
        "INTEGRATE tets OVER level MEASURE count(*), min(q_vol), "
        "min(d_bad);",
        # global aggregation (no OVER) — also part of the same fix
        "INTEGRATE tets MEASURE count(*), min(q_vol), max(max_dihedral);",
        # audit trail: events per scheme/level
        "INTEGRATE tet_refine_events OVER level MEASURE count(*);",
        # geometric ride-alongs on the bundle itself
        "HEALTH tets;",
    ]
    for q in queries:
        out = gql(server, q)
        blob = json.dumps(out)
        print(f"\n  GQL> {q}")
        print("  " + (blob[:600] + (" ..." if len(blob) > 600 else "")))

# --------------------------------------------------------------- main

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--server", help="GIGI base URL, e.g. http://localhost:3199")
    args = ap.parse_args()

    tets = kuhn_cube_mesh(N)
    validate_mesh(tets)
    validate_regge(tets)
    rows, snapshots, events, ids, roots = refinement_telemetry(tets, LEVELS)

    if args.server:
        load_and_query(args.server, tets, snapshots, events, ids, roots)
    else:
        print("\n(no --server given; skipped the live GIGI run)")

if __name__ == "__main__":
    main()
