"""How accurate are cell means? Read-only diagnostic, writes a receipt.

Metric-weighted cell mean (projection onto constant proxies in the face_mass
inner product) of the Whitney curvature and of the closed linear star fit,
each against the exact curvature's mean, relative over all cells, levels 0-2.
A correction with zero mean on every cell keeps Whitney's means exactly, so
Whitney's mean accuracy caps that construction's curvature accuracy.
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_pg_curvature as pg                   # noqa: E402
from ricci_consistency_math import INCIDENCE      # noqa: E402


def main():
    path = pg.OUT / "cell_means.json"
    if path.exists():
        raise SystemExit(f"refusing to overwrite {path}")
    rows = []
    for level in (0, 1, 2):
        L = pg.load_cells(level)
        cells, a = L["cells"], L["a"]
        faces = pg.face_index(cells)
        nw = nf = den = 0.
        for start in range(0, len(cells), 400):
            idx = range(start, min(start + 400, len(cells)))
            q = pg.quadrature(np.array([cells[i]["vertices"] for i in idx], float), 6)
            wq = q["measure"] / q["det"]
            Pex = pg.exact_proxy_from_maurer(q["maurer"])
            for li, ci in enumerate(idx):
                c = cells[ci]
                s = np.where(c["reverse"], -1., 1.)
                g, w = q["g"][li], wq[li]
                Gm = np.einsum("qab,q->ab", g, w)
                mean = lambda P: np.linalg.solve(Gm, np.einsum("qab,qb,q->a", g, P, w))  # noqa: E731
                mF = mean(Pex[li])
                dw = mean(np.einsum("qfa,f->qa", q["two"], INCIDENCE @ (s * a[c["edges"]]))) - mF
                df = mean(pg.proxy_ref(pg.fit_cell(cells, a, faces, ci), q["xref"])) - mF
                nw += float(dw @ Gm @ dw)
                nf += float(df @ Gm @ df)
                den += float(mF @ Gm @ mF)
        rows.append({"level": level, "whitney_mean_error": float(np.sqrt(nw / den)),
                     "fit_mean_error": float(np.sqrt(nf / den))})
        print(rows[-1])
    path.write_text(json.dumps({"kind": "read-only diagnostic", "rows": rows,
                                "source_sha256": {"scripts/ricci_cell_mean_check.py": pg.sha(__file__),
                                                  "scripts/ricci_pg_curvature.py": pg.sha(pg.__file__)}},
                               indent=2) + "\n")
    print("wrote", path.relative_to(pg.ROOT))


if __name__ == "__main__":
    main()
