"""Closed quadratic fields and their exact face fluxes (shared by the count and the base probe)."""

import sys, json, itertools
import numpy as np

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

