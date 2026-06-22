# LOOP_TRANSPORT calling guide (Halcyon-side)

**Audience:** Halcyon orchestrator (`run_holonomy_battery.py` + `LiveLoopTransportClient`).
**Reference:** Halcyon `HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3.md` §4.4 + gigi gate doc `theory/halcyon/HALCYON_PART_VI_GATES.md`.

This is the substrate-side calling convention for `LOOP_TRANSPORT` against a local or production gigi-stream. Conventions discovered during Halcyon's 2026-06-21 live-binding session, consolidated.

---

## Preconditions (in order)

The substrate's executor arm for `Statement::LoopTransport` (`src/parser.rs:10338-10401`) hardcodes the gauge field name `U_lt` and E-field name `E_lt`. The orchestrator must declare these names exactly, in this order, before firing the verb.

```sql
-- 1. Lattice. v3.1.3 §4.4 canonical name is halcyon_canonical_buckyball.
LATTICE halcyon_canonical_buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';

-- 2. Gauge field, named exactly U_lt. INIT IDENTITY or use a thermalized
--    HAAR_RANDOM seed depending on whether you want the canonical
--    pre-thermalized state or a fresh start.
GAUGE_FIELD U_lt ON LATTICE halcyon_canonical_buckyball
            GROUP SU(2) INIT IDENTITY;
-- OR for the thermalized state:
-- GAUGE_FIELD U_lt ON LATTICE halcyon_canonical_buckyball
--             GROUP SU(2) INIT HAAR_RANDOM SEED 20260616;
-- GIBBS_SAMPLE U_lt BETA 2.5 SWEEPS 100 SEED 20260616;

-- 3. E field, named exactly E_lt, attached to U_lt. INIT ZERO is the
--    canonical starting momentum state.
E_FIELD E_lt ON GAUGE_FIELD U_lt INIT ZERO;

-- 4. The spatial loop on the lattice. VI.2 grammar uses `LOOP <name>
--    ON <lattice>` (NOT `DECLARE LOOP`). Two forms:
--      FACE <n>           — wrap the face's edge cycle
--      EDGES (v1, v2, ...) — explicit vertex path
--    The canonical loop is FACE 0 (the first pentagon of the buckyball).
LOOP gamma_unit_in_Q_beta_W ON halcyon_canonical_buckyball FACE 0;
```

**Topology literal:** single quotes only (`'S2'`), not double quotes. The parser rejects `"S2"`.

---

## The verb call

The full v3.1.3 §4.4 parameter pack. The parser keeps `SAMPLE_TRANSPORT` aliased in the v3.1.3 spec text for chain-of-custody, but the implementation name is `LOOP_TRANSPORT` (gigi VI.2 rename, agreed cross-team — resolves the name collision with the existing bundle-side `sample_transport` curvature primitive).

```sql
LOOP_TRANSPORT halcyon_canonical_buckyball
  ALONG_LOOP gamma_unit_in_Q_beta_W
  CONTROL_MANIFOLD (Q, BETA_WILSON)
  ADIABATIC TRUE
  RAMP_RATE_Q 0.04
  RAMP_RATE_BETA_W 0.01
  DRIVE_OMEGA 1.0
  DRIVE_F0 0.01
  N_DISCRETIZATION 10000
  PIN_LAMBDA_Q 1.0
  PIN_LAMBDA_BETA_W 1.0
  EPS_Q 0.05
  EPS_BETA_W 0.05
  ALPHA_HALCYON 1.0
  TAU_0 1.0  BETA_TAU 2.0
  MU_BASELINE 1.0  K_SPRING 1.0  C_DAMP 0.1
  SEEDS [20260616..20260623]
  COMPUTE HOLONOMY_FORWARD
  COMPUTE HOLONOMY_REVERSED
  COMPUTE TRACKING_ERROR_TRACE_Q
  COMPUTE TRACKING_ERROR_TRACE_BETA_W
  COMPUTE ADIABATICITY_CHECK
  RETURN H_FORWARD, H_REVERSED, SIGMA_H_BLOCKED,
         PER_SEED_H_FORWARD, PER_SEED_H_REVERSED,
         TRACKING_ERROR_MAX_Q, TRACKING_ERROR_MAX_BETA_W,
         ADIABATICITY_CHECK;
```

