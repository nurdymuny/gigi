"""Her lowest price along both unstable branches of the family saddle (the half-theta 'meron-type' critical point).
Spectral-flow reading: along a flow line between two critical points, the net number of Hessian eigenvalues crossing
zero equals the index difference. This checks her share of that crossing count on each branch. Seconds."""
import json, numpy as np, sympy as sp
from scipy.integrate import solve_ivp
from scipy.optimize import fsolve
x, y = sp.symbols('x y', real=True); t = sp.symbols('t', positive=True); st = sp.sqrt(t)
B = y**2 - 2*st*x; c = y*(x - 2/st); E = sp.Rational(1, 2)*(B**2 + 2*c**2)
gf = sp.lambdify((x, y, t), [sp.diff(E, x), sp.diff(E, y)]); Hf = sp.lambdify((x, y, t), sp.hessian(E, (x, y)))
j = sp.symbols('j'); Pf = sp.lambdify((x, y, t, j), (x + (j + 2)/st)**2 + 2*st*x - y**2)
G = np.diag([1.0, 2.0]); res = []
for T in [0.5, 1.0, 2.0, 3.0, 3.55, 5.0, 10.0]:
    y0 = np.sqrt((2*np.sqrt(T)*np.sqrt(9*T + 16) - 6*T)/4)
    xs, ys = fsolve(lambda v: gf(v[0], v[1], T), [1/np.sqrt(T), y0])
    ev, V = np.linalg.eig(np.linalg.solve(G, np.array(Hf(xs, ys, T), float))); vu = V[:, np.argmin(ev.real)].real
    for sgn in (+1, -1):
        sol = solve_ivp(lambda s, v: list(-np.linalg.solve(G, np.array(gf(v[0], v[1], T)))), [0, 400],
                        [xs + sgn*1e-6*vu[0], ys + sgn*1e-6*vu[1]], rtol=1e-11, atol=1e-13, dense_output=True)
        pts = sol.sol(np.linspace(0, 400, 40001))
        m = [min(Pf(pts[0, i], pts[1, i], T, jj) for jj in range(41)) for i in range(0, 40001, 10)]
        res.append(dict(t=T, branch_end=[round(float(pts[0, -1]), 5), round(float(pts[1, -1]), 5)],
                        min_over_branch_of_her_lowest_price=float(min(m)), her_lowest_price_at_saddle=float(m[0])))
        print(res[-1])
json.dump(res, open(__file__.replace('.py', '_receipt.json'), 'w'), indent=1)
