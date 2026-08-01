#!/usr/bin/env python
"""Multi-seed GIGI-vs-scikit-learn sweep (evidence rules items 07/08).

Re-runs the full 25-cell head-to-head set from scripts/full_sweep.py across
>= 5 fixed, recorded seeds (default 20260801..20260805), PLUS the two
previously embargoed cells:

  (a) digits eigenspace representation — the SAME GMM (one shared code path,
      src/ml/cluster.rs::gmm_em, identical covariance=diagonal, init, reg,
      convergence) on raw pixels (/cluster method=gmm) vs in the spectral
      eigenspace (/cluster method=spectral head=gmm embed_dim=20
      covariance=diagonal). Controlled representation experiment behind the
      catalog-attested eigenspace-recovery claim. (Before 2026-08-01 the
      spectral head had a separate compact GMM — full covariance, fixed 60
      iterations, reg 1e-4 — a confound the artifact verification flagged;
      the head now calls gmm_em itself.)
  (b) digits label-propagation vs flat kNN — /infer method=diffusion vs the
      flat sklearn kNN baseline (the 0.973 vs 0.944 claim).

Per cell, per arm: ALL raw per-seed values, mean, std (sample std, ddof=1),
and a verdict from the gigi perspective:

  WIN    signed delta (gigi better) >  max(combined_std, 0.005)
  LOSS   signed delta (gigi better) < -max(combined_std, 0.005)
  PARITY otherwise ("within noise")

combined_std = sqrt(std_gigi^2 + std_ref^2). The 0.005 floor exists because
many cells are deterministic in BOTH arms (std 0 on both sides); without a
floor any hair-width delta would count as WIN/LOSS. 0.005 matches the tie
threshold the single-run artifact (scripts/sweep_results.json) used.

Determinism facts (verified in source, recorded honestly, never assumed):
  * The gigi server has NO seed parameter — /cluster and /infer use fixed LCG
    constants (src/ml/cluster.rs, src/ml/infer.rs) — AND record iteration
    order is stable: Bundle::records() yields base-point-sorted records
    (src/bundle.rs). Both are required: before 2026-08-01, records() iterated
    a HashMap whose order is per-process random, so gigi /cluster results
    reproduced only WITHIN one server process (caught by artifact
    verification; fixed the same day). The gigi arm is now deterministic for
    fixed data ACROSS server restarts; we still CALL the server once per seed
    and record what it actually returned, so std 0 is measured, not asserted.
  * Seed variation enters through: sklearn estimators that accept random_state
    (KMeans, GMM, SpectralClustering, IsolationForest, GP, train_test_split),
    the numpy Funk-SVD reference, and seed-dependent data construction (the
    bcancer-rare malignant subsample — which varies BOTH arms of that cell).
  * cross_val_score(cv=5) uses unshuffled (Stratified)KFold — deterministic by
    protocol, exactly as in the single-run artifact. Reported with std 0.

Server lifecycle (managed + verified by this script):
  * Builds target/release/gigi-stream if missing (cargo build --release).
  * Refuses to run if anything already answers on the port (no foreign server).
  * Starts gigi-stream with PORT=<port> and a FRESH scratch GIGI_DATA_DIR
    (tempfile.mkdtemp), polls /v1/health until 200 "ok".
  * Verifies the process is still alive after the run, terminates it, records
    the exit code, and removes the scratch dir. Server stderr goes to a log
    file whose path is recorded in the artifact meta.

All timed/quality runs are strictly serial: one process, one thread, one
HTTP request in flight at a time.

Usage:
  python scripts/multiseed_sweep.py                # full 27-cell x 5-seed run
  python scripts/multiseed_sweep.py --smoke        # 1 cell (iris kmeans) x 2 seeds,
                                                   # writes *_smoke.json — harness validation
  python scripts/multiseed_sweep.py --seeds 20260801,20260802,...
Output: scripts/sweep_results_multiseed.json (or --out).
"""
import argparse
import json
import math
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request
import urllib.error

