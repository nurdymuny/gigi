// ============================================================
// Davis Lab Bracket Predictor — Pure Engine
// Mathematical spec: bracket_predictor_spec.md
// C = τ/K
// ============================================================

const HISTORICAL_SEED_WIN_RATES = {
  "1v16": 0.99, "2v15": 0.94, "3v14": 0.85, "4v13": 0.79,
  "5v12": 0.64, "6v11": 0.63, "7v10": 0.61, "8v9": 0.51,
};

const ROUND_MULTIPLIERS = {
  R64: 1.0, R32: 1.2, S16: 1.4, E8: 1.6, F4: 2.0, NCG: 2.5,
};

const PROBABILITY_FLOOR = 0.05;
const PROBABILITY_CEILING = 0.95;
const SEED_DIFF_COEFFICIENT = 0.03;

// Team data — 2026 Tournament
const TEAMS = {
  east: [
    { seed: 1, name: "Duke", record: "32-2", adj: 0.0897, injury: -0.03, note: "No.1 overall seed, Cameron Boozer NPOY candidate. Foster out, Ngongba uncertain." },
    { seed: 16, name: "Siena", record: "23-11", adj: -0.0377, injury: 0, note: "First tourney in 16 years. McNamara coaching. Top-50 interior D." },
    { seed: 8, name: "Ohio State", record: "21-12", adj: 0.0126, injury: 0, note: "Won 4 straight to close. Thornton averaging 21.8 PPG in stretch." },
    { seed: 9, name: "TCU", record: "22-11", adj: 0.0016, injury: 0, note: "Beat Florida, Wisconsin, Iowa State. Top-25 defense. 6-game win streak." },
    { seed: 5, name: "St. John's", record: "28-6", adj: 0.0562, injury: 0, note: "Back-to-back Big East titles. 16-1 in last 17. Top defense." },
    { seed: 12, name: "Northern Iowa", record: "23-12", adj: -0.0116, injury: 0, note: "MVC tourney champs. Top-25 defense nationally. First dance in decade." },
    { seed: 4, name: "Kansas", record: "23-10", adj: 0.0239, injury: -0.04, note: "Peterson injury saga. Jekyll/Hyde team. Top-10 D when healthy." },
    { seed: 13, name: "Cal Baptist", record: "25-8", adj: 0.0005, injury: 0, note: "Daniels 23.2 PPG, 5th in nation. Scored 47 in one game." },
    { seed: 6, name: "Louisville", record: "23-10", adj: 0.0242, injury: -0.06, note: "Brown Jr. OUT for opening weekend. Conwell 18.7 PPG carries load." },
    { seed: 11, name: "South Florida", record: "25-8", adj: 0.0257, injury: 0, note: "First tourney since 2012. Nelson 15.8/9.7 best mid-major player." },
    { seed: 3, name: "Michigan St.", record: "25-7", adj: 0.0439, injury: 0, note: "Izzo's 9th F4 contender. Fears 15.5/9.1. Top-10 D. No NBA picks." },
    { seed: 14, name: "N. Dakota St.", record: "27-7", adj: 0.0086, injury: 0, note: "55% inside arc, 38% from 3 since NYE. Heating up." },
    { seed: 7, name: "UCLA", record: "23-11", adj: 0.0188, injury: 0, note: "Dent surging: 15.8 PPG on 53% from 3 in last 7. 6-1 run." },
    { seed: 10, name: "UCF", record: "21-11", adj: -0.0088, injury: 0, note: "Seven top-50 KenPom wins. Kugel back from injury. Big 12 quality." },
    { seed: 2, name: "UConn", record: "29-5", adj: 0.0603, injury: 0, note: "Hurley chasing 3 titles in 4 years. 18-game win streak earlier." },
    { seed: 15, name: "Furman", record: "22-12", adj: -0.0477, injury: 0, note: "5th in SoCon but caught fire in tourney. 81 PPG in conf tourney." },
  ],
  west: [
    { seed: 1, name: "Arizona", record: "32-2", adj: 0.0838, injury: 0, note: "7 players avg 8.7+ PPG. Best D in Big 12. Beat UF, UConn, Bama, Houston." },
    { seed: 16, name: "LIU", record: "24-10", adj: -0.0352, injury: 0, note: "Rod Strickland coaching. Best D in NEC. Small-ball lineup." },
    { seed: 8, name: "Villanova", record: "24-8", adj: 0.0269, injury: 0, note: "First tourney since Jay Wright F4. Top-4 in Big East O and D." },
    { seed: 9, name: "Utah State", record: "28-6", adj: 0.0390, injury: 0, note: "Falslev elite in 8/11 shot zones. 4th straight tourney. 42% from 3." },
    { seed: 5, name: "Wisconsin", record: "24-10", adj: 0.0298, injury: 0, note: "Beat Michigan, Illinois, MSU. Boyd has F4 experience (FAU 2023)." },
    { seed: 12, name: "High Point", record: "30-4", adj: 0.0297, injury: 0, note: "Top-5 nationally in TO rate. Best O&D in Big South. 14-game streak." },
    { seed: 4, name: "Arkansas", record: "26-8", adj: 0.0391, injury: 0, note: "Acuff 22.2/6.4/44% from 3. Best Calipari PG ever. SEC tourney champs." },
    { seed: 13, name: "Hawai'i", record: "24-8", adj: -0.0079, injury: 0, note: "Top-50 defense nationally. Johnson 65% inside arc. First dance in decade." },
    { seed: 6, name: "BYU", record: "23-11", adj: 0.0056, injury: -0.05, note: "Dybantsa led nation in scoring. Saunders OUT (knee). 177th defense." },
    { seed: 11, name: "Texas", record: "18-14", adj: -0.0112, injury: 0, note: "First Four winner. 16th best offense nationally. Swain 17.8/7.6." },
    { seed: 3, name: "Gonzaga", record: "30-3", adj: 0.0667, injury: -0.04, note: "8th ranked D. Ike 19.7 PPG. Huff knee injury may return—huge swing." },
    { seed: 14, name: "Kennesaw St.", record: "21-13", adj: -0.0445, injury: -0.03, note: "Lost top scorer Cottle to gambling probe. Taylor stepped up in tourney." },
    { seed: 7, name: "Miami (FL)", record: "25-8", adj: 0.0271, injury: 0, note: "7-24 to 25-8 turnaround. Reneau 19.2 PPG. Top-5 ACC efficiency." },
    { seed: 10, name: "Missouri", record: "20-12", adj: -0.0036, injury: -0.02, note: "Inconsistent. Mitchell 17.9 PPG. Multiple injuries all season." },
    { seed: 2, name: "Purdue", record: "27-8", adj: 0.0459, injury: 0, note: "Smith about to break Hurley assist record. 39% from 3. Big Ten champs." },
    { seed: 15, name: "Queens", record: "21-13", adj: -0.0419, injury: 0, note: "First year eligible. 6 players avg double figures. May merge with Elon." },
  ],
  midwest: [
    { seed: 1, name: "Michigan", record: "31-3", adj: 0.0856, injury: -0.02, note: "Tallest frontcourt in America. Lendeborg lottery pick. Cason injury hurts PG depth." },
    { seed: 16, name: "Howard", record: "23-10", adj: -0.0275, injury: 0, note: "First Four winner. 3 tourneys in 4 years under Blakeney. MEAC champs." },
    { seed: 8, name: "Georgia", record: "22-10", adj: 0.0087, injury: 0, note: "First back-to-back bids since Nickelback era. Top-15 offense. Beat Bama, UK." },
    { seed: 9, name: "Saint Louis", record: "28-5", adj: 0.0396, injury: 0, note: "Avila A-10 POY. 40% from 3, 59% inside arc. 5 avg double figs. Late slide." },
    { seed: 5, name: "Texas Tech", record: "22-10", adj: 0.0289, injury: -0.07, note: "Toppin ACL out. Anderson 19.2/7.8 carries. 40% from 3. Beat Duke, Houston." },
    { seed: 12, name: "Akron", record: "29-5", adj: 0.0354, injury: 0, note: "3rd straight tourney. Scott game-winner. Top-15 nationally from 3." },
    { seed: 4, name: "Alabama", record: "23-9", adj: 0.0343, injury: 0, note: "91.7 PPG leads nation. Philon lottery pick. 61st to 101st D efficiency. Won 9 of last 10." },
    { seed: 13, name: "Hofstra", record: "24-10", adj: -0.0065, injury: 0, note: "Claxton NBA vet coaching. Davis 20.2/4.6/40% from 3. 11-1 in last 12." },
    { seed: 6, name: "Tennessee", record: "22-11", adj: 0.0210, injury: -0.04, note: "Ament knee injury—status unknown. #1 offensive rebounding team. Elite D." },
    { seed: 11, name: "SMU", record: "20-13", adj: -0.0105, injury: 0, note: "Edwards returning from ankle. Miller 19.2/41% from 3. Lost 4 without Edwards." },
    { seed: 3, name: "Virginia", record: "29-5", adj: 0.0567, injury: 0, note: "Odom COTY candidate. 46.8% of shots are 3s. 6th in O-reb rate. 11-1 close." },
    { seed: 14, name: "Wright State", record: "23-11", adj: -0.0180, injury: 0, note: "Young team. Cooper 13.4 PPG. Best offense in Horizon League." },
    { seed: 7, name: "Kentucky", record: "21-13", adj: 0.0016, injury: 0, note: "$20M+ roster. 83rd in adj O since March 3. 4-6 in last 10. Underperforming." },
    { seed: 10, name: "Santa Clara", record: "26-8", adj: 0.0373, injury: 0, note: "First tourney in 30 years. Graves 62% at rim, 41% from 3. Best WCC offense." },
    { seed: 2, name: "Iowa State", record: "27-7", adj: 0.0472, injury: 0, note: "Momcilovic 50% from 3(!). Lipsey elite PG. Top-10 D. 4-4 late slide." },
    { seed: 15, name: "Tennessee St.", record: "23-9", adj: -0.0328, injury: 0, note: "First tourney in 32 years. Nolan Smith coaching. Top-25 TO rate." },
  ],
  south: [
    { seed: 1, name: "Florida", record: "26-7", adj: 0.0634, injury: 0, note: "Defending champs. 11-game win streak. Haugh lottery pick. 59% from 2, 38% from 3." },
    { seed: 16, name: "Prairie View", record: "18-17", adj: -0.0867, injury: 0, note: "First Four winner. Sub-.500 record entering tourney." },
    { seed: 8, name: "Clemson", record: "24-10", adj: 0.0180, injury: 0, note: "Top-20 defense. Lost top 5 scorers from last year. 7 players avg 6.1+ PPG." },
    { seed: 9, name: "Iowa", record: "21-12", adj: 0.0039, injury: 0, note: "McCollum-Stirtz combo won tourney games at every stop. Stirtz 20.0/4.5/38% from 3." },
    { seed: 5, name: "Vanderbilt", record: "26-8", adj: 0.0355, injury: -0.02, note: "Tanner projected 1st round pick. Miles back from knee injury. 16-0 start." },
    { seed: 12, name: "McNeese", record: "28-5", adj: 0.0289, injury: 0, note: "#1 in America in forced TOs. Johnson 17.5/5.5 as freshman. Boom box is back." },
    { seed: 4, name: "Nebraska", record: "26-6", adj: 0.0430, injury: 0, note: "Best D in Big Ten. Force TOs on 20% of possessions. Sandfort 17.9/40% from 3." },
    { seed: 13, name: "Troy", record: "22-11", adj: -0.0282, injury: -0.03, note: "Seng out with knee. Bellamy 15.3 PPG stepping up. 61% inside arc without Seng." },
    { seed: 6, name: "North Carolina", record: "24-8", adj: 0.0262, injury: -0.06, note: "Wilson (top-5 pick) OUT with thumb. Veesaar 16.2 PPG filling in. Top-50 D." },
    { seed: 11, name: "VCU", record: "27-7", adj: 0.0313, injury: 0, note: "13-1 in last 14. Djokovic 13.8/1.3 BPG. A-10 tourney champs again." },
    { seed: 3, name: "Illinois", record: "24-8", adj: 0.0504, injury: 0, note: "#1 offense in country. Wagler 17.9 PPG breakout. 80+ in 11 Big Ten games. D is 41st." },
    { seed: 14, name: "Penn", record: "18-11", adj: -0.0369, injury: -0.03, note: "Roberts concussion status unknown. 9-1 in last 10. Ivy League tourney champs." },
    { seed: 7, name: "Saint Mary's", record: "27-5", adj: 0.0515, injury: 0, note: "Shared WCC title with Gonzaga. 39% from 3 (up from 32%). Lewis 22.6 in last 5." },
    { seed: 10, name: "Texas A&M", record: "21-11", adj: 0.0064, injury: -0.02, note: "Picked 13th, finished 4th in SEC. Mgbako limited to 7 games. COTY candidate coach." },
    { seed: 2, name: "Houston", record: "28-6", adj: 0.0598, injury: 0, note: "Flemings lottery pick, 42-pt game. 3rd best nationally in TO rate. Sampson magic." },
    { seed: 15, name: "Idaho", record: "21-14", adj: -0.0435, injury: 0, note: "Was 5-7 in conf, caught fire in tourney. 91 pts/100 poss in tourney. First bid since 1990." },
  ],
};

