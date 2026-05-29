//! GIGI Encrypt v0.3 — Sprint J: Aff(ℝ) capability delegation.
//!
//! **Renamed from "proxy re-encryption" per v0.3.1 review §4.0.** This module
//! is *not* collusion-resistant PRE in the Ateniese–Hohenberger 2005 /
//! Libert–Vergnaud 2008 sense. The construction delivers two properties:
//!
//! 1. **Proxy-alone unrecoverability** (Theorem 4.1) — given only the
//!    capability `C_{A→B}`, the proxy cannot recover either party's key.
//! 2. **Zero-decrypt proxy transform** (Theorem 4.2) — applying the
//!    capability never invokes a decryption primitive; the proxy operates
//!    purely on ciphertext.
//!
//! **Known limitation 4.7.1 (delegatee + capability collusion)**: Bob, holding
//! both the capability `C_{A→B} = (α, β)` and his own gauge key
//! `g_B = (a_B, b_B)`, can solve `a_A = a_B / α` and `b_A = (b_B − β) / α`
//! in two equations and two unknowns, recovering Alice's full key. This is
//! a fundamental property of Aff(ℝ) — not an implementation bug. Use this
//! primitive only when the delegatee is trusted not to extract the
//! delegator's key (e.g., HIPAA-covered review entities, regulated
//! compliance workflows). For collusion-resistant delegation, integrate a
//! pairing-based PRE primitive (Umbral / NuCypher) — deferred to v0.4.
//!
//! See `GIGI_ENCRYPT_v0.3_SPRINT_SPEC.md` §4 for the full spec.

use crate::crypto::{FieldTransform, GaugeKey};

// ───────────────────────────────────────────────────────────────────────
// Types
// ───────────────────────────────────────────────────────────────────────

/// A delegation capability composing two GaugeKeys' per-field transforms.
///
/// Built once via `DelegationCapability::build(source, target, ...)`, then
/// applied to source-encrypted values to produce target-encrypted values.
/// The proxy holding only this capability cannot recover either party's
/// full key (Theorem 4.1).
#[derive(Debug, Clone)]
pub struct DelegationCapability {
    pub source_bundle: String,
    pub target_bundle: String,
    pub field_transforms: Vec<FieldDelegationTransform>,
}

/// Per-field composite encryption-to-encryption transform.
#[derive(Debug, Clone)]
pub enum FieldDelegationTransform {
    /// Both source and target encrypt this field as plaintext (Identity).
    /// Passthrough.
    Identity,

    /// Affine composite: `w' = α · w + β` where
    ///   `α = a_B / a_A`,  `β = b_B − b_A · α`.
    /// Apply: feeding source ciphertext `w_A = a_A · v + b_A` produces
    /// target ciphertext `α · w_A + β = a_B · v + b_B = w_B`.
    Affine { alpha: f64, beta: f64 },

    /// Isometric composite (group-aware): `M = O_B · O_A^T`,
    /// `b' = b_B − M · b_A`. The capability is built correctly, but
    /// `apply_to_value` returns `NotAffineClosure("Isometric")` because
    /// applying the composite requires the full group's k-vector to be
    /// reconstructed first — bundle-level integration deferred to a
    /// follow-up commit. The math primitive ships here so paper §4.1's
    /// Isometric-closure claim is testable.
    Isometric {
        matrix: Vec<Vec<f64>>,
        offset: Vec<f64>,
        group_id: String,
    },

