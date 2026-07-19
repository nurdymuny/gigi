#!/usr/bin/env python3.11
"""GIGI matrix factorization vs a reference on real MovieLens 100k.
Fetches ml-100k, loads it into a running GIGI server, runs POST /factorize, and
compares held-out RMSE to a numpy Funk-SGD reference and the global-mean baseline.
Usage: python3.11 bench_movielens.py [server_url]"""
import json,urllib.request,time,math,sys
import numpy as np
B=sys.argv[1] if len(sys.argv)>1 else "http://localhost:3180"
def call(p,b,to=180):return json.loads(urllib.request.urlopen(urllib.request.Request(B+p,data=json.dumps(b).encode(),headers={"Content-Type":"application/json"}),timeout=to).read().decode())
raw=urllib.request.urlopen("https://files.grouplens.org/datasets/movielens/ml-100k/u.data",timeout=30).read().decode()
data=[(int(u),int(i),float(r)) for u,i,r,_ in (l.split('\t') for l in raw.strip().splitlines())]
call("/v1/gql",{"query":"DROP BUNDLE ml"}) if True else None
call("/v1/gql",{"query":"CREATE BUNDLE ml (id TEXT BASE, u TEXT FIBER, it TEXT FIBER, r NUMERIC FIBER);"})
recs=[{"id":f"x{k}","u":f"u{u}","it":f"i{i}","r":rt} for k,(u,i,rt) in enumerate(data)]
for c in range(0,len(recs),20000): call("/v1/bundles/ml/insert",{"records":recs[c:c+20000]})
def ref_mf(k,epochs=20,lr=0.02,reg=0.05):
    rng=np.random.RandomState(0); d=data[:]; rng.shuffle(d); cut=int(0.8*len(d)); tr,te=d[:cut],d[cut:]
    nu=max(u for u,_,_ in data)+1; ni=max(i for _,i,_ in data)+1
    mu=np.mean([r for _,_,r in tr]); P=rng.normal(0,0.1,(nu,k)); Q=rng.normal(0,0.1,(ni,k)); bu=np.zeros(nu); bi=np.zeros(ni)
    for _ in range(epochs):
        rng.shuffle(tr)
        for u,i,r in tr:
            e=r-(mu+bu[u]+bi[i]+P[u]@Q[i]); bu[u]+=lr*(e-reg*bu[u]); bi[i]+=lr*(e-reg*bi[i])
            Pu=P[u].copy(); P[u]+=lr*(e*Q[i]-reg*P[u]); Q[i]+=lr*(e*Pu-reg*Q[i])
    return math.sqrt(np.mean([(r-(mu+bu[u]+bi[i]+P[u]@Q[i]))**2 for u,i,r in te]))
print(f"MovieLens 100k: {len(data)} ratings\n{'rank':<6}{'GIGI':>9}{'numpy-ref':>11}{'baseline':>10}")
for k in [10,20]:
    g=call("/v1/bundles/ml/factorize",{"user":"u","item":"it","rating":"r","rank":k,"epochs":20})
    print(f"  {k:<4}{g['rmse']:>9.3f}{ref_mf(k):>11.3f}{g['baseline_rmse']:>10.3f}")