/**
 * Compute win probability for higher seed in a matchup.
 * Returns { higher, lower, prob, higherWins }
 */
function predictGame(team1, team2, roundMultiplier = 1.0) {
  const higher = team1.seed <= team2.seed ? team1 : team2;
  const lower = team1.seed <= team2.seed ? team2 : team1;

  const seedKey = `${higher.seed}v${lower.seed}`;
  let baseProb = HISTORICAL_SEED_WIN_RATES[seedKey];

  if (baseProb === undefined) {
    const diff = lower.seed - higher.seed;
    baseProb = 0.5 + diff * SEED_DIFF_COEFFICIENT;
  }

  const sigmaH = higher.adj + (higher.injury || 0);
  const sigmaL = lower.adj + (lower.injury || 0);
  const strengthDiff = sigmaH - sigmaL;

  let prob = baseProb + strengthDiff * roundMultiplier;
  prob = Math.max(PROBABILITY_FLOOR, Math.min(PROBABILITY_CEILING, prob));

  return { higher, lower, prob, higherWins: prob >= 0.5 };
}

/**
 * Simulate an entire region bracket (16 teams → 1 champion).
 * teams must be ordered: [1v16, 8v9, 5v12, 4v13, 6v11, 3v14, 7v10, 2v15]
 */
