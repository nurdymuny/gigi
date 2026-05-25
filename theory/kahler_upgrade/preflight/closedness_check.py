"""
L8.3 — Closedness pre-flight (catalog §E.5 check 2).

PURPOSE
    Verify that Marcella's attached 2-form B satisfies dB = 0
    everywhere on the embedding manifold. This is the precondition
    for the magnetic geodesic equation (catalog §1.2) to be
    well-posed and for L7.2's quantized holonomy debt to be
    topologically protected.

GROUND TRUTH (discrete dB)
    For B given as a function `b(p) -> antisymmetric n×n matrix`
    over base points p:

        (dB)_{ijk} = ∂_i B_{jk} + ∂_j B_{ki} + ∂_k B_{ij}

    For constant B this is identically zero. For non-constant B we
    approximate the partial derivatives by central finite
    differences on a small ε-mesh and check the max-abs entry of
    the resulting 3-tensor.

    Pass: ‖dB‖_∞ < tolerance (default 1e-10, matching GIGI's
    `ClosedTwoForm::new_with_discrete_d` invariant).

USAGE FROM MARCELLA
    Replace `sample_b_at_point()` with your runtime's B accessor.
    The check then exercises dB at a sample of base points on
    Marcella's manifold.

CALL FROM CI
    PYTHONIOENCODING=utf-8 python -X utf8 closedness_check.py
"""

import math
import random
import sys


# ============================================================
# Discrete exterior derivative (matches ClosedTwoForm::new_with_discrete_d)
# ============================================================

def discrete_db_max_abs(b_at_point, base_point, eps, dim):
    """
    Compute ‖dB‖_∞ at `base_point` using central finite
    differences with step `eps`. Returns the max-abs entry of the
    3-tensor (dB)_{ijk}.
    """
    max_err = 0.0
    for i in range(dim):
        for j in range(i + 1, dim):
            for k in range(j + 1, dim):
                # ∂_i B_{jk} via central difference along axis i.
                def partial_d(axis, comp_a, comp_b):
                    p_plus = list(base_point)
                    p_minus = list(base_point)
                    p_plus[axis] += eps
                    p_minus[axis] -= eps
                    b_plus = b_at_point(p_plus)
                    b_minus = b_at_point(p_minus)
                    return (b_plus[comp_a][comp_b] - b_minus[comp_a][comp_b]) / (2 * eps)

                pi_jk = partial_d(i, j, k)
                pj_ki = partial_d(j, k, i)
                pk_ij = partial_d(k, i, j)
                db_entry = pi_jk + pj_ki + pk_ij
                if abs(db_entry) > max_err:
                    max_err = abs(db_entry)
    return max_err


def closedness_check(
    sample_b_at_point,
    sample_base_points,
    eps=1e-4,
    tolerance=1e-10,
    dim=2,
):
    """
    Returns (passes, worst_violation, worst_point).
    """
    worst = 0.0
    worst_pt = None
    for p in sample_base_points:
        err = discrete_db_max_abs(sample_b_at_point, p, eps, dim)
        if err > worst:
            worst = err
            worst_pt = p
    return (worst < tolerance, worst, worst_pt)


# ============================================================
# Synthetic controls
# ============================================================

def control_constant_b_passes():
    """Constant B has dB ≡ 0 exactly. Closedness check must pass."""
    print("\n=== Control: constant B = 0.5·dx∧dy ===")
    # Constant 2x2 antisymmetric matrix.
    const_b = [[0.0, 0.5], [-0.5, 0.0]]
    sampler = lambda _p: const_b
    points = [(random.random(), random.random()) for _ in range(50)]
    passes, worst, pt = closedness_check(sampler, points, dim=2)
    print(f"  passes = {passes}, worst = {worst:.2e}, point = {pt}")
    assert passes, "constant B must pass closedness"
    assert worst < 1e-10, f"constant B worst = {worst} should be ≈ 0"
    print("  PASS")


def control_position_dependent_b_can_fail():
    """
    B(x, y) = (x · y) · dx ∧ dy. ∂_x B = y, ∂_y B = x. In 2D the
    exterior derivative of a 2-form is automatically zero (no
    3-form on a 2-manifold) — so this case actually passes too.
    To get a real dB ≠ 0 we need dim ≥ 3.
    """
    print("\n=== Control: position-dependent B in 2D (still passes) ===")
    sampler = lambda p: [[0.0, p[0] * p[1]], [-(p[0] * p[1]), 0.0]]
    points = [(random.random(), random.random()) for _ in range(20)]
    passes, worst, pt = closedness_check(sampler, points, dim=2)
    print(f"  passes = {passes}, worst = {worst:.2e}, point = {pt}")
    # In 2D, dim < 3 ⇒ no (i, j, k) triple to sum over ⇒ worst = 0.
    assert passes, "2D position-dependent B is trivially closed"
    print("  PASS (correctly noted: dim < 3 ⇒ dB has no entries)")


def control_3d_non_closed_b_fails():
    """
    In 3D, B = z · dx ∧ dy gives dB = dz ∧ dx ∧ dy ≠ 0. The
    closedness check must FAIL on this — exercises the negative
    path so Marcella's CI catches a non-closed B.
    """
    print("\n=== Control: 3D non-closed B = z · dx ∧ dy ===")
    def sampler(p):
        z = p[2]
        return [
            [0.0, z, 0.0],
            [-z, 0.0, 0.0],
            [0.0, 0.0, 0.0],
        ]
    points = [
        (random.uniform(-1, 1), random.uniform(-1, 1), random.uniform(-1, 1))
        for _ in range(20)
    ]
    passes, worst, pt = closedness_check(sampler, points, dim=3)
    print(f"  passes = {passes}, worst = {worst:.2e}, point = {pt}")
    # ∂_z B_{xy} = 1, ∂_x B_{yz} = 0, ∂_y B_{zx} = 0 ⇒ dB = 1 ≠ 0.
    assert not passes, "non-closed B in 3D must fail"
    assert abs(worst - 1.0) < 1e-3, f"expected ‖dB‖ ≈ 1; got {worst}"
    print("  PASS (correctly failed)")


# ============================================================
# Entry point
# ============================================================

if __name__ == "__main__":
    random.seed(0xDEADBEEF)
    print("L8.3 Closedness pre-flight — synthetic controls")
    print("=" * 60)
    failures = []
    for name, fn in [
        ("constant_b_passes", control_constant_b_passes),
        ("position_dependent_2d_passes", control_position_dependent_b_can_fail),
        ("3d_non_closed_fails", control_3d_non_closed_b_fails),
    ]:
        try:
            fn()
        except AssertionError as e:
            failures.append((name, str(e)))

    if failures:
        print("\nFAILURES:")
        for n, e in failures:
            print(f"  {n}: {e}")
        sys.exit(1)
    print("\nAll synthetic controls passed.")
    print("\nNEXT: implement `sample_b_at_point` from your Marcella")
    print("runtime, supply a list of base points on your embedding")
    print("manifold, and call `closedness_check(b, points, dim=...)`.")
    print("That call must return (True, < 1e-10, _) to clear pre-flight 2.")
    sys.exit(0)
