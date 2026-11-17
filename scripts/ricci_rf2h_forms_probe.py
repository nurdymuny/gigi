"""RF2H at level 0: both energy forms for one Adj field. Read-only diagnostic.

Mirrors e2e/probes/ricci_rf2h_experiment.measure (frozen H, M, D, a; artifact
hashes checked; same norm) but evaluates BOTH the linearised and the quadratic
energy, and each operator's spectrum against M (stability). Depends on the
RF2H code from commit 99882091 (branch claude/rf2h-adjoint-consistent-recovery).

usage: python scripts/ricci_rf2h_forms_probe.py whitney|bdm
Writes output/ricci-pg-curvature-20261117/rf2h_forms_<field>.json; refuses to overwrite.
"""
import hashlib
import json
import sys
import time
from pathlib import Path

import numpy as np
from scipy.linalg import eigh
from scipy.sparse import load_npz
from scipy.sparse.linalg import splu

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "e2e/probes"))
import ricci_rf2h as rf                                    # noqa: E402
from ricci_rf2h_experiment import sha, VAR, SHAPE         # noqa: E402

OUT = ROOT / "output/ricci-pg-curvature-20261117"


def main(field):
    path = OUT / f"rf2h_forms_{field}.json"
    if path.exists():
        raise SystemExit(f"refusing to overwrite {path}")
    key = "level0_q6"
    prior = json.loads((VAR / "diagnostic.json").read_text())["cases"][key]
    for name, digest in prior["artifacts"].items():
        if sha(VAR / name) != digest:
            raise SystemExit("frozen artifact drift: " + name)
    H = load_npz(VAR / f"{key}.h.npz"); M = load_npz(VAR / f"{key}.m.npz").tocsc()
    D = load_npz(VAR / f"{key}.d.npz")
    with np.load(VAR / f"{key}.arrays.npz", allow_pickle=False) as s:
        a = s["a"][:, 0]
    t = time.time()
    b = rf.build_recovery(json.loads((SHAPE / "level0_berger_q6.input.json").read_text()),
                          order=6, verbose=False, adj_field=field)
    r = b["receipt"]
    lu = splu(M); vol = prior["volume"]
    norm = lambda x: float(np.sqrt(max(0., x @ (M @ x)) / vol))   # noqa: E731
    out = {"kind": "read-only diagnostic of RF2H level 0", "adj_field": field,
           "acc_relative_residual": r["acc_relative_residual"],
           "adj_relative_residual": r["adj_relative_residual"],
           "phi_norm": r["phi_norm"], "build_seconds": round(time.time() - t, 1), "forms": {}}
    for form in ("linear", "quad"):
        g = rf.gradient(b["C"], H, D, a, form=form)
        v = lu.solve(-g); v = v + lu.solve(-g - M @ v)
        if form == "linear":
            K = D.T @ (rf.linear_operator(b["C"], H) @ D)
        else:
            K = D.T @ (b["C"].T @ (H @ (b["C"] @ D)))
        K = K.toarray()
        lam = eigh((K + K.T) / 2, M.toarray(), eigvals_only=True)
        phys = lam[abs(lam) >= 1]
        out["forms"][form] = {"relative_error": norm(v + 4 * a) / norm(4 * a),
                              "smallest_physical_eigenvalue": float(phys.min()),
                              "negative_physical_eigenvalues": int((phys < 0).sum()),
                              "physical_modes": int(len(phys))}
        print(form, out["forms"][form])
    out["source_sha256"] = {"scripts/ricci_rf2h_forms_probe.py": sha(__file__),
                            "e2e/probes/ricci_rf2h.py": sha(rf.__file__)}
    path.write_text(json.dumps(out, indent=2) + "\n")
    print("wrote", path.relative_to(ROOT))


if __name__ == "__main__":
    main(sys.argv[1])
