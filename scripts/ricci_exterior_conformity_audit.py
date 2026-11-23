"""Exterior conformity audit: is the boundary-extension target one field?

Read-only diagnostic for the Ricci boundary-extension refusal of 2026-11-17.
Not a feature gate, not a repair, and it changes no evidence or source.

Question. The boundary extension tried to match the curvature of the original
outside cells along the patch boundary and refused (REFUSED_INCOMPATIBLE_BOUNDARY,
witness at global edge 613). Is that a property of this patch, or of the
original exterior itself?

Method. Reuse the verified per-cell evaluation from ricci_boundary_extension.py
verbatim (old_flux -> hodge -> unit_component along the ORIGINAL oriented global
edge) and apply it to every cell in the ring of every edge of the original
level1/n=2 Abelian mesh. No patch, no fit, no reconstruction.

Control, run first: reproduce the published edge-613 witness to 1e-12 or stop.

Mechanism-swap control: point the SAME pipeline at the Whitney one-form (the
potential) whose curl is the curvature under test. Whitney one-forms are
tangentially single-valued by construction, so the pipeline must find agreement
there. If it did not, the curvature result could be a pipeline artifact.

Also records observed convergence rates computed from published RF2D and RF2G
tables. Those numbers are transcribed from docs, not recomputed from flows.

Usage:  python scripts/ricci_exterior_conformity_audit.py
Writes output/ricci-exterior-conformity-20261117/receipt.json; refuses to
overwrite an existing receipt.
"""
import hashlib
import json
import sys
from collections import defaultdict
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
from ricci_boundary_extension import old_flux, unit_component      # noqa: E402
from ricci_push_interfaces import hodge                             # noqa: E402
from ricci_shared_curvature_patch import metric                     # noqa: E402
import ricci_variational_math as rvm                                # noqa: E402

EDGES, GRADS, VERTICES = rvm.EDGES, rvm.GRADS, rvm.VERTICES
GEOM = ROOT / "output/ricci-shape-regular-20261117/berger/level1_berger_q6.input.json"
DATA = ROOT / "output/ricci-consistency-20261117/diagnostic/level1_q6.arrays.npz"
OUT = ROOT / "output/ricci-exterior-conformity-20261117"
TOL = 1e-9
WITNESS = {"edge": 613, "t": 0.9662347571015759,
           "cells": {457: 0.24368429460572172, 461: 0.0759265385083391}}
PATCH = {0, 2, 3, 456, 458, 459}
NODES = np.r_[(np.polynomial.legendre.leggauss(6)[0] + 1) / 2, [.17, .37, .73]]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load():
    payload = json.loads(GEOM.read_text())
    with np.load(DATA, allow_pickle=False) as z:
        all_data = z["abelian_edge_integrals"].copy()
    return payload["cells"], all_data


def oriented(cell, edge, t):
    """Original global orientation of `edge` within `cell`, as in edge_audit."""
    li = cell["edges"].index(edge)
    u, v = EDGES[li]
    if cell["reverse"][li]:
        u, v = v, u
    delta = VERTICES[v] - VERTICES[u]
    return delta, VERTICES[u] + np.atleast_1d(t)[:, None] * delta


def curvature_component(cells, shim, ci, edge, t):
    cell = cells[ci]
    delta, x = oriented(cell, edge, t)
    points = np.array(cell["vertices"])
    hh = hodge(points, x, old_flux(shim, cell))[0]
    return unit_component(hh, delta, metric(points, x))


def barycentric(x):
    return np.array([(1.0 if i == 0 else 0.0) + (x - VERTICES[0]) @ GRADS[i]
                     for i in range(4)]).T


def coefficients(all_data, cell):
    return all_data[cell["edges"]] * np.where(cell["reverse"], -1., 1.)


def potential_component(cells, all_data, ci, edge, t):
    cell = cells[ci]
    delta, x = oriented(cell, edge, t)
    lam, c = barycentric(x), coefficients(all_data, cell)
    oneform = sum(c[k] * (lam[:, [i]] * GRADS[j] - lam[:, [j]] * GRADS[i])
                  for k, (i, j) in enumerate(EDGES))
    return unit_component(oneform, delta, metric(np.array(cell["vertices"]), x))


def spreads(component, by_edge):
    s, scale = [], []
    for edge, ring in sorted(by_edge.items()):
        vals = np.array([component(ci, edge, NODES) for ci in ring])
        s.append(float(np.ptp(vals, axis=0).max()))
        scale.append(float(np.abs(vals).max()))
    return np.array(s), np.array(scale)


def rates(errors, n=(1, 2, 3, 5)):
    n, e = np.array(n, float), np.array(errors, float)
    return (np.log(e[:-1] / e[1:]) / np.log(n[1:] / n[:-1])).tolist()


