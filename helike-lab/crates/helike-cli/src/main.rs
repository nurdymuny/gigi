//! helike: headless runners for helike-lab. Every command prints JSON and can write a receipt (--out); `verify`
//! replays a receipt and compares.

use clap::{Parser, Subcommand, ValueEnum};
use helike_core::helike::{self, Member, SectorData, Spectrum};
use helike_core::problem::Problem;
use helike_core::roots;
use helike_quantum::ritz::SolveSettings;
use helike_quantum::{projection, residual, run};
use helike_stream::{compare, Provenance, Receipt, Status};
use rayon::prelude::*;
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "helike", version, about = "Yang-Mills Hessian sectors, charge blocks and edges, with receipts")]
struct Cli {
    /// Write a receipt (JSON) here.
    #[arg(long, global = true)]
    out: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Copy, Clone, ValueEnum)]
enum Which {
    A,
    S,
}

impl From<Which> for Spectrum {
    fn from(w: Which) -> Self {
        match w {
            Which::A => Spectrum::A,
            Which::S => Spectrum::S,
        }
    }
}

#[derive(Subcommand)]
enum Cmd {
    /// Solve one sector problem from a JSON file (any frame algebra, metric, gauge algebra, field, representation).
    Solve { problem: PathBuf },
    /// The edge of the twist's region in the SU(2) family at (t, y), sectors j <= jmax.
    Edge {
        #[arg(long)]
        t: f64,
        #[arg(long)]
        y: f64,
        #[arg(long, value_enum, default_value = "a")]
        spectrum: Which,
        #[arg(long, default_value_t = 12)]
        jmax: usize,
        #[arg(long, default_value_t = -3.0)]
        lo: f64,
        #[arg(long, default_value_t = 1.0)]
        hi: f64,
        #[arg(long, default_value_t = 81)]
        n: usize,
    },
    /// By Enough Table 1: x*_A / x*_S at t = 2, 3, 4, 6, 8 and y = 0.5, 1.5, sqrt t.
    Table1 {
        #[arg(long, default_value_t = 12)]
        jmax: usize,
    },
    /// phi_A (or phi_S) on an (x, t) grid at fixed y: the edge forming.
    Map {
        #[arg(long)]
        y: f64,
        #[arg(long, default_value_t = 2.0)]
        t_min: f64,
        #[arg(long, default_value_t = 8.0)]
        t_max: f64,
        #[arg(long, default_value_t = 25)]
        nt: usize,
        #[arg(long, default_value_t = -3.0)]
        x_min: f64,
        #[arg(long, default_value_t = 1.0)]
        x_max: f64,
        #[arg(long, default_value_t = 41)]
        nx: usize,
        #[arg(long, value_enum, default_value = "a")]
        spectrum: Which,
        #[arg(long, default_value_t = 12)]
        jmax: usize,
    },
    /// The ladder at one member: flux spectrum, twist, sufficient floor condition and gauge-orthogonal margin per sector.
    Ladder {
        #[arg(long)]
        x: f64,
        #[arg(long)]
        y: f64,
        #[arg(long)]
        t: f64,
        #[arg(long, default_value_t = 12)]
        jmax: usize,
    },
    /// Where the Berger path crosses the edges (t_A for H + GG^*, t* for the gauge slice).
    Berger {
        #[arg(long, default_value_t = 12)]
        jmax: usize,
    },
    /// The sourced condensate at one point by positive-cone Ritz (matrix-free): levels, Helike and competitor.
    Cone {
        #[arg(long, default_value_t = 1.15)]
        s: f64,
        #[arg(long, default_value_t = 3.0)]
        t: f64,
        #[arg(long)]
        g2: f64,
        #[arg(long, default_value_t = 6)]
        mq: usize,
        /// solves use exact Kronecker moments; the independent grid cross-check uses order (Mq + 2) + extra + 2 per axis
        #[arg(long, default_value_t = 4)]
        extra: usize,
    },
    /// The two weak-coupling controls (s = 1.15, t = 3, g^2 = 0.004, 0.002) at one cutoff, with the frozen rule.
    ConeControls {
        #[arg(long, default_value_t = 6)]
        mq: usize,
        #[arg(long, default_value_t = 4)]
        extra: usize,
    },
    /// Report-only state support and omitted-shell residuals; no larger eigensolve or certificate.
    ConeSupport {
        #[arg(long)]
        s: f64,
        #[arg(long)]
        t: f64,
        #[arg(long)]
        g2: f64,
        #[arg(long)]
        mq: usize,
        #[arg(long)]
        support_out: PathBuf,
    },
    /// One nested residual enrichment of a saved support diagnostic; no ambient eigensolve or certificate.
    ConeEnrich {
        #[arg(long)]
        support: PathBuf,
        #[arg(long)]
        states_out: PathBuf,
    },
    /// Basis convergence of the controls between two cutoffs at the frozen 0.003 rule.
    ConeConvergence {
        #[arg(long, default_value_t = 6)]
        from: usize,
        #[arg(long, default_value_t = 8)]
        to: usize,
        #[arg(long, default_value_t = 4)]
        extra: usize,
    },
    /// Rigorous enclosures of the full-operator residuals ‖(H − E)ψ‖/‖ψ‖ of saved trial states (exact centred moment
    /// sum in outward-rounded interval arithmetic). Bounds only: no gate is evaluated here.
    ConeResiduals {
        /// saved states exported by oracle/export_quasimode_states.py
        #[arg(long, default_value = "oracle/quasimode_states_e21087a1.json")]
        states: PathBuf,
        /// restrict to one coupling
        #[arg(long)]
        g2: Option<f64>,
        /// restrict to one cutoff
        #[arg(long)]
        mq: Option<usize>,
    },
    /// The exact SO(2) charge projector by its norm bound (quasimode construction registration §2): enclosed charge
    /// defects ‖(L − m)ψ‖/‖ψ‖ and residuals of the frozen binary64 trial, combined with the Arb tail and leakage
    /// endpoints into q ≤ ‖P_m ηψ‖ and (r + ℓ)/q ≥ the projected-sector residual. Bounds only: no gate is evaluated.
    /// Kato–Temple ordering of Helike against the competitor at the saved controls
    /// (docs/research/helike-temple-ordering-registration-2026-11-25.md): the linear moment s (two routes), Lemma 1's
    /// Rayleigh-quotient enclosure, Lemma 3's windows and the Temple lower bound under the frozen ρ_C. ORDERED is
    /// only possible with `--assume S_C`, and is then conditional on (A) and S_C.
    /// Solve one point with exact moments and export its four quasimode trial states (ground, Helike pair with its
    /// chiral mixing, competitor) as bit patterns, for the per-point domain-validity gate. Writes `--states-out`.
    /// Heat-signal data at one point (registration helike-heat-signal-registration-2026-11-26.md, amendment 1):
    /// levels, identification, bare reference amplitudes and monomial-source overlaps at both resolved orders.
    ConeHeat {
        #[arg(long)]
        s: f64,
        #[arg(long)]
        t: f64,
        #[arg(long)]
        g2: f64,
        #[arg(long)]
        mq: usize,
    },
    ConeExport {
        #[arg(long)]
        s: f64,
        #[arg(long)]
        t: f64,
        #[arg(long)]
        g2: f64,
        #[arg(long)]
        mq: usize,
        #[arg(long)]
        states_out: PathBuf,
    },
    /// M8: solve one point with exact moments and export every Ritz level (vectors, residuals, degenerate clusters with
    /// charges, dressed overlaps, identification). Writes `--levels-out`. Report-only data.
    ConeLevels {
        #[arg(long)]
        s: f64,
        #[arg(long)]
        t: f64,
        #[arg(long)]
        g2: f64,
        #[arg(long)]
        mq: usize,
        #[arg(long)]
        levels_out: PathBuf,
    },
    /// M8: signed physical overlaps <Psi_a(A), Psi_b(B)> between every level of two cone-levels exports, on B's grid
    /// at the given quadrature order (A3.1). Report-only data.
    ConeOverlap {
        #[arg(long)]
        a: PathBuf,
        #[arg(long)]
        b: PathBuf,
        #[arg(long)]
        order: usize,
    },
    ConeTemple {
        #[arg(long, default_value = "oracle/quasimode_states_e21087a1.json")]
        states: PathBuf,
        #[arg(long, default_value = "oracle/projection_inputs_c31aab8e.json")]
        inputs: PathBuf,
        /// the separation hypothesis to assume (only "S_C" is registered)
        #[arg(long)]
        assume: Option<String>,
        /// exact centre shift for the polarization cross-check
        #[arg(long, default_value_t = 0.125)]
        delta: f64,
        #[arg(long)]
        g2: Option<f64>,
        #[arg(long)]
        mq: Option<usize>,
    },
    ConeSectors {
        /// saved states exported by oracle/export_quasimode_states.py
        #[arg(long, default_value = "oracle/quasimode_states_e21087a1.json")]
        states: PathBuf,
        /// frozen trial and Arb endpoints exported by oracle/export_projection_inputs.py
        #[arg(long, default_value = "oracle/projection_inputs_c31aab8e.json")]
        inputs: PathBuf,
        #[arg(long)]
        g2: Option<f64>,
        #[arg(long)]
        mq: Option<usize>,
    },
    /// Replay a receipt's command and compare its results (relative tolerance).
    Verify {
        receipt: PathBuf,
        #[arg(long, default_value_t = 1e-9)]
        rel_tol: f64,
    },
}

