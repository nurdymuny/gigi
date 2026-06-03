//! `walk` — commit to an imagined path with safety envelope.
//!
//! Phase 1 ships the type surface and the **curvature gate**
//! (Marcella feedback #3 load-bearing). Double-cover lifting,
//! SUDOKU pre-flight, and parallel-transport execution are Phase 2 —
//! they require the substrate's parallel_transport primitive and
//! SUDOKU pre-flight wired into the same module.
//!
//! T13 validates the double-cover monodromy resolution math
//! ([`theory/imagine/validation/t13_double_cover_monodromy.py`]) so
//! Phase 2 has a green math foundation when it wires the lift.

use crate::imagine::config::WalkConfig;
use crate::imagine::provenance::ImaginedRecord;

/// Successful walk outcomes.
#[derive(Debug, Clone, PartialEq)]
pub enum WalkOutcome {
    /// The path was walked successfully; the endpoint state is the
    /// last point in the path.
    Walked {
        endpoint: ImaginedRecord,
        accumulated_holonomy: f64,
    },
    /// The walk was lifted to the double cover; the endpoint is in
    /// the covering space with the given monodromy class.
    /// Caller must project back at commit time.
    WalkedInCover {
        endpoint_in_cover: ImaginedRecord,
        monodromy_class: i32,
    },
}

/// Reasons WALK can refuse a path.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum WalkError {
    /// SUDOKU pre-flight detected a substrate constraint the imagined
    /// path violates. Phase 2 wires this; Phase 1 never returns it.
    #[error("SUDOKU pre-flight failed: {violation}")]
    SudokuPreflightFailed { violation: String },

    /// An imagined point has K > max_imagined_curvature.
    /// **This is the load-bearing safety check (Marcella feedback #3).**
    #[error("Curvature gate refused: step {step} K = {k_at_step} > threshold {threshold}")]
    OverCurvatureRefused {
        step: u32,
        k_at_step: f64,
        threshold: f64,
    },

    /// Accumulated holonomy exceeded the configured budget.
    #[error("Holonomy budget exceeded: accumulated {accumulated} > threshold {threshold}")]
    HolonomyBudgetExceeded {
        accumulated: f64,
        threshold: f64,
    },

    /// A seam crossing has Z₂ monodromy and `use_double_cover` is
    /// false. Caller must enable double cover or refuse.
    /// Phase 2 wires this with the seam detector; Phase 1's stub
    /// of `walk` never inspects seams.
    #[error("Unresolved monodromy at seam {seam}")]
    UnresolvedMonodromy { seam: String },

    /// Empty path; nothing to walk.
    #[error("Empty imagined path")]
    EmptyPath,
}

/// Walk an imagined path with the configured safety envelope.
///
/// Phase 1 implementation: enforces the curvature gate (load-bearing
/// per Marcella's feedback) and the holonomy budget. Double-cover
/// lifting, SUDOKU pre-flight, parallel-transport execution, and
/// `materialize_on_success` are Phase 2.
///
/// Returns `Walked` on success (no seams encountered in Phase 1
/// because Phase 1 doesn't yet detect seams; that's Phase 2's job
/// once the substrate's seam detector is wired in).
pub fn walk(
    imagined_path: &[ImaginedRecord],
    config: &WalkConfig,
) -> Result<WalkOutcome, WalkError> {
    if imagined_path.is_empty() {
        return Err(WalkError::EmptyPath);
    }

    // Curvature gate — Marcella feedback #3 load-bearing
    for (step, record) in imagined_path.iter().enumerate() {
        if record.local_k > config.max_imagined_curvature {
            return Err(WalkError::OverCurvatureRefused {
                step: step as u32,
                k_at_step: record.local_k,
                threshold: config.max_imagined_curvature,
            });
        }
    }

    // Holonomy budget gate
    let endpoint = imagined_path.last().unwrap().clone();
    if endpoint.accumulated_holonomy > config.max_accumulated_holonomy {
        return Err(WalkError::HolonomyBudgetExceeded {
            accumulated: endpoint.accumulated_holonomy,
            threshold: config.max_accumulated_holonomy,
        });
    }

    Ok(WalkOutcome::Walked {
        endpoint: endpoint.clone(),
        accumulated_holonomy: endpoint.accumulated_holonomy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imagine::provenance::ImaginedProvenance;

    fn record(local_k: f64, holonomy: f64) -> ImaginedRecord {
        ImaginedRecord {
            coords: vec![0.0, 0.0],
            local_k,
            accumulated_holonomy: holonomy,
            provenance: ImaginedProvenance::Geodesic {
                seed_record_id: "s".into(),
                seed_bundle: "b".into(),
                initial_direction: vec![1.0, 0.0],
                path_length: 0.0,
                integrator_steps: 0,
            },
        }
    }

    #[test]
    fn empty_path_returns_empty_path_error() {
        let err = walk(&[], &WalkConfig::default());
        assert_eq!(err, Err(WalkError::EmptyPath));
    }

    #[test]
    fn safe_path_with_default_config_walks_successfully() {
        let path = vec![record(2.0, 0.0), record(3.0, 0.1), record(3.5, 0.2)];
        let outcome = walk(&path, &WalkConfig::default()).unwrap();
        match outcome {
            WalkOutcome::Walked { accumulated_holonomy, .. } => {
                assert!((accumulated_holonomy - 0.2).abs() < 1e-12);
            }
            _ => panic!("expected Walked outcome"),
        }
    }

    #[test]
    fn over_curvature_refused_at_default_4_0_threshold() {
        // K = 5.0 exceeds the default 4.0 = K(CP¹) ceiling
        let path = vec![record(2.0, 0.0), record(5.0, 0.1)];
        let err = walk(&path, &WalkConfig::default());
        match err {
            Err(WalkError::OverCurvatureRefused { step, k_at_step, threshold }) => {
                assert_eq!(step, 1);
                assert_eq!(k_at_step, 5.0);
                assert_eq!(threshold, 4.0);
            }
            _ => panic!("expected OverCurvatureRefused: {:?}", err),
        }
    }

    #[test]
    fn over_curvature_can_be_opted_out_of_explicitly() {
        // Explicit opt-in: bump max to 10.0
        let path = vec![record(2.0, 0.0), record(5.0, 0.1)];
        let config = WalkConfig {
            max_imagined_curvature: 10.0,
            ..WalkConfig::default()
        };
        let outcome = walk(&path, &config).unwrap();
        assert!(matches!(outcome, WalkOutcome::Walked { .. }));
    }

    #[test]
    fn holonomy_budget_refused_when_exceeded() {
        // Endpoint holonomy 0.8 > default budget 0.5
        let path = vec![record(1.0, 0.0), record(1.0, 0.8)];
        let err = walk(&path, &WalkConfig::default());
        match err {
            Err(WalkError::HolonomyBudgetExceeded { accumulated, threshold }) => {
                assert_eq!(accumulated, 0.8);
                assert_eq!(threshold, 0.5);
            }
            _ => panic!("expected HolonomyBudgetExceeded: {:?}", err),
        }
    }
}
