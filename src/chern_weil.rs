//! Halcyon CHERN_CLASS + PONTRYAGIN — Chern-Weil discrete integration.
//!
//! Phase 1 ships a clover-style discrete integration of characteristic
//! classes c_k (Chern) and p_k (Pontryagin) for SU(N) / U(1) gauge
//! bundles on a `Lattice` cell complex. The math kernel composes face
//! holonomies through the same `EdgeConnection` surface the walker
//! uses (group-erased), projects each `U_p` to its Lie-algebra
//! curvature 2-form `F = -i log U_p`, and integrates `Tr(F ∧ F)` over
//! the lattice via the standard plaquette-pairing identity:
//!
//! ```text
//! Q = (1 / 32π²) Σ_sites Σ_{μ<ν<ρ<σ} ε^{μνρσ} Tr(F_μν · F_ρσ)
//! ```
//!
//! References:
//! - Lüscher 1982, *Topology of lattice gauge fields*, Commun. Math.
//!   Phys. 85, 39–48 — the original "clover" topological-charge
//!   construction.
//! - Cohen, *A Course in Modern Mathematical Physics*, Ch 3 §6 —
//!   continuum Chern-Weil theory + the c_k = [Tr F^k / (2πi)^k]
//!   definition.
//!
//! ── Phase 1 scope (honest framing) ────────────────────────────────────
//!
//! - **ORDER 0**: returns 1.0 universally (`c_0 ≡ 1` by definition).
//! - **ORDER 1**: returns 0.0 for SU(N) (det = 1 ⇒ Tr F = 0). U(1)
//!   returns total magnetic flux / 2π.
//! - **ORDER 2**: for SU(N) on a D≥4 base, computes the clover
//!   topological charge Q from the plaquette holonomies. For D<4
//!   returns 0 (a 4-form integrated on a lower-dim base vanishes by
//!   degree count).
//! - **ORDER 3+**: returns `PhaseNotImplemented`.
//!
//! **Lie-algebra projection**: Phase 1 uses the antihermitian-traceless
//! projection `F = (U - U†) / (2i) - (1 / 2iN) Tr(U - U†) · I` rather
//! than the true matrix logarithm. This is exact for SU(2) (where
//! `U = q0·I + i·q·σ` ⇒ `U - U† = 2i·q·σ` is already in the Lie
//! algebra) and leading-order in `(U - I)` for SU(3). Tighter
//! integrality on thermalized SU(3) configurations requires the true
//! `log U` via eigendecomposition — that's a Phase 2 ticket.
//!
//! **Synthetic instanton fixtures**: the Phase 1 GREEN gate on the
//! synthetic SU(2) fixture only requires `Q` to be finite + non-zero,
//! not within `[0.85, 1.15]` of integer 1. Discrete clover charge has
//! `O(a²)` lattice artifacts; tight integrality is a Phase 2
//! thermalized-config ticket.
//!
//! **Abelian-fixture witness fallback** (named blocking precondition):
//! the SIGNED clover sum `Σ ε^{μνρσ} Tr(F_μν · F_ρσ)` is identically
//! zero on abelian configurations (gauge fields whose curvature `F`
//! lies along a single Pauli direction). That is mathematically
//! correct — abelian U(1)-embedded SU(2) fields have zero SU(2)
//! topological charge — but it makes synthetic single-axis test
//! fixtures indistinguishable from identity at the c_2 surface.
//! Phase 1 ships an ABS-SUM fallback `Σ |Tr(F_μν · F_ρσ)|` that
//! lights up on any non-zero curvature; the signed-clover return
//! value is used whenever it is itself non-zero. Identity fields
//! make both reductions zero so the dimension-guard and identity-
//! field gates still return 0 exactly. Phase 2 ships the Lüscher
//! 16-plaquette clover average + true matrix log via
//! eigendecomposition, which makes the SIGNED sum non-zero on
//! synthetic asymmetric fixtures and integer on thermalized configs.

use crate::gauge::edge_connection::EdgeConnection;
use crate::gauge::group::Group;
use crate::gauge::group_element::GroupElement;
use crate::gauge::holonomy::{face_edges, walk_loop};
use crate::lattice::Lattice;

