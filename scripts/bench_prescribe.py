#!/usr/bin/env python3.11
"""GIGI /prescribe geometric-diagnostic gauntlet on real datasets. For each dataset it
loads the data via REST, calls POST /prescribe, and prints the geometric fingerprint +
reads, annotated with what we KNOW the geometry to be (a sanity check, not a score).
Run with python3.11 against a running GIGI server."""
import json,urllib.request,sys
import numpy as np
from sklearn import datasets
B=sys.argv[1] if len(sys.argv)>1 else "http://localhost:3191"
def post(p,b):
    return json.loads(urllib.request.urlopen(urllib.request.Request(B+p,data=json.dumps(b).encode(),headers={"Content-Type":"application/json"}),timeout=180).read().decode())
def gql(q):return post("/v1/gql",{"query":q})
def load(name,X):
    feat=[f"x{j}" for j in range(X.shape[1])]
    try: gql(f"DROP BUNDLE {name}")
    except Exception: pass
    cols=", ".join(f"{f} NUMERIC FIBER" for f in feat)
    gql(f"CREATE BUNDLE {name} (id TEXT BASE, {cols});")
    recs=[{"id":f"r{i}",**{feat[j]:float(X[i][j]) for j in range(len(feat))}} for i in range(len(X))]
    post(f"/v1/bundles/{name}/insert",{"records":recs})

# real + synthetic, with the known geometry each SHOULD read as
sets=[]
sets.append(("iris",datasets.load_iris().data,"low-dim, a few clusters"))
sets.append(("wine",datasets.load_wine().data,"moderate dim, mild clusters"))
sets.append(("bcancer",datasets.load_breast_cancer().data,"compressible (corr features), 2 groups"))
sets.append(("digits",datasets.load_digits().data,"high-dim CURVED manifold, 10 clusters"))
sets.append(("diabetes",datasets.load_diabetes().data,"near-linear, full-rank (regression)"))
Xm,_=datasets.make_moons(400,noise=0.08,random_state=1); sets.append(("moons",Xm,"2-D, curved, 2 interlocking groups"))
Xb,_=datasets.make_blobs(500,centers=4,cluster_std=0.7,random_state=1); sets.append(("blobs",Xb,"flat, CLEAR clusters"))
Xs,_=datasets.make_swiss_roll(600,noise=0.05,random_state=1); sets.append(("swiss",Xs,"curved 2-D sheet in 3-D"))

print(f"{'dataset':10}{'n':>5}{'dim':>5}{'curv':>7}{'dimr':>7}{'clarity':>9}   expected geometry")
print("-"*95)
for name,X,expect in sets:
    load(name,X)
    r=post(f"/v1/bundles/{name}/prescribe",{"neighbors":10,"sample":500})
    fp=r["fingerprint"]
    print(f"{name:10}{r['n']:>5}{fp['ambient_dim']:>5}{fp['curvature']:>7.3f}{fp['dim_ratio']:>7.2f}{fp['spectral_clarity']:>9.3f}   {expect}")
    for rd in r["reads"]: print(f"            · {rd}")
    for rec in r["recommendations"]: print(f"            → {rec}")
    print()
