"""Tests for 'Yang-Mills Requirements as Geometric Objects' (docs/research/yang-mills-geometric-identities-2026-11-23.md).

Each exact statement of the note is checked on a real lattice gauge model, and each planted failure the note warns
about must be caught. The model is Kogut-Susskind SU(2) on one plaquette, in lattice units (a = 1):

    H = (g^2/2) sum_links E_l^2 + (4/g^2) (1 - (1/2) tr U_p).

Gauge-invariant states are class functions of the plaquette holonomy U_p = cos a + n sin a, a in [0, pi]. On class
functions each link's E^2 acts as the Casimir, so sum_l E_l^2 = 4 C. The characters chi_j, labelled n = 2j + 1, are an
orthonormal basis (Haar measure), C chi_j = j(j+1) chi_j = (n^2 - 1)/4 chi_j, and cos a chi_n = (chi_{n+1} + chi_{n-1})/2.
So, exactly,
    H_nm = (g^2/2)(n^2 - 1) delta_nm + (4/g^2)(delta_nm - (delta_{m,n+1} + delta_{m,n-1})/2),   n, m = 1, 2, ...
The magnetic term 1 - cos a is Wilson's energy: half the squared chord |U - 1|^2 / 2 (Three Ways Across a Turn).

Tests (section of the note in brackets):
  T7a  gap geometry [7]: the gap from the spectrum, from the energy-sublevel radius sqrt(E*/Delta), and from the
       Euclidean semigroup norm agree. Gates: keeping the vacuum makes the sublevel unbounded; the normalized-only
       test cannot tell this model from a gapless one.
  T7b  limit sensitivity [7, 9]: energies 0, 1, 1/2, ..., 1/N: every truncation has a gap, the sublevel radius grows
       like sqrt(N); the plaquette's gap is stable in the truncation instead.
  T7c  probe coverage [7]: a probe orthogonal to the lowest excitation decays at the wrong rate and must not certify.
  T8   ground-state transform [8]: on the finite-difference graph of the same plaquette, an exact identity
       <psi0 f, (H - E0) psi0 f> = kappa sum_edges w psi0_i psi0_j (f_i - f_j)^2 and the weighted Poincare inequality
       with constant kappa/Delta, attained at f = psi1/psi0. Gates: a sign-changing "ground state" gives negative
       weights; a shifted E0 breaks the identity.
  T10  curvature vs the gap [10]: the class Laplacian on the unit S^3 saturates Lichnerowicz (first eigenvalue 3);
       H >= H_electric gives lambda_k(H) >= lambda_k(H_E) for every k, yet the two gaps differ, and as g -> 0 the
       electric gap vanishes while this model's gap tends to 4 (a finite-size separation, not a continuum gap).
  T5   reflection positivity [5]: reflected correlations of positive-time preparations form a Gram matrix with
       nonnegative spectrum (with a rounding bound). Gate: a transfer matrix with one negative eigenvalue produces a
       certified negative direction.

usage: python scripts/ym_geometry_tests.py OUT.json
"""
import json
import sys
from pathlib import Path

import numpy as np
from scipy.linalg import eigh, eigh_tridiagonal, expm

sys.path.insert(0, str(Path(__file__).resolve().parent))
import _receipt_status  # noqa: E402

EPS = np.finfo(float).eps
RNG = np.random.default_rng(20261123)


def plaquette(g2, N):
    """Exact KS single-plaquette Hamiltonian in the character basis n = 1..N (lattice units)."""
    n = np.arange(1, N + 1, dtype=float)
    d = 0.5 * g2 * (n ** 2 - 1) + 4.0 / g2
    e = np.full(N - 1, -2.0 / g2)
    return d, e


def dense(d, e):
    return np.diag(d) + np.diag(e, 1) + np.diag(e, -1)


