"""Build the posting packages (Zenodo and the Drive library) for the Floor, edge and Three Ways papers.

usage: python scripts/make_paper_packages.py OUT_DIR [floor] [edge] [threeways] [threeways-ru]

For each paper it writes OUT_DIR/<name>/ with, for the Drive library:
  <Title>.pdf                    the clean PDF, built from the committed source
  <Title> - LaTeX source.txt     the LaTeX as one readable text file (the library's convention)
  description.html, metadata.json / METADATA.md, UPLOAD.md where the paper has them
and OUT_DIR/<name>/zenodo-upload/ with exactly the files the record's description names:
  paper.pdf (paper_ru.pdf for the Russian edition)   the same PDF under its record name
  paper_ru.pdf, perelman_ru.pdf  (Three Ways) the Russian edition and the chapter, which lives outside the repository
  <slug>-latex.zip               a standalone LaTeX package: the paper directory, the shared style, README.txt
  <slug>-code-and-data.zip       (floor, edge) every script the paper cites, the repository scripts they import,
                                 every receipt directory or file the paper cites, the code README, requirements.txt
  MANIFEST.txt                   SHA-256 and size of every other file in the folder, the commit, and the code
                                 archive's listing
Then it checks the package: the LaTeX compiles from its zip into an empty folder, and for floor and edge the fast
proof scripts rerun from the code zip in an empty folder and exit 0.
"""
import ast
import hashlib
import os
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
PY = sys.executable

# the Perelman chapter is not in the repository; it is posted with both Three Ways records
PERELMAN = str(Path.home() / "Documents/three-ways-ru-zenodo/perelman_ru.pdf")

PAPERS = {
    "floor": dict(dir="docs/papers/berger-spectrum", tex="paper.tex", engine="pdflatex", clean_redline=True,
                  title="The Floor Theorem for the Yang-Mills Hessian at the Berger Field v3.3", slug="floor-theorem-v3.3", upload_pdf="paper.pdf",
                  style=["docs/papers/dgstyle.tex"], zenodo="docs/papers/berger-spectrum/zenodo",
                  exclude=["paper.pdf", "paper_redline.pdf", "paper_v3.pdf", "paper_v3_draft_redline.pdf", "social/", "zenodo/"],
                  code=True, code_readme="docs/papers/berger-spectrum/zenodo/code-README.md",
                  extra_code=["scripts/_receipt_status.py", "scripts/test_receipt_status.py", "LICENSE",
                              "docs/papers/two-ring-pg/make_figure_spectrum_ladder.py"],
                  receipt_dirs=["research/su2-left-invariant-sector-20261119", "research/su2-floor-family-20261119",
                                "research/su2-pg-stability-20261117", "research/su2-spectrum-n3-20261119"],
                  reruns=[["scripts/ricci_su2_round_sphere_check.py", "out/round_sphere_check.json"],
                          ["scripts/ricci_su2_floor_large_j.py", "out/floor_large_j.json", "50", "20", "2.75", "10"],
                          ["scripts/ricci_su2_charge_selection_symbolic.py", "out/charge_selection_symbolic.txt"],
                          ["scripts/test_receipt_status.py"]]),
    "edge": dict(dir="docs/papers/by-enough", tex="paper.tex", engine="pdflatex", clean_redline=False,
                 title="By Enough, But Not Too Much v1.1", slug="by-enough-v1.1", upload_pdf="paper.pdf",
                 style=["docs/papers/dgstyle.tex"], zenodo="docs/papers/by-enough/zenodo",
                 exclude=["paper.pdf", "zenodo/"], code=True, code_readme=None,
                 extra_code=["scripts/edge_paper_figures.py", "scripts/_receipt_status.py", "scripts/test_receipt_status.py", "scripts/ricci_su2_floor_large_j.py",
                             "docs/review-package-2026-11-23/DERIVATIONS_README.md", "LICENSE"],
                 receipt_dirs=["research/su2-floor-family-20261119", "research/edge-theorem-20261121",
                               "research/edge-theorem-20261122-proofs", "research/edge-paper-figures-20261122",
                               "research/edge-theorem-20261122-d", "research/edge-theorem-20261122-e",
                               "research/edge-theorem-20261122-f"],
                 reruns=[["scripts/ricci_su2_weitzenbock_family_symbolic.py", "out/weitzenbock_family.json"],
                         ["scripts/ricci_su2_rows_transcription_check.py", "out/rows_transcription.json"],
                         ["scripts/test_receipt_status.py"]]),
    "threeways": dict(dir="docs/papers/chord-arc-tangent", tex="paper.tex", engine="pdflatex", clean_redline=False,
                      title="Three Ways Across a Turn v2.1", slug="three-ways-v2.1", upload_pdf="paper.pdf",
                      upload_extra=[("@threeways-ru/paper_ru.pdf", "paper_ru.pdf"), (PERELMAN, "perelman_ru.pdf")],
                      style=["docs/papers/dgstyle.tex"], zenodo="docs/papers/chord-arc-tangent/zenodo",
                      exclude=["paper.pdf", "ru/", "zenodo/"], code=False),
    "threeways-ru": dict(dir="docs/papers/chord-arc-tangent/ru", tex="paper_ru.tex", engine="xelatex", clean_redline=False,
                         title="Three Ways Across a Turn v2.1 (Russian edition)", slug="three-ways-ru-v2.1", upload_pdf="paper_ru.pdf",
                         upload_extra=[(PERELMAN, "perelman_ru.pdf")],
                         style=["docs/papers/dgstyle-ru.tex"], zenodo="docs/papers/chord-arc-tangent/ru/zenodo",
                         exclude=["paper_ru.pdf", "zenodo/"], code=False),
}


