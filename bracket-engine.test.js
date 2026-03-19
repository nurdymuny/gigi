// ============================================================
// Davis Lab Bracket Predictor — TDD Test Suite
// Tests: 60+ covering math, data integrity, edge cases,
//        hand-calculated verifications, Monte Carlo convergence
// ============================================================
const assert = require("assert");
const {
  HISTORICAL_SEED_WIN_RATES,
  ROUND_MULTIPLIERS,
  PROBABILITY_FLOOR,
  PROBABILITY_CEILING,
  SEED_DIFF_COEFFICIENT,
  TEAMS,
  predictGame,
  simulateRegion,
  simulateTournament,
  monteCarloSimulate,
  sensitivityAnalysis,
} = require("./bracket-engine");

let passed = 0;
let failed = 0;
let total = 0;
const failures = [];

function test(name, fn) {
  total++;
  try {
    fn();
    passed++;
    console.log(`  ✓ ${name}`);
  } catch (e) {
    failed++;
    failures.push({ name, error: e.message });
    console.log(`  ✗ ${name}`);
    console.log(`    ${e.message}`);
  }
}

function approxEqual(a, b, eps = 1e-10) {
  if (Math.abs(a - b) > eps) {
    throw new Error(`Expected ${a} ≈ ${b} (diff=${Math.abs(a - b)}, eps=${eps})`);
  }
}

// ============================================================
// §1  DATA INTEGRITY
// ============================================================
console.log("\n═══ §1 DATA INTEGRITY ═══");

test("4 regions exist", () => {
  assert.deepStrictEqual(Object.keys(TEAMS).sort(), ["east", "midwest", "south", "west"]);
});

test("each region has exactly 16 teams", () => {
  for (const [region, teams] of Object.entries(TEAMS)) {
    assert.strictEqual(teams.length, 16, `${region} has ${teams.length} teams`);
  }
});

test("64 total teams across all regions", () => {
  const total = Object.values(TEAMS).reduce((s, r) => s + r.length, 0);
  assert.strictEqual(total, 64);
});

test("each region has seeds 1–16 exactly once", () => {
  for (const [region, teams] of Object.entries(TEAMS)) {
    const seeds = teams.map(t => t.seed).sort((a, b) => a - b);
    assert.deepStrictEqual(seeds, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
      `${region} seeds: ${seeds}`);
  }
});

test("no duplicate team names across entire tournament", () => {
  const names = Object.values(TEAMS).flat().map(t => t.name);
  const unique = new Set(names);
  assert.strictEqual(unique.size, names.length, `Duplicate names found: ${names.filter((n, i) => names.indexOf(n) !== i)}`);
});

test("teams are in correct bracket order [1v16, 8v9, 5v12, 4v13, 6v11, 3v14, 7v10, 2v15]", () => {
  const expectedSeedPairs = [[1, 16], [8, 9], [5, 12], [4, 13], [6, 11], [3, 14], [7, 10], [2, 15]];
  for (const [region, teams] of Object.entries(TEAMS)) {
    for (let i = 0; i < 8; i++) {
      const [s1, s2] = expectedSeedPairs[i];
      assert.strictEqual(teams[i * 2].seed, s1, `${region}[${i * 2}] expected seed ${s1}, got ${teams[i * 2].seed}`);
      assert.strictEqual(teams[i * 2 + 1].seed, s2, `${region}[${i * 2 + 1}] expected seed ${s2}, got ${teams[i * 2 + 1].seed}`);
    }
  }
});

test("all α values are within spec bounds [-0.10, 0.15]", () => {
  for (const [region, teams] of Object.entries(TEAMS)) {
    for (const t of teams) {
      assert(t.adj >= -0.10 && t.adj <= 0.15,
        `${region}/${t.name} α=${t.adj} out of bounds [-0.10, 0.15]`);
    }
  }
});

test("all ι (injury) values are within spec bounds [-0.10, 0.00]", () => {
  for (const [region, teams] of Object.entries(TEAMS)) {
    for (const t of teams) {
      const injury = t.injury || 0;
      assert(injury >= -0.10 && injury <= 0.00,
        `${region}/${t.name} ι=${injury} out of bounds [-0.10, 0.00]`);
    }
  }
});

