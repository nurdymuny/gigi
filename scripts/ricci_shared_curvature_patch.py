"""Bounded six-cell construction; not a global flow operator."""
from pathlib import Path
import json
import argparse
import subprocess
import sys
import numpy as np
from ricci_local_push_audit import OUT as PRIOR, NATIVE, OLD, KEY, read, sha
from ricci_push_interfaces import quadrature, hodge, test_field
from ricci_variational_math import EDGES, FACES, GRADS, VERTICES
from ricci_shared_curvature_math import (TOL, RCOND, closed_basis, divergence, evaluate,
    potential, curl, edge_integrals, face_fluxes, minimum_energy, energy_gradient)
from ricci_shared_face_math import trace_report

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT/"output/ricci-shared-curvature-20261117"


def geometry():
    prior = read(PRIOR/"receipt.json")
    inputs = prior["inputs"] | prior["sources"]
    for name, expected in inputs.items():
        if sha(Path(name)) != expected:
            raise ValueError("frozen evidence/source drift: "+name)
    arrays = PRIOR/"attributions.npz"
    if sha(arrays) != prior["arrays_sha256"]:
        raise ValueError("frozen patch drift")
    inputs[str(arrays)] = sha(arrays)
    inputs[str(PRIOR/"receipt.json")] = sha(PRIOR/"receipt.json")
    payload = read(NATIVE/"level1_berger_q6.input.json")
    with np.load(arrays, allow_pickle=False) as saved:
        cell_ids = saved["patch_ids"].tolist()
        points = saved["patch_points"].copy(); chart = saved["chart"].copy()
    if cell_ids != [0, 2, 3, 456, 458, 459]:
        raise ValueError("same six-cell patch required")
    if np.any(np.linalg.det(points) <= 0):
        raise ValueError("positive cover orientations required")
    vertices, cell_vertices = [], []
    for tetrahedron in points:
        ids = []
        for vertex in tetrahedron:
            matches = [i for i, v in enumerate(vertices) if np.max(abs(v-vertex)) < 2e-12]
            if not matches:
                matches = [len(vertices)]; vertices.append(vertex)
            if len(matches) != 1:
                raise ValueError("ambiguous patch vertex")
            ids.append(matches[0])
        cell_vertices.append(ids)
    ambient = np.array(vertices)
    homogeneous = ambient@np.linalg.inv(chart.T).T
    if np.min(homogeneous.sum(axis=1)) <= 0:
        raise ValueError("patch outside fixed gnomonic chart")
    unscaled = homogeneous[:, 1:]/homogeneous.sum(axis=1)[:, None]
    center = unscaled.mean(axis=0); scale = np.linalg.norm(unscaled-center, axis=1).max()
    vertices = (unscaled-center)/scale
    edge_pairs, face_owners = {}, {}
    for ti, ci in enumerate(cell_ids):
        cell, ids = payload["cells"][ci], cell_vertices[ti]
        for ei, (u, v) in enumerate(EDGES):
            pair = (ids[v], ids[u]) if cell["reverse"][ei] else (ids[u], ids[v])
            edge = cell["edges"][ei]
            if edge in edge_pairs and edge_pairs[edge] != pair:
                raise ValueError("global signed edge endpoints disagree")
            edge_pairs[edge] = pair
        for fi, face in enumerate(FACES):
            face_owners.setdefault(tuple(sorted(ids[i] for i in face)), []).append((ti, fi))
    edge_ids = sorted(edge_pairs); pairs = [edge_pairs[e] for e in edge_ids]
    faces = sorted(face_owners)
    g = np.zeros((len(pairs), len(vertices)))
    d = np.zeros((len(faces), len(pairs)))
    boundary = np.zeros((6, len(faces)))
    edge_lookup = {frozenset(pair): ei for ei, pair in enumerate(pairs)}
    for ei, (u, v) in enumerate(pairs):
        g[ei, u] = -1; g[ei, v] = 1
    for fi, (a, b, c) in enumerate(faces):
        for u, v in [(a, b), (b, c), (c, a)]:
            ei = edge_lookup[frozenset((u, v))]
            d[fi, ei] = 1 if pairs[ei] == (u, v) else -1
        triangle = vertices[[a, b, c]]
        normal = np.cross(triangle[1]-triangle[0], triangle[2]-triangle[0])
        for ti, _ in face_owners[(a, b, c)]:
            opposite = next(v for v in cell_vertices[ti] if v not in (a, b, c))
            boundary[ti, fi] = -np.sign(normal@(vertices[opposite]-triangle[0]))
    if np.any(d@g) or np.any(boundary@d):
        raise ValueError("boundary-of-boundary must vanish exactly")
    if (len(vertices), len(pairs), len(faces)) != (8, 19, 18):
        raise ValueError("fixed patch combinatorics changed")
    ranks = [np.linalg.matrix_rank(matrix) for matrix in (g, d, boundary)]
    if ranks != [7, 12, 6]:
        raise ValueError("patch exactness/rank contract failed")
    with np.load(OLD/(KEY+".arrays.npz"), allow_pickle=False) as saved:
        all_data = saved["abelian_edge_integrals"].copy()
    return {"inputs": inputs, "payload": payload, "cell_ids": cell_ids,
        "points": points, "chart": chart, "center": center, "scale": scale,
        "ambient": ambient, "vertices": vertices, "cell_vertices": cell_vertices,
        "edge_ids": edge_ids, "pairs": pairs, "faces": faces, "owners": face_owners,
        "G": g, "D": d, "B": boundary, "data": all_data[edge_ids], "all_data": all_data}