fn source_root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn run(cmd: &Cmd) -> Result<(Value, Value), String> {
    Ok(match cmd {
        Cmd::Solve { problem } => {
            let p: Problem = serde_json::from_str(&std::fs::read_to_string(problem).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            let r = p.solve();
            (serde_json::to_value(&p).unwrap(), serde_json::to_value(&r).unwrap())
        }
        Cmd::Edge { t, y, spectrum, jmax, lo, hi, n } => {
            let x = helike::edge((*spectrum).into(), *y, *t, *jmax, *lo, *hi, *n);
            (json!({"t": t, "y": y, "spectrum": format!("{:?}", Spectrum::from(*spectrum)), "jmax": jmax, "lo": lo, "hi": hi, "n": n}), json!({"x_star": x}))
        }
        Cmd::Table1 { jmax } => {
            let cells: Vec<Value> = [2.0, 3.0, 4.0, 6.0, 8.0]
                .par_iter()
                .flat_map(|&t: &f64| {
                    [0.5, 1.5, t.sqrt()]
                        .into_par_iter()
                        .map(move |y| {
                            let xa = helike::edge(Spectrum::A, y, t, *jmax, -3.0, 1.0, 81);
                            let xs = helike::edge(Spectrum::S, y, t, *jmax, -3.0, 1.0, 81);
                            json!({"t": t, "y": y, "x_star_A": xa, "x_star_S": xs})
                        })
                        .collect::<Vec<_>>()
                })
                .collect();
            (json!({"jmax": jmax}), json!({"cells": cells}))
        }
        Cmd::Map { y, t_min, t_max, nt, x_min, x_max, nx, spectrum, jmax } => {
            let ts = roots::linspace(*t_min, *t_max, *nt);
            let xs = roots::linspace(*x_min, *x_max, *nx);
            let which: Spectrum = (*spectrum).into();
            let grid: Vec<Vec<f64>> = ts
                .par_iter()
                .map(|&t| xs.iter().map(|&x| helike::phi(&Member::new(x, *y, t), which, *jmax)).collect())
                .collect();
            (json!({"y": y, "t": ts, "x": xs, "spectrum": format!("{which:?}"), "jmax": jmax}), json!({"phi": grid}))
        }
        Cmd::Ladder { x, y, t, jmax } => {
            let m = Member::new(*x, *y, *t);
            let sectors: Vec<Value> = (0..=*jmax)
                .into_par_iter()
                .map(|j| {
                    let lam = m.twist(j);
                    let sd = SectorData::new(&m, j);
                    json!({"j": j, "twist": lam, "floor_sufficient": m.floor_sufficient(j),
                           "margin_slice": sd.margin_s(lam), "margin_a": sd.margin_a(lam).0})
                })
                .collect();
            (json!({"x": x, "y": y, "t": t, "jmax": jmax}),
             json!({"fibre_field": m.fibre_field(), "cross_field": m.cross_field(), "flux_spectrum": m.flux_spectrum(),
                    "x_flux": helike::x_flux(*y, *t), "sectors": sectors}))
        }
        Cmd::Berger { jmax } => {
            let ta = roots::brent(|t| helike::phi(&Member::berger(t), Spectrum::A, *jmax), 2.70, 2.76, 1e-13, 200);
            let ts = roots::brent(|t| helike::phi(&Member::berger(t), Spectrum::S, *jmax), 2.68, 2.72, 1e-13, 200);
            (json!({"jmax": jmax, "bracket_A": [2.70, 2.76], "bracket_S": [2.68, 2.72]}), json!({"t_A": ta, "t_star": ts}))
        }
        Cmd::Cone { s, t, g2, mq, extra } => {
            let r = run::solve_point(*s, *t, *g2, *mq, *extra, &SolveSettings::default())?;
            (json!({"s": s, "t": t, "g2": g2, "Mq": mq, "extra": extra, "solver": SolveSettings::default()}), serde_json::to_value(&r).unwrap())
        }
        Cmd::ConeSupport { s, t, g2, mq, support_out } => {
            let r = helike_quantum::support::diagnose(*s, *t, *g2, *mq, &SolveSettings::default())?;
            std::fs::write(support_out, serde_json::to_string(&r).unwrap()).map_err(|e| e.to_string())?;
            (json!({"s":s, "t":t, "g2":g2, "Mq":mq, "probe_Mq":mq+2, "solver":SolveSettings::default()}),
             json!({"identification":r.identification, "seconds":r.seconds, "certified":false, "full_grid_released":false,
                    "support_sha256_lf":helike_stream::text_sha256(support_out)}))
        }
        Cmd::ConeEnrich { support, states_out } => {
            let text=std::fs::read_to_string(support).map_err(|e| e.to_string())?;
            let r=helike_quantum::enrichment::enrich(&text)?;
            std::fs::write(states_out,serde_json::to_string(&r).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            (json!({"support_sha256_lf":helike_stream::text_sha256(support),"solver":SolveSettings::default()}),
             json!({"identification":r.identification,"seconds":r.seconds,"certified":false,"full_grid_released":false,
                    "enrichment_sha256_lf":helike_stream::text_sha256(states_out)}))
        }
        Cmd::ConeControls { mq, extra } => {
            let r = run::controls(*mq, *extra, &SolveSettings::default())?;
            (json!({"Mq": mq, "extra": extra, "solver": SolveSettings::default()}), serde_json::to_value(&r).unwrap())
        }
        Cmd::ConeConvergence { from, to, extra } => {
            let r = run::convergence([*from, *to], *extra, &SolveSettings::default());
            (json!({"ladder": [from, to], "extra": extra, "solver": SolveSettings::default()}), serde_json::to_value(&r).unwrap())
        }
        Cmd::ConeResiduals { states, g2, mq } => {
            let path = if states.is_absolute() { states.clone() } else { source_root().join(states) };
            let saved = residual::load_saved_states(&path)?;
            let mut rows = vec![];
            for c in &saved.controls {
                if g2.is_some_and(|g| g != c.g2) || mq.is_some_and(|m| m != c.mq) {
                    continue;
                }
                eprintln!("enclosing g2 = {}, Mq = {} ...", c.g2, c.mq);
                let r = residual::enclose_control(c, residual::Mutation::None)?;
                eprintln!("  {} groups, {} pairs, {:.1} s", r.group_count, r.moments.pairs, r.seconds);
                rows.push(r);
            }
            if rows.is_empty() {
                return Err("no saved control matches the selection".into());
            }
            (json!({"states": states, "states_sha256_lf": helike_stream::text_sha256(&path),
                    "parent_receipt_sha256": saved.parent_receipt_sha256,
                    "residual_receipt_sha256": saved.residual_receipt_sha256, "g2": g2, "Mq": mq,
                    "arithmetic": "binary64 intervals, round-to-nearest then one ulp outward; exp by Cody-Waite reduction and Taylor with explicit remainder",
                    "scope": "uncut residual of each saved trial state under the registered operator; no cutoff leakage, no gate, no spectral statement"}),
             json!({"controls": rows}))
        }
        Cmd::ConeHeat { s, t, g2, mq } => {
            let r = run::heat_point(*s, *t, *g2, *mq, &SolveSettings::default())?;
            (json!({"s": s, "t": t, "g2": g2, "Mq": mq, "solver": SolveSettings::default(),
                    "registration": "docs/research/helike-heat-signal-registration-2026-11-26.md"}),
             serde_json::to_value(&r).unwrap())
        }
        Cmd::ConeExport { s, t, g2, mq, states_out } => {
            let e = helike_quantum::export::export_point(*s, *t, *g2, *mq, &SolveSettings::default())?;
            std::fs::write(states_out, serde_json::to_string(&e).unwrap()).map_err(|x| x.to_string())?;
            (json!({"s": s, "t": t, "g2": g2, "Mq": mq, "states_out": states_out}),
             json!({"compressed_charges": e.compressed_charges, "pair_weights": e.pair_weights,
                    "competitor_weight": e.competitor_weight, "level_indices": e.level_indices,
                    "states_sha256_lf": helike_stream::text_sha256(states_out)}))
        }
        Cmd::ConeLevels { s, t, g2, mq, levels_out } => {
            let e = helike_quantum::export::export_levels(*s, *t, *g2, *mq, &SolveSettings::default())?;
            std::fs::write(levels_out, serde_json::to_string(&e).unwrap()).map_err(|x| x.to_string())?;
            (json!({"s": s, "t": t, "g2": g2, "Mq": mq, "levels_out": levels_out,
                    "registration": "docs/research/helike-M8-weighted-geometry-registration-2026-11-26.md"}),
             json!({"levels": e.levels, "levels_sha256_lf": helike_stream::text_sha256(levels_out)}))
        }
        Cmd::ConeOverlap { a, b, order } => {
            let la = helike_quantum::m8::Levels::from_json(&std::fs::read_to_string(a).map_err(|x| x.to_string())?)?;
            let lb = helike_quantum::m8::Levels::from_json(&std::fs::read_to_string(b).map_err(|x| x.to_string())?)?;
            let t0 = std::time::Instant::now();
            let o = helike_quantum::m8::overlap(&la, &lb, *order)?;
            (json!({"a": a, "b": b, "order": order, "a_sha256_lf": helike_stream::text_sha256(a),
                    "b_sha256_lf": helike_stream::text_sha256(b),
                    "registration": "docs/research/helike-M8-weighted-geometry-registration-2026-11-26.md"}),
             json!({"shape": [la.k, lb.k], "overlap": o, "seconds": t0.elapsed().as_secs_f64()}))
        }
        Cmd::ConeTemple { states, inputs, assume, delta, g2, mq } => {
            let sep = match assume.as_deref() {
                None => None,
                Some("S_C") => Some(helike_quantum::temple::Separation::SC),
                Some(o) => return Err(format!("unregistered separation hypothesis {o:?}; only S_C is registered")),
            };
            let root = |p: &PathBuf| if p.is_absolute() { p.clone() } else { source_root().join(p) };
            let (spath, ipath) = (root(states), root(inputs));
            let saved = residual::load_saved_states(&spath)?;
            let arb = projection::load_projection_inputs(&ipath)?;
            let states_sha = helike_stream::text_sha256(&spath);
            if Some(&arb.states_file_sha256) != states_sha.as_ref() {
                return Err("projection inputs were exported from a different saved-state file".into());
            }
            let mut rows = vec![];
            for c in &saved.controls {
                if g2.is_some_and(|g| g != c.g2) || mq.is_some_and(|m| m != c.mq) {
                    continue;
                }
                let a = arb.controls.iter().find(|a| a.g2 == c.g2 && a.mq == c.mq).ok_or("no Arb inputs for this control")?;
                eprintln!("temple g2 = {}, Mq = {} ...", c.g2, c.mq);
                let r = helike_quantum::temple::temple_control(c, a, *delta, sep, helike_quantum::temple::TempleMutation::None)?;
                eprintln!("  {} (margin {:.6}), {:.1} s", r.decision.verdict, r.decision.margin_lower, r.seconds);
                rows.push(r);
            }
            if rows.is_empty() {
                return Err("no saved control matches the selection".into());
            }
            (json!({"states": states, "states_sha256_lf": states_sha, "inputs": inputs,
                    "inputs_sha256_lf": helike_stream::text_sha256(&ipath), "assume": assume, "delta": delta,
                    "registration": "docs/research/helike-temple-ordering-registration-2026-11-25.md",
                    "scope": "bounds are interval enclosures; each verdict is conditional on (A) and S_C; the reverse order is never claimed; no grid release"}),
             json!({"controls": rows}))
        }
        Cmd::ConeSectors { states, inputs, g2, mq } => {
            let root = |p: &PathBuf| if p.is_absolute() { p.clone() } else { source_root().join(p) };
            let (spath, ipath) = (root(states), root(inputs));
            let saved = residual::load_saved_states(&spath)?;
            let arb = projection::load_projection_inputs(&ipath)?;
            let states_sha = helike_stream::text_sha256(&spath);
            if Some(&arb.states_file_sha256) != states_sha.as_ref() {
                return Err("projection inputs were exported from a different saved-state file".into());
            }
            let mut rows = vec![];
            for c in &saved.controls {
                if g2.is_some_and(|g| g != c.g2) || mq.is_some_and(|m| m != c.mq) {
                    continue;
                }
                let a = arb.controls.iter().find(|a| a.g2 == c.g2 && a.mq == c.mq).ok_or("no Arb inputs for this control")?;
                eprintln!("projecting g2 = {}, Mq = {} ...", c.g2, c.mq);
                let r = projection::project_control(c, a, residual::Mutation::None)?;
                eprintln!("  {:.1} s", r.seconds);
                rows.push(r);
            }
            if rows.is_empty() {
                return Err("no saved control matches the selection".into());
            }
            (json!({"states": states, "states_sha256_lf": states_sha, "inputs": inputs,
                    "inputs_sha256_lf": helike_stream::text_sha256(&ipath),
                    "cutoff_receipt_sha256": arb.cutoff_receipt_sha256, "cutoff_source_revision": arb.cutoff_source_revision,
                    "construction_receipt_sha256": arb.construction_receipt_sha256, "g2": g2, "Mq": mq,
                    "trial": "the binary64 product coeff @ mixing frozen in the Arb run's trial NPZ; each coefficient is the interval hull of that value and the exact product",
                    "arithmetic": "binary64 intervals, round-to-nearest then one ulp outward; tail and leakage are the Arb receipt's upper endpoints rounded upward to binary64",
                    "bound": "q^2 = (1 - t)^2 - c^2 > 0 gives ||P_m eta psi|| >= q and ||(H - E) P_m eta psi|| / ||P_m eta psi|| <= (r + l)/q; E is the saved centre",
                    "scope": "charge-sector projection of each saved trial state; no separation hypothesis, no Temple bound, no gate, no level identification"}),
             json!({"controls": rows}))
        }
        Cmd::Verify { .. } => unreachable!(),
    })
}

fn main() {
    let cli = Cli::parse();
    if let Cmd::Verify { receipt, rel_tol } = &cli.cmd {
        let r = Receipt::read(receipt).unwrap_or_else(|e| panic!("cannot read receipt: {e}"));
        let mut args = r.command.clone();
        // drop --out VALUE from the recorded command
        if let Some(k) = args.iter().position(|a| a == "--out") {
            args.drain(k..(k + 2).min(args.len()));
        }
        let replay = Cli::try_parse_from(&args).unwrap_or_else(|e| panic!("cannot parse recorded command: {e}"));
        let (_, results) = run(&replay.cmd).unwrap_or_else(|e| panic!("{e}"));
        let diffs = compare(&r.results, &results, *rel_tol);
        let ok = diffs.is_empty();
        println!("{}", serde_json::to_string_pretty(&json!({"receipt": receipt, "replayed": args, "match": ok, "differences": diffs})).unwrap());
        std::process::exit(if ok { 0 } else { 1 });
    }
    let (params, results) = run(&cli.cmd).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(2)
    });
    println!("{}", serde_json::to_string_pretty(&results).unwrap());
    if let Some(out) = &cli.out {
        let prov = Provenance::collect("helike", env!("CARGO_PKG_VERSION"), &source_root(), rayon::current_num_threads());
        let status = if matches!(cli.cmd, Cmd::ConeResiduals { .. } | Cmd::ConeSectors { .. } | Cmd::ConeTemple { .. }) { Status::IntervalEnclosure } else { Status::Computed };
        let receipt = Receipt::new(prov, std::env::args().collect(), status, params, results);
        receipt.write(out).unwrap_or_else(|e| panic!("cannot write receipt: {e}"));
        eprintln!("receipt: {}", out.display());
    }
}
