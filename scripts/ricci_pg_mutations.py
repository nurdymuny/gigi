"""Mechanism removals for test_ricci_pg_curvature.py: each mutation of
ricci_pg_curvature.py must make its target test FAIL. A test that stays green
with its mechanism removed tests nothing. Writes MUTATIONS.json; refuses to
overwrite.
"""
import importlib
import io
import json
import sys
import types
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = (ROOT / "scripts/ricci_pg_curvature.py").read_text()
OUT = ROOT / "output/ricci-pg-curvature-20261117/MUTATIONS_R3.json"

MUTATIONS = [
    ("drop Piola det(J)", "T2",
     'return np.linalg.det(fit["J"]) * np.linalg.solve(fit["J"], P.T).T',
     'return np.linalg.solve(fit["J"], P.T).T'),
    ("flip exact-curvature sign", "T5",
     "return -2 * np.cross(maurer[..., 1, :], maurer[..., 2, :])",
     "return 2 * np.cross(maurer[..., 1, :], maurer[..., 2, :])"),
    ("wrong frame components in exact curvature", "T5",
     "return -2 * np.cross(maurer[..., 1, :], maurer[..., 2, :])",
     "return -2 * np.cross(maurer[..., 0, :], maurer[..., 1, :])"),
    ("flip sign of closing face edge", "T4",
     '(cell["edges"][e(i, k)], -s[e(i, k)])]',
     '(cell["edges"][e(i, k)], s[e(i, k)])]'),
    # R2: the equality-constraint mutation was inert (the star is exactly
    # determined, see spec amendment); replaced by removals of the real mechanism.
    ("regularized solve breaks exact interpolation", "T9",
     "fmap = np.linalg.pinv(rows, rcond=RCOND)",
     "fmap = np.linalg.solve(rows.T @ rows + 1e-3 * np.eye(11), rows.T)"),
    ("no interpolation refusal", "T9",
     "    if residual > 1e-12 * max(1., float(np.max(abs(d)))):", "    if False:"),
    ("drop edge signs in assembly", "T6",
     "emap = weight * (s[:, None] * (INCIDENCE.T @ W @ tmap))",
     "emap = weight * (INCIDENCE.T @ W @ tmap)"),
    ("inverse metric in two-form pairing", "T6",
     'W = np.einsum("qfa,qab,qkb,q->fk", q["two"], g, trial_basis, w)',
     'W = np.einsum("qfa,qab,qkb,q->fk", q["two"], np.linalg.inv(g), trial_basis, w)'),
    ("no rank refusal", "T7",
     "    if rank < 11:", "    if False:"),
    ("no coincidence refusal", "T8",
     "            if gap > COINCIDE:", "            if False:"),
    ("right multiplication transport", "T8",
     "V = np.array([qmul(h, v) for v in V])",
     "V = np.array([qmul(v, qmul(qconj(V[cn[k0]]), K[ck[k0]])) for v in V])"),
    ("affine offset breaks linearity", "T3",
     "    x = fmap @ d\n", "    x = fmap @ d + 1e-6\n"),
    # R3: S4 variational force. (Dropping the cell weight is NOT used: every
    # weight in this data is 1.0, so that mutation would be inert.)
    ("asymmetric Gram breaks symmetric assembly", "T10",
     "emap = weight * (tmap.T @ G @ tmap)",
     "emap = weight * (tmap.T @ (G + np.triu(G, 1)) @ tmap)"),
    ("Gram without the metric", "T11",
     'G = np.einsum("qka,qab,qlb,q->kl", trial_basis, g, trial_basis, w)',
     'G = np.einsum("qka,qab,qlb,q->kl", trial_basis, np.broadcast_to(np.eye(3), g.shape), trial_basis, w)'),
    ("Whitney trial without edge signs", "T12",
     "tmap = INCIDENCE * s[None, :]", "tmap = INCIDENCE * 1.0"),
    ("term bypassing the reconstruction breaks gauge annihilation", "T13",
     "emap = weight * (tmap.T @ G @ tmap)",
     "emap = weight * (tmap.T @ G @ tmap + 1e-3 * np.eye(len(tcols)))"),
]


def run_with(source):
    mod = types.ModuleType("ricci_pg_curvature")
    mod.__file__ = str(ROOT / "scripts/ricci_pg_curvature.py")
    sys.path.insert(0, str(ROOT / "scripts"))
    exec(compile(source, mod.__file__, "exec"), mod.__dict__)
    sys.modules["ricci_pg_curvature"] = mod
    sys.modules.pop("test_ricci_pg_curvature", None)
    tests = importlib.import_module("test_ricci_pg_curvature")
    result = unittest.TextTestRunner(stream=io.StringIO(), verbosity=0).run(
        unittest.defaultTestLoader.loadTestsFromModule(tests))
    return sorted({t.id().split(".")[-1].split("_")[1] for t, _ in result.failures + result.errors})


def main():
    if OUT.exists():
        raise SystemExit(f"refusing to overwrite {OUT}")
    baseline = run_with(SRC)
    if baseline:
        raise SystemExit(f"baseline must be green; failing: {baseline}")
    report, ok = [], True
    for name, target, old, new in MUTATIONS:
        if SRC.count(old) != 1:
            raise SystemExit(f"mutation '{name}' must match exactly once; matched {SRC.count(old)}")
        failing = run_with(SRC.replace(old, new))
        caught = target in failing
        ok &= caught
        report.append({"mutation": name, "target_test": target, "caught": caught, "failing_tests": failing})
        print(f"  [{'CAUGHT' if caught else 'MISSED'}] {name:52s} target {target}  failing {failing}")
    OUT.write_text(json.dumps({"baseline_failures": baseline, "all_caught": ok,
                               "mutations": report}, indent=2) + "\n")
    print(f"\nall caught: {ok}   wrote {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