def rank_audit(patch):
    target = patch["D"]
    target_rank = np.linalg.matrix_rank(target)
    reports, chosen = [], None
    for degree in range(3):
        basis = closed_basis(degree)
        q = flux_matrix(patch, degree, basis)
        u, s, _ = np.linalg.svd(q, full_matrices=False)
        rank = int(np.count_nonzero(s > s[0]*RCOND))
        residual = float(np.max(abs(target-u[:, :rank]@(u[:, :rank].T@target))))
        closure = float(np.max(abs(patch["B"]@q)))
        reports.append({"degree": degree, "closed_dimensions": basis.shape[1], "rank": rank,
            "singular_values": s.tolist(), "closed_space_residual": residual,
            "cell_flux_closure_residual": closure})
        if rank == target_rank and residual <= TOL and closure <= TOL and chosen is None:
            chosen = degree
    return {"target_rank": int(target_rank), "degrees": reports, "chosen_degree": chosen,
            "status": "REPRESENTABLE" if chosen is not None else "REFUSED_NO_FULL_CLOSED_FLUX_SPACE"}


def flux_matrix(patch, degree, basis):
    triangles = patch["vertices"][np.array(patch["faces"])]
    return np.column_stack([face_fluxes(column, degree, triangles) for column in basis.T])


def construct(patch, degree, data):
    basis = closed_basis(degree)
    q = flux_matrix(patch, degree, basis)
    mass = physical_mass(patch, degree, basis)
    fit = minimum_energy(mass, q, patch["D"]@data)
    polynomial = basis@fit["coefficients"]
    connection = potential(polynomial, degree)
    endpoints = patch["vertices"][np.array(patch["pairs"])]
    original_integrals = edge_integrals(connection, degree+1, endpoints)
    gauge = recover_gauge(patch, data-original_integrals)
    integrals = original_integrals+patch["G"]@gauge
    return {**fit, "degree": degree, "mass": mass, "basis": basis, "flux_map": q,
        "edge_map": fit["map"]@patch["D"], "polynomial": polynomial,
        "potential": connection, "gauge": gauge, "edge_integrals": integrals,
        "fluxes": q@fit["coefficients"]}


def metric(points, coordinates):
    b = (points[1:]-points[0]).T
    y = points[0]+coordinates@b.T
    radius = np.linalg.norm(y, axis=1); unit = y/radius[:, None]
    dx = (b[None]-unit[:, :, None]*np.einsum("qi,ia->qa", unit, b)[:, None, :])/radius[:, None, None]
    result = np.einsum("qia,qib->qab", dx, dx)
    if not np.isfinite(result).all() or np.any(np.linalg.det(result) <= 0):
        raise ValueError("positive original radial metric required")
    return result


