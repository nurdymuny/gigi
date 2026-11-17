"""One frozen Abelian case, two adjoint readouts, no flow repair."""
import json
from pathlib import Path
import subprocess
import sys
import time

import numpy as np
from scipy.sparse import coo_matrix, load_npz

from ricci_visual_review import ROOT, NATIVE, OLD, RECOVERED, field, read, section, sha, slice_cells
from ricci_local_push_math import contributions, check_closure
from ricci_consistency_math import INCIDENCE, cell_integrals, link_quaternions
from ricci_variational_math import EDGES, FACES, cell_masses, solve
from ricci_variational_mesh import deck, topology
from ricci_lattice_mesh import plus

OUT = ROOT / "output/ricci-local-push-20261117"
KEY = "level1_q6"


def relative_check(actual, expected, name):
    error = float(np.linalg.norm(actual - expected) / max(np.linalg.norm(expected), 1e-30))
    if not np.isfinite(error) or error > 1e-10:
        raise ValueError(name + " failed: " + str(error))
    return error


def star(payload, state, edge):
    topo = topology(payload)
    elements, _, products, inverses, _ = deck()
    seen = set()
    pending = [state]
    while pending:
        ci, g = pending.pop()
        if (ci, g) in seen:
            continue
        seen.add((ci, g))
        if len(seen) > 12:
            raise ValueError("patch exceeds declared twelve-cell bound")
        cell = payload["cells"][ci]
        a, b = EDGES[cell["edges"].index(edge)]
        for fi_local, face in enumerate(FACES):
            if a in face and b in face:
                fi, own = topo["cell_faces"][ci][fi_local]
                image = products[g, own]
                for neighbor, _, other in topo["owners"][fi]:
                    pending.append((neighbor, int(products[image, inverses[other]])))
    states = sorted(seen)
    ids = [ci for ci, _ in states]
    expected = [ci for ci, cell in enumerate(payload["cells"]) if edge in cell["edges"]]
    if len(set(ids)) != len(ids) or sorted(ids) != expected:
        raise ValueError("edge star not completely represented")
    points = np.array([plus(elements[g], np.array(payload["cells"][ci]["vertices"]))
                       for ci, g in states])
    return states, points