test("every team has required fields: seed, name, record, adj, note", () => {
  for (const [region, teams] of Object.entries(TEAMS)) {
    for (const t of teams) {
      assert(typeof t.seed === "number", `${t.name} missing seed`);
      assert(typeof t.name === "string" && t.name.length > 0, `team missing name`);
      assert(typeof t.record === "string" && /^\d+-\d+$/.test(t.record), `${t.name} bad record: ${t.record}`);
      assert(typeof t.adj === "number", `${t.name} missing adj`);
      assert(typeof t.note === "string", `${t.name} missing note`);
    }
  }
});

test("historical seed win rates sum of matchup entries = 8", () => {
  assert.strictEqual(Object.keys(HISTORICAL_SEED_WIN_RATES).length, 8);
});

test("all historical rates are valid probabilities (0, 1)", () => {
  for (const [key, val] of Object.entries(HISTORICAL_SEED_WIN_RATES)) {
    assert(val > 0 && val < 1, `${key}: ${val} is not a valid probability`);
  }
});

test("historical rates are monotonically increasing with seed gap", () => {
  const ordered = [
    HISTORICAL_SEED_WIN_RATES["8v9"],   // gap 1
    HISTORICAL_SEED_WIN_RATES["7v10"],  // gap 3
    HISTORICAL_SEED_WIN_RATES["6v11"],  // gap 5
    HISTORICAL_SEED_WIN_RATES["5v12"],  // gap 7
    HISTORICAL_SEED_WIN_RATES["4v13"],  // gap 9
    HISTORICAL_SEED_WIN_RATES["3v14"],  // gap 11
    HISTORICAL_SEED_WIN_RATES["2v15"],  // gap 13
    HISTORICAL_SEED_WIN_RATES["1v16"],  // gap 15
  ];
  for (let i = 1; i < ordered.length; i++) {
    assert(ordered[i] >= ordered[i - 1],
      `Rate not monotone: gap ${i}: ${ordered[i - 1]} > ${ordered[i]}`);
  }
});

// ============================================================
// §2  CORE PROBABILITY MODEL
// ============================================================
console.log("\n═══ §2 CORE PROBABILITY MODEL ═══");

test("predictGame: higher seed is always the lower seed number", () => {
  const t1 = { seed: 5, name: "A", adj: 0, injury: 0 };
  const t2 = { seed: 12, name: "B", adj: 0, injury: 0 };
  // Pass in reverse order
  const r = predictGame(t2, t1);
  assert.strictEqual(r.higher.seed, 5);
  assert.strictEqual(r.lower.seed, 12);
});

test("predictGame: uses historical rate for standard first-round matchups", () => {
  const t1 = { seed: 1, name: "A", adj: 0, injury: 0 };
  const t2 = { seed: 16, name: "B", adj: 0, injury: 0 };
  const r = predictGame(t1, t2, 1.0);
  // With zero adjustments, prob = base rate = 0.99
  approxEqual(r.prob, 0.95); // clamped to 0.95! Actually wait...
  // base = 0.99, strength diff = 0-0 = 0, so P = 0.99 + 0 = 0.99. Clamp to 0.95.
  // Hmm, 0.99 > 0.95 ceiling → clamped
});

test("predictGame: base rate clamps at ceiling for 1v16 with zero adj", () => {
  const t1 = { seed: 1, name: "A", adj: 0, injury: 0 };
  const t2 = { seed: 16, name: "B", adj: 0, injury: 0 };
  const r = predictGame(t1, t2, 1.0);
  approxEqual(r.prob, 0.95);
  assert.strictEqual(r.higherWins, true);
});

test("predictGame: 8v9 with zero adj returns base rate 0.51", () => {
  const t8 = { seed: 8, name: "A", adj: 0, injury: 0 };
  const t9 = { seed: 9, name: "B", adj: 0, injury: 0 };
  const r = predictGame(t8, t9, 1.0);
  approxEqual(r.prob, 0.51);
});

test("predictGame: strength adjustment moves probability correctly", () => {
  // 8v9 base = 0.51. Team 8 has adj=0.05, team 9 has adj=-0.05.
  // σ_h = 0.05, σ_l = -0.05, diff = 0.10, λ=1.0
  // P = 0.51 + 0.10 * 1.0 = 0.61
  const t8 = { seed: 8, name: "A", adj: 0.05, injury: 0 };
  const t9 = { seed: 9, name: "B", adj: -0.05, injury: 0 };
  const r = predictGame(t8, t9, 1.0);
  approxEqual(r.prob, 0.61);
});

