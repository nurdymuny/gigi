#!/usr/bin/env python3.11
"""Full GIGI-vs-scikit-learn sweep on real datasets. Runs every method, records the
same metric for GIGI and the reference, and writes results.json for the report site.
Run against a live GIGI server (release build). python3.11 (needs sklearn)."""
import json,urllib.request,sys,math,time
import numpy as np
from sklearn import datasets
from sklearn.preprocessing import StandardScaler
from sklearn.metrics import adjusted_rand_score, average_precision_score
from sklearn.cluster import KMeans, SpectralClustering
from sklearn.mixture import GaussianMixture
from sklearn.decomposition import PCA
from sklearn.neighbors import KNeighborsClassifier, KNeighborsRegressor, LocalOutlierFactor
from sklearn.linear_model import LinearRegression, LogisticRegression
from sklearn.svm import SVC
from sklearn.ensemble import IsolationForest
from sklearn.gaussian_process import GaussianProcessRegressor
from sklearn.gaussian_process.kernels import RBF, WhiteKernel, ConstantKernel
from sklearn.model_selection import cross_val_score, train_test_split
B=sys.argv[1] if len(sys.argv)>1 else "http://localhost:3177"
def call(p,b,to=180):return json.loads(urllib.request.urlopen(urllib.request.Request(B+p,data=json.dumps(b).encode(),headers={"Content-Type":"application/json"}),timeout=to).read().decode())
def load(name,X,extra=None,vals=None):
    feat=[f"x{j}" for j in range(X.shape[1])]
    cols=", ".join(f"{f} NUMERIC FIBER" for f in feat)+((", "+extra) if extra else "")
    call("/v1/gql",{"query":f"CREATE BUNDLE {name} (id TEXT BASE, {cols});"})
    recs=[]
    for i in range(len(X)):
        r={"id":f"r{i}"};
        for j in range(len(feat)): r[feat[j]]=float(X[i][j])
        if vals: r.update(vals(i))
        recs.append(r)
    for c in range(0,len(recs),20000): call(f"/v1/bundles/{name}/insert",{"records":recs[c:c+20000]})
    return feat,[f"r{i}" for i in range(len(X))]
R=[]  # results: dataset, task, method, metric, gigi, ref, ref_name
def rec(ds,task,method,metric,g,r,rn): R.append(dict(dataset=ds,task=task,method=method,metric=metric,gigi=round(g,3),ref=round(r,3),ref_name=rn))

iris=datasets.load_iris(); wine=datasets.load_wine(); bc=datasets.load_breast_cancer(); dia=datasets.load_diabetes(); dig=datasets.load_digits()
# ---- CLUSTERING (ARI) ----
for nm,ds,k in [("iris",iris,3),("wine",wine,3),("digits",dig,10)]:
    feat,ids=load(nm+"_cl",ds.data); Xs=StandardScaler().fit_transform(ds.data); y=ds.target
    def gari(b): return adjusted_rand_score(y,[r["cluster"] for r in sorted(b["results"],key=lambda x:int(x["id"][1:]))])
    gk=call(f"/v1/bundles/{nm}_cl/cluster",{"method":"kmeans","k":k}); rec(nm,"cluster","kmeans","ARI",gari(gk),adjusted_rand_score(y,KMeans(k,n_init=10,random_state=0).fit_predict(Xs)),"sklearn KMeans")
    cov="diagonal" if nm=="digits" else "full"
    gg=call(f"/v1/bundles/{nm}_cl/cluster",{"method":"gmm","k":k,"covariance":cov}); rec(nm,"cluster","gmm","ARI",gari(gg),adjusted_rand_score(y,GaussianMixture(k,random_state=0).fit_predict(Xs)),"sklearn GMM")
    gs=call(f"/v1/bundles/{nm}_cl/cluster",{"method":"spectral","k":k,"neighbors":15,"normalized":True}); rec(nm,"cluster","spectral","ARI",gari(gs),adjusted_rand_score(y,SpectralClustering(k,affinity="nearest_neighbors",n_neighbors=15,random_state=0).fit_predict(Xs)),"sklearn Spectral")
# ---- CLASSIFICATION (accuracy) ----
for nm,ds in [("iris",iris),("wine",wine),("bcancer",bc),("digits",dig)]:
    feat,ids=load(nm+"_c",ds.data,"label TEXT FIBER",lambda i,t=ds.target:{"label":f"c{t[i]}"}); Xs=StandardScaler().fit_transform(ds.data); y=ds.target
    gk=call(f"/v1/bundles/{nm}_c/infer",{"target":"label","method":"knn","k":7})["metric"]["accuracy"]; rec(nm,"classify","knn","accuracy",gk,cross_val_score(KNeighborsClassifier(7),Xs,y,cv=5).mean(),"sklearn KNN")
    gs=call(f"/v1/bundles/{nm}_c/infer",{"target":"label","method":"svm"})["metric"]["accuracy"]; rec(nm,"classify","svm","accuracy",gs,cross_val_score(SVC(),Xs,y,cv=5).mean(),"sklearn SVC(rbf)")
