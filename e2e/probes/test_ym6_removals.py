"""YM6 = BW10 (docs/YANG_MILLS_ENGINE_SPEC.md): every mechanism removal turns its gate red.

For each removal, the same harness runs the production path (the gate must PASS) and the removal
(the same gate must FAIL), at n = 1 (and n = 2 where stated):

  R1  no register              YM1 coefficient parity vs the research reading (register on): FAIL
                               YM2 force vs the research two-ring force; BW05 vs Table 6 (two-ring): FAIL
  R2  finite-difference defect derivative (step 1e-7, the pre-fix method)
                               YM3 K0 vs the B5 matrix at 1e-9: FAIL
  R3  Cayley endpoint evaluation instead of the midpoint
                               YM4 E vs the S3b receipt at 1e-8: FAIL
  R4  a substituted round metric (1, 1, 1) in place of Berger (3, 1, 1)
                               YM2 BW05 vs Table 6: FAIL (which is why YM5 refuses a missing metric)

Requires GIGI_YM6_TARGET (directory holding the release examples) and GIGI_RICCI_DATA_ROOT.
Optional GIGI_PARITY_RECEIPT (refuses to overwrite).
"""
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
T = Path(os.environ["GIGI_YM6_TARGET"])
os.environ["GIGI_RECOVER_CURVATURE_PROBE"] = str(T / "recover_curvature_probe.exe")
os.environ["GIGI_LINK_FORCE_PROBE"] = str(T / "link_force_probe.exe")
os.environ["GIGI_LINK_FLOW_PROBE"] = str(T / "link_flow_probe.exe")
import test_recover_curvature_su2_parity as RC     # noqa: E402
import test_link_force_parity as LF                # noqa: E402
import test_link_force_jacobian as KJ              # noqa: E402
import test_link_flow_parity as FLW                # noqa: E402
import subprocess                                  # noqa: E402
from scipy.sparse import coo_matrix                # noqa: E402


def force_probe(payload, links, method, metric):
    with tempfile.TemporaryDirectory(prefix="gigi-ym6-") as d:
        inp, out = Path(d) / "in.json", Path(d) / "out.json"
        inp.write_text(json.dumps({"metric": metric, "order": 6, "method": method, "rings": 2,
                                   "links": [[float(x) for x in q] for q in links],
                                   "cells": [{"vertices": c["vertices"], "edges": c["edges"],
                                              "reverse": [bool(r) for r in c["reverse"]],
                                              "weight": float(c["weight"])} for c in payload["cells"]]}))
        subprocess.run([os.environ["GIGI_LINK_FORCE_PROBE"], str(inp), str(out)], check=True, timeout=7200)
        return np.asarray(json.loads(out.read_text())["gradient"], float)


def jacobian_probe(level, trial, fd_step):
    payload = json.loads((KJ.BERGER / f"level{level}_berger_q6.input.json").read_text())
    raw = json.loads((KJ.BERGER / f"level{level}_berger_q6.output.json").read_text())
    body = {"metric": [3.0, 1.0, 1.0], "order": 6, "method": KJ.METHOD[trial], "rings": 2,
            "jacobian_eps": 1e-6, "links": raw["links"],
            "cells": [{"vertices": c["vertices"], "edges": c["edges"], "reverse": [bool(r) for r in c["reverse"]],
                       "weight": float(c["weight"])} for c in payload["cells"]]}
    if fd_step is not None:
        body["fd_defect_step"] = fd_step
    with tempfile.TemporaryDirectory(prefix="gigi-ym6K-") as d:
        inp, out = Path(d) / "in.json", Path(d) / "out.json"
        inp.write_text(json.dumps(body))
        subprocess.run([os.environ["GIGI_LINK_FORCE_PROBE"], str(inp), str(out)], check=True, timeout=36000)
        meta = json.loads(out.read_text())
        r = np.fromfile(out.with_suffix(".rows.bin"), dtype="<u4")
        c = np.fromfile(out.with_suffix(".cols.bin"), dtype="<u4")
        v = np.fromfile(out.with_suffix(".vals.bin"), dtype="<f8")
    return coo_matrix((v, (r, c)), shape=(meta["n"], meta["n"])).tocsc()


def flow_probe(level, trial, evaluation):
    lat = FLW.FL.Lattice(level, trial)
    met_step, met_true = FLW.FL.Metric(), FLW.FL.Metric()
    N, h = FLW.N, FLW.T_END / FLW.N
    frac = 0.5 if evaluation == "midpoint" else 1.0
    step_times = [0.0] + [(k + frac) * h for k in range(N)]
    ref_times = [(k + 1) * h for k in range(N)]
    K = FLW.CAY.load_K(level, trial).tocoo()
    with tempfile.TemporaryDirectory(prefix="gigi-ym6F-") as d:
        d = Path(d)
        K.row.astype("<u4").tofile(d / "r.bin"); K.col.astype("<u4").tofile(d / "c.bin")
        K.data.astype("<f8").tofile(d / "v.bin")
        inp, out = d / "in.json", d / "out.json"
        inp.write_text(json.dumps({
            "order": 6, "method": "native" if trial == "native" else "pg2", "steps": N, "t_end": FLW.T_END,
            "evaluation": evaluation,
            "metric_step": [[t, [float(x) for x in met_step(t)]] for t in step_times],
            "metric_ref": [[t, [float(x) for x in met_true(t)]] for t in ref_times],
            "k0": {"rows": str(d / "r.bin"), "cols": str(d / "c.bin"), "vals": str(d / "v.bin")},
            "links": lat.raw["links"], "edge_points": lat.edge_points,
            "cells": [{"vertices": c["vertices"], "edges": c["edges"], "reverse": [bool(r) for r in c["reverse"]],
                       "weight": float(c["weight"])} for c in lat.cells]}))
        subprocess.run([os.environ["GIGI_LINK_FLOW_PROBE"], str(inp), str(out)], check=True, timeout=36000)
        return json.loads(out.read_text())


