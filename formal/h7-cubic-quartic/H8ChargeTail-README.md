# H8 charge tail: Lean scope

`H8ChargeTail.lean` checks five declarations under the pinned toolchain and Mathlib revision of this project.
None adds an axiom or admits a step.

| Declaration | What it checks |
| --- | --- |
| `gauge_rows` | With Y = c•X₁, the rows of circle minus gauge are 0, −(X₃ − c[X₁,X₂]) and X₂ − c[X₃,X₁]. This holds for any bracket that is linear in its second slot and antisymmetric. |
| `ruler` | \|m\|/√t ≤ s²/(2t) + m²/(2s²) for t, s > 0 |
| `zero_point_floor` | c ≤ P(E − P), c ≤ P(E − σ), σ ≤ E and c ≤ σ² imply E ≥ σ + c/σ |
| `large_charge_floor` | c ≤ P(E − P) and E ≥ 0 imply E ≥ 2√c |
| `t3_floor_closed_form` | 5√3/2 + (m²/12)/(5√3/2) = (225 + m²)/(30√3) |

**Outside the formal boundary.** These are written in
[the proof note](../../docs/research/h8-charge-tail-proof-2026-11-27.md) and exercised exactly by
`scripts/check_h8_charge_tail.py`:
- the polynomial identity t|∇W|² = ‖X − Γ_{Y*}‖² + t²‖X₁ − (g/√t)[X₂,X₃]‖²;
- the projection definition of |X_hor|;
- the pointwise Cauchy–Schwarz steps;
- the integration by parts ½‖aψ‖² = E − σ;
- the form-core argument.

The checker rejects four broken variants:

- antisymmetry removed from `gauge_rows`;
- the small-charge condition c ≤ σ² removed. The statement is then false: σ = 1, c = 4, P = 2 and E = 4
  satisfy the remaining hypotheses;
- the twisted bound c ≤ P(E − σ) removed;
- a doubled ruler.

Each must fail to compile for a mathematical reason, not a missing import.
