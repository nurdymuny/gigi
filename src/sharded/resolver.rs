//! Clean Finger Move resolver (T6 §3.6).
//!
//! Phase A: deterministic in-memory resolver matching the Python TDD
//! gate `t6_clean_finger_move.py`. Phase B will wire this into the
//! sharded write path with conflict-set discovery from per-shard WAL
//! diffs.

use std::collections::HashSet;

/// A write conflict between shards. The `sign` field encodes which side
/// of a canceling pair this conflict represents (+1 / -1); `partner_id`
/// is the conflict that algebraically cancels this one.
#[derive(Clone, Debug, PartialEq)]
pub struct WriteConflict {
    pub id: u64,
    pub sign: i8,
    pub partner_id: u64,
    /// Downstream conflict ids whose state would be modified if this
    /// conflict's resolution were to propagate. Informational only —
    /// the Clean Finger Move resolves with LOCAL support, not
    /// propagating downstream effects (Davis Thm 5.3).
    pub downstream: Vec<u64>,
}

/// Termination state of the resolver.
#[derive(Clone, Debug)]
pub struct ResolverTrace {
    pub initial_count: usize,
    pub steps: usize,
    pub residual_size: usize,
    pub monotonic_decrease_violations: u32,
    pub blocked: bool,
    pub blocked_set: Vec<u64>,
}

/// Reasons the resolver can fail.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolverError {
    /// A conflict has no canceling partner in the input. The H_2 = 0
    /// precondition (every conflict has a canceling partner) is
    /// violated.
    NoCancelingPartner { conflict_id: u64 },
    /// A canceling pair has same-sign conflicts; the input is malformed.
    SameSignPartners { a: u64, b: u64, sign: i8 },
}

/// Resolve write conflicts via the Clean Finger Move pattern.
///
/// Precondition: every conflict in `conflicts` has a canceling partner
/// (i.e., the H_2 = 0 algebraic-cancellation analog).
///
/// Algorithm: repeatedly find any canceling pair and remove it. By T6,
/// terminates in exactly `conflicts.len() / 2` steps with zero residual
/// when the precondition holds; the monotonic-decrease invariant is
/// asserted in-loop.
pub fn sharded_write_resolve(
    conflicts: Vec<WriteConflict>,
) -> Result<ResolverTrace, ResolverError> {
    let initial = conflicts.len();
    let by_id: std::collections::HashMap<u64, WriteConflict> =
        conflicts.into_iter().map(|c| (c.id, c)).collect();

    // Precondition validation
    for c in by_id.values() {
        let partner = by_id.get(&c.partner_id).ok_or(ResolverError::NoCancelingPartner {
            conflict_id: c.id,
        })?;
        if c.sign + partner.sign != 0 {
            return Err(ResolverError::SameSignPartners {
                a: c.id,
                b: partner.id,
                sign: c.sign,
            });
        }
    }

    let mut unresolved: HashSet<u64> = by_id.keys().copied().collect();
    let mut steps = 0;
    let mut monotonic_violations = 0;
    let mut last_size = initial;

    while !unresolved.is_empty() {
        // Find a canceling pair (a, b) both unresolved
        let mut pair: Option<(u64, u64)> = None;
        for &a_id in &unresolved {
            let a = &by_id[&a_id];
            if unresolved.contains(&a.partner_id) && a.sign + by_id[&a.partner_id].sign == 0 {
                pair = Some((a_id, a.partner_id));
                break;
            }
        }

        let Some((a_id, b_id)) = pair else {
            // Cannot happen under the precondition, but defensive
            return Ok(ResolverTrace {
                initial_count: initial,
                steps,
                residual_size: unresolved.len(),
                monotonic_decrease_violations: monotonic_violations,
                blocked: true,
                blocked_set: unresolved.into_iter().collect(),
            });
        };

        unresolved.remove(&a_id);
        unresolved.remove(&b_id);
        let new_size = unresolved.len();
        if new_size != last_size - 2 {
            monotonic_violations += 1;
        }
        last_size = new_size;
        steps += 1;
    }

    Ok(ResolverTrace {
        initial_count: initial,
        steps,
        residual_size: 0,
        monotonic_decrease_violations: monotonic_violations,
        blocked: false,
        blocked_set: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_pair(a: u64, b: u64) -> Vec<WriteConflict> {
        vec![
            WriteConflict { id: a, sign: 1, partner_id: b, downstream: vec![] },
            WriteConflict { id: b, sign: -1, partner_id: a, downstream: vec![] },
        ]
    }

    #[test]
    fn empty_input_resolves_trivially() {
        let trace = sharded_write_resolve(vec![]).unwrap();
        assert_eq!(trace.steps, 0);
        assert_eq!(trace.residual_size, 0);
        assert_eq!(trace.monotonic_decrease_violations, 0);
        assert!(!trace.blocked);
    }

    #[test]
    fn single_pair_resolves_in_one_step() {
        let trace = sharded_write_resolve(mk_pair(0, 1)).unwrap();
        assert_eq!(trace.initial_count, 2);
        assert_eq!(trace.steps, 1);
        assert_eq!(trace.residual_size, 0);
        assert_eq!(trace.monotonic_decrease_violations, 0);
    }

    #[test]
    fn many_pairs_resolve_in_n_over_2_steps() {
        let mut conflicts = Vec::new();
        for k in 0..10 {
            let a = 2 * k;
            let b = 2 * k + 1;
            conflicts.extend(mk_pair(a, b));
        }
        let trace = sharded_write_resolve(conflicts).unwrap();
        assert_eq!(trace.initial_count, 20);
        assert_eq!(trace.steps, 10);
        assert_eq!(trace.residual_size, 0);
        assert_eq!(trace.monotonic_decrease_violations, 0);
    }

    #[test]
    fn missing_partner_rejected() {
        let conflicts = vec![WriteConflict {
            id: 0,
            sign: 1,
            partner_id: 99,
            downstream: vec![],
        }];
        let err = sharded_write_resolve(conflicts);
        assert!(matches!(err, Err(ResolverError::NoCancelingPartner { .. })));
    }

    #[test]
    fn same_sign_partner_rejected() {
        let conflicts = vec![
            WriteConflict { id: 0, sign: 1, partner_id: 1, downstream: vec![] },
            WriteConflict { id: 1, sign: 1, partner_id: 0, downstream: vec![] },
        ];
        let err = sharded_write_resolve(conflicts);
        assert!(matches!(err, Err(ResolverError::SameSignPartners { .. })));
    }
}