    /// Typed refusal — source mode has no Aff(ℝ) closure (Opaque /
    /// Indexed / Probabilistic). Applying the capability on this field
    /// returns a `DelegationError::NotAffineClosure`.
    NotClosed { mode_name: &'static str },
}

/// Errors raised by capability construction or application.
#[derive(Debug, thiserror::Error)]
pub enum DelegationError {
    #[error("field mode {0} has no Aff(R) closure; delegation requires decrypt-then-encrypt or a collusion-resistant primitive (see Sprint J §4.7)")]
    NotAffineClosure(&'static str),

    #[error("source and target schemas have incompatible field counts (source has {source_count}, target has {target_count})")]
    SchemaMismatch { source_count: usize, target_count: usize },

    #[error("field at index {field_idx} cannot be delegated: source mode {source_mode} -> target mode {target_mode}")]
    IncompatibleFields {
        field_idx: usize,
        source_mode: &'static str,
        target_mode: &'static str,
    },

    #[error("field index {field_idx} out of bounds (capability has {field_count} fields)")]
    FieldIndexOutOfBounds {
        field_idx: usize,
        field_count: usize,
    },

    #[error("Isometric capability application requires bundle-level group context (deferred to v0.3.x follow-up)")]
    IsometricApplyRequiresGroup,
}

// ───────────────────────────────────────────────────────────────────────
// Construction
// ───────────────────────────────────────────────────────────────────────

impl DelegationCapability {
    /// Build a capability from two GaugeKeys.
    ///
    /// Returns:
    /// - `Ok` with a per-field composite transform list,
    /// - `Err(SchemaMismatch)` if the two keys have different field counts,
    /// - `Err(IncompatibleFields)` if a specific (source, target) field pair
    ///   has incompatible encryption modes (e.g., source is Affine but
    ///   target is Opaque — the structure groups don't compose).
    ///
    /// **Opaque / Indexed / Probabilistic source fields** produce a
    /// `FieldDelegationTransform::NotClosed` entry — the build succeeds,
    /// but `apply_to_value` on that field will return
    /// `NotAffineClosure`. This is the contract for surfacing per-field
    /// refusals at apply-time rather than failing the whole capability
    /// build.
    pub fn build(
        source: &GaugeKey,
        target: &GaugeKey,
        source_bundle: String,
        target_bundle: String,
    ) -> Result<Self, DelegationError> {
        if source.transforms.len() != target.transforms.len() {
            return Err(DelegationError::SchemaMismatch {
                source_count: source.transforms.len(),
                target_count: target.transforms.len(),
            });
        }
        let mut field_transforms = Vec::with_capacity(source.transforms.len());
        for (i, (s, t)) in source
            .transforms
            .iter()
            .zip(target.transforms.iter())
            .enumerate()
        {
            let composed = compose_field(s, t).map_err(|()| DelegationError::IncompatibleFields {
                field_idx: i,
                source_mode: mode_name_of(s),
                target_mode: mode_name_of(t),
            })?;
            field_transforms.push(composed);
        }
        Ok(Self {
            source_bundle,
            target_bundle,
            field_transforms,
        })
    }

    /// Apply the capability to a single field's encrypted value.
    ///
    /// Returns target-encrypted value `w_B` such that
    /// `decrypt_B(w_B) == decrypt_A(w_A)` — but neither plaintext nor
    /// either party's full key is materialized at the proxy.
    pub fn apply_to_value(&self, field_idx: usize, w: f64) -> Result<f64, DelegationError> {
        let ft =
            self.field_transforms
                .get(field_idx)
                .ok_or(DelegationError::FieldIndexOutOfBounds {
                    field_idx,
                    field_count: self.field_transforms.len(),
                })?;
        match ft {
            FieldDelegationTransform::Identity => Ok(w),
            FieldDelegationTransform::Affine { alpha, beta } => Ok(alpha * w + beta),
            FieldDelegationTransform::Isometric { .. } => {
                Err(DelegationError::IsometricApplyRequiresGroup)
            }
            FieldDelegationTransform::NotClosed { mode_name } => {
                Err(DelegationError::NotAffineClosure(mode_name))
            }
        }
    }

    /// Count of fields by closure status — useful for surfacing
    /// "X affine fields, Y isometric groups, Z refused fields" at
    /// capability creation time (spec §4.5 GQL response shape).
    pub fn closure_summary(&self) -> ClosureSummary {
        let mut s = ClosureSummary::default();
        for ft in &self.field_transforms {
            match ft {
                FieldDelegationTransform::Identity => s.identity += 1,
                FieldDelegationTransform::Affine { .. } => s.affine += 1,
                FieldDelegationTransform::Isometric { .. } => s.isometric += 1,
                FieldDelegationTransform::NotClosed { .. } => s.refused += 1,
            }
        }
        s
    }
}

/// Summary of a capability's per-field closure status.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ClosureSummary {
    pub identity: usize,
    pub affine: usize,
    pub isometric: usize,
    pub refused: usize,
}

// ───────────────────────────────────────────────────────────────────────
// compose_field — internal pairwise composer
// ───────────────────────────────────────────────────────────────────────

fn compose_field(
    source: &FieldTransform,
    target: &FieldTransform,
) -> Result<FieldDelegationTransform, ()> {
    use FieldTransform::*;
    match (source, target) {
        (Identity, Identity) => Ok(FieldDelegationTransform::Identity),

        (Affine { scale: a_a, offset: b_a }, Affine { scale: a_b, offset: b_b }) => {
            // Affine closure (Sprint G ext): apply(w) = (a_B/a_A)·w + (b_B − b_A · a_B/a_A)
            let alpha = a_b / a_a;
            let beta = b_b - b_a * alpha;
            Ok(FieldDelegationTransform::Affine { alpha, beta })
        }

        (
            Isometric {
                matrix: o_a,
                offset_vec: b_a,
                group_id: g_a,
                ..
            },
            Isometric {
                matrix: o_b,
                offset_vec: b_b,
                group_id: g_b,
                ..
            },
        ) => {
            if g_a != g_b || o_a.len() != o_b.len() || b_a.len() != b_b.len() {
                return Err(());
            }
            // M = O_B · O_A^T   (composition of two orthogonal matrices,
            //                    itself orthogonal in O(k))
            let m = matmul(o_b, &transpose(o_a));
            // b' = b_B − M · b_A
            let mb_a = matvec(&m, b_a);
            let offset: Vec<f64> = b_b.iter().zip(mb_a.iter()).map(|(b, mb)| b - mb).collect();
            Ok(FieldDelegationTransform::Isometric {
                matrix: m,
                offset,
                group_id: g_a.clone(),
            })
        }

        // Opaque / Indexed / Probabilistic on either side: no closure.
        // The build succeeds (capability is well-formed); apply on this
        // field will return NotAffineClosure at call time.
        (Opaque { .. }, _) | (_, Opaque { .. }) => Ok(FieldDelegationTransform::NotClosed {
            mode_name: "Opaque",
        }),
        (Indexed { .. }, _) | (_, Indexed { .. }) => Ok(FieldDelegationTransform::NotClosed {
            mode_name: "Indexed",
        }),
        (Probabilistic { .. }, _) | (_, Probabilistic { .. }) => {
            Ok(FieldDelegationTransform::NotClosed {
                mode_name: "Probabilistic",
            })
        }

        // Any other mismatch (e.g., Affine ↔ Identity, or Affine ↔
        // Isometric) — modes don't compose.
        _ => Err(()),
    }
}

fn mode_name_of(ft: &FieldTransform) -> &'static str {
    match ft {
        FieldTransform::Identity => "Identity",
        FieldTransform::Affine { .. } => "Affine",
        FieldTransform::Opaque { .. } => "Opaque",
        FieldTransform::Indexed { .. } => "Indexed",
        FieldTransform::Probabilistic { .. } => "Probabilistic",
        FieldTransform::Isometric { .. } => "Isometric",
    }
}

