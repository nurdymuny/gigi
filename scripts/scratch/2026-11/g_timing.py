import sys, time, numpy as np
sys.path.insert(0, 'scripts')
import edge_wp7_groups as GR, arc_principal_survey as S, edge_wp7_principal_circle as F
orig = GR.algebra
GR.algebra = lambda name: S.build(name) if name in ("g2", "sp3") else orig(name)
for gname, ang in (("g2", 0.0), ("sp3", 0.0)):
    t0 = time.time(); g = F.GroupK(gname, ang, sectors={}); print(gname, "init", round(time.time()-t0,1), "d", g.d, "qK", g.qK, "unique", g.top_root_unique, flush=True)
    # adjoint sector straight from ad matrices in the ORIGINAL basis
    C = GR.structure(g.basis)
    RX = [C[a].T.astype(complex) for a in range(g.d)]   # ad(X_a) acting on coords: (ad X_a)_{cb} = C[a,b,c]
    RY = [sum(g.R[k, a] * RX[a] for a in range(g.d)) for k in range(g.d)]
    g.reps["adj"] = RY
    t0 = time.time(); H, G = g.HG(-1.0, (0.5, 0.5, 0.25), "adj"); print(gname, "adj build", H.shape, round(time.time()-t0,1), flush=True)
