# The exact sectors, checked for gauge content: three physical growth directions, not five (19 November 2026)

A review of `su2-exact-sector-2026-11-19.md` and of the new stability paragraph and Table 9 of *Reading the
Curve*.

- **Script:** `scripts/ricci_su2_sector_gauge_check.py`. It reuses `ricci_su2_spin_sector.py`'s construction
  unchanged.
- **Receipt:** `research/su2-left-invariant-sector-20261119/gauge_check.json`.
- **Kind:** an exact finite-matrix computation, not a lattice run.

## Why it was worth checking

The Berger field is not at rest, so gauge directions are not zero modes of the Hessian: they are tilted. Inside a
sector they can also *mix* with physical directions. The note classifies negative eigenvalues by pattern:

- the j + 3 per sector are called gauge-type;
- the 5 in the left-invariant sector are called physical.

The left-invariant sector itself contains gauge directions. A constant gauge parameter ξ ∈ su(2) gives
b_i = [M_i, ξ], which is left-invariant: **3 of the 9 directions are gauge**. So the j = 0 eigenvalues need the
same check.

**Method.** In sector j, the gauge directions are b_i = ê_i ξ + [M_i, ξ], with ξ ∈ V_j ⊗ su(2), giving 3(j+1) of
them. For each eigenvector of the full Hessian, the overlap with that span is computed. The Hessian is also
restricted to the gauge-orthogonal (Coulomb) slice.

## Result

| sector | full Hessian negatives | their gauge overlap | Coulomb-slice spectrum (lowest) | slice negatives |
| --- | --- | --- | --- | --- |
| **spin 0** | −14/3 (×2), −2.558 (×2), −1.088 | **0, 0, 0.405, 0.405, 0** | **−14/3 (×2), 16 − 2√73, 22/3 (×2), 16 + 2√73** | **3** |
| spin 6 | 15, from −2.346 to −0.963 | 0.999 to 1.000 | 154/3 = 51.333 (×2), 59.504 (×2), 67.818 (×2) | 0 |
| spin 10 | 23, from −1.729 to −0.624 | 1.000 | 142 (×2), 155.42 (×2), 173.41 (×2) | 0 |
| spin 12 | 27 | 1.000 | 610/3 = 203.33 (×2), 219.40 (×2) | 0 |

In the j = 0 sector, the Hessian on the 3 gauge directions has Rayleigh values **8, 12, 12**. Specifically:

- **8 is a pure gauge direction:** its eigenvector lies entirely in the gauge span.
- **The pairs −2.558 and 21.891 are mixtures:** 40.5% and 59.5% gauge respectively.
- **On the slice, the mixed pairs become one physical pair, 22/3 (×2).** The value 22/3 is numerical to 10⁻¹⁰;
  it has not been derived symbolically.

## What stands and what changes

**Stands, now confirmed exactly:**

- **Every negative eigenvalue of spins 6, 10 and 12 is a gauge direction**, with overlap at least 0.999. The note's
  "gauge-type" reading is right.
- **The gap.** The physical (slice) spectrum is:
  - spin 0: −14/3 … 33.088;
  - spin 6: from **154/3 = 51.333**;
  - spin 10: from **142**;
  - spin 12: from **610/3**.

  The slice values sit slightly below the full-Hessian values the note quotes (59.504 against 59.52, and so on),
  because the full ones are mildly mixed.
- **All growth comes from the commutator.** The abelianised sector has no negative eigenvalue.
- **The lattice matches the full operator.** The lattice measures the full force Jacobian, which includes the
  gauge-tilted directions, so Table 9's comparison is the right comparison for what the lattice computes. Its
  spin-0 rows simply are not all physical.

**Changes:**

- **The physical growth directions are three, not five:** −14/3 (×2) and 16 − 2√73 = −1.088. These eigenvectors
  are exactly orthogonal to the gauge span, so this holds for any choice of slice.
- The −2.558 pair is 40% gauge. Its value depends on how gauge is split off, so it should not be called a
  physical growth direction.
- **In Table 9, "8" is a gauge direction,** and the −2.558 and 21.891 pairs are mixtures. The table can stay, as a
  comparison of the full operator with the lattice, if those rows are marked.

## Proposed wording for the paper (not applied; the paragraph is Bee's)

First bullet, replace with:

> *The field's growth directions are exactly three:* −14/3 (twice) and 16 − 2√73 = −1.088, each exactly orthogonal
> to the gauge directions. Of the other two negative values of the sector, the pair at −2.558 is 40% gauge. The
> left-invariant sector contains three gauge directions b_i = [M_i, ξ], on which the Hessian takes the values 8, 12
> and 12. On the gauge-orthogonal slice the sector reads −14/3 (twice), −1.088, 22/3 (twice) and 33.088. All of the
> growth comes from the field acting on itself: without the commutator, the same sector is 4/3 (six times) and 12
> (three times).

Table 9: add a column or footnote marking 8 as "gauge" and the ±√1345 pairs as "mixed (40% / 60% gauge)".

Third bullet: replace "the spin-6 sector begins at 51.33" by "the gauge-orthogonal spin-6 sector begins at
154/3 = 51.33". Optionally add spin 12's 610/3 as the third allowed sector.

## A further check worth doing (not done)

The lattice values in Table 9 are matched to the exact ones by nearest eigenvalue. Near −1, the lattice has many
gauge-type eigenvalues, so nearest-value matching is fragile there. The robust version computes each matched
lattice eigenvector's overlap with the discrete left-invariant subspace: the edge integrals of the 9 left-invariant
su(2)-valued 1-forms. At n = 1 and 2 (426 and 3,372 unknowns) this is a dense eigensolve, minutes each.
