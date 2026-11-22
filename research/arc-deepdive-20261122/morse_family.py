"""Morse structure of the Yang-Mills energy restricted to her family (lit_morse.md, Part C).

Family (the basin note): M0 = x tau0, M1 = y tau1, M2 = y tau2 on the Berger sphere (t,1,1).
  E = 1/2 (B^2 + 2 c^2),  B = y^2 - 2 sqrt(t) x,  c = y (x - 2/sqrt(t)),  L2 metric G = diag(1, 2).
  her price (twist eigenvalue, sector j) P_j = (x + (j+2)/sqrt(t))^2 + 2 sqrt(t) x - y^2.

Questions:
 1. critical points of E on the family, their energy and family-restricted Morse index;
 2. her price at those critical points: how many sectors j have her as a negative direction there
    (a lower bound on the full Morse index at that critical point, since she is an exact eigenvector);
 3. the 2D stratification (stable manifolds) of the family flow, and whether the curve where her lowest price
    crosses zero (door -> wall) coincides with a stratum boundary (a separatrix = stable manifold of a saddle);
 4. where the Berger flow line passes relative to the saddle.
Single-threaded, seconds.
"""
import json, numpy as np, sympy as sp
from scipy.integrate import solve_ivp
from scipy.optimize import fsolve

x, y = sp.symbols('x y', real=True)
t = sp.symbols('t', positive=True)
st = sp.sqrt(t)
B = y**2 - 2*st*x; c = y*(x - 2/st)
E = sp.Rational(1, 2)*(B**2 + 2*c**2)
gx, gy = sp.diff(E, x), sp.diff(E, y)
Hs = sp.hessian(E, (x, y))
Ef = sp.lambdify((x, y, t), E); gf = sp.lambdify((x, y, t), [gx, gy]); Hf = sp.lambdify((x, y, t), Hs)
j = sp.symbols('j')
P = (x + (j + 2)/st)**2 + 2*st*x - y**2
Pf = sp.lambdify((x, y, t, j), P)
G = np.diag([1.0, 2.0])

out = {}

# --- exact facts (sympy) ---
# the saddle: y^2 = (2 sqrt(t) sqrt(9t+16) - 6t)/4 from the solver; verify at t = 1 it is the meron (x, y) = (1, 1)
sols = sp.solve([gx, gy], [x, y], dict=True)
out['n_critical_solutions_sympy'] = len(sols)
# family Hessian eigenvalues (G^-1 H) at the flat points, exact
for name, pt in {'A=0': (0, 0), 'A=theta': (2/st, 2)}.items():
    M = sp.simplify(sp.Matrix([[1, 0], [0, 2]]).inv()*Hs.subs({x: pt[0], y: pt[1]}))
    out[f'family_hessian_eigs_{name}'] = [str(sp.simplify(e)) for e in M.eigenvals()]


def flow(T, v0, tmax, events=None):
    f = lambda s, v: list(-np.linalg.solve(G, np.array(gf(v[0], v[1], T))))
    return solve_ivp(f, [0, tmax], v0, rtol=1e-11, atol=1e-13, dense_output=True, events=events)


def saddle(T):
    # numeric root seeded from the sympy closed form for y
    y0 = np.sqrt((2*np.sqrt(T)*np.sqrt(9*T + 16) - 6*T)/4)
    xs, ys = fsolve(lambda v: gf(v[0], v[1], T), [1/np.sqrt(T), y0], xtol=1e-14)
    return xs, ys


