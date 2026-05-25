//! dream_anisotropic_demo — what L13.3's diagonal fit gives DREAM.
//!
//! The original dream_demo used an isotropic Gaussian (σ² same on
//! every axis) so iso vs diag fit gave identical results. Real
//! bundles (Marcella's token fibers especially) are *anisotropic*:
//! different fiber dimensions have very different natural scales.
//!
//! Under the isotropic fit, DREAM uses a single averaged σ². The
//! gradient pulls equally toward μ on every axis, so the dream
//! wanders symmetrically — WRONG on an anisotropic density.
//!
//! Under the diagonal fit (L13.3), DREAM uses per-axis σ²ᵢ. The
//! gradient pull is WEAKER on high-variance axes (1/σ²ᵢ is smaller),
//! so the dream wanders correctly more along the wide axes. RIGHT.
//!
//! Run:
//!     cargo run --release --features kahler --bin dream_anisotropic_demo

#![cfg(feature = "kahler")]

use gigi::geometry::{
    from_diagonal_gaussian, from_isotropic_gaussian, ClosedTwoForm, FlowConfig,
    TwoForm,
};

const X_MIN: f64 = -8.0;
const X_MAX: f64 = 8.0;
const Y_MIN: f64 = -8.0;
const Y_MAX: f64 = 8.0;
const COLS: usize = 52;
const ROWS: usize = 22;

fn main() {
    println!();
    println!("┌──────────────────────────────────────────────────────────────────────┐");
    println!("│  DREAM on an anisotropic bundle:  ISOTROPIC fit  vs  DIAGONAL fit   │");
    println!("│  (L13.3 — Marcella probe Finding 3 fix)                              │");
    println!("└──────────────────────────────────────────────────────────────────────┘");
    println!();
    println!("Synthetic Gaussian density with σ²_x = 0.5 (narrow) and σ²_y = 4.0 (wide).");
    println!("8× variance ratio across axes — the kind of anisotropy Marcella sees on");
    println!("learned token fibers.");
    println!();
    println!("Same DREAM (T = 4.0, 1500 steps, seed 42) from both flows. Only");
    println!("difference: which constructor built the underlying GenerativeFlow.");
    println!();

    let mu = vec![0.0, 0.0];
    let sigma_sq_per_field = vec![0.5_f64, 4.0_f64];
    let iso_sigma_sq = (sigma_sq_per_field[0] + sigma_sq_per_field[1]) / 2.0; // 2.25

    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 1.0, -1.0, 0.0], 2).unwrap(),
    );

    // ── Build both flows on identical (μ, B), different fits ──
    let iso_flow = from_isotropic_gaussian(b.clone(), mu.clone(), iso_sigma_sq).unwrap();
    let diag_flow = from_diagonal_gaussian(b, mu.clone(), sigma_sq_per_field.clone())
        .unwrap();

    // ── DREAM from origin, identical FlowConfig ──
    let config = FlowConfig {
        dt: 0.01,
        temperature: 4.0,
        n_steps: 1_500,
        burn_in: 0,
        seed: Some(42),
    };
    let iso_path = iso_flow.dream(&mu, &config).unwrap();
    let diag_path = diag_flow.dream(&mu, &config).unwrap();

    // ── Render side-by-side ──
    let left = render(&iso_path, '·');
    let right = render(&diag_path, '·');

    println!(
        "  ┌─ ISOTROPIC fit   σ² = {:.2} (averaged)   ┐     ┌─ DIAGONAL fit   σ²_x = {:.2}, σ²_y = {:.2}    ┐",
        iso_sigma_sq, sigma_sq_per_field[0], sigma_sq_per_field[1]
    );
    for (l, r) in left.iter().zip(right.iter()) {
        println!("  │ {} │     │ {} │", l, r);
    }
    println!(
        "  └────────────────────────────────────────────┘     └────────────────────────────────────────────┘"
    );
    println!("    legend:  '+' = density mean   '·' = DREAM trajectory point");
    println!();

    // ── Per-axis spread diagnostics ──
    let axis_stats = |path: &[Vec<f64>]| -> (f64, f64, f64, f64) {
        let mut mean_x = 0.0_f64;
        let mut mean_y = 0.0_f64;
        for p in path {
            mean_x += p[0];
            mean_y += p[1];
        }
        let n = path.len() as f64;
        mean_x /= n;
        mean_y /= n;
        let var_x: f64 =
            path.iter().map(|p| (p[0] - mean_x).powi(2)).sum::<f64>() / n;
        let var_y: f64 =
            path.iter().map(|p| (p[1] - mean_y).powi(2)).sum::<f64>() / n;
        let max_abs_x = path.iter().map(|p| p[0].abs()).fold(0.0_f64, f64::max);
        let max_abs_y = path.iter().map(|p| p[1].abs()).fold(0.0_f64, f64::max);
        (var_x, var_y, max_abs_x, max_abs_y)
    };
    let (iv_x, iv_y, im_x, im_y) = axis_stats(&iso_path);
    let (dv_x, dv_y, dm_x, dm_y) = axis_stats(&diag_path);

    println!("  ─── per-axis trajectory spread ───");
    println!("                 var_x      var_y    var_y/var_x   |x|_max   |y|_max");
    println!(
        "    ISOTROPIC:   {:5.3}      {:5.3}      {:.2}        {:5.2}      {:5.2}",
        iv_x, iv_y, iv_y / iv_x, im_x, im_y
    );
    println!(
        "    DIAGONAL:    {:5.3}      {:5.3}      {:.2}        {:5.2}      {:5.2}",
        dv_x, dv_y, dv_y / dv_x, dm_x, dm_y
    );
    println!(
        "    TARGET RATIO σ²_y / σ²_x = {:.2}  (what an honest sampler should recover)",
        sigma_sq_per_field[1] / sigma_sq_per_field[0]
    );
    println!();

    // ── The takeaway ──
    println!("┌──────────────────────────────────────────────────────────────────────┐");
    println!("│  ISOTROPIC: trajectory wanders ~symmetrically. var_y/var_x ≈ 1     │");
    println!("│  Reads the density as a circle even though it's an ellipse.         │");
    println!("│  WRONG for any real anisotropic manifold.                            │");
    println!("│                                                                      │");
    println!("│  DIAGONAL:  trajectory elongated along the high-σ² axis.            │");
    println!("│  var_y/var_x recovers the target σ²-ratio (modulo MC noise).        │");
    println!("│  RIGHT, and what Marcella's bge / v11_fiber bundles need.            │");
    println!("│                                                                      │");
    println!("│  Same DREAM endpoint, same SDE, same temperature.                    │");
    println!("│  Only difference: one extra request field — fit_mode: \"diagonal\".   │");
    println!("└──────────────────────────────────────────────────────────────────────┘");
}

// ── ASCII scatter plot (shared with dream_demo) ───────────────────

fn render(points: &[Vec<f64>], marker: char) -> Vec<String> {
    let mut grid: Vec<Vec<char>> = vec![vec![' '; COLS]; ROWS];
    if let Some((r, c)) = world_to_grid(0.0, 0.0) {
        grid[r][c] = '+';
    }
    for p in points {
        if let Some((r, c)) = world_to_grid(p[0], p[1]) {
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
    let cy = ((Y_MAX - y) / (Y_MAX - Y_MIN) * (ROWS as f64 - 1.0)) as usize;
    Some((cy.min(ROWS - 1), cx.min(COLS - 1)))
}