import numpy as np
from sklearn import datasets
from sklearn.preprocessing import StandardScaler
from sklearn.metrics import adjusted_rand_score, average_precision_score
from sklearn.cluster import KMeans, SpectralClustering
from sklearn.mixture import GaussianMixture
from sklearn.decomposition import PCA
from sklearn.neighbors import KNeighborsClassifier, KNeighborsRegressor
from sklearn.linear_model import LinearRegression
from sklearn.svm import SVC
from sklearn.ensemble import IsolationForest
from sklearn.gaussian_process import GaussianProcessRegressor
from sklearn.gaussian_process.kernels import RBF, WhiteKernel, ConstantKernel
from sklearn.model_selection import cross_val_score, train_test_split

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEFAULT_SEEDS = [20260801, 20260802, 20260803, 20260804, 20260805]
TIE_FLOOR = 0.005
ML100K_URL = "https://files.grouplens.org/datasets/movielens/ml-100k/u.data"
ML100K_CACHE = os.path.join(REPO, "scripts", ".cache_ml100k_u.data")


# ── HTTP helpers ─────────────────────────────────────────────────────────────

class Gigi:
    def __init__(self, base):
        self.base = base

    def call(self, path, body, timeout=300):
        req = urllib.request.Request(
            self.base + path,
            data=json.dumps(body).encode(),
            headers={"Content-Type": "application/json"})
        return json.loads(urllib.request.urlopen(req, timeout=timeout).read().decode())

    def load(self, name, X, extra=None, vals=None):
        """CREATE BUNDLE <name> with x0..x{d-1} NUMERIC FIBER (+extra) and insert X."""
        feat = [f"x{j}" for j in range(X.shape[1])]
        cols = ", ".join(f"{f} NUMERIC FIBER" for f in feat) + ((", " + extra) if extra else "")
        self.call("/v1/gql", {"query": f"CREATE BUNDLE {name} (id TEXT BASE, {cols});"})
        recs = []
        for i in range(len(X)):
            r = {"id": f"r{i}"}
            for j in range(len(feat)):
                r[feat[j]] = float(X[i][j])
            if vals:
                r.update(vals(i))
            recs.append(r)
        for c in range(0, len(recs), 20000):
            self.call(f"/v1/bundles/{name}/insert", {"records": recs[c:c + 20000]})
        return feat, [f"r{i}" for i in range(len(X))]


# ── Server lifecycle ─────────────────────────────────────────────────────────

class GigiServer:
    def __init__(self, port):
        self.port = port
        self.base = f"http://localhost:{port}"
        self.proc = None
        self.data_dir = None
        self.log_path = None
        self.log_fh = None
        self.exit_code = None
        exe = "gigi-stream.exe" if os.name == "nt" else "gigi-stream"
        self.binary = os.path.join(REPO, "target", "release", exe)

    def _port_in_use(self):
        try:
            with socket.create_connection(("localhost", self.port), timeout=2):
                return True
        except OSError:
            return False

    def _health(self):
        try:
            with urllib.request.urlopen(self.base + "/v1/health", timeout=5) as r:
                return r.status == 200
        except urllib.error.HTTPError as e:
            return False  # 503 while loading
        except OSError:
            return False

    def start(self):
        if self._port_in_use():
            raise RuntimeError(
                f"port {self.port} already has a listener — refusing to run against "
                "a server this script does not own. Stop it or pick another --port.")
        if not os.path.exists(self.binary):
            print(f"[server] release binary missing — building gigi-stream ...", flush=True)
            subprocess.run(["cargo", "build", "--release", "--bin", "gigi-stream"],
                           cwd=REPO, check=True)
        self.data_dir = tempfile.mkdtemp(prefix="gigi_msweep_data_")
        self.log_path = self.data_dir + ".server.log"
        self.log_fh = open(self.log_path, "wb")
        env = dict(os.environ)
        env["PORT"] = str(self.port)
        env["GIGI_DATA_DIR"] = self.data_dir
        # fresh scratch dir: no API key, no rate limit (GIGI_RATE_LIMIT default 0)
        env.pop("GIGI_API_KEY", None)
        env.pop("GIGI_JWT_SECRET", None)
        print(f"[server] starting {self.binary}", flush=True)
        print(f"[server]   PORT={self.port}  GIGI_DATA_DIR={self.data_dir}", flush=True)
        self.proc = subprocess.Popen([self.binary], cwd=REPO, env=env,
                                     stdout=self.log_fh, stderr=self.log_fh)
        deadline = time.time() + 120
        while time.time() < deadline:
            if self.proc.poll() is not None:
                raise RuntimeError(
                    f"gigi-stream exited during startup (code {self.proc.returncode}); "
                    f"see {self.log_path}")
            if self._health():
                print(f"[server] healthy at {self.base}/v1/health", flush=True)
                return
            time.sleep(0.5)
        raise RuntimeError(f"server never became healthy within 120s; see {self.log_path}")

    def verify_alive(self):
        if self.proc is None or self.proc.poll() is not None:
            raise RuntimeError(
                f"gigi-stream died mid-run (code {None if self.proc is None else self.proc.returncode}); "
                f"see {self.log_path}")

    def stop(self):
        if self.proc is None:
            return
        alive_at_stop = self.proc.poll() is None
        if alive_at_stop:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=30)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait(timeout=30)
        self.exit_code = self.proc.returncode
        print(f"[server] stopped (alive_at_stop={alive_at_stop}, exit={self.exit_code})", flush=True)
        if self.log_fh:
            self.log_fh.close()
        # scratch data dir cleanup (retry: Windows can hold locks briefly)
        for attempt in range(5):
            try:
                shutil.rmtree(self.data_dir)
                print(f"[server] scratch data dir removed", flush=True)
                return
            except OSError:
                time.sleep(1.0)
        print(f"[server] WARNING: could not remove scratch dir {self.data_dir}", flush=True)


