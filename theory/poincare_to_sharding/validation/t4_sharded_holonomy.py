"""
T4: Sharded HOLONOMY across a chart boundary on T^2.

================================================================================
CLAIM (poincare_to_sharding.md §3.4):
    For a flat U(1) connection on T^2 with two charts U_L and U_R covering
    the manifold, and a closed loop gamma crossing the chart boundary, the
    holonomy computed by SHARDED parallel transport (transport-in-L, apply
    transition at seam, transport-in-R, apply inverse transition at
    closing seam) equals the holonomy computed by DIRECT global parallel
    transport. This holds even when the per-chart connections A_L and A_R
    are GAUGE-INEQUIVALENT representations on each chart -- because the
    closed-loop holonomy is a gauge-invariant physical observable.

    Operationally for sharded GIGI: HOLONOMY can be computed across
    multi-shard loops by composing per-shard transports with the seam
    transition map, with the same answer as if the loop were computed in
    a single global gauge.

REFERENCES:
    - Nakahara, *Geometry, Topology and Physics* §10 (gauge field theory,
      Wilson loops, gauge invariance of holonomy).
    - Davis, *The Geometry of Sameness* Section 5 (chart-transition
      translators inherit the connection's transition functions).
    - GIGI catalog L1.5 (parallel transport with B-source selector).

GROUND TRUTH (independent, closed-form):
    Connection A_L(x, y) = alpha(x) dx on T^2 with alpha(x) = 1 + sin(2 pi x).
    Holonomy around the horizontal loop gamma_x at y = 0:
        hol_direct = exp(-i * integral_0^1 alpha(x) dx)
                   = exp(-i * 1)
                   = e^{-i}    (closed form).

    Numerical ground truth: Euler integration of the parallel-transport
    ODE dpsi/dt + i alpha(x(t)) (dx/dt) psi = 0 around the full loop.

TEST DESIGN:
    Two parts.

    Part (a) -- IDENTITY transition (sanity check):
        Chart L covers x in [-eps, 1/2 + eps], chart R covers
        x in [1/2 - eps, 1 + eps]. Both use SAME gauge (A_R = A_L on
        overlap; transition g = 1). Sharded transport should trivially
        equal direct.

    Part (b) -- NON-TRIVIAL gauge transition:
        Chart L: A_L(x) = 1 + sin(2 pi x).
        Chart R: A_R(x) = A_L(x) - dh(x)/dx where h(x) = x is the gauge
                 generator. Hence A_R(x) = sin(2 pi x).
        Transition function: g(x) = exp(i * h(x)) = exp(i * x).
        Apply g at x = 1/2 (entering R) and g^{-1} at x = 0 (closing in L).

        Sharded computation:
            psi(0) = ...starting fiber value...
            psi_L(1/2) = exp(-i * integral_0^{1/2} A_L) * psi(0)
            psi_R(1/2) = g(1/2) * psi_L(1/2)
            psi_R(1)   = exp(-i * integral_{1/2}^1 A_R) * psi_R(1/2)
            psi_L(0)_after = g(1)^{-1} * psi_R(1)   [periodic, x=1 == x=0]

        The two charts use DIFFERENT connections (A_L != A_R on overlap),
        but the closed-loop result must equal the direct computation
        because gauge transformations cancel around closed loops.

PASS CRITERION:
    Both parts:
        |hol_sharded - hol_direct| < tol
    where tol = 5e-3 for Euler integration with N = 10000 steps.
    (Euler error ~ O(1/N) ~ 1e-4 per leg; cumulative ~ 1e-3.)

CIRCULAR-LOGIC GUARDS:
    1. Direct ground truth uses A_L only (single global gauge).
       Sharded computation uses A_L on chart L and A_R on chart R;
       A_R is computed INDEPENDENTLY from A_L via the gauge formula
       A_R = A_L - dh/dx, NOT borrowed from the direct integration.
    2. The transition map g(x) = exp(i h(x)) is part of the atlas
       data, declared upfront, not engineered to make the equality hold.
    3. We use a *separate* naive Euler integrator for both direct and
       sharded computations -- they use the SAME numerical method on
       DIFFERENT inputs (different As, different domains, different
       transition factors). Any equality is a math fact about gauge
       invariance, not a numerical artifact.
    4. Substantive non-triviality: we explicitly check that A_L and A_R
       are NOT equal on the overlap (they differ by the gauge gradient
       dh/dx = 1). If they were equal, sharded ~ direct would be a
       tautology.
================================================================================
"""

