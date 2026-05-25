//! dream_demo — show what DREAM does next to SAMPLE.
//!
//! Same Kähler bundle. Same Friston SDE
//!
//!     ẋ = −∇H(x) dt  +  √(2T) dW
//!
//! Two temperatures. Two completely different behaviors. ASCII
//! scatter-plot side-by-side so you can SEE the wandering.
//!
//! Run:
//!     cargo run --release --features kahler --bin dream_demo

#![cfg(feature = "kahler")]

use gigi::geometry::{
    from_isotropic_gaussian, ClosedTwoForm, FlowConfig, TwoForm,
};

const X_MIN: f64 = -7.0;
const X_MAX: f64 = 7.0;
const Y_MIN: f64 = -7.0;
const Y_MAX: f64 = 7.0;
const COLS: usize = 56;
const ROWS: usize = 22;

fn main() {
    println!();
    println!("┌──────────────────────────────────────────────────────────────────────┐");
    println!("│                   DREAM   vs   SAMPLE                                │");
    println!("│           (what does the brain see at different temperatures?)       │");
    println!("└──────────────────────────────────────────────────────────────────────┘");
    println!();
    println!("Same Kähler bundle (isotropic Gaussian density at origin, σ² = 1.0).");
    println!("Same SDE:  ẋ = -∇H(x) dt + √(2T) dW  with H = -log p.");
    println!();
    println!("The ONLY thing different between the two plots is T (temperature).");
    println!();

    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 1.0, -1.0, 0.0], 2).unwrap(),
    );
    let mu = vec![0.0, 0.0];
    let sigma_sq = 1.0;
    let flow = from_isotropic_gaussian(b, mu.clone(), sigma_sq).unwrap();

    // ── SAMPLE @ T = 1.0 ── canonical Langevin, stationary draws ──
    let sample_config = FlowConfig {
        dt: 0.05,
        temperature: 1.0,
        n_steps: 1,
        burn_in: 2_000,
        seed: Some(42),
    };
    let samples = flow
        .sample_many(&[0.0, 0.0], &sample_config, 200, 5)
        .unwrap();

    // ── DREAM @ T = 4.0 ── high-temperature trajectory ──
    let dream_config = FlowConfig {
        dt: 0.05,
        temperature: 4.0,
        n_steps: 1_000,
        burn_in: 0,
        seed: Some(42),
    };
    let dream_path = flow.dream(&[0.0, 0.0], &dream_config).unwrap();

    // Render side-by-side scatter plots.
    let left = render(&samples, '*');
    let right = render(&dream_path, '·');

    println!(
        "  ┌─ SAMPLE  T = 1.0  (200 stationary draws) ┐     ┌─ DREAM  T = 4.0  (1000-step trajectory)  ┐"
    );
    for (l, r) in left.iter().zip(right.iter()) {
        println!("  │ {} │     │ {} │", l, r);
    }
    println!(
        "  └────────────────────────────────────────── ┘     └─────────────────────────────────────────── ┘"
    );
    println!("    legend:  '+' = density mean   '*' = SAMPLE draw  |  '·' = DREAM trajectory point");
    println!();

    // ── Quantitative comparison ──────────────────────────────────
    let stats = |pts: &[Vec<f64>]| -> (f64, f64, f64) {
        let dists: Vec<f64> = pts
            .iter()
            .map(|p| (p[0].powi(2) + p[1].powi(2)).sqrt())
            .collect();
        let mean = dists.iter().sum::<f64>() / dists.len() as f64;
        let max = dists.iter().cloned().fold(0.0_f64, f64::max);
        let var = dists
            .iter()
            .map(|d| (d - mean).powi(2))
            .sum::<f64>()
            / dists.len() as f64;
        (mean, max, var)
    };
    let (s_mean, s_max, _s_var) = stats(&samples);
    let (d_mean, d_max, _d_var) = stats(&dream_path);

    println!("  ─── distance from origin (where the density is concentrated) ───");
    println!("    SAMPLE  mean = {:.3}   max = {:.3}   (stationary around the data)", s_mean, s_max);
    println!("    DREAM   mean = {:.3}   max = {:.3}   (visits ~{:.1}× further)",
             d_mean, d_max, d_mean / s_mean);
    println!();

    // ── A few raw values, to make it concrete ──────────────────
    println!("  ─── first 5 SAMPLE draws (stationary; each is a fresh point) ───");
    for (i, s) in samples.iter().take(5).enumerate() {
        println!("    sample[{}]   ({:6.3}, {:6.3})", i, s[0], s[1]);
    }
    println!();

    println!("  ─── first 10 DREAM steps (trajectory; each follows from the last) ───");
    for (i, s) in dream_path.iter().enumerate().take(10) {
        println!("    step[{:3}]   ({:6.3}, {:6.3})", i * 100, s[0], s[1]);
    }
    println!("    ⋮");
    println!("    step[999]  ({:6.3}, {:6.3})   ← walked WAY out by the end",
             dream_path.last().unwrap()[0], dream_path.last().unwrap()[1]);
    println!();

    println!("┌──────────────────────────────────────────────────────────────────────┐");
    println!("│  Same equation. Different T. SAMPLE hugs the density; DREAM         │");
    println!("│  wanders. The gradient still pulls (you can see it tries to come    │");
    println!("│  back to origin), but the √(2T) noise term dominates and pushes     │");
    println!("│  the trajectory into states the data never visited.                 │");
    println!("│                                                                      │");
    println!("│  This is Friston's free-energy minimization at a higher temperature │");
    println!("│  — what the brain does in REM sleep. The same math, the same        │");
    println!("│  Kähler bundle, one knob turned up.                                  │");
    println!("└──────────────────────────────────────────────────────────────────────┘");
}

// ── ASCII scatter plot ────────────────────────────────────────────

fn render(points: &[Vec<f64>], marker: char) -> Vec<String> {
    let mut grid: Vec<Vec<char>> = vec![vec![' '; COLS]; ROWS];
    // Light dotted background for empty cells.
    for r in 0..ROWS {
        for c in 0..COLS {
            grid[r][c] = ' ';
        }
    }
    // Plot the density mean as '+' at the origin.
    if let Some((r, c)) = world_to_grid(0.0, 0.0) {
        grid[r][c] = '+';
    }
    // Plot all points.
    for p in points {
        if let Some((r, c)) = world_to_grid(p[0], p[1]) {
            // If already a marker, overwrite (showing point density at
            // that cell is fine — overwrites the '+' if a sample
            // lands exactly on origin).
            if grid[r][c] == ' ' || grid[r][c] == '+' {
                grid[r][c] = marker;
            }
        }
    }
    grid.into_iter().map(|row| row.into_iter().collect()).collect()
}

fn world_to_grid(x: f64, y: f64) -> Option<(usize, usize)> {
    if x < X_MIN || x > X_MAX || y < Y_MIN || y > Y_MAX {
        return None;
    }
    let cx = ((x - X_MIN) / (X_MAX - X_MIN) * (COLS as f64 - 1.0)) as usize;
    // Y flipped so up is positive on the plot.
    let cy = ((Y_MAX - y) / (Y_MAX - Y_MIN) * (ROWS as f64 - 1.0)) as usize;
    Some((cy.min(ROWS - 1), cx.min(COLS - 1)))
}
