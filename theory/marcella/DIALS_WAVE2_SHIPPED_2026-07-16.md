# Marcella dials — shipped 2026-07-16 (wave 2 of 3)

Asks 3 + 4 from `GEODESIC_LOOM_PLAN.md` ("gigi-side asks (UPDATED)", signed
Hallie), implemented TDD (RED → GREEN per ask) on top of main @ 6b7a22d, plus
one review-lens follow-up commit. Wave 3 (WAL/snapshot durability) deliberately
untouched and named next.

## Design

### Ask 4 — locus + vector-only statistics on horizon/capacity

- **New unconditional lib module `src/dials.rs`** owns dial report-building;
  the `gigi_stream.rs` handlers are thin glue. `curvature.rs` diff EMPTY —
  zero forked math. Visibility bumps: 2 one-line `pub(crate)` changes
  (`parser::expand_field_range`, `geometry::sample_transport::fiber_d_sq`).
- **Params (both GET dials):** `fields=<spec|Vector-field>` (wave-1 `lo..hi`
  range sugar, comma lists, or exactly one `Value::Vector` fiber name,
  per-component), `locus=<field>=<value>`, `k=<n>` (requires locus, default
  64). Composition: `fields` alone = whole bundle vector-scoped; `locus`
  alone = kNN neighborhood over all numeric scalar fibers; both = the loom's
  real need.
- **Same formulas, scoped inputs:** scoped statistics materialize a transient
  in-memory `BundleStore` (per-component columns through `BundleStore::insert`'s
  own Welford — the identical accumulation the whole-bundle statistics were
  built by), then K / l_c / s_max / λ₁ / λ-budget come from the SAME public
  fns (`scalar_curvature`, `horizon_with` with default estimator config,
  `capacity`, `confidence`, `lambda_budget_for_bundle`, `spectral_gap`).
- **Precedence: `estimator=fixed` > `locus`/`fields` > default.** The fixed
  estimator is Marcella's production escape hatch; when supplied, scoping
  params are ignored entirely and the response is byte-identical to
  fixed-alone.
- **Wire fence (two belts):** absent params → the exact pre-change code path.
  Belt 1: in-process byte-identity replica in `tests/locus_dials.rs`. Belt 2:
  cross-process goldens at `tests/fixtures/dials/*.json` (floats 1e-9-relative
  due to HashMap-order summation). Scoped responses append `lambda_budget` +
  `scope{…}`, both `skip_serializing_if`-absent on the default wire.
- Scoped dials work on mmap/overlay stores (population collected through
  `BundleRef::records()`).

### Ask 3 — WINDOWED_COHERENCE one-shot

- **`POST /v1/bundles/{name}/windowed_coherence`** with
  `{path, key_field, window, fiber, threshold?}` composes server-side what
  the laminar gate previously round-tripped per segment (GQL
  TRANSPORT_ROTATION + POST /local_holonomy of dim×dim frames).
- **θ per segment = arccos(clamp(cos_sim(v_i, v_{i+1}), −1, 1))** — the
  PINNED DEVIATION from the caller-supplied-angle verb (the one-shot has no
  caller angle; it reads the minimal rotation off the segment data).
  Zero-norm endpoints transport as identity. The Rodrigues construction moved
  VERBATIM to `dials::transport_rotation_matrix`; the GQL verb now delegates
  to the same fn body (parity anchor A3-4).
- **Windows:** cumulative frames R_acc[i+1] = R_seg(i)·R_acc[i]; per window
  `curvature::local_holonomy(R_acc[s+w−1], R_acc[s])` → R_window =
  R_seg(s+w−2)···R_seg(s), defect = ‖R_window − I‖_F, coherence =
  1 − defect/(2√dim); laminar = coherence ≥ threshold. n_windows =
  len(path) − window + 1, stride 1.
- **Threshold default 0.91** = Marcella's `COHERENCE_CONFIDENT`
  (`fiber_lm/voice_math/coherence_forecast.py:33`), valid override range
  (0, 1]. Her plan's step-3 prose says "laminar (>0.9)" — the stricter 0.91
  is the pinned default, overridable per request.
- **Memory honesty:** scoped paths collect `store.records()` per call (fine
  at v2 scale); windowed frames are O(len(path))·dim² — keep paths to loom
  scale (tens of records).

