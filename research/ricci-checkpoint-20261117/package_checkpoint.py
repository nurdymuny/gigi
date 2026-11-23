"""Archive retained Ricci evidence verbatim; no experiments or source changes."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import zipfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
LIMIT = 80*1024*1024


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def pack(name, files):
    destination = HERE/(name+".zip")
    if destination.exists():
        raise ValueError("preserve prior archive: "+str(destination))
    entries = []
    with zipfile.ZipFile(destination, "x", compression=zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
        for path in files:
            data = path.read_bytes()
            relative = path.relative_to(ROOT).as_posix()
            info = zipfile.ZipInfo(relative, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            archive.writestr(info, data, compress_type=zipfile.ZIP_DEFLATED, compresslevel=6)
            entries.append({"path": relative, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()})
    if destination.stat().st_size >= 95*1024*1024:
        raise ValueError("archive exceeds checkpoint per-file budget")
    return {"archive": destination.name, "bytes": destination.stat().st_size,
            "sha256": digest(destination), "entries": entries}


def source_references(value, collected):
    if isinstance(value, dict):
        for key, child in value.items():
            source_references(key, collected)
            source_references(child, collected)
    elif isinstance(value, list):
        for child in value:
            source_references(child, collected)
    elif isinstance(value, str) and value.replace("\\", "/").lower().startswith(ROOT.as_posix().lower()+"/"):
        path = Path(value)
        if path.suffix.lower() in (".py", ".rs", ".toml", ".md") and path.is_file():
            collected.add(path)


def build():
    manifest = HERE/"MANIFEST.json"
    if manifest.exists():
        raise ValueError("preserve prior manifest")
    groups = sorted((ROOT/"output").glob("ricci*")) + sorted((ROOT/"output").glob("monodromy-*"))
    archives = []; snapshots = set(); excluded = []
    for directory in groups:
        files = []
        for path in sorted(directory.rglob("*")):
            if not path.is_file():
                continue
            if path.suffix.lower() in (".exe", ".pyc") or "__pycache__" in path.parts:
                excluded.append({"path": path.relative_to(ROOT).as_posix(), "bytes": path.stat().st_size,
                                 "sha256": digest(path), "reason": "compiled artifact; rebuild from source"})
                continue
            files.append(path)
            if path.suffix == ".json" and path.stat().st_size < 4*1024*1024:
                source_references(json.loads(path.read_text(encoding="utf-8")), snapshots)
        batch = []; size = 0; part = 1
        for path in files:
            if batch and size+path.stat().st_size > LIMIT:
                archives.append(pack(directory.name+f"-part{part:02}", batch))
                batch = []; size = 0; part += 1
            batch.append(path); size += path.stat().st_size
        if batch:
            archives.append(pack(directory.name+f"-part{part:02}", batch))
        print(directory.name, "archived", len(files), "files", flush=True)
    # Exact current versions of referenced source files are retained separately
    # from the scoped active-code changes. Historical mismatches stay disclosed.
    for relative in ("Cargo.toml", "src/lib.rs", "src/parser.rs", "src/bin/gigi_stream.rs",
                     "src/halcyon_gql_dispatch.rs", "src/gauge/mod.rs", "GQL_REFERENCE.md"):
        snapshots.add(ROOT/relative)
    archives.append(pack("source-snapshot", sorted(snapshots)))
    receipt = {"schema": "ricci_research_checkpoint_v1", "purpose": "progress preservation, not release acceptance",
        "captured_root": str(ROOT), "base_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "branch": subprocess.check_output(["git", "branch", "--show-current"], cwd=ROOT, text=True).strip(),
        "global_gate": "UNCHANGED_OPEN", "boundary_extension_passed": False,
        "archives": archives, "excluded": excluded,
        "source_snapshot_scope": "current referenced sources including mixed-worktree shared files; not applied to active code"}
    manifest.write_text(json.dumps(receipt, indent=2)+"\n", encoding="utf-8", newline="\n")
    verify()


def verify():
    manifest = json.loads((HERE/"MANIFEST.json").read_text(encoding="utf-8"))
    total = 0
    for item in manifest["archives"]:
        path = HERE/item["archive"]
        if path.stat().st_size != item["bytes"] or digest(path) != item["sha256"]:
            raise ValueError("archive drift: "+str(path))
        with zipfile.ZipFile(path) as archive:
            if archive.namelist() != [entry["path"] for entry in item["entries"]]:
                raise ValueError("archive entry list drift")
            for entry in item["entries"]:
                with archive.open(entry["path"]) as stream:
                    actual = hashlib.file_digest(stream, "sha256").hexdigest()
                if archive.getinfo(entry["path"]).file_size != entry["bytes"] or actual != entry["sha256"]:
                    raise ValueError("retained bytes differ: "+entry["path"])
                total += 1
    print(json.dumps({"archives_verified": len(manifest["archives"]), "entries_verified": total,
        "archive_MiB": round(sum(item["bytes"] for item in manifest["archives"])/1024**2, 2),
        "experiments_rerun": False}), flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(); parser.add_argument("mode", choices=["build", "verify"])
    args = parser.parse_args()
    build() if args.mode == "build" else verify()
