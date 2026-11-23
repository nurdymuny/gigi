"""One pass separating interior and interface contributions; no solves."""
import json
from pathlib import Path
import subprocess
import sys
import time
import numpy as np

from ricci_local_push_audit import OUT, KEY, ROOT, NATIVE, OLD, read, sha
from ricci_push_interfaces import cell_parts
from ricci_variational_mesh import topology


def worker():
    destination = OUT / "interface-receipt.json"
    if destination.exists():
        raise ValueError("preserve previous interface receipt")
    start = time.perf_counter()
    prior = read(OUT / "receipt.json")
    for filename, digest in (prior["inputs"] | prior["sources"]).items():
        if sha(Path(filename)) != digest:
            raise ValueError("prior source drift: " + filename)
    source = OUT / "attributions.npz"
    if sha(source) != prior["arrays_sha256"]:
        raise ValueError("prior attribution drift")
    with np.load(source, allow_pickle=False) as data:
        ids, signs, adjoints = data["ids"].copy(), data["signs"].copy(), data["adjoints"].copy()
        actual, smooth = data["actual"].copy(), data["smooth"].copy()
        patch_ids = set(data["patch_ids"].tolist())
    with np.load(OLD / (KEY + ".arrays.npz"), allow_pickle=False) as data:
        a = data["abelian_edge_integrals"].copy()
    payload = read(NATIVE / "level1_berger_q6.input.json")
    bulk = []; faces = []; smooth_faces = []; direct = []
    for ci, cell in enumerate(payload["cells"]):
        result = cell_parts(np.array(cell["vertices"]), a[ids[ci]]*signs[ci],
                            adjoints[ids[ci]]*signs[ci, :, None])
        bulk.append(result["bulk"]); faces.append(result["faces"])
        smooth_faces.append(result["smooth_faces"]); direct.append(result["direct"])
    bulk, faces, smooth_faces, direct = map(np.array, (bulk, faces, smooth_faces, direct))
    direct_difference = float(np.max(abs(direct - actual)))
    decomposition_error = float(np.max(abs(bulk + faces.sum(axis=1) - actual)))
    if direct_difference > 1e-9 or decomposition_error > 1e-9:
        raise ValueError("independent per-cell boundary accounting failed")
    topo = topology(payload)
    pair_values = []; smooth_pairs = []; inside = []; touching = []
    for fi, owners in enumerate(topo["owners"]):
        pair_values.append(sum(faces[ci, local] for ci, local, _ in owners))
        smooth_pairs.append(sum(smooth_faces[ci, local] for ci, local, _ in owners))
        count = sum(ci in patch_ids for ci, _, _ in owners)
        if count == 2:
            inside.append(fi)
        elif count == 1:
            touching.append(fi)
    pair_values, smooth_pairs = map(np.array, (pair_values, smooth_pairs))
    cancellation_error = float(np.max(abs(smooth_pairs)))
    if cancellation_error > 1e-9:
        raise ValueError("smooth shared-face cancellation failed")
    interior_error = (bulk-smooth).sum(axis=0)
    interface_error = pair_values.sum(axis=0)
    closure = float(np.max(abs(interior_error + interface_error - np.array(prior["action_error"]))))
    if closure > 1e-9:
        raise ValueError("whole-readout interface split failed")
    if np.linalg.norm(interior_error-np.array(prior["action_error"])) < 1e-6:
        raise ValueError("boundary-removal control did not discriminate")
    arrays = OUT / "interfaces.npz"
    np.savez_compressed(arrays, bulk=bulk, faces=faces, smooth_faces=smooth_faces,
                        pairs=pair_values, smooth_pairs=smooth_pairs, inside=inside, touching=touching)
    other = np.ones(len(pair_values), dtype=bool); other[inside+touching] = False
    receipt = {
        "status": "ACCOUNTING_VERIFIED_NOT_A_REPAIR", "case": KEY,
        "prior_receipt_sha256": sha(OUT / "receipt.json"), "prior_arrays_sha256": sha(source),
        "sources": {str(path): sha(path) for path in [Path(__file__),
            ROOT/"scripts/ricci_push_interfaces.py", ROOT/"scripts/test_ricci_push_interfaces.py",
            ROOT/"docs/RICCI_LOCAL_PUSH_INTERFACE_ADDENDUM.md"]},
        "arrays_sha256": sha(arrays), "additional_solves": 0,
        "checks": {"independent_direct_force_max_error": direct_difference,
            "per_cell_integration_by_parts_max_error": decomposition_error,
            "smooth_shared_face_cancellation_max_error": cancellation_error,
            "full_observation_split_error": closure,
            "boundary_removal_discrepancy": float(np.linalg.norm(interior_error-np.array(prior["action_error"])))},
        "interior_error": interior_error.tolist(), "interface_error": interface_error.tolist(),
        "patch_internal_faces": [{"face": fi, "cells": [ci for ci, _, _ in topo["owners"][fi]],
            "sides": [faces[ci, local].tolist() for ci, local, _ in topo["owners"][fi]],
            "smooth_sides": [smooth_faces[ci, local].tolist() for ci, local, _ in topo["owners"][fi]],
            "net": pair_values[fi].tolist()} for fi in inside],
        "patch_internal_interface_sum": pair_values[inside].sum(axis=0).tolist(),
        "patch_external_interface_sum": pair_values[touching].sum(axis=0).tolist(),
        "other_interface_sum": pair_values[other].sum(axis=0).tolist(),
        "elapsed_seconds": time.perf_counter()-start,
        "limitations": "one point on one original Abelian mesh; no convergence or repair claim",
    }
    destination.write_text(json.dumps(receipt, indent=2)+"\n", encoding="utf-8")
    print(json.dumps({k:v for k,v in receipt.items() if k not in ["sources"]}, indent=2))


if __name__ == "__main__":
    if "--worker" in sys.argv:
        worker()
    else:
        subprocess.run([sys.executable, __file__, "--worker"], check=True, timeout=120)