def compute():
    OUT.mkdir(parents=True, exist_ok=True)
    if (OUT / "receipt.json").exists():
        raise ValueError("preserve prior audit")
    start = time.perf_counter()
    inputs = {}
    for directory in (OLD, RECOVERED):
        diagnostic = directory / "diagnostic.json"
        verified = directory / "VERIFIED_DIAGNOSTIC.json"
        record = read(verified)
        if not record["evidence_valid"] or sha(diagnostic) != record["diagnostic_sha256"]:
            raise ValueError("unverified source diagnostic")
        inputs[str(diagnostic)] = sha(diagnostic)
        inputs[str(verified)] = sha(verified)
    old_case = read(OLD / "diagnostic.json")["cases"][KEY]
    new_case = read(RECOVERED / "diagnostic.json")["cases"][KEY]
    source = NATIVE / "level1_berger_q6.input.json"
    sources = {
        source: read(NATIVE / "diagnostic.json")["cases"]["level1_berger_q6"]["input_sha256"],
        OLD / (KEY + ".arrays.npz"): old_case["arrays_sha256"],
        RECOVERED / (KEY + ".arrays.npz"): new_case["artifacts"][KEY + ".arrays.npz"],
        RECOVERED / (KEY + ".m.npz"): new_case["artifacts"][KEY + ".m.npz"],
    }
    for path, expected in sources.items():
        if sha(path) != expected:
            raise ValueError("source hash drift: " + str(path))
        inputs[str(path)] = expected
    payload = read(source)
    assert len(payload["cells"]) == 960 and len(payload["edge_points"]) == 1124
    with np.load(OLD / (KEY + ".arrays.npz"), allow_pickle=False) as data:
        a = data["abelian_edge_integrals"].copy()
        saved_velocity = data["abelian_velocity"].copy()
    with np.load(RECOVERED / (KEY + ".arrays.npz"), allow_pickle=False) as data:
        saved_load = data["load"][:, 0].copy()
        np.testing.assert_allclose(a, data["a"][:, 0], atol=1e-13, rtol=0)
    mass = load_npz(RECOVERED / (KEY + ".m.npz"))
    chart_source = NATIVE / "level0_berger_q6.input.json"
    if sha(chart_source) != read(NATIVE / "diagnostic.json")["cases"]["level0_berger_q6"]["input_sha256"]:
        raise ValueError("chart source drift")
    inputs[str(chart_source)] = sha(chart_source)
    chart = np.array(read(chart_source)["cells"][0]["vertices"])
    cells, _ = slice_cells(payload, chart)
    xy = np.array([[0.27, 0.23]])
    point = np.array([0.27, 0.27, 0.23, 0.23]) @ chart
    point /= np.linalg.norm(point)
    observed, owners, _ = section(payload, cells, chart, [saved_velocity, -4*a], xy)
    state, points, _ = cells[int(owners[0])]
    central = payload["cells"][state[0]]
    edge = central["edges"][0]
    patch, patch_points = star(payload, state, edge)
    print("fixed patch", state, "edge", edge, "cells", patch, flush=True)

    # A readout is linear in the original edge unknowns; the mesh is unchanged.
    readout = np.zeros((len(a), 2))
    for e in central["edges"]:
        basis = np.zeros(len(a)); basis[e] = 1
        readout[e] = field(points, central, basis, point[None, :])[0, 1:]
    np.testing.assert_allclose(readout.T @ saved_velocity, observed[0, 0, 1:], atol=1e-13, rtol=0)
    adjoints = []
    solves = []
    for component in range(2):
        z, receipt = solve(mass, readout[:, component])
        adjoints.append(z); solves.append(receipt)
    adjoints = np.column_stack(adjoints)

    ids = np.array([c["edges"] for c in payload["cells"]])
    signs = np.where(np.array([c["reverse"] for c in payload["cells"]]), -1., 1.)
    weights = np.array([c["weight"] for c in payload["cells"]])
    force = np.zeros((960, 6)); continuum = np.zeros_like(force); reference = np.zeros_like(force)
    rows = []; cols = []; values = []
    for begin in range(0, 960, 64):
        stop = min(begin + 64, 960)
        vertices = np.array([c["vertices"] for c in payload["cells"][begin:stop]])
        links = link_quaternions([1., 1., 1.], vertices[:, [i for i,j in EDGES]],
                                 vertices[:, [j for i,j in EDGES]])
        local = cell_integrals(vertices, links, [1., 1., 1.], 6, velocity=np.zeros((3, 3)))
        _, independent_mass, _, weak = cell_masses(vertices, 6)
        relative_check(local["edge_mass"], independent_mass, "independent local mass")
        at = a[ids[begin:stop]] * signs[begin:stop]
        flux = at @ INCIDENCE.T
        force[begin:stop] = -np.einsum("fe,cfg,cg->ce", INCIDENCE, local["face_mass"], flux)
        continuum[begin:stop] = weak[:, :, 0]
        reference[begin:stop] = -4*np.einsum("cef,cf->ce", local["edge_mass"], at)
        factor = weights[begin:stop, None, None] * signs[begin:stop, :, None] * signs[begin:stop, None, :]
        values.append((local["edge_mass"] * factor).ravel())
        rows.append(np.broadcast_to(ids[begin:stop, :, None], local["edge_mass"].shape).ravel())
        cols.append(np.broadcast_to(ids[begin:stop, None, :], local["edge_mass"].shape).ravel())
    force *= weights[:, None]; continuum *= weights[:, None]; reference *= weights[:, None]
    assembled_mass = coo_matrix((np.concatenate(values), (np.concatenate(rows), np.concatenate(cols))),
                               shape=mass.shape).tocsr()
    mass_difference = assembled_mass - mass
    checks = {}
    checks["mass_relative_difference"] = float(np.linalg.norm(mass_difference.data) / np.linalg.norm(mass.data))
    if not np.isfinite(checks["mass_relative_difference"]) or checks["mass_relative_difference"] > 1e-10:
        raise ValueError("cached mass mismatch")
    def assembled(local):
        result = np.zeros(len(a))
        np.add.at(result, ids.ravel(), (local * signs).ravel())
        return result
    checks["force_vs_saved_velocity"] = relative_check(assembled(force), mass @ saved_velocity, "saved force")
    checks["continuum_load_replay"] = relative_check(assembled(continuum), saved_load, "continuum load")
    checks["reference_load_replay"] = relative_check(assembled(reference), mass @ (-4*a), "reference load")
    actual_parts = contributions(ids, signs, force, adjoints)
    smooth_parts = contributions(ids, signs, continuum, adjoints)
    reference_parts = contributions(ids, signs, reference, adjoints)
    error_parts = actual_parts - reference_parts
    action_parts = actual_parts - smooth_parts
    interpolation_parts = smooth_parts - reference_parts
    expected = observed[0, 0, 1:] - observed[1, 0, 1:]
    checks["error_closure"] = check_closure(error_parts, expected)
    checks["actual_closure"] = check_closure(actual_parts, observed[0, 0, 1:])
    checks["reference_closure"] = check_closure(reference_parts, observed[1, 0, 1:])
    checks["split_closure"] = check_closure(action_parts + interpolation_parts, expected)
    attacks = {}
    for name, broken in (
            ("omit_largest_cell", np.delete(error_parts, int(np.argmax(np.linalg.norm(error_parts, axis=1))), axis=0)),
            ("remove_orientation", contributions(ids, np.ones_like(signs), force-reference, adjoints))):
        try:
            check_closure(broken, expected)
        except ValueError as error:
            attacks[name] = str(error)
        else:
            raise ValueError("bad evidence accepted: " + name)
    patch_ids = np.array([ci for ci, _ in patch])
    outside = np.ones(960, dtype=bool); outside[patch_ids] = False
    arrays = OUT / "attributions.npz"
    np.savez_compressed(arrays, readout=readout, adjoints=adjoints, ids=ids, signs=signs,
        force=force, continuum=continuum, reference=reference, actual=actual_parts,
        smooth=smooth_parts, expected=reference_parts, error=error_parts, action=action_parts,
        interpolation=interpolation_parts, patch_ids=patch_ids, patch_points=patch_points,
        observed=observed, chart=chart, point=point, selected_edge_points=points[:2])
    for path, expected_sha in inputs.items():
        if sha(Path(path)) != expected_sha:
            raise ValueError("source drift during bounded audit")
    source_paths = [Path(__file__), ROOT/"scripts/ricci_local_push_math.py",
        ROOT/"scripts/test_ricci_local_push_math.py", ROOT/"scripts/ricci_visual_review.py",
        ROOT/"docs/RICCI_LOCAL_PUSH_AUDIT.md", ROOT/"e2e/probes/ricci_consistency_math.py",
        ROOT/"e2e/probes/ricci_variational_math.py", ROOT/"e2e/probes/ricci_variational_mesh.py"]
    receipt = {
        "status": "ACCOUNTING_VERIFIED_NOT_A_REPAIR", "case": KEY,
        "scope": "one fixed point, original Abelian control only", "adjoint_solves": solves,
        "flow_solves": 0, "full_suite": "NOT_RUN", "gate_status": "UNCHANGED_OPEN",
        "inputs": inputs, "sources": {str(p): sha(p) for p in source_paths},
        "checks": checks, "rejected_bad_evidence": attacks,
        "patch": {"central_cell": state[0], "edge": edge, "cells": patch_ids.tolist(),
                  "selection": "midpoint of prior crossing, local edge0, complete edge star"},
        "observed_sideways": observed[0, 0, 1:].tolist(),
        "reference_sideways": observed[1, 0, 1:].tolist(),
        "sideways_error": expected.tolist(),
        "action_error": action_parts.sum(axis=0).tolist(),
        "interpolation_error": interpolation_parts.sum(axis=0).tolist(),
        "patch_error": error_parts[patch_ids].sum(axis=0).tolist(),
        "outside_error": error_parts[outside].sum(axis=0).tolist(),
        "per_patch_cell": [{"cell": int(ci), "actual": actual_parts[ci].tolist(),
            "smooth": smooth_parts[ci].tolist(), "error": error_parts[ci].tolist(),
            "action_error": action_parts[ci].tolist(), "interpolation_error": interpolation_parts[ci].tolist()}
            for ci in patch_ids],
        "arrays_sha256": sha(arrays), "elapsed_seconds": time.perf_counter()-start,
    }
    (OUT/"receipt.json").write_text(json.dumps(receipt, indent=2)+"\n", encoding="utf-8")
    print(json.dumps({k: receipt[k] for k in ["checks", "sideways_error", "action_error", "interpolation_error",
                     "patch_error", "outside_error", "per_patch_cell", "elapsed_seconds"]}, indent=2))


if __name__ == "__main__":
    if "--worker" in sys.argv:
        compute()
    else:
        subprocess.run([sys.executable, __file__, "--worker"], check=True, timeout=180)