**Seed range syntax:** `[start..end]` inclusive — NOT the explicit list `[s1, s2, ...]`. v3.1.3 §4.4's canonical bracket is `[20260616..20260623]` (8 seeds).

For α=1000, change only `ALPHA_HALCYON`:
```sql
ALPHA_HALCYON 1000.0
```

For sham flag runs, add a `SHAM { ... }` block before `RETURN` (5 science-gate flags + 2 audit-story flags per VI.4):
```sql
  SHAM { FLAT_FIELD = TRUE }     -- S₁: kills parameter-space coupling
  SHAM { ALPHA_ZERO = TRUE }     -- S₂: ALPHA_HALCYON = 0 override
  SHAM { MASS_BASELINE_SCALED = 0.1 }   -- S₃: μ ∈ {0.1, 1.0, 10.0}
  SHAM { DEGENERATE_LOOP = TRUE } -- S₅: zero-area loop substitute
  SHAM { FROZEN_FIELD = TRUE }    -- S₆: U static across all substeps
  SHAM { EMPTY_LOOP = TRUE }      -- audit: integrator runs 0 substeps
-- OPEN_LOOP is enforced at parser (LoopTransportError::LoopNotClosed);
-- not a runtime SHAM flag.
```

Each sham flag combines independently in one `SHAM` block; the orchestrator runs the verb once per combination.

---

## Response shape

`POST /v1/gql` with the verb above returns a JSON envelope with a single row (the dispatcher emits `ExecResult::Rows(vec![record])`). Field names are **lowercase snake_case**:

```json
{
  "rows": [
    {
      "h_forward": -1.234567e-7,
      "h_reversed": 1.234560e-7,
      "sigma_h_blocked": 4.5e-9,
      "per_seed_h_forward": [..., ..., ..., ..., ..., ..., ..., ...],
      "per_seed_h_reversed": [..., ..., ..., ..., ..., ..., ..., ...],
      "tracking_error_max_q": 0.0123,
      "tracking_error_max_beta_w": 0.0045,
      "adiabaticity_verdict": "ACCEPTABLE",
      "adiabaticity_ratio": 0.072,
      "n_substeps_completed": 10000
    }
  ]
}
```

Per v3.1.3 §3.1, the orchestrator computes Python-side:
```python
H_geom = 0.5 * (row["h_forward"] - row["h_reversed"])
H_sys  = 0.5 * (row["h_forward"] + row["h_reversed"])
```
and applies the §3 POSITIVE / NULL / AMBIGUOUS gates.

`adiabaticity_verdict` is one of `"ACCEPTABLE"` or `"AMBIGUOUS_FORCED"` (per v3.1.3 §4.2: if `adiabaticity_ratio >= 0.1`, the substrate forces AMBIGUOUS regardless of H values).

---

## Local gigi-stream binding (no auth)

For local mode, gigi-stream binds unauthenticated on `localhost`. The `GIGI_API_KEY` env var is for production deployments only.

```powershell
# Start the binary on port 3142 in the background.
$env:GIGI_PORT = "3142"
Start-Process -FilePath ".\target\release\gigi-stream.exe" `
              -ArgumentList "--no-auth" `
              -NoNewWindow -PassThru

# Wait a couple seconds for bind, then probe /v1/health.
Start-Sleep -Seconds 2
Invoke-RestMethod -Uri "http://localhost:3142/v1/health"
# → { "status": "ok", "version": "0.1.0", ... }

# Fire LOOP_TRANSPORT (or any GQL statement) at /v1/gql.
$body = @{ query = "LOOP_TRANSPORT halcyon_canonical_buckyball ALONG_LOOP gamma_unit_in_Q_beta_W ..." } | ConvertTo-Json
Invoke-RestMethod -Uri "http://localhost:3142/v1/gql" -Method POST `
                  -ContentType "application/json" -Body $body
