# Handoff to Marcella team — L1–L7 shipped, L8 ready for flip protocol

**From.** Bee Davis + GIGI team (Claude collab pair).
**To.** Marcella team.
**Date.** 2026-05-24.
**Re.** Kähler upgrade — engineering layer complete, your turn to
empirically validate on Marcella v3's actual embedding manifold.

---

## TL;DR

L1 through L7 of the Kähler upgrade are shipped on GIGI's `main`
branch behind the `kahler` Cargo feature gate. **902 tests passing
with the feature on, 720 still passing with it off** (bit-identical
optionality contract holds). **15 Python validation tests passing**
across 4 suites (v1–v4). Cross-team contract tests in
`tests/kahler_*_marcella_contract.rs` enforce every API shape you
deserialize.

The substrate spec, real-data fingerprint, and §E.5 pre-flight
templates are now in place. We're at gate (1) of the flip protocol;
the next three gates are yours.

---

## What shipped (commits)

| Commit | Layer | Summary |
|---|---|---|
| `f874ac9` | L1 | Kähler structure as optional schema metadata |
| `bd10740` | L2 | Dual adjacency + commutativity classifier |
| `3cc856b` | L1.5 + L1.5.3 | B-perturbed transport end-to-end |
| `b398a1e` | L3 | Jacobi cardinality + cached spectral gap + Marcella surface |
| `11f7645` | L4 | Kähler curvature decomposition |
| `99f2f26` | L5 | Hadamard substructure detection |
| `e563339` | L6 | Hodge complex + Morse compression |
| `6dba318` | L7 | Quantization + Frobenius + Toeplitz + Riemann-Roch |

Plus the docs commit at the head of the catalog history:
- `fc025e7` — catalog + validation suite + implementation plan
- `5069d28` — cross-team correspondence (your letter + my reply)

---

## The cross-team artifacts that matter

1. **`marcella_substrate.md`** (this folder, just landed). The
   single source of truth for which GIGI APIs your runtime
   consumes. **Every API listed there is gated by a contract
   test** that fails BEFORE your deserialization breaks. If you
   only read one doc, read this one.

2. **`preflight/`** (this folder, just landed). Three Python
   templates corresponding to catalog §E.5 checks 1–3:
   - `hadamard_check.py` — sample 1000 Jacobi fields, verify
     non-vanishing.
   - `closedness_check.py` — verify `dB = 0` on Marcella's B.
   - `holo_sectional_check.py` — verify K_H regime
     (Hadamard / FS-like / mixed / high-curvature).

   Each template includes synthetic positive + negative controls
   (ℍ², S², flat — for Hadamard; constant B, 3D non-closed — for
   closedness; flat, disc-uniform, mixed — for K_H). All controls
   pass on our side; you implement the `sample_*` accessor that
   talks to your runtime, then call the check on your actual
   manifold.

3. **`tests/kahler_*_real_data_smoke.rs`** in the GIGI repo (one
   per layer). These exercise the surface on the 20-record sensor
   dataset and are the fastest way to see what each API
   *actually returns* on representative data. Fingerprint summary
   is in `marcella_substrate.md §Real-data smoke fingerprint`.

---

## Flip protocol — where we are

Per the 2026-05-24 reply Q7, `KAHLER_ENABLED` defaults from `false`
to `true` when **all four gates clear**:

| Gate | Owner | Status |
|---|---|---|
| 1. L7 shipped | GIGI team | ✅ as of `6dba318` |
| 2. §E.5 pre-flight passes on Marcella's actual manifold | Marcella team | ⏸ pending |
| 3. Sheets-bundle contract tests pass for ≥ 1 week without regression | Bee | ⏸ pending |
| 4. v3 paper reviewed by ≥ 1 external geometer (ICDG circle preferred) | Bee + Marcella | ⏸ pending |

