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

---

## Addendum: a closed form for the bottom of every sector, derived

- **Script:** `scripts/ricci_su2_sector_formula.py` (sympy, symbolic in j).
- **Output:** `research/su2-left-invariant-sector-20261119/sector_formula.txt`.

**Statement.** At the Berger metric g = (3, 1, 1), in every spin-j/2 sector, the field

    b = f_top · (θ̂¹ + i θ̂²) ⊗ (τ₁ + i τ₂)

is an exact eigenvector of the Hessian. Here f_top is the top-weight vector of V_j along the long axis, with
J₀ f = (j/2) f, and the frame and Lie indices are both circular in the squashed plane. It is orthogonal to every
gauge direction, and its eigenvalue is

    λ(j) = (j(j+2) − 14)/3 = −14/3 + j(j+2)/3.

Its conjugate, at bottom weight, gives the same value.

**Derivation.**

- **Reduction to two vectors.** At top weight, ê₀f = −2i(j/2)f/√3, J₊f = 0, ρ₂f = iρ₁f and ‖ρ₁f‖² = j. Every term
  of the Hessian therefore reduces to the two vectors f and g = ρ₁f/√j, with coefficients polynomial in j.
- **The top-weight block.** On the squashed plane (frame and Lie indices 1, 2) the 9 × 9 top-weight block has a
  4 × 4 piece with eigenvalues (j² + 2j − 14)/3, (j² + 4j − 14)/3 and (j² + 3j + 33 ± 3√(j² + 9))/3.
- **The eigenvector decouples.** The eigenvector of the first value is (−1, −i, −i, 1). Its coupling to the one
  outside direction (τ₀ in b₀) vanishes.
- **No leak to the next weight.** Its image under D has no component along g, because β₂ − iβ₁ = 0. So the
  Hessian sends it nowhere outside the top weight.
- **Gauge.** Its overlap with gauge directions vanishes at every weight.
- **The value.** The Rayleigh quotient is j²/3 + 2j/3 − 14/3.

**What is numerical, not proved:** that this is the *lowest* gauge-orthogonal eigenvalue of the sector. It holds
for every j from 0 to 40, to 2 × 10⁻¹².

**What it means on Poincaré's sphere.**

- The sectors present are j = 0, 12, 20, 24, 30, …, and j(j+2) is the round scalar Laplacian eigenvalue of
  spin j/2.
- So the bottom of the physical spectrum is −14/3 (the growth mode), and the next physical value is
  −14/3 + 168/3 = 154/3. **The gap above the left-invariant sector's growth mode is 168/3 = 56: one third of 168,
  the first nonzero scalar eigenvalue of the Poincaré sphere.**
- The family's j = 0 member is the growth mode −14/3 itself, so the field's strongest instability and the bottom
  of every higher sector are one formula.

**Specific to this metric.** At (2, 1, 1), (4, 1, 1) or round, the bottom of the sector is not −λ₀ + j(j+2)/g₀
(checked numerically). The 3 is tied to g₀ = 3. Whether the circular family stays an exact eigenvector at other
squashings, and with what value, is not computed.

## Addendum 2: minimality proved for j ≤ 120, and the gap proved

- **Script:** `scripts/ricci_su2_sector_minimality.py`.
- **Receipt:** `research/su2-left-invariant-sector-20261119/minimality.json`.
- **In the paper:** Theorem 1.

**The idea.** On the slice, ⟨b, Hb⟩ = ⟨b, (H + GG*)b⟩. And H + GG* = Σᵢ XᵢᴴXᵢ + R₀, where:

- Xᵢ = ρᵢ/√gᵢ + Ωᵢ is the covariant derivative;
- R₀ is a constant 9 × 9 matrix with smallest eigenvalue −12;
- ω = (Σ‖Ωᵢ‖²)^½ = 5.0332.

**The pieces:**

- **(1) An all-j bound:** sector j ≥ (√(j(j+2)/3) − ω)² − 12. This is ≥ 142 for every j ≥ 30.
- **(2) Computed for j = 0 to 120:** the bottom of H + GG* equals λ(j) exactly, twice (top and bottom weight).
  The next eigenvalue is at least λ(j) + 2/3, against round-off below 2 × 10⁻⁵.
- **(3) Spin 0, exactly (sympy):** −14/3 (×2), 16 − 2√73, 22/3 (×2), 16 + 2√73.

**The gap is therefore proved.** Below 142 the gauge-orthogonal spectrum is the spin-0 values plus spin 6 from
154/3 upward, and spin 10 starts at 142. Minimality for j > 120 is open, and the gap does not need it.
