"""Collect the S_C status (R1.8-R2.0 and the outside bound) from the archived logs into one receipt the report pins.
It reads only archived files; it computes nothing. S_C stays a STATED HYPOTHESIS until R2 closes.
usage: python scripts/collect_helike_sc_status.py   -> research/sc-status-20261126/sc_status.json
"""
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
R = ROOT / "research"
OUT = R / "sc-status-20261126" / "sc_status.json"
SOURCES = {
    "c3_old": R / "sc-r1-20261125/r1_2_c3.txt",
    "c3_sym": R / "sc-r1-20261125/r1_2_c3_sym.txt",
    "fold": R / "sc-r1-20261125/r1_2_fold_ladder.txt",
    "base_uniform_0.002": R / "sc-r1-20261125/r1_4_enclosed_g0.002.txt",
    "base_uniform_0.004": R / "sc-r1-20261125/r1_4_enclosed_g0.004.txt",
    "base_folded": R / "sc-r1-20261125/r2_0_folded_base_enclosed.txt",
    "o1_0.002": R / "sc-outside-20261126/o1_g0.002_Rn12.txt",
    "o1_0.004": R / "sc-outside-20261126/o1_g0.004_Rn12.txt",
    "o2_0.002": R / "sc-outside-20261126/o2_g0.002_Rn12.txt",
    "o2_0.004": R / "sc-outside-20261126/o2_g0.004_Rn12.txt",
    "homotopy_0.002": R / "sc-fold-homotopy-20261126/v2_proved_g0.002.json",
    "homotopy_0.004": R / "sc-fold-homotopy-20261126/v2_proved_g0.004.json",
    "tests": R / "sc-r1-20261125/r1_8_tests_and_mutants.txt",
}
FOLDS = {"0.002": (0.36, 0.17), "0.004": (0.45, 0.25)}
RHO_C = {"0.002": 7.31686, "0.004": 7.31395}         # recomputed, relative to V_bg (R1.9.2); frozen value still owed


def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()


def blocks(text):
    """Split a log on '=== g2 = X' headers."""
    out, cur = {}, None
    for line in text.splitlines():
        m = re.match(r"=== g2 = ([0-9.]+)", line)
        if m:
            cur = m.group(1); out[cur] = []
        elif cur:
            out[cur].append(line)
    return {k: "\n".join(v) for k, v in out.items()}


def num(pattern, text):
    m = re.search(pattern, text)
    if not m:
        raise ValueError(f"pattern not found: {pattern}")
    return float(m.group(1))


def base(text):
    return dict(levels_below_rho_b=int(num(r"product levels below rho_b: (\d+)", text)),
                ambiguous=int(num(r"ambiguous at rho_b: (\d+)", text)),
                lambda_11=num(r"lambda_11 = ([0-9.]+)", text), max_radius=num(r"max radius ([0-9.e+-]+)", text))


def main():
    t = {k: p.read_text() for k, p in SOURCES.items() if p.suffix == ".txt"}
    c3o, c3s, bf = blocks(t["c3_old"]), blocks(t["c3_sym"]), blocks(t["base_folded"])
    fold_lines = t["fold"].splitlines()
    controls = []
    for g in ("0.004", "0.002"):
        a, b = FOLDS[g]
        fl = [l for l in fold_lines if f"({a}, {b})" in l and (l.startswith(f"g2={g}") or (g == "0.004" and l.startswith("  FOLD")))]
        hom = json.loads(SOURCES[f"homotopy_{g}"].read_text())[0]["runs"]
        runs = [dict(loss=r["loss"], verdict=r["verdict"].split(" ")[0],
                     trail=[{k: x[k] for k in ("step", "s", "N", "rho", "l11", "l5")} for x in r["trail"]]) for r in hom]
        rho_b = float(re.search(r"rho_b = ([0-9.]+)", hom[0]["verdict"]).group(1))
        controls.append(dict(
            g2=float(g),
            c3_old=num(r"C3 <= ([0-9.]+)", c3o[g]), c3_symmetric=num(r"C3 <= ([0-9.]+)", c3s[g]),
            c3_sampled=num(r"best sampled ([0-9.]+)", c3s[g]), c3_converged="converged" in c3s[g],
            fold=dict(a=a, b=b, verdict="PROVED" if any("PROVED" in l for l in fl) else "MISSING"),
            base_uniform=base(t[f"base_uniform_{g}"]), base_folded=base(bf[g]),
            outside=dict(o1="PROVED" if "O1: PROVED" in t[f"o1_{g}"] else "OPEN",
                         o2="PROVED" if "O2: PROVED" in t[f"o2_{g}"] else "OPEN",
                         o2_cells=int(num(r"O2: PROVED; (\d+) cells", t[f"o2_{g}"]))),
            rho_b=rho_b, rho_C_recomputed=RHO_C[g], homotopy_floating=runs))
    tests = t["tests"]
    status = dict(
        schema="helike/sc-status/v1",
        S_C="STATED_HYPOTHESIS",
        proved_rigorous=["C3 by symmetry reduction", "ray-aware fold D >= 0", "outside of the product tube >= rho_C",
                         "enclosed folded base spectrum"],
        floating_only=["folded Lehmann homotopy (l_5, l_11 at each real step)"],
        remaining=["R2.1 interval A0/A1/A2 with cut Hermite states", "R2.2 verified pencil", "R2.3 chained steps",
                   "frozen rho_C from the Temple receipt (rerun outside bound at it)"],
        tests_passed="OK" in tests and "tests exit 0" in tests, mutants=re.search(r"(\d+/\d+) mutants caught", tests).group(1),
        controls=controls,
        sources={k: dict(path=str(p.relative_to(ROOT)), sha256=sha(p)) for k, p in SOURCES.items()})
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(status, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {OUT.relative_to(ROOT)} sha256 {sha(OUT)}")


if __name__ == "__main__":
    main()