function simulateRegion(teams) {
  // Round of 64 (λ = 1.0)
  const r1 = [];
  for (let i = 0; i < teams.length; i += 2) {
    const result = predictGame(teams[i], teams[i + 1], ROUND_MULTIPLIERS.R64);
    r1.push({ ...result, winner: result.higherWins ? result.higher : result.lower });
  }

  // Round of 32 (λ = 1.2)
  const r2 = [];
  for (let i = 0; i < r1.length; i += 2) {
    const result = predictGame(r1[i].winner, r1[i + 1].winner, ROUND_MULTIPLIERS.R32);
    r2.push({ ...result, winner: result.higherWins ? result.higher : result.lower });
  }

  // Sweet 16 (λ = 1.4)
  const r3 = [];
  for (let i = 0; i < r2.length; i += 2) {
    const result = predictGame(r2[i].winner, r2[i + 1].winner, ROUND_MULTIPLIERS.S16);
    r3.push({ ...result, winner: result.higherWins ? result.higher : result.lower });
  }

  // Elite 8 (λ = 1.6)
  const r4Result = predictGame(r3[0].winner, r3[1].winner, ROUND_MULTIPLIERS.E8);
  const regionWinner = r4Result.higherWins ? r4Result.higher : r4Result.lower;

  return { r1, r2, r3, r4: { ...r4Result, winner: regionWinner }, regionWinner };
}

