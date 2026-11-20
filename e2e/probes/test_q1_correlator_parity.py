"""Q1 gates (docs/HALCYON_QUANTUM_GAP_SPEC.md): the engine's CORRELATOR / EFFECTIVE_MASS against
Halcyon's own M1 functions (davis-wilson-lattice/inertia_damping/su2_4d_glueball.py), on the same
per-configuration time-slice data.

  P1  estimator parity: C(t) and m_eff(t) agree to 1e-6 of their own jackknife error at every t;
      the two errors (C's, and m_eff's in Halcyon's normalisation) agree to relative 1e-8.
      AMENDMENT 1 (before any real-ensemble run): the first form, relative 1e-10 on everything,
      failed on the synthetic dry run at 2.4e-10. Cause: C(t) is a difference of numbers near
      <p>^2 ~ 1 that leaves ~1e-4, so summation-order rounding is amplified ~1e4; Halcyon's
      uncentred sum carries it. The engine now centres first (algebraically identical), and
      parity is measured on the scale that matters, the point's own error bar.
  P2  the plateau rule: the engine's plateau (or refusal) is reported beside Halcyon's auto-window fit.
  N   null control: on Halcyon's shuffled null (its own seed) the engine must REFUSE.
  R   mechanism removal: Halcyon's auto-window on the same null returns a mass; that is the
      behaviour the rule removes.

Also reported, not a gate: the standard jackknife error against Halcyon's normalisation (ratio).

usage: python test_q1_correlator_parity.py HALCYON_ROOT PROBE_EXE P_BAR.npy OUT.json [NULL_SEED]
"""
import json
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np

root, exe, npy, out = Path(sys.argv[1]), sys.argv[2], Path(sys.argv[3]), Path(sys.argv[4])
null_seed = int(sys.argv[5]) if len(sys.argv) > 5 else 20260628 + 1
sys.path.insert(0, str(root))
from inertia_damping import su2_4d_glueball as H   # noqa: E402

p = np.load(npy)
n_cfg, L = p.shape


def engine(arr):
    with tempfile.TemporaryDirectory() as d:
        i, o = Path(d) / "in.json", Path(d) / "out.json"
        i.write_text(json.dumps({"p": arr.tolist()}))
        subprocess.run([exe, str(i), str(o)], check=True)
        return json.loads(o.read_text())


def halcyon(arr):
    C, Ce = H.jackknife_correlator(arr)
    m, me = H.jackknife_effective_mass(arr)
    mass, err, win = H.fit_plateau(m, me)
    return C, Ce, m, me, mass, err, win


def rel(a, b):
    a = np.array([np.nan if x is None else x for x in a], float)
    b = np.asarray(b, float)
    if not np.array_equal(np.isnan(a), np.isnan(b)):
        return float("inf")
    f = ~np.isnan(a)
    if not f.any():
        return 0.0
    return float(np.max(np.abs(a[f] - b[f]) / np.maximum(np.abs(b[f]), 1e-300)))


def in_sigma(a, b, sig):
    a = np.array([np.nan if x is None else x for x in a], float)
    b, sig = np.asarray(b, float), np.asarray(sig, float)
    if not np.array_equal(np.isnan(a), np.isnan(b)):
        return float("inf")
    f = ~np.isnan(a) & (sig > 0)
    return float(np.max(np.abs(a[f] - b[f]) / sig[f])) if f.any() else 0.0


def compare(arr):
    e, (C, Ce, m, me, mass, err, win) = engine(arr), halcyon(arr)
    msig = np.array([np.nan if x is None else x for x in e["m_eff_err_t"]], float)
    par = {"C_t_in_sigma": in_sigma(e["C_t"], C, Ce), "C_err_t_rel": rel(e["C_err_t"], Ce),
           "m_eff_t_in_sigma": in_sigma(e["m_eff_t"], m, np.where(np.isfinite(msig), msig, me)),
           "m_eff_err_t_halcyon_normalisation_rel": rel(e["m_eff_err_t_halcyon_normalisation"], me)}
    std = np.array([np.nan if x is None else x for x in e["m_eff_err_t"]], float)
    ratio = [float(s / h) if np.isfinite(s) and h > 0 else None for s, h in zip(std, me)]
    return {"parity_rel": par, "engine_plateau": e["plateau"], "engine_plateau_as_fixed": e.get("plateau_as_fixed"),
            "halcyon_auto_window": {"window": list(win) if win else None, "mass": mass, "error": err},
            "m_eff_t": e["m_eff_t"], "m_eff_err_t_standard": e["m_eff_err_t"],
            "m_eff_err_t_halcyon": [float(x) for x in me],
            "standard_over_halcyon_error": ratio, "engine_seconds": e["seconds"]}


sig = compare(p)
nul = compare(H.shuffled_null_control(p, seed=null_seed))
tol = {"in_sigma": 1e-6, "rel": 1e-8}
ok = lambda par: all(v <= (tol["in_sigma"] if k.endswith("in_sigma") else tol["rel"]) for k, v in par.items())  # noqa: E731
gates = {
    "P1_parity_signal": ok(sig["parity_rel"]),
    "P1_parity_null": ok(nul["parity_rel"]),
    "N_null_refuses": "refused" in nul["engine_plateau"],
    "R_auto_window_reports_on_null": nul["halcyon_auto_window"]["mass"] is not None,
}
rec = {"kind": "Q1 correlator parity, null and removal", "p_bar": npy.name, "n_cfg": n_cfg, "L": L,
       "null_seed": null_seed, "tolerance": tol, "gates": gates, "signal": sig, "null": nul}
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps(rec, indent=1) + "\n")
print(json.dumps({"gates": gates, "signal_parity": sig["parity_rel"], "signal_plateau": sig["engine_plateau"],
                  "signal_plateau_as_fixed": sig["engine_plateau_as_fixed"],
                  "halcyon_fit": sig["halcyon_auto_window"], "null_plateau": nul["engine_plateau"],
                  "null_halcyon_fit": nul["halcyon_auto_window"]}, indent=1))
sys.exit(0 if all(gates.values()) else 1)