## Anchor numbers

### A4-1 census reproduction (fixture mirroring v2's pathology)

| quantity | whole-bundle (polluted) | scoped `fields=v0..v15` |
|---|---|---|
| l_c (Welford fallback) | 4,237,986,115.57 | 0.05 |
| λ-budget | 1.0 (saturated) | −1599.0 |

The fixture's huge-variance ts-like scalar blows the whole-bundle Welford
radius exactly as `ingested_at` does on prod v2; scoping to the vector family
recovers the real dispersion. K itself stays healthy on both paths
(range²-normalized). Independently hand-derived in-suite: per-component
var = 0.0025 → K = 0.25, l_c = √0.0025 = 0.05, λ = 1 − 1/(0.25·0.05²) = −1599.

**λ-budget does NOT clamp negatives** (`curvature::lambda_budget` is
1 − τ/(K·D²), unbounded below by design; doc comment documents it). Large
negative λ = desaturated = far from horizon closure. Marcella's refuse branch
tests λ ≥ 0.95, so negatives read as "wide open" — documented, not "fixed".

### A3-2 known-rotation pins

2-dim circle fixture advancing φ = 0.05/record with one deliberate π/2 jump:
every window defect equals 2√2·|sin(α/2)| to 1e-9 (α = the window's composed
angle); laminar flips exactly at the constructed discontinuity at the 0.91
default. Two-call parity (A3-4): one-shot defect == TRANSPORT_ROTATION ∘
local_holonomy on the same segment data to 1e-12 (same fn bodies).

### Review follow-ups (commit 23bc0e4, tests + docs only)

- **Exact-antipodal pin:** v → −v derives θ = π but transports as IDENTITY
  (collinear ⇒ Rodrigues e2 degenerates): defect exactly 0.0 — pinned next to
  the near-antipodal case (defect → 2√2 ≈ 2.828), documenting the
  discontinuity. Inherited verb convention; measure-zero on real float
  embeddings.
- **Zero-norm pin:** a zero vector anywhere in the path → identity segments
  through both guards, defect exactly 0.0.
- **3-dim composition-order anchor:** three non-coplanar π/2 segments
  (x→y→z→x). Documented order C·B·A → tr = 1 → defect 2; reversal A·B·C →
  tr = −1 → defect 2√2. The one-shot lands on 2 to 1e-9 — the accumulation
  convention is now pinned by a wire observable (2-dim fixtures cannot do
  this: defect² = 2n − 2·tr is a class function and AB ~ BA, so two segments
  can never distinguish order; three can).
- **Dim-floor doc** on `DEFAULT_LAMINAR_THRESHOLD`: w=2 caps defect at 2√2
  (one plane rotation), so w=2 can be non-laminar at threshold t only when
  dim ≤ 2/(1−t)² (≈247 at 0.91). At dim=384 the w=2 coherence floor is
  1 − 2√2/(2√384) ≈ 0.9278 — unconditionally laminar. w−1 segments cap
  defect at 2√(2(w−1)); w=3 floor ≈ 0.898. Carried to the Marcella letter.
- **kNN tie-break note:** ascending cosine chord distance, ties by record
  iteration order — deterministic in-process; exact-distance ties on heap
  Hashed storage resolve by process-seeded HashMap order across restarts
  (measure-zero on real embeddings; mmap iteration stable).

## Gate table (merged main, pre-push)

