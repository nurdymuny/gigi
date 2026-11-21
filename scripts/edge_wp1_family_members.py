"""WP1: which competitor polynomials are members of the near-top families, and q <-> -q mirror identity
(21 November 2026).

Reads research/edge-theorem-20261121/wp1_top_family.json (D_{q=l+1}(x, y, s, l), D_{q=l}(x, y, s, l)) and
wp1_block_polynomials.json. Exact polynomial division: is D_k(l = j/2) = const * s * (main factor of block (j, q))?
And: are the polynomials of (j, q) and (j, -q) identical (blocks derived with both signs)? Exact (PROVED as algebra).
usage: python scripts/edge_wp1_family_members.py OUT.json
"""
import json
import sys
from pathlib import Path

import sympy as sp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import edge_wp1_block_polynomials as W        # noqa: E402
import edge_wp1_top_family as TF              # noqa: E402

D = Path(__file__).resolve().parent.parent / "research/edge-theorem-20261121"
loc = {"x": W.x, "y": W.y, "s": W.s, "l": TF.L}


def main():
    fam = json.loads((D / "wp1_top_family.json").read_text())["families"]
    pb = json.loads((D / "wp1_block_polynomials.json").read_text())["blocks"]
    Dk = {}
    for name, rec in fam.items():
        expr = sp.Integer(1)
        for f in rec["factors"]:
            expr *= sp.sympify(f["expr"], locals=loc) ** f["mult"]
        Dk[name] = expr
    out = {"membership": [], "mirror": []}
    for key, rec in pb.items():
        j = int(key.split(",")[0][2:]); q = float(key.split(",")[1][2:])
        main_ = max(rec["factors"], key=lambda f: f["total_degree"])
        F = sp.Poly(sp.sympify(main_["expr"], locals=loc), W.x, W.y, W.s)
        for name, k in (("q=l+1", 0), ("q=l", 1)):
            if abs(abs(q) - (j / 2 + 1 - k)) > 1e-9:
                continue
            Dl = sp.Poly(sp.expand(Dk[name].subs(TF.L, sp.Rational(j, 2))), W.x, W.y, W.s)
            if Dl.is_zero:
                out["membership"].append({"block": key, "family": name, "note": "D vanishes identically"}); continue
            quo, rem = sp.div(Dl, F)
            out["membership"].append({"block": key, "family": name, "quotient": str(quo.as_expr()), "remainder_zero": rem.is_zero})
            print(out["membership"][-1], flush=True)
    keys = list(pb)
    for key in keys:
        j = key.split(",")[0]; q = float(key.split(",")[1][2:])
        mk = f"{j},q={-q:g}"
        if q > 0 and mk in pb:
            a = sp.expand(sp.Mul(*[sp.sympify(f["expr"], locals=loc) for f in pb[key]["factors"]]))
            b = sp.expand(sp.Mul(*[sp.sympify(f["expr"], locals=loc) for f in pb[mk]["factors"]]))
            same = sp.expand(a - b) == 0 and pb[key]["content"] == pb[mk]["content"]
            out["mirror"].append({"blocks": [key, mk], "identical_polynomials": same}); print(out["mirror"][-1])
    Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
