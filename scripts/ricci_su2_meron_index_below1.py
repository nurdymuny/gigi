"""Morse index by sector of the squashed meron for t = 0.05..0.95 (step 0.05), sectors j <= 12: the number of negative
eigenvalues of H + G G^* (gauge directions lifted), as in scripts/ricci_su2_meron_family_soft.py, which covers
t = 1..10. The meron is continued down from t = 1 as in scripts/ricci_su2_test2_family.py. Double precision; sampled
stretches and sectors only (Astra's review, finding 2: report the coverage, not a theorem).
usage: python scripts/ricci_su2_meron_index_below1.py OUT.json
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_meron_family_soft as MF     # noqa: E402
import ricci_su2_soft_mode_flow as F         # noqa: E402
import ricci_su2_test2_family as T2          # noqa: E402


def main():
    down = [t for t in T2.GRID[::-1] if t <= 1.0]
    bg = {float(t): x.reshape(3, 3) for t, x in zip(down, MF.follow(down))}
    rows, dev = [], []
    for t in sorted(bg):
        if t >= 1.0:
            continue
        s, c = F.structure(t); m = bg[t]
        sec = {}
        for j in range(13):
            H, G = T2.h_and_g(m, s, c, j)
            ev = np.linalg.eigvalsh(H + G @ G.conj().T)
            neg = int((ev < -1e-9 * max(1.0, np.abs(ev).max())).sum())
            sec[j] = {"neg": neg, "lowest": float(ev[0])}
            if neg != (1 if j == 0 else 0):
                dev.append((t, j, neg))
        rows.append({"t": t, "sectors": sec})
    out = {"t_range": [rows[0]["t"], rows[-1]["t"]], "jmax": 12, "deviations_from_index_one": dev, "rows": rows}
    Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")
    print(f"t = {rows[0]['t']}..{rows[-1]['t']} ({len(rows)} points), j <= 12: deviations from one negative eigenvalue "
          f"(in j = 0): {dev}; smallest |lowest| among j >= 1: "
          f"{min(r['sectors'][j]['lowest'] for r in rows for j in range(1, 13)):.4g}")


if __name__ == "__main__":
    main()
