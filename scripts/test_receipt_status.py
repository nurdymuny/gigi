"""Fault-injection test of scripts/_receipt_status.py (run: python scripts/test_receipt_status.py; exits 1 on failure).

Checks that the helper fails closed:
  - a passing receipt exits 0;
  - a receipt with any pass-type flag false, at any depth, exits 1;
  - a receipt whose status reports FAIL or MISMATCH exits 1;
  - a missing receipt and an unreadable receipt exit 1;
  - a receipt older than the run (a stale PASS left by an earlier run) exits 1;
and, end to end, that a real proof script exits 0 on a good run and that the same receipt, with one flag flipped to
false, is then rejected.
"""
import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
PY = sys.executable


def helper_exit(path, stale_check=False):
    code = ("import sys; sys.path.insert(0, %r); import _receipt_status as R\n" % str(HERE)
            + ("R.begin()\n" if stale_check else "") + "R.exit_on_failure(%r)\n" % str(path))
    return subprocess.run([PY, "-c", code], capture_output=True).returncode


def main():
    results = {}
    with tempfile.TemporaryDirectory() as d:
        d = Path(d)
        cases = {
            "pass": ({"all_proved": True, "sectors": [{"pass": True}]}, 0),
            "nested_flag_false": ({"all_proved": True, "sectors": [{"pass": True}, {"pass": False}]}, 1),
            "top_flag_false": ({"all_exact": False}, 1),
            "status_fail": ({"status": "FAILED at block 3"}, 1),
            "status_mismatch": ({"status": "MISMATCH in row 7"}, 1),
        }
        for name, (obj, want) in cases.items():
            p = d / f"{name}.json"; p.write_text(json.dumps(obj))
            results[name] = helper_exit(p) == want
        results["missing"] = helper_exit(d / "nope.json") == 1
        bad = d / "garbled.json"; bad.write_text("{not json")
        results["unreadable"] = helper_exit(bad) == 1
        stale = d / "stale.json"; stale.write_text(json.dumps({"all_proved": True}))
        old = time.time() - 3600; os.utime(stale, (old, old))
        results["stale_pass"] = helper_exit(stale, stale_check=True) == 1
        fresh = d / "fresh.json"; fresh.write_text(json.dumps({"all_proved": True}))
        results["fresh_pass"] = helper_exit(fresh, stale_check=True) == 0
        # end to end on a real proof script (the j >= 100 Gershgorin tail; seconds)
        out = d / "tail.json"
        rc = subprocess.run([PY, str(HERE / "ricci_su2_floor_large_j.py"), str(out), "50", "20", "2.75", "10"],
                            capture_output=True).returncode
        results["real_script_good_run"] = rc == 0
        obj = json.loads(out.read_text())

        def flip(o):
            if isinstance(o, dict):
                for k, v in o.items():
                    if isinstance(v, bool) and v and k in ("all_proved", "proved", "pass", "ok", "certified"):
                        o[k] = False; return True
                    if flip(v):
                        return True
            if isinstance(o, list):
                return any(flip(v) for v in o)
            return False
        results["real_receipt_has_flag"] = flip(obj)
        out.write_text(json.dumps(obj))
        results["real_receipt_flipped_rejected"] = helper_exit(out) == 1
    for k, v in results.items():
        print(("PASS " if v else "FAIL ") + k)
    sys.exit(0 if all(results.values()) else 1)


if __name__ == "__main__":
    main()
