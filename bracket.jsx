import { useState, useMemo, useCallback } from "react";

// Historical seed win rates (1-seed vs 16-seed, 2 vs 15, etc.)
const HISTORICAL_SEED_WIN_RATES = {
  "1v16": 0.99, "2v15": 0.94, "3v14": 0.85, "4v13": 0.79,
  "5v12": 0.64, "6v11": 0.63, "7v10": 0.61, "8v9": 0.51,
};

// Team data with strength modifiers based on ESPN intel
const TEAMS = {
  // EAST REGION
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
  // WEST REGION
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
  // MIDWEST REGION
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
  // SOUTH REGION
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

// Predict a game outcome
function predictGame(team1, team2, roundMultiplier = 1) {
  const higher = team1.seed <= team2.seed ? team1 : team2;
  const lower = team1.seed <= team2.seed ? team2 : team1;
  
  const seedKey = `${higher.seed}v${lower.seed}`;
  let baseProb = HISTORICAL_SEED_WIN_RATES[seedKey];
  
  if (!baseProb) {
    // For later rounds, use seed difference
    const diff = lower.seed - higher.seed;
    baseProb = 0.5 + (diff * 0.03);
  }
  
  // Adjust for team-specific factors
  const higherStrength = higher.adj + (higher.injury || 0);
  const lowerStrength = lower.adj + (lower.injury || 0);
  const strengthDiff = higherStrength - lowerStrength;
  
  let prob = baseProb + strengthDiff * roundMultiplier;
  prob = Math.max(0.05, Math.min(0.95, prob));
  
  return { higher, lower, prob, higherWins: prob > 0.5 };
}

function simulateRegion(teams) {
  // teams array: [1v16, 8v9, 5v12, 4v13, 6v11, 3v14, 7v10, 2v15]
  const r1 = [];
  for (let i = 0; i < teams.length; i += 2) {
    const result = predictGame(teams[i], teams[i + 1]);
    r1.push({
      ...result,
      winner: result.higherWins ? result.higher : result.lower,
    });
  }
  
  // Round of 32
  const r2 = [];
  for (let i = 0; i < r1.length; i += 2) {
    const result = predictGame(r1[i].winner, r1[i + 1].winner, 1.2);
    r2.push({
      ...result,
      winner: result.higherWins ? result.higher : result.lower,
    });
  }
  
  // Sweet 16
  const r3 = [];
  for (let i = 0; i < r2.length; i += 2) {
    const result = predictGame(r2[i].winner, r2[i + 1].winner, 1.4);
    r3.push({
      ...result,
      winner: result.higherWins ? result.higher : result.lower,
    });
  }
  
  // Elite 8
  const r4 = predictGame(r3[0].winner, r3[1].winner, 1.6);
  const regionWinner = r4.higherWins ? r4.higher : r4.lower;
  
  return { r1, r2, r3, r4: { ...r4, winner: regionWinner }, regionWinner };
}

const REGION_COLORS = {
  east: { bg: "#1a1a2e", accent: "#e94560", light: "#ff6b81" },
  west: { bg: "#1a2e1a", accent: "#00b894", light: "#55efc4" },
  midwest: { bg: "#2e1a1a", accent: "#fd9644", light: "#fdcb6e" },
  south: { bg: "#1a1a3e", accent: "#6c5ce7", light: "#a29bfe" },
};

const REGION_LABELS = { east: "EAST", west: "WEST", midwest: "MIDWEST", south: "SOUTH" };

function GameSlot({ team, prob, isWinner, accent, onClick, small }) {
  return (
    <div
      onClick={onClick}
      style={{
        display: "flex",
        alignItems: "center",
        gap: small ? 4 : 6,
        padding: small ? "3px 6px" : "4px 8px",
        background: isWinner ? `${accent}22` : "rgba(255,255,255,0.03)",
        borderLeft: isWinner ? `3px solid ${accent}` : "3px solid transparent",
        borderRadius: 4,
        cursor: onClick ? "pointer" : "default",
        transition: "all 0.2s",
        minWidth: small ? 120 : 160,
        fontSize: small ? 11 : 12,
      }}
    >
      <span style={{
        fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
        color: "rgba(255,255,255,0.4)",
        fontSize: small ? 9 : 10,
        width: small ? 14 : 18,
        textAlign: "right",
        flexShrink: 0,
      }}>{team.seed}</span>
      <span style={{
        color: isWinner ? "#fff" : "rgba(255,255,255,0.5)",
        fontWeight: isWinner ? 700 : 400,
        fontFamily: "'Outfit', 'DM Sans', sans-serif",
        flex: 1,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
      }}>{team.name}</span>
      {prob != null && (
        <span style={{
          fontFamily: "'JetBrains Mono', monospace",
          fontSize: small ? 9 : 10,
          color: isWinner ? accent : "rgba(255,255,255,0.3)",
          flexShrink: 0,
        }}>{Math.round(prob * 100)}%</span>
      )}
    </div>
  );
}

function RegionBracket({ regionKey, teams, colors, onSelectTeam, selectedTeam }) {
  const sim = useMemo(() => simulateRegion(teams), [teams]);
  
  const rounds = [
    { label: "R64", games: sim.r1 },
    { label: "R32", games: sim.r2 },
    { label: "S16", games: sim.r3 },
    { label: "E8", games: [{ ...sim.r4, winner: sim.regionWinner }] },
  ];
  
  return (
    <div style={{
      background: `linear-gradient(135deg, ${colors.bg}, #0d0d0d)`,
      borderRadius: 12,
      padding: 16,
      border: `1px solid ${colors.accent}33`,
      flex: 1,
      minWidth: 320,
    }}>
      <div style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        marginBottom: 12,
      }}>
        <div style={{
          width: 4,
          height: 20,
          background: colors.accent,
          borderRadius: 2,
        }} />
        <h3 style={{
          margin: 0,
          fontFamily: "'Outfit', sans-serif",
          fontSize: 14,
          fontWeight: 800,
          color: colors.accent,
          letterSpacing: 3,
          textTransform: "uppercase",
        }}>{REGION_LABELS[regionKey]}</h3>
        <div style={{
          marginLeft: "auto",
          background: `${colors.accent}22`,
          padding: "2px 8px",
          borderRadius: 10,
          fontSize: 10,
          color: colors.light,
          fontFamily: "'JetBrains Mono', monospace",
        }}>
          Champion: {sim.regionWinner.name}
        </div>
      </div>
      
      <div style={{ display: "flex", gap: 8, overflow: "auto" }}>
        {rounds.map((round, ri) => (
          <div key={ri} style={{ minWidth: ri === 0 ? 170 : 140 }}>
            <div style={{
              fontSize: 9,
              color: "rgba(255,255,255,0.3)",
              fontFamily: "'JetBrains Mono', monospace",
              marginBottom: 4,
              letterSpacing: 2,
            }}>{round.label}</div>
            <div style={{ display: "flex", flexDirection: "column", gap: ri === 0 ? 2 : 8 }}>
              {round.games.map((game, gi) => {
                const t1 = game.higher || game.winner;
                const t2 = game.lower;
                const w = game.winner;
                return (
                  <div key={gi} style={{
                    display: "flex",
                    flexDirection: "column",
                    gap: 1,
                    marginBottom: ri === 0 ? 4 : 0,
                  }}>
                    <GameSlot
                      team={t1}
                      prob={t2 ? (game.higherWins ? game.prob : 1 - game.prob) : null}
                      isWinner={w && t1.name === w.name}
                      accent={colors.accent}
                      small={ri === 0}
                      onClick={() => onSelectTeam(t1)}
                    />
                    {t2 && <GameSlot
                      team={t2}
                      prob={game.higherWins ? 1 - game.prob : game.prob}
                      isWinner={w && t2.name === w.name}
                      accent={colors.accent}
                      small={ri === 0}
                      onClick={() => onSelectTeam(t2)}
                    />}
                  </div>
                );
              })}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function TeamDetailPanel({ team, onClose }) {
  if (!team) return null;
  
  const injuryFlag = (team.injury || 0) < -0.02;
  const strengthLabel = team.adj > 0.08 ? "ELITE" : team.adj > 0.04 ? "STRONG" : team.adj > 0 ? "SOLID" : "UNDERDOG";
  const strengthColor = team.adj > 0.08 ? "#00b894" : team.adj > 0.04 ? "#fdcb6e" : team.adj > 0 ? "#74b9ff" : "#e94560";
  
  return (
    <div style={{
      background: "rgba(20,20,30,0.95)",
      border: "1px solid rgba(255,255,255,0.1)",
      borderRadius: 12,
      padding: 20,
      backdropFilter: "blur(20px)",
      animation: "fadeIn 0.2s ease",
    }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
        <div>
          <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 6 }}>
            <span style={{
              fontFamily: "'JetBrains Mono', monospace",
              fontSize: 14,
              color: "rgba(255,255,255,0.3)",
            }}>#{team.seed}</span>
            <h2 style={{
              margin: 0,
              fontFamily: "'Outfit', sans-serif",
              fontSize: 22,
              fontWeight: 800,
              color: "#fff",
            }}>{team.name}</h2>
            <span style={{
              background: `${strengthColor}22`,
              color: strengthColor,
              padding: "2px 8px",
              borderRadius: 10,
              fontSize: 10,
              fontWeight: 700,
              fontFamily: "'JetBrains Mono', monospace",
            }}>{strengthLabel}</span>
          </div>
          <div style={{
            fontFamily: "'JetBrains Mono', monospace",
            fontSize: 13,
            color: "rgba(255,255,255,0.5)",
          }}>{team.record}</div>
        </div>
        <button onClick={onClose} style={{
          background: "rgba(255,255,255,0.05)",
          border: "1px solid rgba(255,255,255,0.1)",
          color: "rgba(255,255,255,0.5)",
          borderRadius: 6,
          padding: "4px 10px",
          cursor: "pointer",
          fontSize: 12,
        }}>×</button>
      </div>
      
      <div style={{
        marginTop: 12,
        display: "flex",
        gap: 12,
        flexWrap: "wrap",
      }}>
        <div style={{
          background: "rgba(255,255,255,0.03)",
          borderRadius: 8,
          padding: 12,
          flex: 1,
          minWidth: 200,
        }}>
          <div style={{ fontSize: 10, color: "rgba(255,255,255,0.3)", fontFamily: "'JetBrains Mono', monospace", marginBottom: 6, letterSpacing: 1 }}>SCOUTING REPORT</div>
          <p style={{ margin: 0, fontSize: 13, color: "rgba(255,255,255,0.8)", lineHeight: 1.5, fontFamily: "'Outfit', sans-serif" }}>{team.note}</p>
        </div>
        
        <div style={{
          display: "flex",
          flexDirection: "column",
          gap: 8,
          minWidth: 140,
        }}>
          <div style={{
            background: "rgba(255,255,255,0.03)",
            borderRadius: 8,
            padding: 10,
            textAlign: "center",
          }}>
            <div style={{ fontSize: 9, color: "rgba(255,255,255,0.3)", fontFamily: "'JetBrains Mono', monospace", letterSpacing: 1 }}>STRENGTH</div>
            <div style={{ fontSize: 22, fontWeight: 800, color: strengthColor, fontFamily: "'Outfit', sans-serif" }}>
              {team.adj > 0 ? "+" : ""}{(team.adj * 100).toFixed(0)}
            </div>
          </div>
          {injuryFlag && (
            <div style={{
              background: "rgba(233,69,96,0.1)",
              borderRadius: 8,
              padding: 10,
              textAlign: "center",
              border: "1px solid rgba(233,69,96,0.2)",
            }}>
              <div style={{ fontSize: 9, color: "#e94560", fontFamily: "'JetBrains Mono', monospace", letterSpacing: 1 }}>⚠ INJURY</div>
              <div style={{ fontSize: 16, fontWeight: 800, color: "#e94560", fontFamily: "'Outfit', sans-serif" }}>
                {((team.injury || 0) * 100).toFixed(0)}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export default function BracketPredictor() {
  const [selectedTeam, setSelectedTeam] = useState(null);
  const [showMethodology, setShowMethodology] = useState(false);
  
  const finalFour = useMemo(() => {
    const eastSim = simulateRegion(TEAMS.east);
    const westSim = simulateRegion(TEAMS.west);
    const midwestSim = simulateRegion(TEAMS.midwest);
    const southSim = simulateRegion(TEAMS.south);
    
    // Final Four: East vs West, Midwest vs South
    const semi1 = predictGame(eastSim.regionWinner, westSim.regionWinner, 2.0);
    const semi2 = predictGame(midwestSim.regionWinner, southSim.regionWinner, 2.0);
    const s1Winner = semi1.higherWins ? semi1.higher : semi1.lower;
    const s2Winner = semi2.higherWins ? semi2.higher : semi2.lower;
    const championship = predictGame(s1Winner, s2Winner, 2.5);
    const champion = championship.higherWins ? championship.higher : championship.lower;
    
    return {
      east: eastSim.regionWinner,
      west: westSim.regionWinner,
      midwest: midwestSim.regionWinner,
      south: southSim.regionWinner,
      semi1: { ...semi1, winner: s1Winner },
      semi2: { ...semi2, winner: s2Winner },
      championship: { ...championship, winner: champion },
      champion,
    };
  }, []);
  
  return (
    <div style={{
      background: "#0a0a0f",
      minHeight: "100vh",
      color: "#fff",
      fontFamily: "'Outfit', 'DM Sans', sans-serif",
      padding: "20px 16px",
    }}>
      <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;600;700;800;900&family=JetBrains+Mono:wght@400;700&display=swap" rel="stylesheet" />
      
      <style>{`
        @keyframes fadeIn { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: translateY(0); } }
        @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.6; } }
        ::-webkit-scrollbar { height: 4px; width: 4px; }
        ::-webkit-scrollbar-track { background: rgba(255,255,255,0.02); }
        ::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.1); border-radius: 2px; }
      `}</style>
      
      {/* Header */}
      <div style={{ textAlign: "center", marginBottom: 24 }}>
        <div style={{
          fontSize: 10,
          letterSpacing: 6,
          color: "rgba(255,255,255,0.3)",
          fontFamily: "'JetBrains Mono', monospace",
          marginBottom: 4,
        }}>DAVIS LAB // C = τ/K</div>
        <h1 style={{
          margin: 0,
          fontSize: 28,
          fontWeight: 900,
          background: "linear-gradient(135deg, #e94560, #fd9644, #00b894, #6c5ce7)",
          WebkitBackgroundClip: "text",
          WebkitTextFillColor: "transparent",
          letterSpacing: -0.5,
        }}>MARCH MADNESS 2026</h1>
        <div style={{
          fontSize: 12,
          color: "rgba(255,255,255,0.4)",
          marginTop: 4,
        }}>Bracket Predictor — Seed History + Team Strength + Injury Adjustments</div>
        <div style={{
          marginTop: 8,
          display: "flex",
          justifyContent: "center",
          gap: 8,
        }}>
          <button
            onClick={() => setShowMethodology(!showMethodology)}
            style={{
              background: "rgba(255,255,255,0.05)",
              border: "1px solid rgba(255,255,255,0.1)",
              color: "rgba(255,255,255,0.6)",
              borderRadius: 6,
              padding: "4px 12px",
              cursor: "pointer",
              fontSize: 11,
              fontFamily: "'JetBrains Mono', monospace",
            }}
          >{showMethodology ? "Hide" : "Show"} Methodology</button>
        </div>
      </div>
      
      {showMethodology && (
        <div style={{
          background: "rgba(255,255,255,0.03)",
          border: "1px solid rgba(255,255,255,0.08)",
          borderRadius: 12,
          padding: 16,
          marginBottom: 20,
          maxWidth: 700,
          marginLeft: "auto",
          marginRight: "auto",
          fontSize: 12,
          color: "rgba(255,255,255,0.6)",
          lineHeight: 1.6,
          fontFamily: "'Outfit', sans-serif",
        }}>
          <div style={{ fontSize: 10, color: "rgba(255,255,255,0.3)", fontFamily: "'JetBrains Mono', monospace", letterSpacing: 2, marginBottom: 8 }}>METHODOLOGY</div>
          <p style={{ margin: "0 0 8px" }}><strong style={{ color: "#fff" }}>Base Layer:</strong> Historical seed-vs-seed win rates from 1985–2025 NCAA tournament data (e.g., 1-seeds beat 16-seeds 99% of the time, 5v12 upsets happen 36% of the time).</p>
          <p style={{ margin: "0 0 8px" }}><strong style={{ color: "#fff" }}>Strength Adjustment:</strong> Each team gets a modifier based on: record quality, strength of schedule, offensive/defensive efficiency rankings, quality wins, and late-season momentum from ESPN/KenPom data.</p>
          <p style={{ margin: "0 0 8px" }}><strong style={{ color: "#fff" }}>Injury Penalty:</strong> Teams missing key players (Louisville's Brown, BYU's Saunders, Texas Tech's Toppin, UNC's Wilson, Kansas's Peterson volatility) get negative adjustments proportional to the player's impact.</p>
          <p style={{ margin: 0 }}><strong style={{ color: "#fff" }}>Round Scaling:</strong> Later rounds amplify team quality differences (the cream rises). Round multiplier increases from 1.0 (R64) to 2.5 (Championship).</p>
        </div>
      )}
      
      {/* Final Four + Champion */}
      <div style={{
        background: "linear-gradient(135deg, rgba(233,69,96,0.08), rgba(0,184,148,0.08), rgba(108,92,231,0.08))",
        border: "1px solid rgba(255,255,255,0.1)",
        borderRadius: 16,
        padding: 20,
        marginBottom: 20,
        textAlign: "center",
      }}>
        <div style={{
          fontSize: 10,
          letterSpacing: 4,
          color: "rgba(255,255,255,0.3)",
          fontFamily: "'JetBrains Mono', monospace",
          marginBottom: 12,
        }}>FINAL FOUR → CHAMPIONSHIP</div>
        
        <div style={{
          display: "flex",
          justifyContent: "center",
          alignItems: "center",
          gap: 16,
          flexWrap: "wrap",
          marginBottom: 16,
        }}>
          {/* Semi 1 */}
          <div style={{ display: "flex", flexDirection: "column", gap: 4, alignItems: "center" }}>
            <div style={{ fontSize: 9, color: "rgba(255,255,255,0.3)", fontFamily: "'JetBrains Mono', monospace" }}>EAST vs WEST</div>
            <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
              <span style={{
                padding: "4px 12px",
                background: finalFour.semi1.winner.name === finalFour.east.name ? `${REGION_COLORS.east.accent}33` : "rgba(255,255,255,0.03)",
                borderRadius: 6,
                fontSize: 13,
                fontWeight: finalFour.semi1.winner.name === finalFour.east.name ? 700 : 400,
                color: finalFour.semi1.winner.name === finalFour.east.name ? "#fff" : "rgba(255,255,255,0.4)",
              }}>({finalFour.east.seed}) {finalFour.east.name}</span>
              <span style={{ color: "rgba(255,255,255,0.2)", fontSize: 11 }}>vs</span>
              <span style={{
                padding: "4px 12px",
                background: finalFour.semi1.winner.name === finalFour.west.name ? `${REGION_COLORS.west.accent}33` : "rgba(255,255,255,0.03)",
                borderRadius: 6,
                fontSize: 13,
                fontWeight: finalFour.semi1.winner.name === finalFour.west.name ? 700 : 400,
                color: finalFour.semi1.winner.name === finalFour.west.name ? "#fff" : "rgba(255,255,255,0.4)",
              }}>({finalFour.west.seed}) {finalFour.west.name}</span>
            </div>
          </div>
          
          <div style={{ fontSize: 20, color: "rgba(255,255,255,0.1)" }}>|</div>
          
          {/* Semi 2 */}
          <div style={{ display: "flex", flexDirection: "column", gap: 4, alignItems: "center" }}>
            <div style={{ fontSize: 9, color: "rgba(255,255,255,0.3)", fontFamily: "'JetBrains Mono', monospace" }}>MIDWEST vs SOUTH</div>
            <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
              <span style={{
                padding: "4px 12px",
                background: finalFour.semi2.winner.name === finalFour.midwest.name ? `${REGION_COLORS.midwest.accent}33` : "rgba(255,255,255,0.03)",
                borderRadius: 6,
                fontSize: 13,
                fontWeight: finalFour.semi2.winner.name === finalFour.midwest.name ? 700 : 400,
                color: finalFour.semi2.winner.name === finalFour.midwest.name ? "#fff" : "rgba(255,255,255,0.4)",
              }}>({finalFour.midwest.seed}) {finalFour.midwest.name}</span>
              <span style={{ color: "rgba(255,255,255,0.2)", fontSize: 11 }}>vs</span>
              <span style={{
                padding: "4px 12px",
                background: finalFour.semi2.winner.name === finalFour.south.name ? `${REGION_COLORS.south.accent}33` : "rgba(255,255,255,0.03)",
                borderRadius: 6,
                fontSize: 13,
                fontWeight: finalFour.semi2.winner.name === finalFour.south.name ? 700 : 400,
                color: finalFour.semi2.winner.name === finalFour.south.name ? "#fff" : "rgba(255,255,255,0.4)",
              }}>({finalFour.south.seed}) {finalFour.south.name}</span>
            </div>
          </div>
        </div>
        
        {/* Champion */}
        <div style={{
          background: "linear-gradient(135deg, rgba(253,203,110,0.15), rgba(253,150,68,0.1))",
          border: "1px solid rgba(253,203,110,0.3)",
          borderRadius: 12,
          padding: 16,
          display: "inline-block",
        }}>
          <div style={{
            fontSize: 10,
            letterSpacing: 4,
            color: "#fdcb6e",
            fontFamily: "'JetBrains Mono', monospace",
            marginBottom: 4,
          }}>🏆 PREDICTED NATIONAL CHAMPION</div>
          <div style={{
            fontSize: 32,
            fontWeight: 900,
            color: "#fff",
            fontFamily: "'Outfit', sans-serif",
          }}>{finalFour.champion.name}</div>
          <div style={{
            fontSize: 12,
            color: "rgba(255,255,255,0.5)",
            fontFamily: "'JetBrains Mono', monospace",
          }}>#{finalFour.champion.seed} seed · {finalFour.champion.record}</div>
        </div>
      </div>
      
      {/* Team Detail */}
      {selectedTeam && (
        <div style={{ marginBottom: 20 }}>
          <TeamDetailPanel team={selectedTeam} onClose={() => setSelectedTeam(null)} />
        </div>
      )}
      
      {/* Region Brackets */}
      <div style={{
        display: "grid",
        gridTemplateColumns: "repeat(auto-fit, minmax(340px, 1fr))",
        gap: 16,
      }}>
        {Object.entries(TEAMS).map(([key, teams]) => (
          <RegionBracket
            key={key}
            regionKey={key}
            teams={teams}
            colors={REGION_COLORS[key]}
            onSelectTeam={setSelectedTeam}
            selectedTeam={selectedTeam}
          />
        ))}
      </div>
      
      {/* Upset Alerts */}
      <div style={{
        marginTop: 20,
        background: "rgba(233,69,96,0.05)",
        border: "1px solid rgba(233,69,96,0.15)",
        borderRadius: 12,
        padding: 16,
      }}>
        <div style={{
          fontSize: 10,
          letterSpacing: 3,
          color: "#e94560",
          fontFamily: "'JetBrains Mono', monospace",
          marginBottom: 10,
        }}>⚡ KEY UPSET PICKS & WATCH SPOTS</div>
        <div style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))",
          gap: 8,
          fontSize: 12,
          color: "rgba(255,255,255,0.7)",
          fontFamily: "'Outfit', sans-serif",
        }}>
          <div style={{ padding: 8, background: "rgba(255,255,255,0.02)", borderRadius: 6 }}>
            <span style={{ color: "#e94560", fontWeight: 700 }}>12 McNeese over 5 Vanderbilt</span> — #1 nationally in forced TOs vs a team whose star just returned from injury
          </div>
          <div style={{ padding: 8, background: "rgba(255,255,255,0.02)", borderRadius: 6 }}>
            <span style={{ color: "#fd9644", fontWeight: 700 }}>10 Santa Clara over 7 Kentucky</span> — UK 83rd in adj O since March; SCU has best WCC offense + Graves shoots 41% from 3
          </div>
          <div style={{ padding: 8, background: "rgba(255,255,255,0.02)", borderRadius: 6 }}>
            <span style={{ color: "#00b894", fontWeight: 700 }}>11 South Florida over 6 Louisville</span> — Brown Jr. OUT. Nelson (15.8/9.7) is best mid-major player. USF AAC champs.
          </div>
          <div style={{ padding: 8, background: "rgba(255,255,255,0.02)", borderRadius: 6 }}>
            <span style={{ color: "#6c5ce7", fontWeight: 700 }}>11 VCU over 6 North Carolina</span> — Wilson (top-5 pick) OUT with thumb. VCU 13-1 in last 14. A-10 tourney champs again.
          </div>
          <div style={{ padding: 8, background: "rgba(255,255,255,0.02)", borderRadius: 6 }}>
            <span style={{ color: "#74b9ff", fontWeight: 700 }}>13 Cal Baptist over 4 Kansas</span> — Daniels 23.2 PPG vs a Jekyll/Hyde team. Peterson injury volatility. Could go either way.
          </div>
          <div style={{ padding: 8, background: "rgba(255,255,255,0.02)", borderRadius: 6 }}>
            <span style={{ color: "#fdcb6e", fontWeight: 700 }}>9 Utah State over 8 Villanova</span> — Falslev "Excellent" in 8/11 shot zones. 4th straight dance. 42% from deep.
          </div>
        </div>
      </div>
      
      <div style={{
        marginTop: 16,
        textAlign: "center",
        fontSize: 10,
        color: "rgba(255,255,255,0.2)",
        fontFamily: "'JetBrains Mono', monospace",
      }}>
        Tap any team for scouting details · Model: seed history (1985–2025) + ESPN team strength + injury factors · Not financial advice 😄
      </div>
    </div>
  );
}