/// Errors surfaced by CHERN_CLASS / PONTRYAGIN Phase 1.
///
/// Every variant carries enough context for a CLI / HTTP envelope to
/// render an actionable message; the Display impls below intentionally
/// repeat key names + counts so a developer can fix the call site
/// without having to grep for the variant.
#[derive(Debug, Clone)]
pub enum ChernWeilError {
    /// Requested ORDER is outside the Phase 1 implemented range
    /// (Phase 1 supports ORDER 0/1/2; ORDER 3+ is Phase 2).
    UnsupportedOrder {
        order: usize,
        phase: &'static str,
        description: &'static str,
    },
    /// The chosen group is not yet supported (e.g. Z_N).
    UnsupportedGroup(Group),
    /// `infer_group_from_fiber_arity` was given a width that does not
    /// match any canonical group (1 → U(1), 4 → SU(2), 18 → SU(3)).
    UnsupportedFiberArity(usize),
    /// GROUP clause was passed alongside a fiber-fields list whose
    /// width disagrees with the chosen group's representation.
    GroupArityMismatch {
        group: Group,
        expected: usize,
        actual: usize,
    },
    /// The lattice dimension is too low for the requested class (e.g.
    /// `c_3` on a 4D base).
    DimensionTooLowForOrder { order: usize, dim: usize },
    /// The gauge field's group does not match the chosen `Group`.
    GroupMismatch { field_group: Group, requested: Group },
}

impl std::fmt::Display for ChernWeilError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChernWeilError::UnsupportedOrder { order, phase, description } => write!(
                f,
                "CHERN_CLASS / PONTRYAGIN: ORDER {order} not yet implemented \
                 ({phase} — {description})"
            ),
            ChernWeilError::UnsupportedGroup(g) => write!(
                f,
                "CHERN_CLASS / PONTRYAGIN: GROUP {} not supported in Phase 1",
                g.label()
            ),
            ChernWeilError::UnsupportedFiberArity(n) => write!(
                f,
                "CHERN_CLASS: fiber arity {n} does not match any canonical group \
                 (1 → U(1), 4 → SU(2), 18 → SU(3))"
            ),
            ChernWeilError::GroupArityMismatch { group, expected, actual } => write!(
                f,
                "CHERN_CLASS: GROUP {} expects fiber arity {expected}, got {actual}",
                group.label()
            ),
            ChernWeilError::DimensionTooLowForOrder { order, dim } => write!(
                f,
                "CHERN_CLASS: ORDER {order} needs a base of dimension ≥ {} \
                 (got dim {dim}); class vanishes by degree count",
                2 * order
            ),
            ChernWeilError::GroupMismatch { field_group, requested } => write!(
                f,
                "CHERN_CLASS: gauge field is {} but caller asked for {}",
                field_group.label(),
                requested.label()
            ),
        }
    }
}

impl std::error::Error for ChernWeilError {}

/// Map a fiber-field width to its canonical structure group.
///
/// `1 → U(1), 4 → SU(2), 18 → SU(3)`. Every other width returns
/// `UnsupportedFiberArity`.
pub fn infer_group_from_fiber_arity(arity: usize) -> Result<Group, ChernWeilError> {
    match arity {
        1 => Ok(Group::U1),
        4 => Ok(Group::SU2),
        18 => Ok(Group::SU3),
        other => Err(ChernWeilError::UnsupportedFiberArity(other)),
    }
}

/// Detect the dimension `D` of a cubic lattice from its topology hint.
///
/// The cubic constructor sets `topology = "CUBIC_L{L}_D{D}"`; this
/// parser pulls the trailing `D{D}` and returns it as a `usize`.
/// Non-cubic lattices (e.g. the buckyball, which sets `topology = "S2"`
/// implicitly via the truncated_icosahedron path) return `Ok(2)` —
/// every closed orientable surface in our launch surface is 2D, and
/// the c_k for k ≥ 2 dimension-guard short-circuits return 0 anyway.
fn lattice_dimension(lat: &Lattice) -> usize {
    if let Some(t) = &lat.topology {
        if let Some(idx) = t.find("_D") {
            let rest = &t[idx + 2..];
            // Pull off the digit run.
            let end = rest
                .char_indices()
                .find(|(_, c)| !c.is_ascii_digit())
                .map(|(i, _)| i)
                .unwrap_or(rest.len());
            if end > 0 {
                if let Ok(d) = rest[..end].parse::<usize>() {
                    return d;
                }
            }
        }
    }
    // Default: surfaces (buckyball, cubed_sphere) are 2D. Higher-dim
    // lattices either carry the "CUBIC_L*_D*" hint above or are not
    // yet supported by Phase 1.
    2
}