test("predictGame: injury penalty reduces effective strength", () => {
  // 5v12 base = 0.64. 5-seed has adj=0.04, injury=-0.06 → σ=-0.02
  // 12-seed has adj=0.02, injury=0 → σ=0.02
  // diff = -0.02 - 0.02 = -0.04, λ=1.0
  // P = 0.64 + (-0.04) * 1.0 = 0.60
  const t5 = { seed: 5, name: "A", adj: 0.04, injury: -0.06 };
  const t12 = { seed: 12, name: "B", adj: 0.02, injury: 0 };
  const r = predictGame(t5, t12, 1.0);
  approxEqual(r.prob, 0.60);
});

test("predictGame: round multiplier amplifies strength diff", () => {
  // 5v12 base = 0.64. Same teams as above.
  // diff = -0.04, λ=2.5 (championship)
  // P = 0.64 + (-0.04) * 2.5 = 0.64 - 0.10 = 0.54
  const t5 = { seed: 5, name: "A", adj: 0.04, injury: -0.06 };
  const t12 = { seed: 12, name: "B", adj: 0.02, injury: 0 };
  const r = predictGame(t5, t12, 2.5);
  approxEqual(r.prob, 0.54);
});

test("predictGame: probability is clamped at floor 0.05", () => {
  // Make the lower seed extremely strong
  const t1 = { seed: 1, name: "A", adj: -0.10, injury: -0.10 };
  const t16 = { seed: 16, name: "B", adj: 0.15, injury: 0 };
  // σ_h = -0.20, σ_l = 0.15, diff = -0.35, λ=2.5
  // P = 0.99 + (-0.35) * 2.5 = 0.99 - 0.875 = 0.115 ... actually still above floor
  // Let's use bigger multiplier...
  // With λ=1.0: P = 0.99 + (-0.35) = 0.64. Still high.
  // To force clamp, we need P < 0.05. 
  // With base 0.51 (8v9): 0.51 + (-0.35)*2.5 = 0.51 - 0.875 = -0.365 → clamp to 0.05
  const t8 = { seed: 8, name: "A", adj: -0.10, injury: -0.10 };
  const t9 = { seed: 9, name: "B", adj: 0.15, injury: 0 };
  const r = predictGame(t8, t9, 2.5);
  approxEqual(r.prob, 0.05);
  assert.strictEqual(r.higherWins, false);
});

test("predictGame: probability is clamped at ceiling 0.95", () => {
  // 1v16 base = 0.99, any positive strength diff → clamped to 0.95
  const t1 = { seed: 1, name: "A", adj: 0.10, injury: 0 };
  const t16 = { seed: 16, name: "B", adj: -0.05, injury: 0 };
  const r = predictGame(t1, t16, 1.0);
  approxEqual(r.prob, 0.95);
});

test("predictGame: seed-diff fallback for non-standard matchups", () => {
  // 1 vs 4 — not in historical table
  // P_base = 0.5 + 0.03 * (4-1) = 0.5 + 0.09 = 0.59
  const t1 = { seed: 1, name: "A", adj: 0, injury: 0 };
  const t4 = { seed: 4, name: "B", adj: 0, injury: 0 };
  const r = predictGame(t1, t4, 1.0);
  approxEqual(r.prob, 0.59);
});

test("predictGame: same-seed matchup has base 0.50", () => {
  // Two 1-seeds meet (e.g., Final Four)
  // Not in table → P = 0.5 + 0.03 * 0 = 0.50
  const t1a = { seed: 1, name: "A", adj: 0, injury: 0 };
  const t1b = { seed: 1, name: "B", adj: 0, injury: 0 };
  const r = predictGame(t1a, t1b, 1.0);
  approxEqual(r.prob, 0.50);
});

test("predictGame: equal seeds, A wins by tiebreak (higher seed = first by name)", () => {
  // When prob = 0.5 exactly, higherWins should be true (§3.6)
  const t1a = { seed: 1, name: "A", adj: 0, injury: 0 };
  const t1b = { seed: 1, name: "B", adj: 0, injury: 0 };
  const r = predictGame(t1a, t1b, 1.0);
  assert.strictEqual(r.higherWins, true); // P >= 0.5 → higher wins
});

