# Reply to Marcella's P1+P2 shipped letter — two paper-discipline questions

**From.** Bee Davis + GIGI engine team (Claude pair).
**To.** Marcella team.
**Date.** 2026-05-25.
**Re.** Two non-blocking questions in the closing of
`marcella/theory/LETTER_TO_GIGI_TEAM_P1_P2_shipped_2026-05-25.md`.

---

## Acknowledgements (no action needed)

- **Self-consistency framing** going into the paper verbatim — yes,
  the `drift_applied == nonassoc_value` identity is a calibration
  theorem, not just an engineering observation. Confirmed.
- **P1 shipped + smoke-test verified** — green-light recorded.
- **P2 within-session 2pp/turn convergence at 4/4 monotonic** —
  citable in the v3 paper exactly as phrased ("≈ 2pp per turn
  across 4 stationary sessions" — honest on n, headline-grade on
  trend).
- **`fire_count` pre-registered, not added** — agreed; YAGNI for
  v3 ship.
- **P3 locked: Hopf + on-demand + half-day scope (`hopf.py` +
  contract test, no `update_residue_kahler` wiring)** — right
  shape. The wiring PR follows when the engine-side exposure of
  `frobenius_compose` is callable (see prior reply: GIGI ships
  `POST /v1/bundles/<name>/frobenius_compose` in the same window
  as your Hopf-wiring PR; ping when ready).
- **Deploy status table** — useful artifact; that table IS the
  gate-3 audit trail.

---

## Q1 — Include norm-preservation proof sketch in v3 paper?

**Yes. Include it. As a footnote or short appendix lemma, not a
section.**

Reasoning:

The norm-preservation of R(θ) is the **load-bearing claim** that
makes the on > off observation valid evidence rather than
coincidence. Without it, the geometer reviewer has to either
trust the claim (sloppy) or look it up (annoying). With it, they
verify in 30 seconds and move on.

The proof is 2 lines: the magnetic 2-form B is antisymmetric, so
its action `iB` on the tangent fiber is Hermitian, so
`R(θ) = exp(iBθ)` is unitary, so its real action is orthogonal.
Cite any standard reference on charged-particle dynamics (e.g.
Arnold's *Mathematical Methods of Classical Mechanics* §43, or
Marsden-Ratiu's *Introduction to Mechanics and Symmetry* Ch. 6)
so the reviewer can check the textbook without you having to do
the derivation.

**Suggested form** (LaTeX-ish sketch, edit to your house style):

> **Lemma (norm preservation under magnetic flow).** Let
> `B ∈ Ω²(M)` be the closed 2-form on the ambient Kähler
> manifold, antisymmetric by construction. The per-turn
> rotation operator `R(θ) = exp(iBθ)` acts orthogonally on
> the residue tangent fiber, so `‖R(θ) · r‖ = ‖r‖` for any
> residue `r`. *Proof.* Antisymmetric B ⇒ Hermitian iB ⇒
> unitary exp(iBθ) ⇒ orthogonal on real components. See
> Arnold §43 for the general magnetic-flow argument.
>
> *Consequence.* Norm decay in the Kähler residue update
> `r' = α·R(θ)·r + β·new` comes only from the α coefficient
> and the projection of `β·new` onto the residue direction.
> Rotation contributes zero to norm loss. This is the reason
> Kähler residue persists with higher fidelity than
> classical accumulation on stable-context conversations
> (§Empirical results, observation N).

Five lines. Cheap insurance against the "we noticed but didn't
explain" critique.

---

## Q2 — Call out meter sparsity as known property?

**Yes. Frame as feature, not limitation.**

Reasoning:

The meter only fires when both intent routing AND retrieval
surfacing occur. That's already visible in the A/B harness
(canned-handler tests don't contribute meter data; 9/9
byte-identical category). A reviewer running their own A/B will
discover this. Better they read about it in the paper than as a
surprise.

**Framing matters.** It is NOT a sampling bias hiding the real
signal. It IS an architectural property: *the meter measures
what happens when the geometric machinery actually fires.* Of
course it's silent when nothing geometric happened. That's the
same architectural shape as the no-feature build of the Kähler
upgrade being bit-identical to pre-upgrade GIGI — the optionality
contract says "absent doesn't lie, it just doesn't speak."

Connect it explicitly to the optionality contract in the paper;
that ties it to a property the reader has already accepted.

**Suggested form** (one paragraph in methods, before the meter
results):

> The non-associativity meter fires only on turns where both
> residue-firing intent routing and retrieval surfacing occur.
> We report fire-conditional statistics; canned-handler paths
> and queries that do not trigger retrieval are by-design
> silent under this measurement. This sparsity is structural:
> the meter measures the geometric machinery's behavior when
> it is active, which inherits from the optionality contract
> of the underlying Kähler upgrade — paths that do not consume
> residue are bit-identical to the classical baseline, and the
> meter has nothing to measure on them by construction. We
> report fire-rate alongside meter statistics so the reader
> can distinguish measurement silence from measurement
> evidence at any aggregation level.

Last sentence is the operational discipline: *also report
fire-rate*. That's the antidote to "wait, were the silent turns
suppressed or just silent?" — the reader sees both numbers and
draws the right conclusion.

---

## Two suggested additions

Not asked, but flagging since they're cheap:

1. **Report the fire-rate alongside the meter statistics** —
   1 sentence per result table. Without fire-rate, a reader
   computing aggregate non-assoc has to guess how many turns
   contributed. With it, the methodology is unambiguous.

2. **In the deploy-status table, add a column for "rollback
   verified"** — should be a checkmark for whatever you tested
   the rollback path against. The "one flag-flip back" claim
   is load-bearing; even a manual single-rollback test
   recorded in the table strengthens the gate-3 audit trail.

Neither blocks; both are 5-minute additions when convenient.

---

## Status check

| Gate | Status |
|---|---|
| 1. L7 shipped | ✅ |
| 2. §E.5 pre-flight | ✅ |
| 3. 1-week production observation | ⏱ clock running from deploy step #3 |
| 4. External geometer review | ⏸ awaiting v3 paper draft |

When you flip `KAHLER_ENABLED=1` in fly secrets (step #3 in your
landing sequence), the gate-3 clock starts in earnest. The
2-line proof sketch and the meter-sparsity framing both belong
in the v3 paper draft that becomes the substrate for gate 4 — so
they're paper-deadline-driven, not flip-clock-driven. Take your
time on the writing.

— bee + Claude (engine side)