| Gate | Result |
|---|---|
| `cargo check --features "kahler imagine sharded transactions patterns causal_states wish halcyon" --bin gigi-stream` | Finished, exit 0 |
| `cargo test --no-default-features --lib` | ok — 916 passed, 0 failed |
| `locus_dials` | ok — 25 passed |
| `windowed_coherence` | ok — 21 passed (18 + 3 review pins) |
| `explain_kappa` / `explain_batch` / `explain_errors` / `explain_mmap` / `explain_vector` | ok — 6 / 6 / 3 / 4 / 8 |
| halcyon: `spectral_gauge_basic` / `spectral_gauge_where_basic` / `spectral_full_basic` / `spectral_magnetic_basic` / `u1_flux_basic` | ok — 12 / 21 / 7 / 9 (308s, expected) / 12 |
| halcyon: `ingest_as_gauge_field_basic` / `ingest_gauge_vertex_basic` / `ingest_npz_key_basic` / `ingest_npz_dtype_basic` / `ingest_gql_bypass_basic` / `halcyon_l24_workflow_e2e` | ok — 1 / 18 / 8 / 5 / 4 / 4 |
| halcyon: `chern_class_basic` / `chern_class_bundle_target_basic` / `betti_pi1_basic` / `obstruction_basic` / `topology_verbs_gql_integration` | ok — 8 / 6 / 11 / 5 / 9 |
| kahler,imagine: `imagine_coherence_phase2` | ok — 10 passed |
| halcyon: `halcyon_part_iv_gold` / `aurora_lie_poisson_trait` | ok — 12 / 4 (+1 ignored) |
| lattice: `cubic_lattice` / `lattice_obc_basic` | ok — 7 / 10 |
| kahler: `davis_conjecture_lambda_brain_ridealong` | ok — 25 passed |
| patterns: `pattern_hunt_parser` | ok — 15 passed |
| core wall: `emit_csv` / `noop_notices` / `timestamp_ergonomics` / `gql_reference_truth` / `ingest_dir_gate` / `pathguard_escapes` / `ingest_executor` / `ingest_csv_basic` / `ingest_jsonl_basic` | ok — 1 / 3 / 6 / 1 / 11 / 3 / 4 / 14 / 6 |
| kahler --lib `dials::` unit parity pins (not in the locked gate list; run anyway) | ok — 2 passed |

Zero retries used; zero DLL-init flakes; zero assertion failures.

Grep gate: `git log --format="%B" 6b7a22d..HEAD | grep -c "Co-Authored-By"`
= 0 before every push.

## Deploy

- Pushed main 6b7a22d → 23bc0e4 (origin verified both directions before and
  after docs).
- `flyctl deploy -a gigi-stream` → release **v249**, image
  `deployment-01KXPW3B3QVBF2X1PMKEGTHK3C` (deploy log and `flyctl status`
  agree — the authoritative match). `/v1/health` ok, 5047 bundles /
  13,008,405 records after boot replay + substrate restore.

## Live probes (production, image deployment-01KXPW3B3QVBF2X1PMKEGTHK3C)

**D1 vs D2 — the headline receipt, same bundle, seconds apart:**

| | D1 `GET /horizon` (default) | D2 `GET /horizon?fields=v0..v383` |
|---|---|---|
| s_max | 1.196936100345132e-05 | **1749.0374034039978** |
| l_c | 4702629.127937915 (polluted) | **0.03211728135135101** |
| K | 0.017765944648704048 | 0.01780172659748959 |
| λ₁ | 0.0 | 0.0 (fiber scope carries no graph; Welford fallback) |
| estimator | welford_radius (fallback) | welford_radius (fallback) |
| lambda_budget | (absent — default wire) | **−54456.82861476302** |
| scope | (absent — default wire) | `{"fields":"v0..v383","n_records":9964,"n_fields":384}` |

- **D1** default horizon: byte-SHAPE identical to pre-deploy (same key set,
  no `lambda_budget`/`scope` leak), every value matching to 1e-9 — the
  fence holds on the live wire. PASS.
- **D2** `fields=v0..v383`: scoped receipt above — l_c 0.03-scale, s_max
  1e3-scale, λ large-negative, scope named. PASS.
- **D3** `estimator=fixed&fixed_value=1.0`: s_max 56.287465697634374
  (56.3-class), all fields matching pre-deploy to 1e-9 (max rel-delta
  5.0e-16 — last-ULP float jitter from process-seeded summation order, the
  same tolerance the cross-process goldens pin; the in-process fence is
  byte-exact in-suite). No scope/lambda leak under fixed. PASS.
- **D4** `locus=record_id=section:a_pale_jewel/ch_0019&fields=v0..v383&k=64`:
  200 with scope naming locus + k, n_records 64; s_max 811.0550551156762,
  l_c 0.02622728053558458, K 0.04701066589104941 (the 64-record
  neighborhood is locally more curved than the whole cloud — the per-prompt
  signal the census wanted), lambda_budget −30923.10034716543. PASS.
