"""Bounded, reproducible audit of b418ef79; no source mutation or new study scoring.

Run with OPENBLAS_NUM_THREADS=1, OMP_NUM_THREADS=1, MKL_NUM_THREADS=1:
  python scripts/ym_geometry_adversarial_review.py OUTPUT_DIRECTORY

The witnesses intentionally demonstrate weaknesses. A completed audit is NOT a
passing scientific gate for the submitted study. Large ladder runs are excluded.
"""
import ast
import contextlib
import copy
import hashlib
import inspect
import io
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import time
import warnings
from unittest.mock import patch

import numpy as np
import scipy
import sympy

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import ym_geometry_tests as single
import ym_geometry_lattice_tests as lattice
import ks_su2_lattice as ks

PIN = "b418ef79789769c9ea0ce2ae51906de0bcbc3141"
SOURCE_PATHS = [
    "scripts/ym_geometry_tests.py", "scripts/ym_geometry_rtc_cells.py",
    "scripts/ym_geometry_lattice_tests.py", "scripts/ks_su2_lattice.py",
    "scripts/_receipt_status.py",
    "docs/research/yang-mills-geometric-identities-2026-11-23.md",
    "research/ym-ladder-20261123/prereg_lowest_excitation.md",
    "research/ym-geometry-20261123/lattice_tests.json",
]


def serial(value):
    if isinstance(value, dict):
        return {str(k): serial(v) for k, v in value.items()}
    if isinstance(value, (tuple, list)):
        return [serial(v) for v in value]
    if isinstance(value, np.ndarray):
        return value.tolist()
    if isinstance(value, np.generic):
        return value.item()
    return value


def save(path, value):
    tmp = path.with_suffix(path.suffix + ".pending")
    tmp.write_text(json.dumps(serial(value), indent=2, allow_nan=False) + "\n", encoding="utf-8")
    os.replace(tmp, path)


def text_digest(path):
    return hashlib.sha256(path.read_bytes().replace(b"\r\n", b"\n")).hexdigest()


def submitted_main(module, path):
    with patch.object(sys, "argv", [module.__file__, str(path)]):
        module._receipt_status.begin()
        module.main()
    return json.loads(path.read_text())


class WrongShift(ast.NodeTransformer):
    count = 0

    def visit_BinOp(self, node):
        node = self.generic_visit(node)
        if (isinstance(node.op, ast.Add) and isinstance(node.left, ast.Name)
                and node.left.id == "E0" and isinstance(node.right, ast.Constant)
                and node.right.value == 1e-6):
            node.right = ast.copy_location(ast.Constant(1e-3), node.right)
            self.count += 1
        return node


def old_gate_witnesses(out):
    mutation = WrongShift()
    tree = ast.fix_missing_locations(mutation.visit(ast.parse(inspect.getsource(single.t8))))
    assert mutation.count == 1
    namespace = dict(single.__dict__)
    exec(compile(tree, "<audit:wrong-shift>", "exec"), namespace)
    receipt = out / "wrong-shift-submitted-receipt.json"
    with patch.object(single, "t8", namespace["t8"]):
        result = submitted_main(single, receipt)
    ratios = [r["gate_shifted_E0"]["identity_residual"] / r["gate_shifted_E0"]["expected"]
              for r in result["T8"]]
    assert result["ok"] and all(999 < r < 1001 for r in ratios)

    stale = out / "stale-submitted-receipt.json"
    save(stale, {"ok": True, "run": "previous successful run"})
    before = stale.read_bytes()
    program = (
        "import sys; sys.path.insert(0, sys.argv[1]); import ym_geometry_tests as m; "
        "sys.argv=[m.__file__,sys.argv[2]]; "
        "m.plaquette=lambda *a,**kw: (_ for _ in ()).throw(RuntimeError('planted audit failure')); "
        "m._receipt_status.begin(); m.main()"
    )
    failed = subprocess.run([sys.executable, "-c", program, str(ROOT / "scripts"), str(stale)],
                            capture_output=True, text=True, check=False)
    assert failed.returncode != 0 and stale.read_bytes() == before
    return {
        "wrong_shift": {"submitted_receipt_ok": result["ok"], "residual_to_expected": ratios},
        "stale_pass": {"failed_process_exit": failed.returncode,
                       "receipt_unchanged": stale.read_bytes() == before,
                       "receipt_still_says_ok": json.loads(stale.read_text())["ok"]},
    }


