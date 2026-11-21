"""Q3 step 2 (docs/HALCYON_Q3_INGEST_SPEC.md): the Q3 ensembles as GIGI bundles, every Q3 number read back through
GQL and REST on the stored data and compared with the frozen Q3-C probe on the files.

Gates: I-load, I-store, I-parity, I-persist, I-null (see the spec). With an Amendment-4 engine and probe (the
probe's default rule is then "a4") I-parity also compares the GEVP path. Idempotent: a bundle that already holds the right
record count is not reloaded. Own port (never 3142 / 3147), own key, own data directory.

usage: python test_q3_ingest.py GIGI_STREAM_EXE PROBE_EXE RUN_DIR DATA_DIR RECEIPT.json
"""
import hashlib
import json
import os
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

import numpy as np

EXE, PROBE, RUN, DATA, RECEIPT = Path(sys.argv[1]), sys.argv[2], Path(sys.argv[3]), Path(sys.argv[4]), Path(sys.argv[5])
KEY = "q3-ingest-test-key"
BLOCK = 100
POINTS = [(0, 2.2986, 10), (1, 2.2986, 14), (2, 2.3715, 12), (3, 2.3715, 16), (4, 2.4265, 16), (5, 2.4265, 20),
          (6, 2.3, 16), (7, 2.2986, 12), (8, 2.2986, 14), (9, 2.5115, 20), (10, 2.5115, 24)]
CH = {"scalar": ["s0", "s1", "s2", "s3"], "torelon": ["p0", "p1", "p2", "p3"]}
TOL = 1e-12


def free_port():
    s = socket.socket(); s.bind(("127.0.0.1", 0)); p = s.getsockname()[1]; s.close()
    assert p not in (3142, 3147)
    return p


PORT = free_port()
BASE = f"http://127.0.0.1:{PORT}"


def call(method, path, body=None, timeout=1800):
    req = urllib.request.Request(BASE + path, method=method,
                                 data=None if body is None else json.dumps(body).encode(),
                                 headers={"Content-Type": "application/json", "X-API-Key": KEY})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.status, json.loads(r.read() or b"{}")
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read() or b"{}")


def probe(payload):
    with tempfile.TemporaryDirectory() as d:
        i, o = Path(d) / "i.json", Path(d) / "o.json"
        i.write_text(json.dumps(payload))
        subprocess.run([PROBE, str(i), str(o)], check=True)
        return json.loads(o.read_text())


def start():
    env = dict(os.environ, PORT=str(PORT), GIGI_DATA_DIR=str(DATA), GIGI_API_KEY=KEY)
    srv = subprocess.Popen([str(EXE)], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT)
    for _ in range(600):
        try:
            if call("GET", "/v1/health", timeout=2)[0] in (200, 404):
                return srv
        except Exception:
            pass
        time.sleep(1)
    srv.terminate()
    raise SystemExit("engine did not come up")


def stop(srv):
    srv.terminate()
    srv.wait(timeout=120)


def count(name):
    st, r = call("POST", f"/v1/bundles/{name}/count", {"conditions": []})
    return r.get("count") if st == 200 else None


def load(name, cols, n_cfg, lt):
    """cols: {field: array [n_cfg][lt]}; record id = cfg * lt + t."""
    if count(name) == n_cfg * lt:
        return "present"
    if count(name) is not None:
        call("DELETE", f"/v1/bundles/{name}")
    sch = {"id": "integer", "cfg": "integer", "t": "integer"}
    sch.update({f: "float" for f in cols})
    st, r = call("POST", "/v1/bundles", {"name": name, "schema": {"fields": sch, "keys": ["id"]}})
    if st not in (200, 201):
        raise SystemExit(f"create {name}: {st} {r}")
    batch = []
    for c in range(n_cfg):
        for t in range(lt):
            batch.append({"id": c * lt + t, "cfg": c, "t": t, **{f: float(a[c][t]) for f, a in cols.items()}})
            if len(batch) == 5000:
                st, r = call("POST", f"/v1/bundles/{name}/insert", {"records": batch})
                if st not in (200, 201):
                    raise SystemExit(f"insert {name}: {st} {r}")
                batch = []
    if batch:
        st, r = call("POST", f"/v1/bundles/{name}/insert", {"records": batch})
        if st not in (200, 201):
            raise SystemExit(f"insert {name}: {st} {r}")
    return "loaded"