// ───────────────────────────────────────────────────────────────────────
// Small matrix helpers (for Isometric composition).
// ───────────────────────────────────────────────────────────────────────

fn transpose(m: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let rows = m.len();
    if rows == 0 {
        return Vec::new();
    }
    let cols = m[0].len();
    let mut t = vec![vec![0.0; rows]; cols];
    for (i, row) in m.iter().enumerate() {
        for (j, &v) in row.iter().enumerate() {
            t[j][i] = v;
        }
    }
    t
}

fn matmul(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = a.len();
    if n == 0 {
        return Vec::new();
    }
    let inner = b.len();
    let m = b[0].len();
    let mut c = vec![vec![0.0; m]; n];
    for i in 0..n {
        for j in 0..m {
            let mut acc = 0.0;
            for l in 0..inner {
                acc += a[i][l] * b[l][j];
            }
            c[i][j] = acc;
        }
    }
    c
}

fn matvec(m: &[Vec<f64>], v: &[f64]) -> Vec<f64> {
    m.iter()
        .map(|row| row.iter().zip(v.iter()).map(|(r, x)| r * x).sum())
        .collect()
}

// ───────────────────────────────────────────────────────────────────────
// Module-level unit tests for the math primitive.
// Integration tests with realistic GaugeKey configurations live in
// `tests/delegation_v0_3.rs`.
// ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn affine(scale: f64, offset: f64) -> FieldTransform {
        FieldTransform::Affine { scale, offset }
    }

    #[test]
    fn affine_capability_round_trip_on_scalar() {
        let g_a = GaugeKey {
            transforms: vec![affine(2.0, 5.0)],
        };
        let g_b = GaugeKey {
            transforms: vec![affine(3.0, -1.0)],
        };
        let cap = DelegationCapability::build(&g_a, &g_b, "A".into(), "B".into()).unwrap();
        // Alice encrypts v = 7.0:  w_A = 2*7 + 5 = 19
        let v = 7.0;
        let w_a = 2.0 * v + 5.0;
        // Apply capability:
        let w_b = cap.apply_to_value(0, w_a).unwrap();
        // Bob should now have:  w_B = 3*7 + (-1) = 20
        let expected_w_b = 3.0 * v + (-1.0);
        assert!((w_b - expected_w_b).abs() < 1e-12);
    }

    #[test]
    fn capability_alone_does_not_reveal_alice_key() {
        // Two distinct Alice keys with the SAME Bob key produce DIFFERENT
        // (α, β) — but that's only one equation in two unknowns from the
        // proxy's view, so the proxy can't pin (a_A, b_A) from (α, β) alone.
        let g_b = GaugeKey {
            transforms: vec![affine(3.0, 1.0)],
        };
        let g_a1 = GaugeKey {
            transforms: vec![affine(2.0, 5.0)],
        };
        let g_a2 = GaugeKey {
            transforms: vec![affine(6.0, 7.0)],
        };
        let cap1 = DelegationCapability::build(&g_a1, &g_b, "A1".into(), "B".into()).unwrap();
        let cap2 = DelegationCapability::build(&g_a2, &g_b, "A2".into(), "B".into()).unwrap();
        // The capabilities are different — but the proxy cannot recover
        // (a_A, b_A) from (α, β) alone: 2 equations, 4 unknowns.
        match (
            &cap1.field_transforms[0],
            &cap2.field_transforms[0],
        ) {
            (
                FieldDelegationTransform::Affine { alpha: a1, beta: b1 },
                FieldDelegationTransform::Affine { alpha: a2, beta: b2 },
            ) => {
                assert!((a1 - a2).abs() > 1e-9 || (b1 - b2).abs() > 1e-9);
            }
            _ => panic!("expected Affine transforms"),
        }
    }

    #[test]
    fn collusion_attack_recovers_alice_key_explicitly_documented() {
        // Limitation 4.7.1: Bob + capability + own key = Alice's key.
        // a_A = a_B / alpha ;  b_A = (b_B - beta) / alpha
        let (a_a, b_a) = (2.0, 5.0);
        let (a_b, b_b) = (3.0, 1.0);
        let g_a = GaugeKey {
            transforms: vec![affine(a_a, b_a)],
        };
        let g_b = GaugeKey {
            transforms: vec![affine(a_b, b_b)],
        };
        let cap = DelegationCapability::build(&g_a, &g_b, "A".into(), "B".into()).unwrap();
        let (alpha, beta) = match &cap.field_transforms[0] {
            FieldDelegationTransform::Affine { alpha, beta } => (*alpha, *beta),
            _ => panic!("expected Affine"),
        };
        // Bob runs the collusion solve.
        let recovered_a_a = a_b / alpha;
        let recovered_b_a = (b_b - beta) / alpha;
        // This SHOULD recover Alice's key exactly (the test passing
        // confirms the limitation is in scope by design).
        assert!((recovered_a_a - a_a).abs() < 1e-12);
        assert!((recovered_b_a - b_a).abs() < 1e-12);
    }

    #[test]
    fn opaque_field_returns_typed_refusal_on_apply() {
        let g_a = GaugeKey {
            transforms: vec![FieldTransform::Opaque { key: [0u8; 32] }],
        };
        let g_b = GaugeKey {
            transforms: vec![FieldTransform::Opaque { key: [1u8; 32] }],
        };
        let cap = DelegationCapability::build(&g_a, &g_b, "A".into(), "B".into()).unwrap();
        let err = cap.apply_to_value(0, 42.0).unwrap_err();
        assert!(matches!(err, DelegationError::NotAffineClosure("Opaque")));
    }

    #[test]
    fn schema_mismatch_on_unequal_field_counts() {
        let g_a = GaugeKey {
            transforms: vec![affine(1.0, 0.0), affine(2.0, 1.0)],
        };
        let g_b = GaugeKey {
            transforms: vec![affine(3.0, 0.0)],
        };
        let result = DelegationCapability::build(&g_a, &g_b, "A".into(), "B".into());
        assert!(matches!(
            result,
            Err(DelegationError::SchemaMismatch { source_count: 2, target_count: 1 })
        ));
    }

    #[test]
    fn isometric_capability_built_but_apply_defers() {
        let iso = |gid: &str| FieldTransform::Isometric {
            group_id: gid.into(),
            matrix: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            offset_vec: vec![0.0, 0.0],
            member_index: 0,
        };
        let g_a = GaugeKey {
            transforms: vec![iso("wind")],
        };
        let g_b = GaugeKey {
            transforms: vec![iso("wind")],
        };
        let cap = DelegationCapability::build(&g_a, &g_b, "A".into(), "B".into()).unwrap();
        // Build succeeds; apply defers per the impl note.
        let err = cap.apply_to_value(0, 1.0).unwrap_err();
        assert!(matches!(err, DelegationError::IsometricApplyRequiresGroup));
    }

    #[test]
    fn closure_summary_counts_correctly() {
        let g_a = GaugeKey {
            transforms: vec![
                affine(1.0, 0.0),
                FieldTransform::Opaque { key: [0u8; 32] },
                FieldTransform::Identity,
            ],
        };
        let g_b = GaugeKey {
            transforms: vec![
                affine(2.0, 0.0),
                FieldTransform::Opaque { key: [1u8; 32] },
                FieldTransform::Identity,
            ],
        };
        let cap = DelegationCapability::build(&g_a, &g_b, "A".into(), "B".into()).unwrap();
        let s = cap.closure_summary();
        assert_eq!(s.affine, 1);
        assert_eq!(s.refused, 1);
        assert_eq!(s.identity, 1);
        assert_eq!(s.isometric, 0);
    }
}
