# WP1: which block sets the edge (21 November 2026)

**Scope.** The H + GG\* edge x\*(t, y) of the twist's region over t ∈ [2, 12], y ∈ [0.25, 3]. The map uses sectors
j ≤ 12, and side checks go up to j = 2000. Every number below was computed for this note and comes from the receipt
named next to it.

**Labels.**
- **PROVED**: exact symbolic algebra (determinants, factorisations, polynomial divisions).
- **COMPUTED**: double precision, or 60-digit mpmath for roots.
- **CONJECTURE**: an extrapolation that has not been proved.

## Headline

1. **COMPUTED.** With sectors j ≤ 12, the edge is set by eight blocks, counted up to the q ↔ −q mirror (map below).
2. **PROVED.** Each of those blocks has a closed-form edge polynomial.
3. **COMPUTED.** All 132 grid edges are roots of the right factor of their block's polynomial, with
   |root − edge| ≤ 2.4 × 10⁻¹³.
4. **Main finding: the j ≤ 12 edge is not the all-sector edge wherever it is positive, and at t = 2 for small y.**
   - Two near-top charge families, q = l + 1 and q = l, undercut the twist in sectors j > 12.
   - Their per-sector edges fall toward x = 0 roughly like 1/j (COMPUTED, j up to 2000).
   - Each family's determinant, with j symbolic, has an l-leading coefficient (PROVED) whose only root in the edge range is
     x = 0, for s ≠ 1, √2.
   - So for t ∈ (2, 12] the all-sector edge appears to be min(x\*\_12, 0) (CONJECTURE, explained below).
   - This contradicts the positive edges (x\* > 0 for t ≳ 7) that the j ≤ 12 analyses report, including
     exact_edge.json and the table in docs/research/exact-edge-2026-11-21.md. Those positive edges are a sector-cutoff
     effect.
   - An independent check with the general builder (no charge blocks) confirms the undercut, for H + GG\* and for the
     gauge-orthogonal slice.

## 1. Competitor map, sectors j ≤ 12 (COMPUTED)

- Script: `scripts/edge_wp1_competitor_map.py`.
- Receipts: `wp1_competitor_map.json`, `wp1_competitor_map_log.txt`.

Grid: t = 2, 3, …, 12 (11 values) by y = 0.25, 0.5, …, 3 (12 values). The edge is EE.edge(Blocks.phi_A), on the window
[−3, 1] with brentq to 10⁻¹². Entries are (j, |q|); the sign of q alternates between neighbouring points because
(j, q) and (j, −q) are degenerate.

| t \ y | 0.25 | 0.5 | 0.75 | 1.0 | 1.25 | 1.5 | 1.75 | 2.0 | 2.25 | 2.5 | 2.75 | 3.0 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2 | (12,6)† | (3,½) | (3,½) | (4,0) | (4,0) | (4,0) | (4,0) | (1,3/2) | (1,3/2) | (1,3/2) | (2,2) | (2,2) |
| 3 | (2,1) | (2,1) | (3,½) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (1,3/2) | (1,3/2) | (1,3/2) | (1,3/2) |
| 4 | (2,1) | (2,1) | (3,½) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (1,3/2) | (1,3/2) | (1,3/2) |
| 5 | (2,1) | (2,1) | (3,½) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (1,3/2) | (1,3/2) |
| 6 | (2,1) | (2,1) | (3,½) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) |
| 7 | (2,1) | (2,1) | (1,½) | (1,3/2) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) |
| 8 | (2,1) | (2,1) | (1,½) | (1,3/2) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) |
| 9 | (2,1) | (2,1) | (1,½) | (1,3/2) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) |
| 10 | (2,1) | (2,1) | (1,½) | (12,7)† | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) |
| 11 | (2,1) | (2,1) | (12,7)† | (12,7)† | (12,7)† | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) |
| 12 | (2,1) | (2,1) | (12,7)† | (12,7)† | (12,7)† | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) | (4,0) |

† The competitor is in the top sector allowed, so this entry is a cutoff artifact (§4).

Edge values x\*\_12 (j ≤ 12):

