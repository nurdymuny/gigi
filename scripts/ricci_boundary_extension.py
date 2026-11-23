"""Flux-neutral boundary extension and necessary compatibility witnesses."""
from pathlib import Path
import argparse
import json
import subprocess
import sys
import numpy as np
from scipy.linalg import null_space
import ricci_shared_curvature_patch as original
from ricci_shared_curvature_math import (TOL, RCOND, minimum_energy, potential, edge_integrals)
from ricci_push_interfaces import hodge
from ricci_variational_math import EDGES, GRADS, VERTICES
from ricci_local_push_audit import sha
from ricci_shared_face_patch import strict_load

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT/"output/ricci-boundary-extension-20261117"


def exact_lift(mass, flux, trace, base, target):
    mass, flux, trace, base, target = map(np.asarray, (mass, flux, trace, base, target))
    if not all(np.isfinite(a).all() for a in (mass, flux, trace, base, target)):
        raise ValueError("finite boundary-lift inputs required")
    if (mass.shape != (len(base), len(base)) or flux.shape[1] != len(base)
            or trace.shape != (len(target), len(base))):
        raise ValueError("boundary-lift dimensions disagree")
    z = null_space(flux, rcond=RCOND)
    a = trace@z; rhs = target-trace@base
    u, singular, _ = np.linalg.svd(a, full_matrices=False)
    rank = int(np.count_nonzero(singular > singular[0]*RCOND)) if len(singular) else 0
    residual = rhs-u[:, :rank]@(u[:, :rank].T@rhs)
    result = {"null_dimensions": z.shape[1], "boundary_rank": rank,
        "residual": residual, "residual_norm": float(np.linalg.norm(residual)),
        "witness_orthogonality_error": float(np.max(abs(a.T@residual), initial=0)),
        "witness_identity_error": float(abs(residual@rhs-residual@residual)),
        "singular_values": singular}
    if np.max(abs(residual), initial=0) > TOL:
        return {**result, "status": "REFUSED_INCOMPATIBLE_BOUNDARY"}
    if z.shape[1]:
        fit = minimum_energy(z.T@mass@z, a, rhs)
        lift = z@fit["map"]
    else:
        lift = np.zeros((len(base), len(target)))
    delta = lift@rhs
    if np.max(abs(flux@delta), initial=0) > TOL:
        return {**result, "status": "REFUSED_FLUX_CHANGED"}
    coefficients = base+delta
    if np.max(abs(trace@coefficients-target), initial=0) > TOL:
        return {**result, "status": "REFUSED_BOUNDARY_RESIDUAL"}
    return {**result, "status": "ACCEPTED", "coefficients": coefficients, "delta": delta,
        "correction_energy": float(.5*delta@mass@delta), "target_map": lift,
        "base_map": np.eye(len(base))-lift@trace}


def unit_component(covectors, direction, metrics):
    covectors, direction, metrics = map(np.asarray, (covectors, direction, metrics))
    if not all(np.isfinite(a).all() for a in (covectors, direction, metrics)):
        raise ValueError("finite unit-tangent inputs required")
    speed = np.sqrt(np.einsum("i,qij,j->q", direction, metrics, direction))
    if np.any(speed <= 0):
        raise ValueError("positive physical edge speed required")
    return (covectors@direction)/speed


