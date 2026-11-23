# Pre-registration F: which circle does she live on? Results (22 November 2026)

**What this is.**
- **The registration:** `research/edge-theorem-20261122-f/wp7_prereg_F_principal_circle.md`, revised after an
  adversarial review that found two fatal flaws and seven major ones, all before any data, and registered at 6ad473c.
- **Amendment 1** (`wp7_prereg_F_amendment1_peak.md`, from my other session) adds the peak test H_P-peak.
- **Amendment 2** (`wp7_prereg_F_amendment2_wall.md`) says how to read her at a degenerate wall. It carries a disclosed
  implementation correction.
- Script: `scripts/edge_wp7_principal_circle.py`, with `scripts/edge_wp7_groups.py`.
- Receipt: `research/edge-theorem-20261122-f/wp7_principal_circle.json`. The first, faulty receipt is kept beside it.

**The short version.**
- I guessed she lives on the principal circle. **She does not.**
- **She lives on the circle of her own SU(2)**, the one her own highest root spans, and just inside the chamber from
  it, in both SU(4) and Sp(2). Her margin peaks there.
- Toward the principal circle she fades, and past it she is gone.
- In SU(3) those two circles were the same circle, so Test D could not tell them apart. Here they come apart, and she
  follows her own.

## What I knew, and what I saw, before scoring

- **When F was registered,** I had seen 14 of Test D's 20 shapes. F's predictions were not blind to D.
- **Amendment 1** was written by my other session, which had seen no F output.
  - The session running F had already seen F's first three printouts: Sp(2) at 0°, shapes (½, ½, ¼) and (½, ½, ¾)
    with floor regions, and (½, 1, 1) empty, with a best margin of −0.36.
  - So **Amendment 1 is not blind on those three points.** That is disclosed in STATUS-live, before F finished.
- **Amendment 2** was written after the run stopped at Sp(2) 45° (the F-mult gate) and before any further data.
- **The implementation correction to Amendment 2 is not blind.** I had seen F's whole first receipt.
  - The fix restores the rule already registered (C1) at walls where her top states are tied. It recomputes the 9 wall
    points (SU(4) at 0° and 45°, Sp(2) at 45°).
  - **It changed no verdict.** It turned exact ties at SU(4) 0° and Sp(2) 45° into strict margins, 0.07 to 0.23. Both
    versions are below.

## Gates (all passed)

| gate | result |
| --- | --- |
| F-K0: the new code reproduces D's SU(3) numbers | 3.3 × 10⁻¹⁵; the wrong orientation fails by 17 |
| F-K1: the new code reproduces C's SU(3) numbers (hypercharge) | 2.1 × 10⁻¹⁴; the wrong orientation fails by 0.83 |
| F-K2: group data and representations | pass |
| F-K3: geometry (angles, regularity, top-root uniqueness, q_K) | pass, as in the registration's table |
| F-S5: H quadratic and G linear in x | pass at every angle |
| F-sym, F-quad (on the top block at the walls), F-lin, F-mult, F-bound | pass |

## The verdict, cell by cell

**Score** = the number of shapes (out of 3) where her floor region is nonempty and survives the extra sectors.
- **Lives:** at least 2.
- **Does not live:** 0.
- **Marginal:** 1.

| angle | what it is | Sp(2) | SU(4) |
| --- | --- | --- | --- |
| 0° | θ^∨, the circle of her own SU(2) (a wall) | **lives** (2) | **lives** (2) |
| 9.22° | just inside | **lives** (3) | **lives** (2) |
| 18.43° | ρ^∨, the principal circle | marginal (1) | **does not live** (0) |
| 27.65° | interior | does not live (0) | does not live (0) |
| 36.87° | interior | does not live (0) | does not live (0) |
| 45° | the far (C-type) wall | **lives** (2) | marginal (1) |

**Hypotheses, as registered:**