| t \ y | 0.25 | 0.5 | 0.75 | 1.0 | 1.25 | 1.5 | 1.75 | 2.0 | 2.25 | 2.5 | 2.75 | 3.0 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2 | −0.6441 | −0.6132 | −0.6118 | −0.6433 | −0.6664 | −0.6828 | −0.6944 | −0.7241 | −0.7757 | −0.8165 | −0.8685 | −0.9213 |
| 3 | −0.3672 | −0.3355 | −0.3194 | −0.3294 | −0.3544 | −0.3716 | −0.3828 | −0.3898 | −0.4102 | −0.4355 | −0.4568 | −0.4744 |
| 4 | −0.2373 | −0.2030 | −0.1736 | −0.1690 | −0.1927 | −0.2080 | −0.2170 | −0.2212 | −0.2221 | −0.2308 | −0.2433 | −0.2540 |
| 5 | −0.1632 | −0.1283 | −0.0905 | −0.0753 | −0.0966 | −0.1094 | −0.1156 | −0.1168 | −0.1145 | −0.1098 | −0.1059 | −0.1105 |
| 6 | −0.1173 | −0.0828 | −0.0395 | −0.0163 | −0.0348 | −0.0449 | −0.0483 | −0.0466 | −0.0414 | −0.0337 | −0.0245 | −0.0147 |
| 7 | −0.0871 | −0.0535 | −0.0107 | +0.0170 | +0.0069 | −0.0004 | −0.0011 | +0.0032 | +0.0112 | +0.0216 | +0.0336 | +0.0463 |
| 8 | −0.0663 | −0.0338 | +0.0061 | +0.0331 | +0.0362 | +0.0315 | +0.0333 | +0.0401 | +0.0505 | +0.0633 | +0.0778 | +0.0930 |
| 9 | −0.0515 | −0.0201 | +0.0174 | +0.0440 | +0.0572 | +0.0550 | +0.0592 | +0.0682 | +0.0808 | +0.0959 | +0.1125 | +0.1300 |
| 10 | −0.0405 | −0.0103 | +0.0253 | +0.0492 | +0.0727 | +0.0727 | +0.0790 | +0.0902 | +0.1048 | +0.1219 | +0.1405 | +0.1600 |
| 11 | −0.0323 | −0.0031 | +0.0289 | +0.0512 | +0.0798 | +0.0862 | +0.0946 | +0.1076 | +0.1241 | +0.1430 | +0.1634 | +0.1847 |
| 12 | −0.0260 | +0.0022 | +0.0298 | +0.0528 | +0.0823 | +0.0968 | +0.1069 | +0.1217 | +0.1399 | +0.1605 | +0.1825 | +0.2054 |

**Checks** (all from `wp1_competitor_map.json`):
- **Single crossing everywhere.** phi_A has exactly one sign change on a 401-point scan of [−3, 1] at all 132 points.
- **Left end is safely inside her region.** phi_A(−3) ≥ 1.43.
- **The competitor block really sits at the edge.** Its value at the edge is within 2.7 × 10⁻¹².
- **The two identifications agree.** The argmin of phi_A just right of the edge matches the per-block argmin at the
  edge at all 132 points.
- **Mirror degeneracy.** f(j, q) and f(j, −q) agree to 2.8 × 10⁻¹⁴ at the edge.
- **Near-ties.** The runner-up gap is smallest at:
  - (t, y) = (2, 0.75): 0.0053;
  - (10, 1.25): 0.0079;
  - (10, 1.0): 0.0121.

## 2. Switch boundaries (COMPUTED)

Method:
- Along each grid line where the competitor changes, the switch is the root of x_A − x_B.
- x_K is the edge of block K alone (brentq, 10⁻¹⁰).
- At the switch I also check that x_A equals the full edge. If it does not, a third block intervenes.

