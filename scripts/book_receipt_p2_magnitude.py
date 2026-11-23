"""Receipt for the Helike book, section 4.2 (Long Form 4.2, rung 6): the numerically located edges make P2 vanish to
within 1e-8, "against values of 10^3 to 10^6 everywhere else". Reports raw |P2| and |P2| relative to its own scale
(|P2| divided by the sum of the absolute values of its terms at that point), since raw values depend on normalisation.

P2 is the degree-(7, 8, 16) factor of the edge polynomial P = det(B_{4,0} - lambda), re-factored here from the saved
expression (research/su2-floor-family-20261119/edge_polynomial.srepr). It is evaluated
  (a) at every located edge where the (4,0) block is the competitor (research/su2-floor-family-20261119/exact_edge.json),
  (b) on a grid x in [-3, 0] (31 points), y in {0.5, 1, 1.5, 2, 2.5, 3}, t in {2, 3, 4, 6, 8, 10}, keeping only points
      at least 0.1 from every real root of P2 in x at that (y, t): "everywhere else".
usage (repository root): python scripts/book_receipt_p2_magnitude.py research/book-receipts-20261122/p2_magnitude.json
"""
import json
import sys
from pathlib import Path

import numpy as np
import sympy as sp
from sympy import *  # noqa: F401,F403  (the srepr names)

x, y, s = sp.symbols("x y s", real=True)
P = eval(Path("research/su2-floor-family-20261119/edge_polynomial.srepr").read_text())  # noqa: S307
P = P.subs({v: {"x": x, "y": y, "s": s}[v.name] for v in P.free_symbols})
fl = sp.factor_list(sp.expand(P))
P2 = None
for fac, _ in fl[1]:
    pol = sp.Poly(fac, x, y, s)
    if (pol.degree(x), pol.degree(y), pol.degree(s)) == (7, 8, 16):
        P2 = fac
assert P2 is not None, "P2 factor not found"
f = sp.lambdify((x, y, s), P2, "numpy")
terms = sp.Add.make_args(sp.expand(P2))
fabs = sp.lambdify((x, y, s), sum(sp.Abs(tm) for tm in terms), "numpy")   # sum of |each term|: P2's own scale
px = sp.Poly(P2, x)

edges = json.loads(Path("research/su2-floor-family-20261119/exact_edge.json").read_text())["edges"]
at_edge = []
for e in edges:
    if list(e["competitor_block_A"]) == [4, 0.0]:
        args = (e["edge_A"], e["y"], np.sqrt(e["t"]))
        at_edge.append(dict(t=e["t"], y=e["y"], x=e["edge_A"], P2=float(f(*args)), rel=float(abs(f(*args)) / fabs(*args))))

away = []
away_rel = []
for t in (2.0, 3.0, 4.0, 6.0, 8.0, 10.0):
    for yv in (0.5, 1.0, 1.5, 2.0, 2.5, 3.0):
        coeffs = [float(c) for c in sp.Poly(px.as_expr().subs({y: yv, s: np.sqrt(t)}), x).all_coeffs()]
        roots = [r.real for r in np.roots(coeffs) if abs(r.imag) < 1e-9]
        for xv in np.linspace(-3, 0, 31):
            if all(abs(xv - r) >= 0.1 for r in roots):
                away.append(abs(float(f(xv, yv, np.sqrt(t)))))
                away_rel.append(abs(float(f(xv, yv, np.sqrt(t)))) / float(fabs(xv, yv, np.sqrt(t))))
away = np.array(away)
away_rel = np.array(away_rel)
res = dict(
    P2_degrees=[7, 8, 16], P2_terms=len(sp.Poly(P2, x, y, s).terms()),
    at_edges=dict(points=len(at_edge), max_abs_P2=max(abs(r["P2"]) for r in at_edge),
                  max_relative=max(r["rel"] for r in at_edge), rows=at_edge),
    away=dict(points=int(away.size), min_abs_P2=float(away.min()), max_abs_P2=float(away.max()),
              p05=float(np.percentile(away, 5)), median=float(np.median(away)), p95=float(np.percentile(away, 95)),
              min_relative=float(away_rel.min()), median_relative=float(np.median(away_rel))))
print(f"P2: degrees (7,8,16), {res['P2_terms']} terms")
print(f"at {res['at_edges']['points']} located (4,0) edges: max |P2| = {res['at_edges']['max_abs_P2']:.3e}; "
      f"relative to P2's own scale (|P2| / sum|terms|): max {res['at_edges']['max_relative']:.3e}")
a = res["away"]
print(f"away from every root ({a['points']} grid points): |P2| from {a['min_abs_P2']:.3e} to {a['max_abs_P2']:.3e}; "
      f"5%-95%: {a['p05']:.3e} .. {a['p95']:.3e}; median {a['median']:.3e}; relative: min {a['min_relative']:.3e}, "
      f"median {a['median_relative']:.3e}")
Path(sys.argv[1]).parent.mkdir(parents=True, exist_ok=True)
Path(sys.argv[1]).write_text(json.dumps(res, indent=1))
