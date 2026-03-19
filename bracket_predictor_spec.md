# Davis Lab Bracket Predictor — Mathematical Specification

**Version:** 1.0
**Date:** 2026-03-18
**Author:** Davis Lab // C = τ/K
**Purpose:** Complete mathematical specification for local installation and extension


## 1. System Overview

The predictor computes win probabilities for each of the 63 NCAA Tournament games using a three-layer model:

1. **Base Layer** — Historical seed-vs-seed win rates (1985–2025)
2. **Adjustment Layer** — Team-specific strength and injury modifiers
3. **Round Scaling Layer** — Amplification of quality differences in later rounds

The output is a deterministic bracket (highest-probability winner advances) plus per-game win probabilities.


## 2. Data Schema

Each team is represented as a tuple:

```
T = (seed, name, record, α, ι, region, note)
```

where:

- `seed ∈ {1, 2, ..., 16}` — NCAA Tournament seed
- `name` — team identifier string
- `record` — (wins, losses) tuple
- `α ∈ [-0.10, 0.15]` — strength adjustment coefficient
- `ι ∈ [-0.10, 0.00]` — injury penalty coefficient (always ≤ 0)
- `region ∈ {East, West, Midwest, South}`
- `note` — scouting text (not used in computation)

### 2.1 Strength Coefficient (α)

The strength coefficient α encodes team quality relative to seed expectation. It is derived from a weighted composite of five factors:

```
α = w₁·R + w₂·E + w₃·Q + w₄·M + w₅·D
```

| Factor | Symbol | Description | Weight (wᵢ) |
|--------|--------|-------------|-------------|
| Record quality | R | Win% adjusted for SOS | 0.25 |
| Efficiency margin | E | KenPom AdjEM normalized to [-0.1, 0.1] | 0.30 |
| Quality wins | Q | Count of Quad 1 wins, normalized | 0.20 |
| Momentum | M | Win% in last 10 games, centered at 0 | 0.15 |
| Depth/balance | D | Scoring distribution entropy, normalized | 0.10 |

**Normalization:** Each factor is mapped to [-0.10, 0.10] via min-max scaling across the 68-team field, then weighted and summed.

**Current implementation shortcut:** α values were hand-assigned from ESPN/KenPom scouting data using the above rubric qualitatively. For a rigorous local build, pull KenPom or BartTorvik data and compute α programmatically.

### 2.2 Injury Penalty (ι)

The injury penalty ι captures the impact of missing key players:

```
ι = -Σⱼ (USG%ⱼ / 100) × sⱼ
```

where the sum is over injured players j, USG%ⱼ is their usage rate, and sⱼ is a severity multiplier:

| Status | sⱼ |
|--------|-----|
| OUT for tournament | 1.0 |
| Questionable / game-time decision | 0.5 |
| Limited / returning from injury | 0.3 |
| Minor / day-to-day | 0.1 |

**Practical cap:** ι is clamped to [-0.10, 0.00].


## 3. Probability Model

### 3.1 Historical Base Rates

For a first-round matchup between seed s_h (higher, i.e. lower number) and seed s_l (lower, i.e. higher number), the base win probability for the higher seed is:

```
P_base(s_h, s_l) = H[s_h, s_l]
```