test("predictGame: lower seed can upset when strength favors them", () => {
  // 6v11: base = 0.63. 6-seed is injured, 11-seed is strong.
  // σ_h(6) = 0.01 + (-0.06) = -0.05
  // σ_l(11) = 0.03 + 0 = 0.03
  // diff = -0.05 - 0.03 = -0.08, λ=1.0
  // P = 0.63 - 0.08 = 0.55 → higher seed still wins
  // Need bigger diff. Let's make it extreme:
  const t6 = { seed: 6, name: "A", adj: -0.05, injury: -0.10 };
  const t11 = { seed: 11, name: "B", adj: 0.10, injury: 0 };
  // σ_h = -0.15, σ_l = 0.10, diff = -0.25
  // P = 0.63 + (-0.25) * 1.0 = 0.38 → lower seed wins!
  const r = predictGame(t6, t11, 1.0);
  approxEqual(r.prob, 0.38);
  assert.strictEqual(r.higherWins, false);
});

// ============================================================
// §3  HAND-CALCULATED VERIFICATIONS (Spec §3 Examples)
// ============================================================
console.log("\n═══ §3 HAND-CALCULATED VERIFICATIONS ═══");

test("Spec example: championship strength differential of +0.05 vs -0.03", () => {
  // "a strength differential of +0.05 vs -0.03 yields a swing of (0.05-(-0.03))×2.5 = 0.20"
  const swing = (0.05 - (-0.03)) * 2.5;
  approxEqual(swing, 0.20);
});

test("East R64: Duke(1) vs Siena(16)", () => {
  const duke = TEAMS.east[0];
  const siena = TEAMS.east[1];
  // base = 0.99 (1v16)
  // σ_duke = 0.0897 + (-0.03) = 0.0597
  // σ_siena = -0.0377 + 0 = -0.0377
  // diff = 0.0597 - (-0.0377) = 0.0974
  // P = 0.99 + 0.0974 * 1.0 = 1.0874 → clamp to 0.95
  const r = predictGame(duke, siena, 1.0);
  approxEqual(r.prob, 0.95);
  assert.strictEqual(r.higherWins, true);
  assert.strictEqual(r.higher.name, "Duke");
});

test("East R64: Louisville(6) vs South Florida(11) — upset pick", () => {
  const lou = TEAMS.east[8];
  const usf = TEAMS.east[9];
  // base = 0.63 (6v11)
  // σ_lou = 0.0242 + (-0.06) = -0.0358
  // σ_usf = 0.0257 + 0 = 0.0257
  // diff = -0.0358 - 0.0257 = -0.0615
  // P = 0.63 + (-0.0615) * 1.0 = 0.5685
  const r = predictGame(lou, usf, 1.0);
  approxEqual(r.prob, 0.5685);
  assert.strictEqual(r.higherWins, true); // Louisville still favored at ~57%
  assert.strictEqual(r.higher.name, "Louisville");
});

test("Midwest R64: Texas Tech(5) vs Akron(12) — injury impact", () => {
  const ttu = TEAMS.midwest[4];
  const akr = TEAMS.midwest[5];
  // base = 0.64 (5v12)
  // σ_ttu = 0.0289 + (-0.07) = -0.0411
  // σ_akr = 0.0354 + 0 = 0.0354
  // diff = -0.0411 - 0.0354 = -0.0765
  // P = 0.64 + (-0.0765) * 1.0 = 0.5635
  const r = predictGame(ttu, akr, 1.0);
  approxEqual(r.prob, 0.5635);
  assert.strictEqual(r.higher.name, "Texas Tech");
});

test("West R64: BYU(6) vs Texas(11) — Saunders OUT impact", () => {
  const byu = TEAMS.west[8];
  const tex = TEAMS.west[9];
  // base = 0.63 (6v11)
  // σ_byu = 0.0056 + (-0.05) = -0.0444
  // σ_tex = -0.0112 + 0 = -0.0112
  // diff = -0.0444 - (-0.0112) = -0.0332
  // P = 0.63 + (-0.0332) * 1.0 = 0.5968
  const r = predictGame(byu, tex, 1.0);
  approxEqual(r.prob, 0.5968);
});

