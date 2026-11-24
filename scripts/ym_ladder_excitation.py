"""The lowest excitation on SU(2) ladders: the analysis registered in
research/ym-ladder-20261123/prereg_lowest_excitation.md (original registration and Amendment 1).

Measurements (all gauge invariant, independent of intertwiner phase conventions):
  * levels: eigenvalues grouped by relative 1e-8; a level is used through its spectral projector P, never through one
    eigenvector;
  * winding sector (rings): z(s) = (-1)^(2 j_bottom0 + 2 j_top0), preserved by every plaquette operator;
  * transition kernel C_kl = <O_k Omega, P O_l Omega>, O_k = tr U_k; intensity I_k = C_kk; envelope sqrt(I_k);
    edge weight W = (I_1 + I_p) / sum I_k;
  * parity, basis independently: the strength of P in the reflection-even probe channel (O_k + O_{p+1-k}) and in the
    odd channel (O_k - O_{p+1-k}); for a rank-one level one of them vanishes;
  * node-free (rank one): every C_1k of one sign;
  * energy profile e_k = Tr(P h_k)/r - <Omega, h_k Omega>, with h_k = (2/g^2)(2 - tr U_k) + (g^2/2) sum_links w_l j_l(j_l+1),
    a shared rung split equally; sum_k h_k = H, so sum_k e_k is the level's mean excitation energy;
  * absolute gap versus probe-accessible gap (lowest level of the vacuum's sector with nonzero transition strength).

Classification (Scoring section, as amended): Band, Edge, Band with distinct ends, None of these; a level the probes do
not see is reported as such and never classified.
"""
import numpy as np
import scipy.sparse as sp
from scipy.linalg import eigh
from scipy.sparse.linalg import eigsh

import ks_su2_lattice as ks

UNSEEN = "probe does not see this level"


# ── generic analysis on (H, probes, Omega) ───────────────────────────────────────────────────────────────────────────

def levels(w, V, rel=1e-8):
    out, i = [], 0
    while i < len(w):
        j = i + 1
        while j < len(w) and abs(w[j] - w[i]) <= rel * max(1.0, abs(w[i])):
            j += 1
        out.append((float(np.mean(w[i:j])), float(w[j - 1] - w[i]), V[:, i:j]))
        i = j
    return out


def kernel(Q, probes_on_vacuum):
    """C_kl = <O_k Omega, P O_l Omega> with P = Q Q^T (Q: orthonormal basis of the level)."""
    A = Q.T @ probes_on_vacuum                 # r x p
    return A.T @ A