| switch between | along | at | edge there | x_A − edge |
| --- | --- | --- | --- | --- |
| (3,½) → (2,1) | y = 0.5 | t = 2.304727 | −0.49703 | 0 |
| (3,½) → (4,0) | t = 2 | y = 0.752443 | −0.611741 | 0 |
| (4,0) → (1,3/2) | t = 2 | y = 1.896879 | −0.699953 | 0 |
| (1,3/2) → (4,0) | y = 2 | t = 2.536641 | −0.508019 | 0 |
| (1,3/2) → (2,2) | t = 2 | y = 2.582101 | −0.827689 | 0 |
| (2,2) → (1,3/2) | y = 2.75 | t = 2.10413 | −0.791315 | 0 |
| (2,2) → (1,3/2) | y = 3 | t = 2.2345 | −0.750227 | 0 |
| (2,1) → (3,½) | t = 3, 4, 5, 6 | y = 0.574998, 0.621151, 0.650418, 0.673813 | −0.32154, −0.177293, −0.09483, −0.043711 | 0 |
| (3,½) → (4,0) | t = 3, 4, 5, 6 | y = 0.874254, 0.930635, 0.965022, 0.988971 | −0.313198, −0.160701, −0.071515, −0.015234 | 0 |
| (4,0) → (1,3/2) | t = 3, 4, 5 | y = 2.089121, 2.340417, 2.70679 | −0.391613, −0.221826, −0.104994 | 0 |
| (1,3/2) → (4,0) | y = 2.25, 2.5, 2.75, 3 | t = 3.686653, 4.473959, 5.102234, 5.664 | −0.266838, −0.16259, −0.094486, −0.039537 | 0 |
| (3,½) → (1,½) | y = 0.75 | t = 6.417382 | −0.024069 | 0 |
| (4,0) → (1,3/2) | y = 1 | t = 6.57462 | +0.007963 | 0 |
| (2,1) → (1,½) | t = 7, 8, 9, 10 | y = 0.672346, 0.661432, 0.652745, 0.64488 | −0.015818, −3.3e−5, +0.01044, +0.017372 | 0 |
| (1,½) → (1,3/2) | t = 8, 9 | y = 0.904672, 0.885287 | +0.027983, +0.035753 | 0 |
| (1,3/2) → (4,0) | t = 7, 8, 9 | y = 1.053246, 1.140626, 1.195526 | +0.018513, +0.040751, +0.058762 | 0 |
| (1,½) → (1,3/2) | t = 7 | y = 0.923099 | +0.010603 | **4.3e−3**: a third block intervenes |
| (12,6) → (3,½) | t = 2 | y = 0.41306 | −0.611456 | **6.5e−4**: a third block intervenes |
| (12,7) → (4,0) | t = 10 | y = 1.220602 | +0.071889 | **1.3e−3**: a third block intervenes |
| switches involving (12,7) at t = 9–12 | | see JSON | | † cutoff artifact |

**Failures and limits** (plainly):
- **One boundary failed.** For (12,6) → (2,1) along y = 0.25, t ∈ [2, 3], x_K had no crossing in the search window
  and the script raised TypeError. That boundary is not located.
- **Boundaries are only searched between neighbouring grid points.** A competitor that appears and disappears
  between two grid points would be missed.

**What the boundaries show.**
- y ≈ 0.57–0.67 separates (2,1) from (3,½) and (1,½).
- y ≈ 0.75–0.99 separates (3,½) from (4,0).
- An upper curve, from y ≈ 1.9 at t = 2 to y ≈ 3 at t ≈ 5.7, separates (4,0) from (1,3/2).

## 3. Closed-form polynomials (PROVED as algebra; root checks COMPUTED)

- Scripts: `scripts/edge_wp1_block_polynomials.py`, `scripts/edge_wp1_poly_check_final.py`.
- Receipts: `wp1_block_polynomials.{txt,json}` (full factor expressions), `wp1_poly_check_final.json`.

**Method.**
- B_{j,q} is built symbolically, with the same rows as EE.
- Numeric check: it agrees with Blocks.block to ≤ 1.4 × 10⁻¹⁴.
- The algebraic numbers that occur in the entries are collected automatically.
- det(s²(B − λ_j I)) is taken over that field with DomainMatrix.
- All determinants come out with rational coefficients, and they are factored over ℚ. Irreducibility is claimed over
  ℚ only.