def exact_small_controls():
    H = np.array([[1., -1.], [-1., 1.]])
    psi = np.ones(2) / np.sqrt(2)
    f = np.array([1., 1j])
    x = psi * f
    lhs = np.vdot(x, H @ x)
    wrong = psi[0] * psi[1] * (f[0] - f[1]) ** 2
    right = psi[0] * psi[1] * abs(f[0] - f[1]) ** 2
    assert abs(lhs - right) < 1e-14 and abs(lhs - wrong) > 1
    G = np.diag([1., -1e-15])
    bound = 10 * len(G) * np.finfo(float).eps * abs(G).max()
    accepted = bool(np.linalg.eigvalsh(G).min() >= -bound)
    assert accepted and G[1, 1] < 0

    # Toy transition amplitudes only: no preregistered ladder state is measured.
    sine = np.sin(np.pi * np.arange(1, 4) / 4)
    intensity = sine ** 2
    amplitude_weight = (intensity[0] + intensity[-1]) / intensity.sum()
    squared_intensity_weight = (intensity[0] ** 2 + intensity[-1] ** 2) / (intensity ** 2).sum()
    even, odd = np.array([1., 0., 1.]), np.array([1., 0., -1.])
    assert np.array_equal(even ** 2, odd ** 2)
    assert abs(amplitude_weight - .5) < 1e-14
    assert abs(squared_intensity_weight - 1/3) < 1e-14
    return {
        "complex_graph": {"lhs": [lhs.real, lhs.imag], "rhs_as_written": [wrong.real, wrong.imag],
                          "rhs_modulus_squared": right, "signed_state_measure": [.5, .5]},
        "psd_label": {"negative_eigenvalue": G[1, 1], "accepted_as_certified": accepted, "tolerance": bound},
        "projector_profile": {"sine_amplitudes": sine, "diagonal_intensities": intensity,
                               "correct_edge_weight": amplitude_weight,
                               "edge_weight_if_intensities_are_squared_again": squared_intensity_weight,
                               "opposite_parities_have_identical_diagonal_intensities": True},
    }


def lattice_reducer_witnesses(out, frozen):
    def validation(name, fail=False):
        r = copy.deepcopy(frozen[name]); r["pass_"] = not fail
        return lambda: r

    invalid_entered_physics = []
    def marker(*args):
        invalid_entered_physics.append(True)
        raise RuntimeError("audit stopped before any new physics computation")

    with contextlib.ExitStack() as stack:
        for name in ("V1", "V2", "V3", "V4", "V5"):
            stack.enter_context(patch.object(lattice, name.lower(), validation(name, fail=name == "V1")))
        stack.enter_context(patch.object(lattice, "physics", marker))
        try:
            submitted_main(lattice, out / "invalid-validation-not-published.json")
        except RuntimeError as e:
            assert "audit stopped" in str(e)
    assert invalid_entered_physics

    # Replay saved fixtures through the ACTUAL reducer, not fresh simulations.
    table = copy.deepcopy(frozen["table"])
    for row in table:
        if row["p"] == 5:
            row["gap"] = 1e-12
            row["sublevel_radius_E1"] = 1e6
    with contextlib.ExitStack() as stack:
        for name in ("V1", "V2", "V3", "V4", "V5"):
            stack.enter_context(patch.object(lattice, name.lower(), validation(name)))
        stack.enter_context(patch.object(lattice, "physics", lambda *args: table))
        reflection = copy.deepcopy(frozen["P5_reflection"])
        reflection["pass_"] = reflection["pass"]
        stack.enter_context(patch.object(lattice, "reflection", lambda: reflection))
        receipt = submitted_main(lattice, out / "collapsed-gap-reducer-receipt.json")
    assert receipt["ok"] and receipt["P2_volume"]["pass"]
    return {
        "fixture_kind": "committed fixture replay with deliberate corruption, NOT measured physics",
        "failed_validation_still_enters_physics": bool(invalid_entered_physics),
        "collapsed_gap": {"injected_p5_gap": 1e-12, "submitted_receipt_ok": receipt["ok"],
                          "volume_check_pass": receipt["P2_volume"]["pass"]},
    }


