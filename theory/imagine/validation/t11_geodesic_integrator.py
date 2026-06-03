"""
T11: Geodesic integrator math validation.

================================================================================
CLAIM (IMAGINE_AND_WALK.md §1 + §7 T11):
    The RK4 geodesic integrator on closed-form analytic manifolds
    (S^2 unit sphere with stereographic chart, T^2 flat, CP^1
    Fubini-Study with stereographic chart) reproduces the closed-form
    geodesics to machine precision.

    The integrator works in chart coordinates -- it consumes the
    metric's conformal factor and its derivatives, computes the
    Christoffel symbols, and integrates the geodesic ODE via 4th-order
    Runge-Kutta. The result is then compared to the closed-form
    geodesic computed by an INDEPENDENT path (embedded picture for
    S^2 / CP^1; linear flow for T^2).

REFERENCES:
    - do Carmo, *Riemannian Geometry*, §3.2 (geodesic equation).
    - Lee, *Introduction to Smooth Manifolds*, Ch. 14 (Christoffel
      symbols on conformally flat metrics).
    - poincare_to_sharding.md T3 §3.3 (closed-form CP^1 Fubini-Study
      curvature reference -- K = 4).
    - IMAGINE_AND_WALK.md §7 (this test's spec entry).

GROUND TRUTH (independent of integrator):
    S^2: parameterize the unit sphere in R^3, integrate great circles
         analytically:
         gamma_3D(t) = cos(omega*t) * P + sin(omega*t) * V_normalized
         where omega = |V_3D| is the 3D tangent speed at the seed.
         Then project gamma_3D(t) back to stereographic coordinates
         via sigma(X,Y,Z) = (X/(1-Z), Y/(1-Z)).

    CP^1 Fubini-Study: stereographic chart is the same manifold S^2
         topologically, but with metric rho = 1/(1+r^2)^2 (vs S^2's
         rho = 4/(1+r^2)^2). The geodesics are still great circles
         on the embedded sphere; what changes is the proper-time
         parameterization (the |V| in the closed form is scaled by
         sqrt(rho/rho_S2) = 1/2).

    T^2 flat: straight lines in (x, y).
         gamma(t) = (x0 + vx*t, y0 + vy*t)

TEST DESIGN:
    For each of {S^2, T^2, CP^1}:
        1. Pick a seed point (x0, y0) and initial tangent (vx, vy)
           in chart coordinates.
        2. Integrate the geodesic ODE forward to t = 1 using RK4
           with N = 1000 steps. Record state at t in {0.1, 0.5, 1.0}.
        3. Compute the closed-form geodesic at the same t values
           via the INDEPENDENT path.
        4. Assert |integrated - closed_form| < 1e-9 in chart coords.

PASS CRITERION:
    Across all three manifolds and three checkpoint times:
        max error < 1e-9
    With N=1000 RK4 steps, RK4 truncation error is ~O(h^4) = O(1e-12)
    per step, ~O(1e-9) cumulative -- comfortably within tolerance.

CIRCULAR-LOGIC GUARDS:
    1. The integrator consumes ONLY the conformal factor rho(x,y) and
       its log-derivatives phi_x, phi_y. It NEVER sees the closed-form
       geodesic.
    2. The closed-form is computed via an INDEPENDENT path:
       - S^2 / CP^1: embedded picture in R^3, great-circle formula,
         then project back to stereographic coords.
       - T^2: trivial linear flow.
       Neither path uses the Christoffel symbols the integrator uses.
    3. For S^2 and CP^1, the seed is chosen so the closed-form path
       stays inside the chart for t in [0, 1] (avoids the projection
       singularity at the antipode).
================================================================================
"""

from __future__ import annotations
import math
import sys
from dataclasses import dataclass
from typing import Callable, Tuple

import numpy as np