GIGI proposes the flip when all four clear. **You have veto with
a stated reason** — the principle is "you're the gatekeeper of
the thing you consume." A postponement is data; usually means
a pre-flight check is borderline and we should investigate
rather than override.

Co-announce when it happens.

---

## Answered carryovers from your 7 questions

Quick reference (full text in `REPLY_TO_MARCELLA_TEAM_2026-05-24.md`
and `REPLY_TO_CONSUMPTION_DRAFT_2026-05-24.md`):

| Q | Topic | Disposition in L1–L7 |
|---|---|---|
| Q1 | Transport result field shape | Shipped per consumption draft v2 §2; contract test `kahler_transport_marcella_contract.rs` |
| Q2 | Spectral-gap surfacing | Both surfaces shipped: in-response field on `/curvature` + dedicated `/v1/bundles/<name>/spectral_gap` endpoint |
| Q3 | BSource enum names | Shipped as `Bundle \| Override \| None \| FallbackNonClosed` |
| Q4 | Two surfaces for spectral gap | Both shipped (see Q2) |
| Q5 | Region-partition response for Frobenius | Variant `QuantumCohomology::NonToy` returns explicit refusal with `reason`; full region-map struct sketched but not yet shipped — file an issue when you need it |
| Q6 | Toeplitz `ℏ` safe bound | Shipped: `ℏ ≥ 4/embedding_dim` enforced; opt-in path returns `truncation_dominates_correction` flag |
| Q7 | Flip protocol | Documented above; gates 2–4 are yours |

---

## What's still TBD on our side (after your pre-flight)

Bee has chip-tasks queued for two production-readiness items:

1. **`morse_compress` face-count cap.** Current implementation is
   O(V³) on dense field-index graphs. Hits 175s on the 20-record
   sensor bundle (F=1140); won't scale to your 10⁶+ substrate
   without a cap + degraded-mode flag. Will land before flip gate 2.

2. **High-dim Chern compression.** Current `LineBundle::from_constant_two_form`
   only handles dim=2 (the `IntegralityError::DimensionUnsupported
   { dim }` variant is the explicit refusal). Multi-dim path
   lands when you signal demand — your embedding manifold's actual
   dimensionality drives the priority.

If your pre-flight surfaces other rough edges, file them and we'll
prioritize.

---

## How to run the pre-flight templates

From the GIGI repo root:

```bash
PYTHONIOENCODING=utf-8 python -X utf8 theory/kahler_upgrade/preflight/hadamard_check.py
PYTHONIOENCODING=utf-8 python -X utf8 theory/kahler_upgrade/preflight/closedness_check.py
PYTHONIOENCODING=utf-8 python -X utf8 theory/kahler_upgrade/preflight/holo_sectional_check.py
```

Each exits 0 on synthetic-control pass, 1 on fail. The `NEXT:`
prompt at the bottom of each output tells you exactly which
function to replace with your runtime's accessor.

To clear gate 2, the check must return the equivalent of:

| Pre-flight | Required output |
|---|---|
| `hadamard_check` | `(True, 0.0, [])` on 1000 sampled geodesics |
| `closedness_check` | `(True, < 1e-10, _)` on a representative point set |
| `holo_sectional_check` | `regime ∈ {"hadamard", "fs_like"}` for the regions you cite §1.4-§1.5 on |

Send the output back as a PR to this folder
(`preflight/marcella_v3_results.txt` or similar) when ready — that
becomes the audit trail for the flip.

---

## Bee's running ground rule (Joy, not fear)

If anything in the substrate doesn't match what you expected from
the consumption draft, **say so right away**. We've iterated on
this catalog four times already; a fifth pass costs nothing if it
catches a real misunderstanding. Don't sit on a doubt to save
face — that's how cross-team handoffs corrode.

I'd rather have a 2-page reply pointing at five borderline things
than a single "looks good, ship it" that flips into trouble on
prod data.

— bee + Claude (GIGI side)