```

For production (`gigi-stream.fly.dev`), add the `X-API-Key` header (the key the engine team uses for the production deploy):
```
X-API-Key: <key>
```
Note: header is `X-API-Key`, NOT `Authorization: Bearer ...`. The Bearer pattern returns `Invalid token`.

---

## Rebuild from source (if your binary predates a substrate fix)

```powershell
cd C:\Users\nurdm\OneDrive\Documents\gigi
cargo build --release --features halcyon --bin gigi-stream
```

The binary lands at `target/release/gigi-stream.exe`. The VI.2 verb went live at commit `777c7ad`; VI.3 (GC battery + 2 verb correctness patches) at `1d2bd39`; VI.4 (SHAM dispatch) at `3f4b63b`; VI.5 (gold fixture) at `90d1697`; VI.2b (HTTP dispatcher recognition for LoopDecl + LoopTransport) pending after AURORA Phase 2 workflow lands.

To verify the binary has the VI.2b HTTP dispatcher fix landed:
```powershell
# Fire LOOP_TRANSPORT against a thermalized U_lt / E_lt setup. The
# response should be a non-empty Rows envelope (the 10-field shape
# above). If it's just {"status":"ok"}, the binary is pre-VI.2b
# and needs rebuilding from a commit that includes VI.2b.
```

---

## Three things Halcyon discovered the hard way during the 2026-06-21 session (so future-you doesn't repeat them)

1. **`LOOP` declaration grammar is purely spatial.** It's `LOOP <name> ON <lattice> FACE <n>` (or `EDGES (...)`) — no `DECLARE` prefix, no `CONTROL_MANIFOLD` clause, no `PATH` / `T_LOOP` / `SEGMENTS` clauses. The (Q, β_W) parameter-space ramp lives **inside** `LOOP_TRANSPORT`'s own clause list (`CONTROL_MANIFOLD (Q, BETA_WILSON)`), not in the LOOP declaration. v3.1.3 conceptually merged them as "γ_unit"; gigi's parser splits the spatial loop (which face / vertex path on the buckyball) from the parameter-space ramp.

2. **`U_lt` / `E_lt` are mandatory names.** The executor arm at `parser.rs:10338` hardcodes these names. If you declare your gauge field as `halcyon_canonical_buckyball` (or any other name), `LOOP_TRANSPORT` won't find it. The naming is documented in the gate doc §Per-verb specs as the "convention bound to the lattice."

3. **Seed bracket is range syntax, not explicit list.** `SEEDS [20260616..20260623]` parses; `SEEDS [20260616, 20260617, ...]` does not. The bracket is inclusive on both ends (8 seeds = 20260623 - 20260616 + 1).

---

## What's logged in the impl chain for chain-of-custody

- v3.1.3 SPEC: `davis-wilson-lattice/inertia_damping/HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3.md` at commit `44c70b1`, Zenodo DOI `10.5281/zenodo.20785681`, git-tagged `spec-v3.1.3-zenodo-20785681`
- gigi VI gate doc: `theory/halcyon/HALCYON_PART_VI_GATES.md` at commit `9a73dc0` (Bee read-approved)
- gigi VI impl log: `theory/halcyon/HALCYON_PART_VI_IMPLEMENTATION_LOG.md`
- gigi VI gold fixture: `tests/fixtures/halcyon/part_vi/loop_transport_canonical.json` at commit `90d1697`
- gigi VI.2b HTTP dispatch fix: this guide is at the same commit (pending) — references the regression at `tests/halcyon_part_vi_b_http_dispatch.rs`
