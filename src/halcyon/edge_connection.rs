//! `EdgeConnection` — group-erased trait for reading a per-edge
//! group element off a connection.
//!
//! Closes the trait half of TDD-HAL-I.3. Object-safe; the walker
//! reads through `&dyn EdgeConnection` so the same code path
//! serves the synthetic Part-I test connections, the SU(2) gauge
//! field that lands in Part II, and any future group implementation
//! without recompiling the walker.
//!
//! The trait's contract is one method:
//!
//! ```ignore
//! fn edge_element(&self, edge: EdgeId, orientation: EdgeOrientation) -> GroupElement;
//! ```
//!
//! Forward orientation returns the canonical U_e committed at the
//! edge; reverse orientation returns `U_e.inverse()`. Implementations
//! must return the same variant of `GroupElement` for every edge
//! (mixing groups across the same lattice is an unrecoverable
//! programming error and is caught by the walker's `compose` panic).

use super::group_element::GroupElement;
use super::lattice::{EdgeId, EdgeOrientation};

/// Object-safe trait the walker reads through.
pub trait EdgeConnection {
    /// Return the group element committed at `edge`, traversed in
    /// the requested `orientation`. Forward = canonical U_e;
    /// reverse = U_e.inverse().
    fn edge_element(&self, edge: EdgeId, orientation: EdgeOrientation) -> GroupElement;
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Test-only `FixedEdgeConnection`. Backed by a `HashMap<EdgeId,
    //! GroupElement>`; unspecified edges default to `SU(2)` identity.
    //!
    //! Strictly `#[cfg(test)]` so the shipped artifact does not
    //! depend on a HashMap-backed connection — the Part-II GaugeField
    //! ships its own dense buffer.

    use std::collections::HashMap;

    use super::*;

    pub struct FixedEdgeConnection {
        pub elements: HashMap<EdgeId, GroupElement>,
    }

    impl FixedEdgeConnection {
        pub fn identity_everywhere() -> Self {
            Self {
                elements: HashMap::new(),
            }
        }

        pub fn with_edge(mut self, edge: EdgeId, g: GroupElement) -> Self {
            self.elements.insert(edge, g);
            self
        }
    }

    impl EdgeConnection for FixedEdgeConnection {
        fn edge_element(
            &self,
            edge: EdgeId,
            orientation: EdgeOrientation,
        ) -> GroupElement {
            let canonical = self
                .elements
                .get(&edge)
                .copied()
                .unwrap_or_else(GroupElement::su2_identity);
            match orientation {
                EdgeOrientation::Forward => canonical,
                EdgeOrientation::Reverse => canonical.inverse(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::FixedEdgeConnection;
    use super::*;

    /// TDD-HAL-I.3 — identity FixedConnection returns SU(2) identity
    /// for any edge in both orientations.
    #[test]
    fn tdd_hal_i_3_identity_connection_is_identity() {
        let conn = FixedEdgeConnection::identity_everywhere();
        let id = GroupElement::su2_identity();
        for edge in 0..32usize {
            assert_eq!(conn.edge_element(edge, EdgeOrientation::Forward), id);
            assert_eq!(conn.edge_element(edge, EdgeOrientation::Reverse), id);
        }
    }

    /// Compose(identity, identity) == identity.
    #[test]
    fn tdd_hal_i_3_identity_compose_identity() {
        let id = GroupElement::su2_identity();
        assert_eq!(id.compose(&id), id);
    }

    /// Inverse(identity) == identity.
    #[test]
    fn tdd_hal_i_3_identity_inverse_is_identity() {
        let id = GroupElement::su2_identity();
        assert_eq!(id.inverse(), id);
    }

    /// Trait is object-safe — we can store `Box<dyn EdgeConnection>`.
    #[test]
    fn edge_connection_is_object_safe() {
        let conn: Box<dyn EdgeConnection> =
            Box::new(FixedEdgeConnection::identity_everywhere());
        let _ = conn.edge_element(0, EdgeOrientation::Forward);
    }
}
