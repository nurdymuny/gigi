"""Independent curvature via Koszul/Riemann; DOP853 trajectory, not GIGI RK4.

Only validation. No stored or hard-coded trajectories. Dependencies: numpy,
scipy. Conventions are checked against round and Berger closed forms.
"""
import itertools
import math
import numpy as np
from scipy.integrate import solve_ivp

BRACKET = np.zeros((3, 3, 3))
for i, j, k in [(0, 1, 2), (1, 2, 0), (2, 0, 1)]:
    BRACKET[i, j, k] = 2
    BRACKET[j, i, k] = -2


def curvature(metric):
    g = np.asarray(metric)
    connection = np.zeros((3, 3, 3), dtype=np.result_type(g, float))
    for i, j, k in itertools.product(range(3), repeat=3):
        connection[i, j, k] = (BRACKET[i, j, k]*g[k]
            - BRACKET[j, k, i]*g[i] + BRACKET[k, i, j]*g[j])/(2*g[k])
    riemann = (np.einsum("jkm,iml->ijkl", connection, connection)
               - np.einsum("ikm,jml->ijkl", connection, connection)
               - np.einsum("ijm,mkl->ijkl", BRACKET, connection))
    ricci = np.einsum("ijki->jk", riemann)
    eigenvalues = np.diag(ricci)/g
    sectional = np.array([riemann[0, 1, 1, 0]/g[1],
                          riemann[1, 2, 2, 1]/g[2],
                          riemann[2, 0, 0, 2]/g[0]])
    return np.diag(ricci), eigenvalues.sum(), sectional


def velocity(metric, normalized):
    r, scalar, _ = curvature(metric)
    return -2*r + ((2/3)*scalar*np.asarray(metric) if normalized else 0)


def trajectory(metric, time, steps, normalized):
    times = np.linspace(0, time, steps+1)
    result = solve_ivp(lambda t, g: velocity(g, normalized), (0, time), metric,
                       t_eval=times, method="DOP853", rtol=2e-12, atol=2e-14)
    if not result.success or result.y.shape != (3, steps+1):
        raise ValueError("independent Ricci reference failed")
    return result.y.T


def self_check():
    for g in [[1., 1., 1.], [2., 2., 2.], [3., 1., 1.]]:
        r, scalar, sectional = curvature(g)
        a, b, _ = g
        np.testing.assert_allclose(r, [2*a*a/b**2, 4-2*a/b, 4-2*a/b], atol=1e-13)
        np.testing.assert_allclose(sectional, [a/b**2, 4/b-3*a/b**2, a/b**2], atol=1e-13)
    for g in itertools.product([0.5, 1., 2., 5.], repeat=3):
        g = np.array(g)
        r, scalar, _ = curvature(g)
        eig = r/g
        for normalized in [False, True]:
            v = velocity(g, normalized)
            derivative = curvature(g.astype(complex)+1e-25j*v)[1].imag/1e-25
            expected = 2*np.dot(eig, eig) - ((2/3)*scalar**2 if normalized else 0)
            np.testing.assert_allclose(derivative, expected, rtol=2e-12, atol=2e-10)
            log_volume_rate = 0.5*np.sum(v/g)
            np.testing.assert_allclose(log_volume_rate, 0 if normalized else -scalar, atol=2e-12)
    return {"curvature_grid": 64, "scalar_evolution_checks": 128,
            "reference_integrator": "SciPy DOP853", "rtol": 2e-12, "atol": 2e-14}


if __name__ == "__main__":
    print(self_check())
