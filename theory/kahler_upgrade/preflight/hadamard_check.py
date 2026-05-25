"""
L8.2 — Hadamard pre-flight (catalog §E.5 check 1).

PURPOSE
    Verify that Marcella's actual embedding manifold satisfies the
    Hadamard condition: along randomly sampled geodesics, the
    Jacobi field never vanishes (no conjugate points). This is the
    precondition for catalog §1.4 ideal-boundary application and
    §1.5 global invertibility theorems Marcella v3 cites.

GROUND TRUTH
    Solve J'' + K(t) · J = 0 along each sampled geodesic with
    J(0) = 0, J'(0) = 1. Check that J(t) > 0 for all t > 0 in the
    test radius.

    - K = 0 (flat): J(t) = t, always positive.
    - K = -1 (hyperbolic ℍ²): J(t) = sinh(t), always positive.
    - K = +1 (spherical S²): J(t) = sin(t), zero at t = π
      ⇒ conjugate point, NOT Hadamard.

USAGE FROM MARCELLA
    Replace `sample_curvature_along_geodesic()` with your runtime
    surface — e.g., call `bundle.kahler_curvature().holo_sectional`
    repeatedly along a sampled token-pair geodesic; or, if you have
    a richer curvature estimator, use that.

    Then this script's existing logic does the Jacobi integration
    + non-vanishing check. The synthetic controls below prove the
    integrator + check fire correctly on known-positive (ℍ²) and
    known-negative (S²) cases.

CALL FROM CI
    PYTHONIOENCODING=utf-8 python -X utf8 hadamard_check.py
    Exits 0 on pass, 1 on fail.
"""

import math
import random
import sys


# ============================================================
# Jacobi-field integrator (matches src/cost/jacobi_estimator.rs)
# ============================================================

def jacobi_field(k_at_t, t_max, n_steps):
    """
    Solve J'' + K(t) · J = 0 via RK4 with J(0) = 0, J'(0) = 1.
    Returns (times, J_values, first_conjugate_t_or_None).

    Integrates slightly past `t_max` (one extra step) so that
    conjugate points landing exactly at the boundary (e.g. sin(t)
    zero at t = π) fire as detected zeros rather than slipping
    past the loop.
    """
    dt = t_max / n_steps
    # Detection tolerance — J(t) is treated as effectively zero
    # when below this. RK4 of sin hits ~1e-11 at the analytic zero;
    # this catches it without false positives on small numerical
    # noise far from a zero.
    eps_j = 1e-6
    times = [0.0]
    j_values = [0.0]
    j = 0.0
    jp = 1.0
    first_conj = None

    def deriv(_t, y_j, y_jp, k_val):
        return (y_jp, -k_val * y_j)

    # Integrate one extra step past t_max to catch boundary zeros.
    for step in range(n_steps + 1):
        t = step * dt
        k_mid = k_at_t((t + dt / 2.0))

        # RK4 on (J, J')
        k_t = k_at_t(t)
        k1_j, k1_jp = deriv(t, j, jp, k_t)

        k2_j, k2_jp = deriv(t + dt / 2.0, j + dt / 2.0 * k1_j,
                            jp + dt / 2.0 * k1_jp, k_mid)
        k3_j, k3_jp = deriv(t + dt / 2.0, j + dt / 2.0 * k2_j,
                            jp + dt / 2.0 * k2_jp, k_mid)
        k4_j, k4_jp = deriv(t + dt, j + dt * k3_j,
                            jp + dt * k3_jp, k_at_t(t + dt))

        j_new = j + dt / 6.0 * (k1_j + 2 * k2_j + 2 * k3_j + k4_j)
        jp_new = jp + dt / 6.0 * (k1_jp + 2 * k2_jp + 2 * k3_jp + k4_jp)

        # Detect conjugate point: J went through zero OR is touching
        # zero from above (boundary case). The `eps_j` threshold
        # catches the sin(t) at t = π case where the analytic value
        # is exactly 0 but RK4 accumulated error keeps it strictly
        # positive (so a strict sign-flip never fires).
        if first_conj is None and step > 0 and (
            (j * j_new) <= 0.0 or (j > eps_j and abs(j_new) < eps_j)
        ):
            crossing_t = t + dt  # conservative report
            if j_new != j:
                frac = abs(j) / abs(j_new - j)
                crossing_t = t + frac * dt
            # Only flag if the crossing lies within [0, t_max] —
            # we extended one step past so the integrator can SEE
            # the boundary zero, but we only call it a conjugate
            # if it would fire inside the test radius.
            if crossing_t <= t_max + dt / 2.0:
                first_conj = crossing_t

        j = j_new
        jp = jp_new
        times.append(t + dt)
        j_values.append(j)

    return times, j_values, first_conj