# ============================================================================
# Conformally flat metric primitives
# ============================================================================
#
# A conformally flat 2D metric is ds^2 = exp(2*phi) (dx^2 + dy^2),
# parameterized by phi(x, y). For S^2 stereographic, exp(2*phi) =
# 4 / (1 + r^2)^2. For CP^1 Fubini-Study, exp(2*phi) = 1 / (1 + r^2)^2.
# Both give phi_x = -2x/(1+r^2), phi_y = -2y/(1+r^2). The Christoffels
# computed from phi alone differ only in their relationship to the
# proper-time parameterization, not their formula.
#
# Christoffel symbols for a conformally flat 2D metric:
#   Gamma^x_xx = phi_x,   Gamma^x_xy = phi_y,   Gamma^x_yy = -phi_x
#   Gamma^y_xx = -phi_y,  Gamma^y_xy = phi_x,   Gamma^y_yy = phi_y
#
# Geodesic equation:
#   x'' + Gamma^x_jk x'^j x'^k = 0
#   y'' + Gamma^y_jk x'^j x'^k = 0
# Expanded:
#   x'' = -phi_x ((x')^2 - (y')^2) - 2*phi_y * x' * y'
#   y'' = +phi_y ((x')^2 - (y')^2) - 2*phi_x * x' * y'


@dataclass
class ConformallyFlatMetric:
    """A conformally flat 2D metric specified by phi and its derivatives."""

    phi_x: Callable[[float, float], float]
    phi_y: Callable[[float, float], float]

    def geodesic_acceleration(self, x: float, y: float, vx: float, vy: float) -> Tuple[float, float]:
        """The right-hand side of the geodesic ODE: (x'', y'')."""
        px = self.phi_x(x, y)
        py = self.phi_y(x, y)
        diff = vx * vx - vy * vy
        cross = 2.0 * vx * vy
        ax = -px * diff - py * cross
        ay = +py * diff - px * cross
        return ax, ay


# Stereographic charts on S^2 and CP^1 share the same phi_x, phi_y
# (the conformal-factor constant doesn't enter derivatives of log phi).
def _sphere_phi_x(x: float, y: float) -> float:
    return -2.0 * x / (1.0 + x * x + y * y)


def _sphere_phi_y(x: float, y: float) -> float:
    return -2.0 * y / (1.0 + x * x + y * y)


S2_METRIC = ConformallyFlatMetric(phi_x=_sphere_phi_x, phi_y=_sphere_phi_y)
CP1_METRIC = ConformallyFlatMetric(phi_x=_sphere_phi_x, phi_y=_sphere_phi_y)


# T^2 flat metric: phi = 0 everywhere, so all Christoffels vanish.
def _zero(x: float, y: float) -> float:
    return 0.0


T2_METRIC = ConformallyFlatMetric(phi_x=_zero, phi_y=_zero)


# ============================================================================
# RK4 geodesic integrator (the math claim under test)
# ============================================================================


def integrate_geodesic(
    metric: ConformallyFlatMetric,
    x0: float, y0: float, vx0: float, vy0: float,
    t_end: float, n_steps: int,
) -> list[Tuple[float, float, float, float, float]]:
    """
    Integrate the geodesic ODE from (x0, y0) with tangent (vx0, vy0)
    using 4th-order Runge-Kutta on the (x, y, vx, vy) state vector.

    Returns: list of (t, x, y, vx, vy) at every step.

    Circular-logic guard #1: the integrator consumes ONLY `metric`
    (a `ConformallyFlatMetric` providing phi_x, phi_y). It does not
    see the closed-form geodesic.
    """
    h = t_end / n_steps
    state = (x0, y0, vx0, vy0)
    trajectory = [(0.0, *state)]

    def f(s: Tuple[float, float, float, float]) -> Tuple[float, float, float, float]:
        x, y, vx, vy = s
        ax, ay = metric.geodesic_acceleration(x, y, vx, vy)
        return (vx, vy, ax, ay)

    def step(s: Tuple[float, float, float, float]) -> Tuple[float, float, float, float]:
        k1 = f(s)
        s2 = tuple(s[i] + 0.5 * h * k1[i] for i in range(4))
        k2 = f(s2)
        s3 = tuple(s[i] + 0.5 * h * k2[i] for i in range(4))
        k3 = f(s3)
        s4 = tuple(s[i] + h * k3[i] for i in range(4))
        k4 = f(s4)
        return tuple(
            s[i] + (h / 6.0) * (k1[i] + 2 * k2[i] + 2 * k3[i] + k4[i])
            for i in range(4)
        )

    for k in range(1, n_steps + 1):
        state = step(state)
        trajectory.append((k * h, *state))
    return trajectory


# ============================================================================
# Closed-form geodesics (independent ground truth)
# ============================================================================