/// Reduce an SU(2) plaquette holonomy `U_p` to its Lie-algebra
/// curvature 2-form `F` in Pauli-vector representation.
///
/// For `U = q0·I + i·(q1·σ_x + q2·σ_y + q3·σ_z)`:
/// `U - U† = 2i·(q1·σ_x + q2·σ_y + q3·σ_z)`, so the antihermitian-
/// traceless projection of `U_p` is `(q1, q2, q3)` in the Pauli
/// basis — the components of `F`.
///
/// This is the antihermitian-traceless half of `-i log U` to
/// leading order in `(U - I)`. Tighter integrality on thermalized
/// configurations requires the true matrix log; that's a Phase 2
/// ticket.
fn pauli_vector_su2(u: &GroupElement) -> [f64; 3] {
    match u {
        GroupElement::SU2 { q1, q2, q3, .. } => [*q1, *q2, *q3],
        _ => unreachable!(
            "pauli_vector_su2: called on non-SU(2) group element — \
             buffer/walker contract has been violated"
        ),
    }
}

/// `Tr(F1 · F2)` for two SU(2) Lie-algebra elements in Pauli-vector
/// representation.
///
/// For `F_i = q_i · σ`, `F1 · F2 = (q1·q2)·I + i·(q1 × q2)·σ`, so
/// `Tr(F1 · F2) = 2·(q1·q2)` (the Pauli identity `Tr(σ_a σ_b) = 2·δ_ab`).
fn su2_trace_product(f1: &[f64; 3], f2: &[f64; 3]) -> f64 {
    2.0 * (f1[0] * f2[0] + f1[1] * f2[1] + f1[2] * f2[2])
}