# ── Results assembly ─────────────────────────────────────────────────────────

def stats(values):
    vals = [float(v) for v in values]
    mean = float(np.mean(vals))
    std = float(np.std(vals, ddof=1)) if len(vals) > 1 else 0.0
    return mean, std


def make_cell(dataset, task, method, metric, direction, ref_name, config=None, embargoed=False):
    return {
        "dataset": dataset, "task": task, "method": method, "metric": metric,
        "direction": direction,           # "higher" or "lower" (= better)
        "config": config or {},
        "embargoed": embargoed,
        "arms": {"gigi": {"name": "gigi", "values": []},
                 "ref": {"name": ref_name, "values": []}},
        "extra_arms": {},
    }


def add_extra_arm(cell, key, name):
    cell["extra_arms"][key] = {"name": name, "values": []}


def finalize(cell):
    for arm in list(cell["arms"].values()) + list(cell["extra_arms"].values()):
        arm["mean"], arm["std"] = stats(arm["values"])
    g, r = cell["arms"]["gigi"], cell["arms"]["ref"]
    signed = (g["mean"] - r["mean"]) if cell["direction"] == "higher" else (r["mean"] - g["mean"])
    combined = math.sqrt(g["std"] ** 2 + r["std"] ** 2)
    thr = max(combined, TIE_FLOOR)
    cell["delta_mean"] = g["mean"] - r["mean"]
    cell["signed_delta_gigi_better"] = signed
    cell["combined_std"] = combined
    cell["threshold"] = thr
    cell["verdict"] = "WIN" if signed > thr else ("LOSS" if signed < -thr else "PARITY")
    return cell


# ── The sweep ────────────────────────────────────────────────────────────────

