#!/usr/bin/env python3.11
"""GIGI vs scikit-learn head-to-head on real (sklearn-bundled) datasets.
Run with python3.11 against a running GIGI server. Loads each dataset via REST,
calls the GIGI endpoint, runs the sklearn baseline, prints the same metric for both.
Honest: whatever the numbers are, they print. Reproducible bench for the mission."""
import json,urllib.request,sys
import numpy as np
from sklearn import datasets
from sklearn.metrics import adjusted_rand_score, average_precision_score
from sklearn.cluster import KMeans, SpectralClustering, DBSCAN
from sklearn.mixture import GaussianMixture
from sklearn.decomposition import PCA
from sklearn.neighbors import KNeighborsClassifier, KNeighborsRegressor, LocalOutlierFactor
from sklearn.linear_model import LinearRegression, LogisticRegression
from sklearn.ensemble import IsolationForest
from sklearn.model_selection import cross_val_score
from sklearn.preprocessing import StandardScaler
B=sys.argv[1] if len(sys.argv)>1 else "http://localhost:3190"
def post(p,b):
    return json.loads(urllib.request.urlopen(urllib.request.Request(B+p,data=json.dumps(b).encode(),headers={"Content-Type":"application/json"}),timeout=180).read().decode())
def gql(q):return post("/v1/gql",{"query":q})
def load(name, X, feat, extra=None):
    try: gql(f"DROP BUNDLE {name}")
    except Exception: pass
    cols=", ".join(f"{f} NUMERIC FIBER" for f in feat)
    if extra: cols += ", " + ", ".join(f"{k} {t}" for k,t in extra)
    gql(f"CREATE BUNDLE {name} (id TEXT BASE, {cols});")
    recs=[]
    for i in range(len(X)):
        r={"id":f"r{i}"}; 
        for j in range(len(feat)): r[feat[j]]=float(X[i][j])
        recs.append(r)
    return recs, [f"r{i}" for i in range(len(X))]

def row(label, g, s, better_hi=True):
    mark = "  ⇐ GIGI" if (g>s)==better_hi and abs(g-s)>0.005 else ("  (tie)" if abs(g-s)<=0.005 else "")
    print(f"  {label:<22}{g:>8.3f}{s:>10.3f}{mark}")

def clf_bench(name, X, y):
    feat=[f"x{j}" for j in range(X.shape[1])]
    recs,ids=load(name+"_c", X, feat, extra=[("label","TEXT FIBER")])
    for i,r in enumerate(recs): r["label"]=f"c{y[i]}"
    post(f"/v1/bundles/{name}_c/insert",{"records":recs})
    Xs=StandardScaler().fit_transform(X)
    print(f"\n== CLASSIFY: {name} (n={len(X)}, dim={X.shape[1]}, {len(set(y))} classes) — 5-fold accuracy ==")
    g=post(f"/v1/bundles/{name}_c/infer",{"target":"label","k":7})
    ga=g["metric"]["accuracy"]
    sk=cross_val_score(KNeighborsClassifier(7),Xs,y,cv=5).mean()
    row("kNN (GIGI vs sklearn)", ga, sk)
    lr=cross_val_score(LogisticRegression(max_iter=2000),Xs,y,cv=5).mean()
    print(f"  {'(sklearn LogReg ref)':<22}{'':>8}{lr:>10.3f}")

def reg_bench(name, X, y):
    feat=[f"x{j}" for j in range(X.shape[1])]
    recs,ids=load(name+"_r", X, feat, extra=[("y","NUMERIC FIBER")])
    for i,r in enumerate(recs): r["y"]=float(y[i])
    post(f"/v1/bundles/{name}_r/insert",{"records":recs})
    Xs=StandardScaler().fit_transform(X)
    print(f"\n== REGRESS: {name} (n={len(X)}, dim={X.shape[1]}) — 5-fold R² ==")
    for m,skl in [("local_linear",KNeighborsRegressor(20)),("knn",KNeighborsRegressor(20)),("ols",LinearRegression())]:
        g=post(f"/v1/bundles/{name}_r/infer",{"target":"y","method":m,"k":20})
        ga=g["metric"]["r2"]
        sk=cross_val_score(skl,Xs,y,cv=5,scoring="r2").mean()
        skname="sklearn KNN-reg" if m!="ols" else "sklearn LinReg"
        row(f"{m} vs {skname}", ga, sk)

def pca_bench(name, X):
    feat=[f"x{j}" for j in range(X.shape[1])]
    recs,ids=load(name+"_p", X, feat)
    post(f"/v1/bundles/{name}_p/insert",{"records":recs})
    Xs=StandardScaler().fit_transform(X)
    print(f"\n== PCA: {name} (n={len(X)}, dim={X.shape[1]}) — cumulative explained variance (k=2) ==")
    g=post(f"/v1/bundles/{name}_p/reduce",{"k":2})
    ga=g["cumulative_explained_variance"]
    sk=PCA(2).fit(Xs).explained_variance_ratio_.sum()
    row("PCA top-2 (GIGI vs sklearn)", ga, sk)

def anom_bench(name, X, y_anom):
    feat=[f"x{j}" for j in range(X.shape[1])]
    recs,ids=load(name+"_a", X, feat)
    post(f"/v1/bundles/{name}_a/insert",{"records":recs})
    Xs=StandardScaler().fit_transform(X)
    print(f"\n== ANOMALY: {name} (n={len(X)}, {int(y_anom.sum())} anomalies={y_anom.mean():.1%}) — PR-AUC ==")
    g=post(f"/v1/bundles/{name}_a/scan",{"budget":0.1,"limit":0})
    sc={r["id"]:r["score"] for r in g["results"]}
    ga=average_precision_score(y_anom,[sc[i] for i in ids])
    iso=-IsolationForest(random_state=0).fit(Xs).score_samples(Xs)
    si=average_precision_score(y_anom,iso)
    lof=-LocalOutlierFactor(n_neighbors=20).fit(Xs).negative_outlier_factor_
    sl=average_precision_score(y_anom,lof)
    row("GIGI /scan vs IsoForest", ga, si)
    print(f"  {'(sklearn LOF ref)':<22}{'':>8}{sl:>10.3f}")

iris=datasets.load_iris(); wine=datasets.load_wine(); bc=datasets.load_breast_cancer(); dia=datasets.load_diabetes()
clf_bench("iris", iris.data, iris.target); clf_bench("wine", wine.data, wine.target); clf_bench("bcancer", bc.data, bc.target)
reg_bench("diabetes", dia.data, dia.target)
pca_bench("wine", wine.data); pca_bench("bcancer", bc.data)
# anomaly: benign=normal, downsample malignant to ~6% as rare anomalies
rng=np.random.RandomState(0); ben=np.where(bc.target==1)[0]; mal=np.where(bc.target==0)[0]
keep=np.concatenate([ben, rng.choice(mal, 22, replace=False)])
Xa=bc.data[keep]; ya=(bc.target[keep]==0).astype(int)
anom_bench("bcancer", Xa, ya)
