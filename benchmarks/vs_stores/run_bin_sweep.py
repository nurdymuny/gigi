"""BIN-COUNT ROBUSTNESS SWEEP — ROUND 4 LOCKED ADDENDUM, clause 3.

The interaction lens buckets time-named numeric fields into 12 equal-width
bins (src/ml/scan.rs:332-337). The fairness audit flagged that 12 happens to
land near the expert arm's 2h partition (24h / 2h = 12). This sweep rebuilds
gigi-stream with the bin constant at 6, 8, 12, 16, 24 and runs the SAME
zero-config /scan anomaly cell (20k labeled subset, labels never sent, the
ONE shared PR-AUC fn) against each binary, strictly serially.

QUALITY CELL, NOT A TIMING CELL: /scan scoring is deterministic for a fixed
binary + dataset, so each bin count is run ONCE (no warmup/reps). Wall time
is not reported.

STABILITY CLAIM (locked before running): every bin count in {8, 12, 16} must
land within 0.05 absolute PR-AUC of the 12-bin value. Outside that window the
report says FRAGILE, honestly. 6 and 24 are reported as context either way.

BUILD HYGIENE (working tree ends byte-clean; only the 12-bin build is the
product):
  * src/ml/scan.rs is snapshotted (raw bytes) before any patch; every patched
    build restores the snapshot and asserts byte-equality afterward.
  * Patches are exact-substring replacements, each asserted to occur exactly
    once, on the three coupled sites of the constant:
        `let w = (mx - mn) / 12.0;`   -> `/ {B}.0;`
        `.clamp(0, 11)`               -> `.clamp(0, {B-1})`
        `"12 equal-width {} bins ..."`-> `"{B} equal-width {} bins ..."`
  * Patched binaries are copied OUT to a scratch dir (local disk, never
    OneDrive, never committed); after the last patched build the pristine
    source is rebuilt so target/release/gigi_stream.exe is the 12-bin binary
    again — that pristine build IS the sweep's 12-bin binary.
  * All builds complete before any timed/served phase (round-4 rule 4).

Per-binary serial run protocol:
  fresh scratch data dir -> spawn binary (PORT=3143, GIGI_DATA_DIR=<fresh>,
  auth/rate env stripped) -> poll /v1/health -> create bench_anom bundle +
  ingest the 20k labeled subset in 1,000-row batches (untimed setup) -> one
  zero-config /scan (the exact run_gigi.gigi_scan function, so the cell is
  literally the round-1/3 cell) -> read the lens `notes` and verify the
  "{B} equal-width hour bins" phrase (proof the patch took) -> PR-AUC via
  eval_common.average_precision -> kill server, verify the port is dead ->
  next binary.

Usage:
  python run_bin_sweep.py                # full sweep: build all 5, run all 5
  python run_bin_sweep.py --smoke 8      # one non-12 value: patch, build,
                                         # run, verify the notes phrase, then
                                         # restore + pristine rebuild. Proves
                                         # the loop without running the sweep.
  python run_bin_sweep.py --build-only   # build all 5 binaries, run nothing
  python run_bin_sweep.py --run-only     # binaries already built: run all 5

Output (full sweep): results_bin_sweep.json in this directory + a printed
table with the stability verdict.
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

from eval_common import (
    GIGI_BASE, GIGI_HOST, GIGI_PORT, N_SUBSET, average_precision, env_info,
    load_labels, read_subset, require_dataset,
)
from run_gigi import ANOM_BUNDLE, Gigi, fresh_bundle, gigi_scan, insert_batches, record_count

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
SCAN_RS = REPO / "src" / "ml" / "scan.rs"
EXE_NAME = "gigi-stream.exe" if os.name == "nt" else "gigi-stream"
BUILT_EXE = REPO / "target" / "release" / EXE_NAME

BIN_VALUES = [6, 8, 12, 16, 24]
STABILITY_SET = [8, 12, 16]      # locked: these must sit within ±0.05 of 12
STABILITY_TOL = 0.05

# Scratch root MUST be local disk (never OneDrive — same rule as the server
# data dir in run_all.md). Binaries + per-run data dirs + server logs live
# here; nothing under it is ever committed.
SCRATCH = Path(
    os.environ.get("VS_STORES_BIN_SWEEP_DIR")
    or Path(os.environ.get("LOCALAPPDATA", Path.home())) / "gigi_bin_sweep"
)

# The three coupled sites of the bin constant in src/ml/scan.rs (asserted
# unique). {B} = bin count, {BM1} = B - 1.
PATCH_SITES = [
    ("let w = (mx - mn) / 12.0;", "let w = (mx - mn) / {B}.0;"),
    (".clamp(0, 11)", ".clamp(0, {BM1})"),
    ('desc: format!("12 equal-width {} bins', 'desc: format!("{B} equal-width {{}} bins'),
]


def scratch_exe(bins: int) -> Path:
    return SCRATCH / f"bins_{bins}" / EXE_NAME


# ── build phase ──────────────────────────────────────────────────────────────
def patch_source(original: bytes, bins: int) -> bytes:
    text = original.decode("utf-8")
    for anchor, template in PATCH_SITES:
        n = text.count(anchor)
        assert n == 1, f"patch anchor not unique in scan.rs ({n} hits): {anchor!r}"
        text = text.replace(anchor, template.format(B=bins, BM1=bins - 1))
    return text.encode("utf-8")


def cargo_build() -> None:
    t0 = time.perf_counter()
    r = subprocess.run(
        ["cargo", "build", "--release", "--bin", "gigi-stream"],
        cwd=REPO, capture_output=True, text=True,
    )
    if r.returncode != 0:
        sys.exit(f"cargo build failed:\n{r.stdout[-2000:]}\n{r.stderr[-4000:]}")
    print(f"    built in {time.perf_counter() - t0:.0f}s")


def build_binary(bins: int, original: bytes) -> Path:
    """Patch scan.rs to `bins`, build, copy the exe out, restore the source.
    The restore runs in a finally block — a failed build cannot leave the
    tree dirty."""
    dest = scratch_exe(bins)
    dest.parent.mkdir(parents=True, exist_ok=True)
    print(f"  [build] bins={bins} -> {dest}")
    try:
        SCAN_RS.write_bytes(patch_source(original, bins))
        cargo_build()
        shutil.copy2(BUILT_EXE, dest)
    finally:
        SCAN_RS.write_bytes(original)
    assert SCAN_RS.read_bytes() == original, "scan.rs restore failed byte-compare"
    return dest


def verify_tree_clean() -> None:
    assert SCAN_RS.read_bytes() == ORIGINAL_SCAN_RS, "scan.rs differs from its pre-run snapshot"
    r = subprocess.run(
        ["git", "status", "--porcelain", "--", "src/ml/scan.rs"],
        cwd=REPO, capture_output=True, text=True,
    )
    status = r.stdout.strip()
    print(f"  [hygiene] scan.rs byte-identical to pre-run snapshot; "
          f"git status of scan.rs: {status or '(clean vs HEAD)'}")


def build_all(bins_list: list[int]) -> dict[int, Path]:
    """All patched builds first, pristine (12-bin) build LAST, so the repo's
    target/release exe ends as the 12-bin product."""
    out: dict[int, Path] = {}
    for b in [b for b in bins_list if b != 12]:
        out[b] = build_binary(b, ORIGINAL_SCAN_RS)
    # pristine rebuild — always, even when 12 is not in bins_list, so the
    # repo's own release binary never lingers on a patched variant
    print("  [build] pristine 12-bin rebuild (the product binary)")
    cargo_build()
    if 12 in bins_list:
        dest = scratch_exe(12)
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(BUILT_EXE, dest)
        out[12] = dest
    verify_tree_clean()
    return out


# ── run phase (strictly serial) ──────────────────────────────────────────────
def wait_health(g: Gigi, timeout_s: float = 120.0) -> dict:
    t0 = time.perf_counter()
    while time.perf_counter() - t0 < timeout_s:
        try:
            return g.ok("GET", "/v1/health")
        except Exception:
            time.sleep(0.25)
    raise RuntimeError(f"server not healthy at {GIGI_BASE} within {timeout_s}s")


def wait_dead(timeout_s: float = 30.0) -> None:
    import socket
    t0 = time.perf_counter()
    while time.perf_counter() - t0 < timeout_s:
        try:
            with socket.create_connection((GIGI_HOST, GIGI_PORT), timeout=1):
                pass
            time.sleep(0.25)
        except OSError:
            return
    raise RuntimeError(f"port {GIGI_PORT} still accepting connections after {timeout_s}s")


def server_env(data_dir: Path) -> dict:
    env = dict(os.environ)
    for k in ("GIGI_API_KEY", "GIGI_JWT_SECRET", "GIGI_RATE_LIMIT",
              "GIGI_PUBLIC_BUNDLES", "GIGI_APP_BUNDLES"):
        env.pop(k, None)
    env["PORT"] = str(GIGI_PORT)
    env["GIGI_DATA_DIR"] = str(data_dir)
    return env


def run_cell(bins: int, exe: Path, subset_records: list[dict], labels: dict) -> dict:
    """One serial quality-cell run: fresh data dir, boot, ingest (untimed),
    one zero-config /scan, notes check, PR-AUC, teardown."""
    print(f"  [run] bins={bins}")
    assert exe.exists(), f"binary missing: {exe} (build phase first)"
    if _port_listening():
        raise RuntimeError(
            f"pre-flight: something is already listening on {GIGI_HOST}:{GIGI_PORT} — "
            "the sweep owns that port; stop the other server first")
    data_dir = SCRATCH / f"data_bins_{bins}"
    if data_dir.exists():
        shutil.rmtree(data_dir)
    data_dir.mkdir(parents=True)
    log_path = SCRATCH / f"server_bins_{bins}.log"
    with open(log_path, "wb") as log:
        proc = subprocess.Popen(
            [str(exe)], cwd=exe.parent, env=server_env(data_dir),
            stdout=log, stderr=subprocess.STDOUT,
        )
        try:
            g = Gigi()
            health = wait_health(g)
            print(f"    health: {json.dumps(health)[:120]}")

            # untimed setup: the exact anomaly-bundle shape from run_gigi.py
            fresh_bundle(g, ANOM_BUNDLE)
            insert_batches(g, ANOM_BUNDLE, subset_records)
            assert record_count(g, ANOM_BUNDLE) == N_SUBSET

            # the cell: ONE zero-config /scan (quality cell — 1 run, no reps)
            status, doc = g.request(
                "POST", f"/v1/bundles/{ANOM_BUNDLE}/scan", {"budget": 0.05, "limit": 0})
            assert status == 200, doc
            notes = doc.get("notes", [])
            phrase = f"{bins} equal-width hour bins"
            bin_notes = [nt for nt in notes if "equal-width hour bins" in nt]
            if not any(phrase in nt for nt in notes):
                raise RuntimeError(
                    f"bins={bins}: expected note containing {phrase!r}; "
                    f"bin-axis notes seen: {bin_notes or notes}")
            print(f"    lens note: {bin_notes[0]}")

            # score via the shared cell function (same fn as round 1/3)
            scores = gigi_scan(g)
            assert len(scores) == N_SUBSET, f"scan returned {len(scores)} scores"
            pr = average_precision(labels, scores)
            print(f"    PR-AUC: {pr:.4f}")
            return {
                "bins": bins,
                "pr_auc": round(pr, 4),
                "n_scored": N_SUBSET,
                "lens_note": bin_notes[0],
                "binary": str(exe),
                "server_log": str(log_path),
                "runs": 1,
            }
        finally:
            proc.terminate()
            try:
                proc.wait(timeout=15)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=15)
            wait_dead()
            print(f"    server down, port {GIGI_PORT} free")


def _port_listening() -> bool:
    import socket
    try:
        with socket.create_connection((GIGI_HOST, GIGI_PORT), timeout=0.5):
            return True
    except OSError:
        return False


# ── reporting ────────────────────────────────────────────────────────────────
def report(cells: list[dict]) -> None:
    by_bins = {c["bins"]: c["pr_auc"] for c in cells}
    ref = by_bins[12]
    print("\nbins   PR-AUC   delta vs 12")
    for b in BIN_VALUES:
        if b in by_bins:
            print(f"{b:>4}   {by_bins[b]:.4f}   {by_bins[b] - ref:+.4f}")
    deltas = {b: abs(by_bins[b] - ref) for b in STABILITY_SET if b in by_bins}
    stable = set(deltas) == set(STABILITY_SET) and all(d <= STABILITY_TOL for d in deltas.values())
    verdict = "STABLE" if stable else "FRAGILE"
    print(f"\nstability ({{8,12,16}} within {STABILITY_TOL} of 12-bin): {verdict}")
    if not stable:
        worst = max(deltas, key=deltas.get) if deltas else None
        print(f"  reported honestly — worst offender: bins={worst}, |delta|={deltas.get(worst, float('nan')):.4f}")
    out = {
        "cell": "bin_count_robustness_sweep (round 4 addendum, clause 3)",
        "cell_kind": "quality (1 run per bin count — /scan is deterministic per binary+dataset; not a timing cell)",
        "environment": env_info(),
        "bin_constant_site": "src/ml/scan.rs interaction-lens time-named bucket axis (12 equal-width bins; three coupled literals patched together)",
        "stability_claim": {
            "set": STABILITY_SET, "tolerance_abs": STABILITY_TOL,
            "reference_bins": 12, "verdict": verdict,
            "deltas_abs": {str(b): round(d, 4) for b, d in sorted(deltas.items())},
        },
        "cells": cells,
        "disclosures": [
            "Only the 12-bin binary is a committed product; 6/8/16/24 builds are scratch binaries from a patched working tree, restored byte-clean after each build (verified against a pre-run snapshot).",
            "Quality cell: one /scan per bin count, no warmup/reps, wall time not reported. Labels never sent to the server; PR-AUC via the one shared eval_common.average_precision.",
            "Runs strictly serial; all builds completed before the first server boot; one server at a time on port 3143, verified dead between runs.",
        ],
    }
    path = HERE / "results_bin_sweep.json"
    with open(path, "w", encoding="utf-8") as f:
        json.dump(out, f, indent=2)
    print(f"\nwrote {path}")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    mode = ap.add_mutually_exclusive_group()
    mode.add_argument("--smoke", type=int, metavar="B",
                      help="patch+build+run ONE non-12 bin count, verify the notes phrase, restore; no sweep")
    mode.add_argument("--build-only", action="store_true")
    mode.add_argument("--run-only", action="store_true")
    args = ap.parse_args()

    require_dataset()
    SCRATCH.mkdir(parents=True, exist_ok=True)
    print(f"scratch root: {SCRATCH} (local disk; never committed)")

    if args.smoke is not None:
        b = args.smoke
        assert b != 12 and b in BIN_VALUES, f"--smoke wants a non-12 value from {BIN_VALUES}"
        exe = build_binary(b, ORIGINAL_SCAN_RS)
        print("  [build] pristine 12-bin rebuild (leave no patched product behind)")
        cargo_build()
        verify_tree_clean()
        subset = read_subset()
        records = [{"txn_id": t, "merchant": m, "customer": c, "hour": h, "amount": a}
                   for t, m, c, h, a in subset]
        cell = run_cell(b, exe, records, load_labels())
        print(f"\nSMOKE OK: bins={b} produced a differently-binned lens "
              f"({cell['lens_note']!r}), PR-AUC {cell['pr_auc']:.4f}. "
              "Full sweep NOT run (by design).")
        return

    if args.run_only:
        exes = {b: scratch_exe(b) for b in BIN_VALUES}
    else:
        exes = build_all(BIN_VALUES)
        if args.build_only:
            print("build-only: binaries staged, nothing run")
            return

    subset = read_subset()
    records = [{"txn_id": t, "merchant": m, "customer": c, "hour": h, "amount": a}
               for t, m, c, h, a in subset]
    labels = load_labels()
    cells = [run_cell(b, exes[b], records, labels) for b in BIN_VALUES]
    report(cells)


ORIGINAL_SCAN_RS = SCAN_RS.read_bytes()

if __name__ == "__main__":
    main()
