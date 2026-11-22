"""Receipt for the book's "the field along her circle turns to run with her at a flow time of about 0.03" (section 4.3).
Family flow from the Berger field (same flow as family_flow.py): the flow time at which x crosses zero, and the time
at which her lowest price (j <= 12) turns positive, at t = 2, 3, 3.55, 5.
usage (repository root): python docs/research/basin-2026-11-22/x_turns_positive.py
"""
import numpy as np
import sympy as sp
from scipy.integrate import solve_ivp

x, y, t = sp.symbols("x y t", real=True)
st = sp.sqrt(t)
E = sp.Rational(1, 2) * ((y**2 - 2*st*x)**2 + 2*(y*(x - 2/st))**2)
fx = sp.lambdify((x, y, t), -sp.diff(E, x)); fy = sp.lambdify((x, y, t), -sp.diff(E, y) / 2)
lam = lambda xv, yv, tv: min((xv + (j + 2) / np.sqrt(tv))**2 + 2*np.sqrt(tv)*xv - yv**2 for j in range(13))  # noqa: E731
for T in (2.0, 3.0, 3.55, 5.0):
    sol = solve_ivp(lambda s, v: [fx(v[0], v[1], T), fy(v[0], v[1], T)], [0, 1], [(2 - T)/np.sqrt(T), np.sqrt(T)],
                    dense_output=True, rtol=1e-11, atol=1e-12)
    ss = np.linspace(0, 1, 200001)
    xs = np.array([sol.sol(s)[0] for s in ss]); ls = np.array([lam(*sol.sol(s), T) for s in ss])
    tx = ss[np.argmax(xs > 0)]
    tl = ss[np.argmax(ls > 0)] if ls[0] < 0 else 0.0
    print(f"t={T:4}: x turns positive at tau = {tx:.4f}; her lowest price (j<=12) turns positive at tau = {tl:.4f}")