def main():
    receipt_path = OUT / "receipt.json"
    if receipt_path.exists():
        raise SystemExit(f"refusing to overwrite {receipt_path}")

    cells, all_data = load()
    shim = {"all_data": all_data}
    curv = lambda ci, e, t: curvature_component(cells, shim, ci, e, t)       # noqa: E731
    pot = lambda ci, e, t: potential_component(cells, all_data, ci, e, t)     # noqa: E731

    # control 1: published witness
    reproduced = {c: float(curv(c, WITNESS["edge"], WITNESS["t"])[0]) for c in WITNESS["cells"]}
    witness_error = max(abs(reproduced[c] - WITNESS["cells"][c]) for c in WITNESS["cells"])
    if witness_error > 1e-12:
        raise SystemExit(f"witness control failed: {witness_error:.3e}")

    # control 2: the potential's curl must be exactly the curvature under test
    curl_error = max(float(np.max(abs(
        sum(coefficients(all_data, c)[k] * 2 * np.cross(GRADS[i], GRADS[j])
            for k, (i, j) in enumerate(EDGES)) - old_flux(shim, c)))) for c in cells)
    if curl_error > 1e-12:
        raise SystemExit(f"potential/curvature identity failed: {curl_error:.3e}")

    by_edge = defaultdict(list)
    for ci, c in enumerate(cells):
        for e in c["edges"]:
            by_edge[e].append(ci)

    ring = by_edge[WITNESS["edge"]]
    ring_values = {ci: float(curv(ci, WITNESS["edge"], WITNESS["t"])[0]) for ci in ring}
    outside = [v for ci, v in ring_values.items() if ci not in PATCH]

    c_spread, c_scale = spreads(curv, by_edge)
    p_spread, _ = spreads(pot, by_edge)

    # mechanism-swap control must hold or the curvature result is not trusted
    if (p_spread > TOL).any():
        raise SystemExit("potential control failed: pipeline is not trustworthy")

    order = sorted(by_edge)
    receipt = {
        "kind": "read-only diagnostic; not a feature gate, repair, or flow result",
        "date": "2026-11-17",
        "inputs_sha256": {
            str(GEOM.relative_to(ROOT)): sha(GEOM),
            str(DATA.relative_to(ROOT)): sha(DATA),
        },
        "sources_sha256": {
            "scripts/ricci_exterior_conformity_audit.py": sha(__file__),
            "scripts/ricci_boundary_extension.py": sha(ROOT / "scripts/ricci_boundary_extension.py"),
            "scripts/ricci_push_interfaces.py": sha(ROOT / "scripts/ricci_push_interfaces.py"),
            "scripts/ricci_shared_curvature_patch.py": sha(ROOT / "scripts/ricci_shared_curvature_patch.py"),
            "e2e/probes/ricci_variational_math.py": sha(rvm.__file__),
        },
        "tolerance": TOL,
        "controls": {
            "witness_reproduction_error": witness_error,
            "witness_reproduced": {str(k): v for k, v in reproduced.items()},
            "potential_curl_equals_old_flux_max_error": curl_error,
        },
        "edge_613_ring": {
            "cells": {str(ci): {"value": v, "in_patch": ci in PATCH} for ci, v in ring_values.items()},
            "outside_cells_disagree_among_themselves_by": max(outside) - min(outside),
        },
        "curvature_mesh_wide": {
            "edges": len(order), "cells": len(cells), "points_per_edge": len(NODES),
            "edges_disagreeing_beyond_tol": int((c_spread > TOL).sum()),
            "spread_percentiles_0_25_50_75_100": np.percentile(c_spread, [0, 25, 50, 75, 100]).tolist(),
            "median_spread_over_field_scale": float(np.median(c_spread / np.maximum(c_scale, 1e-300))),
            "edge_613_spread": float(c_spread[order.index(WITNESS["edge"])]),
        },
        "potential_mesh_wide_control": {
            "edges_disagreeing_beyond_tol": int((p_spread > TOL).sum()),
            "max_spread": float(p_spread.max()),
        },
        "observed_rates_from_published_tables": {
            "note": "transcribed from docs/RICCI_OPERATOR_CONSISTENCY_RESULTS.md and "
                    "docs/RICCI_FINAL_BOUNDED_RESULTS.md; h proportional to 1/n",
            "RF2D_strong_flow_error": rates([29.8111, 25.9850, 25.8603, 26.1324]),
            "RF2D_rayleigh_error": rates([0.02172954, 0.00575878, 0.00258432, 0.00093488]),
            "RF2D_energy_density_error": rates([0.03282617, 0.00871547, 0.00391174, 0.00141513]),
            "RF2G_full_error": rates([33.4651864, 23.7021842, 21.5955404, 20.5839286]),
        },
    }
    OUT.mkdir(parents=True, exist_ok=False)
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({k: receipt[k] for k in
                      ("controls", "edge_613_ring", "curvature_mesh_wide",
                       "potential_mesh_wide_control")}, indent=2))
    print(f"\nwrote {receipt_path.relative_to(ROOT)}  sha256 {sha(receipt_path)}")


if __name__ == "__main__":
    main()