Polynomials in (x, y, s), s = √t:

| block | size | field of entries | det degrees (x,y,s) | factors over ℚ (beyond constants) | edge factor | grid points | max \|root − edge\| | max scale-free residual |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| (4, 0) | 9 | ℚ(√2, √3) | (12, 14, 28) | P₁ (5,6,12), P₂ (7,8,16) | **P₂** | 77 | 2.4e−13 | 2.7e−13 (other factor ≥ 2.7e−3) |
| (3, ½) | 8 | ℚ(√2, √6) | (11, 14, 26) | one irreducible (11,14,26), 400 terms | it | 6 | 1.4e−14 | 2.5e−15 |
| (2, 1) | 6 | ℚ | (8, 10, 19) | s · (8,10,18) | (8,10,18) | 20 | 2.3e−13 | 2.9e−13 |
| (1, ½) | 5 | ℚ(√2) | (7, 10, 17) | s · (7,10,16) | (7,10,16) | 4 | 1.5e−16 | 3.1e−16 |
| (1, 3/2) | 3 | ℚ(√2) | (4, 4, 9) | s · (4,4,8) | (4,4,8) | 15 | 8.5e−14 | 5.5e−14 |
| (2, 2) | 3 | ℚ | (4, 4, 9) | s · (4,4,8) | (4,4,8) | 2 | 1.7e−14 | 2.2e−15 |
| (12, 6) † | 6 | ℚ(√6, √11) | (8, 10, 19) | s · (8,10,18) | (8,10,18) | 1 | 1.7e−15 | 2.3e−18 |
| (12, 7) † | 3 | ℚ(√6) | (4, 4, 9) | s · (4,4,8) | (4,4,8) | 7 | 1.1e−13 | 6.8e−13 |

**Cross-checks** (PROVED unless marked):
- **(4, 0) reproduces the earlier work.** Its full determinant equals the earlier edge_polynomial.srepr: the ratio is
  1 at 4 rational points (that comparison is COMPUTED). Its edge factor equals P₂ of
  research/su2-floor-family-20261119/edge_polynomial.txt (polynomial division: quotient 1, remainder 0).
- **Mirror identity.** The polynomials of (1, ±3/2) and (3, ±½) are identical (`wp1_family_members.json`).
  Numerically, every mirror pair is degenerate (§1).
- **Root method.** "root − edge" is mpmath findroot (60 digits) started at the edge. The residual is
  |F| / Σ|terms| at the edge.

## 4. The near-top families and the sector cutoff

- Scripts: `scripts/edge_wp1_jmax_check.py`, `scripts/edge_wp1_top_family.py`, `scripts/edge_wp1_tail_check.py`,
  `scripts/edge_wp1_tail_negx.py`, `scripts/edge_wp1_fullsector_check.py`, `scripts/edge_wp1_family_members.py`.

**Family polynomials (PROVED as algebra).**
- With l = j/2 symbolic, the blocks q = l + 1 (3 states) and q = l (6 states) give determinant numerators:
  - D₊(x, y, s, l), degrees (4, 4, 9, 3);
  - D₀(x, y, s, l), degrees (8, 10, 19, 6).
- No odd powers of the ladder square roots occur.
- Numeric check against Blocks.block: ≤ 6.8 × 10⁻¹³.
- Family membership, by exact division (`wp1_family_members.json`):
  - D₊(l = ½, 1, 6) = s · (edge factor of (1, 3/2), (2, 2), (12, 7));
  - D₀(l = 1, 6) = s · (edge factor of (2, 1), (12, 6));
  - D₀(l = ½) = (s²x + s) · (edge factor of (1, ½)). At l = ½ the q = l formula carries one spurious state.
- So six of the eight competitors are members of two one-parameter families. Only **(4, 0) and (3, ½) are
  "sporadic"**.
- Leading coefficients in l:
  - D₊: c₊ = −128 s x (s − 1)(s + 1)(s² − 2);
  - D₀: c₀ = −65536 s x (s − 1)(s + 1)(s² − 2)(3s² − 4)(s x − 2s² + 2)(s x − s² + 2).
  - In the edge range their only real root in x is **x = 0**. The other roots of c₀ are at x ≥ 1.4 on the grid.
  - Both coefficients **vanish identically at s = √2 (t = 2)**.

