//! Graph operators for the Kähler upgrade.
//!
//! Implements catalog §1.1: dual principal / auxiliary adjacency
//! operators on the bundle's field-index graph, with a
//! commutativity classifier that the query planner uses to reorder
//! joins safely.
//!
//! The catalog's claim: on a Kähler graph where the principal and
//! auxiliary generating sets are (a) subsets of an abelian group,
//! or (b) unions of conjugacy classes of a non-abelian group, the
//! adjacency operators commute. Commuting operators share an
//! eigenbasis, so query plans factor through that basis and join
//! reorderings come with a theorem rather than a heuristic.
//!
//! The catalog also flags the Test-1 caveat: vertex-transitivity
//! alone is NOT enough — generating-set centrality is the load-
//! bearing condition. This module reflects that distinction in
//! `CommutativityClass` (the planner gets the WHY, not just the
//! WHAT, of any commutation).
//!
//! References:
//! - `theory/kahler_upgrade/catalog.md §1.1`
//! - `theory/kahler_upgrade/IMPLEMENTATION_PLAN.md` L2

pub mod adjacency;
pub mod commutativity;

pub use adjacency::{AuxiliaryAdjacency, PrincipalAdjacency, SparseAdjacency};
pub use commutativity::{commute, CommutativityClass};
