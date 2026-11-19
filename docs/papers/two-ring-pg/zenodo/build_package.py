"""Assemble the Zenodo upload for Reading the Curve into zenodo/upload/ (gitignored).

usage (from anywhere):  python docs/papers/two-ring-pg/zenodo/build_package.py

Produces exactly the files to upload:
  paper.pdf                                  the paper
  reading-the-curve-v2-latex.zip             paper.tex, figures, README (compiles with pdflatex)
  reading-the-curve-v2-code-and-data.zip     every script behind the paper's numbers (with their
                                             imports), the Rust verb, receipts, frozen inputs,
                                             specifications and results, licence; repo paths kept
  MANIFEST.txt                               SHA-256 and size of each file above
"""
import ast
import hashlib
import os
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
PAPER = HERE.parent
REPO = PAPER.parents[2]
OUT = HERE / "upload"
TAG = "reading-the-curve-v2"

ROOT_SCRIPTS = [
    "scripts/ricci_pg_tworing.py", "scripts/test_ricci_pg_tworing.py", "scripts/ricci_pg_tworing_gate.py",
    "scripts/ricci_su2_recovery.py", "scripts/test_ricci_su2_recovery.py", "scripts/ricci_su2_g2_probe.py",
    "scripts/ricci_su2_pg_force.py", "scripts/ricci_su2_pg_stability.py", "scripts/ricci_su2_pg_numrange.py",
    "scripts/ricci_su2_pg_flow.py", "scripts/ricci_su2_pg_flow_filter.py", "scripts/ricci_su2_pg_flow_cayley.py",
    "scripts/ricci_su2_batched.py", "scripts/test_ricci_su2_exact_defect_derivative.py",
]
EXTRA_FILES = ["docs/papers/dgstyle.tex", "LICENSE", "LICENSE.md", "LICENSE.txt"]
DIRS = [
    "research/su2-pg-force-20261117", "research/su2-pg-stability-20261117", "research/su2-numrange-20261118",
    "research/su2-flow-20261118", "research/su2-filter-20261118", "research/su2-cayley-20261118",
    "research/su2-batched-20261119", "research/recover-curvature-parity-20261117",
    "research/recover-curvature-su2-20261117",
    "output/ricci-shape-regular-20261117", "output/ricci-consistency-20261117",
    "output/ricci-variational-20261117", "output/ricci-shared-face-20261117",
    "output/ricci-shared-curvature-20261117", "output/ricci-pg-curvature-20261117",
    "output/ricci-exterior-conformity-20261117", "output/ricci-local-push-20261117",
]
DOC_GLOBS = ["docs/RICCI_SU2_*.md", "docs/RECOVER_CURVATURE*.md", "docs/RICCI_PG_*.md",
             "docs/RICCI_SHAPE_REGULAR*.md", "docs/RING_FOLD*.md", "docs/RICCI_PROJECTION_LAYER*.md"]
PAPER_EXCLUDE = {"zenodo", "social", "__pycache__"}
PAPER_EXCLUDE_SUFFIX = {".aux", ".log", ".out", ".bbl", ".blg", ".toc", ".pyc"}


def import_closure():
    """Every repo module the root scripts import, transitively (only modules that live in the
    repo's script folders; installed packages such as numpy are not part of the archive)."""
    search = [REPO / "scripts", REPO / "e2e" / "probes", PAPER]
    roots = [REPO / r for r in ROOT_SCRIPTS] + sorted(PAPER.glob("*.py"))
    roots += [p for p in (REPO / "research").rglob("*.py")
              if any(str(p.relative_to(REPO)).replace(os.sep, "/").startswith(d) for d in DIRS)]
    found, todo = set(), list(roots)
    while todo:
        p = todo.pop()
        rel = p.resolve().relative_to(REPO)
        if rel in found:
            continue
        found.add(rel)
        tree = ast.parse(p.read_text(encoding="utf-8"))
        names = set()
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                names.update(a.name.split(".")[0] for a in node.names)
            elif isinstance(node, ast.ImportFrom) and node.module and node.level == 0:
                names.add(node.module.split(".")[0])
        for name in names:
            for d in [p.parent] + search:
                cand = d / f"{name}.py"
                if cand.exists():
                    todo.append(cand)
                    break
    return sorted(found)


def tracked(rel):
    return subprocess.run(["git", "ls-files", "--error-unmatch", str(rel)], cwd=REPO,
                          capture_output=True).returncode == 0


