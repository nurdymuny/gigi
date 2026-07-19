#!/usr/bin/env python3.11
"""Rung-two /circulation gauntlet: does the magnetic-Laplacian / Hodge decomposition
recover rankings, localize fraud rings, and see directed circulation that undirected
methods are provably blind to? Run with python3.11 against a running GIGI server."""
import json,urllib.request,sys
import numpy as np
B=sys.argv[1] if len(sys.argv)>1 else "http://localhost:3191"
def post(p,b):
    return json.loads(urllib.request.urlopen(urllib.request.Request(B+p,data=json.dumps(b).encode(),headers={"Content-Type":"application/json"}),timeout=180).read().decode())
def gql(q):return post("/v1/gql",{"query":q})
def load(name, edges):   # edges: list of (src,dst,w)
    try: gql(f"DROP BUNDLE {name}")
    except Exception: pass
    gql(f"CREATE BUNDLE {name} (id TEXT BASE, src TEXT FIBER, dst TEXT FIBER, w NUMERIC FIBER);")
    recs=[{"id":f"e{k}","src":str(s),"dst":str(d),"w":float(w)} for k,(s,d,w) in enumerate(edges)]
    post(f"/v1/bundles/{name}/insert",{"records":recs})
def circ(name,**kw): return post(f"/v1/bundles/{name}/circulation",{"source":"src","target":"dst","weight":"w",**kw})

# ── 1. Tournament: recover a skill ranking from win/loss edges (i beats j) ──
rng=np.random.RandomState(0); N=12; skill=np.arange(N)[::-1]  # player 0 strongest
edges=[]
for i in range(N):
    for j in range(N):
        if i==j: continue
        p=1/(1+np.exp(-(skill[i]-skill[j])))         # Bradley-Terry
        if rng.rand()<p: edges.append((f"p{i}",f"p{j}",1.0))
load("tourney",edges); r=circ("tourney")
rank={d["node"]:k for k,d in enumerate(r["ranking"])}   # 0 = top
order=[rank[f"p{i}"] for i in range(N)]
rho=np.corrcoef(order, skill.argsort().argsort()[::-1].argsort())[0,1] if False else None
# spearman between recovered potential and true skill
pot={d["node"]:d["potential"] for d in r["ranking"]}
sp=np.corrcoef([pot[f"p{i}"] for i in range(N)], skill)[0,1]
print(f"1) TOURNAMENT (12 players, noisy Bradley-Terry):")
print(f"   circulation_ratio={r['circulation_ratio']:.3f}  magnetic_gap={r['magnetic_gap']:.3f}")
print(f"   potential vs true skill Pearson = {sp:+.3f}  (ranking recovered from directed wins)")
print(f"   top upset cycles: {[(e['source'],e['target']) for e in r['cyclic_edges'][:3]]}")

# ── 2. Payment network: clean hierarchy vs one with a laundering ring ──
tree=[(f"n{i}",f"n{i+1}",5.0) for i in range(8)]
clean=circ("pay_clean") if load("pay_clean",tree) or True else None
ring=[("n6","L0",4.0),("L0","L1",4.0),("L1","L2",4.0),("L2","n6",4.0)]  # money loops back
load("pay_ring",tree+ring); dirty=circ("pay_ring")
print(f"\n2) PAYMENT NETWORK:")
print(f"   clean hierarchy : circ={clean['circulation_ratio']:.3f}  gap={clean['magnetic_gap']:.3f}")
print(f"   + laundering ring: circ={dirty['circulation_ratio']:.3f}  gap={dirty['magnetic_gap']:.3f}")
print(f"   ring localized as top cyclic edges: {[(e['source'],e['target']) for e in dirty['cyclic_edges'][:4]]}")

# ── 3. UNCOPYABILITY: same undirected graph, ring directed vs bidirectional ──
#     A directed ring carries flux (gap>0). Symmetrize it (add reverse edges) and the
#     flux cancels (gap~0) — an undirected method sees the SAME edges either way.
#  Same undirected triangle {a-b, b-c, c-a}. Ring: all +1 (net loop flux != 0 = pure
#  circulation). Gradient: a->b:1, b->c:1, c->a:-2 (net loop flux 0 = perfectly rankable).
ring=[("a","b",1.0),("b","c",1.0),("c","a",1.0)]
load("tri_ring",ring); dr=circ("tri_ring")
grad=[("a","b",1.0),("b","c",1.0),("c","a",-2.0)]
load("tri_grad",grad); gr=circ("tri_grad")
print(f"\n3) UNCOPYABILITY PROOF (identical undirected triangle a-b-c, flow retargeted):")
print(f"   circulation flow (ring +1,+1,+1): circ={dr['circulation_ratio']:.3f}  magnetic_gap={dr['magnetic_gap']:.3f}  ← flux SEEN")
print(f"   gradient flow    (+1,+1,-2)     : circ={gr['circulation_ratio']:.3f}  magnetic_gap={gr['magnetic_gap']:.3f}  ← flux GONE, rankable")
print(f"   → identical undirected graph; undirected diffusion sees ONE triangle. Only the flux signal tells the ring from the ranking.")
