# Where we are (19 November 2026, evening)

One page, to find the plot again. Details live in `docs/research/status-2026-11-19.md` and on
each branch; this note is the map.

## The goal

The Davis–Wilson lattice correspondence needs a discrete Yang–Mills force, computed from edge data
alone, that is

1. **consistent**: it converges to the continuum force;
2. **stable**: no runaway modes;
3. **variational**: it is the gradient of an energy (my decision of 19 November);
4. **faithful in time**: its flow follows the continuum flow.

## Where the force stands

| | consistent | stable | variational | faithful in time |
| --- | --- | --- | --- | --- |
| Whitney (native) | no (26% floor) | yes | yes | — |
| two-ring Petrov–Galerkin (*Reading the Curve*) | **yes** | **yes** | **no** | **yes** (SU(2), n ≤ 3, one window) |
| two-ring variational (energy of the recovery) | no (diverges) | yes | yes | — |
| V1 constrained variational (today) | not yet (61.6% at n = 2) | yes | yes | — |

The two-ring force is published and settles 1, 2 and 4. **Item 3 is the whole open problem.**

## The critical path: a variational force (`feat/variational-force`)

- **What we learned.** The variational force fails because the recovery responds to rough test
  variations (the *test-variation* term). A constraint on the recovery's transpose removes that
  term almost entirely: 276% → 1.2% at n = 2. That part works.
- **What blocks it.** Every way we have tried to satisfy that constraint makes the recovery respond
  more strongly to data, and the *reconstruction* term takes over: 61.6% at n = 2 (V1). Minimising
  the operator's stiffness instead (V1b) did not help at n = 1 (485%); V1b at n = 2 is running.
- **Honest assessment.** We have been trying objectives; the next step should be analysis, not
  another variant. The likely fix: the change must not alter how the recovery reads *smooth* data at
  all, only how it reads rough data. That means extending the row condition (today: exact on closed
  linear fields) to the next order (closed quadratic fields), so the reconstruction of smooth data
  stays second order by construction, while the column condition removes the test-variation term.
  That is a design question to settle on paper first.

## Side quests (useful, not on the critical path)

| branch | question | state | why it matters |
| --- | --- | --- | --- |
| `feat/s2d-n3-spectrum` | is the SU(2) two-ring force stable at n = 3? | native done; two-ring running | closes a stated limitation of *Reading the Curve* |
| `feat/s3c-long-window` | does native's stiff error matter on a 5× longer window? | native: yes (order 1.32 < 1.38); two-ring running | explains the native result in *Reading the Curve* |
| `feat/s3d-startup` (S3d, S3e) | why does the Cayley step fail at N = 4? | S3e running; a second-order damping start-up halves native's first-step gap | diagnostic only: N = 8 already passes every gate |
| `feat/s4-strong-field` | does the chord–tangent blend matter for strong fields? | done | discretization error dominates; the blend needs the axis chord |

**Recommendation:** let the four running jobs finish and write them up, then start no new side
experiments. Put the effort into the variational design (the quadratic-exact constrained recovery)
before running anything else on it.

## Decisions I still have to make

1. Do I pause new side experiments and focus on item 3?
2. Is the next variational step a design note (quadratic-exact rows plus the column
   condition), reviewed before code?
3. When the running jobs land: fold S2d (n = 3 stability) and S3c into *Reading the Curve*'s next
   version?