def rel(a, b):
    return abs(a - b) / max(abs(b), 1e-300)


def em_gql(name, fields):
    return call("POST", "/v1/gql", {"query": f"EFFECTIVE_MASS {name} ON {', '.join(fields)} BLOCK {BLOCK} GEVP 1, 2;"})


def em_rest(name, fields):
    return call("POST", f"/v1/bundles/{name}/effective_mass", {"fields": fields, "block": BLOCK, "gevp": [1, 2]})


def compare_em(row_status, row, pr):
    """Engine row (status, body) against the probe output pr. Returns (ok, detail)."""
    g = pr.get("gevp") or {}
    if "refused" in g:
        want = g["refused"]
    elif "refused" in (pr.get("plateau") or {}):
        want = pr["plateau"]["refused"]
    else:
        want = None
    if want is not None:
        err = json.dumps(row)
        return (row_status != 200 and want in err), {"probe_refused": want, "engine": [row_status, row]}
    if row_status != 200:
        return False, {"engine_refused": row}
    r = row["rows"][0]; p = pr["plateau"]; af = pr.get("plateau_as_fixed") or {}
    checks = {"mass": rel(r["mass"], p["mass"]), "error": rel(r["error"], p["error"]),
              "window": float((int(r["t_lo"]), int(r["t_hi"])) != (p["t_lo"], p["t_hi"])),
              "kept": float([int(x) for x in r["gevp_kept"]] != g["kept"]),
              "directions": float(int(r["gevp_directions_kept"]) != (g.get("directions_kept") or 0))}
    if "path" in g:  # Amendment 4 probe: the engine must report the same GEVP path
        checks["path"] = float(r.get("gevp_path") != g["path"])
    if "mass" in af:
        checks["as_fixed_mass"] = rel(r["as_fixed_mass"], af["mass"])
        checks["as_fixed_error"] = rel(r["as_fixed_error"], af["error"])
    else:
        checks["as_fixed_refused"] = float("as_fixed_refused" not in r)
    return max(checks.values()) <= TOL, checks


