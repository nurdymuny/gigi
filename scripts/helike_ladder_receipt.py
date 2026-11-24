"""Receipt for the By Enough v2 section "The twist as the top rung" (helicity minus flux).

usage: python scripts/helike_ladder_receipt.py OUT.json

Light: 9(j+1)-dimensional sector matrices only. Checks, at the stated points:
 1. curl is Hermitian, H = curl^2 + Phi, and (H, G) equal the general builder's (built from Q, no curl);
 2. Phi = phi (x) I with the closed-form spectrum of phi (sectors 0 and 3);
 3. the ladder identity curl psi = h_m psi + chi, chi = -i G* psi in the frame-0 slot, leak = 2[l(l+1) - m(m+1)],
    for E = V_j (x) span{w+, w-} defined directly (no eigenspace condition), both branches, all weights;
 4. the twist: h_l^2 - B = lambda, mu_l = kappa - sqrt t (relation to the anatomy's fibre momentum);
 5. the sufficient floor theorem: wherever its condition holds the twist is the gauge-orthogonal floor
    (Berger path and random family members); the gauge factor is needed (counterexample);
    sector-0 threshold 2 sqrt 5 - 2 on the Berger path; the sectors covered at t = 4, 6, 10, 20;
 6. the flux boundary x_flux against Table 2's x*_A;
 7. the flux gap B - |Theta| on A = s theta (zero on the round sphere);
 8. observation: the twist is the gauge-orthogonal floor of every sector j <= 12 at t = 10, 12, 15, 20.
"""
import json
import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
_argv, sys.argv = sys.argv, sys.argv[:1]
import ricci_su3_test2_family as T2  # noqa: E402
sys.argv = _argv
import helike_curl_spectrum as hc  # noqa: E402
import ricci_su2_spin_sector as rs  # noqa: E402


def theta(x, y, t):
    return y * (x - 2 / np.sqrt(t))


def phi_spec(B, k):
    r = np.sqrt(B * B + 8 * k * k)
    return np.sort([-B, -B, B, k, -k, k, -k, (B + r) / 2, (B - r) / 2])


def go_floor(x, y, t, j):
    _, H, _, _, _ = hc.operators(x, y, t, j)
    return float(hc.gauge_orthogonal(H, hc.gauge_map(x, y, t, j))[0][0])


