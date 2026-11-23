import sys, time, numpy as np
sys.path.insert(0,'scripts')
import edge_wp8_fast as FA, edge_wp7_principal_circle as F
# validation on F's public groups only: block method vs matrix-free, per sector margin at two x
for gname, ang, sh, x, secs in (("sp2", 0.0, (0.5,0.5,0.25), -1.3, ("14","16","10")), ("sp2", 45.0, (0.5,0.5,0.75), -1.2, ("14","16","4")), ("su4", 0.0, (0.5,0.5,0.25), -1.4, ("6","10"))):
    main, extra = F.sectors_for(gname)
    sub = {k: main[k] for k in secs}
    gb = FA.FastGroupK(gname, ang, sectors=sub); gc = FA.CheckGroupK(gname, ang, sectors=sub); gc.check = tuple(secs)
    for s in secs:
        a = gb.evaluate(x, sh, s); rest = a["mu"][np.argsort(np.abs(a["mu"]-a["lam"]))[a["ncopies"]:]]; ma = np.sort(rest)[0]-a["lam"]
        t0=time.time(); b = gc.evaluate(x, sh, s); mb = b["mu"][0]-b["lam"]
        print(gname, ang, s, f"block {ma:+.12f} free {mb:+.12f} diff {abs(ma-mb):.1e} lam diff {abs(a['lam']-b['lam']):.1e} copies {a['ncopies']}/{b['ncopies_removed']} {time.time()-t0:.1f}s", flush=True)
