"""Multi-plaquette tests for 'Yang-Mills Requirements as Geometric Objects' (Sections 5, 7, 9, 10), on Kogut-Susskind
SU(2) ladders of p plaquettes, built with scripts/ks_su2_lattice.py.

Validation first (nothing physical is reported unless it passes):
  V1  link operators against sampled Wigner D-functions: the product rule for U_ab, the inverse link, the gauge
      generators under U -> g U h^{-1};
  V2  one plaquette in the spin-network basis equals the exact character-basis Hamiltonian;
  V3  two-plaquette ladder (spins <= 1/2) and theta graph (spins <= 1, 3/2): spin network = brute force (product space
      with Gauss's law as a penalty), state for state;
  V4  the brute-force ground state is gauge invariant, checked on sampled configurations and gauge transformations;
  V5  overlapping plaquette operators commute away from the truncation edge; their spectra lie in [-2, 2].

Physics (lattice units; nothing here is a continuum statement):
  P1  regulator removal: the gap converges as the link-spin cutoff jmax grows, at each size;
  P2  uniformity in volume (Section 9): the gap and the fixed-energy sublevel radius across ladders p = 1..5;
      the ground energy per plaquette;
  P3  the gap three ways (Section 7) on multi-plaquette lattices: spectrum, sublevel radius, Euclidean semigroup;
  P4  electric versus full gap (Section 10) in volume;
  P5  reflection positivity on the lattice (Section 5): the transfer matrix built from Wilson time-links has positive
      character coefficients (Osterwalder-Seiler), and link-reflected correlations are a Gram matrix. Gate: a mixed
      fundamental-adjoint action with a negative adjoint coupling has a negative coefficient, and link reflection
      finds the negative direction.

usage: python scripts/ym_geometry_lattice_tests.py OUT.json
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
import scipy.sparse as sp
from scipy.integrate import quad
from scipy.linalg import eigh, eigh_tridiagonal, expm, logm
from scipy.sparse.linalg import eigsh
from scipy.special import iv

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import _receipt_status  # noqa: E402
import ks_su2_lattice as ks  # noqa: E402

RNG = np.random.default_rng(20261123)
EPS = np.finfo(float).eps


def positivity(ev_min, bound):
    """'numerically_consistent' (not below the rounding allowance; not a certificate), 'negative_witness' (below 100
    allowances), or 'unresolved'. 'certified' is reserved for an exact argument."""
    if ev_min >= -bound:
        return "numerically_consistent"
    if ev_min < -100 * bound:
        return "negative_witness"
    return "unresolved"
SIG = [np.array([[0, 1], [1, 0]]), np.array([[0, -1j], [1j, 0]]), np.diag([1, -1])]
THETA = {"nv": 2, "links": [(0, 1)] * 3, "name": "theta",
         "plaqs": [([(0, True), (1, False)], [0, 1]), ([(1, True), (2, False)], [0, 1])]}


def theta_of(W):
    A = logm(W) / 1j
    return np.array([np.real(np.trace(A @ s)) for s in SIG])


# ── validation ───────────────────────────────────────────────────────────────────────────────────────────────────────

def v1():
    L = ks.LinkSpace(1.5)
    low = np.array([j < 1.5 for j, _, _ in L.basis])
    e_prod = e_inv = e_gauge = 0.0
    for _ in range(20):
        th = RNG.standard_normal(3) * 2
        f, g = L.f(th), ks.D(0.5, th)
        for (ia, ib), M in L.U.items():
            e_prod = max(e_prod, np.max(np.abs((g[ia, ib] * f - M.T @ f)[low])))
        for ia, a in enumerate(ks.HALF):
            for ib, b in enumerate(ks.HALF):
                e_inv = max(e_inv, abs(np.conj(g[ib, ia]) - (-1) ** round(b - a) * g[1 - ib, 1 - ia]))
    for _ in range(10):
        tg, th_, tu = RNG.standard_normal((3, 3))
        c = RNG.standard_normal(L.d) + 1j * RNG.standard_normal(L.d)
        G = sum(tg[a] * L.gen_tail[a] for a in range(3)); Hh = sum(th_[a] * L.gen_head[a] for a in range(3))
        cp = expm(1j * G) @ expm(1j * Hh) @ c
        W = ks.D(0.5, tg) @ ks.D(0.5, tu) @ np.linalg.inv(ks.D(0.5, th_))
        e_gauge = max(e_gauge, abs(c @ L.f(theta_of(W)) - cp @ L.f(tu)))
    return dict(product_rule=e_prod, inverse_link=e_inv, gauge_generators=e_gauge,
                pass_=bool(max(e_prod, e_inv, e_gauge) < 1e-12))


def v2():
    rows = []
    for jmax in (1.0, 2.5):
        for g2 in (0.5, 1.0, 4.0):
            ev = np.linalg.eigvalsh(ks.SpinNetwork(ks.ladder(1), jmax).hamiltonian(g2).toarray())
            N = int(2 * jmax + 1); n = np.arange(1, N + 1)
            ex = eigh_tridiagonal(0.5 * g2 * (n ** 2 - 1) + 4 / g2, np.full(N - 1, -2 / g2), eigvals_only=True)
            rows.append({"jmax": jmax, "g2": g2, "max_diff": float(np.max(np.abs(ev - ex)))})
    return dict(cases=rows, pass_=all(r["max_diff"] < 1e-12 for r in rows))


def v3():
    rows = []
    for graph, jmax, g2 in ((ks.ladder(2), 0.5, 1.0), (THETA, 1.0, 1.3), (THETA, 1.5, 1.3)):
        sn = ks.SpinNetwork(graph, jmax)
        evs = np.linalg.eigvalsh(sn.hamiltonian(g2).toarray())
        bf = ks.BruteForce(graph, jmax)
        H, P = bf.hamiltonian(g2)
        w = np.sort(eigsh((H + P).tocsc(), k=len(evs) + 4, which="SA")[0])
        phys = w[w < 150]
        same = len(phys) == len(evs)
        rows.append({"graph": graph["name"], "jmax": jmax, "spin_network_states": len(evs),
                     "brute_force_dim": bf.dim, "brute_force_physical": int(len(phys)),
                     "max_diff": float(np.max(np.abs(phys - evs))) if same else None})
    return dict(cases=rows, pass_=all(r["max_diff"] is not None and r["max_diff"] < 1e-9 for r in rows))


def v4():
    bf = ks.BruteForce(ks.ladder(2), 0.5)
    H, P = bf.hamiltonian(1.0)
    w, V = eigsh((H + P).tocsc(), k=1, which="SA")
    c = V[:, 0]
    graph = bf.g
    worst, scale = 0.0, 0.0
    for _ in range(6):
        thetas = RNG.standard_normal((bf.nl, 3)) * 1.5
        gauge = RNG.standard_normal((graph["nv"], 3)) * 1.5
        Us = [ks.D(0.5, th) for th in thetas]
        gs = [ks.D(0.5, th) for th in gauge]
        Ut = [gs[tl] @ U @ np.linalg.inv(gs[hd]) for U, (tl, hd) in zip(Us, graph["links"])]
        a = bf.wavefunction(c, thetas)
        b = bf.wavefunction(c, [theta_of(W) for W in Ut])
        worst = max(worst, abs(a - b)); scale = max(scale, abs(a))
    return dict(samples=6, max_change_under_gauge=worst, typical_amplitude=scale, pass_=bool(worst < 1e-9 * max(scale, 1)))


def v5():
    rows = []
    for graph, jmax in ((ks.ladder(2), 2.0), (ks.ladder(3), 1.5), (ks.ladder(3, periodic=True), 1.5)):
        sn = ks.SpinNetwork(graph, jmax); sn.hamiltonian(1.0)
        T = [x.toarray() for x in sn.plaquette_ops]
        inner = np.array([max(s) <= jmax - 1 for s in sn.states])
        comm = max(float(np.max(np.abs((T[a] @ T[b] - T[b] @ T[a])[np.ix_(inner, inner)])))
                   for a in range(len(T)) for b in range(a + 1, len(T)))
        spec = max(float(np.max(np.abs(np.linalg.eigvalsh(x)))) for x in T)
        rows.append({"graph": graph["name"], "jmax": jmax, "states": len(sn.states), "commutator_interior": comm,
                     "max_abs_eigenvalue_trU": spec})
    return dict(cases=rows, pass_=all(r["commutator_interior"] < 1e-12 and r["max_abs_eigenvalue_trU"] <= 2 + 1e-12
                                      for r in rows))


# ── physics ──────────────────────────────────────────────────────────────────────────────────────────────────────────

def assemble(sn, g2):
    n = len(sn.states)
    H = sp.diags(0.5 * g2 * sn.electric()).astype(complex)
    for T in sn.plaquette_ops:
        H = H + (2 / g2) * (2 * sp.identity(n) - T)
    if abs(H.imag).max() > 1e-12:
        raise ValueError("Hamiltonian not real")
    return H.real.tocsc()


def low(H, k=4):
    if H.shape[0] <= 600:
        w, V = eigh(H.toarray())
        return w[:k], V[:, :k]
    w, V = eigsh(H, k=k, which="SA", tol=1e-12)
    o = np.argsort(w)
    return w[o], V[:, o]


def physics(sizes, couplings):
    table = []
    for p, jmaxes in sizes:
        for jmax in jmaxes:
            t0 = time.time()
            sn = ks.SpinNetwork(ks.ladder(p), jmax)
            sn.hamiltonian(1.0)
            E = sn.electric()
            elec_gap_units = float(np.min(E[E > 1e-12]))        # smallest nonzero electric Casimir sum
            for g2 in couplings:
                H = assemble(sn, g2)
                w, V = low(H)
                row = {"p": p, "jmax": jmax, "states": len(sn.states), "g2": g2, "E0": float(w[0]),
                       "E0_per_plaquette": float(w[0] / p), "gap": float(w[1] - w[0]),
                       "gap_electric": 0.5 * g2 * elec_gap_units, "seconds": round(time.time() - t0, 1)}
                row["sublevel_radius_E1"] = float(np.sqrt(1.0 / row["gap"]))
                if len(sn.states) <= 600:                    # the gap three ways, dense
                    Hd = H.toarray() - w[0] * np.eye(len(sn.states))
                    Om = V[:, 0]
                    P = np.eye(len(sn.states)) - np.outer(Om, Om)
                    s = 0.6
                    row["gap_semigroup"] = float(-np.log(np.linalg.norm(expm(-s * Hd) @ P, 2)) / s)
                    X = P @ RNG.standard_normal((len(sn.states), 2000))
                    q = np.einsum("ij,ij->j", X, Hd @ X)
                    row["sublevel_radius_sampled_max"] = float(np.max(np.linalg.norm(X, axis=0) / np.sqrt(q)))
                    row["sublevel_radius_attained"] = float(1 / np.sqrt(V[:, 1] @ Hd @ V[:, 1]))
                table.append(row)
                print({k: (round(v, 6) if isinstance(v, float) else v) for k, v in row.items()}, flush=True)
    return table


def wilson_coefficients(betaF, betaA, jmax):
    """lambda_j / lambda_0 for the time-link weight w(U) = exp(betaF/2 tr U + betaA/3 chi_1(U)): the eigenvalue of the
    time-link convolution on the spin-j sector is c_j/(2j+1), c_j = int w chi_j dU (Haar on the class angle)."""
    def w(a):
        tr = 2 * np.cos(a)
        return np.exp(betaF / 2 * tr + betaA / 3 * (tr ** 2 - 1))
    lam = []
    for j in ks.spins(jmax):
        chi = lambda a, j=j: np.sin((2 * j + 1) * a) / np.sin(a) if abs(np.sin(a)) > 1e-14 else 2 * j + 1  # noqa: E731
        c = quad(lambda a: w(a) * chi(a) * (2 / np.pi) * np.sin(a) ** 2, 0, np.pi, epsabs=1e-14, epsrel=1e-13, limit=200)[0]
        lam.append(c / (2 * j + 1))
    lam = np.array(lam)
    return lam / lam[0]


def reflection(p=3, jmax=2.0, betaF=2.0, betaA_bad=-2.0):
    sn = ks.SpinNetwork(ks.ladder(p), jmax)
    sn.hamiltonian(1.0)
    n = len(sn.states)
    Vmag = sum((2 * sp.identity(n) - T) for T in sn.plaquette_ops).toarray().real
    Ehalf = expm(-0.5 * (betaF / 2) * Vmag)                      # e^{-V/2}: positive whatever the action
    js = np.array(sn.states)
    out = {"p": p, "jmax": jmax, "states": n}
    # independent check of the quadrature: pure Wilson coefficients are Bessel ratios I_{2j+1}/I_1
    lamW = wilson_coefficients(betaF, 0.0, jmax)
    bessel = np.array([iv(2 * j + 1, betaF) / iv(1, betaF) for j in ks.spins(jmax)])
    out["wilson_lambda"] = lamW.tolist()
    out["wilson_vs_bessel_max_rel_err"] = float(np.max(np.abs(lamW / bessel - 1)))

    def transfer(lam):
        idx = np.rint(2 * js).astype(int)
        kin = np.prod(lam[idx], axis=1)                      # each spatial link's spin sector
        return Ehalf @ np.diag(kin) @ Ehalf

    # preparations: every spin-network state at the first time site (so the family reaches every spin sector), and
    # the first few states and plaquette-excited combinations at sites 2 and 3. A negative transfer direction is seen
    # only if the preparations reach it: the kinetic weight of a state is a product over links, so two links in a
    # negative sector cancel and only states with an odd number of such links carry a negative weight.
    I = np.eye(n)
    extra = [I[:, i] for i in range(min(n, 6))]
    extra += [sn.plaquette_ops[k].toarray().real @ I[:, 0] for k in range(len(sn.plaquette_ops))]
    preps = [(I[:, i], 1) for i in range(n)] + [(x, t) for x in extra for t in (2, 3)]
    X = np.array([x for x, _ in preps]).T
    tt = np.array([t for _, t in preps])

    def link_gram(T):
        P = {t: np.linalg.matrix_power(T, t) for t in range(1, 6)}
        G = np.zeros((len(preps), len(preps)))
        for a in (1, 2, 3):
            for b in (1, 2, 3):
                ia, ib = np.where(tt == a)[0], np.where(tt == b)[0]
                G[np.ix_(ia, ib)] = X[:, ia].T @ P[a + b - 1] @ X[:, ib]
        G = 0.5 * (G + G.T)
        ev = np.linalg.eigvalsh(G)
        return float(ev.min()), float(20 * len(preps) * np.finfo(float).eps * np.abs(G).max())

    mn, bd = link_gram(transfer(lamW))
    out["wilson"] = {"lambda_min": float(lamW.min()), "gram_min_eigenvalue": mn, "rounding_bound": bd,
                     "positivity": positivity(mn, bd),
                     "exact_argument": "T = E K E with E = exp(-V/2) symmetric and K diagonal with positive entries "
                                       "(products of the positive Wilson coefficients), so T = S^T S with S = K^(1/2) E: "
                                       "positive by construction (Lean link_reflection_positive covers the pairing); "
                                       "the computed Gram is only checked for numerical consistency with that"}
    lamB = wilson_coefficients(betaF, betaA_bad, jmax)
    negw = np.prod(lamB[np.rint(2 * js).astype(int)], axis=1) < 0
    out["states_with_negative_kinetic_weight"] = int(negw.sum())
    mnb, bdb = link_gram(transfer(lamB))
    out["gate_negative_adjoint"] = {"betaF": betaF, "betaA": betaA_bad, "lambda": lamB.tolist(),
                                    "gram_min_eigenvalue": mnb, "rounding_bound": bdb,
                                    "positivity": positivity(mnb, bdb),
                                    "caught": bool(lamB.min() < 0 and positivity(mnb, bdb) == "negative_witness")}
    out["pass_"] = bool(out["wilson_vs_bessel_max_rel_err"] < 1e-9 and lamW.min() > 0 and out["wilson"]["positivity"] == "numerically_consistent")
    return out


PHYSICS_CALLS = {"n": 0}
VALIDATIONS = [("V1", v1), ("V2", v2), ("V3", v3), ("V4", v4), ("V5", v5)]
SIZES = [(1, (1.5, 2.0, 2.5)), (2, (1.5, 2.0, 2.5)), (3, (1.5, 2.0, 2.5)), (4, (1.5, 2.0, 2.5)), (5, (1.5, 2.0))]
COUPLINGS = (1.0, 2.0)


def finite_size_diagnostic(gaps):
    """Declared on 23 November after the table had been seen: a consistency diagnostic, not a prediction and not a
    bound. Flags a size sequence whose gap falls below half its one-plaquette value, or whose step-to-step decrements
    grow (beyond 5%). A finite list cannot establish uniformity (the 1/n countermodel); this only catches collapse."""
    floor_ok = min(gaps) >= 0.5 * gaps[0]
    dec = [abs(b - a) for a, b in zip(gaps, gaps[1:])]
    shrinking = all(d2 <= 1.05 * d1 for d1, d2 in zip(dec[1:], dec[2:])) if len(dec) > 2 else True
    return {"floor_half_of_first": bool(floor_ok), "decrements_not_growing": bool(shrinking),
            "flagged": not (floor_ok and shrinking)}


def reduce(T, sizes, couplings):
    """Aggregate the table. Observations are reported, not gated: finite-size gaps (P2) and adjacent-cutoff changes
    (P1) are labelled as what they are."""
    out = {}
    conv = []
    for p, jm in sizes:
        for g2 in couplings:
            a, b = [next(r for r in T if r["p"] == p and r["jmax"] == j and r["g2"] == g2)["gap"] for j in jm[-2:]]
            conv.append({"p": p, "g2": g2, "jmax": list(jm[-2:]), "gap_rel_change": abs(b / a - 1)})
    out["P1_adjacent_cutoff_observation"] = {
        "meaning": "relative change of the gap between the two largest cutoffs; agreement of adjacent cutoffs is not "
                   "an error bound to the infinite-cutoff value", "rows": conv}
    vol = []
    for g2 in couplings:
        best = [max((r for r in T if r["p"] == p and r["g2"] == g2), key=lambda r: r["jmax"]) for p, _ in sizes]
        gaps = [r["gap"] for r in best]
        vol.append({"g2": g2, "p": [r["p"] for r in best], "gap": gaps,
                    "radius": [r["sublevel_radius_E1"] for r in best],
                    "E0_per_plaquette": [r["E0_per_plaquette"] for r in best],
                    "finite_size_diagnostic": finite_size_diagnostic(gaps)})
    out["P2_finite_size_observation"] = {
        "meaning": "gaps on the sizes computed, at fixed spacing; not a uniform lower bound", "rows": vol}
    three = [r for r in T if "gap_semigroup" in r]
    out["P3_three_ways"] = {"cases": len(three),
                            "max_semigroup_rel_err": max(abs(r["gap_semigroup"] / r["gap"] - 1) for r in three),
                            "max_radius_attained_rel_err": max(abs(r["sublevel_radius_attained"] / r["sublevel_radius_E1"] - 1) for r in three),
                            "sampled_never_exceeds": all(r["sublevel_radius_sampled_max"] <= r["sublevel_radius_E1"] * (1 + 1e-9) for r in three)}
    out["P3_three_ways"]["pass_"] = bool(out["P3_three_ways"]["max_semigroup_rel_err"] < 1e-8
                                         and out["P3_three_ways"]["max_radius_attained_rel_err"] < 1e-8
                                         and out["P3_three_ways"]["sampled_never_exceeds"])
    out["P4_electric_vs_full"] = {"rows": [{"p": r["p"], "jmax": r["jmax"], "g2": r["g2"], "gap": r["gap"],
                                            "gap_electric": r["gap_electric"]} for r in T if r["jmax"] == 2.0],
                                  "differ_everywhere": all(abs(r["gap"] - r["gap_electric"]) > 1e-6 for r in T)}
    out["P4_electric_vs_full"]["pass_"] = out["P4_electric_vs_full"]["differ_everywhere"]
    # gate: a collapsed gap (every five-plaquette gap replaced by 1e-12) must be flagged by the diagnostic
    collapsed = [dict(r) for r in T]
    for r in collapsed:
        if r["p"] == max(p for p, _ in sizes):
            r["gap"] = 1e-12
            r["sublevel_radius_E1"] = 1e6
    flags = []
    for g2 in couplings:
        best = [max((r for r in collapsed if r["p"] == p and r["g2"] == g2), key=lambda r: r["jmax"]) for p, _ in sizes]
        flags.append(finite_size_diagnostic([r["gap"] for r in best])["flagged"])
    out["gate_collapsed_gap"] = {"fixture": "largest-size gaps replaced by 1e-12 (not simulated physics)",
                                 "flagged": flags, "caught": bool(all(flags))}
    return out


def tidy(o):
    if isinstance(o, dict):
        return {("pass" if k == "pass_" else k): tidy(v) for k, v in o.items()}
    if isinstance(o, list):
        return [tidy(v) for v in o]
    if isinstance(o, (np.floating, np.integer, np.bool_)):
        return o.item()
    return o


def main(out=None, validations=None, sizes=SIZES, couplings=COUPLINGS):
    out = Path(out or sys.argv[1])
    res = {"note": "docs/research/yang-mills-geometric-identities-2026-11-23.md, Sections 5, 7, 9, 10",
           "model": "Kogut-Susskind SU(2) on ladders of p plaquettes, lattice units, link spins <= jmax"}
    for name, fn in (validations or VALIDATIONS):
        t0 = time.time(); res[name] = fn(); print(name, "pass", res[name]["pass_"], round(time.time() - t0, 1), "s",
                                                   flush=True)
    vnames = [n for n, _ in (validations or VALIDATIONS)]
    if not all(res[n]["pass_"] for n in vnames):
        res = tidy(res)
        res["status"] = "FAILED: validation did not pass; no physics computed or published"
        res["ok"] = False
        _receipt_status.publish(out, res)
        print(res["status"])
        return res
    PHYSICS_CALLS["n"] += 1
    res["table"] = physics(sizes, couplings)
    res.update(reduce(res["table"], sizes, couplings))
    t0 = time.time(); res["P5_reflection"] = reflection(); print("P5", res["P5_reflection"]["pass_"],
                                                                 res["P5_reflection"]["gate_negative_adjoint"]["caught"],
                                                                 round(time.time() - t0, 1), "s", flush=True)
    res = tidy(res)
    keys = vnames + ["P3_three_ways", "P4_electric_vs_full", "P5_reflection"]
    res["all_pass"] = all(res[k]["pass"] for k in keys)
    res["gates"] = {"negative_adjoint": res["P5_reflection"]["gate_negative_adjoint"]["caught"],
                    "collapsed_gap": res["gate_collapsed_gap"]["caught"]}
    res["all_gates_caught"] = all(res["gates"].values())
    res["ok"] = res["all_pass"] and res["all_gates_caught"]
    _receipt_status.publish(out, res)
    print({k: res[k]["pass"] for k in keys}, res["gates"], "ok:", res["ok"])
    return res


def selftest_short_circuit(out):
    """A false validation must stop the run before any physics, with a FAILED current receipt."""
    stub = lambda ok: (lambda: {"pass_": ok})  # noqa: E731
    PHYSICS_CALLS["n"] = 0
    _receipt_status.start_run(out)
    r = main(out, validations=[("V1", stub(False)), ("V2", stub(True))])
    rec = json.loads(Path(out).read_text())
    ok = PHYSICS_CALLS["n"] == 0 and rec.get("ok") is False and "FAILED" in rec.get("status", "")
    print("short-circuit self-test:", "PASS" if ok else "FAIL", "| physics calls", PHYSICS_CALLS["n"])
    return ok


if __name__ == "__main__":
    if sys.argv[1] == "--selftest-short-circuit":
        sys.exit(0 if selftest_short_circuit(sys.argv[2]) else 1)
    _receipt_status.run(sys.argv[1], main)
    _receipt_status.exit_on_failure(sys.argv[1])