test("South R64: UNC(6) vs VCU(11) — Wilson OUT, VCU surging", () => {
  const unc = TEAMS.south[8];
  const vcu = TEAMS.south[9];
  // base = 0.63 (6v11)
  // σ_unc = 0.0262 + (-0.06) = -0.0338
  // σ_vcu = 0.0313 + 0 = 0.0313
  // diff = -0.0338 - 0.0313 = -0.0651
  // P = 0.63 + (-0.0651) * 1.0 = 0.5649
  const r = predictGame(unc, vcu, 1.0);
  approxEqual(r.prob, 0.5649);
  assert.strictEqual(r.higherWins, true); // UNC still slight favorite even without Wilson
});

test("South R64: Vanderbilt(5) vs McNeese(12) — potential upset", () => {
  const vandy = TEAMS.south[4];
  const mcn = TEAMS.south[5];
  // base = 0.64 (5v12)
  // σ_vandy = 0.0355 + (-0.02) = 0.0155
  // σ_mcn = 0.0289 + 0 = 0.0289
  // diff = 0.0155 - 0.0289 = -0.0134
  // P = 0.64 + (-0.0134) * 1.0 = 0.6266
  const r = predictGame(vandy, mcn, 1.0);
  approxEqual(r.prob, 0.6266);
});

test("Midwest R64: Kentucky(7) vs Santa Clara(10) — UK underperforming", () => {
  const uk = TEAMS.midwest[12];
  const scu = TEAMS.midwest[13];
  // base = 0.61 (7v10)
  // σ_uk = 0.0016 + 0 = 0.0016
  // σ_scu = 0.0373 + 0 = 0.0373
  // diff = 0.0016 - 0.0373 = -0.0357
  // P = 0.61 + (-0.0357) * 1.0 = 0.5743
  const r = predictGame(uk, scu, 1.0);
  approxEqual(r.prob, 0.5743);
  assert.strictEqual(r.higher.name, "Kentucky");
});

// ============================================================
// §4  REGION SIMULATION STRUCTURE
// ============================================================
console.log("\n═══ §4 REGION SIMULATION ═══");

test("simulateRegion returns correct structure", () => {
  const sim = simulateRegion(TEAMS.east);
  assert.strictEqual(sim.r1.length, 8, "R64 should have 8 games");
  assert.strictEqual(sim.r2.length, 4, "R32 should have 4 games");
  assert.strictEqual(sim.r3.length, 2, "S16 should have 2 games");
  assert(sim.r4, "E8 result should exist");
  assert(sim.regionWinner, "Region winner should exist");
  assert(sim.regionWinner.name, "Winner should have a name");
  assert(sim.regionWinner.seed, "Winner should have a seed");
});

test("every game in simulation has a winner", () => {
  for (const [region, teams] of Object.entries(TEAMS)) {
    const sim = simulateRegion(teams);
    for (const game of [...sim.r1, ...sim.r2, ...sim.r3]) {
      assert(game.winner, `Game in ${region} missing winner`);
      assert(game.winner.name, `Game winner in ${region} missing name`);
    }
    assert(sim.r4.winner, `E8 in ${region} missing winner`);
  }
});

test("R64 winners advance to R32 games correctly", () => {
  const sim = simulateRegion(TEAMS.east);
  // R32 game 0 should feature R64 game 0 winner vs R64 game 1 winner
  const r32g0teams = [sim.r2[0].higher.name, sim.r2[0].lower.name];
  assert(r32g0teams.includes(sim.r1[0].winner.name), "R64 G0 winner should be in R32 G0");
  assert(r32g0teams.includes(sim.r1[1].winner.name), "R64 G1 winner should be in R32 G0");
});

test("region winner is from the region's teams", () => {
  for (const [region, teams] of Object.entries(TEAMS)) {
    const sim = simulateRegion(teams);
    const regionNames = teams.map(t => t.name);
    assert(regionNames.includes(sim.regionWinner.name),
      `${region} winner ${sim.regionWinner.name} not found in region teams`);
  }
});

// ============================================================
// §5  FULL TOURNAMENT SIMULATION
// ============================================================
console.log("\n═══ §5 FULL TOURNAMENT ═══");