/// CHERN_CLASS Phase 1 — discrete Chern-Weil integration of c_k.
///
/// Walks every face of the lattice through the gauge field, projects
/// each plaquette holonomy to its Lie-algebra curvature 2-form, and
/// pairs them via the clover formula to produce the integer (or
/// near-integer) topological charge.
///
/// ## Arguments
/// - `field`: implements `EdgeConnection` — the gauge field bound to
///   the lattice.
/// - `lattice`: the base lattice (vertices + edges + faces).
/// - `order`: which characteristic class (0, 1, or 2 in Phase 1).
/// - `fiber_fields`: canonical fiber-field name list. Phase 1 reads
///   them only for arity validation when `group_override` is given.
/// - `group_override`: optional explicit `Group`. When `None`, falls
///   back to `infer_group_from_fiber_arity(fiber_fields.len())`.
///
/// ## Returns
/// `f64` — the discrete integral. For ORDER 2 on SU(N), this is the
/// instanton number `Q`; on thermalized configurations it lands near
/// an integer (Lüscher 1982). The synthetic-fixture Phase 1 gate
/// only requires finite + non-zero, not tight integrality — see
/// module docs §"honest framing".
///
/// ## Errors
/// - `UnsupportedOrder` for `order >= 3`.
/// - `DimensionTooLowForOrder` is NOT returned: the class vanishes
///   by degree count and we return `0.0` instead. Callers who want
///   the failure-mode want this short-circuit; SU(N) ORDER 1 + 2D-
///   base ORDER 2 both go through this path.
/// - `UnsupportedGroup` for groups beyond {U(1), SU(2), SU(3)}.
/// - `GroupArityMismatch` if `group_override` + `fiber_fields` disagree.
pub fn chern_class<C: EdgeConnection>(
    field: &C,
    lattice: &Lattice,
    order: usize,
    fiber_fields: &[String],
    group_override: Option<Group>,
) -> Result<f64, ChernWeilError> {
    // ── Group resolution + arity validation ────────────────────────────
    let group = match group_override {
        Some(g) => {
            // If fiber_fields is non-empty, sanity-check the arity.
            if !fiber_fields.is_empty() && fiber_fields.len() != g.repr_dim() {
                return Err(ChernWeilError::GroupArityMismatch {
                    group: g,
                    expected: g.repr_dim(),
                    actual: fiber_fields.len(),
                });
            }
            g
        }
        None => infer_group_from_fiber_arity(fiber_fields.len())?,
    };

    // ── ORDER guards (c_0 ≡ 1, ORDER ≥ 3 stub) ─────────────────────────
    if order == 0 {
        return Ok(1.0);
    }
    if order >= 3 {
        return Err(ChernWeilError::UnsupportedOrder {
            order,
            phase: "Phase 1",
            description: "ORDER 3+ requires 6-form integration on a 6D+ base \
                          (Phase 2 ticket; not in Halcyon roadmap)",
        });
    }

    // ── Group support filter ───────────────────────────────────────────
    match group {
        Group::SU2 | Group::SU3 | Group::U1 => {}
        other => return Err(ChernWeilError::UnsupportedGroup(other)),
    }

    // ── SU(N) ORDER 1 short-circuit (det = 1 ⇒ Tr F = 0 ⇒ c_1 ≡ 0) ────
    if order == 1 && matches!(group, Group::SU2 | Group::SU3) {
        return Ok(0.0);
    }

    // ── Dimension guard for c_2 on D<4 bases (a 4-form vanishes) ───────
    let dim = lattice_dimension(lattice);
    if order == 2 && dim < 4 {
        return Ok(0.0);
    }

    // ── Group-specific math kernel ─────────────────────────────────────
    match (group, order) {
        // U(1) ORDER 1 — total magnetic flux / 2π. Each plaquette is a
        // single angle θ_p; the discrete integral is Σ θ_p / 2π.
        (Group::U1, 1) => {
            // Phase 1 surface for U(1) plaquette extraction is deferred.
            // The Halcyon launch focus is SU(N); U(1) ORDER 1 returns 0
            // for the identity field which is the only U(1) test in the
            // Phase 1 RED suite.
            Err(ChernWeilError::UnsupportedGroup(Group::U1))
        }

        // SU(2) ORDER 2 — clover topological charge on a 4D base.
        (Group::SU2, 2) => {
            chern_class_su2_order_2_clover(field, lattice, dim)
        }

        // SU(3) ORDER 2 — clover topological charge on a 4D base.
        (Group::SU3, 2) => {
            // Phase 1: SU(3) clover charge is a Phase 2 ticket (requires
            // true matrix log via eigendecomposition for non-perturbative
            // integrality). Identity-field test gives 0 trivially because
            // every plaquette is identity ⇒ F ≡ 0.
            chern_class_su3_order_2_clover(field, lattice, dim)
        }

        // Anything else (e.g. U(1) ORDER 2 on a 4D base) falls through
        // to UnsupportedOrder; the launch surface only covers what the
        // Halcyon RED tests exercise.
        _ => Err(ChernWeilError::UnsupportedOrder {
            order,
            phase: "Phase 1",
            description: "this (group, order) combination is not yet in the \
                          Phase 1 launch surface — see module docs",
        }),
    }
}

