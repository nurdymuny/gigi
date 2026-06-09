//! Phase 4.3 — Orthogonality scan: (H[X], |Ω|) across process families.
//!
//! Empirical evidence for the paper's TH1 thesis (commutator captures
//! information beyond symbol entropy).
//!
//! Writes `theory/causal_states/orthogonality_scan.csv` with one row per
//! process and one summary file. The figure: scatter `H[X]` on the x-axis
//! against `|Ω|_TV` on the y-axis. Visible features:
//!
//!   - iid Bernoulli family: a horizontal line at `Ω = 0` sweeping `H[X]`
//!     across `[h(p), p ∈ (0, 1)]` = `[0, 1]`.
//!   - Symmetric HMM family: vertical scatter at `H[X] = 1` sweeping
//!     `|Ω|` over `(0, 0.5)`.
//!   - Even Process: single point at `(0.918, 1.0)` — saturating |Ω| at
//!     a moderate H[X], **collinear in x with the iid Bernoulli(2/3) point
//!     at (0.918, 0)**. Same audit-confounder entropy, opposite commutator.
//!
//! Run:
//!   cargo run --features causal_states --bin causal_states_orthogonality

use gigi::causal_states::{
    commutator, EvenU0, EvenU1, HmmUpdate, UpdateError, UpdateOperator,
};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

fn h(p: f64) -> f64 {
    if p <= 0.0 || p >= 1.0 {
        return 0.0;
    }
    -(p * p.log2() + (1.0 - p) * (1.0 - p).log2())
}

struct IidBernoulli {
    p: f64,
}
impl UpdateOperator for IidBernoulli {
    fn apply(&self, _belief: &[f64]) -> Result<Vec<f64>, UpdateError> {
        Ok(vec![1.0 - self.p, self.p])
    }
}

fn main() {
    let out_dir: PathBuf = ["theory", "causal_states"].iter().collect();
    std::fs::create_dir_all(&out_dir).expect("create theory/causal_states/");
    let csv_path = out_dir.join("orthogonality_scan.csv");
    let summary_path = out_dir.join("orthogonality_summary.txt");

    let mut w = BufWriter::new(File::create(&csv_path).expect("create CSV"));
    writeln!(w, "family,name,param_a,param_b,h_x,omega_tv,omega_hellinger,annotation").unwrap();

    // ─── Family 1: iid Bernoulli(p) sweep ────────────────────────────────
    let mu_half = vec![0.5, 0.5];
    eprintln!("─── iid Bernoulli family ──────────────────");
    for p_times_100 in 5..=95 {
        let p = f64::from(p_times_100) / 100.0;
        // Within a single iid process, U_0 ≡ U_1 ≡ "return stationary".
        // Commutator vanishes identically.
        let u_0 = IidBernoulli { p };
        let u_1 = IidBernoulli { p };
        let omega = commutator(&u_0, &u_1, &mu_half).unwrap();
        let annot = if (p - 2.0 / 3.0).abs() < 0.01 {
            "match_even_h_x"
        } else {
            ""
        };
        writeln!(
            w,
            "iid_bernoulli,iid_p={p:.2},{p:.4},,{:.6},{:.6},{:.6},{annot}",
            h(p),
            omega.tv,
            omega.hellinger
        )
        .unwrap();
        if !annot.is_empty() {
            eprintln!(
                "  iid Bernoulli(p={p:.4}): H[X]={:.4}, |Ω|={:.6}  ← {annot}",
                h(p),
                omega.tv
            );
        }
    }

    // ─── Family 2: symmetric noisy HMM grid ─────────────────────────────
    eprintln!("─── symmetric HMM grid ────────────────────");
    for alpha_times_100 in 5..=45 {
        for beta_times_100 in 5..=45 {
            let alpha = f64::from(alpha_times_100) / 100.0;
            let beta = f64::from(beta_times_100) / 100.0;
            let u_0 = HmmUpdate { alpha, beta, symbol: 0 };
            let u_1 = HmmUpdate { alpha, beta, symbol: 1 };
            let omega = commutator(&u_0, &u_1, &mu_half).unwrap();
            // Symmetric HMM stationary on hidden = (0.5, 0.5);
            // P(X = 0) = 0.5; H[X] = 1.0 exactly.
            writeln!(
                w,
                "hmm_symmetric,hmm_a={alpha:.2}_b={beta:.2},{alpha:.4},{beta:.4},1.000000,{:.6},{:.6},",
                omega.tv, omega.hellinger
            )
            .unwrap();
        }
    }

    // ─── Family 3: Even Process — the headline anchor ───────────────────
    eprintln!("─── Even Process (sofic anchor) ───────────");
    let mu_even = vec![2.0 / 3.0, 1.0 / 3.0];
    let omega_even = commutator(&EvenU0, &EvenU1, &mu_even).unwrap();
    // Stationary symbol distribution = (1/3, 2/3), so H[X] = h(2/3).
    let h_even = h(2.0 / 3.0);
    writeln!(
        w,
        "even_process,even_mu,,,{h_even:.6},{:.6},{:.6},sofic_anchor_match_iid_two_thirds_h_x",
        omega_even.tv, omega_even.hellinger
    )
    .unwrap();
    eprintln!(
        "  Even Process @ μ=(2/3,1/3): H[X]={h_even:.4}, |Ω|={:.6}  ← sofic anchor",
        omega_even.tv
    );

    w.flush().unwrap();
    drop(w);

    // ─── Summary report ──────────────────────────────────────────────────
    // Re-locate the iid Bernoulli(2/3) and Even Process points to write
    // the headline orthogonality stat.
    let p_iid = 2.0 / 3.0;
    let omega_iid = commutator(
        &IidBernoulli { p: p_iid },
        &IidBernoulli { p: p_iid },
        &mu_half,
    )
    .unwrap();

    let summary = format!(
        "Causal States — orthogonality scan summary\n\
         ===========================================\n\
         \n\
         Companion to Davis (2026), \"Causal States as Predictive Sections.\"\n\
         Generated by examples/causal_states_orthogonality.rs.\n\
         \n\
         CSV: orthogonality_scan.csv\n\
         \n\
         Process families scanned:\n\
           - iid Bernoulli(p):  91 points  (p ∈ {{0.05, 0.06, ..., 0.95}})\n\
           - symmetric HMM:    1681 points (α, β ∈ {{0.05, 0.06, ..., 0.45}}²)\n\
           - Even Process:        1 point  (sofic anchor)\n\
         Total:                1773 rows.\n\
         \n\
         Headline orthogonality (TH1 empirical evidence):\n\
           Two processes with H[X] = h(2/3) ≈ {h_even:.4} bits:\n\
             iid Bernoulli(2/3)     |Ω|_TV = {:.6}\n\
             Even Process @ μ       |Ω|_TV = {:.6}\n\
           Same audit-confounder entropy, |Ω| gap = {:.6}.\n\
         \n\
         Secondary orthogonality:\n\
           Symmetric HMM family holds H[X] = 1.0 exactly while |Ω|_TV\n\
           ranges over (0, 0.5) per Eq 6.4. Single-value entropy does NOT\n\
           determine the commutator.\n\
         \n\
         These two demonstrations together establish empirically that\n\
         the commutator is not a function of H[X]. The paper's thesis\n\
         is operational, not just theoretical.\n",
        omega_iid.tv,
        omega_even.tv,
        omega_even.tv - omega_iid.tv,
    );

    std::fs::write(&summary_path, &summary).expect("write summary");
    eprintln!("\n{summary}");
    eprintln!("Wrote: {}", csv_path.display());
    eprintln!("Wrote: {}", summary_path.display());
}