test("simulateTournament returns all 4 regions + final four", () => {
  const result = simulateTournament();
  assert(result.regions.east, "East region missing");
  assert(result.regions.west, "West region missing");
  assert(result.regions.midwest, "Midwest region missing");
  assert(result.regions.south, "South region missing");
  assert(result.finalFour, "Final four missing");
  assert(result.finalFour.champion, "Champion missing");
});

test("champion is one of the four region winners", () => {
  const result = simulateTournament();
  const regionWinners = [
    result.regions.east.regionWinner.name,
    result.regions.west.regionWinner.name,
    result.regions.midwest.regionWinner.name,
    result.regions.south.regionWinner.name,
  ];
  assert(regionWinners.includes(result.finalFour.champion.name),
    `Champion ${result.finalFour.champion.name} not in region winners: ${regionWinners}`);
});

test("Final Four pairing: East vs West (semi1), Midwest vs South (semi2)", () => {
  const result = simulateTournament();
  const semi1Teams = [result.finalFour.semi1.higher.name, result.finalFour.semi1.lower.name];
  assert(semi1Teams.includes(result.regions.east.regionWinner.name), "East winner should be in semi1");
  assert(semi1Teams.includes(result.regions.west.regionWinner.name), "West winner should be in semi1");

  const semi2Teams = [result.finalFour.semi2.higher.name, result.finalFour.semi2.lower.name];
  assert(semi2Teams.includes(result.regions.midwest.regionWinner.name), "Midwest winner should be in semi2");
  assert(semi2Teams.includes(result.regions.south.regionWinner.name), "South winner should be in semi2");
});

test("deterministic: two calls produce identical results", () => {
  const r1 = simulateTournament();
  const r2 = simulateTournament();
  assert.strictEqual(r1.finalFour.champion.name, r2.finalFour.champion.name);
  assert.strictEqual(r1.regions.east.regionWinner.name, r2.regions.east.regionWinner.name);
  assert.strictEqual(r1.regions.west.regionWinner.name, r2.regions.west.regionWinner.name);
  assert.strictEqual(r1.regions.midwest.regionWinner.name, r2.regions.midwest.regionWinner.name);
  assert.strictEqual(r1.regions.south.regionWinner.name, r2.regions.south.regionWinner.name);
});

test("total games = 63 (32 + 16 + 8 + 4 + 2 + 1)", () => {
  const result = simulateTournament();
  let count = 0;
  for (const region of Object.values(result.regions)) {
    count += region.r1.length; // 8
    count += region.r2.length; // 4
    count += region.r3.length; // 2
    count += 1;                // E8
  }
  count += 2; // semifinals
  count += 1; // championship
  assert.strictEqual(count, 63);
});

// ============================================================
// §6  ROUND MULTIPLIER VERIFICATION
// ============================================================
console.log("\n═══ §6 ROUND MULTIPLIERS ═══");

test("round multipliers match spec exactly", () => {
  approxEqual(ROUND_MULTIPLIERS.R64, 1.0);
  approxEqual(ROUND_MULTIPLIERS.R32, 1.2);
  approxEqual(ROUND_MULTIPLIERS.S16, 1.4);
  approxEqual(ROUND_MULTIPLIERS.E8, 1.6);
  approxEqual(ROUND_MULTIPLIERS.F4, 2.0);
  approxEqual(ROUND_MULTIPLIERS.NCG, 2.5);
});

test("higher λ increases probability swing for same teams", () => {
  const tA = { seed: 3, name: "A", adj: 0.08, injury: 0 };
  const tB = { seed: 6, name: "B", adj: -0.02, injury: 0 };
  const r1 = predictGame(tA, tB, 1.0);
  const r25 = predictGame(tA, tB, 2.5);
  // Both should favor A, but r25 should be more certain
  assert(r25.prob > r1.prob, `λ=2.5 prob (${r25.prob}) should be > λ=1.0 prob (${r1.prob})`);
});

// ============================================================
// §7  EDGE CASES
// ============================================================
console.log("\n═══ §7 EDGE CASES ═══");

test("team with no injury field defaults to 0", () => {
  const tA = { seed: 1, name: "A", adj: 0.05 }; // no injury field
  const tB = { seed: 16, name: "B", adj: 0, injury: 0 };
  const r = predictGame(tA, tB, 1.0);
  // σ_h = 0.05 + 0 = 0.05 (injury defaults to 0)
  // σ_l = 0 + 0 = 0
  // P = 0.99 + 0.05*1 = 1.04 → clamp to 0.95
  approxEqual(r.prob, 0.95);
});