def run(gg, seeds, smoke):
    """Returns list of finalized cells. Strictly serial throughout."""
    t_all = time.time()
    cells = []

    def log(msg):
        print(f"[{time.time()-t_all:8.1f}s] {msg}", flush=True)

    iris = datasets.load_iris(); wine = datasets.load_wine()
    bc = datasets.load_breast_cancer(); dia = datasets.load_diabetes()
    dig = datasets.load_digits()

    # ---- CLUSTERING (ARI): 9 cells --------------------------------------
    cluster_sets = [("iris", iris, 3)] if smoke else [("iris", iris, 3), ("wine", wine, 3), ("digits", dig, 10)]
    cl_cells = {}
    for nm, ds, k in cluster_sets:
        gg.load(nm + "_cl", ds.data)
        log(f"loaded bundle {nm}_cl ({len(ds.data)} records)")
        methods = ["kmeans"] if smoke else ["kmeans", "gmm", "spectral"]
        for m in methods:
            cfg = {"k": k}
            if m == "gmm":
                cfg["covariance"] = "diagonal" if nm == "digits" else "full"
            if m == "spectral":
                cfg.update({"neighbors": 15, "normalized": True})
            refname = {"kmeans": "sklearn KMeans", "gmm": "sklearn GMM",
                       "spectral": "sklearn Spectral"}[m]
            cl_cells[(nm, m)] = make_cell(nm, "cluster", m, "ARI", "higher", refname, cfg)

    def gari(nm, resp, y):
        lab = [r["cluster"] for r in sorted(resp["results"], key=lambda x: int(x["id"][1:]))]
        return adjusted_rand_score(y, lab)

    for seed in seeds:
        for nm, ds, k in cluster_sets:
            Xs = StandardScaler().fit_transform(ds.data); y = ds.target
            for m in (["kmeans"] if smoke else ["kmeans", "gmm", "spectral"]):
                cell = cl_cells[(nm, m)]
                gg_body = {"method": m, "k": k}
                if m == "gmm":
                    gg_body["covariance"] = cell["config"]["covariance"]
                if m == "spectral":
                    gg_body.update({"neighbors": 15, "normalized": True})
                gv = gari(nm, gg.call(f"/v1/bundles/{nm}_cl/cluster", gg_body), y)
                if m == "kmeans":
                    rv = adjusted_rand_score(y, KMeans(k, n_init=10, random_state=seed).fit_predict(Xs))
                elif m == "gmm":
                    rv = adjusted_rand_score(y, GaussianMixture(k, random_state=seed).fit_predict(Xs))
                else:
                    rv = adjusted_rand_score(y, SpectralClustering(
                        k, affinity="nearest_neighbors", n_neighbors=15,
                        random_state=seed).fit_predict(Xs))
                cell["arms"]["gigi"]["values"].append(round(float(gv), 6))
                cell["arms"]["ref"]["values"].append(round(float(rv), 6))
                log(f"seed {seed} cluster/{nm}/{m}: gigi={gv:.4f} ref={rv:.4f}")
    cells += [finalize(c) for c in cl_cells.values()]
    if smoke:
        return cells

    # ---- CLASSIFICATION (accuracy): 8 cells -----------------------------
    clf_sets = [("iris", iris), ("wine", wine), ("bcancer", bc), ("digits", dig)]
    clf_cells = {}
    for nm, ds in clf_sets:
        gg.load(nm + "_c", ds.data, "label TEXT FIBER",
                lambda i, t=ds.target: {"label": f"c{t[i]}"})
        log(f"loaded bundle {nm}_c")
        clf_cells[(nm, "knn")] = make_cell(nm, "classify", "knn", "accuracy", "higher",
                                           "sklearn KNN", {"k": 7})
        clf_cells[(nm, "svm")] = make_cell(nm, "classify", "svm", "accuracy", "higher",
                                           "sklearn SVC(rbf)", {})
    for seed in seeds:
        for nm, ds in clf_sets:
            Xs = StandardScaler().fit_transform(ds.data); y = ds.target
            gk = gg.call(f"/v1/bundles/{nm}_c/infer",
                         {"target": "label", "method": "knn", "k": 7})["metric"]["accuracy"]
            rk = float(cross_val_score(KNeighborsClassifier(7), Xs, y, cv=5).mean())
            gs = gg.call(f"/v1/bundles/{nm}_c/infer",
                         {"target": "label", "method": "svm"})["metric"]["accuracy"]
            rs = float(cross_val_score(SVC(), Xs, y, cv=5).mean())
            clf_cells[(nm, "knn")]["arms"]["gigi"]["values"].append(round(float(gk), 6))
            clf_cells[(nm, "knn")]["arms"]["ref"]["values"].append(round(rk, 6))
            clf_cells[(nm, "svm")]["arms"]["gigi"]["values"].append(round(float(gs), 6))
            clf_cells[(nm, "svm")]["arms"]["ref"]["values"].append(round(rs, 6))
            log(f"seed {seed} classify/{nm}: knn g={gk:.4f} r={rk:.4f} | svm g={gs:.4f} r={rs:.4f}")
    cells += [finalize(c) for c in clf_cells.values()]

    # ---- REGRESSION (R^2) + GP coverage: 3 cells ------------------------
    gg.load("dia_r", dia.data, "y NUMERIC FIBER", lambda i: {"y": float(dia.target[i])})
    log("loaded bundle dia_r")
    reg_cells = {
        "local_linear": make_cell("diabetes", "regress", "local_linear", "R²", "higher",
                                  "sklearn KNN-reg", {"k": 20}),
        "ols": make_cell("diabetes", "regress", "ols", "R²", "higher",
                         "sklearn LinReg", {"k": 20}),
        "gp": make_cell("diabetes", "regress", "gp (90% coverage)", "coverage", "higher",
                        "sklearn GP", {"k": 30, "test_size": 0.3}),
    }
    for seed in seeds:
        Xs = StandardScaler().fit_transform(dia.data); y = dia.target
        for m, skl in [("local_linear", KNeighborsRegressor(20)), ("ols", LinearRegression())]:
            g = gg.call("/v1/bundles/dia_r/infer",
                        {"target": "y", "method": m, "k": 20})["metric"]["r2"]
            r = float(cross_val_score(skl, Xs, y, cv=5, scoring="r2").mean())
            reg_cells[m]["arms"]["gigi"]["values"].append(round(float(g), 6))
            reg_cells[m]["arms"]["ref"]["values"].append(round(r, 6))
            log(f"seed {seed} regress/{m}: gigi={g:.4f} ref={r:.4f}")
        gcov = gg.call("/v1/bundles/dia_r/infer",
                       {"target": "y", "method": "gp", "k": 30})["metric"]["coverage_90"]
        Xtr, Xte, ytr, yte = train_test_split(Xs, y, test_size=0.3, random_state=seed)
        skgp = GaussianProcessRegressor(kernel=ConstantKernel() * RBF() + WhiteKernel(),
                                        normalize_y=True, random_state=seed).fit(Xtr, ytr)
        mu, sd = skgp.predict(Xte, return_std=True)
        rcov = float(np.mean(np.abs(yte - mu) <= 1.645 * sd))
        reg_cells["gp"]["arms"]["gigi"]["values"].append(round(float(gcov), 6))
        reg_cells["gp"]["arms"]["ref"]["values"].append(round(rcov, 6))
        log(f"seed {seed} regress/gp: gigi={gcov:.4f} ref={rcov:.4f}")
    cells += [finalize(c) for c in reg_cells.values()]

    # ---- PCA (cumulative explained variance): 3 cells --------------------
    pca_cells = {}
    for nm, ds in [("wine", wine), ("bcancer", bc), ("digits", dig)]:
        gg.load(nm + "_p", ds.data)
        log(f"loaded bundle {nm}_p")
        pca_cells[nm] = make_cell(nm, "pca", "top-2", "expl.var", "higher",
                                  "sklearn PCA", {"k": 2})
    for seed in seeds:
        for nm, ds in [("wine", wine), ("bcancer", bc), ("digits", dig)]:
            Xs = StandardScaler().fit_transform(ds.data)
            g = gg.call(f"/v1/bundles/{nm}_p/reduce", {"k": 2})["cumulative_explained_variance"]
            r = float(PCA(2).fit(Xs).explained_variance_ratio_.sum())
            pca_cells[nm]["arms"]["gigi"]["values"].append(round(float(g), 6))
            pca_cells[nm]["arms"]["ref"]["values"].append(round(r, 6))
            log(f"seed {seed} pca/{nm}: gigi={g:.4f} ref={r:.4f}")
    cells += [finalize(c) for c in pca_cells.values()]

    # ---- ANOMALY (PR-AUC): 1 cell — seed drives the malignant subsample --
    # (both arms vary with seed here; a fresh per-seed bundle isolates each draw)
    an_cell = make_cell("bcancer-rare", "anomaly", "/scan", "PR-AUC", "higher",
                        "sklearn IsolationForest",
                        {"budget": 0.1, "n_malignant": 22,
                         "note": "malignant subsample drawn with RandomState(seed) — both arms vary"})
    for seed in seeds:
        rng = np.random.RandomState(seed)
        ben = np.where(bc.target == 1)[0]; mal = np.where(bc.target == 0)[0]
        keep = np.concatenate([ben, rng.choice(mal, 22, replace=False)])
        Xa = bc.data[keep]; ya = (bc.target[keep] == 0).astype(int)
        bname = f"bc_a_s{seed}"
        _, ids = gg.load(bname, Xa)
        Xas = StandardScaler().fit_transform(Xa)
        sc = {r["id"]: r["score"]
              for r in gg.call(f"/v1/bundles/{bname}/scan", {"budget": 0.1, "limit": 0})["results"]}
        gv = float(average_precision_score(ya, [sc[i] for i in ids]))
        iso = -IsolationForest(random_state=seed).fit(Xas).score_samples(Xas)
        rv = float(average_precision_score(ya, iso))
        an_cell["arms"]["gigi"]["values"].append(round(gv, 6))
        an_cell["arms"]["ref"]["values"].append(round(rv, 6))
        log(f"seed {seed} anomaly/scan: gigi={gv:.4f} ref={rv:.4f}")
    cells.append(finalize(an_cell))

    # ---- MATRIX FACTORIZATION (RMSE) on MovieLens 100k: 1 cell ----------
    mf_cell = make_cell("MovieLens 100k", "recommend", "matrix factorization",
                        "RMSE (lower=better)", "lower", "numpy Funk-SVD",
                        {"rank": 10, "epochs": 20, "ref_lr": 0.02, "ref_reg": 0.05})
    try:
        if os.path.exists(ML100K_CACHE):
            raw = open(ML100K_CACHE, encoding="utf-8").read()
            mf_cell["config"]["data_source"] = "cache"
        else:
            raw = urllib.request.urlopen(ML100K_URL, timeout=30).read().decode()
            with open(ML100K_CACHE, "w", encoding="utf-8") as f:
                f.write(raw)
            mf_cell["config"]["data_source"] = "downloaded"
        data = [(int(u), int(i), float(r))
                for u, i, r, _ in (l.split("\t") for l in raw.strip().splitlines())]
        gg.call("/v1/gql", {"query": "CREATE BUNDLE ml (id TEXT BASE, u TEXT FIBER, "
                                     "it TEXT FIBER, r NUMERIC FIBER);"})
        recs = [{"id": f"x{k}", "u": f"u{u}", "it": f"i{i}", "r": rt}
                for k, (u, i, rt) in enumerate(data)]
        for c in range(0, len(recs), 20000):
            gg.call("/v1/bundles/ml/insert", {"records": recs[c:c + 20000]})
        log("loaded bundle ml (MovieLens 100k)")

        def refmf(seed, k=10, ep=20, lr=0.02, reg=0.05):
            rr = np.random.RandomState(seed)
            d = data[:]; rr.shuffle(d); cut = int(0.8 * len(d)); tr, te = d[:cut], d[cut:]
            nu = max(u for u, _, _ in data) + 1; ni = max(i for _, i, _ in data) + 1
            mu = np.mean([r for _, _, r in tr])
            P = rr.normal(0, 0.1, (nu, k)); Q = rr.normal(0, 0.1, (ni, k))
            bu = np.zeros(nu); bi = np.zeros(ni)
            for _ in range(ep):
                rr.shuffle(tr)
                for u, i, r in tr:
                    e = r - (mu + bu[u] + bi[i] + P[u] @ Q[i])
                    bu[u] += lr * (e - reg * bu[u]); bi[i] += lr * (e - reg * bi[i])
                    Pu = P[u].copy()
                    P[u] += lr * (e * Q[i] - reg * P[u]); Q[i] += lr * (e * Pu - reg * Q[i])
            return math.sqrt(np.mean([(r - (mu + bu[u] + bi[i] + P[u] @ Q[i])) ** 2
                                      for u, i, r in te]))

        for seed in seeds:
            g = gg.call("/v1/bundles/ml/factorize",
                        {"user": "u", "item": "it", "rating": "r",
                         "rank": 10, "epochs": 20}, timeout=600)["rmse"]
            r = refmf(seed)
            mf_cell["arms"]["gigi"]["values"].append(round(float(g), 6))
            mf_cell["arms"]["ref"]["values"].append(round(float(r), 6))
            log(f"seed {seed} recommend/mf: gigi={g:.4f} ref={r:.4f}")
        cells.append(finalize(mf_cell))
    except Exception as e:
        mf_cell["skipped"] = f"{type(e).__name__}: {e}"
        cells.append(mf_cell)
        log(f"movielens SKIPPED: {e}")

    # ---- EMBARGOED (a): digits eigenspace representation -----------------
    # SAME GMM, raw pixels vs spectral eigenspace: both arms run the ONE
    # shared EM implementation (src/ml/cluster.rs::gmm_em — the spectral
    # head=gmm calls it too since 2026-08-01) with identical settings
    # (covariance=diagonal, k-means++ init, reg 1e-6, converged EM). Primary
    # arm = eigenspace (/cluster method=spectral head=gmm embed_dim=20
    # covariance=diagonal); ref arm = raw pixels (/cluster method=gmm
    # covariance=diagonal) — the controlled representation experiment behind
    # the catalog-attested eigenspace-recovery claim. sklearn GMM on raw
    # pixels rides along as a seeded external reference arm.
    eig_cell = make_cell(
        "digits", "cluster-representation", "gmm: eigenspace vs raw", "ARI", "higher",
        "GIGI GMM on raw pixels (same algorithm)",
        {"k": 10, "eigenspace": {"method": "spectral", "head": "gmm", "embed_dim": 20,
                                 "neighbors": 15, "normalized": True,
                                 "covariance": "diagonal"},
         "raw": {"method": "gmm", "covariance": "diagonal"},
         "note": "arms.gigi = GMM in the spectral eigenspace; arms.ref = the SAME GMM "
                 "(shared gmm_em code path, identical covariance/init/convergence) on raw "
                 "pixels. Only the representation differs. Verdict measures the "
                 "representation effect, not gigi-vs-sklearn."},
        embargoed=True)
    add_extra_arm(eig_cell, "sklearn_gmm_raw", "sklearn GMM raw pixels (random_state=seed)")
    y = dig.target
    Xs = StandardScaler().fit_transform(dig.data)
    for seed in seeds:
        ge = gari("digits", gg.call("/v1/bundles/digits_cl/cluster",
                                    {"method": "spectral", "k": 10, "neighbors": 15,
                                     "normalized": True, "head": "gmm", "embed_dim": 20,
                                     "covariance": "diagonal"}), y)
        gr = gari("digits", gg.call("/v1/bundles/digits_cl/cluster",
                                    {"method": "gmm", "k": 10, "covariance": "diagonal"}), y)
        sr = adjusted_rand_score(y, GaussianMixture(10, random_state=seed).fit_predict(Xs))
        eig_cell["arms"]["gigi"]["values"].append(round(float(ge), 6))
        eig_cell["arms"]["ref"]["values"].append(round(float(gr), 6))
        eig_cell["extra_arms"]["sklearn_gmm_raw"]["values"].append(round(float(sr), 6))
        log(f"seed {seed} EMBARGO-a eigenspace={ge:.4f} raw={gr:.4f} sklearn_raw={sr:.4f}")
    cells.append(finalize(eig_cell))

    # ---- EMBARGOED (b): digits label-propagation vs flat kNN --------------
    # /infer method=diffusion (harmonic label-propagation on the kNN-graph
    # Laplacian) vs the flat sklearn kNN baseline — the 0.973 vs 0.944 claim.
    dif_cell = make_cell(
        "digits", "classify", "diffusion (label-prop) vs flat knn", "accuracy", "higher",
        "sklearn KNN (flat)", {"k": 7, "gigi_method": "diffusion"}, embargoed=True)
    for seed in seeds:
        g = gg.call("/v1/bundles/digits_c/infer",
                    {"target": "label", "method": "diffusion", "k": 7})["metric"]["accuracy"]
        r = float(cross_val_score(KNeighborsClassifier(7), Xs, dig.target, cv=5).mean())
        dif_cell["arms"]["gigi"]["values"].append(round(float(g), 6))
        dif_cell["arms"]["ref"]["values"].append(round(r, 6))
        log(f"seed {seed} EMBARGO-b diffusion={g:.4f} flat_knn={r:.4f}")
    cells.append(finalize(dif_cell))

    return cells