from __future__ import annotations
import math
import sys


# ============================================================================
# Connection data and gauge transition
# ============================================================================


def A_L(x: float) -> float:
    """
    Connection 1-form coefficient in chart L (single global gauge):
    A_L(x) = 1 + sin(2 pi x).

    Total integral over loop: integral_0^1 A_L dx = 1.
    """
    return 1.0 + math.sin(2.0 * math.pi * x)


def h_gauge(x: float) -> float:
    """
    Gauge generator: h(x) = x. The transition function on overlap is
    g(x) = exp(i * h(x)) = exp(i * x).
    """
    return x


def dh_dx(x: float) -> float:
    """Derivative of gauge generator: dh/dx = 1."""
    return 1.0


def A_R(x: float) -> float:
    """
    Connection 1-form coefficient in chart R, gauge-transformed from
    chart L by g = exp(i * h):
        A_R = A_L - dh/dx
    So A_R(x) = sin(2 pi x).

    Note: A_R is derived from gauge formula, NOT borrowed from a global
    integration. Circular-logic guard #1.
    """
    return A_L(x) - dh_dx(x)


# ============================================================================
# Euler integrator for U(1) parallel transport
# ============================================================================


def transport_euler(A_fn, x_start: float, x_end: float, psi0: complex, n_steps: int = 10000) -> complex:
    """
    Numerically integrate parallel transport
        d psi / dx = -i A(x) psi
    from x = x_start to x = x_end with N Euler steps.

    Args:
        A_fn:   connection coefficient as a function of x.
        x_start, x_end:  integration endpoints.
        psi0:   initial fiber value.
        n_steps:  number of Euler steps.

    Returns: psi(x_end) as a complex number.
    """
    psi = complex(psi0)
    dx = (x_end - x_start) / n_steps
    for k in range(n_steps):
        x = x_start + k * dx
        # Forward Euler: psi <- psi + dpsi/dx * dx
        # dpsi/dx = -i A(x) psi
        psi = psi + (-1j * A_fn(x) * psi) * dx
    return psi


# ============================================================================
# Holonomy computations: direct vs sharded
# ============================================================================


def holonomy_direct(psi0: complex = 1.0 + 0.0j, n_steps: int = 10000) -> complex:
    """
    Direct holonomy: integrate transport from x = 0 to x = 1 using A_L
    throughout (single global gauge).

    Returns: psi(1) / psi(0).
    """
    psi_end = transport_euler(A_L, 0.0, 1.0, psi0, n_steps=n_steps)
    return psi_end / psi0


def holonomy_sharded_identity(psi0: complex = 1.0 + 0.0j, n_steps: int = 5000) -> complex:
    """
    Part (a): sharded holonomy with IDENTITY transition.

    Transport in chart L from 0 to 1/2 with A_L, identity transition at
    seam, transport in chart R from 1/2 to 1 with A_L (same gauge),
    identity transition at closing seam (x=1 ~ x=0).

    Should trivially equal direct.
    """
    psi_at_half = transport_euler(A_L, 0.0, 0.5, psi0, n_steps=n_steps)
    # identity transition L -> R
    psi_R_at_half = psi_at_half
    psi_R_at_one = transport_euler(A_L, 0.5, 1.0, psi_R_at_half, n_steps=n_steps)
    # identity transition R -> L (periodic, x=1 == x=0)
    psi_end = psi_R_at_one
    return psi_end / psi0


def holonomy_sharded_gauge(psi0: complex = 1.0 + 0.0j, n_steps: int = 5000) -> complex:
    """
    Part (b): sharded holonomy with NON-TRIVIAL gauge transition.

    Chart L uses A_L. Chart R uses A_R = A_L - dh/dx (gauge-transformed).
    Transition g(x) = exp(i h(x)) applied at seam x = 1/2 (entering R).
    Transition g^{-1}(x) applied at seam x = 1 == 0 (closing in L).

    Should equal direct because gauge transformations cancel around
    closed loops.
    """
    # 1. Transport in chart L from 0 to 1/2 with A_L
    psi_L_at_half = transport_euler(A_L, 0.0, 0.5, psi0, n_steps=n_steps)

    # 2. Transition L -> R at x = 1/2: psi_R = g(1/2) * psi_L
    g_at_half = complex(math.cos(h_gauge(0.5)), math.sin(h_gauge(0.5)))
    psi_R_at_half = g_at_half * psi_L_at_half

    # 3. Transport in chart R from 1/2 to 1 with A_R
    psi_R_at_one = transport_euler(A_R, 0.5, 1.0, psi_R_at_half, n_steps=n_steps)

    # 4. Transition R -> L at x = 1 == 0: psi_L = g(1)^{-1} * psi_R
    g_at_one = complex(math.cos(h_gauge(1.0)), math.sin(h_gauge(1.0)))
    g_at_one_inv = g_at_one.conjugate()  # U(1): inverse = conjugate
    psi_L_back = g_at_one_inv * psi_R_at_one

    return psi_L_back / psi0


