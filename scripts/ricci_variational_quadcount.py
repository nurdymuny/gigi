"""Does the two-ring clue space hold anything beyond closed linear + closed quadratic fields?
Per cell: rank of the face-flux map D (independent clues), and of [linear 11 | quadratic 15]
closed-field fluxes, and of their union with range(D). Exact quadrature (edge midpoints, degree 2)."""
import sys, json, itertools
import numpy as np
sys.path[:0] = ['scripts', 'e2e/probes']
import ricci_variational_v1 as V
pg, tw = V.pg, V.tw

# divergence-free homogeneous quadratic vector fields on R^3: 18 monomial fields, null space of div
mon = list(itertools.combinations_with_replacement(range(3), 2))      # 6 monomials x_i x_j
def div_row(comp, m):                                                 # d/dx_comp of x_i x_j -> linear coeffs
    r = np.zeros(3); i, j = m
    if i == comp: r[j] += 1
    if j == comp: r[i] += 1
    return r
Dv = np.zeros((3, 18))
for comp in range(3):
    for mi, m in enumerate(mon):
        Dv[:, comp * 6 + mi] = div_row(comp, m)
_, s, vt = np.linalg.svd(Dv); Q = vt[np.sum(s > 1e-12):]               # 15 x 18
assert Q.shape[0] == 15
def quad_field(x):                                                    # x: (...,3) -> (...,15,3)
    vals = np.stack([x[..., i] * x[..., j] for i, j in mon], -1)      # (...,6)
    F = np.zeros(x.shape[:-1] + (18,)); 
    out = np.zeros(x.shape[:-1] + (15, 3))
    for comp in range(3):
        out[..., comp] = vals @ Q[:, comp * 6:(comp + 1) * 6].T
    return out
def quad_flux(tri):
    z1, z2, z3 = tri
    area = np.cross(z2 - z1, z3 - z1) / 2
    mids = np.array([(z1 + z2) / 2, (z2 + z3) / 2, (z3 + z1) / 2])
    return quad_field(mids).mean(0) @ area                            # 15

def rank(A, tol=1e-9):
    s = np.linalg.svd(A, compute_uv=False); return int((s > tol * s[0]).sum())

out = {}
for level in (0, 1):
    L = pg.load_level(level, V.ORDER); cells = L['cells']; faces = pg.face_index(cells)
    rows = []
    for rings in (2, 3):
        for ci in range(0, len(cells), max(1, len(cells) // 30)):
            nb = tw.neighbourhood_rings(cells, faces, ci, rings)
            to_orth, J, t0 = pg.chart(nb[ci])
            seen, lin, quad, terms = set(), [], [], []
            for nj in [ci] + [k for k in nb if k != ci]:
                O = to_orth(nb[nj])
                for f in pg.FACES:
                    key = pg.face_key(cells[nj], f)
                    if key in seen: continue
                    seen.add(key); tri = O[list(f)]
                    lin.append(pg.flux_row(tri)); quad.append(quad_flux(tri)); terms.append(pg.face_edge_terms(cells[nj], f))
            cols = sorted({e for t in terms for e, _ in t}); cix = {e: i for i, e in enumerate(cols)}
            D = np.zeros((len(terms), len(cols)))
            for i, t in enumerate(terms):
                for e, c in t: D[i, cix[e]] += c
            Xl, Xq = np.array(lin), np.array(quad)
            Xall = np.hstack([Xl, Xq])
            rows.append({"rings": rings, "faces": len(terms), "clues": rank(D), "lin": rank(Xl),
                         "lin_quad": rank(Xall), "lin_in_D": rank(np.hstack([D, Xl])) - rank(D),
                         "linquad_in_D": rank(np.hstack([D, Xall])) - rank(D)})
    for rings in (2, 3):
        R = [r for r in rows if r["rings"] == rings]
        summ = {k: [min(r[k] for r in R), max(r[k] for r in R)] for k in R[0] if k != "rings"}
        out[f"n{[1,2][level]}_rings{rings}"] = summ
        print(level, rings, summ, flush=True)
json.dump(out, open(sys.argv[1], 'w'), indent=1) if len(sys.argv) > 1 else None