def main():
    if RECEIPT.exists():
        raise SystemExit(f"refusing to overwrite {RECEIPT}")
    DATA.mkdir(parents=True, exist_ok=True)
    rec = {"kind": "Q3 step 2: ensembles inside GIGI", "spec": "docs/HALCYON_Q3_INGEST_SPEC.md", "port": PORT,
           "data_dir": str(DATA), "engine_sha256": hashlib.sha256(EXE.read_bytes()).hexdigest(),
           "probe_sha256": hashlib.sha256(Path(PROBE).read_bytes()).hexdigest(), "points": {}}
    gates = {"I-load": True, "I-store": True, "I-parity": True}
    srv = start()
    try:
        for k, beta, L in POINTS:
            f = RUN / f"k{k}_b{beta}_L{L}.ensemble.json"
            if not f.exists():
                rec["points"][f"k{k}"] = "ensemble not present yet"
                continue
            e = json.loads(f.read_text())
            n, lt = len(e["scalar"][0]), len(e["scalar"][0][0])
            plaq = np.repeat(np.array(e["mean_plaquette"])[:, None], lt, axis=1)
            cols = {**{CH["scalar"][a]: e["scalar"][a] for a in range(4)},
                    **{CH["torelon"][a]: e["torelon"][a] for a in range(4)}, "plaq": plaq.tolist()}
            name = f"q3_k{k}"
            t0 = time.time(); how = load(name, cols, n, lt); load_s = time.time() - t0
            cnt = count(name)
            pt = {"bundle": name, "records": cnt, "expected": n * lt, "ingest": how, "seconds": round(load_s, 1)}
            gates["I-load"] &= cnt == n * lt
            # I-store: every field's correlator on the stored data equals the probe's on the file
            worst = 0.0
            for ch in CH:
                for a, fld in enumerate(CH[ch]):
                    pr = probe({"p": e[ch][a], "block": BLOCK})
                    st, g = call("POST", "/v1/gql", {"query": f"CORRELATOR {name} ON {fld} BLOCK {BLOCK};"})
                    rows = sorted(g.get("rows", []), key=lambda r: r["t"]) if st == 200 else []
                    if len(rows) != lt:
                        worst = float("inf"); continue
                    for r in rows:
                        t = int(r["t"])
                        worst = max(worst, rel(r["C"], pr["C_t"][t]))
                        if pr["C_err_t"][t] is not None and pr["C_err_t"][t] > 0:
                            worst = max(worst, rel(r["C_err"], pr["C_err_t"][t]))
            pt["store_worst_rel"] = worst
            gates["I-store"] &= worst <= TOL
            # I-parity: EFFECTIVE_MASS GEVP through GQL and REST against the probe
            for ch in CH:
                pr = probe({"ops": e[ch], "t0": 1, "t1": 2, "block": BLOCK})
                ok_g, d_g = compare_em(*em_gql(name, CH[ch]), pr)
                ok_r, d_r = compare_em(*em_rest(name, CH[ch]), pr)
                pt[ch] = {"gql_ok": ok_g, "rest_ok": ok_r, "gql": d_g, "rest": d_r,
                          "probe_plateau": pr.get("plateau"), "probe_gevp_refused": (pr.get("gevp") or {}).get("refused")}
                gates["I-parity"] &= ok_g and ok_r
            rec["points"][f"k{k}"] = pt
            print(json.dumps({k: pt[k] for k in ("bundle", "records", "ingest", "seconds", "store_worst_rel")}
                             | {ch: [pt[ch]["gql_ok"], pt[ch]["rest_ok"]] for ch in CH}), flush=True)
        # I-null on k0: the shuffled projected 0++ series refuses
        e0 = json.loads((RUN / "k0_b2.2986_L10.ensemble.json").read_text())
        pr0 = probe({"ops": e0["scalar"], "t0": 1, "t1": 2, "block": BLOCK})
        v = np.array(pr0["gevp"]["vector"]); s = np.einsum("a,act->ct", v, np.array(e0["scalar"]))
        rng = np.random.default_rng(e0["seed"] + 7); flat = s.flatten(); rng.shuffle(flat); s = flat.reshape(s.shape)
        # the null bundle is keyed to the projection vector, so a bundle stored from another rule's vector is
        # never reused (the idempotent loader checks only the record count)
        nb = "q3_k0_null_" + hashlib.sha256(np.array(v).tobytes()).hexdigest()[:12]
        load(nb, {"n": s.tolist()}, s.shape[0], s.shape[1])
        st_n, r_n = call("POST", "/v1/gql", {"query": f"EFFECTIVE_MASS {nb} ON n BLOCK {BLOCK};"})
        pn = probe({"p": s.tolist(), "block": BLOCK})["plateau"]
        gates["I-null"] = st_n != 200 and "refused" in pn and pn["refused"] in json.dumps(r_n)
        rec["null_k0"] = {"bundle": nb, "engine": [st_n, r_n], "probe": pn}
        before = em_gql("q3_k0", CH["scalar"])
    finally:
        stop(srv)
    # I-persist: restart on the same data directory (the process was killed, not shut down cleanly)
    srv = start()
    try:
        after = em_gql("q3_k0", CH["scalar"])
    finally:
        stop(srv)
    gates["I-persist"] = before[0] == 200 and after[0] == 200 and before[1] == after[1]
    rec["persist_k0"] = {"before": before, "after": after}
    rec["gates"] = gates
    rec["all_pass"] = all(gates.values())
    RECEIPT.parent.mkdir(parents=True, exist_ok=True)
    RECEIPT.write_text(json.dumps(rec, indent=1, default=float) + "\n")
    print(json.dumps({"gates": gates}, indent=1))


if __name__ == "__main__":
    main()