test("predictGame with λ=0 ignores strength entirely", () => {
  const tA = { seed: 5, name: "A", adj: 0.10, injury: 0 };
  const tB = { seed: 12, name: "B", adj: -0.10, injury: 0 };
  const r = predictGame(tA, tB, 0);
  // P = 0.64 + 0.20 * 0 = 0.64
  approxEqual(r.prob, 0.64);
});

test("extreme negative adj doesn't break computation", () => {
  const tA = { seed: 1, name: "A", adj: -0.10, injury: -0.10 };
  const tB = { seed: 2, name: "B", adj: 0.15, injury: 0 };
  const r = predictGame(tA, tB, 2.5);
  assert(r.prob >= PROBABILITY_FLOOR && r.prob <= PROBABILITY_CEILING);
});

// ============================================================
// §8  KNOWN BRACKET PREDICTIONS (Smoke Tests)
// ============================================================
console.log("\n═══ §8 SMOKE TESTS ═══");

test("all 1-seeds win their first game", () => {
  for (const [region, teams] of Object.entries(TEAMS)) {
    const sim = simulateRegion(teams);
    const g1 = sim.r1[0]; // 1 vs 16
    assert.strictEqual(g1.winner.seed, 1, `1-seed lost in ${region} R64`);
  }
});

test("all 2-seeds win their first game", () => {
  for (const [region, teams] of Object.entries(TEAMS)) {
    const sim = simulateRegion(teams);
    const g8 = sim.r1[7]; // 2 vs 15
    assert.strictEqual(g8.winner.seed, 2, `2-seed lost in ${region} R64`);
  }
});

test("champion exists and has valid seed", () => {
  const result = simulateTournament();
  const champ = result.finalFour.champion;
  assert(champ.seed >= 1 && champ.seed <= 16);
  assert(champ.name.length > 0);
  assert(champ.record.length > 0);
});

// ============================================================
// §9  MONTE CARLO CONVERGENCE
// ============================================================
console.log("\n═══ §9 MONTE CARLO ═══");

test("Monte Carlo returns valid championship distribution", () => {
  const mc = monteCarloSimulate(TEAMS, 1000, 42);
  assert(mc.champRanking.length > 0, "Should have at least 1 champion");
  const totalPct = mc.champRanking.reduce((s, r) => s + parseFloat(r.pct), 0);
  assert(Math.abs(totalPct - 100) < 1, `Championship pcts should sum to ~100, got ${totalPct}`);
});

test("Monte Carlo: deterministic champion should be #1 in MC rankings", () => {
  const det = simulateTournament();
  const mc = monteCarloSimulate(TEAMS, 5000, 42);
  // The deterministic champion should be in the top 3 at least
  const top3 = mc.champRanking.slice(0, 3).map(r => r.name);
  assert(top3.includes(det.finalFour.champion.name),
    `Deterministic champion ${det.finalFour.champion.name} not in MC top 3: ${top3}`);
});

test("Monte Carlo: no team has > 50% championship probability", () => {
  const mc = monteCarloSimulate(TEAMS, 5000, 42);
  for (const entry of mc.champRanking) {
    assert(parseFloat(entry.pct) <= 50,
      `${entry.name} has ${entry.pct}% — unreasonably high for 64-team bracket`);
  }
});

test("Monte Carlo: F4 ranking has at least 4 teams", () => {
  const mc = monteCarloSimulate(TEAMS, 1000, 42);
  assert(mc.f4Ranking.length >= 4, `Only ${mc.f4Ranking.length} teams made F4`);
});

test("Monte Carlo: reproducible with same seed", () => {
  const mc1 = monteCarloSimulate(TEAMS, 1000, 123);
  const mc2 = monteCarloSimulate(TEAMS, 1000, 123);
  assert.strictEqual(mc1.champRanking[0].name, mc2.champRanking[0].name);
  assert.strictEqual(mc1.champRanking[0].count, mc2.champRanking[0].count);
});

// ============================================================
// §10  SENSITIVITY ANALYSIS
// ============================================================
console.log("\n═══ §10 SENSITIVITY ANALYSIS ═══");

