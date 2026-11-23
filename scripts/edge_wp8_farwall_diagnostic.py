"""Post-hoc diagnostic (disclosed; feeds the design of pre-registration G, not a test):
at Sp(2)'s far wall (45 deg, e_K along the short highest root's coroot) and at 0 deg, which root planes carry her
state (the cheapest exact state of top Q_K-charge) in the trivial sector? Weights of the state on each pair
(form-plane a, algebra-plane b) of root planes, and on the Cartan.
usage: OMP_NUM_THREADS=1 python scripts/edge_wp8_farwall_diagnostic.py OUT.json
"""
import json, sys
from pathlib import Path
import numpy as np
sys.path.insert(0, str(Path(__file__).resolve().parent))
import edge_wp7_principal_circle as F   # noqa: E402

out = {}
for gname, ang in (("sp2", 45.0), ("sp2", 0.0), ("su4", 45.0), ("su4", 0.0)):
    main, _ = F.sectors_for(gname)
    g = F.GroupK(gname, ang, sectors={"1": main["1"]})
    pairs = [float(w @ g.eK) for w in g.plane_roots]
    rec = {"plane_roots": [list(map(float, w)) for w in g.plane_roots], "pairing_with_eK": pairs,
           "top_plane_used_for_y_theta": g.top_plane, "qK": g.qK}
    for shape in F.SHAPES:
        pr = g.prepare(shape, "1"); P = pr["P"]; top = pr["top"]
        rows = []
        for x in (-1.5, -1.0, -0.5):
            Hp = P["H0"] + x * P["H1"] + x * x * P["H2"]
            Ht = Hp[np.ix_(top, top)]; Ht = 0.5 * (Ht + Ht.conj().T); wt, vt = np.linalg.eigh(Ht)
            v = pr["U"][:, top] @ vt[:, 0]
            d = g.d; M = v.reshape(d, d)      # (form index, algebra index); trivial sector dV = 1
            r = g.rank
            idx = {"cartan": list(range(r))}
            for k in range(len(g.plane_roots)):
                idx[f"plane{k}"] = [r + 2 * k, r + 2 * k + 1]
            wts = {}
            for a, ia in idx.items():
                for b, ib in idx.items():
                    w_ = float(np.sum(np.abs(M[np.ix_(ia, ib)]) ** 2))
                    if w_ > 1e-6:
                        wts[f"{a}|{b}"] = round(w_, 6)
            rows.append({"x": x, "top_dim": int(len(top)), "top_eigs": [round(float(e), 6) for e in wt], "weights": wts})
        rec[str(shape)] = rows
    out[f"{gname}_{ang}"] = rec
    print(gname, ang, "pairings", np.round(pairs, 4), "y_theta plane", g.top_plane)
    for shape in F.SHAPES:
        print("  ", shape, rec[str(shape)][1])
Path(sys.argv[1]).write_text(json.dumps(out, indent=1))