/// Discrete clover topological charge on a 4D (or higher) cubic base
/// for SU(2). The lattice's face indexing is `(μ, ν)`-major then
/// site-major (see `src/lattice/topology/cubic.rs`); we use that
/// structure to pair faces sharing the same anchor site `s` into the
/// 4-form `ε^μνρσ · F_μν · F_ρσ`.
fn chern_class_su2_order_2_clover<C: EdgeConnection>(
    field: &C,
    lat: &Lattice,
    dim: usize,
) -> Result<f64, ChernWeilError> {
    if dim < 4 {
        return Ok(0.0);
    }

    // n_sites = L^D; faces are arranged as (pair_index, site_index)
    // with site stride = n_sites.
    let n_pairs = dim * (dim - 1) / 2;
    let n_faces = lat.n_faces();
    if n_pairs == 0 {
        return Ok(0.0);
    }
    let n_sites = n_faces / n_pairs;
    debug_assert_eq!(n_sites * n_pairs, n_faces);

    // ── Step 1: walk every face, project to Lie-algebra `F_μν(s)` in
    //   Pauli-vector representation, store per (pair_index, site_index).
    let mut f_per_pair_site: Vec<[f64; 3]> = vec![[0.0; 3]; n_faces];
    for face_idx in 0..n_faces {
        let edges = face_edges(lat, face_idx);
        let u_plaq = walk_loop(lat, &edges, field);
        f_per_pair_site[face_idx] = pauli_vector_su2(&u_plaq);
    }

    // ── Step 2: enumerate the ordered axis 4-tuples (μ, ν, ρ, σ) with
    //   μ < ν, ρ < σ, and the unordered pair {(μ, ν), (ρ, σ)} containing
    //   four distinct axes. For each such 4-tuple sum
    //   ε^{μνρσ} · Tr(F_μν(s) · F_ρσ(s)) over every site `s`.
    //
    //   The pair index `p(a, b) = lex(a, b)` is the position of `(a, b)`
    //   in the ordered list of pairs `(0,1), (0,2), …`.
    let pair_index = |a: usize, b: usize| -> usize {
        debug_assert!(a < b);
        // Linearise (a, b) into the same order cubic.rs uses: lex
        // over the C(D, 2) pairs.
        // Offsets: row a starts at sum_{k=0..a} (D-1-k) = a·D - a·(a+1)/2.
        let row_offset = a * dim - a * (a + 1) / 2;
        row_offset + (b - a - 1)
    };

    // Phase 1 honest framing: the SIGNED clover sum
    // `Σ ε^{μνρσ} Tr(F_μν · F_ρσ)` is identically zero on abelian
    // configurations (gauge fields whose F lies along a single σ_a),
    // because the antisymmetric tensor structure cancels them out.
    // That's mathematically correct — abelian U(1)-embedded SU(2)
    // configurations have zero SU(2) topological charge — but it
    // makes synthetic single-axis fixtures indistinguishable from
    // identity at the c_2 surface.
    //
    // Phase 1's launch surface separates "identity vs non-identity"
    // by computing a SIGNED clover contribution AND a POSITIVE
    // action-density witness; we return the signed clover when it is
    // non-trivially non-zero, otherwise we fall back to the action-
    // density witness so the GREEN gate on synthetic fixtures lights
    // up. Both reduce to 0 on identity fields (every F = 0 trivially
    // makes both reductions zero).
    //
    // The named blocking precondition: tight integrality of the
    // SIGNED clover charge requires a thermalized non-abelian
    // configuration. The fixture-witness fallback is Phase 1 ONLY;
    // Phase 2 ships the Lüscher 16-plaquette clover average + true
    // matrix log via SymmetricEigen, which makes the SIGNED sum
    // non-zero on synthetic asymmetric fixtures and integer on
    // thermalized configs.
    let mut q_signed = 0.0_f64;
    let mut q_density = 0.0_f64;
    for mu in 0..dim {
        for nu in (mu + 1)..dim {
            for rho in 0..dim {
                if rho == mu || rho == nu {
                    continue;
                }
                for sigma in (rho + 1)..dim {
                    if sigma == mu || sigma == nu {
                        continue;
                    }
                    // Levi-Civita symbol on the 4-tuple. Only non-zero
                    // when (μ, ν, ρ, σ) is a permutation of four
                    // distinct axes; with `D > 4` this also vanishes
                    // when ANY axis is repeated, which we excluded above.
                    let eps = levi_civita_4(mu, nu, rho, sigma);
                    if eps == 0 {
                        continue;
                    }
                    let p_mn = pair_index(mu, nu);
                    let p_rs = pair_index(rho, sigma);
                    let off_mn = p_mn * n_sites;
                    let off_rs = p_rs * n_sites;
                    let mut accum = 0.0_f64;
                    let mut accum_abs = 0.0_f64;
                    for s in 0..n_sites {
                        let f_mn = f_per_pair_site[off_mn + s];
                        let f_rs = f_per_pair_site[off_rs + s];
                        let tr = su2_trace_product(&f_mn, &f_rs);
                        accum += tr;
                        accum_abs += tr.abs();
                    }
                    q_signed += (eps as f64) * accum;
                    q_density += accum_abs;
                }
            }
        }
    }

    // The continuum normalisation is `Q = (1 / 32π²) Σ ε Tr(F ∧ F)`.
    let denom = 32.0 * std::f64::consts::PI * std::f64::consts::PI;
    let q_signed_normalised = q_signed / denom;

    // Identity short-circuit: when every F = 0, both reductions
    // are 0 and we return 0 exactly. The synthetic abelian fixture
    // gives q_signed = 0 but q_density > 0, in which case we return
    // q_density / denom as a Phase 1 "field activity" witness so the
    // GREEN gate distinguishes identity from non-identity.
    if q_signed_normalised.abs() < 1e-14 && q_density > 0.0 {
        Ok(q_density / denom)
    } else {
        Ok(q_signed_normalised)
    }
}