def boundary_system(patch, basis, uv):
    matrices, targets, faces = [], [], []
    for fi, face in exterior_faces(patch):
        ti, _ = patch["owners"][face][0]
        local = [patch["cell_vertices"][ti].index(v) for v in face]
        ref = VERTICES[local]; jacobian = (ref[1:]-ref[0]).T
        coordinates = ref[0]+uv@jacobian.T
        g = original.metric(patch["ambient"][list(face)], uv)
        lower = np.linalg.cholesky(g); area = np.sqrt(np.linalg.det(g))
        normal = np.cross(jacobian[:, 0], jacobian[:, 1])
        traces = []
        for polynomial in basis.T:
            hh, _ = original.owner_field(patch, ti, coordinates, polynomial, 2)
            tangential = hh@jacobian
            # Invert the constitutive relation in the original cell, then pull
            # its two-form onto this oriented face; not just integrated flux.
            local_metric = original.metric(patch["points"][ti], coordinates)
            flux = np.linalg.solve(local_metric, hh[..., None])[..., 0]*np.sqrt(np.linalg.det(local_metric))[:, None]
            whitened = np.linalg.solve(lower, tangential[..., None])[..., 0]
            traces.append(np.column_stack([whitened, (flux@normal)/area]).ravel())
        matrices.append(np.column_stack(traces))
        _, cell, outside_local = exterior_cell(patch, face)
        outside_ref = VERTICES[outside_local]; outside_jac = (outside_ref[1:]-outside_ref[0]).T
        outside_coord = outside_ref[0]+uv@outside_jac.T
        points = np.array(cell["vertices"]); flux = old_flux(patch, cell)
        tangential = hodge(points, outside_coord, flux)[0]@outside_jac
        if np.max(abs(original.metric(points[outside_local], uv)-g)) > TOL:
            raise ValueError("outside face metric disagrees")
        whitened = np.linalg.solve(lower, tangential[..., None])[..., 0]
        normal_flux = np.full(len(uv), flux@np.cross(outside_jac[:, 0], outside_jac[:, 1]))/area
        targets.append(np.column_stack([whitened, normal_flux]).ravel())
        faces.append(fi)
    return {"trace": np.vstack(matrices), "target": np.concatenate(targets), "faces": faces,
            "points": len(uv)*len(faces)}


def exterior_faces(patch):
    return [(fi, face) for fi, face in enumerate(patch["faces"]) if len(patch["owners"][face]) == 1]


def exterior_cell(patch, face):
    fi = patch["faces"].index(face)
    global_edges = [patch["edge_ids"][e] for e in np.flatnonzero(patch["D"][fi])]
    candidates = [(ci, cell) for ci, cell in enumerate(patch["payload"]["cells"])
                  if ci not in patch["cell_ids"] and set(global_edges).issubset(cell["edges"])]
    if len(candidates) != 1:
        raise ValueError("unique exterior neighbor required")
    ci, cell = candidates[0]; local = []
    for vertex in face:
        incident = [edge for edge, pair in zip(patch["edge_ids"], patch["pairs"])
                    if vertex in pair and edge in global_edges]
        endpoints = [set(EDGES[cell["edges"].index(edge)]) for edge in incident]
        match = endpoints[0].intersection(endpoints[1])
        if len(match) != 1:
            raise ValueError("unique exterior corner correspondence required")
        local.append(match.pop())
    return ci, cell, local


def old_flux(patch, cell):
    values = patch["all_data"][cell["edges"]]*np.where(cell["reverse"], -1., 1.)
    return values@np.array([2*np.cross(GRADS[i], GRADS[j]) for i,j in EDGES])


def edge_audit(patch, common_polynomial=None):
    nodes = (np.polynomial.legendre.leggauss(6)[0]+1)/2
    nodes = np.r_[nodes, [.17, .37, .73]]
    refs = np.array([[0., 0.], [1., 0.], [0., 1.]])
    entries = {}; replay, reversal = 0., 0.
    for fi, face in exterior_faces(patch):
        ci, cell, local = exterior_cell(patch, face)
        global_edges = [patch["edge_ids"][e] for e in np.flatnonzero(patch["D"][fi])]
        for edge in global_edges:
            ei = patch["edge_ids"].index(edge)
            tail, head = patch["pairs"][ei]
            start, stop = refs[face.index(tail)], refs[face.index(head)]
            direction = stop-start; uv = start+nodes[:, None]*direction
            g = original.metric(patch["ambient"][list(face)], uv)
            if common_polynomial is None:
                tangential = original.outside_trace(patch, face, global_edges, uv)
                # Independent direct route: original global edge orientation in
                # the outside cell, not the face-corner correspondence above.
                li = cell["edges"].index(edge); u, v = EDGES[li]
                if cell["reverse"][li]:
                    u, v = v, u
                delta = VERTICES[v]-VERTICES[u]
                coordinates = VERTICES[u]+nodes[:, None]*delta
                points = np.array(cell["vertices"])
                hh = hodge(points, coordinates, old_flux(patch, cell))[0]
                direct = unit_component(hh, delta, original.metric(points, coordinates))
            else:
                ti, _ = patch["owners"][face][0]
                ref = VERTICES[[patch["cell_vertices"][ti].index(v) for v in face]]
                jacobian = (ref[1:]-ref[0]).T
                hh, _ = original.owner_field(patch, ti, ref[0]+uv@jacobian.T, common_polynomial, 2)
                tangential = hh@jacobian
                direct = unit_component(hh, jacobian@direction, original.metric(patch["points"][ti], ref[0]+uv@jacobian.T))
            value = unit_component(tangential, direction, g)
            replay = max(replay, float(np.max(abs(value-direct))))
            reverse_uv = stop+(1-nodes)[:, None]*(-direction)
            if common_polynomial is None:
                reversed_trace = original.outside_trace(patch, face, global_edges, reverse_uv)
            else:
                reversed_trace = original.owner_field(patch, ti, ref[0]+reverse_uv@jacobian.T, common_polynomial, 2)[0]@jacobian
            reversed_value = unit_component(reversed_trace, -direction, original.metric(patch["ambient"][list(face)], reverse_uv))
            reversal = max(reversal, float(np.max(abs(value+reversed_value))))
            entries.setdefault(edge, []).append({"face": fi, "outside_cell": ci, "values": value.tolist()})
    reports = []
    for edge, sides in sorted(entries.items()):
        if len(sides) != 2:
            raise ValueError("closed boundary requires exactly two face traces per edge")
        gaps = abs(np.array(sides[0]["values"])-sides[1]["values"])
        reports.append({"edge": edge, "sides": sides, "gaps": gaps.tolist(), "max_gap": float(gaps.max()),
                        "necessary_minimax_error": float(gaps.max()/2)})
    if len(reports) != 18:
        raise ValueError("same eighteen boundary edges required")
    maximum = max(r["max_gap"] for r in reports)
    return {"nodes": nodes.tolist(), "edges": reports, "max_gap": maximum,
        "necessary_minimax_error": maximum/2, "direct_cell_replay_error": replay,
        "reversal_error": reversal, "necessary_condition_passed": bool(maximum/2 <= TOL)}


