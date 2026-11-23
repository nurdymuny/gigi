# The Floor: LinkedIn cards

Five cards for *The Floor Theorem for the Yang–Mills Hessian at the Berger Field*
(doi:10.5281/zenodo.24412603). They use the plate style and palette of the Reading the Curve cards
(`../../two-ring-pg/make_polished_plates.py`).

```
python docs/papers/berger-spectrum/social/make_floor_cards.py
```

Each card is written as a 1200 × 1200 PNG to `cards/`, apart from the link preview, which is 1200 × 627.

## Where every number comes from

| Card | What it shows | Source |
|---|---|---|
| 01 cover | λ_j(t) = [j(j+2) − 2(t−3)j − 2(t−2)(t+4)]/t for j = 0…8, with the certified range shaded and the point t = 3, j = 0 → −14/3 marked | the floor law in the published v2; the curves are evaluated from the formula itself, so there is no data file |
| 02 results | every t; [2.75, 10]; 48,762 block-slices, then Gershgorin; ≤ 9 states per block; −14/3 (twice at t = 3); 168/3 | the published v2, abstract and main theorems |
| 03 anatomy | λ = κ² + (2j + t + y²) + ((4 − 2t) − 2B), with κ = (t−2)/√t − x − j/√t and B = y² − 2√t x | the v3 draft; checked in Lean as `Helike.price_anatomy` (standard axioms) on branch `feat/helike-lean`. The ballooning square γ/2 + ¾(s/κ − α)² is from the CHIHIRO v3 draft (Lean `Chihiro/Ballooning.lean`), and the card labels it a shared shape, not shared physics |
| 04 what it is not | scope; the edge as the root of an explicit polynomial; the three simpler readings that failed | items 1 and 3 from v2; items 2 and 4 from the v3 draft (Proposition “The edge”, and the paragraph “What does not decide it”). The card labels them that way |
| 05 link preview | the cover's curves, small | same as 01 |

Nothing on these cards was produced by a run of the GIGI engine.
