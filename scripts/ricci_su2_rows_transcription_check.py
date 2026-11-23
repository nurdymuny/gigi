"""Transcription check (T1): the sympy expressions that scripts/ricci_su2_exact_edge.py symbolic_rows feeds to lambdify
(w_a, C, Omega_+, Omega_-) are EXACTLY the rows proved from the operator in
research/edge-theorem-20261122-proofs/weitzenbock_family.json (exact sympy, no numerics: lambdify is replaced by the
identity so the symbolic expressions themselves are compared).
Covers the closed-form rows used by Blocks (edges, P2, tests A-E). Also compares the interval certificate's own symbolic rows (ricci_su2_family_block_cert.symbolic: w, C, L+- = 2i Omega_+-,
t = s^2). Also the rebuilt rows of edge_wp1_block_polynomials.ingredients, edge_wp3_slice_iff.symbolic_data, and the edge
polynomial's block B_{4,0} (ricci_su2_edge_polynomial.symbolic_block). It does not cover the near-end matrices of
edge_wp2_interval_proof.py.
usage: OMP_NUM_THREADS=1 python scripts/ricci_su2_rows_transcription_check.py OUT.json
"""
import json
import sys
from pathlib import Path

import sympy as sp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_exact_edge as EE          # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
EE.sp.lambdify = lambda args, expr, *a_, **k_: (lambda *a: expr)
ch, f = EE.symbolic_rows()
w, C, Op, Om = f(0, 0, 0)
R = json.loads((ROOT / "research/edge-theorem-20261122-proofs/weitzenbock_family.json").read_text())["exact_rows"]
x, y = sp.symbols("x y", real=True); t = sp.symbols("t", positive=True); loc = {"x": x, "y": y, "t": t}


def same(A, B):
    bad = []
    for k, (a, b) in enumerate(zip(A, B)):
        a = sp.sympify(a); a = a.subs({s: loc[s.name] for s in a.free_symbols})
        if sp.simplify(a - sp.sympify(b, locals=loc)) != 0:
            bad.append(k)
    return bad


out = {"w": same(list(w), R["w"]), "C": same(list(C), [e for r in R["C"] for e in r]),
       "Omega_plus": same(list(Op), [e for r in R["Omega_plus"] for e in r]),
       "Omega_minus": same(list(Om), [e for r in R["Omega_minus"] for e in r])}
# T2: the interval certificate's own symbolic rows (ricci_su2_family_block_cert.symbolic, t = s^2, L+- = 2i Omega_+-)
import ricci_su2_family_block_cert as BC   # noqa: E402
S_ = BC.symbolic()
sv = sp.symbols("sv", positive=True)
toS = lambda e: sp.sympify(e, locals=loc).subs(t, sv ** 2)          # noqa: E731
def same2(A, B):
    bad = []
    for k, (a, b) in enumerate(zip(A, B)):
        a = a(0, 0, 0) if callable(a) else a
        a = sp.sympify(a); a = a.subs({s_: ({"x": x, "y": y, "sv": sv}[s_.name]) for s_ in a.free_symbols if s_.name in ("x", "y", "sv")})
        if sp.simplify(a - b) != 0:
            bad.append(k)
    return bad
out["block_cert_w"] = same2(S_["w"], [toS(e) for e in R["w"]])
out["block_cert_C"] = same2([e for r in S_["C"] for e in r], [toS(e) for r in R["C"] for e in r])
out["block_cert_Lp"] = same2([e for r in S_["Lp"] for e in r], [2 * sp.I * toS(e) for r in R["Omega_plus"] for e in r])
out["block_cert_Lm"] = same2([e for r in S_["Lm"] for e in r], [2 * sp.I * toS(e) for r in R["Omega_minus"] for e in r])
# T3: the other scripts that rebuild the rows themselves
_argv, sys.argv = sys.argv, [sys.argv[0]]
import edge_wp1_block_polynomials as WP1   # noqa: E402
import edge_wp3_slice_iff as WP3           # noqa: E402
import ricci_su2_edge_polynomial as EP     # noqa: E402
sys.argv = _argv
Rw = [sp.sympify(e, locals=loc) for e in R["w"]]
RC = sp.Matrix([[sp.sympify(e, locals=loc) for e in r] for r in R["C"]])
RP = sp.Matrix([[sp.sympify(e, locals=loc) for e in r] for r in R["Omega_plus"]])
RM = sp.Matrix([[sp.sympify(e, locals=loc) for e in r] for r in R["Omega_minus"]])
ING = WP1.ingredients()
out["wp1_w"] = same(list(ING["w"]), R["w"]); out["wp1_C"] = same(list(ING["C"]), [e for r in R["C"] for e in r])
out["wp1_Op"] = same(list(ING["Op"]), [e for r in R["Omega_plus"] for e in r]); out["wp1_Omn"] = same(list(ING["Omn"]), [e for r in R["Omega_minus"] for e in r])
d3 = WP3.symbolic_data()
out["wp3_w"] = same(list(d3[1]), R["w"]); out["wp3_C"] = same(list(d3[2]), [e for r in R["C"] for e in r])
out["wp3_Op"] = same(list(d3[3]), [e for r in R["Omega_plus"] for e in r]); out["wp3_Omn"] = same(list(d3[4]), [e for r in R["Omega_minus"] for e in r])
# the edge polynomial's block B_{4,0} - lambda_4, rebuilt from the proved rows with the same assembly
M_code, n = EP.symbolic_block()
s_ = sp.symbols("s", positive=True); j, q = 4, 0; l = sp.Rational(j, 2)
chs = [(cf, cl) for cf in (-1, 0, 1) for cl in (-1, 0, 1)]
states = [(q - a - b, k) for k, (a, b) in enumerate(chs) if abs(q - a - b) <= l]
idx = {st: k for k, st in enumerate(states)}; B = sp.zeros(len(states), len(states))
for (mm, a), k in idx.items():
    B[k, k] += (Rw[a] - 2 * mm / sp.sqrt(t)) ** 2 + 4 * (l * (l + 1) - mm ** 2)
    for (m2, a2), k2 in idx.items():
        if m2 == mm:
            B[k2, k] += RC[a2, a]
        if m2 == mm + 1:
            B[k2, k] += 2 * sp.I * sp.sqrt((l - mm) * (l + mm + 1)) * RM[a2, a]
        if m2 == mm - 1:
            B[k2, k] += 2 * sp.I * sp.sqrt((l + mm) * (l - mm + 1)) * RP[a2, a]
lam = (x + (j + 2) / sp.sqrt(t)) ** 2 + 2 * sp.sqrt(t) * x - y ** 2
M_ref = ((B - lam * sp.eye(len(states))).subs(t, s_ ** 2) * s_ ** 2)
Mc = M_code.subs({sym: {"x": x, "y": y, "s": s_}[sym.name] for sym in M_code.free_symbols if sym.name in ("x", "y", "s")})
out["edge_polynomial_block_B40"] = [k for k in range(n * n) if sp.simplify(sp.expand(Mc[k] - M_ref[k])) != 0]
out["all_exact"] = all(not v for k, v in out.items())
out["status"] = "PROVED (exact sympy equality of the code's symbolic rows with the operator-derived rows)" if out["all_exact"] else "MISMATCH"
Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")
print(out)

import _receipt_status  # noqa: E402
_receipt_status.exit_on_failure(sys.argv[1])
