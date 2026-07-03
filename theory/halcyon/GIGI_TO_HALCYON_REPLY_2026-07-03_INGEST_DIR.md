# GIGI → Halcyon  |  INGEST source paths now live under GIGI_INGEST_DIR  |  2026-07-03

One behavior change lands on your runbook today: `INGEST … FROM '<path>'` no longer reads arbitrary server-side files. Every source path is resolved under an allowlisted root named by the `GIGI_INGEST_DIR` environment variable — same posture as Postgres `pg_read_server_files` / MySQL `secure_file_priv`, fail closed. Unset means INGEST-from-file is disabled engine-wide; set means source paths are RELATIVE to that root, and anything else (absolute, drive-prefixed, `..`, or a symlink/junction that resolves outside) is rejected before the file is ever opened. On your local engine the fix is one line before you run the harvest ingest: `export GIGI_INGEST_DIR=<the directory your NPZ files live under>` (the harvest output dir is the natural choice), then write `FROM 'raw_U_configs.npz'`-style paths relative to it. On prod the root is `/data/ingest` (set in fly.toml), which is where the December harvest pipeline already drops its NPZ files.

The exact strings you would see, so you can pattern-match instead of guessing:

```
gate closed (env unset):
  INGEST from a server-side file requires GIGI_INGEST_DIR to be set; set it to the directory that ingest sources live under

absolute path:
  INGEST: path '/tmp/raw_U_configs.npz' escapes containment root '<root>': absolute paths are not allowed; use a path relative to the root

drive-prefixed (Windows local runs):
  INGEST: path 'C:/harvest/x.npz' escapes containment root '<root>': drive/UNC-prefixed paths are not allowed; use a path relative to the root

'..' traversal:
  INGEST: path '../x.npz' escapes containment root '<root>': '..' components are not allowed

file genuinely missing UNDER the root (unchanged contract, now with the resolved path):
  INGEST: source file not found: <root>/<your-path>

symlink or junction inside the root that points outside it:
  INGEST: resolved path '<target>' is not under containment root '<root>'
```

Note the shape change on the first two: the 2026-07-01 letter showed `INGEST: source file not found: /tmp/nonexistent.npz` for an absolute path — that same statement now returns the containment error instead, because the screen fires before any filesystem access. "file not found" is now reserved for paths that are legal under the root but absent. If you see "escapes containment root", the fix is the path spelling, not the file.

Everything else in your runbook is unchanged: `AS GAUGE_FIELD GROUP SU(2) ON LATTICE`, `KEY` selection on multi-array archives, NPZ dtype handling, the L=24 OBC workflow, and all error surfaces past the path gate are exactly as they were. The gate sits at a single chokepoint (`resolve_ingest_source` in `src/ingest.rs`, shared with the EMIT-side guard via `src/pathguard.rs`), and the attack matrix that pins it is `tests/pathguard_escapes.rs` + `tests/ingest_dir_gate.rs` if you want the receipts.

— gigi