# ---- REGRESSION (R²) + GP coverage ----
feat,ids=load("dia_r",dia.data,"y NUMERIC FIBER",lambda i:{"y":float(dia.target[i])}); Xs=StandardScaler().fit_transform(dia.data); y=dia.target
for m,skl,sn in [("local_linear",KNeighborsRegressor(20),"sklearn KNN-reg"),("ols",LinearRegression(),"sklearn LinReg")]:
    g=call("/v1/bundles/dia_r/infer",{"target":"y","method":m,"k":20})["metric"]["r2"]; rec("diabetes","regress",m,"R²",g,cross_val_score(skl,Xs,y,cv=5,scoring="r2").mean(),sn)
gp=call("/v1/bundles/dia_r/infer",{"target":"y","method":"gp","k":30})["metric"]
Xtr,Xte,ytr,yte=train_test_split(Xs,y,test_size=0.3,random_state=0)
skgp=GaussianProcessRegressor(kernel=ConstantKernel()*RBF()+WhiteKernel(),normalize_y=True,random_state=0).fit(Xtr,ytr)
mu,sd=skgp.predict(Xte,return_std=True); rec("diabetes","regress","gp (90% coverage)","coverage",gp["coverage_90"],float(np.mean(np.abs(yte-mu)<=1.645*sd)),"sklearn GP")
# ---- PCA (cumulative explained variance) ----
for nm,ds in [("wine",wine),("bcancer",bc),("digits",dig)]:
    feat,ids=load(nm+"_p",ds.data); Xs=StandardScaler().fit_transform(ds.data)
    g=call(f"/v1/bundles/{nm}_p/reduce",{"k":2})["cumulative_explained_variance"]; rec(nm,"pca","top-2","expl.var",g,float(PCA(2).fit(Xs).explained_variance_ratio_.sum()),"sklearn PCA")
# ---- ANOMALY (PR-AUC): breast-cancer with rare malignant ----
rng=np.random.RandomState(0); ben=np.where(bc.target==1)[0]; mal=np.where(bc.target==0)[0]
keep=np.concatenate([ben,rng.choice(mal,22,replace=False)]); Xa=bc.data[keep]; ya=(bc.target[keep]==0).astype(int)
feat,ids=load("bc_a",Xa); Xas=StandardScaler().fit_transform(Xa)
sc={r["id"]:r["score"] for r in call("/v1/bundles/bc_a/scan",{"budget":0.1,"limit":0})["results"]}
ga=average_precision_score(ya,[sc[i] for i in ids]); iso=-IsolationForest(random_state=0).fit(Xas).score_samples(Xas)
rec("bcancer-rare","anomaly","/scan","PR-AUC",ga,average_precision_score(ya,iso),"sklearn IsolationForest")
# ---- MATRIX FACTORIZATION (RMSE) on MovieLens 100k ----
try:
    raw=urllib.request.urlopen("https://files.grouplens.org/datasets/movielens/ml-100k/u.data",timeout=30).read().decode()
    data=[(int(u),int(i),float(r)) for u,i,r,_ in (l.split('\t') for l in raw.strip().splitlines())]
    call("/v1/gql",{"query":"CREATE BUNDLE ml (id TEXT BASE, u TEXT FIBER, it TEXT FIBER, r NUMERIC FIBER);"})
    recs=[{"id":f"x{k}","u":f"u{u}","it":f"i{i}","r":rt} for k,(u,i,rt) in enumerate(data)]
    for c in range(0,len(recs),20000): call("/v1/bundles/ml/insert",{"records":recs[c:c+20000]})
    g=call("/v1/bundles/ml/factorize",{"user":"u","item":"it","rating":"r","rank":10,"epochs":20})
    def refmf(k=10,ep=20,lr=0.02,reg=0.05):
        rr=np.random.RandomState(0); d=data[:]; rr.shuffle(d); cut=int(0.8*len(d)); tr,te=d[:cut],d[cut:]
        nu=max(u for u,_,_ in data)+1; ni=max(i for _,i,_ in data)+1; mu=np.mean([r for _,_,r in tr])
        P=rr.normal(0,0.1,(nu,k)); Q=rr.normal(0,0.1,(ni,k)); bu=np.zeros(nu); bi=np.zeros(ni)
        for _ in range(ep):
            rr.shuffle(tr)
            for u,i,r in tr:
                e=r-(mu+bu[u]+bi[i]+P[u]@Q[i]); bu[u]+=lr*(e-reg*bu[u]); bi[i]+=lr*(e-reg*bi[i]); Pu=P[u].copy(); P[u]+=lr*(e*Q[i]-reg*P[u]); Q[i]+=lr*(e*Pu-reg*Q[i])
        return math.sqrt(np.mean([(r-(mu+bu[u]+bi[i]+P[u]@Q[i]))**2 for u,i,r in te]))
    rec("MovieLens 100k","recommend","matrix factorization","RMSE (lower=better)",g["rmse"],refmf(),"numpy Funk-SVD")
except Exception as e:
    print("movielens skipped:",e)
json.dump(R,open("/tmp/sweep_results.json","w"),indent=1)
print(f"=== {len(R)} head-to-heads ===")
for r in R:
    better = "GIGI" if ((r["gigi"]>r["ref"]) == (r["metric"]!="RMSE (lower=better)")) and abs(r["gigi"]-r["ref"])>0.005 else ("tie" if abs(r["gigi"]-r["ref"])<=0.005 else "ref")
    print(f"  {r['dataset']:16} {r['task']:9} {r['method']:22} {r['metric']:10} GIGI={r['gigi']:.3f} {r['ref_name']}={r['ref']:.3f}  [{better}]")
