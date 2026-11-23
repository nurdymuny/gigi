"""Fixed-patch shared-face checker and fail-closed evidence replay."""
from pathlib import Path
import argparse
import json
import subprocess
import sys
import numpy as np

from ricci_local_push_audit import OUT as PRIOR, NATIVE, OLD, KEY, read, sha
from ricci_push_interfaces import quadrature, hodge, smooth_hodge, test_field
from ricci_shared_face_math import pullback, trace_report, TOLERANCE
from ricci_variational_math import EDGES, FACES, GRADS, VERTICES
from ricci_variational_mesh import topology, deck
from ricci_lattice_mesh import plus

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "output/ricci-shared-face-20261117"
FIXED_FACES = [0, 3, 8, 9, 1009, 1012]


def face_data(payload, topo, fi, a, permutation=(0, 1, 2)):
    uv, weights = quadrature(6, 2)
    triangle = topo["faces"][fi][list(permutation)]
    b = (triangle[1:]-triangle[0]).T
    y = triangle[0] + uv@b.T
    radius = np.linalg.norm(y, axis=1)
    x = y/radius[:, None]
    dx = (b[None]-x[:, :, None]*np.einsum("qi,ia->qa", x, b)[:, None, :])/radius[:, None, None]
    metric = np.einsum("qia,qib->qab", dx, dx)
    elements = deck()[0]
    owners = []
    global_edges = None
    curls = np.array([2*np.cross(GRADS[i], GRADS[j]) for i, j in EDGES])
    for ci, local_face, gi in topo["owners"][fi]:
        cell = payload["cells"][ci]
        points = np.array(cell["vertices"])
        transformed = plus(elements[gi], triangle)
        homogeneous = transformed@np.linalg.inv(points.T).T
        bary = homogeneous/homogeneous.sum(axis=1)[:, None]
        if np.max(abs(bary-np.round(bary))) > 2e-12:
            raise ValueError("shared-face vertex correspondence failed")
        chart_vertices = bary[:, 1:]
        jacobian = (chart_vertices[1:]-chart_vertices[0]).T
        coordinates = chart_vertices[0] + uv@jacobian.T
        ids = np.array(cell["edges"])
        signs = np.where(cell["reverse"], -1., 1.)
        face_vertices = FACES[local_face]
        local_edges = [i for i, (u, v) in enumerate(EDGES) if u in face_vertices and v in face_vertices]
        edge_ids = sorted(ids[local_edges].tolist())
        if global_edges is None:
            global_edges = edge_ids
        elif global_edges != edge_ids:
            raise ValueError("owners disagree on original face-edge identities")
        unit_tests = np.zeros((6, 3))
        for j, edge in enumerate(global_edges):
            index = cell["edges"].index(edge)
            unit_tests[index, j] = signs[index]
        basis = np.einsum("qea,ab->qeb", test_field(coordinates, unit_tests), jacobian)
        flux = (a[ids]*signs)@curls
        frozen = pullback(hodge(points, coordinates, flux)[0], jacobian)
        smooth = pullback(smooth_hodge(points, coordinates), jacobian)
        ref_face = VERTICES[list(face_vertices)]
        outward = np.cross(ref_face[1]-ref_face[0], ref_face[2]-ref_face[0])
        opposite = next(i for i in range(4) if i not in face_vertices)
        if outward@(VERTICES[opposite]-ref_face[0]) > 0:
            outward = -outward
        direction = float(outward@np.cross(jacobian[:, 0], jacobian[:, 1]))
        if abs(direction) < 1e-12:
            raise ValueError("degenerate face orientation")
        owners.append({"cell": ci, "orientation": 1 if direction > 0 else -1,
                       "basis": basis, "frozen_whitney": frozen, "oracle_control": smooth})
    if len(owners) != 2 or owners[0]["orientation"] != -owners[1]["orientation"]:
        raise ValueError("opposite outward owner orientations required")
    basis_error = float(np.max(abs(owners[0]["basis"]-owners[1]["basis"])))
    if basis_error > TOLERANCE:
        raise ValueError("original Whitney test traces disagree")
    result = {"face": fi, "cells": [o["cell"] for o in owners], "edge_ids": global_edges,
              "orientation": owners[0]["orientation"], "test_trace_agreement_error": basis_error}
    for kind in ("frozen_whitney", "oracle_control"):
        result[kind] = trace_report(owners[0][kind], owners[1][kind], metric, weights,
                                    owners[0]["basis"], owners[0]["orientation"])
    return result


