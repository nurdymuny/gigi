"""Which lattice eigenvectors are the exact left-invariant sector? (check of Table 9, Reading the Curve)

Table 9 matches exact sector values to lattice eigenvalues by nearest value. Near -1 the lattice has many
gauge-type eigenvalues, so this script identifies the sector modes by their eigenvectors instead.

The 9 discrete left-invariant directions: the Berger field's link on edge (p, q) is
U = exp(-1/2 i sum_a c_a sigma_a), c_a = x_a xi_a (e2e/probes/ricci_lattice_demo.matrix_link), with xi the
left-frame log of p^-1 q. Perturbing the connection by a left-invariant su(2)-valued 1-form, c_a -> c_a + eps xi_i
for direction (i, a), changes every link; its right-trivialised coordinate (the coordinates of K's columns,
U -> U exp(i t . sigma)) is t = log(U0^-1 U(eps)) / eps. The span L of those 9 vectors is the discrete sector.

For every eigenpair (lambda, w) of (K, B) with Re lambda in [-6, 40], the B-overlap ||P_L w||_B^2 / ||w||_B^2 is
reported, together with the nearest-value matches of Table 9.

K: the B5 batched Jacobian, rebuilt in memory (scripts/ricci_su2_batched.py); B: the native tangent mass.
usage: python scripts/ricci_su2_sector_eigvec_check.py LEVEL TRIAL [OUT.json]    LEVEL 0, 1; TRIAL native | pg
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
from scipy.linalg import eig, expm, logm

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
sys.path.insert(0, str(HERE)); sys.path.insert(0, str(REPO / "e2e" / "probes"))
import ricci_su2_batched as RB                                         # noqa: E402
import ricci_su2_pg_flow as FL                                         # noqa: E402
from ricci_su2_pg_force import compose                                 # noqa: E402
from ricci_lattice_demo import matrix, SIGMA                           # noqa: E402
from ricci_wilson_oracle import connection_matrices, axial              # noqa: E402
from test_ricci_operator_native import sparse_matrix                    # noqa: E402

EXACT = [(-14 / 3, 2), ((29 - np.sqrt(1345)) / 3, 2), (16 - 2 * np.sqrt(73), 1), (8.0, 1),
         ((29 + np.sqrt(1345)) / 3, 2), (16 + 2 * np.sqrt(73), 1)]
GAUGE = {8.0: "gauge", (29 - np.sqrt(1345)) / 3: "mixed", (29 + np.sqrt(1345)) / 3: "mixed"}


def link_matrix(c):
    return expm(-0.5j * np.einsum("a,ajk->jk", c, SIGMA))


def main():
    level, trial = int(sys.argv[1]), sys.argv[2]
    outp = Path(sys.argv[3]) if len(sys.argv) > 3 else None
    if outp and outp.exists():
        raise SystemExit(f"refusing to overwrite {outp}")
    t0 = time.time()
    bat = RB.BatchedLattice(level, trial)
    K = bat.build_K().toarray()
    payload = json.loads((FL.F.BERGER / f"level{level}_berger_q6.input.json").read_text())
    raw = json.loads((FL.F.BERGER / f"level{level}_berger_q6.output.json").read_text())
    B = sparse_matrix(raw).toarray()
    links = np.asarray(raw["links"], float)
    n = len(links)
    out = {"level": level, "trial": trial, "dof": 3 * n,
           "kind": "eigenvector check (research script, not an engine run)"}

    # controls: quaternion product = matrix product; stored links reproduced by the formula
    rng = np.random.default_rng(0)
    qa, qb = rng.normal(size=4), rng.normal(size=4); qa /= np.linalg.norm(qa); qb /= np.linalg.norm(qb)
    out["C_compose"] = float(np.abs(matrix(compose(qa, qb)) - matrix(qa) @ matrix(qb)).max())
    x = np.diag(axial(connection_matrices(np.array([3.0, 1.0, 1.0]))))
    L = np.zeros((3 * n, 9)); worst_link = 0.0; eps = 1e-6
    for e, (p, q) in enumerate(payload["edge_points"]):
        p, q = np.asarray(p, float), np.asarray(q, float)
        a_ = matrix(np.r_[p[0], -p[1:]]); b_ = matrix(np.r_[q[0], -q[1:]])
        lg = logm(a_.conj().T @ b_)
        xi = np.array([(0.5j * np.trace(s @ lg)).real for s in SIGMA])
        c0 = x * xi
        U0 = link_matrix(c0)
        worst_link = max(worst_link, float(min(np.abs(U0 - matrix(links[e])).max(),
                                               np.abs(U0 + matrix(links[e])).max())))
        U0i = U0.conj().T
        for i in range(3):
            for a in range(3):
                d = np.zeros(3); d[a] = xi[i]
                tp = FL.qlog(FL.to_quat(U0i @ link_matrix(c0 + eps * d)))
                tm = FL.qlog(FL.to_quat(U0i @ link_matrix(c0 - eps * d)))
                L[3 * e:3 * e + 3, 3 * i + a] = (tp - tm) / (2 * eps)
    out["C_links_reproduced_max"] = worst_link
    G = L.T @ B @ L
    out["L_rank"] = int(np.linalg.matrix_rank(G, tol=1e-10 * np.abs(G).max()))
    print("controls: compose", out["C_compose"], "links", worst_link, "rank L", out["L_rank"], flush=True)

    lam, W = eig(K, B)
    t_eig = time.time()
    # C-repro against B6's committed dense eigenvalues
    ref = REPO / "research/su2-exact-stability-20261119" / f"dense_L{level}_{trial}.eigs.npy"
    if ref.exists():
        r = np.load(ref)
        d = np.abs(lam[:, None] - r[None, :]).min(axis=1)
        out["C_repro_vs_B6_max_rel"] = float(d.max() / np.abs(r).max())
    Gi = np.linalg.inv(G)
    sel = np.where((lam.real > -6) & (lam.real < 40))[0]
    rows = []
    for k in sel:
        w = W[:, k]
        nb = float(np.real(w.conj() @ B @ w))
        cvec = Gi @ (L.T @ B @ w)
        ov = float(np.real(cvec.conj() @ G @ cvec) / nb)
        rows.append({"re": float(lam[k].real), "im": float(lam[k].imag), "overlap": ov})
    rows.sort(key=lambda r: r["re"])
    out["eigs_in_window"] = rows
    # sector modes identified by overlap; compare with nearest-value matching
    sector = sorted([r for r in rows if r["overlap"] > 0.5], key=lambda r: r["re"])
    out["sector_modes_by_overlap"] = sector
    match = []
    for val, mult in EXACT:
        near = sorted(rows, key=lambda r: abs(r["re"] - val))[:mult]
        match.append({"exact": val, "mult": mult, "content": GAUGE.get(val, "physical"),
                      "nearest": near, "nearest_overlaps": [r["overlap"] for r in near]})
    out["nearest_value_matches"] = match
    out["count_overlap_gt_0.5"] = len(sector)
    out["seconds"] = round(time.time() - t0, 1)
    print(f"{len(sel)} eigenvalues in [-6, 40]; {len(sector)} with sector overlap > 0.5 (expect 9)", flush=True)
    for m in match:
        print(f"  exact {m['exact']:9.4f} x{m['mult']} [{m['content']}]: nearest " +
              ", ".join(f"{r['re']:.4f}{r['im']:+.3f}i (ov {r['overlap']:.3f})" for r in m["nearest"]), flush=True)
    print("  by overlap:", ", ".join(f"{r['re']:.4f} ({r['overlap']:.3f})" for r in sector), flush=True)
    weak = [r for r in rows if 0.1 < r["overlap"] <= 0.5]
    out["partial_overlap_0.1_to_0.5"] = weak
    print("  partial (0.1-0.5):", ", ".join(f"{r['re']:.4f}{r['im']:+.3f}i ({r['overlap']:.3f})" for r in weak), flush=True)
    if outp:
        outp.parent.mkdir(parents=True, exist_ok=True)
        outp.write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
