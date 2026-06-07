# theory/patterns — substrate-level Patterns surface

The Patterns substrate is the **named, weighted, anti-joined, verdict-bearing** primitive that operators use to express "find me rows that match this shape, ranked, with near-miss receipts."

## Lineage

- **2026-06-04**: SCJ writes their first letter asking whether the substrate can express their `risk_score.py` heuristic
- **2026-06-06**: PATTERN_HUNT_SPEC v0.1 spec authored (lives at `theory/scj/PATTERN_HUNT_SPEC_v0.1.md` — kept there because the SCJ correspondence is what made it real)
- **2026-06-08**: Ask G — Patterns spec circulated to SCJ
- **2026-06-09**: v0.1 shipped, deployed to prod, 49 pattern tests + 1064 total green, 74/74 live probe checks
- **2026-06-09 (later)**: v0.2 spec authored — this directory

## What lives here

| File | What it covers |
|---|---|
| [`SPEC_v0.2_VERDICT.md`](SPEC_v0.2_VERDICT.md) | Full v0.2 spec — the five SUDOKU-lift primitives, GQL surface, HTTP surface, TDD phase gates, open questions |
| [`validation_tests.py`](validation_tests.py) | Python math validation for every primitive — green-on-toy-data proof before any Rust |
| [`IMPLEMENTATION_PLAN.md`](IMPLEMENTATION_PLAN.md) | TDD sprint plan with phase gates (PE → PP → VT → PR → K_P) |

## The shape in one paragraph

v0.1 answers "what matches?" — return ranked rows. v0.2 answers four more questions from the same substrate machinery: "how tight is this pattern?" (K_P curvature), "can it even match here?" (preflight), "sat/unsat/near-miss?" (verdict trichotomy), "what's the cheapest flip to make a near-miss match?" (repair menu), and "why was this row scored 7.3?" (explain). All five lift directly from the SUDOKU machinery that shipped in `src/geometry/sudoku.rs` (W6 spec). No new geometry, just wiring.

## The principle

Constraints are curvature. Patterns are constraints. Therefore the substrate already knows how to:

- measure pattern concentration (variance of neighbor-match-ratio = K_P)
- preflight pattern contradiction (holonomy on the constraint graph)
- classify sat/unsat/near-miss (Γ trichotomy)
- enumerate minimum-cost repair sequences (relaxation menu)
- decompose a score into per-term contributions (energy descent)

The substrate is domain-blind. The same machinery serves vuln-hunt, fraud monitoring, education, hiring, compliance, PRISM, ICARUS. Domain-swap discipline (4 parallel test variants per primitive) is the proof.

## Why we organized it this way

Files live in `theory/<topic>/` (next to their catalog + validation tests), not in `theory/specs/`. Centralizing files breaks the "spec + math + sprint + correspondence all in one place" invariant. Instead we centralize the **index** at `theory/SPECS_INDEX.md`, which is the navigation layer.

`v0.1` stays at `theory/scj/PATTERN_HUNT_SPEC_v0.1.md` because the SCJ correspondence is the historical record of how v0.1 was negotiated. `v0.2` lives here because v0.2 is about the substrate primitive itself, not any specific consumer.