/**
 * Simulate the full 63-game tournament.
 * Returns detailed results for every round + champion.
 */
function simulateTournament(teamsData = TEAMS) {
  const regions = {};
  for (const key of ["east", "west", "midwest", "south"]) {
    regions[key] = simulateRegion(teamsData[key]);
  }

  // Final Four (λ = 2.0): East vs West, Midwest vs South
  const semi1 = predictGame(regions.east.regionWinner, regions.west.regionWinner, ROUND_MULTIPLIERS.F4);
  const s1Winner = semi1.higherWins ? semi1.higher : semi1.lower;

  const semi2 = predictGame(regions.midwest.regionWinner, regions.south.regionWinner, ROUND_MULTIPLIERS.F4);
  const s2Winner = semi2.higherWins ? semi2.higher : semi2.lower;

  // Championship (λ = 2.5)
  const championship = predictGame(s1Winner, s2Winner, ROUND_MULTIPLIERS.NCG);
  const champion = championship.higherWins ? championship.higher : championship.lower;

  return {
    regions,
    finalFour: {
      semi1: { ...semi1, winner: s1Winner },
      semi2: { ...semi2, winner: s2Winner },
      championship: { ...championship, winner: champion },
      champion,
    },
  };
}

/**
 * Monte Carlo simulation — run N tournaments with probabilistic outcomes.
 * Returns championship frequencies, F4 frequencies, upset counts.
 */
