//! helike: headless runners for helike-lab. Every command prints JSON and can write a receipt (--out); `verify`
//! replays a receipt and compares.

use clap::{Parser, Subcommand, ValueEnum};
use helike_core::helike::{self, Member, SectorData, Spectrum};
use helike_core::problem::Problem;
use helike_core::roots;
use helike_quantum::ritz::SolveSettings;
use helike_quantum::run;
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
        /// quadrature order per axis is (Mq + 2) + extra; a second order (extra + 2) is always checked
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
    /// Basis convergence of the controls between two cutoffs at the frozen 0.003 rule.
    ConeConvergence {
        #[arg(long, default_value_t = 6)]
        from: usize,
        #[arg(long, default_value_t = 8)]
        to: usize,
        #[arg(long, default_value_t = 4)]
        extra: usize,
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
        Cmd::ConeControls { mq, extra } => {
            let r = run::controls(*mq, *extra, &SolveSettings::default())?;
            (json!({"Mq": mq, "extra": extra, "solver": SolveSettings::default()}), serde_json::to_value(&r).unwrap())
        }
        Cmd::ConeConvergence { from, to, extra } => {
            let r = run::convergence([*from, *to], *extra, &SolveSettings::default());
            (json!({"ladder": [from, to], "extra": extra, "solver": SolveSettings::default()}), serde_json::to_value(&r).unwrap())
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
        let receipt = Receipt::new(prov, std::env::args().collect(), Status::Computed, params, results);
        receipt.write(out).unwrap_or_else(|e| panic!("cannot write receipt: {e}"));
        eprintln!("receipt: {}", out.display());
    }
}
