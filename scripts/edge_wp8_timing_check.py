# timing only on G groups: prints NO eigenvalue, margin or spectrum
import sys, time, resource, numpy as np
sys.path.insert(0,'scripts')
import edge_wp8_fast as FA, edge_wp8_two_new_groups as G
for lab in ("g0", "s4"):
    c = next(c for c in G.circles() if c["label"] == lab)
    g = FA.CheckGroupK(c["group"], c["angle"], eK_torus=c["eK"], sectors={})
    main, extra = FA.sector_specs(g, c["group"]); FA.add_sectors(g, extra); g.check = tuple(extra)
    for s in extra:
        t0 = time.time(); g.evaluate(-1.2, (0.5,0.5,0.25), s); t1 = time.time(); g.evaluate(-1.0, (0.5,0.5,0.25), s); t2 = time.time()
        print(lab, s, "N", g.d*g.d*g.reps[s][0].shape[0], f"first {t1-t0:.0f}s second {t2-t1:.0f}s", "maxrss", f"{resource.getrusage(resource.RUSAGE_SELF).ru_maxrss/1e9:.1f}GB", flush=True)
        g.cache.clear()
