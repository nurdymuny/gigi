"""Per-point domain-validity gate of the flux-grid method amendment (§3.3).

For one grid point (s, t, g², Mq), with every step either exact or a rigorous enclosure:
1. Rust `cone-export`: solve with exact moments and export the four trial states (ground, Helike pair with its
   chiral mixing, competitor) as bit patterns, chart included.
2. Write those states in the `helike-lab/quasimode-states/v1` schema (the Python potential and metric expansions
   added for the interval code's coefficient checks; no floating reference moments).
3. Arb `helike_cutoff_bounds.bound_state` on the binary64 product C = coeff @ mixing: tail t and leakage l.
4. Rust `cone-sectors` on the same frozen product: r_lower, r_upper, c, q, R.
5. The §5.7 cut-energy gate (`helike_leakage_gate.classify_state_cut`): e_cut = l + 2 t r <= 3e-4 from the upper
   bounds, plus the unchanged tail and normalisation budgets, for helike_plus, helike_minus and competitor. PASS only
   if all three pass. The old relative ratio l / r_lower is reported only (relative_ratio_reported).
"""
from fractions import Fraction
import hashlib
import json
import math
from pathlib import Path
import subprocess
import sys
import tempfile

import numpy as np

import helike_cutoff_bounds as cb
import helike_leakage_gate as lg
import helike_positive_cone as pc

ROOT = Path(__file__).resolve().parents[1]
HELIKE_BIN = ROOT / ("helike-lab/target/release/helike" + (".exe" if sys.platform == "win32" else ""))
NAMES = ("ground", "helike_plus", "helike_minus", "competitor")
CHARGES = (0, 2, -2, 0)
GATED = ("helike_plus", "helike_minus", "competitor")


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def text_sha(path):
    return hashlib.sha256(Path(path).read_bytes().replace(b"\r\n", b"\n").replace(b"\r", b"\n")).hexdigest()


def f64(bits_list, shape=None):
    a = np.array(bits_list, dtype=np.uint64).view(np.float64)
    return a.reshape(shape) if shape else a


def bits(x):
    return np.ascontiguousarray(np.asarray(x, dtype="<f8")).view("<u8").ravel().tolist()


def upward(q):
    """The least binary64 >= q (q a nonnegative exact rational)."""
    if q < 0:
        raise ValueError("nonnegative bound expected")
    x = float(q)
    if Fraction(x) < q:
        x = math.nextafter(x, math.inf)
    return x


def run_helike(args, receipt):
    subprocess.run([str(HELIKE_BIN), "--out", str(receipt), *args], check=True, capture_output=True, text=True)
    return json.loads(Path(receipt).read_text())


def load_export(path):
    e = json.loads(Path(path).read_text())
    if e.get("schema") != "helike-lab/cone-state-export/v1" or e.get("moments") != "exact":
        raise ValueError("not an exact-moment cone-state export")
    n = e["coeff_shape"][0]
    return dict(raw=e, u0=f64(e["u0"]), S=f64(e["S"], (6, 6)), w=f64(e["w"]), G0=f64(e["G0"], (3, 3)),
                basis=[tuple(b) for b in e["basis"]], coeff=f64(e["coeff"], (n, 4)),
                mixing=f64(e["mixing_re"], (4, 4)) + 1j * f64(e["mixing_im"], (4, 4)), centers=f64(e["centers"]))


def states_document(export_path, x):
    e = x["raw"]
    s, t, g2 = e["s"], e["t"], e["g2"]
    if math.sqrt(g2) != e["g"]:
        raise ValueError("coupling g disagrees with sqrt(g2)")
    metric, V = pc.laurent_model(s, t, math.sqrt(g2))
    control = dict(
        g2=g2, Mq=e["Mq"], s=s, t=t, state_file=Path(export_path).name, state_sha256=sha(export_path),
        s_bits=bits(s)[0], t_bits=bits(t)[0], g2_bits=bits(g2)[0], g_bits=bits(math.sqrt(g2))[0],
        u0=e["u0"], S=e["S"], saved_S_equals_chart_S=True, w=e["w"], basis=e["basis"],
        coeff=e["coeff"], coeff_shape=e["coeff_shape"], mixing_re=e["mixing_re"], mixing_im=e["mixing_im"],
        centers=e["centers"], names=list(NAMES),
        python_potential=[dict(key=list(map(int, k)), c=bits(float(c))[0]) for k, c in V.items()],
        python_metric=[dict(i=i, j=j, key=list(map(int, k)), c=float(c))
                       for i in range(6) for j in range(6) for k, c in metric[i][j].items()],
        reference_centered_moments=None)
    return dict(schema="helike-lab/quasimode-states/v1", parent_receipt_sha256=sha(export_path),
                residual_receipt_sha256="", controls=[control])


