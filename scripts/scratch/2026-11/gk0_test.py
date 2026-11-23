import sys, time, numpy as np
sys.path.insert(0, 'scripts')
import edge_wp8_fast as FA, edge_wp7_principal_circle as F
for gname, ang, sh, x in (("sp2", 0.0, (0.5,0.5,0.25), -1.3), ("sp2", 45.0, (0.5,0.5,0.75), -1.2), ("su4", 0.0, (0.5,0.5,0.25), -1.4)):
    main, extra = F.sectors_for(gname)
    if gname == "su4": main = {k: main[k] for k in ("1", "4", "6")}
    t0=time.time(); gd = F.GroupK(gname, ang, sectors=main); gf = FA.FastGroupK(gname, ang, sectors=main)
    worst = 0
    for s in main:
        a = gd.evaluate(x, sh, s); t1=time.time(); b = gf.evaluate(x, sh, s)
        dm = np.max(np.abs(np.sort(a["mu"]) - b["mu"])) / max(1, np.abs(a["mu"]).max())
        dl = abs(a["lam"] - b["lam"]) / max(1, abs(a["lam"]))
        worst = max(worst, dm, dl)
        print(gname, ang, s, "N", len(a["mu"]), len(b["mu"]), "dmu", f"{dm:.1e}", "dlam", f"{dl:.1e}", "ncop", a["ncopies"], b["ncopies"], "mt", a["mirror_tied"], b["mirror_tied"], flush=True)
    pa = F.phi(gd, x, sh, list(main)); pb = F.phi(gf, x, sh, list(main))
    print("PHI", gname, ang, pa, pb, "worst", worst, round(time.time()-t0), flush=True)