rows = []
for T in [0.5, 0.9, 1.0, 1.1, 1.5, 2.0, 2.75, 3.0, 3.55, 5.0, 7.0, 10.0]:
    xs, ys = saddle(T)
    H = np.array(Hf(xs, ys, T), float)
    ev, evec = np.linalg.eig(np.linalg.solve(G, H))
    order = np.argsort(ev.real); ev = ev.real[order]; evec = evec.real[:, order]
    fam_index = int((ev < 0).sum())
    prices = [Pf(xs, ys, T, jj) for jj in range(0, 41)]
    neg_sectors = [jj for jj, p in enumerate(prices) if p < 0]
    # her contribution to the full index: sector j carries her with multiplicity (j+1) (left SU(2) copies), times 2 for her mirror?
    # we report only the list of sectors; multiplicities follow the Floor-paper convention and are not asserted here.
    # stable manifold of the saddle in the family plane: integrate backwards along the stable eigenvector
    vs = evec[:, 1]  # positive eigenvalue direction = stable direction of the downward flow
    sep = []
    for sgn in (+1, -1):
        v0 = [xs + sgn*1e-7*vs[0], ys + sgn*1e-7*vs[1]]
        fb = lambda s, v: list(+np.linalg.solve(G, np.array(gf(v[0], v[1], T))))  # backward flow
        ev_big = lambda s, v: np.hypot(v[0], v[1]) - 12; ev_big.terminal = True
        sol = solve_ivp(fb, [0, 50], v0, rtol=1e-11, atol=1e-13, events=ev_big, max_step=0.01)
        sep.append(sol.y.T)
    # the unstable manifold: where does each branch land?
    vu = evec[:, 0]
    lands = []
    for sgn in (+1, -1):
        sol = flow(T, [xs + sgn*1e-6*vu[0], ys + sgn*1e-6*vu[1]], 400)
        xe, ye = sol.y[:, -1]
        lands.append([round(float(xe), 6), round(float(ye), 6), float(Ef(xe, ye, T))])
    # her lowest price along the separatrix: does the separatrix coincide with {min_j P_j = 0}?
    def minP(px, py):
        return min(Pf(px, py, T, jj) for jj in range(0, 41))
    sep_minP = [minP(p[0], p[1]) for br in sep for p in br[::max(1, len(br)//200)]]
    # Berger flow line
    xb, yb = (2 - T)/np.sqrt(T), np.sqrt(T)
    sol = flow(T, [xb, yb], 50)
    ss = np.linspace(0, 50, 50001)
    path = sol.sol(ss)
    dist = np.hypot(path[0] - xs, np.sqrt(2)*(path[1] - ys))  # L2 distance in the family metric
    k = int(np.argmin(dist))
    m = np.array([minP(path[0, i], path[1, i]) for i in range(0, 5001)])  # first tau in [0,5]
    kz = int(np.argmax(m > 0)) if (m > 0).any() and m[0] < 0 else None
    rows.append(dict(
        t=T, saddle=[float(xs), float(ys)], E_saddle=float(Ef(xs, ys, T)),
        family_eigs_saddle=[float(e) for e in ev], family_index_saddle=fam_index,
        her_price_saddle_j0_to_6=[float(p) for p in prices[:7]], her_negative_sectors_at_saddle=neg_sectors,
        unstable_branches_land_at=lands,
        separatrix_minP_range=[float(min(sep_minP)), float(max(sep_minP))],
        berger_point=[float(xb), float(yb)], E_berger=float(Ef(xb, yb, T)), minP_berger=float(minP(xb, yb)),
        berger_lands_at=[float(path[0, -1]), float(path[1, -1])],
        berger_closest_approach_to_saddle=float(dist[k]), at_tau=float(ss[k]),
        door_to_wall_tau=(float(ss[kz]) if kz is not None else None),
        E_at_door_to_wall=(float(Ef(path[0, kz], path[1, kz], T)) if kz is not None else None),
    ))

out['rows'] = rows
# exact check: meron at t=1
out['meron_check_t1'] = {k: float(v) for k, v in zip(['gx', 'gy'], gf(1.0, 1.0, 1.0))}
json.dump(out, open(__file__.replace('.py', '_receipt.json'), 'w'), indent=1)
for r in rows:
    print(f"t={r['t']:5}: saddle=({r['saddle'][0]:+.4f},{r['saddle'][1]:.4f}) E={r['E_saddle']:.4f} famIdx={r['family_index_saddle']} "
          f"eigs={np.round(r['family_eigs_saddle'],4)} her<0 sectors={r['her_negative_sectors_at_saddle']}")
    print(f"        unstable->{r['unstable_branches_land_at']}  sep minP in {np.round(r['separatrix_minP_range'],3)}")
    print(f"        Berger E={r['E_berger']:.3f} minP={r['minP_berger']:+.3f} lands {np.round(r['berger_lands_at'],4)} "
          f"closest to saddle {r['berger_closest_approach_to_saddle']:.4f} @tau={r['at_tau']:.3f}; door->wall tau={r['door_to_wall_tau']} E={r['E_at_door_to_wall']}")
print(out['family_hessian_eigs_A=0'], out['family_hessian_eigs_A=theta'], out['meron_check_t1'])