def arb_bounds(x):
    cb.ctx.prec = cb.PRECISION          # the registered 128-bit ball arithmetic
    C = x["coeff"] @ x["mixing"]
    p = cb.plateau(x["u0"], x["S"], x["w"], x["G0"])
    return C, {name: cb.bound_state(x["u0"], x["S"], x["w"], x["G0"], x["basis"], C[:, j], p) for j, name in enumerate(NAMES)}


def inputs_document(states_path, export_path, x, C, arb, arb_path):
    states = []
    for name, m in zip(NAMES, CHARGES):
        a = arb[name]
        t, l = lg.dyadic(a["tail_norm"]["upper"]), lg.dyadic(a["leakage_norm"]["upper"])
        states.append(dict(name=name, charge=m, tail_norm_upper=upward(t), leakage_norm_upper=upward(l),
                           tail_norm_upper_dyadic=a["tail_norm"]["upper"], leakage_norm_upper_dyadic=a["leakage_norm"]["upper"],
                           reference_charge_sq={}))
    e = x["raw"]
    control = dict(g2=e["g2"], Mq=e["Mq"], trial_file=Path(export_path).name, trial_sha256=sha(export_path),
                   trial_coeff_re=bits(C.real), trial_coeff_im=bits(C.imag), states=states)
    return dict(schema="helike-lab/projection-inputs/v1", states_file_sha256=text_sha(states_path),
                cutoff_receipt_sha256=text_sha(arb_path), cutoff_source_revision="domain-gate",
                construction_receipt_sha256=sha(export_path), controls=[control])


def verdict(rows):
    """§5.7: PASS only if every gated state passes; FAIL if any fails with its enclosures resolved; otherwise OPEN."""
    return lg.overall_cut([rows[n] for n in GATED])


# §5.10: the gate checks exactly the states the grid identified, under the grid's own rule (w~ >= 0.9). The
# quasimode certificate's 0.95 selection stays in the certificate path.
SELECTION = "grid"


def export_args(s, t, g2, mq, export_path, chart=None):
    args = ["cone-export", "--s", str(s), "--t", str(t), "--g2", str(g2), "--mq", str(mq),
            "--states-out", str(export_path), "--selection", SELECTION]
    if chart is not None:
        args += ["--chart", str(chart)]
    return args


def domain_gate_point(s, t, g2, mq, workdir=None, chart=None):
    with tempfile.TemporaryDirectory() as tmp:
        d = Path(workdir) if workdir else Path(tmp)
        d.mkdir(parents=True, exist_ok=True)
        export_path = d / "export.json"
        exp = run_helike(export_args(s, t, g2, mq, export_path, chart), d / "export-receipt.json")
        x = load_export(export_path)
        states_path = d / "states.json"
        states_path.write_text(json.dumps(states_document(export_path, x)) + "\n")
        C, arb = arb_bounds(x)
        arb_path = d / "arb.json"
        arb_path.write_text(json.dumps(arb) + "\n")
        inputs_path = d / "inputs.json"
        inputs_path.write_text(json.dumps(inputs_document(states_path, export_path, x, C, arb, arb_path)) + "\n")
        sec_path = d / "sectors.json"
        sec = run_helike(["cone-sectors", "--states", str(states_path), "--inputs", str(inputs_path)], sec_path)
        if sec["status"] != "INTERVAL_ENCLOSURE":
            raise ValueError("sector bounds are not an interval enclosure")
        rust = {st["name"]: st for st in sec["results"]["controls"][0]["states"]}
        rows = {}
        for name in NAMES:
            a = arb[name]
            rl = lg.RigorousLower(rust[name]["r_lower"], source="cone-sectors", sha256=sha(sec_path), field="r_lower",
                                  status=sec["status"])
            ru = lg.RigorousUpper(rust[name]["r_upper"], source="cone-sectors", sha256=sha(sec_path), field="r_upper",
                                  status=sec["status"])
            r = lg.classify_state_cut(tail_sq_upper=lg.dyadic(a["tail_sq"]["upper"]), norm_sq_lower=lg.dyadic(a["norm_sq"]["lower"]),
                                      norm_sq_upper=lg.dyadic(a["norm_sq"]["upper"]),
                                      leakage_upper=lg.dyadic(a["leakage_norm"]["upper"]),
                                      tail_upper=lg.dyadic(a["tail_norm"]["upper"]), r_upper=ru, r_lower=rl)
            sb = rust[name]["sector"].get("Ok")
            r.update(r_lower=rust[name]["r_lower"], r_upper=rust[name]["r_upper"], charge_upper=rust[name]["charge_upper"],
                     leakage_norm_upper=float(lg.dyadic(a["leakage_norm"]["upper"])), q_lower=sb and sb["q_lower"],
                     projected_residual_upper=sb and sb["residual_upper"])
            rows[name] = r
        return dict(s=s, t=t, g2=g2, Mq=mq, verdict=verdict(rows), states=rows,
                    compressed_charges=exp["results"]["compressed_charges"], pair_weights=exp["results"]["pair_weights"],
                    export_sha256=sha(export_path), sectors_sha256=sha(sec_path))
