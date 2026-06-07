# GIGI specs — index

Single source of truth for "what specs exist and where they live." Files stay in their topic-specific subdirectories (specs live next to their catalogs and validation tests, which keeps history tight); this index is the navigation layer.

When you write a new spec, add a row here. When you ship a spec to a feature flag, mark it. When the feature graduates off its flag, mark that too.

---

## Active specs

| Spec | Topic | Status | Subject docs alongside |
|---|---|---|---|
| [PATTERN_HUNT_SPEC_v0.1](scj/PATTERN_HUNT_SPEC_v0.1.md) | SCJ Pattern Hunt v0.1 — DEFINE / HUNT / EXCLUDING IN, min/max in WEIGHT, _score-last on wire | **Live on prod** (feature flag: `patterns`) | `scj/REPLY_TO_REPLY_4_2026-06-09_ASK_G_ANSWERS.md`, `scj/SHIP_REPORT_2026-06-09_PATTERNS_LIVE.md` |
| [PATTERN_VERDICT_SPEC_v0.2](patterns/SPEC_v0.2_VERDICT.md) | Patterns v0.2 — five SUDOKU-lift primitives: K_P curvature, preflight, verdict trichotomy, REPAIR menu, EXPLAIN | **Spec + math validated**, TDD pending | `patterns/IMPLEMENTATION_PLAN.md`, `patterns/validation_tests.py`, `patterns/README.md` |
| [SUDOKU_PRIMITIVE_SPEC](kahler_upgrade/SUDOKU_PRIMITIVE_SPEC.md) | 13th brain primitive — constraint curvature K_c, holonomy preflight, Γ trichotomy, RelaxationOption menu | **Live** (`kahler` flag) | `brain_primitives/catalog.md` |
| [SHARDING_SPEC](poincare_to_sharding/SHARDING_SPEC.md) | Sharded bundles — Atlas, per-chart execution, tournament merge, refusal regimes | **Live** (`sharded` flag) | `poincare_to_sharding/validation/`, `poincare_to_sharding/` |
| [ATOMIC_SHEAF_COMMIT_SPEC](transactions/ATOMIC_SHEAF_COMMIT_SPEC.md) | ACID transactions — 2PC + MVCC + deadlock detection + geometric coherence snapshots | **Live** (`transactions` flag) | `transactions/` |
| [PHASE_2_DIM_LIFT_SPEC](imagine/PHASE_2_DIM_LIFT_SPEC.md) | IMAGINE Phase 2 — dim lift beyond n=2 | **Spec only**, implementation pending | `imagine/` |
| [GIGI_ENCRYPT_v0.4_SPRINT_SPEC](encryption/GIGI_ENCRYPT_v0.4_SPRINT_SPEC.md) | Per-field encryption — OPAQUE / INDEXED gauges | **Live** (always-on) | `encryption/`, `encryption/paper_geometric_encryption_v0.1.tex` |
| [SPEC_FLOOR_TUNABLE_S1_WAVE2](kahler_upgrade/SPEC_FLOOR_TUNABLE_S1_WAVE2.md) | SUDOKU σ² floor tunability | **Live** | `kahler_upgrade/IMPLEMENTATION_PLAN.md` |

---

## Implementation plans

| Plan | Covers | Status |
|---|---|---|
| [kahler_upgrade/IMPLEMENTATION_PLAN](kahler_upgrade/IMPLEMENTATION_PLAN.md) | L1–L8 Kähler upgrade phase gates | Complete |
| [patterns/IMPLEMENTATION_PLAN](patterns/IMPLEMENTATION_PLAN.md) | v0.2 verdict primitives TDD sprint plan | Active |

---

## Catalogs (math + validation, not feature specs)

| Catalog | Scope | Items shipped |
|---|---|---|
| [kahler_upgrade/catalog](kahler_upgrade/catalog.md) | Kähler L1–L8 math + 16/21 items shipped | 16/21 |
| [brain_primitives/catalog](brain_primitives/catalog.md) | 12 brain primitives + SUDOKU as 13th | 13/13 |
| [post_kahler_directions/catalog](post_kahler_directions/catalog.md) | 9 next-direction math programs (Sasaki, Wasserstein, Tropical, etc.) | 0/9 shipped (30/30 math validated) |

---

## Lineage / correspondence

These aren't specs but they shape the spec docs and live in the same tree:

- `scj/` — SCJ correspondence (11 letters, 2026-06-04 → 2026-06-09)
- `kahler_upgrade/HANDOFF_TO_MARCELLA_*` — Marcella correspondence
- `brain_primitives/` — handoff letters + sprint reports

---

## Conventions

- **Spec naming**: `<TOPIC>_SPEC.md` or `<TOPIC>_SPEC_v<X.Y>.md` for versioned. Keep the version suffix when shipping a follow-up that doesn't supersede the original.
- **Location**: spec lives next to its catalog + validation tests, in `theory/<topic>/`. Don't centralize the files — centralize the index (this doc).
- **Status flags**: "Live on prod" (deployed), "Live (`<flag>` flag)" (shipped behind a feature flag), "Spec + math validated" (math green, code pending), "Spec only" (math not validated yet).
- **When a v0.1 ships and a v0.2 starts**: leave v0.1 in place, write v0.2 alongside, update this index. v0.1 becomes the historical record once v0.2 supersedes it.