def s2_stereographic_to_embedded(x: float, y: float) -> np.ndarray:
    """Inverse stereographic from north pole onto the unit sphere."""
    s = 1.0 + x * x + y * y
    return np.array([2.0 * x / s, 2.0 * y / s, (x * x + y * y - 1.0) / s])


def s2_embedded_to_stereographic(P: np.ndarray) -> Tuple[float, float]:
    """Stereographic projection from north pole."""
    X, Y, Z = float(P[0]), float(P[1]), float(P[2])
    denom = 1.0 - Z
    if denom < 1e-12:
        raise ValueError("at or near north pole; geodesic left the chart")
    return X / denom, Y / denom


def s2_stereographic_tangent_to_3d(x: float, y: float, vx: float, vy: float) -> np.ndarray:
    """
    Pushforward of a 2D tangent vector at (x, y) in stereographic coords
    to a 3D tangent vector at the corresponding sphere point.

    Uses the Jacobian of sigma_inv:
        sigma_inv(x, y) = (2x, 2y, x^2+y^2-1) / (x^2+y^2+1)
    Partial derivatives give the basis for the tangent plane at P.
    """
    r2 = x * x + y * y
    s = 1.0 + r2
    # d/dx sigma_inv
    dPx = np.array([
        (2.0 * s - 2.0 * x * (2.0 * x)) / (s * s),
        -4.0 * x * y / (s * s),
        (2.0 * x * s - (r2 - 1.0) * 2.0 * x) / (s * s),
    ])
    # d/dy sigma_inv
    dPy = np.array([
        -4.0 * x * y / (s * s),
        (2.0 * s - 2.0 * y * (2.0 * y)) / (s * s),
        (2.0 * y * s - (r2 - 1.0) * 2.0 * y) / (s * s),
    ])
    return vx * dPx + vy * dPy


def s2_closed_form_geodesic(
    x0: float, y0: float, vx0: float, vy0: float, t: float
) -> Tuple[float, float]:
    """
    Closed-form geodesic on the unit S^2:
        gamma_3D(t) = cos(omega*t) * P0 + sin(omega*t) * V_normalized
        where P0 = sigma_inv(x0, y0)
              V = pushforward tangent in 3D
              omega = |V|_3D
    Project back to stereographic coords.

    Circular-logic guard #2: this function uses the EMBEDDED picture
    (R^3 + standard inner product) and does NOT touch any Christoffel
    symbol from the integrator's metric.
    """
    P0 = s2_stereographic_to_embedded(x0, y0)
    V = s2_stereographic_tangent_to_3d(x0, y0, vx0, vy0)
    omega = float(np.linalg.norm(V))
    if omega < 1e-12:
        return x0, y0
    V_hat = V / omega
    P_t = np.cos(omega * t) * P0 + np.sin(omega * t) * V_hat
    return s2_embedded_to_stereographic(P_t)


def t2_closed_form_geodesic(
    x0: float, y0: float, vx0: float, vy0: float, t: float
) -> Tuple[float, float]:
    """Flat torus: straight lines."""
    return x0 + vx0 * t, y0 + vy0 * t


def cp1_closed_form_geodesic(
    x0: float, y0: float, vx0: float, vy0: float, t: float
) -> Tuple[float, float]:
    """
    Closed-form geodesic on CP^1 Fubini-Study, stereographic chart.

    The Christoffel symbols are identical to S^2's (because the
    conformal factor's CONSTANT is what differs, and constants don't
    enter phi_x / phi_y). So the integrator output for CP^1 with the
    same seed + tangent is identical to S^2's integrator output.

    The 3D embedded picture changes only by an overall metric scale:
    on CP^1 FS with rho = 1/(1+r^2)^2, the "proper time" omega computed
    from the embedded sphere's tangent speed differs from S^2 by a
    constant factor sqrt(1/4) = 1/2. The geodesic equation in the
    chart is the same; the time scale is the same in CHART time. So
    the closed form in chart coords IS the same as S^2.

    (Mathematically: both manifolds are conformally equivalent to the
    sphere; geodesics in the conformal class depend only on the
    conformal structure when expressed in chart coordinates.)
    """
    return s2_closed_form_geodesic(x0, y0, vx0, vy0, t)


# ============================================================================
# Test cases
# ============================================================================


