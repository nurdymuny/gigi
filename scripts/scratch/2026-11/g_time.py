import sys, time, numpy as np, resource
sys.path.insert(0, 'scripts')
import edge_wp8_fast as FA
for gname in sys.argv[1:]:
    g = FA.FastGroupK(gname, 0.0, sectors={})
    main, extra = FA.sector_specs(g, gname)
    t0=time.time(); FA.add_sectors(g, {**main, **extra}); print(gname, "sectors built", round(time.time()-t0,1), {k: g.reps[k][0].shape[0] for k in g.reps}, flush=True)
    for k in g.reps: print("  repcheck", k, f"{FA.rep_check(g,k):.1e}", flush=True)
    for k in list(main) + list(extra):
        t0=time.time(); pr = g.prepare((0.5,0.5,0.25), k); t1=time.time()
        ev = g.evaluate(-1.2, (0.5,0.5,0.25), k); t2=time.time()
        bs = sorted(len(b["idx"]) for b in pr["blocks"].values())
        print(f"  {k}: N {pr['N']} prepare {t1-t0:.1f}s evaluate {t2-t1:.1f}s maxblock {bs[-1]} nblocks {len(bs)} topdim {ev['top_dim']} maxrss {resource.getrusage(resource.RUSAGE_SELF).ru_maxrss/1e9:.1f}GB", flush=True)
        g.cache.clear()