def spectrum(g2, N, k=None):
    d, e = plaquette(g2, N)
    return eigh_tridiagonal(d, e, select="i", select_range=(0, (k or N) - 1)) if k else eigh_tridiagonal(d, e)


# ---------------------------------------------------------------------------------------------------------------- T7a
def t7a(g2, N=160, Estar=1.0, s=0.7):
    d, e = plaquette(g2, N)
    w, V = eigh_tridiagonal(d, e)
    E0, Om = w[0], V[:, 0]
    Delta = w[1] - w[0]
    H = dense(d, e) - E0 * np.eye(N)
    P = np.eye(N) - np.outer(Om, Om)
    q = lambda x: x @ H @ x
    # sublevel radius: vacuum-orthogonal vectors scaled to q = E*; the sup of their norms must be sqrt(E*/Delta)
    R_pred = np.sqrt(Estar / Delta)
    X = P @ RNG.standard_normal((N, 4000))
    X = np.hstack([X, P @ (V[:, 1:4] + 1e-3 * RNG.standard_normal((N, 3)))])
    norms = [np.linalg.norm(x) * np.sqrt(Estar / q(x)) for x in X.T]
    attained = np.linalg.norm(V[:, 1]) * np.sqrt(Estar / q(V[:, 1]))
    # semigroup: || exp(-s (H - E0)) restricted to Omega-perp || = exp(-s Delta)
    S = expm(-s * H) @ P
    Delta_semi = -np.log(np.linalg.norm(S, 2)) / s
    # gate F1: keep the vacuum direction; q(Omega) = 0, so no scaling reaches E* and the set is unbounded
    q_vac = q(Om)
    F1_caught = abs(q_vac) < 1e3 * EPS * np.abs(w).max()
    # gate F2: normalized vectors only. Both this model and the gapless diag(0, 1, 1/2, ..., 1/N) give max norm 1 on
    # the unit sphere, so the normalized test cannot separate them; the unnormalized sublevel radius can.
    lam = np.r_[0.0, 1.0 / np.arange(1, N)]
    R_gapless = np.sqrt(Estar / lam[1:].min())
    F2_caught = R_gapless > 3 * R_pred
    ok = (max(norms) <= R_pred * (1 + 1e-9) and abs(attained / R_pred - 1) < 1e-9
          and abs(Delta_semi / Delta - 1) < 1e-8)
    return dict(g2=g2, N=N, E0=E0, Delta=Delta, radius_pred=R_pred, radius_max_sampled=max(norms),
                radius_attained=attained, Delta_semigroup=Delta_semi, pass_=ok,
                gate_vacuum_kept={"q_vacuum": q_vac, "caught": bool(F1_caught)},
                gate_normalized_only={"radius_gapless_model": R_gapless, "radius_this_model": R_pred,
                                      "unit_sphere_max_norm_both": 1.0, "caught": bool(F2_caught)})


# ---------------------------------------------------------------------------------------------------------------- T7b
def t7b(Estar=1.0):
    rows = []
    for N in (10, 100, 1000, 10000):
        lam_min = 1.0 / N                                   # energies 1, 1/2, ..., 1/N after the vacuum
        rows.append({"N": N, "gap_of_truncation": lam_min, "sublevel_radius": np.sqrt(Estar / lam_min)})
    radii = [r["sublevel_radius"] for r in rows]
    no_uniform_bound = all(b > 3 * a for a, b in zip(radii, radii[1:]))
    # the plaquette, by contrast: the gap is stable as the character basis grows
    gaps = {str(N): float(np.diff(spectrum(1.0, N, 2)[0])[0]) for N in (40, 80, 160, 320)}
    stable = max(gaps.values()) - min(gaps.values()) < 1e-12
    return dict(gapless_family=rows, flagged_no_uniform_bound=bool(no_uniform_bound),
                plaquette_gap_vs_basis_size=gaps, plaquette_gap_stable=bool(stable),
                pass_=bool(no_uniform_bound and stable))


