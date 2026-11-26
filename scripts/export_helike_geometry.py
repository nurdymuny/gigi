"""Geometry of the control runs, for the viewer's Geometry scene. Reads committed `helike cone-heat` receipts and
writes, per control point (g2, Mq), what each Ritz level is made of and how each source's heat signal falls. Every
derived number comes from `helike_heat_signal` (the same functions the registered verdicts use); nothing here
decides anything. Floating point; the viewer labels it COMPUTED, not a bound.

usage: python scripts/export_helike_geometry.py RECEIPT_DIR [RECEIPT_DIR ...] OUT.json
"""
import json
import re
import sys
from pathlib import Path

import numpy as np

import helike_heat_signal as hs

ROLE_NAMES = ["h1", "h2", "c", "x", "s1", "s2"]   # registration order of the roles (amendment 1)
TAU_POINTS = 97


def point(rec, path):
    k = len(rec["levels"])
    E0 = rec["levels"][0]
    roles = rec["roles"]
    out = {"g2": rec["g2"], "Mq": rec["Mq"], "s": rec["s"], "t": rec["t"], "receipt": path,
           "levels": [float(e - E0) for e in rec["levels"]],
           "frequencies": rec["frequencies"],
           "roles": {ROLE_NAMES[i]: int(m) for i, m in enumerate(roles)},
           "identified": {"helike": rec["identification"]["helike"]["level_index"],
                          "competitor": rec["identification"]["competitor"]["level_index"]},
           "orders": {}}
    for order in ("coarse", "fine"):
        amp = np.array(rec[order]["reference_amplitudes"]).reshape(7, k)
        out["orders"][order] = {
            "order": rec[order]["order"],
            # per level: squared overlap with the one-quantum state of each role, and with the vacuum
            "mode_weights": [{ROLE_NAMES[i]: float(amp[roles[i], c] ** 2) for i in range(6)} for c in range(k)],
            "vacuum_weight": [float(amp[6, c] ** 2) for c in range(k)],
        }
    sources = {}
    for name, (sector, _) in hs.SOURCES.items():
        w, miss = hs.level_weights(rec, name)
        n, delta = hs.plateau(rec["levels"], w)
        tau = np.linspace(0.0, 6.0 / delta, TAU_POINTS) if np.isfinite(delta) else np.linspace(0.0, 6.0, TAU_POINTS)
        C = hs.signal(rec["levels"], w, tau)
        # effective note m(tau) = -d log C / d tau, by the closed form sum w E e^{-E tau} / sum w e^{-E tau}
        E = np.asarray(rec["levels"])[: len(w)] - rec["levels"][0]
        m = [float(np.sum(w * E * np.exp(-E * t)) / max(np.sum(w * np.exp(-E * t)), 1e-300)) for t in tau]
        tn = hs.tone(rec["levels"], w)
        wc, _ = hs.level_weights(rec, name, order="coarse")
        sources[name] = {"sector": sector, "registered": hs.REGISTERED[sector] == name,
                         "weights": [float(x) for x in w], "weights_coarse": [float(x) for x in wc],
                         "missing": float(miss),
                         "plateau_level": n, "plateau": float(delta),
                         "tau": [float(x) for x in tau], "C": [float(x) for x in C], "m": m,
                         "tone": tn}
    out["sources"] = sources
    return out


def main():
    *dirs, out = sys.argv[1:]
    pts = []
    for d in dirs:
        for f in sorted(Path(d).glob("cone_heat_g*_Mq*.json")):
            if not re.match(r"cone_heat_g[0-9.]+_Mq[0-9]+\.json$", f.name):
                continue
            rec = json.loads(f.read_text())
            pts.append(point(rec, f.as_posix()))
    seen = set()
    uniq = []
    for p in sorted(pts, key=lambda p: (p["g2"], p["Mq"])):
        key = (p["g2"], p["Mq"])
        if key not in seen:
            seen.add(key)
            uniq.append(p)
    doc = {"what": "geometry of the registered control runs (levels, mode composition, heat signals)",
           "source_module": "scripts/helike_heat_signal.py", "role_names": ROLE_NAMES, "points": uniq}
    Path(out).write_text(json.dumps(doc, indent=0) + "\n")
    print(f"{len(uniq)} points -> {out}")


if __name__ == "__main__":
    main()