def common_coordinates(patch, ti, coordinates):
    # Cell barycentric coordinates and the fixed gnomonic chart have a
    # projective, not affine, transition. Use its full position-dependent Jacobian.
    points = patch["points"][ti]
    inverse = np.linalg.inv(patch["chart"].T)
    first = inverse@points[0]
    derivative = inverse@(points[1:]-points[0]).T
    homogeneous = first+coordinates@derivative.T
    denominator = homogeneous.sum(axis=1)
    if np.any(denominator <= 0):
        raise ValueError("cell outside fixed chart")
    unscaled = homogeneous[:, 1:]/denominator[:, None]
    jacobian = (derivative[1:][None]-unscaled[:, :, None]*derivative.sum(axis=0)[None, None, :])
    jacobian /= denominator[:, None, None]*patch["scale"]
    if np.any(np.linalg.det(jacobian) <= 0):
        raise ValueError("orientation-preserving coordinate transition required")
    return (unscaled-patch["center"])/patch["scale"], jacobian


def two_form_pullback(values, jacobian):
    return np.einsum("qij,qnj->qni", np.linalg.inv(jacobian), values)*np.linalg.det(jacobian)[:, None, None]


def physical_mass(patch, degree, basis):
    positions, weights = quadrature(6, 3)
    mass = np.zeros((basis.shape[1], basis.shape[1]))
    for ti, ci in enumerate(patch["cell_ids"]):
        y, jacobian = common_coordinates(patch, ti, positions)
        values = np.stack([evaluate(c, degree, y) for c in basis.T], axis=1)
        flux = two_form_pullback(values, jacobian)
        g = metric(patch["points"][ti], positions)
        weight = patch["payload"]["cells"][ci]["weight"]
        if weight != 1:
            raise ValueError("fixed patch cell weight changed")
        mass += np.einsum("qni,qij,qmj,q->nm", flux, g, flux, weights/np.sqrt(np.linalg.det(g)))
    return mass


def recover_gauge(patch, remainder):
    if np.max(abs(patch["D"]@remainder)) > TOL:
        raise ValueError("edge correction is not closed")
    values = np.full(len(patch["vertices"]), np.nan); values[0] = 0.
    for _ in range(len(values)):
        for ei, (tail, head) in enumerate(patch["pairs"]):
            if np.isfinite(values[tail]) and not np.isfinite(values[head]):
                values[head] = values[tail]+remainder[ei]
            elif np.isfinite(values[head]) and not np.isfinite(values[tail]):
                values[tail] = values[head]-remainder[ei]
    if not np.isfinite(values).all() or np.max(abs(patch["G"]@values-remainder)) > TOL:
        raise ValueError("edge correction is not an exact vertex gradient")
    return values


def owner_field(patch, ti, coordinates, polynomial, degree):
    y, jacobian = common_coordinates(patch, ti, coordinates)
    values = evaluate(polynomial, degree, y)[:, None, :]
    flux = two_form_pullback(values, jacobian)[:, 0, :]
    g = metric(patch["points"][ti], coordinates)
    root = np.sqrt(np.linalg.det(g))
    h = np.einsum("qij,qj->qi", g, flux)/root[:, None]
    inverse_flux = np.linalg.solve(g, h[..., None])[..., 0]*root[:, None]
    return h, float(np.max(abs(inverse_flux-flux)))