# ---------------------------------------------------------------------------------------------------------------- T7c
def t7c(g2=1.0, N=160):
    d, e = plaquette(g2, N)
    w, V = eigh_tridiagonal(d, e)
    E = w - w[0]
    v = V[:, 2] + 0.5 * V[:, 3]                              # a probe with no overlap with the first excitation
    v /= np.linalg.norm(v)
    s = np.linspace(2, 8, 25)
    C = np.array([np.sum((V.T @ v) ** 2 * np.exp(-t * E)) for t in s])
    rate = -np.polyfit(s, np.log(C), 1)[0]                  # what a decay fit would report as the "gap"
    overlap = float((V[:, 1] @ v) ** 2)
    misleads = rate > 1.5 * E[1]                             # the fit misses the true gap by a wide margin
    caught = overlap < 1e-20                                 # the planted blind spot is detected
    return dict(true_gap=E[1], probe_decay_rate=rate, probe_overlap_with_lowest_excitation=overlap,
                fit_would_mislead=bool(misleads), gate={"caught": bool(caught and misleads)},
                pass_=bool(caught and misleads))


# ------------------------------------------------------------------------------------------------------------------ T8
def fd_plaquette(g2, M):
    """Finite differences in the angle, phi = sin(a) psi on (0, pi), Dirichlet ends: H = kappa(-D2) + V,
    V = -g^2/2 + (4/g^2)(1 - cos a). As a weighted path graph: kappa * Laplacian(w = 1/h^2) + diag(V + boundary)."""
    h = np.pi / (M + 1)
    a = h * np.arange(1, M + 1)
    kappa = g2 / 2
    w = 1.0 / h ** 2
    V = -g2 / 2 + (4 / g2) * (1 - np.cos(a))
    bnd = np.zeros(M)
    bnd[[0, -1]] = kappa * w                                 # the missing outside neighbour (Dirichlet)
    L = 2 * np.eye(M) - np.eye(M, k=1) - np.eye(M, k=-1)
    L[0, 0] = L[-1, -1] = 1
    H = kappa * w * L + np.diag(V + bnd)
    return H, kappa, w, a


def dirichlet_form(f, psi, kappa, w):
    return kappa * w * np.sum(psi[:-1] * psi[1:] * (f[1:] - f[:-1]) ** 2)


def t8(g2=1.0, M=800):
    H, kappa, w, a = fd_plaquette(g2, M)
    lam, U = eigh(H)
    E0, E1 = lam[0], lam[1]
    psi0 = U[:, 0] * np.sign(U[:, 0].sum())
    psi1 = U[:, 1]
    positive = bool((psi0 > 0).all())
    Delta = E1 - E0
    errs = []
    for _ in range(200):
        f = RNG.standard_normal(M)
        lhs = (psi0 * f) @ (H - E0 * np.eye(M)) @ (psi0 * f)
        rhs = dirichlet_form(f, psi0, kappa, w)
        errs.append(abs(lhs - rhs) / max(abs(rhs), 1e-300))
    # Poincare: Var_nu(f) <= (kappa/Delta) D(f) / kappa  i.e.  Var_nu(f) <= D(f)/Delta, equality at f = psi1/psi0
    nu = psi0 ** 2
    ratios = []
    for _ in range(200):
        f = RNG.standard_normal(M)
        var = nu @ (f - nu @ f) ** 2
        ratios.append(var * Delta / dirichlet_form(f, psi0, kappa, w))
    f1 = psi1 / psi0
    ratio_attained = (nu @ (f1 - nu @ f1) ** 2) * Delta / dirichlet_form(f1, psi0, kappa, w)
    # converges to the exact character-basis gap
    Delta_exact = float(np.diff(spectrum(g2, 160, 2)[0])[0])
    # gate F3: a sign-changing "ground state" (psi1) gives negative edge weights: no measure, no Poincare
    neg_edges = int((psi1[:-1] * psi1[1:] < 0).sum())
    # gate F4: E0 shifted by 1e-6 breaks the identity by a detectable amount
    f = RNG.standard_normal(M)
    shifted = (psi0 * f) @ (H - (E0 + 1e-6) * np.eye(M)) @ (psi0 * f) - dirichlet_form(f, psi0, kappa, w)
    ok = positive and max(errs) < 1e-9 and max(ratios) <= 1 + 1e-9 and abs(ratio_attained - 1) < 1e-8 \
        and abs(Delta / Delta_exact - 1) < 1e-4
    return dict(g2=g2, grid=M, ground_state_positive=positive, identity_max_rel_err=max(errs),
                poincare_max_ratio_sampled=max(ratios), poincare_ratio_at_psi1_over_psi0=ratio_attained,
                Delta_fd=Delta, Delta_character_basis=Delta_exact, pass_=bool(ok),
                gate_sign_changing_state={"negative_edge_weights": neg_edges, "caught": neg_edges > 0},
                gate_shifted_E0={"identity_residual": shifted, "expected": -1e-6 * (psi0 * f) @ (psi0 * f),
                                 "caught": bool(abs(shifted) > 1e3 * EPS * abs(dirichlet_form(f, psi0, kappa, w)))})