def build_receipt():
    prior = read(PRIOR/"receipt.json")
    interface = read(PRIOR/"interface-receipt.json")
    if sha(PRIOR/"receipt.json") != interface["prior_receipt_sha256"]:
        raise ValueError("prior attribution receipt drift")
    inputs = prior["inputs"] | prior["sources"] | interface["sources"]
    for filename, expected in inputs.items():
        if sha(Path(filename)) != expected:
            raise ValueError("prior source/evidence drift: "+filename)
    arrays = PRIOR/"attributions.npz"
    if sha(arrays) != prior["arrays_sha256"]:
        raise ValueError("saved adjoint evidence drift")
    inputs[str(arrays)] = sha(arrays)
    inputs[str(PRIOR/"receipt.json")] = sha(PRIOR/"receipt.json")
    inputs[str(PRIOR/"interface-receipt.json")] = sha(PRIOR/"interface-receipt.json")
    with np.load(arrays, allow_pickle=False) as data:
        adjoints = data["adjoints"].copy()
        if data["patch_ids"].tolist() != [0, 2, 3, 456, 458, 459]:
            raise ValueError("fixed patch changed")
    with np.load(OLD/(KEY+".arrays.npz"), allow_pickle=False) as data:
        a = data["abelian_edge_integrals"].copy()
    payload = read(NATIVE/"level1_berger_q6.input.json")
    topo = topology(payload)
    old_faces = interface["patch_internal_faces"]
    if [f["face"] for f in old_faces] != FIXED_FACES:
        raise ValueError("fixed internal faces changed")
    results = []
    for old in old_faces:
        face = face_data(payload, topo, old["face"], a)
        reversed_face = face_data(payload, topo, old["face"], a, (0, 2, 1))
        errors = []
        for kind in ("frozen_whitney", "oracle_control"):
            moments = np.array(face[kind]["moments"])
            reversed_moments = np.array(reversed_face[kind]["moments"])
            reversal_error = float(np.max(abs(moments-reversed_moments)))
            if reversal_error > TOLERANCE:
                raise ValueError("common-chart reversal changed signed moments")
            face[kind]["chart_reversal_moment_error"] = reversal_error
            observed = moments@adjoints[face["edge_ids"]]
            expected = np.array(old["net"] if kind == "frozen_whitney" else np.sum(old["smooth_sides"], axis=0))
            replay_error = float(np.max(abs(observed-expected)))
            if replay_error > TOLERANCE:
                raise ValueError("three-moment replay of saved face readout failed")
            face[kind]["replayed_readouts"] = observed.tolist()
            errors.append(replay_error)
        face["saved_readout_replay_error"] = max(errors)
        if not (face["oracle_control"]["sampled_trace_match"] and face["oracle_control"]["whitney_moments_match"]):
            raise ValueError("smooth reference control failed")
        results.append(face)
    source_paths = [Path(__file__), ROOT/"scripts/ricci_shared_face_math.py",
        ROOT/"scripts/test_ricci_shared_face_math.py", ROOT/"scripts/test_ricci_shared_face_patch.py",
        ROOT/"docs/RICCI_SHARED_FACE_CONSTRAINT_SPEC.md"]
    return {"schema": "shared_face_constraint_v1", "case": KEY,
        "scope": "six internal faces only; no repair, whole-mesh compatibility or convergence claim",
        "mapping": "UNCHANGED", "bw05": "UNCHANGED_OPEN", "new_solves": 0,
        "source_kind": "frozen signed Whitney data; smooth reference is oracle_control only",
        "inputs": inputs, "sources": {str(p): sha(p) for p in source_paths}, "faces": results,
        "original_patch_compatible": all(f["frozen_whitney"]["sampled_trace_match"]
            and f["frozen_whitney"]["whitney_moments_match"] for f in results)}


def strict_load(path):
    def reject_constant(value):
        raise ValueError("nonfinite JSON evidence: "+value)
    def unique_object(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("duplicate evidence key: "+key)
            result[key] = value
        return result
    return json.loads(path.read_text(encoding="utf-8"), parse_constant=reject_constant,
                      object_pairs_hook=unique_object)


def verify(directory):
    directory = Path(directory)
    verified_path = directory/"VERIFIED.json"
    verified_path.unlink(missing_ok=True)
    actual = strict_load(directory/"diagnostic.json")
    expected = build_receipt()
    if json.dumps(actual, sort_keys=True, allow_nan=False) != json.dumps(expected, sort_keys=True, allow_nan=False):
        raise ValueError("shared-face evidence does not match independent replay")
    result = {"constraint_checker_verified": True,
              "original_patch_compatible": expected["original_patch_compatible"],
              "release_gate": "UNCHANGED_OPEN", "new_solves": 0,
              "diagnostic_sha256": sha(directory/"diagnostic.json"),
              "verifier_sha256": sha(Path(__file__))}
    verified_path.write_text(json.dumps(result, indent=2)+"\n", encoding="utf-8")
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=["run", "verify"])
    parser.add_argument("--output", type=Path, default=OUT)
    parser.add_argument("--worker", action="store_true")
    args = parser.parse_args()
    if not args.worker:
        subprocess.run([sys.executable, __file__, args.mode, "--output", str(args.output), "--worker"],
                       check=True, timeout=90)
        return
    args.output.mkdir(parents=True, exist_ok=True)
    if args.mode == "run":
        path = args.output/"diagnostic.json"
        if path.exists():
            raise ValueError("preserve previous diagnostic")
        receipt = build_receipt()
        path.write_text(json.dumps(receipt, indent=2, allow_nan=False)+"\n", encoding="utf-8")
        print(json.dumps([{k: face[k] for k in ("face", "frozen_whitney", "oracle_control",
                                               "saved_readout_replay_error")} for face in receipt["faces"]], indent=2))
    else:
        print(json.dumps(verify(args.output), indent=2))


if __name__ == "__main__":
    main()
