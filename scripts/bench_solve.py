#!/usr/bin/env python3.11
"""Fit-store demo: does GIGI /solve give EXACT leave-one-out over the whole ridge path
from ONE decomposition? Verify against sklearn brute-force LOO on real data (diabetes),
and time one /solve call vs sklearn refitting per alpha."""
import json,urllib.request,sys,time
import numpy as np
from sklearn import datasets
from sklearn.preprocessing import StandardScaler
from sklearn.linear_model import Ridge
B=sys.argv[1] if len(sys.argv)>1 else "http://localhost:3191"
def post(p,b):
    return json.loads(urllib.request.urlopen(urllib.request.Request(B+p,data=json.dumps(b).encode(),headers={"Content-Type":"application/json"}),timeout=180).read().decode())
def gql(q):return post("/v1/gql",{"query":q})

X,y=datasets.load_diabetes(return_X_y=True); n,d=X.shape
feat=[f"x{j}" for j in range(d)]
try: gql("DROP BUNDLE diab")
except Exception: pass
gql(f"CREATE BUNDLE diab (id TEXT BASE, {', '.join(f'{f} NUMERIC FIBER' for f in feat)}, y NUMERIC FIBER);")
recs=[{"id":f"r{i}",**{feat[j]:float(X[i][j]) for j in range(d)},"y":float(y[i])} for i in range(n)]
post("/v1/bundles/diab/insert",{"records":recs})

alphas=[0.001,0.01,0.1,1.0,3.0,10.0,30.0,100.0]
t0=time.time(); r=post("/v1/bundles/diab/solve",{"target":"y","alphas":alphas}); gigi_t=time.time()-t0

# brute-force LOO with sklearn, SAME preprocessing (standardize on full data, center y)
Xs=StandardScaler().fit_transform(X); yc=y-y.mean()
def brute_loo(a):
    p2=0.0
    for h in range(n):
        m=np.ones(n,bool); m[h]=False
        rid=Ridge(alpha=a,fit_intercept=False).fit(Xs[m],yc[m])
        p2+=(yc[h]-rid.predict(Xs[h:h+1])[0])**2
    return np.sqrt(p2/n)
t0=time.time(); sk={a:brute_loo(a) for a in alphas}; sk_t=time.time()-t0

print(f"diabetes: n={n}, d={d}\n")
print(f"{'alpha':>8}{'GIGI exact LOO':>16}{'sklearn brute LOO':>20}{'|diff|':>12}{'train_R2':>10}{'df':>7}")
gp={p['alpha']:p for p in r['ridge_path']}
maxdiff=0
for a in alphas:
    g=gp[a]; diff=abs(g['loo_rmse']-sk[a]); maxdiff=max(maxdiff,diff)
    print(f"{a:>8.3f}{g['loo_rmse']:>16.5f}{sk[a]:>20.5f}{diff:>12.2e}{g['train_r2']:>10.3f}{g['effective_df']:>7.2f}")
print(f"\nmax |GIGI − sklearn| over the path = {maxdiff:.2e}  (exact identity, not approximate)")
print(f"GIGI best_alpha={r['best_alpha']} (LOO {r['best_loo_rmse']:.4f}) chosen from the exact curve")
print(f"\ntiming: GIGI 1 call (SVD + whole {len(alphas)}-pt path + exact LOO) = {gigi_t*1000:.1f} ms")
print(f"        sklearn brute LOO ({len(alphas)} alphas × {n} refits = {len(alphas)*n} fits) = {sk_t*1000:.0f} ms")
print(f"        speedup ≈ {sk_t/max(gigi_t,1e-6):.0f}×")