# ── Entry point ──────────────────────────────────────────────────────────────

def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--seeds", default=",".join(str(s) for s in DEFAULT_SEEDS),
                    help="comma-separated seed list (default 20260801..20260805)")
    ap.add_argument("--port", type=int, default=3143)
    ap.add_argument("--smoke", action="store_true",
                    help="harness validation: iris kmeans cell only, first 2 seeds, "
                         "separate output file")
    ap.add_argument("--out", default=None,
                    help="output path (default scripts/sweep_results_multiseed.json, "
                         "or *_smoke.json with --smoke)")
    args = ap.parse_args()

    seeds = [int(s) for s in args.seeds.split(",") if s.strip()]
    if args.smoke:
        seeds = seeds[:2]
    elif len(seeds) < 5:
        ap.error("full run requires >= 5 seeds (evidence rule 1)")
    out = args.out or os.path.join(
        REPO, "scripts",
        "sweep_results_multiseed_smoke.json" if args.smoke else "sweep_results_multiseed.json")

    try:
        commit = subprocess.run(["git", "rev-parse", "HEAD"], cwd=REPO,
                                capture_output=True, text=True).stdout.strip()
    except Exception:
        commit = "unknown"
    import sklearn
    server = GigiServer(args.port)
    server.start()
    started = time.strftime("%Y-%m-%dT%H:%M:%S")
    cells, run_error = [], None
    try:
        cells = run(Gigi(server.base), seeds, args.smoke)
        server.verify_alive()
    except Exception as e:
        run_error = f"{type(e).__name__}: {e}"
        raise
    finally:
        server.stop()
        meta = {
            "artifact": "multi-seed GIGI-vs-sklearn sweep (evidence rules 07/08)",
            "mode": "smoke" if args.smoke else "full",
            "generated": started,
            "finished": time.strftime("%Y-%m-%dT%H:%M:%S"),
            "seeds": seeds,
            "git_commit": commit,
            "server": {
                "binary": server.binary,
                "binary_mtime": time.strftime(
                    "%Y-%m-%dT%H:%M:%S", time.localtime(os.path.getmtime(server.binary))),
                "port": args.port,
                "data_dir": server.data_dir,
                "log": server.log_path,
                "exit_code": server.exit_code,
                "lifecycle": "started fresh by this script on a scratch data dir; "
                             "health-polled; verified alive post-run; terminated; "
                             "scratch dir removed",
            },
            "versions": {"python": sys.version.split()[0],
                         "sklearn": sklearn.__version__, "numpy": np.__version__},
            "std": "sample std, ddof=1, across seeds; 0.0 reported as such for "
                   "deterministic cells",
            "verdict_rule": "gigi-perspective signed delta vs threshold = "
                            "max(sqrt(std_gigi^2+std_ref^2), 0.005); "
                            "WIN if > threshold, LOSS if < -threshold, else PARITY; "
                            "0.005 floor = the single-run artifact's tie threshold, "
                            "needed because deterministic cells have combined_std 0",
            "determinism_notes": [
                "gigi /cluster and /infer expose no seed parameter (fixed LCG constants "
                "in src/ml/cluster.rs and src/ml/infer.rs); the gigi arm is re-called "
                "once per seed and its returned value recorded, so std 0 is measured "
                "not assumed",
                "record order is stable across restarts: Bundle::records() yields "
                "base-point-sorted records (src/bundle.rs). Before 2026-08-01 it "
                "iterated a per-process-random HashMap, so fixed-LCG results "
                "reproduced only within one server process — caught by artifact "
                "verification, fixed same day; cross-process reproducibility is "
                "re-verified by scripts/multiseed_repro_probe.py",
                "embargoed eigenspace cell: BOTH arms run the shared gmm_em "
                "(src/ml/cluster.rs) with identical covariance=diagonal settings — "
                "the spectral head=gmm calls the same EM as method=gmm since "
                "2026-08-01 (previously a separate compact GMM, a flagged confound)",
                "cross_val_score(cv=5) uses unshuffled folds — deterministic by protocol, "
                "identical to the single-run artifact scripts/sweep_results.json",
                "seed enters via sklearn random_state (KMeans/GMM/Spectral/IsolationForest/"
                "GP/train_test_split), the numpy Funk-SVD reference, and the bcancer-rare "
                "malignant subsample (which varies BOTH arms)",
                "strictly serial: one process, one request in flight at a time",
            ],
            "run_error": run_error,
        }
        with open(out, "w", encoding="utf-8") as f:
            json.dump({"meta": meta, "cells": cells}, f, indent=1)
        print(f"\nwrote {out} ({len(cells)} cells x {len(seeds)} seeds)", flush=True)

    print(f"\n=== {len(cells)} cells ===")
    for c in cells:
        g, r = c["arms"]["gigi"], c["arms"]["ref"]
        tag = " [EMBARGOED]" if c["embargoed"] else ""
        if c.get("skipped"):
            print(f"  {c['dataset']:16} {c['task']:24} {c['method']:34} SKIPPED: {c['skipped']}")
            continue
        print(f"  {c['dataset']:16} {c['task']:24} {c['method']:34} "
              f"gigi={g['mean']:.3f}+/-{g['std']:.3f} ref={r['mean']:.3f}+/-{r['std']:.3f} "
              f"[{c['verdict']}]{tag}")


if __name__ == "__main__":
    main()
