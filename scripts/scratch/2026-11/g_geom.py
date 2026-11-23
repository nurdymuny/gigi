import sys, time, numpy as np
sys.path.insert(0, 'scripts')
import edge_wp8_fast as FA
for gname in ("g2", "sp3"):
    g = FA.FastGroupK(gname, 0.0, sectors={})
    wts = np.array([w for w, _ in g.rts]); key = np.arange(1, g.rank+1.)
    pos = [w for w in wts if w @ key > 1e-9]
    L2 = max(w@w for w in pos); short = [w for w in pos if w@w < L2-1e-9]
    ths = max(short, key=lambda w: w @ g.rhov) if short else None
    thsv = 2*ths/(ths@ths) if ths is not None else None
    ang = lambda a,b: np.degrees(np.arccos(np.clip(a@b/np.linalg.norm(a)/np.linalg.norm(b),-1,1)))
    print(gname, "rank", g.rank, "angle(thv,rhov)", round(ang(g.thv,g.rhov),4), "theta_s", ths, "angle(thv,thsv)", round(ang(g.thv,thsv),4))
    # where along the arc is the far wall? scan
    prev = None
    for a in np.arange(0.0, 90.0, 0.01):
        e = np.cos(np.radians(a))*g.u0 + np.sin(np.radians(a))*g.perp
        reg = min(abs(w @ e) for w in pos) > 1e-9
        if a > 0.5 and not reg:
            print("  arc wall at", round(a,2), "eK", np.round(e,4), "angle to thsv", round(ang(e,thsv),3)); break
    # thsv off-arc distance
    comp = thsv/np.linalg.norm(thsv); inplane = (comp@g.u0)**2 + (comp@g.perp)**2
    print("  thsv in arc plane?", round(inplane,6))
    for a in (0.0,):
        pass
print("--- sign-change scan")
for gname in ("g2", "sp3"):
    g = FA.FastGroupK(gname, 0.0, sectors={})
    wts = np.array([w for w, _ in g.rts]); key = np.arange(1, g.rank+1.)
    pos = [w for w in wts if w @ key > 1e-9]
    E = lambda a: np.cos(np.radians(a))*g.u0 + np.sin(np.radians(a))*g.perp
    s0 = np.sign([w @ E(0.5) for w in pos])
    for a in np.arange(0.5, 120, 0.001):
        s = np.sign([w @ E(a) for w in pos])
        if np.any(s != s0):
            k = np.where(s!=s0)[0]; print(gname, "first wall crossing near", round(a,3), [np.round(pos[i],4).tolist() for i in k]); break
    for a in (0.0, 45.0):
        e = E(a); pr = sorted([round(float(abs(w@e)),4) for w in pos], reverse=True); print("  at", a, "pairings", pr[:5])
