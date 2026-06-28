//! Canonical topology-hint registry.
//!
//! Closes the "topology_hint const table" row of the AURORA Phase 1
//! status board (receipt: `theory/aurora/
//! GIGI_TO_AURORA_2026-06-19_v0_1_REPLY_2.md`). This module is the
//! single source of truth that maps a canonical constructor identifier
//! (the right-hand side of `LATTICE name FROM <CANONICAL_ID>`) to the
//! topology hint string it lives in (e.g. `"S2"` for the 2-sphere).
//!
//! The table is metadata-only in Phase 1: the parser executor still
//! receives its hint from either the explicit `TOPOLOGY` clause or the
//! constructor's own stamp. The table exists so callers — and
//! downstream verifiers — can ask "what topology does CANONICAL_ID
//! belong to?" without instantiating the constructor.
//!
//! Extension protocol: **future topologies extend this registry by
//! adding a row to `TOPOLOGY_HINTS`**. Do not introduce a second table;
//! that path leads to drift between the table and the constructor.
//!
//! Contract:
//!
//! - Lookup is case-insensitive over the canonical identifier (e.g.
//!   `"cubed_sphere"`, `"Cubed_Sphere"`, and `"CUBED_SPHERE"` all
//!   resolve to the same entry). Stored keys are upper-case.
//! - Unknown identifiers return `None`, never a default — silent
//!   defaults would hide drift between the table and the constructors.

/// Canonical (CANONICAL_ID, topology_hint) rows. Keys MUST be stored
/// upper-case; `lookup` upper-cases the query before probing.
///
/// Add new topologies here. Keep the list sorted alphabetically by key
/// so the diff for a new entry is a one-line insertion.
///
/// CUBIC note: this table is parameterless metadata, so the entry
/// stores the `D`-torus "family" hint `"T^D"`. The instantiated
/// lattice carries the fully-resolved per-(L,D) topology string (e.g.
/// `"CUBIC_L12_D4"`) stamped by the constructor at build time —
/// callers needing the resolved value read it off the `Lattice` after
/// construction; callers needing only the family hint use this table.
const TOPOLOGY_HINTS: &[(&str, &str)] = &[
    ("CUBED_SPHERE", "S2"),
    ("CUBIC", "T^D"),
    ("TRUNCATED_ICOSAHEDRON", "S2"),
];

/// Resolve a canonical constructor identifier to its topology hint.
///
/// Returns `Some(hint)` if `canonical_id` is registered in
/// [`TOPOLOGY_HINTS`] (matched case-insensitively), `None` otherwise.
/// `None` is never a default — it means the caller asked about a
/// constructor the registry does not know.
pub fn lookup(canonical_id: &str) -> Option<&'static str> {
    let needle = canonical_id.to_ascii_uppercase();
    TOPOLOGY_HINTS
        .iter()
        .find(|(name, _)| *name == needle.as_str())
        .map(|(_, hint)| *hint)
}

/// Alias for [`lookup`] using the verbose name from the AURORA spec.
///
/// Provided so call sites that read more clearly with the long name
/// (`topology_hint_for("CUBED_SPHERE")`) do not have to qualify
/// further. Behaviour is identical to [`lookup`].
pub fn topology_hint_for(constructor_name: &str) -> Option<&'static str> {
    lookup(constructor_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cubed_sphere_resolves_to_s2() {
        assert_eq!(lookup("CUBED_SPHERE"), Some("S2"));
    }

    #[test]
    fn truncated_icosahedron_resolves_to_s2() {
        assert_eq!(lookup("TRUNCATED_ICOSAHEDRON"), Some("S2"));
    }

    #[test]
    fn lookup_is_case_insensitive() {
        assert_eq!(lookup("cubed_sphere"), Some("S2"));
        assert_eq!(lookup("Cubed_Sphere"), Some("S2"));
        assert_eq!(lookup("truncated_icosahedron"), Some("S2"));
    }

    #[test]
    fn unknown_returns_none() {
        assert_eq!(lookup("Z3/SOMETHING_WEIRD"), None);
        assert_eq!(lookup(""), None);
        assert_eq!(lookup("CUBED"), None);
    }

    #[test]
    fn topology_hint_for_alias_matches_lookup() {
        assert_eq!(
            topology_hint_for("CUBED_SPHERE"),
            lookup("CUBED_SPHERE"),
        );
        assert_eq!(
            topology_hint_for("TRUNCATED_ICOSAHEDRON"),
            lookup("TRUNCATED_ICOSAHEDRON"),
        );
        assert_eq!(topology_hint_for("nope"), lookup("nope"));
    }
}
