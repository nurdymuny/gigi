// ============================================================
// Davis Lab Bracket Predictor — Full Output
// Run: node bracket-run.js
// ============================================================
const {
  TEAMS,
  simulateTournament,
  simulateRegion,
  monteCarloSimulate,
  sensitivityAnalysis,
  predictGame,
  ROUND_MULTIPLIERS,
} = require("./bracket-engine");

const G = "\x1b[32m";  // green
const Y = "\x1b[33m";  // yellow
const R = "\x1b[31m";  // red
const C = "\x1b[36m";  // cyan
const M = "\x1b[35m";  // magenta
const W = "\x1b[37m";  // white
const B = "\x1b[1m";   // bold
const D = "\x1b[2m";   // dim
const X = "\x1b[0m";   // reset

function pad(s, n) { return String(s).padEnd(n); }
function padL(s, n) { return String(s).padStart(n); }

console.log(`\n${B}${Y}════════════════════════════════════════════════════════════════${X}`);
console.log(`${B}${Y}  DAVIS LAB BRACKET PREDICTOR 2026 — C = τ/K${X}`);
console.log(`${B}${Y}════════════════════════════════════════════════════════════════${X}\n`);

// ── DETERMINISTIC BRACKET ──────────────────────────────────────
const result = simulateTournament();
const regionColors = { east: R, west: G, midwest: Y, south: M };
const regionLabels = { east: "EAST", west: "WEST", midwest: "MIDWEST", south: "SOUTH" };

for (const regionKey of ["east", "west", "midwest", "south"]) {
  const sim = result.regions[regionKey];
  const col = regionColors[regionKey];
  console.log(`${B}${col}── ${regionLabels[regionKey]} REGION ──${X}`);

  // R64
  console.log(`  ${D}Round of 64:${X}`);
  for (const g of sim.r1) {
    const mark = g.winner.seed > g.higher.seed ? `${R}⚡UPSET${X}` : "";
    const pct = Math.round(g.prob * 100);
    const loserPct = 100 - pct;
    const wSeed = g.winner.seed;
    const lSeed = g.winner === g.higher ? g.lower.seed : g.higher.seed;
    const wName = g.winner.name;
    const lName = g.winner === g.higher ? g.lower.name : g.higher.name;
    console.log(`    ${B}(${padL(wSeed, 2)}) ${pad(wName, 16)}${X} ${G}${pct}%${X}  over  ${D}(${padL(lSeed, 2)}) ${pad(lName, 16)} ${loserPct}%${X} ${mark}`);
  }

  // R32
  console.log(`  ${D}Round of 32:${X}`);
  for (const g of sim.r2) {
    const pct = Math.round(g.prob * 100);
    const wName = g.winner.name;
    const lName = g.winner === g.higher ? g.lower.name : g.higher.name;
    console.log(`    ${B}(${padL(g.winner.seed, 2)}) ${pad(wName, 16)}${X} ${G}${pct}%${X}  over  ${D}(${padL((g.winner === g.higher ? g.lower : g.higher).seed, 2)}) ${lName}${X}`);
  }

  // S16
  console.log(`  ${D}Sweet 16:${X}`);
  for (const g of sim.r3) {
    const pct = Math.round(g.prob * 100);
    const wName = g.winner.name;
    const lName = g.winner === g.higher ? g.lower.name : g.higher.name;
    console.log(`    ${B}(${padL(g.winner.seed, 2)}) ${pad(wName, 16)}${X} ${C}${pct}%${X}  over  ${D}${lName}${X}`);
  }

  // E8
  const e8 = sim.r4;
  const e8pct = Math.round(e8.prob * 100);
  console.log(`  ${D}Elite 8:${X}`);
  console.log(`    ${B}${col}(${e8.winner.seed}) ${e8.winner.name}${X} → ${B}REGION CHAMPION${X}  ${C}${e8pct}%${X}`);
  console.log();
}

// ── FINAL FOUR ─────────────────────────────────────────────────
const ff = result.finalFour;
console.log(`${B}${Y}══════════════════════════════════════${X}`);
console.log(`${B}${Y}  FINAL FOUR${X}`);
console.log(`${B}${Y}══════════════════════════════════════${X}`);

console.log(`\n  ${D}Semifinal 1 (East vs West):${X}`);
console.log(`    ${B}(${ff.semi1.higher.seed}) ${ff.semi1.higher.name}${X}  vs  ${B}(${ff.semi1.lower.seed}) ${ff.semi1.lower.name}${X}`);
console.log(`    → ${B}${G}${ff.semi1.winner.name}${X} wins (${Math.round(ff.semi1.prob * 100)}% for higher seed)`);

console.log(`\n  ${D}Semifinal 2 (Midwest vs South):${X}`);
console.log(`    ${B}(${ff.semi2.higher.seed}) ${ff.semi2.higher.name}${X}  vs  ${B}(${ff.semi2.lower.seed}) ${ff.semi2.lower.name}${X}`);
console.log(`    → ${B}${G}${ff.semi2.winner.name}${X} wins (${Math.round(ff.semi2.prob * 100)}% for higher seed)`);