# ============================================================
# Hadamard pre-flight check
# ============================================================

def hadamard_check(
    sample_curvature_along_geodesic,
    n_samples=1000,
    t_max=math.pi,
    n_steps=2000,
    seed=0xDEADBEEF,
):
    """
    Sample `n_samples` random geodesics on Marcella's embedding
    manifold and verify the Jacobi field never vanishes on any of
    them inside `t_max`. Default `t_max = π` matches GIGI's
    `HADAMARD_TEST_RADIUS` so the canonical S² conjugate at π
    fires.

    `sample_curvature_along_geodesic(seed) -> callable(t) -> float`
        Returns a function that, given t in [0, t_max], reports the
        sectional curvature at parameter t along the seed-th
        sampled geodesic. In Marcella's runtime this calls
        `bundle.kahler_curvature().holo_sectional` (or a per-token-
        pair refinement).

    Returns (passes, failure_fraction, sample_failures).
    """
    random.seed(seed)
    n_failures = 0
    sample_failures = []
    for i in range(n_samples):
        k_at_t = sample_curvature_along_geodesic(i)
        _, _, conj = jacobi_field(k_at_t, t_max, n_steps)
        if conj is not None:
            n_failures += 1
            if len(sample_failures) < 5:
                sample_failures.append((i, conj))
    failure_fraction = n_failures / n_samples
    passes = n_failures == 0
    return passes, failure_fraction, sample_failures


# ============================================================
# Synthetic controls (verify the check itself works)
# ============================================================

def control_hyperbolic():
    """Positive control: K = -1 (ℍ²) ⇒ Jacobi = sinh(t), no zeros."""
    print("\n=== Control: hyperbolic ℍ² (K = -1) ===")
    f = lambda _i: (lambda _t: -1.0)
    passes, frac, fails = hadamard_check(f, n_samples=100)
    print(f"  passes = {passes}, failure_fraction = {frac:.4f}, "
          f"first failures = {fails}")
    assert passes, "hyperbolic control must pass — Hadamard signal incorrect"
    print("  PASS")


def control_spherical():
    """Negative control: K = +1 (S²) ⇒ Jacobi = sin(t), zero at π."""
    print("\n=== Control: spherical S² (K = +1) ===")
    f = lambda _i: (lambda _t: 1.0)
    passes, frac, fails = hadamard_check(f, n_samples=100)
    print(f"  passes = {passes}, failure_fraction = {frac:.4f}, "
          f"first failures = {fails}")
    # Sin(t) zero at t = π → integrator detects it on every sample.
    assert not passes, "spherical control must FAIL — conjugate point at π"
    assert frac > 0.9, f"spherical: expected ≥ 90% failure; got {frac}"
    print("  PASS (correctly failed)")


def control_flat():
    """Edge case: K = 0 (flat ℝⁿ) ⇒ Jacobi = t, never zero in (0, t_max]."""
    print("\n=== Control: flat ℝⁿ (K = 0) ===")
    f = lambda _i: (lambda _t: 0.0)
    passes, frac, fails = hadamard_check(f, n_samples=100)
    print(f"  passes = {passes}, failure_fraction = {frac:.4f}, "
          f"first failures = {fails}")
    assert passes, "flat control must pass — Jacobi = t never zero"
    print("  PASS")


# ============================================================
# Entry point
# ============================================================

if __name__ == "__main__":
    print("L8.2 Hadamard pre-flight — synthetic controls")
    print("=" * 60)
    failures = []
    for name, fn in [
        ("hyperbolic", control_hyperbolic),
        ("spherical", control_spherical),
        ("flat", control_flat),
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
    print("\nNEXT: implement `sample_curvature_along_geodesic` from your")
    print("Marcella runtime and call `hadamard_check(your_sampler)`.")
    print("That call must return (True, 0.0, []) to clear pre-flight 1.")
    sys.exit(0)
