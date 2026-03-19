// ============================================================
// Davis Lab Bracket Predictor — Data Pipeline
// Fetches BartTorvik data and computes α per spec §2.1
// C = τ/K
// ============================================================

// Map our bracket-engine team names → BartTorvik team names
const NAME_MAP = {
  // East
  "Duke": "Duke", "Siena": "Siena", "Ohio State": "Ohio St.",
  "TCU": "TCU", "St. John's": "St. John's", "Northern Iowa": "Northern Iowa",
  "Kansas": "Kansas", "Cal Baptist": "Cal Baptist", "Louisville": "Louisville",
  "South Florida": "South Florida", "Michigan St.": "Michigan St.",
  "N. Dakota St.": "North Dakota St.", "UCLA": "UCLA", "UCF": "UCF",
  "UConn": "Connecticut", "Furman": "Furman",
  // West
  "Arizona": "Arizona", "LIU": "LIU", "Villanova": "Villanova",
  "Utah State": "Utah St.", "Wisconsin": "Wisconsin", "High Point": "High Point",
  "Arkansas": "Arkansas", "Hawai'i": "Hawaii", "BYU": "BYU", "Texas": "Texas",
  "Gonzaga": "Gonzaga", "Kennesaw St.": "Kennesaw St.",
  "Miami (FL)": "Miami FL", "Missouri": "Missouri", "Purdue": "Purdue",
  "Queens": "Queens",
  // Midwest
  "Michigan": "Michigan", "Howard": "Howard", "Georgia": "Georgia",
  "Saint Louis": "Saint Louis", "Texas Tech": "Texas Tech", "Akron": "Akron",
  "Alabama": "Alabama", "Hofstra": "Hofstra", "Tennessee": "Tennessee",
  "SMU": "SMU", "Virginia": "Virginia", "Wright State": "Wright St.",
  "Kentucky": "Kentucky", "Santa Clara": "Santa Clara",
  "Iowa State": "Iowa St.", "Tennessee St.": "Tennessee St.",
  // South
  "Florida": "Florida", "Prairie View": "Prairie View A&M",
  "Clemson": "Clemson", "Iowa": "Iowa", "Vanderbilt": "Vanderbilt",
  "McNeese": "McNeese St.", "Nebraska": "Nebraska", "Troy": "Troy",
  "North Carolina": "North Carolina", "VCU": "VCU", "Illinois": "Illinois",
  "Penn": "Penn", "Saint Mary's": "Saint Mary's", "Texas A&M": "Texas A&M",
  "Houston": "Houston", "Idaho": "Idaho",
};

// BartTorvik JSON column indices
const COL = {
  TRANK: 0,
  NAME: 1,
  CONF: 2,
  RECORD: 3,
  ADJOE: 4,
  ADJOE_RK: 5,
  ADJDE: 6,
  ADJDE_RK: 7,
  BARTHAG: 8,
  BARTHAG_RK: 9,
  ADJ_W: 10,
  ADJ_L: 11,
  CONF_W: 12,
  CONF_L: 13,
  CONF_REC: 14,
};

/**
 * Normalize a value to [-0.10, 0.10] via min-max scaling.
 */
function normalize(val, min, max) {
  if (max === min) return 0;
  return -0.10 + 0.20 * ((val - min) / (max - min));
}

/**
 * Parse win-loss record string → [wins, losses]
 */
function parseRecord(rec) {
  const parts = rec.split("-").map(Number);
  return { wins: parts[0], losses: parts[1] };
}

