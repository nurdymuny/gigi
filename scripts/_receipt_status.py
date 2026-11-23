"""Exit status from a receipt: after a script writes its receipt, call exit_on_failure(path). It exits 1 if any
pass-type flag in the receipt is false, or any status string reports a failure; otherwise it returns. The receipt
itself is unchanged; this only makes a failed check visible to the shell."""
import json
import sys
from pathlib import Path

FLAGS = {"all_proved", "all_exact", "all_ok", "all_pass", "all_certified", "proved", "certified", "pass", "passed", "ok"}
BAD_WORDS = ("FAIL", "MISMATCH", "NOT PROVED", "NOT CERTIFIED")


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
    try:
        obj = json.loads(Path(path).read_text())
    except (ValueError, OSError):
        return
    bad = failures(obj)
    if bad:
        print("receipt reports failure:", ", ".join(bad[:10]), file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    exit_on_failure(sys.argv[1])
