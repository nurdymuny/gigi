"""Exterior residuals and rigorous residual enclosures for the Kogut-Susskind SU(2) ladders (the HP08 test of the
Helike quantum probe, applied to the link-spin cutoff).

A plaquette operator changes every link spin it touches by exactly 1/2. So for a state psi supported on link spins
<= jmax, the full (untruncated) Hamiltonian maps it into link spins <= jmax + 1/2, and H psi can be computed exactly
by acting locally, without enumerating the larger space. Then
    eps = || (H - E) psi ||        (in the untruncated lattice Hilbert space, not the truncated one)
is exact, and Weinstein's theorem gives, rigorously, an eigenvalue of the untruncated lattice Hamiltonian in
[E - eps, E + eps]. Inside the truncated space the residual is zero (E is an eigenvalue there); all of eps is the
part of H psi that the cutoff threw away. This replaces "adjacent cutoffs agree" (a diagnostic) by an enclosure.

usage: python scripts/ym_ladder_exterior.py OUT.json            (the open ladders of Section 12 of the note)
"""
import itertools
import json
import sys
import time
from pathlib import Path

import numpy as np
from scipy.linalg import eigh
from scipy.sparse.linalg import eigsh

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ks_su2_lattice as ks  # noqa: E402
import ym_ladder_excitation as X  # noqa: E402


class Local(ks.SpinNetwork):
    """The same graph and intertwiner conventions as SpinNetwork, without enumerating the basis: plaquette operators
    act on one state at a time, and admissibility is checked at the vertices they touch."""

    def __init__(self, graph, jmax):  # noqa: D107 - deliberately skips enumeration
        self.g, self.jmax = graph, jmax
        self.ends = ks.ends_at(graph)
        self.nl = len(graph["links"])

    def apply(self, pl, s):
        steps, verts = self.g["plaqs"][pl]
        n = len(steps)
        links = [e for e, _ in steps]
        out = {}
        for shifts in itertools.product((-0.5, 0.5), repeat=n):
            s2 = list(s)
            for e, dj in zip(links, shifts):
                s2[e] = s[e] + dj
            if any(j < 0 or j > self.jmax + 1e-9 for j in (s2[e] for e in links)):
                continue
            s2 = tuple(s2)
            if not all(self._vertex_ok(v, s2) for v in verts):
                continue
            amp = np.prod([np.sqrt((2 * s[e] + 1) / (2 * s2[e] + 1)) for e in links])
            for k, v in enumerate(verts):
                xmap = {}
                e_out, fwd_out = steps[k]
                e_in, fwd_in = steps[k - 1]
                xmap[(e_out, "tail") if fwd_out else (e_out, "head")] = ks._plus if fwd_out else ks._minus
                xmap[(e_in, "head") if fwd_in else (e_in, "tail")] = ks._plus if fwd_in else ks._minus
                phase = {(True, True): ks._ph_none, (False, True): ks._ph_out, (True, False): ks._ph_in,
                         (False, False): ks._ph_both}[(fwd_out, fwd_in)]
                amp = amp * self._vertex_factor(v, s, s2, xmap, phase)
                if abs(amp) < 1e-15:
                    break
            if abs(amp) > 1e-14:
                out[s2] = out.get(s2, 0) + amp
        return out


def full_action(local, states, vec, g2):
    """(H psi) in the untruncated space, as a dict state -> amplitude."""
    out = {}
    for s, c in zip(states, vec):
        if c == 0:
            continue
        el = sum(j * (j + 1) for j in s)
        out[s] = out.get(s, 0) + (0.5 * g2 * el + (2 / g2) * 2 * len(local.g["plaqs"])) * c
        for pl in range(len(local.g["plaqs"])):
            for s2, a in local.apply(pl, s).items():
                out[s2] = out.get(s2, 0) - (2 / g2) * a * c
    return out


def enclosures(p, jmax, g2, k=6, periodic=False):
    lad = X.Ladder(p, jmax, periodic)
    H = lad.hamiltonian(g2)
    n = H.shape[0]
    if n <= 2500:
        w, V = eigh(H.toarray()); w, V = w[:k], V[:, :k]
    else:
        w, V = eigsh(H, k=k, which="SA", tol=1e-12); o = np.argsort(w); w, V = w[o], V[:, o]
    local = Local(lad.graph, jmax + 0.5)
    rows = []
    for i in range(min(k, 3)):
        act = full_action(local, lad.sn.states, V[:, i], g2)
        index = lad.sn.index
        interior = np.zeros(n); exterior = 0.0
        for s, a in act.items():
            if s in index:
                interior[index[s]] += np.real(a)
            else:
                exterior += abs(a) ** 2
        r_in = float(np.linalg.norm(interior - w[i] * V[:, i]))
        eps = float(np.sqrt(r_in ** 2 + exterior))
        rows.append({"level": i, "E": float(w[i]), "interior_residual": r_in, "residual": eps,
                     "weinstein": [float(w[i] - eps), float(w[i] + eps)]})
    out = {"p": p, "jmax": jmax, "g2": g2, "periodic": periodic, "states": n, "levels": rows}
    if len(rows) >= 2:
        e0, e1 = rows[0], rows[1]
        out["gap"] = e1["E"] - e0["E"]
        out["gap_interval_if_levels_identified"] = [e1["weinstein"][0] - e0["weinstein"][1],
                                                    e1["weinstein"][1] - e0["weinstein"][0]]
    return out


def selftest():
    """The local action equals the enumerated larger space's operator, column by column."""
    lad = X.Ladder(2, 1.0)
    big = ks.SpinNetwork(ks.ladder(2), 1.5)
    big.hamiltonian(1.0)
    H_big = X.Ladder.__new__(X.Ladder)
    local = Local(lad.graph, 1.5)
    worst = 0.0
    for col, s in enumerate(lad.sn.states):
        vec = np.zeros(len(lad.sn.states)); vec[col] = 1.0
        act = full_action(local, lad.sn.states, vec, 1.3)
        dense = np.zeros(len(big.states))
        for s2, a in act.items():
            dense[big.index[s2]] += np.real(a)
        n = len(big.states)
        Hb = (np.diag(0.5 * 1.3 * np.array([sum(j * (j + 1) for j in t) for t in big.states]))
              + (2 / 1.3) * sum(2 * np.eye(n) - T.toarray().real for T in big.plaquette_ops))
        worst = max(worst, float(np.max(np.abs(dense - Hb[:, big.index[s]]))))
    return worst


def main():
    out = Path(sys.argv[1])
    sys.path.insert(0, str(HERE))
    import _receipt_status as R
    res = {"selftest_local_vs_enumerated": selftest()}
    print("selftest", res["selftest_local_vs_enumerated"], flush=True)
    rows = []
    for p, jm in ((1, 2.5), (2, 2.5), (3, 2.5), (4, 2.5), (5, 2.0)):
        for g2 in (2.0, 1.0):
            t0 = time.time()
            e = enclosures(p, jm, g2)
            e["seconds"] = round(time.time() - t0, 1)
            rows.append(e)
            print(p, jm, g2, "gap", round(e["gap"], 6), "residual E0", f"{e['levels'][0]['residual']:.2e}",
                  "E1", f"{e['levels'][1]['residual']:.2e}", e["seconds"], "s", flush=True)
    res["enclosures"] = rows
    res["ok"] = bool(res["selftest_local_vs_enumerated"] < 1e-12)
    R.publish(out, res)


if __name__ == "__main__":
    import _receipt_status as R  # noqa: F811
    R.run(sys.argv[1], main)
    R.exit_on_failure(sys.argv[1])