def diagnostics(patch, construction, data):
    c = construction; degree = c["degree"]
    polynomial = c["polynomial"]
    endpoints = patch["vertices"][np.array(patch["pairs"])]
    circulation = patch["D"]@edge_integrals(c["potential"], degree+1, endpoints)
    triangles = patch["vertices"][np.array(patch["faces"])]
    flux = face_fluxes(polynomial, degree, triangles)
    gradient = energy_gradient(c["edge_map"], c["mass"], c["coefficients"])
    numerical = []
    for ei in range(len(data)):
        step = np.zeros(len(data)); step[ei] = 1e-5
        plus = minimum_energy(c["mass"], c["flux_map"], patch["D"]@(data+step))["energy"]
        minus = minimum_energy(c["mass"], c["flux_map"], patch["D"]@(data-step))["energy"]
        numerical.append((plus-minus)/2e-5)
    derivative_error = float(np.max(abs(gradient-numerical)/np.maximum(1., abs(gradient))))
    stiffness = c["edge_map"].T@c["mass"]@c["edge_map"]
    gauge_curl = 0.
    for vertices in patch["cell_vertices"]:
        xyz = patch["vertices"][vertices]; values = c["gauge"][vertices]
        local_gradient = np.linalg.solve(xyz[1:]-xyz[0], values[1:]-values[0])
        local_edges = np.array([local_gradient@(xyz[j]-xyz[i]) for i, j in EDGES])
        local_curl = local_edges@np.array([2*np.cross(GRADS[i], GRADS[j]) for i, j in EDGES])
        gauge_curl = max(gauge_curl, float(np.max(abs(local_curl))))
    face_results, exterior, constitutive = face_diagnostics(patch, c)
    interior, _ = quadrature(6, 3)
    for ti in range(6):
        _, error = owner_field(patch, ti, interior, polynomial, degree)
        constitutive = max(constitutive, error)
    from scipy.linalg import null_space
    null = null_space(c["flux_map"], rcond=RCOND)
    stationarity = float(np.max(abs(null.T@c["mass"]@c["coefficients"]), initial=0))
    return {"edge_error": float(np.max(abs(c["edge_integrals"]-data))),
        "flux_error": float(np.max(abs(flux-patch["D"]@data))),
        "closure_error": float(np.max(abs(divergence(degree)@polynomial), initial=0)),
        "curl_error": float(np.max(abs(curl(c["potential"], degree+1)-polynomial))),
        "stokes_error": float(np.max(abs(circulation-flux))), "gauge_curl_error": gauge_curl,
        "constitutive_error": constitutive, "derivative_error": derivative_error,
        "symmetry_error": float(np.max(abs(stiffness-stiffness.T))),
        "null_stationarity_error": stationarity,
        "gradient": gradient.tolist(), "finite_difference_gradient": numerical,
        "energy": c["energy"], "mass_condition": c["mass_condition"], "schur_condition": c["schur_condition"],
        "internal_faces": face_results, "exterior_faces": exterior,
        "exterior_trace_change_max": max(f["old_inside_change"] for f in exterior),
        "exterior_old_outside_mismatch_max": max(f["old_outside_mismatch"] for f in exterior)}


def face_diagnostics(patch, construction):
    uv, weights = quadrature(6, 2)
    off = np.array([[.2, .2], [.6, .2], [.2, .6], [.1, .35], [.45, .45], [.7, .1], [1/3, 1/3]])
    internal, exterior, constitutive = [], [], 0.
    for fi, face in enumerate(patch["faces"]):
        owners = patch["owners"][face]
        triangle = patch["ambient"][list(face)]
        face_metric = metric(triangle, uv)
        off_metric = metric(triangle, off)
        edge_indices = np.flatnonzero(patch["D"][fi])
        global_edges = [patch["edge_ids"][e] for e in edge_indices]
        fields, off_fields, bases, old_fields, orientations = [], [], [], [], []
        for ti, local_face in owners:
            ci = patch["cell_ids"][ti]; cell = patch["payload"]["cells"][ci]
            local = [patch["cell_vertices"][ti].index(v) for v in face]
            ref = VERTICES[local]; jacobian = (ref[1:]-ref[0]).T
            coordinates = ref[0]+uv@jacobian.T
            h, error = owner_field(patch, ti, coordinates, construction["polynomial"], construction["degree"])
            constitutive = max(constitutive, error); fields.append(h@jacobian)
            hh, error = owner_field(patch, ti, ref[0]+off@jacobian.T, construction["polynomial"], construction["degree"])
            constitutive = max(constitutive, error); off_fields.append(hh@jacobian)
            signs = np.where(cell["reverse"], -1., 1.)
            tests = np.zeros((6, 3))
            for j, edge in enumerate(global_edges):
                ei = cell["edges"].index(edge); tests[ei, j] = signs[ei]
            bases.append(np.einsum("qea,ab->qeb", test_field(coordinates, tests), jacobian))
            old_flux = (patch["all_data"][cell["edges"]]*signs)@np.array([2*np.cross(GRADS[i], GRADS[j]) for i,j in EDGES])
            old_fields.append(hodge(patch["points"][ti], coordinates, old_flux)[0]@jacobian)
            normal = np.cross(jacobian[:, 0], jacobian[:, 1])
            opposite = next(i for i in range(4) if i not in local)
            orientations.append(-int(np.sign(normal@(VERTICES[opposite]-ref[0]))))
        if len(owners) == 2:
            if orientations[0] != -orientations[1] or np.max(abs(bases[0]-bases[1])) > TOL:
                raise ValueError("owner orientation or original test trace mismatch")
            result = trace_report(fields[0], fields[1], face_metric, weights, bases[0], orientations[0])
            old = trace_report(old_fields[0], old_fields[1], face_metric, weights, bases[0], orientations[0])
            internal.append({"patch_face": fi, "edge_ids": global_edges,
                "cells": [patch["cell_ids"][ti] for ti, _ in owners], "quadrature": result,
                "original_whitney": old, "off_quadrature_max": physical_max(off_fields[0]-off_fields[1], off_metric)})
        else:
            outside = outside_trace(patch, face, global_edges, uv)
            exterior.append({"patch_face": fi, "edge_ids": global_edges,
                "old_inside_change": physical_max(fields[0]-old_fields[0], face_metric),
                "original_inside_outside_mismatch": physical_max(old_fields[0]-outside, face_metric),
                "old_outside_mismatch": physical_max(fields[0]-outside, face_metric)})
    if len(internal) != 6 or len(exterior) != 12:
        raise ValueError("fixed six internal / twelve external faces required")
    return internal, exterior, constitutive


