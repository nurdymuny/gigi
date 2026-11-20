"""Charge blocks of the floor problem (spectral-skeleton note, section 7). Double precision.

The fibre charge Q = J0 + R_frame + R_Lie commutes with H + G G^* in every sector j on the Berger sphere (t, 1, 1), so
each sector splits into blocks of fixed charge, each at most 9 x 9. Reports, per (t, j): the twist's block (minimum
equal to lambda_t(j)), and the smallest margin lambda_min(block) - lambda_t(j) over every other block.
usage: python scripts/ricci_su2_charge_blocks.py OUT.json
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_sector_gauge_check as C   # noqa: E402
import ricci_su2_spin_sector as S          # noqa: E402
import ricci_su2_chiral_bound as CB        # noqa: E402

lam = lambda t, j: (j * (j + 2) - 2 * (t - 3) * j - 2 * (t - 2) * (t + 4)) / t     # noqa: E731


def blocks(t, j):
    H, G = C.hessian_and_gauge([t, 1, 1], j); A = H + G @ G.conj().T; d = j + 1; rho = S.spin_rep(j); I3 = np.eye(3)
    Q = (np.kron(np.eye(9), 0.5j * rho[0]) + np.kron(np.kron(CB.rot0(), I3), np.eye(d))
         + np.kron(np.kron(I3, CB.rot0()), np.eye(d)))
    q, U = np.linalg.eigh(0.5 * (Q + Q.conj().T)); out = []
    for qv in np.unique(np.round(q, 8)):
        P = U[:, np.abs(q - qv) < 1e-6]; ev = np.linalg.eigvalsh(P.conj().T @ A @ P)
        out.append({"q": float(qv), "size": int(P.shape[1]), "min": float(ev[0]), "second": float(ev[1]) if len(ev) > 1 else None})
    return out


def main():
    rows = []
    for t in (2.75, 3.0, 4.0, 6.0, 10.0):
        for j in (1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 128):
            bl = blocks(t, j); L = lam(t, j)
            tw = [b for b in bl if abs(b["min"] - L) < 1e-8]
            others = [b for b in bl if abs(b["min"] - L) >= 1e-8]
            mo = min(others, key=lambda b: b["min"])
            rows.append({"t": t, "j": j, "lambda": L, "max_block": max(b["size"] for b in bl),
                         "twist_blocks_q": [b["q"] for b in tw],
                         "twist_block_second_minus_lambda": min([b["second"] - L for b in tw if b["second"] is not None], default=None),
                         "min_margin_other_blocks": mo["min"] - L, "at_q": mo["q"]})
            r = rows[-1]
            print(f"t={t} j={j}: blocks<= {r['max_block']}, twist q={r['twist_blocks_q']}, next in twist block +"
                  f"{r['twist_block_second_minus_lambda']}, other blocks margin {r['min_margin_other_blocks']:.3f} (q={r['at_q']})", flush=True)
    Path(sys.argv[1]).write_text(json.dumps(rows, indent=1) + "\n")


if __name__ == "__main__":
    main()
