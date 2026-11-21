# The alignment test (Test 4): results (21 November 2026)

The protocol was fixed before any code or data:
[`alignment-test-preregistration-2026-11-21.md`](alignment-test-preregistration-2026-11-21.md).

**Claim under test:** "a field yields along its twist when its own field strength lines up with the twist's spin"
(the direction governs her). It is tested on aligned backgrounds that are not the Berger field: A = sθ on the
unstretched group, s = 1.05 to 3.00.

**Controls** (`alignment_test_controls.json`), all passed before any family data:
- C1: at s = 1 the field is flat (energy 0); at s = ½ it is at rest (gradient ≤ 3 × 10⁻¹⁰).
- C2: on SU(2) at s = ½, the published meron spectrum −2, 1, 6, 7, 10 is reproduced to 10⁻¹⁴.
- C3: Test 2's K1 and K3 rerun clean.

## Family 4b, SU(2): class N, so the prediction fails

Receipt `alignment_test_4b.json`.

- **Precondition A2 held:** the field is aligned at every grid point. s_F runs from −0.42 at s = 1.05 to −48 at s = 3.
- **No check stopped the run.**
- **The twist is the floor of the j = 0 sector for s ≥ 1.5, and of no sector j ≥ 1 at any s.** S_all is empty, so the
  class is N.
- **Alignment does not pull her down in the higher sectors.** Her gap above the floor grows with the alignment:
  in j = 1, from 16.5 at s = 1.05 to 35.9 at s = 3. She sits at about rank 5 of 12 in j = 1 and rank 49 of 78 in
  j = 12 at s = 3.
- **By the pre-registered rule, this counts against "alignment suffices".** A field strength aligned with the
  twist's spin, however strong, does not on its own make her the floor.

## Exploratory (after the result, not pre-registered): direction is not enough; she must be singled out

On the aligned SU(2) backgrounds, the twist's frame β **is** in the lowest eigenspace of the curvature term
R₀ = Ric − 2[F, ·]. But that eigenspace is **five-dimensional**:

| s | lowest value of R₀, with multiplicity | ⟨β, R₀β⟩ |
| --- | --- | --- |
| 1.05 | 1.58 (×5) | 1.58 |
| 1.5 | −4 (×5) | −4 |
| 2 | −14 (×5) | −14 |
| 3 | −46 (×5) | −46 |

The round group's extra symmetry makes five frame states share the deepest curvature. In sectors j ≥ 1, the
others' kinetic cost is lower, so they win. On the Berger field the stretch breaks that degeneracy: the lowest
eigenspace of R₀ is β and its conjugate alone (12 − 8t, multiplicity two), and the twist is the floor.

**Refined reading (a hypothesis, one positive example):** she is the floor when the field points at her *and only
at her*, when her frame spans, with its conjugate, the lowest eigenspace of the curvature term on its own.
Alignment is necessary-looking (Test 2) but not sufficient (this test). Being singled out may be the missing part.

**A test that could fail.** SU(2) with A = sθ on the stretched metrics (t, 1, 1): alignment from s, singling-out
from t. Register in advance where the twist must be the floor (where β alone spans the lowest eigenspace of R₀,
with enough margin) and where it must not.

## Family 4a, SU(3): class N

Receipt `alignment_test_4a.json`.

- Aligned at every grid point (precondition A2 held). No check stopped the run.
- The twist is the floor of the trivial sector (0,0) for s ≥ 1.5 and of no other sector at any s, so S_all is empty
  and the class is **N**.
- This is the same pattern as SU(2) (family 4b).
- **Both families are N, so "alignment suffices" is counted against twice.**