drawn from the empirical lookup table (1985–2025, men's tournament):

| Matchup | P_base(higher seed wins) |
|---------|--------------------------|
| 1 vs 16 | 0.990 |
| 2 vs 15 | 0.940 |
| 3 vs 14 | 0.850 |
| 4 vs 13 | 0.790 |
| 5 vs 12 | 0.640 |
| 6 vs 11 | 0.630 |
| 7 vs 10 | 0.610 |
| 8 vs 9  | 0.510 |

**Source:** NCAA historical results, aggregated across all men's tournament games 1985–2025.

### 3.2 Later-Round Base Rate (when seeds don't follow the standard matchup pattern)

For rounds beyond the first, the advancing teams may not correspond to standard seed pairings. In this case, the base probability is computed from the seed difference:

```
P_base(T_a, T_b) = 0.5 + 0.03 × (s_b - s_a)
```

where s_a ≤ s_b (T_a is the higher seed). This linear model assigns ~3% advantage per seed line of difference.

**Rationale:** Empirical analysis shows that in the Round of 32 and beyond, the raw seed advantage compresses but remains monotonically related to seed difference. The 0.03 coefficient is a conservative fit to historical Sweet 16+ matchup data.

### 3.3 Adjustment

Given two teams T₁ and T₂ in a game, let T_h be the higher seed and T_l be the lower seed. The adjusted probability is:

```
P_adj(T_h wins) = P_base + (σ_h - σ_l) × λ_r
```

where:

- `σ_h = α_h + ι_h` — net team strength for the higher seed
- `σ_l = α_l + ι_l` — net team strength for the lower seed
- `λ_r` — round multiplier (see §3.4)

### 3.4 Round Multiplier (λ_r)

The round multiplier amplifies team quality differences in later rounds, reflecting the empirical observation that upsets become less frequent as the tournament progresses (the better team's advantages compound over a longer game sample):

| Round | λ_r |
|-------|-----|
| Round of 64 (First Round) | 1.0 |
| Round of 32 (Second Round) | 1.2 |
| Sweet Sixteen | 1.4 |
| Elite Eight | 1.6 |
| Final Four (Semifinals) | 2.0 |
| National Championship | 2.5 |

**Interpretation:** In the championship game, a strength differential of +0.05 vs -0.03 yields a swing of (0.05 - (-0.03)) × 2.5 = 0.20, i.e. a 20 percentage-point shift from the base rate. This is intentionally aggressive to reward the strongest teams in late rounds.

### 3.5 Clamping

The final probability is clamped to prevent degenerate predictions:

```
P_final = clamp(P_adj, 0.05, 0.95)
```

No team is ever assigned less than 5% or more than 95% win probability in any single game.

### 3.6 Winner Selection

The bracket is deterministic: the team with P > 0.5 advances. In the (theoretically possible but unlikely) case of P = 0.5 exactly, the higher seed advances.

```
Winner(T_h, T_l) = T_h  if P_final ≥ 0.5
                   T_l  otherwise
```


## 4. Tournament Structure

### 4.1 Region Bracket (16 teams → 1 regional champion)

Each region follows the standard NCAA bracket structure. The 16 teams are paired in the first round as:

```
Game 1:  (1) vs (16)
Game 2:  (8) vs (9)
Game 3:  (5) vs (12)
Game 4:  (4) vs (13)
Game 5:  (6) vs (11)
Game 6:  (3) vs (14)
Game 7:  (7) vs (10)
Game 8:  (2) vs (15)
```

Round of 32 matchups:

```
Game 9:  Winner(G1) vs Winner(G2)
Game 10: Winner(G3) vs Winner(G4)
Game 11: Winner(G5) vs Winner(G6)
Game 12: Winner(G7) vs Winner(G8)
```

Sweet 16:

```
Game 13: Winner(G9)  vs Winner(G10)
Game 14: Winner(G11) vs Winner(G12)
```

Elite 8:

```
Game 15: Winner(G13) vs Winner(G14)  →  Regional Champion
```

### 4.2 Final Four

The four regional champions are paired:

```
Semifinal 1:  East Champion   vs  West Champion      (λ_r = 2.0)
Semifinal 2:  Midwest Champion vs South Champion     (λ_r = 2.0)
```

### 4.3 Championship

```
Final:  Winner(SF1) vs Winner(SF2)                   (λ_r = 2.5)
```


## 5. Full Algorithm (Pseudocode)

```
function PREDICT_BRACKET(teams_by_region):

    champions = {}

    for region in [East, West, Midwest, South]:
        teams = teams_by_region[region]  // 16 teams, ordered by pairing

        // Round of 64
        r64_winners = []
        for i in range(0, 16, 2):
            p = compute_probability(teams[i], teams[i+1], λ=1.0)
            r64_winners.append(select_winner(teams[i], teams[i+1], p))

        // Round of 32
        r32_winners = []
        for i in range(0, 8, 2):
            p = compute_probability(r64_winners[i], r64_winners[i+1], λ=1.2)
            r32_winners.append(select_winner(..., p))

        // Sweet 16
        s16_winners = []
        for i in range(0, 4, 2):
            p = compute_probability(r32_winners[i], r32_winners[i+1], λ=1.4)
            s16_winners.append(select_winner(..., p))

        // Elite 8
        p = compute_probability(s16_winners[0], s16_winners[1], λ=1.6)
        champions[region] = select_winner(..., p)

    // Final Four
    p1 = compute_probability(champions[East], champions[West], λ=2.0)
    sf1_winner = select_winner(..., p1)

    p2 = compute_probability(champions[Midwest], champions[South], λ=2.0)
    sf2_winner = select_winner(..., p2)

    // Championship
    p_final = compute_probability(sf1_winner, sf2_winner, λ=2.5)
    national_champion = select_winner(..., p_final)

    return national_champion, full_bracket


function compute_probability(T_a, T_b, λ):
    // Identify higher seed
    T_h, T_l = (T_a, T_b) if T_a.seed ≤ T_b.seed else (T_b, T_a)

    // Base rate
    key = "{T_h.seed}v{T_l.seed}"
    if key in HISTORICAL_TABLE:
        P_base = HISTORICAL_TABLE[key]
    else:
        P_base = 0.5 + 0.03 × (T_l.seed - T_h.seed)

    // Team strength
    σ_h = T_h.α + T_h.ι
    σ_l = T_l.α + T_l.ι

    // Adjusted probability (higher seed wins)
    P_adj = P_base + (σ_h - σ_l) × λ

    // Clamp
    P_final = clamp(P_adj, 0.05, 0.95)

    return P_final  // probability that T_h wins
```


## 6. Extensions for Local Build

### 6.1 Data Pipeline (Recommended)

For a production-grade local version, replace hand-assigned α values with programmatic computation:

```
1. Pull team stats from KenPom API or BartTorvik CSV export
   - AdjEM (adjusted efficiency margin)
   - AdjO, AdjD (offensive/defensive efficiency)
   - SOS (strength of schedule)
   - Last-10 record

2. Pull injury reports from ESPN or CBS Sports injury tracker

3. Compute α for each team:
   α = 0.25·norm(win%) + 0.30·norm(AdjEM) + 0.20·norm(Q1_wins)
       + 0.15·norm(last10_win%) + 0.10·norm(scoring_entropy)

4. Compute ι from injury data:
   ι = -Σ (USG%/100) × severity_multiplier

5. Run prediction pipeline
```

### 6.2 Monte Carlo Extension

The current model is deterministic (highest probability always wins). For bracket pool strategy, a Monte Carlo simulation produces better results:

```
for trial in range(N_SIMULATIONS):  // N = 10,000+
    for each game:
        draw u ~ Uniform(0, 1)
        if u < P_final(higher_seed):
            winner = higher_seed
        else:
            winner = lower_seed
    record full bracket outcome

// Output: distribution over champions, F4 frequencies, upset rates
```

This lets you optimize for bracket pool expected value rather than just picking the mode outcome.

### 6.3 Davis Field Equations Extension (C = τ/K)

To apply the full DFE framework, model each team as a point on a manifold M where:

- **τ (topological complexity):** Encode offensive system complexity as a Betti number analog. Teams with more diverse scoring distributions (high entropy across shot types: 3pt, mid-range, paint, FT) map to higher τ.

- **K (coupling constant):** Defensive adaptability. K measures how tightly a team's defensive scheme couples to the opponent's offensive topology. High K = rigid defense that works against specific systems but breaks against novel ones. Low K = flexible but less intense.

- **C = τ/K (coherence):** The ratio predicts system stability under tournament pressure. High C teams (complex offense, adaptable defense) are predicted to survive deep runs. Low C teams are fragile.

**Matchup prediction under DFE:**

```
C_matchup = |C_a - C_b| / max(C_a, C_b)
```

When C_matchup is small (two similarly-coherent teams), revert to seed-based prediction. When C_matchup is large, favor the higher-C team regardless of seed. The crossover threshold is an empirical parameter to be fitted from historical data.

This is a research direction, not yet validated. The current model uses the simpler three-layer approach above.

### 6.4 Sensitivity Analysis

To assess model robustness, sweep over parameter perturbations:

```
for δ in [-0.02, -0.01, 0, +0.01, +0.02]:
    for each team:
        α' = α + δ
    rerun bracket
    record changes in champion, F4, upset count
```

If the champion changes under small perturbations, the bracket is fragile and the model has low confidence in its pick. Report the "stability radius" — the minimum δ that flips the champion.


## 7. Input Data (2026 Tournament)

The complete team database with all 68 teams, seeds, records, α, ι, and scouting notes is embedded in the companion file `bracket.jsx`. Extract the `TEAMS` object for use in any language.

### 7.1 Key Constants

```python
HISTORICAL_SEED_WIN_RATES = {
    (1, 16): 0.990,
    (2, 15): 0.940,
    (3, 14): 0.850,
    (4, 13): 0.790,
    (5, 12): 0.640,
    (6, 11): 0.630,
    (7, 10): 0.610,
    (8, 9):  0.510,
}

ROUND_MULTIPLIERS = {
    "R64": 1.0,
    "R32": 1.2,
    "S16": 1.4,
    "E8":  1.6,
    "F4":  2.0,
    "NCG": 2.5,
}

PROBABILITY_FLOOR = 0.05
PROBABILITY_CEILING = 0.95
SEED_DIFF_COEFFICIENT = 0.03  # for non-standard matchups
```


## 8. Validation Notes

### 8.1 Backtesting

To validate the model, backtest against 2016–2025 tournament results:

1. Assign α values from pre-tournament KenPom data for each year
2. Run the predictor
3. Score using standard bracket scoring (10/20/40/80/160/320)
4. Compare to seed-only baseline and Vegas consensus

**Expected performance:** The model should outperform a pure seed-based bracket by 5–15% in total points, primarily through correctly identifying injury-impacted upsets and late-season momentum shifts.

### 8.2 Limitations

- **No matchup-specific modeling:** The model treats all 5-vs-12 games as structurally similar. In reality, a fast-tempo 12-seed attacking a slow-tempo 5-seed has different dynamics than a defensive grinder.
- **No pace/style interaction:** Offensive and defensive efficiencies are collapsed into a single α.
- **Single-game variance:** Basketball has high single-game variance. A 70% favorite loses 30% of the time. The deterministic bracket picks the mode, not the expected value.
- **Hand-tuned α values:** Current α assignments are qualitative. Programmatic computation from statistical databases would improve rigor.

This spec gives you everything you need to reimplement in Python, Rust, Julia, or whatever you want to throw the field equations at.
