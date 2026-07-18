#!/usr/bin/env python3
"""scan_fit — learn supervised lens weights for GIGI SCAN from confirmed frauds.

    python3 scan_fit.py <bundle> --label-field is_fraud        # label lives in the bundle
    python3 scan_fit.py <bundle> --labels-json labels.json     # {id:{"fraud":0/1}} (test/offline)
    python3 scan_fit.py <bundle> --labels fraud_ids.txt        # one confirmed-fraud base-id per line

Pipeline: POST /scan (per-record lens features are in the response) → join labels →
k-fold logistic regression (pure stdlib, class-weighted) → report HELD-OUT PR-AUC →
train final weights on all data → print a ready-to-use `/scan` call.

The endpoint already applies weights: POST /scan {"weights": {...}} uses a weighted
linear fusion instead of max. So: fit once on confirmed frauds, deploy the weights.
No numpy, no sklearn, no network beyond the GIGI server.
"""
import sys, json, math, argparse, urllib.request, random
rng = random.Random(0)

ap = argparse.ArgumentParser()
ap.add_argument("bundle")
ap.add_argument("--server", default="http://localhost:3142")
ap.add_argument("--label-field", help="name of a 0/1 or bool fraud field in the bundle")
ap.add_argument("--labels-json", help="path to {id:{'fraud':0/1}} json")
ap.add_argument("--labels", help="path to file with one confirmed-fraud base-id per line")
ap.add_argument("--folds", type=int, default=5)
ap.add_argument("--epochs", type=int, default=400)
ap.add_argument("--out", help="write weights json here")
a = ap.parse_args()

def post(p, b):
    return json.loads(urllib.request.urlopen(urllib.request.Request(
        a.server + p, data=json.dumps(b).encode(), headers={"Content-Type": "application/json"})).read().decode())

# 1) run SCAN — limit 0 returns every record with its per-lens normalized features
scan = post(f"/v1/bundles/{a.bundle}/scan", {"budget": 0.05, "limit": 0})
lens_names = scan["lenses"]
base_key = next(k for k in scan["results"][0] if k not in ("score","top_lens","flagged","lenses"))
feats = {r[base_key]: [r["lenses"].get(ln, 0.0) for ln in lens_names] for r in scan["results"]}
ids = list(feats)
print(f"● SCAN on '{a.bundle}': {len(ids)} records, {len(lens_names)} lenses: {lens_names}")

# 2) labels
y = {}
if a.labels_json:
    lab = json.load(open(a.labels_json)); y = {i: int(lab[i]["fraud"]) for i in ids if i in lab}
elif a.labels:
    fr = set(l.strip() for l in open(a.labels) if l.strip()); y = {i: (1 if i in fr else 0) for i in ids}
elif a.label_field:
    q = post(f"/v1/bundles/{a.bundle}/query", {"conditions": [], "limit": 10_000_000})["data"]
    def truthy(v): return 1 if v in (1, 1.0, True, "1", "true", "True") else 0
    y = {r[base_key]: truthy(r.get(a.label_field)) for r in q if r.get(base_key) in feats}
else:
    sys.exit("provide --label-field, --labels-json, or --labels")
ids = [i for i in ids if i in y]
P = sum(y[i] for i in ids)
if P == 0 or P == len(ids): sys.exit(f"need both fraud and non-fraud labels (got {P} fraud / {len(ids)})")
print(f"● labels: {P} fraud / {len(ids)} total ({100*P/len(ids):.1f}%)")

D = len(lens_names)
def sigmoid(z): return 1/(1+math.exp(-max(-30, min(30, z))))
def train(tr):
    w = [0.0]*D; b = 0.0; lr = 0.5
    pos = sum(y[i] for i in tr); wpos = (len(tr)-pos)/max(1, pos)  # class weight
    for _ in range(a.epochs):
        for i in tr:
            x = feats[i]; p = sigmoid(b + sum(w[k]*x[k] for k in range(D)))
            g = (p - y[i]) * (wpos if y[i] else 1.0)
            b -= lr*g/len(tr)
            for k in range(D): w[k] -= lr*g*x[k]/len(tr)
    return w, b
def ap_score(order):
    tp = fp = 0; s = 0.0; pr = 0.0; tot = sum(y[i] for i in order)
    for i in order:
        if y[i]: tp += 1
        else: fp += 1
        r = tp/tot; p = tp/(tp+fp)
        if r > pr: s += (r-pr)*p; pr = r
    return s

def pr_auc_maxfusion():                       # unsupervised baseline for comparison
    sc = {i: max(feats[i]) for i in ids}
    return ap_score(sorted(ids, key=lambda i: -sc[i]))

# 3) k-fold CV — HONEST held-out estimate
shuf = ids[:]; rng.shuffle(shuf); folds = [shuf[f::a.folds] for f in range(a.folds)]
held = {}
for f in range(a.folds):
    te = set(folds[f]); tr = [i for i in ids if i not in te]
    w, b = train(tr)
    for i in te: held[i] = b + sum(w[k]*feats[i][k] for k in range(D))
cv_auc = ap_score(sorted(ids, key=lambda i: -held[i]))

# 4) final model on all data
w, b = train(ids)
weights = {ln: round(w[k], 4) for k, ln in enumerate(lens_names)}

print(f"\n● PR-AUC:  unsupervised max-fusion {pr_auc_maxfusion():.3f}  →  supervised (held-out {a.folds}-fold) {cv_auc:.3f}")
print("● learned weights (bias %.3f):" % b)
for ln in sorted(weights, key=lambda l: -weights[l]):
    print(f"    {ln:<32} {weights[ln]:+.4f}")
payload = {"weights": weights}
if a.out: json.dump(payload, open(a.out, "w"), indent=2); print(f"\n● wrote {a.out}")
print("\n● deploy — score new data with these weights:")
print(f"  curl -X POST {a.server}/v1/bundles/{a.bundle}/scan \\\n       -d '{json.dumps({'budget':0.05,**payload})}'")
