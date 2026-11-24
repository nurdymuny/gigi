"""Exit status from a receipt: after a script writes its receipt, call exit_on_failure(path). It exits 1 if any
pass-type flag in the receipt is false, or any status string reports a failure; otherwise it returns. The receipt
itself is unchanged; this only makes a failed check visible to the shell.

Run lifecycle (optional; used by the Yang-Mills geometry scripts): start_run(path) gives the attempt an identity,
moves any previous receipt aside to <name>.previous.json (kept as history), and writes a RUNNING receipt at once, so a
consumer reading only the file never sees an earlier PASS describing this attempt. publish(path, obj) writes the
completed receipt atomically with the run identity; fail_run(path, exc) writes FAILED. A process killed mid-run leaves
RUNNING, which counts as a failure. exit_on_failure then also requires the receipt to carry this run's identity."""
import json
import os
import sys
import time
from pathlib import Path

_START = None
_RUN = None


def _atomic_write(path, obj):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_name(path.name + f".tmp{os.getpid()}")
    tmp.write_text(json.dumps(obj, indent=1, default=lambda o: o.item() if hasattr(o, "item") else str(o)))
    os.replace(tmp, path)


def start_run(path):
    """Begin an attempt: stale-receipt clock, run identity, previous receipt kept as history, RUNNING written now."""
    global _START, _RUN
    _START = time.time()
    _RUN = f"{time.strftime('%Y%m%dT%H%M%S')}-{os.getpid()}"
    path = Path(path)
    if path.exists():
        os.replace(path, path.with_name(path.stem + ".previous.json"))
    _atomic_write(path, {"status": "RUNNING", "run_id": _RUN, "started": _START})
    return _RUN


def publish(path, obj):
    obj = dict(obj, run_id=_RUN, finished=time.time())
    _atomic_write(path, obj)


def fail_run(path, exc):
    _atomic_write(path, {"status": "FAILED", "run_id": _RUN, "error": f"{type(exc).__name__}: {exc}"[:500]})


def run(path, main):
    """start_run, call main(), and on any exception record FAILED before re-raising."""
    start_run(path)
    try:
        main()
    except BaseException as e:  # noqa: BLE001 - every interruption must leave a FAILED receipt
        fail_run(path, e)
        raise


def begin():
    """Call before the script's main(): a receipt older than this moment counts as stale."""
    global _START
    _START = time.time()

FLAGS = {"all_proved", "all_exact", "all_ok", "all_pass", "all_certified", "proved", "certified", "pass", "passed", "ok"}
BAD_WORDS = ("FAIL", "MISMATCH", "NOT PROVED", "NOT CERTIFIED", "RUNNING", "INCOMPLETE")


def failures(obj, path=""):
    out = []
    if isinstance(obj, dict):
        for k, v in obj.items():
            p = f"{path}.{k}" if path else str(k)
            if k in FLAGS and v is False:
                out.append(p)
            elif k == "status" and isinstance(v, str) and any(w in v.upper() for w in BAD_WORDS):
                out.append(f"{p}={v[:60]}")
            else:
                out += failures(v, p)
    elif isinstance(obj, list):
        for i, v in enumerate(obj):
            out += failures(v, f"{path}[{i}]")
    return out


def exit_on_failure(path):
    """Fail closed: a missing, unreadable, stale or failing receipt exits 1."""
    p = Path(path)
    try:
        obj = json.loads(p.read_text())
    except (ValueError, OSError) as e:
        print(f"receipt missing or unreadable ({p}): {e}", file=sys.stderr)
        sys.exit(1)
    if _START is not None and p.stat().st_mtime < _START - 1.0:
        print(f"receipt is stale: {p} was not written by this run", file=sys.stderr)
        sys.exit(1)
    if _RUN is not None and obj.get("run_id") != _RUN:
        print(f"receipt does not belong to this run ({obj.get('run_id')} != {_RUN})", file=sys.stderr)
        sys.exit(1)
    bad = failures(obj)
    if bad:
        print("receipt reports failure:", ", ".join(bad[:10]), file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    exit_on_failure(sys.argv[1])