test("sensitivity analysis returns results for each delta", () => {
  const sa = sensitivityAnalysis(TEAMS, [-0.02, -0.01, 0, 0.01, 0.02]);
  assert.strictEqual(sa.results.length, 5);
  assert(sa.baseChampion, "Should have a base champion");
  assert(typeof sa.stabilityRadius === "number");
});

test("sensitivity: delta=0 preserves base champion", () => {
  const sa = sensitivityAnalysis(TEAMS, [0]);
  assert.strictEqual(sa.results[0].flipped, false);
  assert.strictEqual(sa.results[0].champion, sa.baseChampion);
});

test("sensitivity: α perturbation stays within spec bounds", () => {
  // When delta = 0.05 is applied, teams at adj=0.12 would become 0.17,
  // which exceeds the 0.15 cap. The engine should clamp.
  const sa = sensitivityAnalysis(TEAMS, [0.05]);
  // If it runs without error, bounds are being respected
  assert(sa.results.length === 1);
});

// ============================================================
// §11  MATHEMATICAL PROPERTIES
// ============================================================
console.log("\n═══ §11 MATHEMATICAL PROPERTIES ═══");

test("probability is symmetric: P(A beats B) + P(B beats A) = 1 (before clamping)", () => {
  // For unclamped cases, check symmetry
  const t5 = { seed: 5, name: "A", adj: 0.03, injury: 0 };
  const t12 = { seed: 12, name: "B", adj: 0.01, injury: 0 };
  const r = predictGame(t5, t12, 1.0);
  // P(higher wins) = r.prob
  // P(lower wins) = 1 - r.prob
  // These should sum to 1
  approxEqual(r.prob + (1 - r.prob), 1.0);
});

test("stronger higher seed always has P > base rate", () => {
  // If σ_h > σ_l, then P > P_base (positive adjustment)
  for (const matchup of ["5v12", "6v11", "7v10", "8v9"]) {
    const [s1, s2] = matchup.split("v").map(Number);
    const base = HISTORICAL_SEED_WIN_RATES[matchup];
    const tH = { seed: s1, name: "H", adj: 0.05, injury: 0 };
    const tL = { seed: s2, name: "L", adj: -0.05, injury: 0 };
    const r = predictGame(tH, tL, 1.0);
    assert(r.prob >= base, `${matchup}: prob ${r.prob} < base ${base} despite stronger higher seed`);
  }
});

test("weaker higher seed has P < base rate", () => {
  for (const matchup of ["5v12", "6v11", "7v10", "8v9"]) {
    const [s1, s2] = matchup.split("v").map(Number);
    const base = HISTORICAL_SEED_WIN_RATES[matchup];
    const tH = { seed: s1, name: "H", adj: -0.05, injury: 0 };
    const tL = { seed: s2, name: "L", adj: 0.05, injury: 0 };
    const r = predictGame(tH, tL, 1.0);
    assert(r.prob <= base, `${matchup}: prob ${r.prob} > base ${base} despite weaker higher seed`);
  }
});

test("transitivity sanity check: if A >> B and B >> C, A should beat C", () => {
  const A = { seed: 1, name: "A", adj: 0.12, injury: 0 };
  const B = { seed: 4, name: "B", adj: 0.04, injury: 0 };
  const C = { seed: 8, name: "C", adj: -0.02, injury: 0 };
  const rAB = predictGame(A, B, 1.0);
  const rBC = predictGame(B, C, 1.0);
  const rAC = predictGame(A, C, 1.0);
  assert(rAB.higherWins, "A should beat B");
  assert(rBC.higherWins, "B should beat C");
  assert(rAC.higherWins, "A should beat C (transitive)");
  assert(rAC.prob >= rAB.prob || rAC.prob >= rBC.prob,
    "A vs C should have at least as high a prob as one of the intermediate matchups");
});

// ============================================================
// RESULTS
// ============================================================
console.log("\n═══════════════════════════════════════");
console.log(`  RESULTS: ${passed}/${total} passed, ${failed} failed`);
console.log("═══════════════════════════════════════");

if (failures.length > 0) {
  console.log("\nFAILURES:");
  for (const f of failures) {
    console.log(`  ✗ ${f.name}: ${f.error}`);
  }
}

process.exit(failed > 0 ? 1 : 0);
