import sys, time, numpy as np, scipy.sparse.linalg as spla
sys.path.insert(0,'scripts')
import edge_wp8_fast as FA, edge_wp8_two_new_groups as G
c = next(c for c in G.circles() if c["label"] == "g0")
g = FA.CheckGroupK(c["group"], c["angle"], eK_torus=c["eK"], sectors={})
main, extra = FA.sector_specs(g, c["group"]); FA.add_sectors(g, {"77s": extra["77s"]})
t=time.time(); ops = g.ops(-1.2,(0.5,0.5,0.25),"77s"); print("ops", round(time.time()-t,1))
t=time.time(); Hm = FA.MatFreeH(ops); print("matfree build", round(time.time()-t,1), "nnz D", Hm.D.nnz)
v=np.random.default_rng(0).standard_normal(Hm.D.shape[1])+0j
t=time.time(); [Hm(v) for _ in range(10)]; print("10 matvecs", round(time.time()-t,2))
t=time.time(); ok=g.sign_fit((0.5,0.5,0.25),"77s"); print("sign fit", round(time.time()-t,1))
