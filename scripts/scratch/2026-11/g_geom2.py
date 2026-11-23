import sys, numpy as np
sys.path.insert(0, 'scripts')
import edge_wp8_fast as FA
from scipy.optimize import brentq
for gname in ("g2", "sp3"):
    g = FA.FastGroupK(gname, 0.0, sectors={})
    wts = np.array([w for w, _ in g.rts]); key = np.arange(1, g.rank+1.)
    pos = [w for w in wts if w @ key > 1e-9]; L2 = max(w@w for w in pos)
    short = [w for w in pos if w@w < L2-1e-9]; ths = max(short, key=lambda w: w @ g.rhov); thsv = 2*ths/(ths@ths)
    E = lambda a: np.cos(np.radians(a))*g.u0 + np.sin(np.radians(a))*g.perp
    # exact wall angle
    def fwall(a): return min((w @ E(a)) for w in pos)
    aw = brentq(fwall, 40, 50) if gname == "sp3" else 30.0
    rhoang = np.degrees(np.arccos(g.rhov@g.u0/np.linalg.norm(g.rhov)))
    print(gname, "rho angle", rhoang, "wall angle", aw)
    for lab, e in (("wall", E(aw)), ("thsv", thsv/np.linalg.norm(thsv)), ("rho", E(rhoang))):
        pr = sorted([(float(abs(w@e)), "L" if w@w > L2-1e-9 else "S", np.round(w,3).tolist()) for w in pos], reverse=True)
        top = pr[0][0]; tied = [p for p in pr if abs(p[0]-top) < 1e-9]
        cor = [2*w/(w@w) for w in pos]
        par = [abs(abs(c@e)/np.linalg.norm(c) - 1) < 1e-9 for c in cor]
        print("  ", lab, "top", round(top,4), "tied", [(t[1], t[2]) for t in tied], "next", round(pr[len(tied)][0],4), pr[len(tied)][1], "| parallel to coroot of:", [ (("L" if pos[i]@pos[i] > L2-1e-9 else "S"), np.round(pos[i],3).tolist()) for i in range(len(pos)) if par[i]])
