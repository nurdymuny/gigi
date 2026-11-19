"""Clue / unknown count for V1 (no new measurement): per cell, how many fluxes the two-ring fit
reads, how many it must reproduce, how much freedom the change C_K has, and how many column-condition
rows reach it. Reads the V1 construction only (scripts/ricci_variational_v1.py).

usage: python scripts/ricci_variational_count.py LEVEL [OUT.json]
"""
import json
import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ricci_variational_v1 as V           # noqa: E402

pg = V.pg


def main():
    level = int(sys.argv[1])
    L = pg.load_level(level, V.ORDER)
    cells = L["cells"]
    faces = pg.face_index(cells)
    C = [V.Cell(cells, faces, ci) for ci in range(len(cells))]
    ne = len(L["a"])
    left_is = "minus"          # as recorded by the V1 receipts (C-span)

    def enforced(c):
        plus, minus, lin = V.generators_on(c, c.verts)
        return np.concatenate([minus if left_is == "plus" else plus, lin], axis=1)

    owner = {}
    for ci, c in enumerate(cells):
        for e in c["edges"]:
            owner.setdefault(e, ci)
    m_e = {}
    for e in range(ne):
        r = C[owner[e]]
        g = enforced(r)
        Sg = np.einsum("qma,qab,qnb,q->mn", g, r.g, g, r.w)
        ev = np.linalg.eigvalsh(Sg)
        m_e[e] = int((ev > 1e-10 * ev.max()).sum())
    rows = sum(m_e.values())
    users = {}
    for k, c in enumerate(C):
        for e in c.tcols:
            users.setdefault(e, []).append(k)
    per = []
    for k, c in enumerate(C):
        nf = c.N.shape[0]                    # fluxes read (clues)
        nz, nq = c.Z.shape[1], c.N.shape[1]  # free coefficient directions, spare clues
        touching = sum(m_e[e] for e in c.tcols)
        per.append({"faces_read": nf, "own_rank": 11 - nz, "Z_dim": nz, "spare_clues": nq,
                    "C_unknowns": nz * nq, "edges_read": len(c.tcols), "rows_touching": touching})
    U = sum(p["C_unknowns"] for p in per)
    # per-edge: rows m_e against the unknowns that can act on it
    loc = [m_e[e] / sum(per[k]["C_unknowns"] for k in users[e]) for e in range(ne)]
    med = lambda key: float(np.median([p[key] for p in per]))   # noqa: E731
    out = {"level": level, "cells": len(C), "edges": ne,
           "fit_unknowns": 11,
           "faces_read_median": med("faces_read"), "faces_read_min": min(p["faces_read"] for p in per),
           "redundancy_median": med("faces_read") / 11,
           "own_rank_median": med("own_rank"), "Z_dim_median": med("Z_dim"),
           "spare_clues_median": med("spare_clues"),
           "C_unknowns_per_cell_median": med("C_unknowns"),
           "C_unknowns_total": U,
           "constraint_rows_total": rows, "rows_per_edge_median": float(np.median(list(m_e.values()))),
           "rows_over_unknowns_global": rows / U,
           "rows_touching_cell_median": med("rows_touching"),
           "rows_touching_over_C_unknowns_median": float(np.median([p["rows_touching"] / p["C_unknowns"] for p in per])),
           "edge_rows_over_acting_unknowns_median": float(np.median(loc))}
    print(json.dumps(out, indent=1))
    if len(sys.argv) > 2:
        Path(sys.argv[2]).write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