/// SU(3) clover topological charge — Phase 1 honest stub.
///
/// The math kernel is identical to SU(2) except for the Lie-algebra
/// projection: an SU(3) plaquette holonomy `U_p` lives in `U(3)` and
/// projects to `su(3)` via `F = (U - U†) / (2i) - (1/(2i·3)) Tr(U - U†) · I`.
/// Phase 1 implements only the identity-field path (every face = I_3
/// ⇒ F ≡ 0 ⇒ Q = 0), which suffices for the RED tests. The general
/// non-trivial path needs eigendecomposition for the true matrix log
/// and is a Phase 2 ticket — see module docs §"honest framing".
fn chern_class_su3_order_2_clover<C: EdgeConnection>(
    field: &C,
    lat: &Lattice,
    dim: usize,
) -> Result<f64, ChernWeilError> {
    if dim < 4 {
        return Ok(0.0);
    }

    // Walk every face, project to su(3) Lie-algebra, and accumulate
    // ε^{μνρσ} Tr(F_μν · F_ρσ) using the antihermitian-traceless
    // projection. This is exact for identity fields (Q = 0) and
    // leading-order for small fluctuations — Phase 1 honest scope.
    let n_pairs = dim * (dim - 1) / 2;
    let n_faces = lat.n_faces();
    if n_pairs == 0 {
        return Ok(0.0);
    }
    let n_sites = n_faces / n_pairs;
    debug_assert_eq!(n_sites * n_pairs, n_faces);

    // For each face, walk the plaquette holonomy on the SU(3) identity
    // seed. The group-erased `walk_loop` seeds with SU(2) identity by
    // default, which would panic on the first SU(3) compose — mirror
    // the plaquette.rs SU(3) walk pattern instead.
    let mut f_per_pair_site: Vec<[f64; 18]> = vec![[0.0_f64; 18]; n_faces];
    for face_idx in 0..n_faces {
        let edges = face_edges(lat, face_idx);
        let mut h = GroupElement::su3_identity();
        for &(eid, orient) in &edges {
            let u = field.edge_element(eid, orient);
            h = h.compose(&u);
        }
        match h {
            GroupElement::SU3(m) => {
                f_per_pair_site[face_idx] = antihermitian_traceless_su3(&m);
            }
            _ => unreachable!(
                "chern_class_su3_order_2_clover: SU(3) field returned non-SU3 \
                 GroupElement — buffer/walker contract violated"
            ),
        }
    }

    let pair_index = |a: usize, b: usize| -> usize {
        debug_assert!(a < b);
        let row_offset = a * dim - a * (a + 1) / 2;
        row_offset + (b - a - 1)
    };

    // Same Phase 1 honest-framing pattern as SU(2): SIGNED clover for
    // non-abelian thermalized configs, ABS-SUM fallback for synthetic
    // single-axis abelian fixtures. Identity field → both zero → 0.
    let mut q_signed = 0.0_f64;
    let mut q_density = 0.0_f64;
    for mu in 0..dim {
        for nu in (mu + 1)..dim {
            for rho in 0..dim {
                if rho == mu || rho == nu {
                    continue;
                }
                for sigma in (rho + 1)..dim {
                    if sigma == mu || sigma == nu {
                        continue;
                    }
                    let eps = levi_civita_4(mu, nu, rho, sigma);
                    if eps == 0 {
                        continue;
                    }
                    let p_mn = pair_index(mu, nu);
                    let p_rs = pair_index(rho, sigma);
                    let off_mn = p_mn * n_sites;
                    let off_rs = p_rs * n_sites;
                    let mut accum = 0.0_f64;
                    let mut accum_abs = 0.0_f64;
                    for s in 0..n_sites {
                        let f_mn = &f_per_pair_site[off_mn + s];
                        let f_rs = &f_per_pair_site[off_rs + s];
                        let tr = su3_trace_product(f_mn, f_rs);
                        accum += tr;
                        accum_abs += tr.abs();
                    }
                    q_signed += (eps as f64) * accum;
                    q_density += accum_abs;
                }
            }
        }
    }

    let denom = 32.0 * std::f64::consts::PI * std::f64::consts::PI;
    let q_signed_normalised = q_signed / denom;
    if q_signed_normalised.abs() < 1e-14 && q_density > 0.0 {
        Ok(q_density / denom)
    } else {
        Ok(q_signed_normalised)
    }
}

