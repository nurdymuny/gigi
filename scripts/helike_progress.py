"""Progress lines for the viewer's Controls scene (docs/research/helike-view-controls-scene-spec-2026-11-26.md, §5).

One JSON object per stage, appended to OUT_DIR/progress.jsonl. It computes nothing and changes no receipt; a failure
to write is ignored so a runner never stops because of it.
"""
import json
import subprocess
import time
from pathlib import Path

_COMMIT = None


def _commit():
    global _COMMIT
    if _COMMIT is None:
        try:
            _COMMIT = subprocess.run(["git", "rev-parse", "--short", "HEAD"], capture_output=True, text=True,
                                     cwd=Path(__file__).resolve().parent).stdout.strip() or ""
        except Exception:
            _COMMIT = ""
    return _COMMIT


def emit(out_dir, suite, g2, mq, stage):
    """Append one progress event. stage == "done" marks the point finished."""
    try:
        line = {"suite": suite, "g2": g2, "Mq": mq, "stage": stage, "t": time.time(), "commit": _commit()}
        with open(Path(out_dir) / "progress.jsonl", "a", encoding="utf-8") as f:
            f.write(json.dumps(line) + "\n")
    except Exception:
        pass
