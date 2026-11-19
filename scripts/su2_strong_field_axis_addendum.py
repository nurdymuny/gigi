"""S4 exploratory addendum (written AFTER seeing S4's results; not a gate).
Hypothesis: the 2:1 blend loses its cancellation on GIGI's Whitney-mass action because the native
chord reading U - 1 carries a scalar part (cos a - 1) that enters the face-coupled form at the order
the blend should cancel. Test: the same blend with the chord read along the holonomy axis,
2 sin(a/2) n (pure vector, same magnitude as |U - 1|).
usage: python scripts/su2_strong_field_axis_addendum.py [RECEIPT.json]"""
import json, sys
from pathlib import Path
import numpy as np
sys.path.insert(0, str(Path(__file__).resolve().parent))
import su2_strong_field_energy as S4
import ricci_su2_pg_flow as FL
import ricci_su2_pg_force as F

def run():
    rows = []
    for level in S4.LEVELS:
        lat = FL.Lattice(level, "native")
        for Aval in S4.A_VALUES:
            g = np.array([Aval, 1.0, 1.0]); mp = lat.metric_pieces(g); L = lat.exact_links(g)
            E = {"arc": 0.0, "chord_axis": 0.0, "tangent": 0.0}
            for ci, c in enumerate(lat.cells):
                U = F.defects(F.local_links(c, L)) + S4.ONE
                R, a = S4.readings(U)
                n = R["arc"][:, 1:] / np.where(a > 1e-300, a, 1)[:, None]
                ca = np.hstack([np.zeros((4, 1)), (2 * np.sin(a / 2))[:, None] * n])
                M = mp["m2"][ci]
                for k, X in (("arc", R["arc"]), ("chord_axis", ca), ("tangent", R["tangent"])):
                    E[k] += c["weight"] * 2 * float(np.einsum("ij,ik,jk->", M, X, X))
            blend_axis = (2 * E["chord_axis"] + E["tangent"]) / 3
            rows.append({"n": (1, 2, 3)[level], "A": Aval,
                         "delta_chord_axis": E["chord_axis"] / E["arc"] - 1,
                         "delta_blend_axis": blend_axis / E["arc"] - 1})
            print(json.dumps(rows[-1]), flush=True)
    return rows

if __name__ == "__main__":
    rows = run()
    if len(sys.argv) > 1:
        Path(sys.argv[1]).write_text(json.dumps({"kind": "S4 exploratory addendum (post hoc, not gated)", "rows": rows}, indent=1) + "\n")