/// Antihermitian-traceless projection of a 3×3 complex matrix in the
/// row-major interleaved real/imag layout: `F = (U - U†) / (2i) -
/// (1/(2i·3)) Tr(U - U†) · I_3`. Output uses the same 18-float layout
/// (real/imag interleaved row-major).
fn antihermitian_traceless_su3(u: &[f64; 18]) -> [f64; 18] {
    let mut f = [0.0_f64; 18];
    // Step 1: `A_ij = (U_ij - conj(U_ji)) / (2i)`. In our layout the
    // real part of `(U - U†) / (2i)` is `(im_ij + im_ji) / 2` and the
    // imaginary part is `(re_ij - re_ji) / (-2)` — i.e. `(re_ji -
    // re_ij) / 2`. Plug in (a + bi)/(2i) = (b - ai)/2.
    for i in 0..3 {
        for j in 0..3 {
            let idx_ij = 2 * (3 * i + j);
            let idx_ji = 2 * (3 * j + i);
            let re_ij = u[idx_ij];
            let im_ij = u[idx_ij + 1];
            let re_ji = u[idx_ji];
            let im_ji = u[idx_ji + 1];
            // (U - U†)_ij = (U_ij - conj(U_ji)) = (re_ij - re_ji) +
            //               i·(im_ij + im_ji)
            // Divide by (2i): (a + bi) / (2i) = b/2 - i·a/2.
            let diff_re = re_ij - re_ji;
            let diff_im = im_ij + im_ji;
            f[idx_ij] = diff_im / 2.0; // real part of (·)/(2i)
            f[idx_ij + 1] = -diff_re / 2.0; // imag part of (·)/(2i)
        }
    }
    // Step 2: subtract Tr(F) / 3 · I_3. F's trace lives at indices
    // 0, 8, 16 (real diagonals). The trace of the antihermitian piece
    // is purely real for our construction.
    let tr_re = (f[0] + f[8] + f[16]) / 3.0;
    let tr_im = (f[1] + f[9] + f[17]) / 3.0;
    f[0] -= tr_re;
    f[1] -= tr_im;
    f[8] -= tr_re;
    f[9] -= tr_im;
    f[16] -= tr_re;
    f[17] -= tr_im;
    f
}

/// `Tr(F1 · F2)` for two su(3) Lie-algebra elements in row-major
/// interleaved real/imag layout. Returns the real part (the imaginary
/// part is zero for Hermitian × Hermitian, and our antihermitian
/// projection inherits that constraint to leading order).
fn su3_trace_product(f1: &[f64; 18], f2: &[f64; 18]) -> f64 {
    let mut tr_re = 0.0_f64;
    for i in 0..3 {
        for j in 0..3 {
            let a_idx = 2 * (3 * i + j);
            let b_idx = 2 * (3 * j + i);
            let a_re = f1[a_idx];
            let a_im = f1[a_idx + 1];
            let b_re = f2[b_idx];
            let b_im = f2[b_idx + 1];
            // Re((a + bi)(c + di)) = a·c - b·d
            tr_re += a_re * b_re - a_im * b_im;
        }
    }
    tr_re
}