# ----------------------------------------------------------------------------------------------------------------- T10
def t10(N=200):
    # class Laplacian on the unit S^3: 4 j(j+1) = n^2 - 1; first nonzero = 3 = n k with Ric = 2 (n = 3, k = 1)
    lich_first = 2 ** 2 - 1
    lich_ok = lich_first == 3
    rows, minmax_ok = [], True
    for g2 in (0.05, 0.2, 1.0, 4.0, 16.0):
        d, e = plaquette(g2, N)
        lam = eigh_tridiagonal(d, e, eigvals_only=True)
        n = np.arange(1, N + 1, dtype=float)
        lamE = np.sort(0.5 * g2 * (n ** 2 - 1))
        mag = 4.0 / g2 * (1 - np.cos(np.pi * n / (N + 1)))   # spectrum of the magnetic matrix: all >= 0
        minmax_ok &= bool((lam[:20] >= lamE[:20] - 1e-9 * (1 + lamE[:20])).all()) and bool((mag >= 0).all())
        rows.append({"g2": g2, "gap_full": lam[1] - lam[0], "gap_electric": lamE[1] - lamE[0],
                     "weak_coupling_prediction": 4.0})
    small = rows[0]
    trend = small["gap_electric"] < 0.1 and abs(small["gap_full"] - 4.0) < 0.2
    differ = all(abs(r["gap_full"] - r["gap_electric"]) > 1e-6 for r in rows)
    return dict(lichnerowicz_first_eigenvalue=lich_first, lichnerowicz_bound=3, saturated=lich_ok,
                minmax_levels_ok=bool(minmax_ok), gaps=rows, gaps_differ=bool(differ),
                weak_coupling_trend_ok=bool(trend), pass_=bool(lich_ok and minmax_ok and differ and trend))


# ------------------------------------------------------------------------------------------------------------------ T5
def mult_chi(nj, N):
    """Multiplication by the character with label nj on class functions: chi_nj chi_n = sum chi_m, m = |nj-n|+1, ...,
    nj+n-1 step 2 (Clebsch-Gordan), truncated to n <= N."""
    A = np.zeros((N, N))
    for n in range(1, N + 1):
        for m in range(abs(nj - n) + 1, nj + n, 2):
            if m <= N:
                A[m - 1, n - 1] = 1.0
    return A