def physical_max(difference, g):
    raised = np.linalg.solve(g, difference[..., None])[..., 0]
    return float(np.sqrt(np.maximum(0., np.einsum("qi,qi->q", difference, raised))).max())


def outside_trace(patch, face, global_edges, uv):
    candidates = [(ci, cell) for ci, cell in enumerate(patch["payload"]["cells"])
        if ci not in patch["cell_ids"] and set(global_edges).issubset(cell["edges"])]
    if len(candidates) != 1:
        raise ValueError("unique original exterior neighbor required")
    _, cell = candidates[0]
    local = []
    for vertex in face:
        incident = [edge for edge, pair in zip(patch["edge_ids"], patch["pairs"])
                    if vertex in pair and edge in global_edges]
        ends = [set(EDGES[cell["edges"].index(edge)]) for edge in incident]
        match = ends[0].intersection(ends[1])
        if len(match) != 1:
            raise ValueError("outside face corner identity mismatch")
        local.append(match.pop())
    ref = VERTICES[local]; jacobian = (ref[1:]-ref[0]).T
    coordinates = ref[0]+uv@jacobian.T
    points = np.array(cell["vertices"])
    if np.max(abs(metric(points[local], uv)-metric(patch["ambient"][list(face)], uv))) > TOL:
        raise ValueError("exterior neighbor physical face metric disagrees")
    flux = (patch["all_data"][cell["edges"]]*np.where(cell["reverse"], -1., 1.))
    flux = flux@np.array([2*np.cross(GRADS[i], GRADS[j]) for i,j in EDGES])
    return hodge(points, coordinates, flux)[0]@jacobian


def verify(directory):
    directory = Path(directory)
    marker = directory/"VERIFIED.json"
    marker.unlink(missing_ok=True)
    from ricci_shared_face_patch import strict_load
    actual = strict_load(directory/"diagnostic.json")
    expected = build_receipt()
    if json.dumps(actual, sort_keys=True, allow_nan=False) != json.dumps(expected, sort_keys=True, allow_nan=False):
        raise ValueError("shared-curvature evidence does not match independent replay")
    if not expected["local_gate_passed"]:
        raise ValueError("local gate did not pass replay")
    result = {"local_curvature_construction_verified": True, "global_repair": False,
        "release_gate": "UNCHANGED_OPEN", "diagnostic_sha256": sha(directory/"diagnostic.json"),
        "verifier_sha256": sha(Path(__file__)), "new_global_solves": 0}
    marker.write_text(json.dumps(result, indent=2)+"\n", encoding="utf-8")
    return result