@dataclass
class TestCase:
    name: str
    metric: ConformallyFlatMetric
    closed_form: Callable[[float, float, float, float, float], Tuple[float, float]]
    seed: Tuple[float, float, float, float]  # (x0, y0, vx0, vy0)


def make_cases() -> list[TestCase]:
    return [
        TestCase(
            name="S^2 stereographic, small tangent",
            metric=S2_METRIC,
            closed_form=s2_closed_form_geodesic,
            seed=(0.0, 0.0, 0.3, 0.0),  # seed at chart origin, tangent in +x
        ),
        TestCase(
            name="S^2 stereographic, off-center seed",
            metric=S2_METRIC,
            closed_form=s2_closed_form_geodesic,
            seed=(0.2, 0.3, 0.4, -0.2),
        ),
        TestCase(
            name="T^2 flat, axis-aligned",
            metric=T2_METRIC,
            closed_form=t2_closed_form_geodesic,
            seed=(0.1, 0.2, 0.5, 0.0),
        ),
        TestCase(
            name="T^2 flat, diagonal",
            metric=T2_METRIC,
            closed_form=t2_closed_form_geodesic,
            seed=(-0.4, 0.1, 0.3, 0.4),
        ),
        TestCase(
            name="CP^1 Fubini-Study (same chart as S^2)",
            metric=CP1_METRIC,
            closed_form=cp1_closed_form_geodesic,
            seed=(0.1, -0.1, 0.25, 0.25),
        ),
    ]


# ============================================================================
# Runner
# ============================================================================


def run_case(case: TestCase, n_steps: int = 1000, tol: float = 1e-9) -> bool:
    print(f"\n-- {case.name} " + "-" * (60 - len(case.name)))
    x0, y0, vx0, vy0 = case.seed
    print(f"  seed: ({x0:+.4f}, {y0:+.4f})  tangent: ({vx0:+.4f}, {vy0:+.4f})")

    # Integrate
    trajectory = integrate_geodesic(case.metric, x0, y0, vx0, vy0, t_end=1.0, n_steps=n_steps)

    # Compare at three checkpoint times
    checkpoints = [0.1, 0.5, 1.0]
    max_err = 0.0
    for t_check in checkpoints:
        # Find the integrated state at t_check
        idx = int(round(t_check * n_steps))
        idx = min(idx, len(trajectory) - 1)
        _, x_int, y_int, _, _ = trajectory[idx]
        # Closed form
        x_cf, y_cf = case.closed_form(x0, y0, vx0, vy0, t_check)
        err = math.hypot(x_int - x_cf, y_int - y_cf)
        max_err = max(max_err, err)
        print(
            f"  t = {t_check:.2f}: integrator = ({x_int:+.6f}, {y_int:+.6f})  "
            f"closed form = ({x_cf:+.6f}, {y_cf:+.6f})  err = {err:.2e}"
        )

    ok = max_err < tol
    print(f"  max error: {max_err:.3e}   tolerance: {tol:.0e}   PASS: {ok}")
    return ok


def main():
    print("=" * 72)
    print("T11: Geodesic integrator math validation")
    print("=" * 72)
    print("  Compares RK4 integrator output (chart coords, Christoffels from")
    print("  conformal factor) to closed-form geodesic (embedded picture for")
    print("  S^2 / CP^1; linear flow for T^2). N=1000 RK4 steps; tol=1e-9.")

    cases = make_cases()
    results = []
    for c in cases:
        ok = run_case(c)
        results.append((c.name, ok))

    print("\n" + "=" * 72)
    print("SUMMARY")
    print("=" * 72)
    all_ok = True
    for name, ok in results:
        flag = "PASS" if ok else "FAIL"
        print(f"  [{flag}] {name}")
        all_ok = all_ok and ok

    if all_ok:
        print("\n  T11 GREEN -- geodesic integrator validated:")
        print("    RK4 integrator using Christoffel symbols from the conformal")
        print("    factor reproduces closed-form geodesics on S^2, T^2, CP^1")
        print("    to machine precision (1e-9 tolerance, 1000 RK4 steps).")
        print("  IMAGINE primitive's geodesic engine is unblocked.")
        return 0
    else:
        print("\n  T11 RED.")
        return 1


if __name__ == "__main__":
    sys.exit(main())