def sha256(p):
    h = hashlib.sha256()
    with open(p, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def add(z, rel):
    z.write(REPO / rel, str(rel).replace(os.sep, "/"))


def main():
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=REPO, capture_output=True, text=True).stdout.strip()
    tag = subprocess.run(["git", "rev-parse", f"{TAG}^{{commit}}"], cwd=REPO, capture_output=True, text=True).stdout.strip()
    # empty the folder in place (OneDrive refuses to delete and recreate a synced folder)
    OUT.mkdir(exist_ok=True)
    for p in OUT.iterdir():
        shutil.rmtree(p) if p.is_dir() else p.unlink()

    shutil.copy2(PAPER / "paper.pdf", OUT / "paper.pdf")

    with zipfile.ZipFile(OUT / "reading-the-curve-v2-latex.zip", "w", zipfile.ZIP_DEFLATED) as z:
        # paper.tex loads ../dgstyle.tex and tables/*.tex: keep that layout
        z.write(PAPER.parent / "dgstyle.tex", "dgstyle.tex")
        for rel in ["paper.tex", "README.md"]:
            z.write(PAPER / rel, f"reading-the-curve/{rel}")
        for sub in ("figures", "tables"):
            for p in sorted((PAPER / sub).rglob("*")):
                if p.is_file():
                    z.write(p, "reading-the-curve/" + str(p.relative_to(PAPER)).replace(os.sep, "/"))

    files = set(import_closure())
    for rel in EXTRA_FILES:
        if (REPO / rel).exists():
            files.add(Path(rel))
    for d in DIRS:
        for p in (REPO / d).rglob("*"):
            if p.is_file() and "__pycache__" not in p.parts:
                files.add(p.relative_to(REPO))
    for g in DOC_GLOBS:
        files.update(p.relative_to(REPO) for p in REPO.glob(g))
    for p in PAPER.rglob("*"):
        rel = p.relative_to(PAPER)
        if p.is_file() and not (set(rel.parts) & PAPER_EXCLUDE) and p.suffix not in PAPER_EXCLUDE_SUFFIX:
            files.add(p.relative_to(REPO))
    untracked = [f for f in sorted(files) if not tracked(f)]
    k_files = []
    readme = f"""# Reading the Curve, Not Counting the Points: code and data (version 2)

Bee Rosa Davis, Davis Geometric. Everything behind the paper's numbers, with the paths they have
in the GIGI repository (https://github.com/nurdymuny/gigi, tag `{TAG}`, commit {tag or head}).
The full engine is in the repository; this archive holds what the paper's results run through.

- `scripts/`, `e2e/probes/`: the research scripts and every module they import.
- The GIGI engine itself (including the `RECOVER_CURVATURE` verb, `src/gauge/curvature_recovery.rs`)
  is not in this archive; it is in the repository at the tag above.
- `docs/papers/two-ring-pg/`: the manuscript, its figure and table scripts, `data/` and `tables/`.
- `docs/*.md`: the specification and results of each experiment.
- `research/`: receipts, gate verdicts, run final states.
- `output/`: the frozen inputs (meshes, native outputs; `ARTIFACT_HASHES.json` pins them) and the
  Abelian diagnostics.
- `outputs/su2-cayley/`: the Cayley step's Newton iteration matrices K0 at n = 1, 2, 3. They set
  how fast Newton converges, not the answer; `ricci_su2_pg_flow_cayley.py buildK` rebuilds them.

To rerun from the repository root, set nothing: the scripts find `output/` relative to the
repository. Each script's docstring gives its usage. Examples:
  python scripts/ricci_su2_pg_flow_cayley.py run 1 pg 8
  python scripts/ricci_su2_batched.py parity 0 pg
  python docs/papers/two-ring-pg/make_figures_su2flow.py

The SU(2) results are research scripts, not engine runs. Licence: PolyForm Noncommercial 1.0.0
(see LICENSE in the repository).
"""
    with zipfile.ZipFile(OUT / "reading-the-curve-v2-code-and-data.zip", "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("README_PACKAGE.md", readme)
        for rel in sorted(files):
            add(z, rel)
        # The Cayley step's Newton iteration matrices K0 (convergence speed only, not the answer).
        # Shipped as sparse .npz at outputs/su2-cayley/, where ricci_su2_pg_flow_cayley.load_K looks;
        # the dense n = 1, 2 copies are converted exactly (same values, smaller file).
        import numpy as np
        from scipy.sparse import csc_matrix, save_npz
        tmp = OUT / "_k"
        tmp.mkdir()
        for level in (0, 1, 2):
            for trial in ("native", "pg"):
                name = f"L{level}_{trial}_K.npz"
                dense = REPO / "outputs" / "su2-numrange" / f"L{level}_{trial}_K.npy"
                sparse = REPO / "outputs" / "su2-cayley" / name
                if sparse.exists():
                    src = sparse
                elif dense.exists():
                    src = tmp / name
                    save_npz(src, csc_matrix(np.load(dense)))
                else:
                    raise SystemExit(f"missing K0 for level {level} {trial}")
                z.write(src, f"outputs/su2-cayley/{name}")
                k_files.append(f"outputs/su2-cayley/{name}")
        shutil.rmtree(tmp)

    lines = [f"# Reading the Curve, version 2: files for Zenodo",
             f"# built from commit {head} (tag {TAG} -> {tag or 'not found'})", ""]
    for name in ["paper.pdf", "reading-the-curve-v2-latex.zip", "reading-the-curve-v2-code-and-data.zip"]:
        p = OUT / name
        lines.append(f"{sha256(p)}  {p.stat().st_size:>11,}  {name}")
    lines += ["", f"code-and-data archive: {len(files) + len(k_files)} files"
              f" ({len(k_files)} of them the Cayley step's K0 matrices, outputs/su2-cayley/)"]
    if untracked:
        lines += ["files not tracked by git (included because the results depend on them):"]
        lines += [f"  {str(f).replace(os.sep, '/')}" for f in untracked]
    (OUT / "MANIFEST.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print("\n".join(lines))


if __name__ == "__main__":
    main()