async function main() {
  console.log("================================================================");
  console.log("  DAVIS LAB — α DATA PIPELINE (BartTorvik 2026)");
  console.log("  Spec §2.1: α = 0.25·R + 0.30·E + 0.20·Q + 0.15·M + 0.10·D");
  console.log("================================================================\n");

  // 1. Fetch data
  console.log("→ Fetching BartTorvik 2026 data...");
  const resp = await fetch("https://barttorvik.com/2026_team_results.json");
  const allTeams = await resp.json();
  console.log(`  ✓ ${allTeams.length} teams loaded\n`);

  // 2. Build lookup by BartTorvik name
  const lookup = {};
  for (const row of allTeams) {
    lookup[row[COL.NAME]] = row;
  }

  // 3. Match our 64 teams
  const matched = [];
  const missing = [];

  for (const [ourName, btvName] of Object.entries(NAME_MAP)) {
    const row = lookup[btvName];
    if (!row) {
      missing.push({ ourName, btvName });
      continue;
    }

    const rec = parseRecord(row[COL.RECORD]);
    const winPct = rec.wins / (rec.wins + rec.losses);
    const adjEM = row[COL.ADJOE] - row[COL.ADJDE];
    const barthag = row[COL.BARTHAG];

    // Conf record for momentum proxy
    const confW = row[COL.CONF_W];
    const confL = row[COL.CONF_L];
    const confWinPct = confW / (confW + confL);

    matched.push({
      ourName,
      btvName,
      trank: row[COL.TRANK],
      record: row[COL.RECORD],
      winPct,
      adjOE: row[COL.ADJOE],
      adjDE: row[COL.ADJDE],
      adjEM,
      barthag,
      confWinPct,
    });
  }

  if (missing.length > 0) {
    console.log("⚠ UNMATCHED TEAMS:");
    for (const m of missing) {
      console.log(`  ${m.ourName} (looked for "${m.btvName}")`);
    }
    console.log();
  }

  console.log(`  ✓ ${matched.length}/64 teams matched\n`);

  // 4. Compute normalization ranges across our 64 teams
  const stats = {
    winPct: { min: Infinity, max: -Infinity },
    adjEM: { min: Infinity, max: -Infinity },
    barthag: { min: Infinity, max: -Infinity },
    confWinPct: { min: Infinity, max: -Infinity },
  };

  for (const t of matched) {
    for (const key of Object.keys(stats)) {
      stats[key].min = Math.min(stats[key].min, t[key]);
      stats[key].max = Math.max(stats[key].max, t[key]);
    }
  }

  console.log("NORMALIZATION RANGES (across 64-team field):");
  console.log(`  R (win%)     : ${stats.winPct.min.toFixed(4)} – ${stats.winPct.max.toFixed(4)}`);
  console.log(`  E (AdjEM)    : ${stats.adjEM.min.toFixed(2)} – ${stats.adjEM.max.toFixed(2)}`);
  console.log(`  Q (Barthag)  : ${stats.barthag.min.toFixed(4)} – ${stats.barthag.max.toFixed(4)}`);
  console.log(`  M (conf W%)  : ${stats.confWinPct.min.toFixed(4)} – ${stats.confWinPct.max.toFixed(4)}`);
  console.log(`  D (entropy)  : no data — set to 0 for all teams`);
  console.log();

  // 5. Compute α for each team
  const weights = { R: 0.25, E: 0.30, Q: 0.20, M: 0.15, D: 0.10 };

  const results = matched.map(t => {
    const R = normalize(t.winPct, stats.winPct.min, stats.winPct.max);
    const E = normalize(t.adjEM, stats.adjEM.min, stats.adjEM.max);
    const Q = normalize(t.barthag, stats.barthag.min, stats.barthag.max);
    const M = normalize(t.confWinPct, stats.confWinPct.min, stats.confWinPct.max);
    const D = 0; // no entropy data

    const alpha_raw = weights.R * R + weights.E * E + weights.Q * Q + weights.M * M + weights.D * D;
    // Clamp α to spec bounds [-0.10, 0.15]
    const alpha = Math.max(-0.10, Math.min(0.15, alpha_raw));

    return { ...t, R, E, Q, M, D, alpha_raw, alpha };
  });

  // Sort by α descending
  results.sort((a, b) => b.alpha - a.alpha);

  // 6. Print results
  console.log("COMPUTED α VALUES (sorted by α):");
  console.log("─".repeat(95));
  console.log(
    "Team".padEnd(20),
    "T-Rank".padStart(6),
    "Record".padStart(7),
    "AdjEM".padStart(7),
    "Barthag".padStart(8),
    "  R".padStart(7),
    "  E".padStart(7),
    "  Q".padStart(7),
    "  M".padStart(7),
    "  α".padStart(7)
  );
  console.log("─".repeat(95));

  for (const r of results) {
    console.log(
      r.ourName.padEnd(20),
      String(r.trank).padStart(6),
      r.record.padStart(7),
      r.adjEM.toFixed(1).padStart(7),
      r.barthag.toFixed(4).padStart(8),
      r.R.toFixed(3).padStart(7),
      r.E.toFixed(3).padStart(7),
      r.Q.toFixed(3).padStart(7),
      r.M.toFixed(3).padStart(7),
      r.alpha.toFixed(4).padStart(7)
    );
  }
  console.log("─".repeat(95));

  // 7. Generate update code snippet
  console.log("\n\nGENERATED α UPDATE MAP:");
  console.log("========================");
  console.log("const COMPUTED_ALPHA = {");
  for (const r of results.sort((a, b) => a.ourName.localeCompare(b.ourName))) {
    console.log(`  "${r.ourName}": ${r.alpha.toFixed(4)},`);
  }
  console.log("};");

  // 8. Comparison with hand-tuned
  const { TEAMS } = require("./bracket-engine.js");
  const handTuned = {};
  for (const region of Object.values(TEAMS)) {
    for (const t of region) {
      handTuned[t.name] = t.adj;
    }
  }

  console.log("\n\nHAND-TUNED vs COMPUTED α COMPARISON:");
  console.log("─".repeat(60));
  console.log("Team".padEnd(20), "Hand".padStart(7), "Computed".padStart(9), "Δ".padStart(8));
  console.log("─".repeat(60));

  const deltas = [];
  for (const r of results.sort((a, b) => b.alpha - a.alpha)) {
    const hand = handTuned[r.ourName];
    if (hand !== undefined) {
      const delta = r.alpha - hand;
      deltas.push({ name: r.ourName, hand, computed: r.alpha, delta });
      const flag = Math.abs(delta) > 0.03 ? " ◄" : "";
      console.log(
        r.ourName.padEnd(20),
        hand.toFixed(2).padStart(7),
        r.alpha.toFixed(4).padStart(9),
        (delta >= 0 ? "+" : "") + delta.toFixed(4).padStart(7) + flag
      );
    }
  }
  console.log("─".repeat(60));

  // Stats
  const avgDelta = deltas.reduce((s, d) => s + Math.abs(d.delta), 0) / deltas.length;
  const maxDelta = deltas.reduce((m, d) => Math.max(m, Math.abs(d.delta)), 0);
  const bigSwings = deltas.filter(d => Math.abs(d.delta) > 0.03);

  console.log(`\n  Mean |Δ|: ${avgDelta.toFixed(4)}`);
  console.log(`  Max  |Δ|: ${maxDelta.toFixed(4)}`);
  console.log(`  Big swings (|Δ| > 0.03): ${bigSwings.length}`);
  if (bigSwings.length > 0) {
    for (const s of bigSwings.sort((a, b) => Math.abs(b.delta) - Math.abs(a.delta))) {
      console.log(`    ${s.name}: ${s.hand.toFixed(2)} → ${s.computed.toFixed(4)} (Δ = ${s.delta >= 0 ? "+" : ""}${s.delta.toFixed(4)})`);
    }
  }
}

main().catch(console.error);