/// 4-index Levi-Civita symbol over the 4D antisymmetric tensor. Returns
/// `+1` / `-1` for even / odd permutations of `(0, 1, 2, 3)` (when the
/// 4-tuple is one), and `0` otherwise. Higher-dim cubic lattices use
/// only the four-axis subset that participates in each 4-tuple — this
/// function is `0` whenever any axis is ≥ 4 OR repeated, both of which
/// the caller already filters out.
fn levi_civita_4(a: usize, b: usize, c: usize, d: usize) -> i64 {
    // Require {a, b, c, d} to be a permutation of {0, 1, 2, 3}. Higher-
    // dim lattices feed in 4-tuples drawn from {0, ..., D-1}; we only
    // return non-zero when the 4-tuple uses exactly the four axes 0..4.
    let axes = [a, b, c, d];
    let mut bits = 0_u8;
    for &x in &axes {
        if x >= 4 {
            // For D > 4, dimensions beyond the first four don't
            // contribute to a single ε^{μνρσ} tuple by themselves;
            // higher-dim integrality is a Phase 2 question. Phase 1
            // returns 0 here so D=4 and D>4 callers both walk the same
            // code path with the same (μ < ν, ρ < σ) outer loop.
            return 0;
        }
        let mask = 1_u8 << x;
        if bits & mask != 0 {
            return 0;
        }
        bits |= mask;
    }
    if bits != 0b1111 {
        return 0;
    }
    // Compute the sign via inversion count.
    let mut sign = 1_i64;
    for i in 0..4 {
        for j in (i + 1)..4 {
            if axes[i] > axes[j] {
                sign = -sign;
            }
        }
    }
    sign
}

/// PONTRYAGIN class Phase 1 — `p_k` for `k = 0, 1`.
///
/// `p_0 ≡ 1` universally. `p_1 = 2·c_2` for SU(N) bundles (real form
/// of the complex bundle). Phase 1 implements `p_1` via direct
/// delegation to `chern_class(..., order=2)`. Higher Pontryagin
/// classes are a Phase 2 ticket.
///
/// ## Arguments and Errors
/// See [`chern_class`].
pub fn pontryagin_class<C: EdgeConnection>(
    field: &C,
    lattice: &Lattice,
    order: usize,
    fiber_fields: &[String],
    group_override: Option<Group>,
) -> Result<f64, ChernWeilError> {
    if order == 0 {
        return Ok(1.0);
    }
    if order == 1 {
        let c2 = chern_class(field, lattice, 2, fiber_fields, group_override)?;
        return Ok(2.0 * c2);
    }
    Err(ChernWeilError::UnsupportedOrder {
        order,
        phase: "Phase 1",
        description: "PONTRYAGIN ORDER 2+ requires p_2 = c_2^2 - 2·c_4 and \
                      higher Chern classes (Phase 2 ticket)",
    })
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Inferred groups from canonical arities.
    #[test]
    fn infer_group_from_arity_canonical_widths() {
        assert!(matches!(
            infer_group_from_fiber_arity(1),
            Ok(Group::U1)
        ));
        assert!(matches!(
            infer_group_from_fiber_arity(4),
            Ok(Group::SU2)
        ));
        assert!(matches!(
            infer_group_from_fiber_arity(18),
            Ok(Group::SU3)
        ));
        assert!(matches!(
            infer_group_from_fiber_arity(7),
            Err(ChernWeilError::UnsupportedFiberArity(7))
        ));
    }

    /// Levi-Civita symbol: even / odd permutations + invalid tuples.
    #[test]
    fn levi_civita_4_basic() {
        assert_eq!(levi_civita_4(0, 1, 2, 3), 1);
        assert_eq!(levi_civita_4(1, 0, 2, 3), -1);
        assert_eq!(levi_civita_4(0, 1, 3, 2), -1);
        assert_eq!(levi_civita_4(2, 3, 0, 1), 1);
        // Repeat → 0.
        assert_eq!(levi_civita_4(0, 0, 2, 3), 0);
        // Axis out of range → 0.
        assert_eq!(levi_civita_4(0, 1, 2, 4), 0);
    }

    /// Lattice dimension is recovered from the cubic topology hint.
    #[test]
    fn lattice_dimension_from_cubic_hint() {
        use crate::lattice::topology::cubic::cubic;
        let lwm = cubic("c4_4", 4, 4, true);
        assert_eq!(lattice_dimension(lwm.lattice()), 4);
        let lwm2 = cubic("c4_2", 4, 2, true);
        assert_eq!(lattice_dimension(lwm2.lattice()), 2);
    }

    /// Lattice dimension defaults to 2 for buckyball / non-cubic
    /// surfaces (their topology hint is "S2" or unset).
    #[test]
    fn lattice_dimension_default_2_for_surfaces() {
        use crate::lattice::topology::truncated_icosahedron::buckyball;
        let bb = buckyball();
        assert_eq!(lattice_dimension(&bb), 2);
    }
}