console.log(`\n  ${D}Championship:${X}`);
console.log(`    ${B}(${ff.championship.higher.seed}) ${ff.championship.higher.name}${X}  vs  ${B}(${ff.championship.lower.seed}) ${ff.championship.lower.name}${X}`);
console.log(`    → ${B}${Y}🏆 ${ff.champion.name}${X} ${B}NATIONAL CHAMPION${X} (${Math.round(ff.championship.prob * 100)}%)\n`);

// ── MONTE CARLO ────────────────────────────────────────────────
console.log(`${B}${C}══════════════════════════════════════${X}`);
console.log(`${B}${C}  MONTE CARLO SIMULATION (N=50,000)${X}`);
console.log(`${B}${C}══════════════════════════════════════${X}\n`);

const mc = monteCarloSimulate(TEAMS, 50000, 2026);

console.log(`  ${B}Championship Probability:${X}`);
const top15 = mc.champRanking.slice(0, 15);
for (const entry of top15) {
  const bar = "█".repeat(Math.round(parseFloat(entry.pct) / 2));
  const color = parseFloat(entry.pct) > 10 ? G : parseFloat(entry.pct) > 5 ? Y : D;
  console.log(`    ${pad(entry.name, 16)} ${color}${padL(entry.pct, 5)}%${X}  ${color}${bar}${X}`);
}

console.log(`\n  ${B}Final Four Probability:${X}`);
const topF4 = mc.f4Ranking.slice(0, 15);
for (const entry of topF4) {
  const bar = "█".repeat(Math.round(parseFloat(entry.pct) / 3));
  const color = parseFloat(entry.pct) > 30 ? G : parseFloat(entry.pct) > 15 ? Y : D;
  console.log(`    ${pad(entry.name, 16)} ${color}${padL(entry.pct, 5)}%${X}  ${color}${bar}${X}`);
}

// ── SENSITIVITY ANALYSIS ───────────────────────────────────────
console.log(`\n${B}${M}══════════════════════════════════════${X}`);
console.log(`${B}${M}  SENSITIVITY ANALYSIS${X}`);
console.log(`${B}${M}══════════════════════════════════════${X}\n`);

const sa = sensitivityAnalysis(TEAMS);
console.log(`  ${B}Base Champion:${X} ${G}${sa.baseChampion}${X}`);
console.log(`  ${B}Stability Radius:${X} ${sa.stabilityRadius === Infinity ? `${G}∞ (rock solid)${X}` : `${Y}δ = ±${sa.stabilityRadius}${X}`}\n`);

for (const r of sa.results) {
  const delta = r.delta >= 0 ? `+${r.delta.toFixed(2)}` : r.delta.toFixed(2);
  const flip = r.flipped ? `${R}← FLIP${X}` : "";
  console.log(`    δ=${delta}  Champion: ${pad(r.champion, 14)} F4: [${r.f4.join(", ")}] ${flip}`);
}

// ── UPSET WATCH ────────────────────────────────────────────────
console.log(`\n${B}${R}══════════════════════════════════════${X}`);
console.log(`${B}${R}  UPSET PROBABILITY WATCH${X}`);
console.log(`${B}${R}══════════════════════════════════════${X}\n`);

const upsetWatch = [];
for (const [region, teams] of Object.entries(TEAMS)) {
  for (let i = 0; i < teams.length; i += 2) {
    const r = predictGame(teams[i], teams[i + 1], 1.0);
    const upsetProb = 1 - r.prob;
    if (upsetProb > 0.30) {
      upsetWatch.push({
        region,
        higher: r.higher,
        lower: r.lower,
        upsetProb,
        prob: r.prob,
      });
    }
  }
}

upsetWatch.sort((a, b) => b.upsetProb - a.upsetProb);

for (const u of upsetWatch) {
  const pct = Math.round(u.upsetProb * 100);
  const bar = "█".repeat(Math.round(pct / 3));
  const color = pct >= 45 ? R : pct >= 40 ? Y : D;
  console.log(`  ${color}${padL(pct, 2)}%${X} ${pad(`(${u.lower.seed}) ${u.lower.name}`, 22)} over ${pad(`(${u.higher.seed}) ${u.higher.name}`, 18)} ${D}[${u.region}]${X}  ${color}${bar}${X}`);
  if (u.lower.note) {
    console.log(`       ${D}→ ${u.lower.note.substring(0, 80)}${X}`);
  }
}

console.log(`\n${D}── Model: seed history (1985–2025) + ESPN strength + injury adjustment + round scaling ──${X}`);
console.log(`${D}── Monte Carlo: 50,000 simulations with seeded PRNG for reproducibility ──${X}`);
console.log(`${D}── Davis Lab // C = τ/K ──${X}\n`);