function monteCarloSimulate(teamsData = TEAMS, n = 10000, seed = 42) {
  // Simple seeded PRNG (xorshift32) for reproducibility
  let state = seed;
  function random() {
    state ^= state << 13;
    state ^= state >> 17;
    state ^= state << 5;
    return (state >>> 0) / 4294967296;
  }

  function mcPredictGame(team1, team2, roundMultiplier) {
    const result = predictGame(team1, team2, roundMultiplier);
    const u = random();
    const higherWins = u < result.prob;
    return { ...result, higherWins, winner: higherWins ? result.higher : result.lower };
  }

  function mcSimulateRegion(teams) {
    const r1 = [];
    for (let i = 0; i < teams.length; i += 2) {
      const result = mcPredictGame(teams[i], teams[i + 1], ROUND_MULTIPLIERS.R64);
      r1.push(result);
    }
    const r2 = [];
    for (let i = 0; i < r1.length; i += 2) {
      const result = mcPredictGame(r1[i].winner, r1[i + 1].winner, ROUND_MULTIPLIERS.R32);
      r2.push(result);
    }
    const r3 = [];
    for (let i = 0; i < r2.length; i += 2) {
      const result = mcPredictGame(r2[i].winner, r2[i + 1].winner, ROUND_MULTIPLIERS.S16);
      r3.push(result);
    }
    const r4 = mcPredictGame(r3[0].winner, r3[1].winner, ROUND_MULTIPLIERS.E8);
    return { ...r4, regionWinner: r4.winner };
  }

  const champCount = {};
  const f4Count = {};
  const upsetTracker = {};
  let totalUpsets = 0;

  for (let trial = 0; trial < n; trial++) {
    const eastResult = mcSimulateRegion(teamsData.east);
    const westResult = mcSimulateRegion(teamsData.west);
    const midwestResult = mcSimulateRegion(teamsData.midwest);
    const southResult = mcSimulateRegion(teamsData.south);

    const f4Teams = [eastResult.regionWinner, westResult.regionWinner, midwestResult.regionWinner, southResult.regionWinner];
    for (const t of f4Teams) {
      f4Count[t.name] = (f4Count[t.name] || 0) + 1;
    }

    const semi1 = mcPredictGame(eastResult.regionWinner, westResult.regionWinner, ROUND_MULTIPLIERS.F4);
    const semi2 = mcPredictGame(midwestResult.regionWinner, southResult.regionWinner, ROUND_MULTIPLIERS.F4);
    const champ = mcPredictGame(semi1.winner, semi2.winner, ROUND_MULTIPLIERS.NCG);

    champCount[champ.winner.name] = (champCount[champ.winner.name] || 0) + 1;
  }

  // Sort by frequency
  const champRanking = Object.entries(champCount)
    .map(([name, count]) => ({ name, count, pct: (count / n * 100).toFixed(1) }))
    .sort((a, b) => b.count - a.count);

  const f4Ranking = Object.entries(f4Count)
    .map(([name, count]) => ({ name, count, pct: (count / n * 100).toFixed(1) }))
    .sort((a, b) => b.count - a.count);

  return { champRanking, f4Ranking, n };
}

/**
 * Sensitivity analysis — sweep α perturbations and check if champion flips.
 */
function sensitivityAnalysis(teamsData = TEAMS, deltas = [-0.03, -0.02, -0.01, 0, 0.01, 0.02, 0.03]) {
  const results = [];
  const baseResult = simulateTournament(teamsData);
  const baseChampion = baseResult.finalFour.champion.name;

  for (const delta of deltas) {
    const perturbed = {};
    for (const region of Object.keys(teamsData)) {
      perturbed[region] = teamsData[region].map(t => ({
        ...t,
        adj: Math.max(-0.10, Math.min(0.15, t.adj + delta)),
      }));
    }
    const result = simulateTournament(perturbed);
    const f4 = [
      result.regions.east.regionWinner.name,
      result.regions.west.regionWinner.name,
      result.regions.midwest.regionWinner.name,
      result.regions.south.regionWinner.name,
    ];
    results.push({
      delta,
      champion: result.finalFour.champion.name,
      flipped: result.finalFour.champion.name !== baseChampion,
      f4,
    });
  }

  // Stability radius = smallest |δ| that flips the champion
  const flips = results.filter(r => r.flipped && r.delta !== 0);
  const stabilityRadius = flips.length > 0
    ? Math.min(...flips.map(r => Math.abs(r.delta)))
    : Infinity;

  return { baseChampion, results, stabilityRadius };
}

module.exports = {
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
};