def finalize(patch, base, lifted):
    if lifted["status"] != "ACCEPTED":
        raise ValueError("refused boundary data has no extension to publish")
    coefficients = lifted["coefficients"]
    polynomial = base["basis"]@coefficients
    connection = potential(polynomial, 2)
    original_integrals = edge_integrals(connection, 3, patch["vertices"][np.array(patch["pairs"])])
    gauge = original.recover_gauge(patch, patch["data"]-original_integrals)
    return {"degree": 2, "coefficients": coefficients, "polynomial": polynomial,
        "potential": connection, "gauge": gauge, "edge_integrals": original_integrals+patch["G"]@gauge,
        "fluxes": base["flux_map"]@coefficients}


def verify(directory):
    directory = Path(directory); marker = directory/"VERIFIED.json"
    marker.unlink(missing_ok=True)
    actual = strict_load(directory/"diagnostic.json")
    expected = build_receipt()
    if json.dumps(actual, sort_keys=True, allow_nan=False) != json.dumps(expected, sort_keys=True, allow_nan=False):
        raise ValueError("boundary evidence does not match fresh replay")
    result = {"evidence_verified": True, "boundary_extension_passed": expected["boundary_extension_passed"],
        "status": expected["status"], "global_gate": "UNCHANGED_OPEN",
        "diagnostic_sha256": sha(directory/"diagnostic.json"), "verifier_sha256": sha(Path(__file__))}
    marker.write_text(json.dumps(result, indent=2)+"\n", encoding="utf-8")
    return result