def main(out):
    rec = {}
    S, C2, eps = T2.su2_data()
    rng = np.random.default_rng(20261124)
    # 1
    w = dict(herm=0., square=0., builder_H=0., builder_G=0.)
    for _ in range(12):
        x, y, t, j = rng.uniform(-3, 1), rng.uniform(0, 3), rng.uniform(.5, 10), int(rng.integers(0, 7))
        C, H, Phi, _, _ = hc.operators(x, y, t, j)
        Hr, Gr = T2.hessian_and_gauge(C2, [t, 1, 1], eps, [x * np.eye(3)[0], y * np.eye(3)[1], y * np.eye(3)[2]],
                                      S.spin_rep(j))
        w["herm"] = max(w["herm"], float(np.abs(C - C.conj().T).max()))
        w["square"] = max(w["square"], float(np.abs(H - C @ C - Phi).max()))
        w["builder_H"] = max(w["builder_H"], float(np.abs(H - Hr).max()))
        w["builder_G"] = max(w["builder_G"], float(np.abs(hc.gauge_map(x, y, t, j) - Gr).max()))
    rec["curl_form"] = dict(points=12, **w)
    # 2
    worst = 0.
    for _ in range(12):
        x, y, t = rng.uniform(-3, 2), rng.uniform(.2, 3), rng.uniform(.5, 12)
        B, k = hc.fibre_field(x, y, t), theta(x, y, t)
        for j in (0, 3):
            ev = np.linalg.eigvalsh(hc.operators(x, y, t, j)[2])
            worst = max(worst, float(np.abs(ev - np.repeat(phi_spec(B, k), j + 1)).max()))
    rec["flux_spectrum"] = dict(points=12, sectors=[0, 3], max_error=worst)
    # 3 and 4
    worst_id, worst_leak, worst_tw, worst_rel = 0., 0., 0., 0.
    pts = [(-2., 1.5, 4., 4), (.7, .3, 1.5, 3), (*hc.berger_xy(6.), 6., 5), (1.2, .4, 1.5, 6),
           (-.4, 2.2, 2.2, 2), (*hc.berger_xy(1.), 1., 3)]
    for (x, y, t, j) in pts:
        C, H, Phi, _, _ = hc.operators(x, y, t, j)
        G = hc.gauge_map(x, y, t, j)
        B, d, l = hc.fibre_field(x, y, t), j + 1, j / 2
        rho = rs.spin_rep(j)
        J = [0.5j * r for r in rho]
        ev, V = np.linalg.eigh(J[0])
        E = []
        for sgn in (1, -1):
            u = np.array([0, 1, 1j * sgn])
            Jpm = J[1] + 1j * sgn * J[2]
            for m in np.arange(-l, l + 1):
                f = V[:, np.argmin(np.abs(ev - m))]
                psi = np.zeros(9 * d, complex)
                psi[3 * d:6 * d] = np.kron(u, f)
                psi[6 * d:] = 1j * sgn * np.kron(u, f)
                E.append(psi / np.linalg.norm(psi))
                mm = m if sgn == 1 else -m
                h = -(x + (2 * mm + 2) / np.sqrt(t))
                chi = C @ psi - h * psi
                worst_id = max(worst_id, float(np.abs(chi[3 * d:]).max()),
                               float(np.abs(chi[:3 * d] - 2 * sgn * np.kron(u, Jpm @ f)).max()),
                               float(np.abs(chi[:3 * d] + 1j * sgn * (G.conj().T @ psi)[:3 * d]).max()),
                               float(np.abs(Phi @ psi + B * psi).max()))
                ratio = np.linalg.norm(chi) ** 2 / np.linalg.norm(psi) ** 2
                worst_leak = max(worst_leak, abs(ratio - 2 * (l * (l + 1) - mm * (mm + 1))))
        Pm = np.array(E).T
        P = Pm @ Pm.conj().T
        comp = np.linalg.eigvalsh(Pm.conj().T @ C @ Pm)
        want = np.sort([-(x + (2 * m + 2) / np.sqrt(t)) for m in np.arange(-l, l + 1) for _ in (0, 1)])
        worst_id = max(worst_id, float(np.abs(np.sort(comp) - want).max()))
        lam = hc.twist_closed_form(x, y, t, j)
        hl = -(x + (j + 2) / np.sqrt(t))
        kap = (t - 2) / np.sqrt(t) - x - j / np.sqrt(t)
        worst_tw = max(worst_tw, abs(hl * hl - B - lam))
        worst_rel = max(worst_rel, abs(hl - (kap - np.sqrt(t))))
        _ = P
    rec["ladder_identity"] = dict(points=[list(map(float, p)) for p in pts], identity_max_error=worst_id,
                                  leak_formula_max_error=float(worst_leak))
    rec["twist"] = dict(lambda_minus_h2_plus_B=float(worst_tw), h_minus_kappa_plus_sqrt_t=float(worst_rel))
    # 5
    cases, margin_min = 0, np.inf
    for t in (2.48, 2.6, 3., 4., 6., 10., 20., 40.):
        x, y = hc.berger_xy(t)
        for j in range(0, 21):
            if hc.floor_sufficient(x, y, t, j):
                cases += 1
                margin_min = min(margin_min, go_floor(x, y, t, j) - hc.twist_closed_form(x, y, t, j))
    rnd, rnd_min = 0, np.inf
    for _ in range(300):
        x, y, t = rng.uniform(-3, 1), rng.uniform(.1, 3), rng.uniform(1.5, 12)
        for j in range(0, 7):
            if hc.floor_sufficient(x, y, t, j):
                rnd += 1
                rnd_min = min(rnd_min, go_floor(x, y, t, j) - hc.twist_closed_form(x, y, t, j))
    cx = (0.0674, 2.6596, 3.5715, 1)
    B, k = hc.fibre_field(*cx[:3]), theta(*cx[:3])
    hmu2 = (cx[0] + (cx[3] + 2) / np.sqrt(cx[2])) ** 2
    covered = {str(t): [j for j in range(0, 61 if t == 40 else 41) if hc.floor_sufficient(*hc.berger_xy(t), t, j)]
               for t in (4., 6., 10., 20., 40.)}
    t0 = 2 * np.sqrt(5) - 2
    rec["floor_theorem"] = dict(
        berger_cases=cases, berger_min_margin=float(margin_min),
        random_cases=rnd, random_min_margin=float(rnd_min),
        counterexample=dict(point=list(cx), weak_condition_holds=bool(B - abs(k) >= hmu2),
                            theorem_condition_holds=bool(hc.floor_sufficient(*cx)),
                            floor_minus_twist=go_floor(*cx) - hc.twist_closed_form(*cx)),
        sector0_threshold=float(t0),
        sector0_below=bool(hc.floor_sufficient(*hc.berger_xy(t0 - 1e-9), t0 - 1e-9, 0)),
        sector0_above=bool(hc.floor_sufficient(*hc.berger_xy(t0 + 1e-9), t0 + 1e-9, 0)),
        sectors_covered=covered)
    # 6  (By Enough Table 2: x*_A / x*_S at y = 0.5, 1.5, sqrt t)
    tab = {2: [(-0.613240, -0.605292), (-0.682793, -0.657567), (-0.677814, -0.655100)],
           3: [(-0.335488, -0.318061), (-0.371582, -0.351580), (-0.382168, -0.355187)],
           4: [(-0.202972, -0.191702), (-0.208048, -0.191652), (-0.221163, -0.189150)],
           6: [(-0.082842, -0.078032), (-0.044874, -0.033249), (-0.035386, +0.008031)],
           8: [(-0.033799, -0.031530), (+0.031482, +0.040169), (+0.082513, +0.138269)]}
    cells = []
    for t, row in tab.items():
        for y, (xa, xs) in zip((.5, 1.5, np.sqrt(t)), row):
            xf = float(hc.x_flux(y, t))
            cells.append(dict(t=t, y=float(y), x_flux=xf, x_star_A=xa, x_star_S=xs, margin=xf - max(xa, xs)))
    rec["flux_boundary"] = dict(cells=cells, all_right_of_edges=all(c["margin"] > 0 for c in cells),
                                min_margin=min(c["margin"] for c in cells))
    # 7
    rec["s_theta_flux_gap"] = [dict(s=s, t=t, B=4 * s * (s - 1), Theta=float(theta(2 * s / np.sqrt(t), 2 * s, t)),
                                    gap=float(4 * s * (s - 1) - abs(theta(2 * s / np.sqrt(t), 2 * s, t))))
                               for s in (1.5, 2.) for t in (1., 3., 8.)]
    # 8
    rec["no_upper_edge_observation"] = [hc.per_sector_status(t, 12) for t in (10., 12., 15., 20.)]
    for r in rec["no_upper_edge_observation"]:
        r["H_worst"] = [int(r["H_worst"][0]), float(r["H_worst"][1])]
        r["curl_worst"] = [int(r["curl_worst"][0]), float(r["curl_worst"][1])]
        r["H_every_sector"], r["curl_every_sector"] = bool(r["H_every_sector"]), bool(r["curl_every_sector"])
    Path(out).write_text(json.dumps(rec, indent=1))
    print(json.dumps(rec, indent=1))


if __name__ == "__main__":
    main(sys.argv[1])