def build_receipt():
    p = geometry(); audit = rank_audit(p)
    if audit["chosen_degree"] is None:
        return {"schema": "shared_curvature_patch_v1", "rank_audit": audit,
                "local_gate_passed": False, "release_gate": "UNCHANGED_OPEN"}
    degree = audit["chosen_degree"]
    c = construct(p, degree, p["data"])
    report = diagnostics(p, c, p["data"])
    names = ("edge_error", "flux_error", "closure_error", "curl_error", "stokes_error",
             "gauge_curl_error", "constitutive_error", "derivative_error", "symmetry_error", "null_stationarity_error")
    passed = all(np.isfinite(report[k]) and report[k] <= TOL for k in names)
    passed &= all(f["quadrature"]["sampled_trace_match"] and f["quadrature"]["whitney_moments_match"]
                  and f["off_quadrature_max"] <= TOL for f in report["internal_faces"])
    passed &= all(not f["original_whitney"]["sampled_trace_match"]
                  and not f["original_whitney"]["whitney_moments_match"] for f in report["internal_faces"])
    rng = np.random.default_rng(17062026)
    controls = []
    vectors = [("arbitrary_"+str(i), rng.normal(size=19)) for i in range(3)]
    vectors += [("pure_gauge", p["G"]@rng.normal(size=8)), ("zero", np.zeros(19))]
    for name, data in vectors:
        control = construct(p, degree, data)
        faces, _, constitutive = face_diagnostics(p, control)
        checks = {"edge_error": float(np.max(abs(control["edge_integrals"]-data))),
            "flux_error": float(np.max(abs(control["fluxes"]-p["D"]@data))),
            "closure_error": float(np.max(abs(divergence(degree)@control["polynomial"]))),
            "curl_error": float(np.max(abs(curl(control["potential"], degree+1)-control["polynomial"]))),
            "constitutive_error": constitutive,
            "internal_trace_max": max(f["quadrature"]["physical_jump_max"] for f in faces),
            "off_quadrature_max": max(f["off_quadrature_max"] for f in faces),
            "internal_moment_max": max(max(abs(v) for v in f["quadrature"]["moments"]) for f in faces)}
        if name in ("pure_gauge", "zero"):
            checks["zero_curvature_error"] = float(np.max(abs(control["polynomial"])))
        passed &= all(np.isfinite(v) and v <= TOL for v in checks.values())
        controls.append({"name": name, "data": data.tolist(), "checks": checks})
    source_files = [Path(__file__), ROOT/"docs/RICCI_SHARED_CURVATURE_PATCH_SPEC.md",
        ROOT/"scripts/ricci_shared_curvature_math.py", ROOT/"scripts/ricci_shared_face_math.py",
        ROOT/"scripts/ricci_shared_face_patch.py", ROOT/"scripts/ricci_push_interfaces.py",
        ROOT/"scripts/test_ricci_shared_curvature_math.py", ROOT/"scripts/test_ricci_shared_curvature_patch.py",
        ROOT/"scripts/test_ricci_shared_curvature_evidence.py"]
    return {"schema": "shared_curvature_patch_v1", "local_gate_passed": bool(passed),
        "scope": "six-cell Abelian patch only; incompatible with unchanged exterior traces",
        "mapping": "UNCHANGED", "tolerance": TOL, "release_gate": "UNCHANGED_OPEN",
        "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "worktree": "dirty; source hashes identify tested implementation", "new_global_solves": 0,
        "inputs": p["inputs"], "sources": {str(f): sha(f) for f in source_files},
        "cell_ids": p["cell_ids"], "edge_ids": p["edge_ids"], "edge_values": p["data"].tolist(),
        "face_edge_incidence": p["D"].tolist(), "cell_face_incidence": p["B"].tolist(),
        "normalization": {"center": p["center"].tolist(), "scale": p["scale"]},
        "rank_audit": audit, "polynomial": c["polynomial"].tolist(),
        "coefficients": c["coefficients"].tolist(), "potential": c["potential"].tolist(),
        "vertex_gauge": c["gauge"].tolist(), "fluxes": c["fluxes"].tolist(),
        "recovered_edge_integrals": c["edge_integrals"].tolist(), "report": report, "controls": controls}


def main():
    parser = argparse.ArgumentParser(); parser.add_argument("mode", choices=["run", "verify"])
    parser.add_argument("--output", type=Path, default=OUT); parser.add_argument("--worker", action="store_true")
    args = parser.parse_args()
    if not args.worker:
        (args.output/"VERIFIED.json").unlink(missing_ok=True)
        subprocess.run([sys.executable, __file__, args.mode, "--output", str(args.output), "--worker"],
                       check=True, timeout=180)
        return
    args.output.mkdir(parents=True, exist_ok=True)
    if args.mode == "run":
        path = args.output/"diagnostic.json"
        if path.exists():
            raise ValueError("preserve diagnostic; do not overwrite experiments")
        receipt = build_receipt()
        path.write_text(json.dumps(receipt, indent=2, allow_nan=False)+"\n", encoding="utf-8")
        print(json.dumps({"local_gate_passed": receipt["local_gate_passed"], "rank_audit": receipt["rank_audit"]}, indent=2))
        if not receipt["local_gate_passed"]:
            raise ValueError("bounded candidate failed; do not enlarge family or change thresholds")
    else:
        print(json.dumps(verify(args.output), indent=2))


if __name__ == "__main__":
    main()
