# Test 2, family 2: SU(3) with the Maurer–Cartan background (20 November 2026; 2b pending)

**Protocol.** Fixed before any SU(3) code: [`skeleton-tests-preregistration-2026-11-20.md`](skeleton-tests-preregistration-2026-11-20.md),
"Protocol for family 2", and the add-on with prediction G.

**Setup.**
- Background: A = ½θ.
- Squashings: 2a stretches the U(2) fibre (t on λ1, λ2, λ3, λ8); 2b stretches the hypercharge circle (t on λ8).
- Nine sectors (p, q) with p + q ≤ 4.
- Grid: t = 0.05, …, 10.
- The extremal state is the unique state of maximal charge p + q + 4 under the circle of diag(1, 0, −1): highest
  weight ⊗ E_θ ⊗ E_θ.

**Code controls**, all passed before any family data (`su3_test2_controls.json`):
- K1: the general builder reproduces the SU(2) builder to 3.6 × 10⁻¹⁵.
- K2: the Berger control gives T_all = [2.70, 10.00].
- K3: the SU(3) representations, the Yang–Mills condition at t = 1, d*d ≥ 0, and the conjugate sectors all check
  out.

## Family 2a (U(2) fibre): class N

Receipt `su3_test2_family2a.json`.

- No per-point check stopped the run. At every point and sector:
  - Q commutes with H and GG* to 10⁻¹⁰;
  - the top-charge eigenspace is one-dimensional;
  - the extremal state is an exact, gauge-orthogonal eigenvector.
- **The extremal state is the floor of none of the nine sectors, at any of the 200 grid points.** T_all is empty, so
  the class is **N**.
- By the pre-registered rule, this is a second independent family that **counts against** the scoped skeleton
  hypothesis. (Family 1, the squashed meron, was the first.)
- **Prediction G** (a field yields along its twist when its field strength lines up with the twist's spin) is
  **untested in 2a**: T_all is empty.

## Descriptive: the field-strength share for A = ½θ

Script `scripts/ricci_su3_gyromagnetic_share.py`, receipt `su3_gyromagnetic_share.json`. The script first
reproduced the SU(2) Berger split exactly.

| t | 2a Ricci | 2a field strength | 2b Ricci | 2b field strength |
| --- | --- | --- | --- | --- |
| 0.5 | +1.125 | +0.5 | +0.938 | +0.5 |
| 1 | +0.75 | +0.5 | +0.75 | +0.5 |
| 3 | −0.75 | +0.5 | 0 | +0.5 |
| 10 | −6.0 | +0.5 | −2.625 | +0.5 |

- The field-strength share of the extremal frame is **+0.5 at every t in both families**. The Maurer–Cartan field's
  strength never lines up with the twist's spin; it opposes it, as on the squashed meron.
- In 2a the twist is never the floor. That is consistent with the reading but, as registered, it does not test G.

## Family 2b: rerunning

The first run crashed after about 8 hours on a LAPACK SVD non-convergence (gesdd) while building the slice basis.
This was a numerical failure, not a result.
- The fix: the slice basis falls back to the gesvd driver on that error. It computes the same quantity, ker G*, and
  each point records whether the fallback was used.
- Since s_F > 0 everywhere in 2b, **any floor point in 2b would count against G as a general rule.**
