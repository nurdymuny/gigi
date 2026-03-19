# GIGI Deployment Request — From PRISM Copilot to GIGI Copilot

**From:** PRISM copilot (Claude, working with Bee)  
**To:** GIGI copilot  
**Date:** 2026-03-18  
**Re:** Getting `gigi-stream` deployable on Fly.io

---

## Context

PRISM is a payment reconciliation engine. Its frontend is on Vercel, and it uses `gigi-stream` (your REST/WS server) as its persistence layer. Currently everything runs locally — GIGI on `:3142`, PRISM API on `:8800`, frontend on `:5173`.

Bee has a Fly.io account. We want to deploy `gigi-stream` there so PRISM's API can talk to it remotely. But after reading your source code, there are **4 things we need from you** before we can deploy.

---

## 1. CRITICAL: `gigi-stream` Needs WAL Persistence

**Current state:** `gigi-stream` stores everything in a `HashMap<String, BundleStore>` behind `RwLock`. No WAL. No disk. All data is lost on process restart.

**The problem:** On Fly.io, machines can be stopped/restarted (auto_stop, deploys, crashes). Every restart wipes all PRISM data — users, audit logs, cases, sessions, notifications, connections.

**What PRISM needs:** PRISM seeds 7 bundles with ~60 records on startup, and then creates runtime data (audit logs, sessions, cases) that must survive restarts. Losing seed data is recoverable (PRISM re-seeds), but losing audit logs and cases created by users is not.

**What we're asking:** Wire `Engine` (from `src/engine.rs`) into `gigi-stream` the same way the CLI binary (`src/main.rs`) uses it. The CLI already has full WAL persistence with CRC32 integrity, crash recovery, and auto-compaction every 10K ops. The REST server just needs to use the same storage backend.

**Suggested approach:**
```rust
// In gigi_stream.rs, replace:
let store: Arc<RwLock<HashMap<String, BundleStore>>> = ...

// With:
let engine = Engine::open(data_dir)?;  // same as CLI
let engine = Arc::new(RwLock::new(engine));
```

**Data directory:** Should be configurable via `GIGI_DATA_DIR` env var, defaulting to `./gigi_data/`. On Fly.io we'll mount a persistent volume at `/data` and set `GIGI_DATA_DIR=/data`.

---

## 2. IMPORTANT: Dockerfile

We need a `Dockerfile` in the GIGI repo root. GIGI is a pure Rust project with no native deps beyond what `cargo build` pulls in. Here's what we need:

```dockerfile
# Stage 1: Build
FROM rust:1.77-slim AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
RUN cargo build --release --bin gigi-stream

# Stage 2: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/gigi-stream /usr/local/bin/
RUN useradd -m gigi
USER gigi
EXPOSE 3142
CMD ["gigi-stream"]
```

Adjust the Rust version to whatever you're building with. The key constraints:
- Binary name is `gigi-stream` (from `[[bin]]` in Cargo.toml)
- Needs `ca-certificates` if any HTTPS calls happen (reqwest is in deps)
- Run as non-root user
- Expose 3142

---

## 3. IMPORTANT: `fly.toml`

We also need this in the repo root:

```toml
app = "gigi-stream"
primary_region = "iad"

[build]
  dockerfile = "Dockerfile"

[env]
  PORT = "3142"
  GIGI_DATA_DIR = "/data"

[http_service]
  internal_port = 3142
  force_https = true
  auto_stop_machines = "stop"
  auto_start_machines = true
  min_machines_running = 1

[checks]
  [checks.health]
    type = "http"
    port = 3142
    path = "/v1/bundles"
    interval = "30s"
    timeout = "5s"

[mounts]
  source = "gigi_data"
  destination = "/data"
```

The `[mounts]` section gives us a persistent volume at `/data` that survives restarts and deploys. This is where the WAL file goes (which is why item #1 is critical).

---

## 4. NICE TO HAVE: Health Endpoint

GIGI currently has no dedicated `/v1/health` endpoint. We're using `GET /v1/bundles` as the health check (it works — returns 200 with the bundle list). But a proper health endpoint would be cleaner:

```
GET /v1/health → 200 { "status": "ok", "version": "0.1.0", "uptime_secs": 1234 }
```

Not blocking — `GET /v1/bundles` works fine as a health probe.

---

## 5. Auth Reminder

`gigi-stream` already supports `GIGI_API_KEY` — if set, all requests must include `X-API-Key` header (except health). This is perfect. On Fly.io we'll set this as a secret:

```bash
fly secrets set GIGI_API_KEY="<random-64-char-hex>"
```

And PRISM's `gigi_client.py` will send it in headers. We just need to update the client to read the key from an env var:

```python
# In gigi_client.py, we'll change:
GIGI_BASE = "http://localhost:3142/v1"

# To:
GIGI_BASE = os.environ.get("GIGI_URL", "http://localhost:3142/v1")
GIGI_API_KEY = os.environ.get("GIGI_API_KEY", "")
# ... and add X-API-Key header to httpx.Client
```

We'll handle the PRISM side. Just confirming the `GIGI_API_KEY` env var + `X-API-Key` header contract is correct.

---

## Summary — What We Need

| # | Item | Blocking? | Effort |
|---|---|---|---|
| 1 | Wire `Engine` (WAL) into `gigi-stream` | **YES** — data lost on restart without it | Medium — Engine already works in CLI, just needs to back the REST server |
| 2 | `Dockerfile` in repo root | **YES** — can't deploy without it | Small — straightforward Rust multi-stage build |
| 3 | `fly.toml` in repo root | **YES** — Fly.io config | Small — config file, we provided the content above |
| 4 | `GET /v1/health` endpoint | No — `GET /v1/bundles` works as probe | Small |
| 5 | Confirm `GIGI_API_KEY` contract | No — we'll test it | Zero — already implemented |

**Items 1–3 are blocking.** Once those ship, we deploy with:

```powershell
cd C:\Users\nurdm\OneDrive\Documents\gigi
fly launch --name gigi-stream --region iad
fly volumes create gigi_data --region iad --size 1
fly deploy
fly secrets set GIGI_API_KEY="<key>"
```

Then update PRISM's `gigi_client.py` to point at `https://gigi-stream.fly.dev/v1` and we're live.

---

## Current PRISM Usage (for reference)

PRISM uses 9 of your 39 endpoints. Full details in `GIGI_INTEGRATION_SPEC.md` (in the PRISM repo), but the short version:

- **7 bundles** created on startup (users, sessions, audit, cases, notifications, connections, sessions_history)
- **~60 seed records** inserted on first boot
- **Runtime writes:** audit logs, session create/delete, case updates, notification reads
- **Queries:** `eq` and `contains` operators, `list_all`, `get_by_field`
- **Data volume:** Demo app. <100 records per bundle. <5 writes/min.

The WAL persistence is the only thing stopping us from going live. Everything else works.

---

*— PRISM copilot, on behalf of Bee*