def channels(Q, probes_on_vacuum):
    p = probes_on_vacuum.shape[1]
    ev = [probes_on_vacuum[:, k] + probes_on_vacuum[:, p - 1 - k] for k in range((p + 1) // 2)]
    od = [probes_on_vacuum[:, k] - probes_on_vacuum[:, p - 1 - k] for k in range(p // 2)]
    se = sum(float(np.sum((Q.T @ v) ** 2)) for v in ev) if ev else 0.0
    so = sum(float(np.sum((Q.T @ v) ** 2)) for v in od) if od else 0.0
    return se, so


def profile(Q, probes_on_vacuum, tol=1e-12):
    C = kernel(Q, probes_on_vacuum)
    I = np.diag(C).copy()
    total = float(I.sum())
    scale = float(np.sum(probes_on_vacuum ** 2))
    if total <= tol * max(scale, 1.0):
        return {"seen": False, "verdict": UNSEEN}
    p = len(I)
    env = np.sqrt(np.clip(I, 0, None))
    W = float((I[0] + I[-1]) / total) if p > 1 else 1.0
    se, so = channels(Q, probes_on_vacuum)
    parity = "even" if so <= 1e-10 * (se + so) else ("odd" if se <= 1e-10 * (se + so) else "mixed")
    out = {"seen": True, "kernel": C, "intensity": I, "envelope": env, "W": W, "even_strength": se,
           "odd_strength": so, "parity": parity, "rank": Q.shape[1]}
    if Q.shape[1] == 1:
        ref = int(np.argmax(np.abs(C[0])))  # reference plaquette with nonzero coupling
        row = C[ref]
        nz = np.abs(row) > 1e-10 * np.abs(row).max()
        out["node_free"] = bool(np.all(np.sign(row[nz]) == np.sign(row[nz][0])))
    else:
        out["node_free"] = None
    return out


def sine(p):
    k = np.arange(1, p + 1)
    return np.sin(np.pi * k / (p + 1))


def classify(prof, p, ring_comparisons):
    """ring_comparisons: list of (open_gap, ring_probe_accessible_gap) pairs, sector matched."""
    if not prof.get("seen"):
        return UNSEEN
    S = sine(p)
    env = prof["envelope"]
    C = float(env @ S / (np.linalg.norm(env) * np.linalg.norm(S)))
    WS = float((S[0] ** 2 + S[-1] ** 2) / np.sum(S ** 2))
    ring_below = bool(ring_comparisons) and all(r < o for o, r in ring_comparisons)
    ring_above = bool(ring_comparisons) and all(r > o for o, r in ring_comparisons)
    prof["cosine_to_sine"], prof["W_sine"] = C, WS
    if C >= 0.99 and abs(prof["W"] - WS) <= 0.05 and ring_below:
        return "Band"
    if prof["W"] >= WS + 0.15 and ring_above:
        return "Edge"
    if prof.get("node_free") and ring_below:
        return "Band with distinct ends"
    return "None of these"


# ── the lattice ──────────────────────────────────────────────────────────────────────────────────────────────────────

class Ladder:
    def __init__(self, p, jmax, periodic=False):
        self.p, self.periodic = p, periodic
        self.graph = ks.ladder(p, periodic)
        self.sn = ks.SpinNetwork(self.graph, jmax)
        self.sn.hamiltonian(1.0)
        n = len(self.sn.states)
        nv = p if periodic else p + 1
        bottom, top, rung = list(range(p)), list(range(p, 2 * p)), list(range(2 * p, 2 * p + nv))
        self.weights = []           # per plaquette: {link: weight} for the electric energy
        for k in range(p):
            w = {bottom[k]: 1.0, top[k]: 1.0}
            for r in (rung[k % nv], rung[(k + 1) % nv]):
                shared = periodic or (r not in (rung[0], rung[-1]))
                w[r] = w.get(r, 0.0) + (0.5 if shared else 1.0)
            self.weights.append(w)
        js = np.array(self.sn.states)
        self.cas = js * (js + 1)
        self.z = np.ones(n) if not periodic else (-1.0) ** np.rint(2 * (js[:, bottom[0]] + js[:, top[0]]))

    def hamiltonian(self, g2):
        n = len(self.sn.states)
        H = sp.diags(0.5 * g2 * self.cas.sum(axis=1)).astype(complex)
        for T in self.sn.plaquette_ops:
            H = H + (2 / g2) * (2 * sp.identity(n) - T)
        assert abs(H.imag).max() < 1e-12
        return H.real.tocsr()

    def h_local(self, g2, k):
        n = len(self.sn.states)
        el = sum(wt * self.cas[:, link] for link, wt in self.weights[k].items())
        return (sp.diags(0.5 * g2 * el) + (2 / g2) * (2 * sp.identity(n) - self.sn.plaquette_ops[k].real)).tocsr()

    def solve(self, g2, k=12):
        H = self.hamiltonian(g2)
        n = H.shape[0]
        if n <= 2500:
            w, V = eigh(H.toarray())
            w, V = w[:k], V[:, :k]
        else:
            w, V = eigsh(H, k=k, which="SA", tol=1e-12)
            o = np.argsort(w); w, V = w[o], V[:, o]
        return H, w, V

    def analyse(self, g2, k=12):
        H, w, V = self.solve(g2, k)
        L = levels(w, V)
        E0, _, Q0 = L[0]
        assert Q0.shape[1] == 1, "vacuum degenerate"
        Om = Q0[:, 0]
        probes = np.column_stack([self.sn.plaquette_ops[j].real @ Om for j in range(self.p)])
        vac_sector = float(np.sign(Om @ (self.z * Om)))       # +1 if the vacuum lives in z = +1
        out = {"E0": E0, "levels": []}
        for E, spread, Q in L[1:]:
            zq = float(np.sum(Q * (self.z[:, None] * Q)) / Q.shape[1])
            prof = profile(Q, probes)
            e = [float(np.trace(Q.T @ (self.h_local(g2, j) @ Q)) / Q.shape[1] - Om @ (self.h_local(g2, j) @ Om))
                 for j in range(self.p)]
            out["levels"].append({"E": E, "spread": spread, "rank": Q.shape[1], "winding": zq,
                                  "in_vacuum_sector": bool(abs(zq - vac_sector) < 1e-8), "profile": prof,
                                  "energy_profile": e, "energy_profile_sum": float(sum(e))})
        out["absolute_gap"] = out["levels"][0]["E"] - E0
        acc = [lv for lv in out["levels"] if lv["in_vacuum_sector"] and lv["profile"].get("seen")]
        out["probe_accessible_gap"] = acc[0]["E"] - E0 if acc else None
        out["lowest_accessible"] = acc[0] if acc else None
        return out


# ── planted systems for the gates ────────────────────────────────────────────────────────────────────────────────────

def planted_chain(p, eps=3.0, t=0.2, end_potential=0.0, periodic=False, hidden=None):
    """Vacuum (energy 0) plus a chain of p sites (on-site eps, hopping -t, optional end potential). Probe k creates
    site k from the vacuum. Optionally a hidden state of energy `hidden` that no probe reaches (another sector)."""
    n = 1 + p + (1 if hidden is not None else 0)
    H = np.zeros((n, n))
    for k in range(p):
        H[1 + k, 1 + k] = eps + (end_potential if k in (0, p - 1) else 0.0)
        if k + 1 < p:
            H[1 + k, 2 + k] = H[2 + k, 1 + k] = -t
    if periodic and p > 2:
        H[1, p] = H[p, 1] = -t
    if hidden is not None:
        H[-1, -1] = hidden
    w, V = np.linalg.eigh(H)
    Om = np.zeros(n); Om[0] = 1.0
    probes = np.zeros((n, p))
    for k in range(p):
        probes[1 + k, k] = 1.0
    L = levels(w, V)
    first = [lv for lv in L if lv[0] > 1e-12]
    absolute = first[0][0]
    seen = [lv for lv in first if profile(lv[2], probes).get("seen")]
    return {"levels": first, "probes": probes, "absolute_gap": absolute,
            "probe_accessible_gap": seen[0][0] if seen else None,
            "first_profile": profile(first[0][2], probes), "accessible_profile": profile(seen[0][2], probes)}


def gates():
    g = {}
    p = 5
    # G1 planted band: open and ring chains without end potential
    o, r = planted_chain(p), planted_chain(p, periodic=True)
    g["G1_planted_band"] = classify(o["accessible_profile"], p, [(o["probe_accessible_gap"], r["probe_accessible_gap"])])
    # G2 planted edge: an attractive end potential binds; the ring has no ends
    oe, re = planted_chain(p, end_potential=-0.6), planted_chain(p, periodic=True)
    g["G2_planted_edge"] = classify(oe["accessible_profile"], p, [(oe["probe_accessible_gap"], re["probe_accessible_gap"])])
    # G6 a lower hidden state in another sector: reported as unseen, absolute and accessible gaps separate
    h = planted_chain(p, periodic=True, hidden=1.0)
    g["G6_inaccessible_winding"] = {"first_level_verdict": classify(h["first_profile"], p, []),
                                    "absolute_gap": h["absolute_gap"], "probe_accessible_gap": h["probe_accessible_gap"]}
    # G7 zero overlap
    Qz = np.zeros((6, 1)); Qz[5, 0] = 1.0
    g["G7_zero_overlap"] = profile(Qz, np.eye(6)[:, :3])["verdict"] if not profile(Qz, np.eye(6)[:, :3])["seen"] else "seen"
    # G8 kernel formulas: three-site sine gives W = 1/2; (1,0,1) and (1,0,-1) resolved as even and odd
    probes3 = np.vstack([np.zeros((1, 3)), np.eye(3)])
    s3 = sine(3) / np.linalg.norm(sine(3))
    q = np.r_[0.0, s3][:, None]
    g["G8_sine_W"] = profile(q, probes3)["W"]
    ev = np.r_[0, 1, 0, 1.0][:, None] / np.sqrt(2); od = np.r_[0, 1, 0, -1.0][:, None] / np.sqrt(2)
    g["G8_parity"] = (profile(ev, probes3)["parity"], profile(od, probes3)["parity"],
                      bool(np.allclose(profile(ev, probes3)["intensity"], profile(od, probes3)["intensity"])))
    # G9 planted doublet: random rotation inside the level leaves kernel and parity strengths unchanged
    rng = np.random.default_rng(7)
    D = np.linalg.qr(rng.standard_normal((8, 2)))[0]
    probes8 = rng.standard_normal((8, 4))
    R = np.linalg.qr(rng.standard_normal((2, 2)))[0]
    P1, P2 = profile(D, probes8), profile(D @ R, probes8)
    single1 = (D[:, 0] @ probes8) ** 2
    single2 = ((D @ R)[:, 0] @ probes8) ** 2
    g["G9_doublet"] = {"kernel_invariant": bool(np.allclose(P1["kernel"], P2["kernel"])),
                       "channels_invariant": bool(np.allclose([P1["even_strength"], P1["odd_strength"]],
                                                              [P2["even_strength"], P2["odd_strength"]])),
                       "single_vector_changes": bool(not np.allclose(single1, single2))}
    caught = {
        "G1": g["G1_planted_band"] == "Band",
        "G2": g["G2_planted_edge"] == "Edge",
        "G6": g["G6_inaccessible_winding"]["first_level_verdict"] == UNSEEN
              and g["G6_inaccessible_winding"]["probe_accessible_gap"] > g["G6_inaccessible_winding"]["absolute_gap"],
        "G7": g["G7_zero_overlap"] == UNSEEN,
        "G8": abs(g["G8_sine_W"] - 0.5) < 1e-12 and g["G8_parity"] == ("even", "odd", True),
        "G9": all(g["G9_doublet"].values()),
    }
    return g, caught


# ── the registered run ───────────────────────────────────────────────────────────────────────────────────────────────

OPEN = {2: (2.0, 2.5), 3: (2.0, 2.5), 4: (2.0, 2.5), 5: (1.5, 2.0)}
RING = {3: (1.5, 2.0), 4: (1.5, 2.0), 5: (1.0, 1.5)}
SCORED = (3, 4, 5)


def summarise(a):
    lv = a["lowest_accessible"]
    prof = lv["profile"] if lv else {"seen": False}
    return {"E0": a["E0"], "absolute_gap": a["absolute_gap"], "probe_accessible_gap": a["probe_accessible_gap"],
            "first_level": {k: a["levels"][0][k] for k in ("E", "rank", "winding", "in_vacuum_sector")},
            "accessible_level": None if lv is None else {
                "rank": lv["rank"], "spread": lv["spread"], "winding": lv["winding"],
                "intensity": prof.get("intensity"), "envelope": prof.get("envelope"), "W": prof.get("W"),
                "parity": prof.get("parity"), "even_strength": prof.get("even_strength"),
                "odd_strength": prof.get("odd_strength"), "node_free": prof.get("node_free"),
                "kernel": prof.get("kernel"), "energy_profile": lv["energy_profile"],
                "energy_profile_sum": lv["energy_profile_sum"]},
            "bookkeeping_max_residual": max(abs(l["energy_profile_sum"] - (l["E"] - a["E0"])) for l in a["levels"])}


def converged(lo, hi):
    """G5 and G10 between two cutoffs."""
    out = {}
    for key in ("absolute_gap", "probe_accessible_gap"):
        a, b = lo[key], hi[key]
        out[key + "_rel_change"] = None if a is None or b is None else abs(b / a - 1)
    ea = lo["accessible_level"]["envelope"] if lo["accessible_level"] else None
    eb = hi["accessible_level"]["envelope"] if hi["accessible_level"] else None
    out["envelope_cosine"] = None if ea is None or eb is None else float(
        np.dot(ea, eb) / (np.linalg.norm(ea) * np.linalg.norm(eb)))
    out["ok"] = bool(out["envelope_cosine"] is not None and out["envelope_cosine"] >= 0.999
                     and all(v is not None and v <= 1e-3 for k, v in out.items() if k.endswith("rel_change")))
    return out


def main(out):
    import json
    import sys
    import time
    from pathlib import Path
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import _receipt_status as R

    def tidy(o):
        if isinstance(o, dict):
            return {k: tidy(v) for k, v in o.items()}
        if isinstance(o, (list, tuple)):
            return [tidy(v) for v in o]
        if isinstance(o, np.ndarray):
            return tidy(o.tolist())
        if isinstance(o, (np.floating, np.integer, np.bool_)):
            return o.item()
        return o

    res = {"prereg": "research/ym-ladder-20261123/prereg_lowest_excitation.md (original, Amendment 1, clarification)"}
    g, caught = gates()
    res["gates_planted"] = {"results": g, "caught": caught}
    if not all(caught.values()):
        res["status"] = "FAILED: a planted gate was not caught; no lattice measured"
        R.publish(out, tidy(res)); return
    meas = {"open": {}, "ring": {}}
    for kind, table in (("open", OPEN), ("ring", RING)):
        for p, cuts in table.items():
            for g2 in ((2.0, 1.0) if kind == "open" else (2.0,)):
                rows = {}
                for jm in cuts:
                    t0 = time.time()
                    lad = Ladder(p, jm, periodic=(kind == "ring"))
                    rows[str(jm)] = summarise(lad.analyse(g2))
                    rows[str(jm)]["states"] = len(lad.sn.states)
                    rows[str(jm)]["seconds"] = round(time.time() - t0, 1)
                    print(kind, p, g2, jm, rows[str(jm)]["states"], rows[str(jm)]["seconds"], "s", flush=True)
                rows["convergence"] = converged(rows[str(cuts[0])], rows[str(cuts[1])])
                meas[kind][f"p{p}_g{g2}"] = rows
    res["measurements"] = meas
    G4 = max(v["bookkeeping_max_residual"] for kind in meas for r in meas[kind].values()
             for k, v in r.items() if k != "convergence")
    res["G4_bookkeeping_max_residual"] = G4
    verdicts = {}
    for p in SCORED:
        o = meas["open"][f"p{p}_g2.0"]
        r = meas["ring"].get(f"p{p}_g2.0")
        hi_o = o[str(OPEN[p][1])]
        if not o["convergence"]["ok"]:
            verdicts[p] = "not converged (not scored)"; continue
        comps = []
        if r is not None and r["convergence"]["ok"]:
            hi_r = r[str(RING[p][1])]
            if hi_r["probe_accessible_gap"] is not None and hi_o["probe_accessible_gap"] is not None:
                comps.append((hi_o["probe_accessible_gap"], hi_r["probe_accessible_gap"]))
        lv = hi_o["accessible_level"]
        prof = {"seen": lv is not None, "envelope": np.array(lv["envelope"]) if lv else None,
                "W": lv["W"] if lv else None, "node_free": lv["node_free"] if lv else None}
        verdicts[p] = {"verdict": classify(prof, p, comps), "ring_comparisons": comps,
                       "cosine_to_sine": prof.get("cosine_to_sine"), "W": prof.get("W"), "W_sine": prof.get("W_sine")}
    res["verdicts_g2"] = verdicts
    labels = [v["verdict"] if isinstance(v, dict) else v for v in verdicts.values()]
    res["reading_holding_at_every_scored_p"] = labels[0] if len(set(labels)) == 1 else "none (differs across p)"
    res["ok"] = bool(G4 < 1e-10)
    R.publish(out, tidy(res))
    print(json.dumps(tidy(verdicts), indent=1))


if __name__ == "__main__":
    import sys
    sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))
    import _receipt_status as R
    R.run(sys.argv[1], lambda: main(sys.argv[1]))
    R.exit_on_failure(sys.argv[1])