def git(*a):
    return subprocess.run(["git", "-C", str(REPO), *a], capture_output=True, text=True, check=True).stdout


def tracked(d):
    return [l for l in git("ls-files", d).splitlines() if l]


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def cited_paths(tex):
    toks = re.findall(r"\\path\{([^}]*)\}", tex)
    scripts, receipts = set(), set()
    for t in toks:
        t = t.strip()
        if t.endswith(".py"):
            cand = [t] if t.startswith(("scripts/", "docs/")) else [f"scripts/{t}"]
            for c in cand:
                if (REPO / c).exists():
                    scripts.add(c)
        elif t.startswith("research/"):
            base = t.split("*")[0].rstrip("/")
            p = REPO / base
            if p.is_dir():
                receipts.add(base)
            elif p.exists():
                receipts.add(base)
            elif (REPO / base).parent.is_dir():
                receipts.add(str(Path(base).parent))
    return scripts, receipts


def import_closure(scripts):
    seen, todo = set(), list(scripts)
    while todo:
        s = todo.pop()
        if s in seen or not (REPO / s).exists():
            continue
        seen.add(s)
        try:
            tree = ast.parse((REPO / s).read_text())
        except SyntaxError:
            continue
        here = (REPO / s).parent
        for node in ast.walk(tree):
            names = []
            if isinstance(node, ast.Import):
                names = [a.name.split(".")[0] for a in node.names]
            elif isinstance(node, ast.ImportFrom) and node.module and node.level == 0:
                names = [node.module.split(".")[0]]
            for n in names:
                for cand in (here / f"{n}.py", REPO / "scripts" / f"{n}.py"):
                    if cand.exists():
                        todo.append(str(cand.relative_to(REPO)))
                        break
    return sorted(seen)


def build_pdf(workdir, tex, engine):
    for _ in range(2):
        r = subprocess.run([engine, "-interaction=nonstopmode", "-halt-on-error", tex], cwd=workdir, capture_output=True, text=True, errors="replace")
    if r.returncode != 0:
        raise SystemExit(f"LaTeX failed in {workdir}:\n{r.stdout[-2000:]}")
    return Path(workdir) / (Path(tex).stem + ".pdf")