# ============================================================================
# Closed-form check
# ============================================================================


def holonomy_closed_form() -> complex:
    """
    Closed-form integral: int_0^1 A_L(x) dx = int_0^1 (1 + sin(2 pi x)) dx
                                            = 1 + 0 = 1.
    Holonomy = exp(-i * 1) = e^{-i}.
    """
    return complex(math.cos(-1.0), math.sin(-1.0))


# ============================================================================
# Test runner
# ============================================================================


def main():
    print("=" * 72)
    print("T4: Sharded HOLONOMY across T^2 chart boundary")
    print("=" * 72)

    hol_cf = holonomy_closed_form()
    print(f"  Closed-form holonomy: exp(-i) = {hol_cf:.6f}")

    hol_direct = holonomy_direct()
    print(f"  Direct numerical hol: {hol_direct:.6f}")
    direct_err = abs(hol_direct - hol_cf)
    print(f"  |direct - closed_form|: {direct_err:.3e}")

    # Substantive non-triviality: check A_L != A_R on overlap
    print(f"\n  Non-triviality check (A_L vs A_R on overlap):")
    for x in [0.25, 0.5, 0.75]:
        d = A_R(x) - A_L(x)
        print(f"    x = {x}: A_L = {A_L(x):.4f}, A_R = {A_R(x):.4f}, "
              f"diff = {d:.4f}  (gauge gradient dh/dx = {dh_dx(x):.1f})")
    a_l_ne_a_r = any(abs(A_R(x) - A_L(x)) > 1e-10 for x in [0.25, 0.5, 0.75])
    print(f"  A_L != A_R on overlap: {a_l_ne_a_r}")
    print(f"  -> sharded test is NOT a tautology (gauges genuinely differ)")

    # Part (a): identity transition
    print(f"\n-- Part (a): IDENTITY transition (sanity) " + "-" * 27)
    hol_a = holonomy_sharded_identity()
    err_a = abs(hol_a - hol_cf)
    tol = 5e-3
    print(f"  Sharded (identity transition): {hol_a:.6f}")
    print(f"  |sharded - closed_form|: {err_a:.3e}")
    print(f"  tolerance: {tol:.0e}")
    pass_a = err_a < tol
    print(f"  PASS: {pass_a}")

    # Part (b): non-trivial gauge transition
    print(f"\n-- Part (b): GAUGE-NONTRIVIAL transition " + "-" * 27)
    hol_b = holonomy_sharded_gauge()
    err_b = abs(hol_b - hol_cf)
    print(f"  Sharded (gauge transition g = exp(i*x)): {hol_b:.6f}")
    print(f"  |sharded - closed_form|: {err_b:.3e}")
    print(f"  tolerance: {tol:.0e}")
    pass_b = err_b < tol
    print(f"  PASS: {pass_b}")

    print("\n" + "=" * 72)
    print("SUMMARY")
    print("=" * 72)
    print(f"  [{('PASS' if pass_a else 'FAIL')}] Part (a) identity transition")
    print(f"  [{('PASS' if pass_b else 'FAIL')}] Part (b) non-trivial gauge transition")
    print(f"  [{('PASS' if a_l_ne_a_r else 'FAIL')}] non-triviality (A_L != A_R on overlap)")

    all_ok = pass_a and pass_b and a_l_ne_a_r
    if all_ok:
        print("\n  T4 GREEN -- sharded HOLONOMY validated:")
        print("    (a) Sharded transport with trivial transition = direct.")
        print("    (b) Sharded transport with non-trivial GAUGE transition = direct,")
        print("        even though per-chart connections A_L and A_R DIFFER on overlap.")
        print("        Gauge invariance of closed-loop holonomy holds under sharding.")
        print("  Sharded HOLONOMY claim is unblocked.")
        return 0
    else:
        print("\n  T4 RED -- do not promote sharded HOLONOMY claim.")
        return 1


if __name__ == "__main__":
    sys.exit(main())