def winding_control():
    """Kinematic spin-parity control, not the preregistered finite-coupling study."""
    ring = ks.SpinNetwork(ks.ladder(3, periodic=True), .5)
    ring.hamiltonian(1.)  # Assembly only; no full-H eigenstates or profiles measured.
    sectors = np.array([(-1) ** round(2 * (s[0] + s[3])) for s in ring.states])
    cross = max(float(abs(T.toarray()[np.ix_(sectors == 1, sectors == -1)]).max())
                for T in ring.plaquette_ops)
    E = .5 * ring.electric()  # coefficient of g^2 in H_E, not the full Hamiltonian
    even = E[sectors == 1]
    odd = E[sectors == -1]
    assert cross == 0 and np.min(odd) == 9/8 and np.min(even[even > 0]) == 3/2
    return {"p": 3, "jmax": .5, "states": len(ring.states),
            "sector_counts": {"even": int(sum(sectors == 1)), "odd": int(sum(sectors == -1))},
            "max_cross_sector_plaquette_entry": cross,
            "electric_odd_min_over_g_squared": float(np.min(odd)),
            "electric_even_nonzero_min_over_g_squared": float(np.min(even[even > 0])),
            "scope": "electric-only and selection-rule control; no finite-coupling ring gap claim"}


def run(out):
    source_hashes = {}
    for rel in SOURCE_PATHS:
        expected = subprocess.check_output(["git", "show", f"{PIN}:{rel}"], cwd=ROOT)
        actual = (ROOT / rel).read_bytes().replace(b"\r\n", b"\n")
        assert actual == expected.replace(b"\r\n", b"\n"), f"source differs: {rel}"
        source_hashes[rel] = hashlib.sha256(expected).hexdigest()
    frozen = json.loads((ROOT / "research/ym-geometry-20261123/lattice_tests.json").read_text())
    baseline = submitted_main(single, out / "plaquette-baseline.json")
    assert baseline["ok"]
    validations = {}
    for name in ("v1", "v2", "v3", "v4", "v5"):
        start = time.monotonic()
        validations[name] = getattr(lattice, name)()
        assert validations[name]["pass_"]
        print("REPLAY", name, "seconds", round(time.monotonic() - start, 2), flush=True)
    table = lattice.physics([(1, (1., 1.5)), (2, (1., 1.5)), (3, (1.,))], (1., 2.))
    with warnings.catch_warnings(record=True) as numerical_warnings:
        warnings.simplefilter("always")
        reflection = lattice.reflection()
    assert reflection["pass_"] and reflection["gate_negative_adjoint"]["caught"]
    result = {
        "reviewed_source": PIN,
        "lean_source_inspected_not_rebuilt": "c06bed173d937263eae4b26c32644b11c0c59789",
        "environment": {"python": platform.python_version(), "numpy": np.__version__,
                        "scipy": scipy.__version__, "sympy": sympy.__version__,
                        "threads": {key: os.environ.get(key) for key in
                                    ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS", "MKL_NUM_THREADS")}},
        "source_sha256_git_blobs": source_hashes,
        "replayed_validations": validations,
        "bounded_new_ladder_rows": table,
        "replayed_reflection": reflection,
        "reflection_warnings": [str(item.message) for item in numerical_warnings],
        "earlier_gate_witnesses": old_gate_witnesses(out),
        "small_counterexamples": exact_small_controls(),
        "lattice_reducer_witnesses": lattice_reducer_witnesses(out, frozen),
        "winding_control": winding_control(),
        "scope_exclusions": ["large p=4,5 production rerun", "new preregistered excitation profiles or scores",
                             "Lean rebuild", "Rust engine suite", "live GIGI server", "continuum proof"],
        "status": "ADVERSARIAL_FINDINGS_REPRODUCED_NOT_SCIENTIFIC_GATE_PASS",
    }
    save(out / "review.json", result)
    return result


if __name__ == "__main__":
    destination = Path(sys.argv[1]).resolve()
    destination.mkdir(parents=True, exist_ok=True)
    save(destination / "review.json", {"status": "RUNNING", "reviewed_source": PIN})
    try:
        with (destination / "replay.log").open("w", encoding="utf-8") as log:
            with contextlib.redirect_stdout(log), contextlib.redirect_stderr(log):
                result = run(destination)
        result["hash_contract"] = "SHA256 of text with CRLF normalized to LF, matching Git blobs"
        result["audit_script_sha256"] = text_digest(Path(__file__))
        result["evidence_sha256"] = {
            name: text_digest(destination / name)
            for name in ("replay.log", "plaquette-baseline.json", "wrong-shift-submitted-receipt.json",
                         "stale-submitted-receipt.json", "collapsed-gap-reducer-receipt.json")
        }
        prior = destination / "prior-rtc-replay-63588263.json"
        if prior.exists():
            result["prior_cell_receipt"] = {"origin": "earlier 63588263 replay, not rerun here",
                                            "sha256": text_digest(prior)}
        save(destination / "review.json", result)
        print(result["status"], flush=True)
    except BaseException as error:
        save(destination / "review.json", {"status": "AUDIT_FAILED", "error": repr(error), "reviewed_source": PIN})
        raise