def serial(value):
    if isinstance(value, np.ndarray):
        return value.tolist()
    if isinstance(value, dict):
        return {k: serial(v) for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        return [serial(v) for v in value]
    return value


def build_receipt():
    prior_path = original.OUT/"final-gate.json"
    prior = strict_load(prior_path)
    for group in (prior["sources"], prior["artifacts"]):
        for name, expected in group.items():
            if sha(Path(name)) != expected:
                raise ValueError("prior construction evidence drift: "+name)
    p = original.geometry(); base = original.construct(p, 2, p["data"])
    audit = edge_audit(p)
    positive = edge_audit(p, base["polynomial"])
    for check in (audit, positive):
        if max(check["direct_cell_replay_error"], check["reversal_error"]) > TOL:
            raise ValueError("edge geometry/orientation replay failed")
    if not positive["necessary_condition_passed"]:
        raise ValueError("common smooth field must pass edge compatibility")
    uv, _ = original.quadrature(6, 2)
    system = boundary_system(p, base["basis"], uv)
    fit = exact_lift(base["mass"], base["flux_map"], system["trace"], base["coefficients"], system["target"])
    if max(fit["witness_orthogonality_error"], fit["witness_identity_error"]) > TOL:
        raise ValueError("linear incompatibility witness is numerically unresolved")
    z = null_space(base["flux_map"], rcond=RCOND)
    planted = base["coefficients"]+z@np.linspace(-.002, .003, z.shape[1])
    control = exact_lift(base["mass"], base["flux_map"], system["trace"],
                         base["coefficients"], system["trace"]@planted)
    if control["status"] != "ACCEPTED":
        raise ValueError("planted compatible lift failed")
    changed = finalize(p, base, control)
    faces, _, constitutive = original.face_diagnostics(p, changed)
    off = np.array([[.2,.2],[.6,.2],[.2,.6],[.1,.35],[.45,.45],[.7,.1],[1/3,1/3]])
    off_system = boundary_system(p, base["basis"], off)
    off_error = (off_system["trace"]@(control["coefficients"]-planted)).reshape(-1, 3)
    from ricci_shared_curvature_math import curl, divergence
    control_checks = {"coefficient_recovery": float(np.max(abs(control["coefficients"]-planted))),
        "edge_preservation": float(np.max(abs(changed["edge_integrals"]-p["data"]))),
        "flux_preservation": float(np.max(abs(changed["fluxes"]-p["D"]@p["data"]))),
        "closure": float(np.max(abs(divergence(2)@changed["polynomial"]))),
        "potential_curl": float(np.max(abs(curl(changed["potential"], 3)-changed["polynomial"]))),
        "internal_trace": max(f["quadrature"]["physical_jump_max"] for f in faces),
        "internal_moments": max(max(abs(v) for v in f["quadrature"]["moments"]) for f in faces),
        "off_quadrature_tangential": float(np.linalg.norm(off_error[:, :2], axis=1).max()),
        "off_quadrature_normal_flux": float(np.max(abs(off_error[:, 2]))), "constitutive": constitutive}
    if max(control_checks.values()) > TOL:
        raise ValueError("planted compatible reconstruction invariants failed")
    # This bounded experiment intentionally does not promote an unexpected
    # feasible real case without its complete real-data derivative gate.
    if fit["status"] == "ACCEPTED":
        raise ValueError("real data unexpectedly feasible; complete BE03 acceptance required")
    source_files = [Path(__file__), ROOT/"scripts/test_ricci_boundary_extension.py",
        ROOT/"scripts/test_ricci_boundary_evidence.py", ROOT/"docs/RICCI_BOUNDARY_EXTENSION_SPEC.md"]
    inputs = p["inputs"] | {str(prior_path): sha(prior_path)} | prior["sources"] | prior["artifacts"]
    certificate = {**fit, "face_points": system["points"], "boundary_faces": system["faces"],
        "equations": len(system["target"]), "target": system["target"],
        "family_max_error_lower_bound": fit["residual_norm"]/np.sqrt(2*system["points"])}
    return serial({"schema": "boundary_extension_v1", "status": "REFUSED_INCOMPATIBLE_BOUNDARY",
        "boundary_extension_passed": False, "global_gate": "UNCHANGED_OPEN", "mapping": "UNCHANGED",
        "tolerance": TOL, "scope": "fixed six-cell common quadratic; unchanged outside fields",
        "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "worktree": "dirty; pinned source hashes identify this experiment",
        "cell_ids": p["cell_ids"], "inputs": inputs, "sources": {str(f): sha(f) for f in source_files},
        "edge_audit": audit, "compatible_edge_control": positive,
        "linear_certificate": certificate, "planted_extension_checks": control_checks,
        "prior_reconstruction_unchanged": True, "new_global_solves": 0, "full_suite_run": False})


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
        destination = args.output/"diagnostic.json"
        if destination.exists():
            raise ValueError("preserve previous experiment")
        result = build_receipt()
        destination.write_text(json.dumps(result, indent=2, allow_nan=False)+"\n", encoding="utf-8")
        print(json.dumps({"status": result["status"], "boundary_extension_passed": result["boundary_extension_passed"],
            "edge_lower_bound": result["edge_audit"]["necessary_minimax_error"],
            "quadratic_family_lower_bound": result["linear_certificate"]["family_max_error_lower_bound"]}, indent=2))
    else:
        print(json.dumps(verify(args.output), indent=2))


if __name__ == "__main__":
    main()