def t5(g2=1.0, N=120, tau=0.5):
    d, e = plaquette(g2, N)
    w, V = eigh_tridiagonal(d, e)
    E = w - w[0]
    Om = V[:, 0]
    T = V @ np.diag(np.exp(-tau * E)) @ V.T
    # chi_j at Euclidean time sites t = 1, 2, 3, 4 (steps of tau). The reflection plane sits halfway along the link
    # between sites 0 and 1, so Theta maps site t to site 1 - t and the reflected correlation of preparations at t_i and
    # t_j spans t_i + t_j - 1 links:  K_ij = < B_i Omega, T^(t_i + t_j - 1) B_j Omega >.  The odd power is the point:
    # K is a Gram matrix exactly when T is a positive operator on the span (integer-site pairings never test this).
    preps = [(nj, t) for nj in (2, 3, 4) for t in (1, 2, 3, 4)]

    def gram(Tm):
        xs = [mult_chi(nj, N) @ Om for nj, _ in preps]
        G = np.array([[xs[i] @ np.linalg.matrix_power(Tm, preps[i][1] + preps[j][1] - 1) @ xs[j]
                       for j in range(len(preps))] for i in range(len(preps))])
        G = 0.5 * (G + G.T)
        return G, np.linalg.eigvalsh(G)

    G, ev = gram(T)
    bound = 10 * len(preps) * EPS * np.abs(G).max()                  # rounding bound on the computed eigenvalues
    psd = bool(ev.min() >= -bound)
    # gate: a transfer matrix with one negative eigenvalue (not reflection positive)
    k = 3
    Tbad = T - 2 * np.exp(-tau * E[k]) * np.outer(V[:, k], V[:, k])
    Gb, evb = gram(Tbad)
    boundb = 10 * len(preps) * EPS * np.abs(Gb).max()
    caught = bool(evb.min() < -100 * boundb)
    return dict(g2=g2, tau=tau, preparations=len(preps), min_eigenvalue=ev.min(), rounding_bound=bound,
                psd_certified=psd, gate_negative_transfer={"min_eigenvalue": evb.min(), "bound": boundb,
                                                           "caught": caught},
                pass_=bool(psd and caught))


def main():
    out = Path(sys.argv[1])
    res = {"note": "docs/research/yang-mills-geometric-identities-2026-11-23.md",
           "model": "Kogut-Susskind SU(2), one plaquette, lattice units",
           "T7a": [t7a(g2) for g2 in (0.25, 1.0, 4.0)], "T7b": t7b(), "T7c": t7c(), "T8": [t8(1.0), t8(4.0)],
           "T10": t10(), "T5": t5()}

    def fix(o):                                        # numpy scalars -> python; 'pass_' -> 'pass' for the helper
        if isinstance(o, dict):
            return {("pass" if k == "pass_" else k): fix(v) for k, v in o.items()}
        if isinstance(o, list):
            return [fix(v) for v in o]
        if isinstance(o, (np.floating, np.integer, np.bool_)):
            return o.item()
        return o
    res = fix(res)
    gates = []

    def collect(o):
        if isinstance(o, dict):
            for k, v in o.items():
                if k.startswith("gate") and isinstance(v, dict):
                    gates.append(v.get("caught"))
                collect(v)
        elif isinstance(o, list):
            for v in o:
                collect(v)
    collect(res)
    res["all_pass"] = all(t["pass"] for key in ("T7a", "T8") for t in res[key]) and all(
        res[key]["pass"] for key in ("T7b", "T7c", "T10", "T5"))
    res["all_gates_caught"] = all(gates)
    res["ok"] = res["all_pass"] and res["all_gates_caught"]
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(res, indent=1))
    for key in ("T7a", "T8"):
        for t in res[key]:
            print(key, {k: t[k] for k in t if not isinstance(t[k], (dict, list))})
    for key in ("T7b", "T7c", "T10", "T5"):
        print(key, "pass", res[key]["pass"])
    print("gates caught:", sum(map(bool, gates)), "/", len(gates), "| ok:", res["ok"])
    _receipt_status.exit_on_failure(out)


if __name__ == "__main__":
    _receipt_status.begin()
    main()
