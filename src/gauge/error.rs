//! `GaugeFieldError` — typed error surface for the GAUGE_FIELD
//! declaration + construction path.
//!
//! Lifted from the inline `unimplemented_for_group!` panic to a
//! return-value enum so the parser/executor can surface failure as a
//! normal user error and Halcyon's G2.D regex anchor `SU\(2\)` has a
//! stable `Display` impl to match against (Bee's locked decision 5).
//!
//! Source of truth for cross-binding error messages. Variants:
//!
//! - `SeedRequired` — `INIT HAAR_RANDOM` was declared but no SEED
//!   was provided. Display includes the literal "SEED" so Halcyon's
//!   `match="SEED"` substring check hits.
//! - `UnsupportedGroup(Group)` — group compiles but has no live math
//!   at launch (everything except `SU(2)`). Display includes the
//!   group's stable label (`"SU(2)"`, `"SU(3)"`, …).
//! - `LatticeNotDeclared(name)` — `GAUGE_FIELD ON L` referenced a
//!   lattice that the registry does not know.
//! - `FieldNotDeclared(name)` — `INIT FROM_FIELD X` referenced a
//!   field that the registry does not know.
//! - `BufferShapeMismatch { expected, got }` — a buffer materialized
//!   in a different shape than the lattice's `n_edges * repr_dim`
//!   demands.
//!
//! Inner math (`compose`, `inverse`, `read_element`) keeps its
//! Part-I panic — reaching it from a well-typed buffer is a
//! programming error, not a user error.

use super::group::Group;

/// Typed error surface for GAUGE_FIELD construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GaugeFieldError {
    /// `INIT HAAR_RANDOM` declared without a SEED clause.
    SeedRequired,
    /// Group variant compiles but has no live math at launch
    /// (everything except `SU(2)`).
    UnsupportedGroup(Group),
    /// `GAUGE_FIELD ON L` references an unknown lattice.
    LatticeNotDeclared(String),
    /// `INIT FROM_FIELD X` references an unknown source field.
    FieldNotDeclared(String),
    /// A materialized buffer's flat-length does not match
    /// `n_edges * repr_dim` for the bound lattice.
    BufferShapeMismatch { expected: usize, got: usize },
}

impl std::fmt::Display for GaugeFieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GaugeFieldError::SeedRequired => write!(
                f,
                "gauge: INIT HAAR_RANDOM requires a SEED clause \
                 (intra-binding bit-identity is the contract — declare \
                 INIT HAAR_RANDOM SEED <u64>)"
            ),
            GaugeFieldError::UnsupportedGroup(g) => write!(
                f,
                "gauge: group {} is not implemented (Part II ships SU(2) math only; \
                 future groups land as separate EdgeConnection impls per the group-erasure plan)",
                g.label()
            ),
            GaugeFieldError::LatticeNotDeclared(name) => write!(
                f,
                "gauge: lattice '{name}' is not declared (DECLARE the LATTICE before attaching a GAUGE_FIELD)"
            ),
            GaugeFieldError::FieldNotDeclared(name) => write!(
                f,
                "gauge: source field '{name}' is not declared (INIT FROM_FIELD needs an existing GAUGE_FIELD)"
            ),
            GaugeFieldError::BufferShapeMismatch { expected, got } => write!(
                f,
                "gauge: buffer shape mismatch (expected {expected} f64s, got {got})"
            ),
        }
    }
}

impl std::error::Error for GaugeFieldError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// `SeedRequired` Display contains the literal "SEED" so Halcyon's
    /// `match="SEED"` substring check hits.
    #[test]
    fn seed_required_display_contains_seed() {
        let err = GaugeFieldError::SeedRequired;
        assert!(err.to_string().contains("SEED"));
    }

    /// `UnsupportedGroup(Group::U1)` Display contains the literal
    /// "SU(2)" so Halcyon's `match="SU\\(2\\)"` regex anchor hits.
    #[test]
    fn unsupported_group_display_contains_su2() {
        let err = GaugeFieldError::UnsupportedGroup(Group::U1);
        assert!(err.to_string().contains("SU(2)"));
    }

    /// Every variant has a non-empty Display.
    #[test]
    fn every_variant_displays() {
        let variants = [
            GaugeFieldError::SeedRequired,
            GaugeFieldError::UnsupportedGroup(Group::SU3),
            GaugeFieldError::LatticeNotDeclared("L".into()),
            GaugeFieldError::FieldNotDeclared("U".into()),
            GaugeFieldError::BufferShapeMismatch { expected: 360, got: 100 },
        ];
        for v in &variants {
            assert!(!v.to_string().is_empty());
        }
    }
}