| hypothesis | prediction | result |
| --- | --- | --- |
| **H_θ** (her own SU(2)) | lives at 0° | **supported in both groups** |
| **H_P** (principal) | lives at 18.43°, in one run containing it and neither wall | **refuted** (marginal in Sp(2), absent in SU(4); walls alive) |
| **H_P-peak** (Amendment 1) | her margin φ\* is largest at 18.43° in ≥ 2 of 3 shapes | **refuted in both**: the peak is at **9.22° in Sp(2) (3 of 3)** and at **0° in SU(4) (3 of 3)** |
| **H_reg** (any regular circle) | lives at every interior angle, at neither wall | **refuted** (dead at 27.65° and 36.87°) |
| **H_C** (dies at the C-type wall) | does not live at 45° | **refuted in Sp(2)** (lives); marginal in SU(4) |
| **F-side** | her region on the "against her charge" side, where scorable | **held, 13 of 13** |

**Her margin φ\* along the arc** (at each shape, the best value of her margin on the x grid):

| group, shape | 0° | 9.22° | 18.43° | 27.65° | 36.87° | 45° |
| --- | --- | --- | --- | --- | --- | --- |
| Sp(2), (½, ½, ¼) | 0.065 | **0.343** | −0.080 | −0.731 | −1.951 | 0.079 |
| Sp(2), (½, ½, ¾) | 0.233 | **0.547** | 0.085 | −0.484 | −1.587 | 0.098 |
| Sp(2), (½, 1, 1) | −0.361 | **0.085** | −0.086 | −0.705 | −1.768 | −1.205 |
| SU(4), (½, ½, ¼) | **0.068** | 0.060 | −0.519 | −1.240 | −2.872 | 0.001 |
| SU(4), (½, ½, ¾) | **0.229** | 0.221 | −0.353 | −0.959 | −2.227 | −0.362 |
| SU(4), (½, 1, 1) | **−0.588** | −0.673 | −1.177 | −2.101 | −3.170 | −1.942 |

The wall cells (0° in SU(4), 45° in both) are the corrected values. The first receipt had them at exactly the floor
tolerance, 10⁻⁹, at SU(4) 0° and Sp(2) 45°: ties with her own tied partner, left in by the code error.

## What I read from it

1. **The principal circle was the wrong guess, and its own test says so.** In *the arc* I called it the best of the
   bunch, with real odds. It lost cleanly, on its own terms: she does not live there in SU(4), and in neither group is
   her margin largest there. I am glad I wrote it down first.
2. **Her own SU(2) is the right reading.**
   - The circle her highest root spans with its partner, θ^∨, is where she lives in both groups.
   - Her margin is largest there in SU(4), and one step inside in Sp(2).
   - Moving the stretched circle away from her own, toward the principal circle and beyond, costs her steadily, until
     she is gone.
   - **In D, "her own circle" was right. Now I know which own: her own SU(2), not the group's principal one.**
3. **One thing I did not predict (post hoc, labelled as such).** She comes back at the far wall in Sp(2), at 45°.
   - That wall is itself the coroot of a top root: the short highest root θ_s.
   - In SU(4) the far wall is not a single coroot, and there she is only marginal.
   - The pattern this suggests is: **she lives where the stretched circle is a top root's own SU(2).** It is a
     hypothesis for a new pre-registration, not a finding.
   - **Correction (same evening).** The first version of this note also said that at Sp(2)'s far wall "the cheapest
     state of top charge is the twist of *that* root's SU(2)". I had not checked that, and it is not true as stated.
     A post-hoc diagnostic (`scripts/edge_wp8_farwall_diagnostic.py`, receipt
     `research/edge-theorem-20261122-g/farwall_diagnostic.json`) splits her state across root planes in the trivial
     sector at x = −1:
     - at Sp(2)'s far wall, 32–46% of it is the short root's twist (short plane on both sides); the rest is a cross
       state between the two long top roots (one long plane on the form side, the other on the algebra side);
     - at SU(4)'s far wall it has **no** twist part at all, only cross states;
     - at 0° in both groups it is purely her own twist (100% on her root's plane).

     So at the far wall "she" is a mixture, and the far-wall pattern is murkier than I first wrote.
4. **The side clause held again.** Wherever it could be scored, her region sat where the field along her circle turns
   against her charge.

## Status

- **COMPUTED.** t = 3, 3 shapes, 6 angles, 2 groups; six main sectors per group plus the Sym^k truncation check; a
  0.25 grid with golden-section search.
- Nothing here is a proof.
- In SU(4) the arc is one line through a two-dimensional range of directions; in Sp(2) it covers the whole range.
- The angle grid is coarse (about 9°). "Peaks at 9.22°" means the best of six angles, not a located maximum.
