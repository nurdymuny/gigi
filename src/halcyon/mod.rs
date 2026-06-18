//! HALCYON — Davis Wilson Lattice substrate (Part I).
//!
//! This module is gated behind the `halcyon` Cargo feature. When the
//! feature is OFF, none of these types or impls compile in, and the
//! engine's default-feature build is byte-identical to the pre-HAL
//! tree (the optionality contract from
//! `theory/kahler_upgrade/IMPLEMENTATION_PLAN.md`).
//!
//! Part I scope (`HALCYON_PART_I_GATES.md`):
//!
//! - **I.1** `lattice` — `Lattice` storage round-trip (GQL re-emit form
//!   is the canonical serialization).
//! - **I.2** `truncated_icosahedron` — buckyball constructor (60v / 90e
//!   / 32f, Euler χ = 2, S² topology).
//! - **I.3** `edge_connection` + `group_element` — object-safe
//!   `EdgeConnection` trait + `GroupElement` enum (the
//!   group-erasure gate; only `SU2` has implemented math at launch).
//! - **I.4** `holonomy` — walker; `walk_loop(lattice, edges, conn)`
//!   returns a `GroupElement` (identity on every face for an
//!   identity connection on every edge).
//! - **I.5** orientation false-pass guard (lives in `holonomy.rs`).
//! - **I.6** bit-identity gold gate (in `tests/halcyon_part_i_bit_identity.rs`).
//! - **I.7** parser surface (in `src/parser.rs` — feature-gated).
//! - **I.8** executor + `SHOW LATTICE` round-trip (in `registry.rs` +
//!   `src/parser.rs::execute`).
//! - **I.9** implementation log (in
//!   `theory/halcyon/HALCYON_PART_I_IMPLEMENTATION_LOG.md`).
//!
//! Quaternion convention (pinned from harvest phase, see
//! `tests/fixtures/halcyon/buckyball_gold_provenance.json`):
//!
//! - Scalar-first layout `(q0, q1, q2, q3)` with `q0 = cos(θ/2)`.
//! - Matrix form `A = q0·I + i·(q1·σ_x + q2·σ_y + q3·σ_z)`.
//!   `Re Tr A = 2·q0`, `det A = q0² + q1² + q2² + q3² = 1`.
//! - Product rule (left-action; matches
//!   `davis-wilson-lattice/inertia_damping/buckyball_action.py`):
//!   `c0 = a0·b0 - a·b`, `c_vec = a0·b_vec + b0·a_vec - a × b`.
//! - Conjugate: `qconj(a) = (a0, -a1, -a2, -a3)`.
//! - Face-holonomy composition is **left-to-right** in cyclic face
//!   order: `U_f = U_e0^s0 · U_e1^s1 · … · U_ek^sk` (multiplication
//!   left-associative; `h ← qmul(h, U_e^s)` per edge).

pub mod lattice;
pub mod truncated_icosahedron;
pub mod group_element;
pub mod edge_connection;
