"""Exact preliminary checks for the Helike quantum-probe specification.

This does not construct a quantum Hamiltonian matrix or measure an excitation.
Run with --inject-defect to verify that the corresponding false identity fails.
"""

import argparse
import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path

import sympy as sp


def is_zero(value):
    entries = list(value) if isinstance(value, sp.MatrixBase) else [value]
    return all(sp.simplify(entry) == 0 for entry in entries)


def check_contract(defect):
    x = sp.symbols("x0:9", real=True)
    matrix = sp.Matrix(3, 3, x)
    gram = matrix * matrix.T
    root2 = sp.sqrt(2)
    cosine = sp.diag(0, 1, -1) / root2
    sine = sp.Matrix([[0, 0, 0], [0, 0, 1], [0, 1, 0]]) / root2
    if defect == "flipped_imaginary":
        sine = -sine
    tensors = [
        cosine,
        sine,
        sp.Matrix([[0, 1, 0], [1, 0, 0], [0, 0, 0]]) / root2,
        sp.Matrix([[0, 0, 1], [0, 0, 0], [1, 0, 0]]) / root2,
        sp.diag(-2, 1, 1) / sp.sqrt(6),
    ]

    cubic = 0 if defect == "drop_cubic" else -6 * matrix.det()
    scaled_potential = (
        sp.trace(gram) + cubic
        + (sp.trace(gram)**2 - sp.trace(gram * gram)) / 2
    ) / 2  # g^2 V_g, with no coupling or floating-point substitution.
    local = matrix if defect == "missing_frame_scale" else 2 * matrix
    local_energy = sp.Integer(0)
    for i in range(3):
        for j in range(i + 1, 3):
            curvature = local.row(i).T.cross(local.row(j).T)
            for k in range(3):
                curvature -= 2 * sp.LeviCivita(i, j, k) * local.row(k).T
            local_energy += curvature.dot(curvature) / 2

    checks = {"all_entry_energy_map": is_zero(local_energy - 16 * scaled_potential)}
    cofactor = matrix.cofactor_matrix()
    checks["published_potential_as_curvature_square"] = is_zero(
        scaled_potential - sum(v*v for v in matrix - cofactor) / 2
    )
    paper_vector = sp.Matrix([0, 1, sp.I])
    checks["paper_complex_orientation"] = is_zero(
        (cosine + sp.I * sine) / root2 - paper_vector * paper_vector.T / 2
    )
    checks["orthonormal_tensor_basis"] = all(
        is_zero(sp.trace(a.T * b) - int(i == j))
        for i, a in enumerate(tensors) for j, b in enumerate(tensors)
    )

    background = dict(zip(x, list(sp.eye(3) / 2)))
    hessian = sp.hessian(scaled_potential, x).subs(background)
    for name, tensor in (("cosine", cosine), ("sine", sine)):
        checks[f"classical_hessian_{name}"] = is_zero(
            hessian * sp.Matrix(list(tensor))
            - sp.Rational(5, 2) * sp.Matrix(list(tensor))
        )
        observable = sp.trace(tensor * gram)
        if defect == "coloured_probe":
            observable = sp.trace(tensor.T * matrix)
        checks[f"observable_gradient_{name}"] = is_zero(sp.Matrix([
            sp.diff(observable, xi).subs(background) - component
            for xi, component in zip(x, list(tensor))
        ]))
        # Exact infinitesimal right-colour action checks all three generators.
        for axis in range(3):
            generator = sp.Matrix(3, 3, lambda a, b: sp.LeviCivita(a, axis, b))
            tangent = -matrix * generator
            gauss_action = sum(
                sp.diff(observable, xi) * component
                for xi, component in zip(x, list(tangent))
            )
            checks[f"gauss_{name}_{axis}"] = is_zero(gauss_action)

    checks["scalar_orbit_identity"] = is_zero(
        sum(sp.trace(tensor * gram)**2 for tensor in tensors)
        - sp.trace(gram * gram) + sp.trace(gram)**2 / 3
    )
    return checks


def publish(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    pending = path.with_suffix(path.suffix + ".pending")
    pending.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    pending.replace(path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--inject-defect", choices=(
        "missing_frame_scale", "drop_cubic", "flipped_imaginary", "coloured_probe"
    ))
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    note = root / "docs/research/helike-quantum-probe-preregistration-2026-11-23.md"
    receipt = {
        "status": "RUNNING",
        "scope": "preliminary_exact_algebra_only",
        "quantum_spectrum_status": "NOT_RUN",
        "created_utc": datetime.now(timezone.utc).isoformat(),
        "injected_defect": args.inject_defect,
        "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "spec_sha256": hashlib.sha256(note.read_bytes()).hexdigest(),
        "sympy_version": sp.__version__,
    }
    if args.out:
        publish(args.out, receipt)
    try:
        checks = check_contract(args.inject_defect)
        receipt.update(checks=checks, status="PASS" if all(checks.values()) else "FAIL")
    except Exception as error:
        receipt.update(status="FAIL", error=f"{type(error).__name__}: {error}")
    if args.out:
        publish(args.out, receipt)
    print(json.dumps(receipt, indent=2))
    return 0 if receipt["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