- **D5** windowed_coherence, 4 real keys (COVER FIRST 4), window=2, fiber
  v0..v383: 200, 3 windows, defects 1.3745683981203956 /
  1.2898399641567435 / 1.1546353680674903 (finite), coherence
  0.9649271750838562 / 0.967089064978815 / 0.9705388803048334, laminar
  all true, threshold_used 0.91, dim 384, lambda_budget
  0.9999999999974547 (whole-bundle ride-along, still saturated on v2 —
  documented in the letter). PASS. (All-laminar is the dim-384 w=2 floor
  demonstrated live: min possible w=2 coherence at dim 384 is 0.9278.)
- **D6** windowed_coherence with one missing key: HTTP 404
  `{"error":"windowed_coherence: no record at record_id=\"claim:definitely_missing_xyz/claim_9999\" in 'marcella_source_embeddings_bge_v2'"}` — typed, names the key. PASS.
- **D7** Marcella IMAGINE sanity (dim=4 body, 3 steps): 200, step-0
  coherence 1.0 exactly, endpoint_coherence 0.9724419621012066,
  refused=false. PASS.
- **D8** wave-1 regression, EXPLAIN missing key: HTTP 404 naming
  `record_id='definitely_missing_xyz'` and the bundle. PASS.

8/8 probes PASS.

## Pinned deviations + consumer caveats (carried to the letter)

1. **Derived θ vs caller-supplied angle:** the one-shot derives
   θ = arccos(clamp(cos_sim, −1, 1)) per segment; the GQL verb keeps its
   caller-supplied angle. Same Rodrigues fn body (verb delegates), parity
   pinned to 1e-12.
2. **Negative λ unclamped:** large-negative λ = desaturated (far from
   horizon). Marcella's refuse branch tests λ ≥ 0.95; documented, not
   "fixed".
3. **Threshold 0.91 vs plan's >0.9:** default pinned to her
   COHERENCE_CONFIDENT (0.91); the plan's literal 0.9 is one body field
   away.
4. **Dim-384 w=2 laminar floor:** w=2 cannot be non-laminar at 0.91 above
   dim ≈ 247 (defect cap 2√2 < required 3.527); w=3 floor 0.898. Letter
   advises w ≥ 3, higher threshold, or gating raw holonomy_defect.
5. **Exact-antipodal transports as identity** (collinear ⇒ e2 degenerate):
   defect 0, discontinuous vs near-antipodal (→ 2√2). Pinned by test;
   measure-zero on real embeddings.
6. **kNN exact-tie cross-process caveat** on heap hashed storage
   (process-seeded iteration); in-process deterministic; mmap stable.
7. **Memory:** scoped paths collect records() per call; windowed frames
   O(len(path))·dim² — loom-scale paths.

## Substrate drill

- Pre-deploy export: `.deploy-backups/2026-07-16-eve/claude_substrate_v0_backup.json`,
  **20 records**.
- The restart dropped the bundle (known snapshot-wedge fragility; live COVER
  404'd post-deploy). Restored via `CREATE BUNDLE claude_substrate_v0 (…)` +
  `POST /v1/bundles/claude_substrate_v0/import` `{"records":[…]}` → imported
  20/20, post-restore COVER 20 rows. **Count matches.**

## Commits (this wave, base 6b7a22d)

| commit | subject |
|---|---|
| 10e14f8 | tests(locus-dials): RED — A4-1..A4-4 census reproduction, defaults fence, locus correctness, fixed precedence |
| 16a6dce | impl(locus-dials): GREEN — fields/locus/k scoping on horizon+capacity through the same formula fns |
| 614ba5c | tests(windowed-coherence): RED — A3-1..A3-4 identity transport, known rotation, window arithmetic, two-call parity |
| 02351b7 | impl(windowed-coherence): GREEN — POST /v1/bundles/{b}/windowed_coherence one-shot |
| 23bc0e4 | fix(review): lens follow-ups — antipodal-identity and zero-norm edges pinned, 3-dim composition-order anchor, dim-floor + kNN tie-break docs |

(Anchor totals: locus_dials 25/25, windowed_coherence 21/21 — the four
cherry-picked TDD commits landed 18 there; the review commit adds 3 pins.)
