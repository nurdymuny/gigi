import sys, numpy as np
sys.path.insert(0,'scripts')
import edge_wp8_fast as FA, edge_wp8_two_new_groups as G
cs = G.circles()
g0 = FA.FastGroupK("sp3", 0.0, sectors={}); w, V, fv = FA.defining_weights(g0); pos = w[fv > 0]
e = np.linalg.lstsq(pos, np.ones(len(pos)), rcond=None)[0]; e /= np.linalg.norm(e)
cs.append({"group": "sp3", "label": "s5", "angle": float(np.degrees(np.arccos(abs(e @ g0.u0)))), "eK": e})
geos = {gn: G.geometry(gn) for gn in ("g2", "sp3")}
for c in cs:
    g = FA.FastGroupK(c["group"], c["angle"], eK_torus=c["eK"], sectors={}); geo = geos[c["group"]]
    gr = G.geometry_record(g, c["group"], geo)
    zero = sum(abs(w_ @ g.eK) < 1e-9 for w_ in geo["pos"])
    L = lambda w_: "L" if w_ @ w_ > geo["L2"] - 1e-9 else "S"
    yth = g.plane_roots[g.top_plane]
    print(c["label"], f"{c['angle']:.3f}", "qK", round(g.qK,4), "zero roots", zero, "ties", len(gr["tied_top"]), [t[-1] for t in gr["tied_top"]], "y_theta on", L(yth), np.round(yth,3).tolist(), "parallel coroot", [p[-1] for p in gr["parallel_to_coroot_of"]])