def assemble_tex(d, tex):
    """One readable text file: the main file with \\input/\\include files expanded and marked."""
    def expand(path, depth=0):
        out = []
        for line in path.read_text().splitlines():
            m = re.match(r"\s*\\(input|include)\{([^}]*)\}", line)
            if m and depth < 4:
                name = m.group(2)
                cand = (path.parent / name)
                cand = cand if cand.suffix == ".tex" else cand.with_suffix(".tex")
                if cand.exists():
                    out.append(f"% ===== begin {cand.relative_to(REPO)} =====")
                    out += expand(cand, depth + 1)
                    out.append(f"% ===== end {cand.relative_to(REPO)} =====")
                    continue
            out.append(line)
        return out
    return "\n".join(expand(REPO / d / tex)) + "\n"


def package(key, OUT, commit):
    cfg = PAPERS[key]; d = cfg["dir"]; out = OUT / key; out.mkdir(parents=True, exist_ok=True)
    up = out / "zenodo-upload"
    if up.exists():
        shutil.rmtree(up)
    up.mkdir()
    stage = Path(tempfile.mkdtemp())
    # LaTeX package: tracked files of the paper directory (minus built PDFs and upload folders) + shared style
    L = stage / "latex"
    files = [f for f in tracked(d) if not any(f[len(d) + 1:].startswith(x) for x in cfg["exclude"])]
    for f in files + cfg["style"]:
        (L / f).parent.mkdir(parents=True, exist_ok=True); shutil.copy2(REPO / f, L / f)
    texp = L / d / cfg["tex"]
    if cfg["clean_redline"]:
        texp.write_text(re.sub(r"^\\redlinetrue", r"\\redlinefalse", texp.read_text(), flags=re.M))
    (L / "README.txt").write_text(
        f"{cfg['title']}: LaTeX source (from commit {commit} of github.com/nurdymuny/gigi).\n"
        f"Build:  cd {d} && {cfg['engine']} {cfg['tex']} && {cfg['engine']} {cfg['tex']}\n"
        + ("This copy builds the clean version (\\redlinefalse). Set \\redlinetrue to see the changes against version 2.\n"
           if cfg["clean_redline"] else ""))
    zl = up / f"{cfg['slug']}-latex.zip"
    with zipfile.ZipFile(zl, "w", zipfile.ZIP_DEFLATED) as z:
        for p in sorted(L.rglob("*")):
            if p.is_file():
                z.write(p, p.relative_to(L))
    # build the PDF from the zip contents (this is also the compile check)
    pdf = build_pdf(L / d, cfg["tex"], cfg["engine"])
    shutil.copy2(pdf, out / f"{cfg['title']}.pdf")
    # readable source text (expands \input of the paper's own parts)
    (out / f"{cfg['title']} - LaTeX source.txt").write_text(assemble_tex(d, cfg["tex"]) if not cfg["clean_redline"]
                                                             else re.sub(r"^\\redlinetrue", r"\\redlinefalse", assemble_tex(d, cfg["tex"]), flags=re.M))
    shutil.copy2(pdf, up / cfg["upload_pdf"])
    for src, name in cfg.get("upload_extra", []):
        src = OUT / src[1:].replace("/", "/zenodo-upload/", 1) if src.startswith("@") else Path(src)
        if not src.exists():
            raise SystemExit(f"[{key}] missing upload file {src} (build threeways-ru first)")
        shutil.copy2(src, up / name)
    for old in ("MANIFEST.txt", *(p.name for p in out.glob("*.zip"))):
        if (out / old).exists():
            (out / old).unlink()
    code_list = []
    if cfg.get("code"):
        tex = (REPO / d / cfg["tex"]).read_text()
        scripts, receipts = cited_paths(tex)
        receipts |= set(cfg.get("receipt_dirs", []))
        for f in cfg.get("extra_code", []):
            if f.endswith(".py"):
                scripts.add(f)
        allpy = import_closure(scripts)
        C = stage / "code"; items = set(allpy) | {f for f in cfg.get("extra_code", []) if not f.endswith(".py")}
        for r in receipts:
            p = REPO / r
            items |= set(tracked(r)) if p.is_dir() else {r}
        for f in sorted(items):
            if not (REPO / f).exists():
                continue
            (C / f).parent.mkdir(parents=True, exist_ok=True); shutil.copy2(REPO / f, C / f)
        readme = cfg.get("code_readme")
        if readme:
            shutil.copy2(REPO / readme, C / "README.md")
        else:
            (C / "README.md").write_text(
                f"# {cfg['title']}: code and data\n\nEvery script the paper cites, the repository scripts they import, and "
                f"every receipt directory the paper cites, at their repository paths (commit {commit}).\n"
                f"The claim-by-claim list (script, receipt, command, runtime, status) is "
                f"`docs/review-package-2026-11-23/DERIVATIONS_README.md`, included here.\n\n"
                f"Requirements: Python 3 with NumPy, SciPy, SymPy and mpmath (`requirements.txt`). Run scripts from this "
                f"folder, e.g. `python scripts/ricci_su2_weitzenbock_family_symbolic.py out/weitzenbock_family.json`.\n"
                f"Proof scripts exit 1 if their receipt records a failure (`scripts/_receipt_status.py`; its test is "
                f"`scripts/test_receipt_status.py`).\n")
        (C / "requirements.txt").write_text("numpy\nscipy\nsympy\nmpmath\n")
        zc = up / f"{cfg['slug']}-code-and-data.zip"
        with zipfile.ZipFile(zc, "w", zipfile.ZIP_DEFLATED) as z:
            for p in sorted(C.rglob("*")):
                if p.is_file():
                    z.write(p, p.relative_to(C)); code_list.append(str(p.relative_to(C)))
        # rerun check from an empty folder
        T = Path(tempfile.mkdtemp()); zipfile.ZipFile(zc).extractall(T); (T / "out").mkdir()
        for cmd in cfg["reruns"]:
            r = subprocess.run([PY, *cmd], cwd=T, capture_output=True, text=True, errors="replace")
            status = "ok" if r.returncode == 0 else f"FAILED ({r.returncode})"
            print(f"  [{key}] rerun {cmd[0]}: {status}")
            if r.returncode != 0:
                print(r.stdout[-800:], r.stderr[-800:])
                raise SystemExit(f"rerun failed: {cmd}")
        shutil.rmtree(T)
    for f in ("description.html", "metadata.json", "METADATA.md", "UPLOAD.md"):
        p = REPO / cfg["zenodo"] / f
        if p.exists():
            shutil.copy2(p, out / f)
    uploaded = sorted(p for p in up.iterdir() if p.name != "MANIFEST.txt")
    with open(up / "MANIFEST.txt", "w") as m:
        m.write(f"# {cfg['title']}: the files of this record\n# built from commit {commit} of github.com/nurdymuny/gigi\n\n")
        for f in uploaded:
            m.write(f"{sha(f)}  {f.stat().st_size:>12,}  {f.name}\n")
        if code_list:
            m.write(f"\ncode-and-data archive: {len(code_list)} files\n")
            m.writelines(f"  {c}\n" for c in code_list)
    shutil.rmtree(stage)
    print(f"[{key}] {out}: {', '.join(p.name for p in sorted(out.iterdir()))}")
    print(f"[{key}] zenodo-upload: {', '.join(p.name for p in sorted(up.iterdir()))}")


def main():
    OUT = Path(sys.argv[1]).resolve(); keys = sys.argv[2:] or list(PAPERS)
    if "threeways" in keys and "threeways-ru" in keys:  # the English record carries the Russian PDF
        keys.remove("threeways-ru"); keys.insert(keys.index("threeways"), "threeways-ru")
    commit = git("rev-parse", "--short", "HEAD").strip()
    for k in keys:
        package(k, OUT, commit)


if __name__ == "__main__":
    main()