**Per-sector edges of the families (COMPUTED, `wp1_top_family.txt`).**
- For t ≥ 6 they decrease toward 0 roughly like 1/j. For example, at (t, y) = (12, 2) the q = l + 1 family gives:

| j | 12 | 24 | 48 | 96 | 200 | 400 | 1000 | 2000 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| edge | 0.2075 | 0.1458 | 0.0905 | 0.0513 | 0.0264 | 0.0137 | 0.0056 | 0.0028 |

- **At t = 2 they converge instead to a negative limit**, consistent with c ≡ 0 there. For the q = l family:
  - y = 0.25: j = 2000 gives −0.68467;
  - y = 0.5: j = 2000 gives −0.61843.

**Grid consequences (COMPUTED).**
- **Undercut at x\*\_12.** A sector 13 ≤ j ≤ 200 undercuts the twist at x\*\_12 at 56 grid points
  (`wp1_tail_check.json`; near-top charges |q| ∈ {l+1, l, l−1, l−2}). These are:
  - every point with x\*\_12 > 0 except (7, 2.0), (7, 2.25), (7, 2.5), which have small positive edges and need
    j > 200, and (12, 3), which the tail run did not include;
  - (2, 0.25) and (2, 0.5).
- **Edge versus cutoff J.** For example, at (12, 2.75): J = 12, 24, 48, 96, 192 give
  0.1825, 0.1825, 0.1706, 0.0969, 0.0519.
- **Just left of zero, nothing undercuts.** At every grid point with x\*\_12 > 0, at x = −0.01 and x = −0.001, all
  near-top blocks of 13 ≤ j ≤ 2000 and all blocks of j ≤ 12 stay above the twist. The smallest margin is 0.029
  (`wp1_tail_negx.json`).
- **Independent general-builder check** (`wp1_fullsector_check.json`; full sector matrices from
  ricci_su3_test2_family, no blocks):
  - (12, 2, x = 0.06): j = 80 has an eigenvalue 0.0029 below λ_j in H + GG\*, and 0.0023 below it in the slice. j ≤ 60
    does not.
  - (10, 1, x = 0.03): j = 40, 60, 80 are below by 0.216, 0.957, 1.712.
  - Controls at x < 0, (6, 1.5, −0.06) and (12, 2, −0.01): nothing below, j ≤ 80.

**CONJECTURE (all sectors, H + GG\*).**
- For 2 < t ≤ 12 the twist's region is x < min(x\*\_12(t, y), 0). The supremum 0 is approached by the near-top
  families as j → ∞ and is not attained at any finite j.
- At t = 2 the limit is a negative j → ∞ value of the q = l family; for y = 0.25 it is below −0.6847.
- Not proved:
  - other charges at large j are covered only for |q| ≥ l − 2 beyond j = 40;
  - uniformity in (t, y) near t = 2 is open, because c vanishes there.

## Consequences for the edge theorem

- **The statement "the edge crosses zero" for j ≤ 12 does not survive all sectors.** For the full operator, the
  region appears to be capped at x = 0 for t > 2. This matches the earlier exploratory map
  (twist-family-map-2026-11-21.md: every floor cell had x ≤ 0).
- **The same undercut appears in the slice spectrum** (general builder, above). So the gauge-orthogonal edges with
  x\* > 0 in exact_edge.json are also cutoff effects at those points. Checked at the one point
  (12, 2, 0.06) only.
- **A theorem domain with x ≤ −δ < 0 is unaffected** by the near-top families: their per-sector edges are positive
  for t ≥ 6 in every case computed. Examples are D of family_large_j and the WP2 slabs.
- **Near t = 2 the families must be included.** At t = 2 the family limit is negative (computed at y = 0.25 and 0.5).
  For t in (2, 3) the families were not examined, apart from the j ≤ 200 tail at the t = 3 grid points, which is
  positive there.