class Removals(unittest.TestCase):
    def test_every_removal_turns_its_gate_red(self):
        rec = {"kind": "YM6 / BW10: each removal turns its gate red (production passes in the same run)"}

        # R1 no register: YM1 and YM2
        for level in (0, 1):
            cells, links = RC.load(level)
            py = RC.run_python(cells, links, 2, True)
            on, off = RC.compare(RC.run_engine(cells, links, 2, True), py), RC.compare(RC.run_engine(cells, links, 2, False), py)
            payload = json.loads((LF.F.BERGER / f"level{level}_berger_q6.input.json").read_text())
            raw = json.loads((LF.F.BERGER / f"level{level}_berger_q6.output.json").read_text())
            gp = force_probe(payload, links, "pg2", LF.METRIC)
            gn = force_probe(payload, links, "pg2_noregister", LF.METRIC)
            bw_on = LF.measures(payload, LF.METRIC, dict(raw, gradient=gp.tolist()))["relative_error"] * 100
            bw_off = LF.measures(payload, LF.METRIC, dict(raw, gradient=gn.tolist()))["relative_error"] * 100
            want = LF.TABLE6["pg2"][level]
            rec[f"R1_no_register_level{level}"] = {
                "YM1_production_coef_rel": on["coef_rel"], "YM1_removal_coef_rel": off["coef_rel"],
                "YM2_production_bw05": bw_on, "YM2_removal_bw05": bw_off, "table6_two_ring": want}
            self.assertLess(on["coef_rel"], 1e-12)
            self.assertGreater(off["coef_rel"], 1e-3)
            self.assertEqual(round(bw_on, 2), want)
            self.assertNotEqual(round(bw_off, 2), want)

        # R2 finite-difference defect derivative: YM3
        Kr, _ = KJ.reference(0, "pg")
        for name, step in (("production", None), ("removal", 1e-7)):
            Ke = jacobian_probe(0, "pg", step)
            dd = Ke - Kr
            rel = float(np.sqrt(dd.multiply(dd).sum()) / np.sqrt(Kr.multiply(Kr).sum()))
            rec.setdefault("R2_fd_defect_derivative_level0", {})[f"YM3_{name}_K_rel"] = rel
            if step is None:
                self.assertLess(rel, KJ.TOL)
            else:
                self.assertGreater(rel, KJ.TOL)

        # R3 endpoint evaluation: YM4
        ref = json.loads((FLW.RECEIPTS / "L0_pg_N8.json").read_text())
        for ev in ("midpoint", "endpoint"):
            eng = flow_probe(0, "pg", ev)
            e_rel = max(abs(a["E"] - b["E"]) / abs(b["E"]) for a, b in zip(eng["rows"], ref["rows"]))
            rec.setdefault("R3_endpoint_level0", {})[f"YM4_{ev}_E_rel"] = e_rel
            rec["R3_endpoint_level0"][f"{ev}_R_final"] = eng["rows"][-1]["R"]
            if ev == "midpoint":
                self.assertLess(e_rel, 1e-8)
            else:
                self.assertGreater(e_rel, 1e-8)

        # R4 substituted round metric: YM2 BW05 vs Table 6
        payload = json.loads((LF.F.BERGER / "level0_berger_q6.input.json").read_text())
        raw = json.loads((LF.F.BERGER / "level0_berger_q6.output.json").read_text())
        links = np.asarray(raw["links"], float)
        g_round = force_probe(payload, links, "pg2", [1.0, 1.0, 1.0])
        bw_round = LF.measures(payload, LF.METRIC, dict(raw, gradient=g_round.tolist()))["relative_error"] * 100
        rec["R4_round_metric_level0"] = {"YM2_removal_bw05": bw_round, "table6_two_ring": LF.TABLE6["pg2"][0]}
        self.assertNotEqual(round(bw_round, 2), LF.TABLE6["pg2"][0])

        print(json.dumps(rec, indent=1), flush=True)
        out = os.environ.get("GIGI_PARITY_RECEIPT")
        if out:
            p = Path(out)
            if p.exists():
                raise SystemExit(f"refusing to overwrite {p}")
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(json.dumps(rec, indent=2) + "\n")


if __name__ == "__main__":
    unittest.main()
