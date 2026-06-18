//! `Lattice` — graph topology with incidence + face-cycle tables.
//!
//! Behind the `lattice` Cargo feature. General-purpose graph-topology
//! primitive: vertices + signed edges + face-cycle table + an
//! optional topology hint string. Halcyon's Davis Wilson Lattice
//! substrate is the first consumer, but the type is not Halcyon-
//! specific — it's the discrete-manifold base any subsequent gauge-
//! field module (this engine's `gauge` feature, or downstream
//! callers) attaches a connection to.
//!
//! Submodules:
//!
//! - `registry` — in-memory `LATTICE`-keyed registry the executor
//!   materializes declared lattices into; `SHOW LATTICE name` reads
//!   back through it.
//! - `topology` — canonical-graph constructors (buckyball / future
//!   cubic / hexagonal / …) that produce a `Lattice` ready for the
//!   `LATTICE name FROM CANONICAL_ID` GQL shorthand.
//!
//! Closes TDD-HAL-I.1 (storage round-trip). The canonical
//! serialization is the GQL re-emit form of `LATTICE`; a `Lattice`
//! parses back from its own emitted form bit-identical (modulo
//! whitespace).
//!
//! Storage layout:
//!
//! - `n_vertices` — number of vertices V.
//! - `edges` — `Vec<(VertexId, VertexId)>`, length E. The pair order
//!   defines the canonical orientation of the edge (edge index 0 is
//!   `edges[0]`, oriented `edges[0].0 → edges[0].1`); the walker's
//!   `EdgeOrientation::Reverse` traverses against that canonical
//!   direction and reads through the connection's `inverse` at use
//!   site.
//! - `faces` — `Vec<Vec<VertexId>>`, length F. Each face is an
//!   ordered cycle of vertex indices in the orientation the LATTICE
//!   declaration commits to; the walker resolves the per-edge sign
//!   from `(face[i], face[(i+1) % len])` against the `edges`
//!   incidence table.
//! - `topology` — optional hint string (`"S2"`, `"T2"`, `"R3"`, …).
//!   Free-form per the spec; the engine does not interpret it in
//!   Part I beyond round-tripping.

pub mod registry;
pub mod topology;

use std::fmt;

/// Index into `Lattice::edges`. Engine-internal handle; the parser
/// surface does not expose `EdgeId` directly.
pub type EdgeId = usize;

/// Index of a vertex (0..n_vertices).
pub type VertexId = usize;

/// Edge traversal direction. `Forward` means the canonical
/// orientation from `Lattice::edges`; `Reverse` means against it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeOrientation {
    Forward,
    Reverse,
}

impl EdgeOrientation {
    /// Sign integer (`+1` / `-1`) used in the GQL re-emit form.
    pub fn sign(self) -> i8 {
        match self {
            EdgeOrientation::Forward => 1,
            EdgeOrientation::Reverse => -1,
        }
    }
}

/// Declared graph topology. The canonical serialization is the GQL
/// re-emit form (see `Lattice::to_gql`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lattice {
    /// User-facing name (the `ident` in `LATTICE ident …;`).
    pub name: String,
    /// V — number of vertices.
    pub n_vertices: usize,
    /// E pairs of vertex indices. `edges[i]` defines edge index `i`,
    /// oriented `edges[i].0 → edges[i].1`.
    pub edges: Vec<(VertexId, VertexId)>,
    /// F ordered cycles of vertex indices. Walker resolves per-edge
    /// signs by looking each `(face[i], face[i+1])` consecutive pair
    /// up in `edges`.
    pub faces: Vec<Vec<VertexId>>,
    /// Optional topology hint (`"S2"` / `"T2"` / `"R3"` / …). Stored
    /// verbatim, not interpreted in Part I.
    pub topology: Option<String>,
}

impl Lattice {
    /// Construct an explicit Lattice. Performs no validation — the
    /// caller is responsible for ensuring `faces` reference valid
    /// edges. Validation is a separate Part-II concern when the
    /// gauge field gets attached.
    pub fn new(
        name: impl Into<String>,
        n_vertices: usize,
        edges: Vec<(VertexId, VertexId)>,
        faces: Vec<Vec<VertexId>>,
        topology: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            n_vertices,
            edges,
            faces,
            topology,
        }
    }

    /// Number of edges E.
    pub fn n_edges(&self) -> usize {
        self.edges.len()
    }

    /// Number of faces F.
    pub fn n_faces(&self) -> usize {
        self.faces.len()
    }

    /// Euler characteristic χ = V − E + F.
    pub fn euler_characteristic(&self) -> i64 {
        self.n_vertices as i64 - self.n_edges() as i64 + self.n_faces() as i64
    }

    /// Resolve a face's consecutive vertex pair `(a, b)` to the edge
    /// index + orientation. Returns `None` if the pair is not an
    /// edge of the lattice in either direction.
    pub fn resolve_edge(&self, a: VertexId, b: VertexId) -> Option<(EdgeId, EdgeOrientation)> {
        for (idx, &(u, v)) in self.edges.iter().enumerate() {
            if u == a && v == b {
                return Some((idx, EdgeOrientation::Forward));
            }
            if u == b && v == a {
                return Some((idx, EdgeOrientation::Reverse));
            }
        }
        None
    }

    /// Emit the canonical GQL re-emit form (explicit `VERTICES n EDGES
    /// ((…)) FACES ((…))` shape). Round-trips through `parse_gql`
    /// bit-identical (modulo whitespace).
    pub fn to_gql(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("LATTICE {} VERTICES {}", self.name, self.n_vertices));
        out.push_str(" EDGES (");
        for (i, (u, v)) in self.edges.iter().enumerate() {
            if i > 0 {
                out.push_str(",");
            }
            out.push_str(&format!("({},{})", u, v));
        }
        out.push_str(") FACES (");
        for (i, face) in self.faces.iter().enumerate() {
            if i > 0 {
                out.push_str(",");
            }
            out.push_str("(");
            for (j, v) in face.iter().enumerate() {
                if j > 0 {
                    out.push_str(",");
                }
                out.push_str(&format!("{}", v));
            }
            out.push_str(")");
        }
        out.push_str(")");
        if let Some(ref t) = self.topology {
            out.push_str(&format!(" TOPOLOGY \"{}\"", t));
        }
        out.push_str(";");
        out
    }

    /// Parse the canonical GQL re-emit form back into a `Lattice`.
    /// Tolerant of whitespace; rejects anything else.
    pub fn from_gql(gql: &str) -> Result<Self, String> {
        let s = gql.trim();
        let s = s.strip_suffix(';').ok_or_else(|| "Lattice GQL must end with ';'".to_string())?;
        let s = s.trim();
        // LATTICE <name> VERTICES <n> EDGES (...) FACES (...) [TOPOLOGY "..."]
        let s = s.strip_prefix("LATTICE")
            .ok_or_else(|| "Lattice GQL must start with LATTICE".to_string())?;
        let s = s.trim_start();
        // name
        let (name, rest) = take_ident(s)?;
        let s = rest.trim_start();
        let s = s.strip_prefix("VERTICES")
            .ok_or_else(|| "Expected VERTICES after lattice name".to_string())?;
        let s = s.trim_start();
        let (n_vertices, rest) = take_uint(s)?;
        let s = rest.trim_start();
        let s = s.strip_prefix("EDGES")
            .ok_or_else(|| "Expected EDGES after VERTICES n".to_string())?;
        let s = s.trim_start();
        let (edges_str, rest) = take_paren_block(s)?;
        let edges = parse_edge_list(&edges_str)?;
        let s = rest.trim_start();
        let s = s.strip_prefix("FACES")
            .ok_or_else(|| "Expected FACES after EDGES (…)".to_string())?;
        let s = s.trim_start();
        let (faces_str, rest) = take_paren_block(s)?;
        let faces = parse_face_list(&faces_str)?;
        let s = rest.trim_start();
        let topology = if let Some(rest) = s.strip_prefix("TOPOLOGY") {
            let r = rest.trim_start();
            let (t, _) = take_string(r)?;
            Some(t)
        } else {
            None
        };
        Ok(Lattice::new(name, n_vertices, edges, faces, topology))
    }
}

impl fmt::Display for Lattice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_gql())
    }
}

// ── tiny scanner helpers (Part I keeps them local; the GQL parser
//    proper lives in src/parser.rs and exercises the same shape via
//    its own tokenizer).

fn take_ident(s: &str) -> Result<(String, &str), String> {
    let end = s
        .char_indices()
        .find(|(_, c)| !(c.is_ascii_alphanumeric() || *c == '_'))
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    if end == 0 {
        return Err("Expected identifier".to_string());
    }
    Ok((s[..end].to_string(), &s[end..]))
}

fn take_uint(s: &str) -> Result<(usize, &str), String> {
    let end = s
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit())
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    if end == 0 {
        return Err("Expected unsigned integer".to_string());
    }
    let n: usize = s[..end].parse().map_err(|e| format!("bad uint: {e}"))?;
    Ok((n, &s[end..]))
}

fn take_string(s: &str) -> Result<(String, &str), String> {
    let s = s.strip_prefix('"').ok_or_else(|| "Expected '\"'".to_string())?;
    let end = s.find('"').ok_or_else(|| "Unterminated string".to_string())?;
    Ok((s[..end].to_string(), &s[end + 1..]))
}

fn take_paren_block(s: &str) -> Result<(String, &str), String> {
    let s = s.strip_prefix('(').ok_or_else(|| "Expected '('".to_string())?;
    // Find matching close-paren accounting for nesting.
    let mut depth = 1usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok((s[..i].to_string(), &s[i + 1..]));
                }
            }
            _ => {}
        }
    }
    Err("Unbalanced parentheses".to_string())
}

fn parse_edge_list(body: &str) -> Result<Vec<(VertexId, VertexId)>, String> {
    let mut edges = Vec::new();
    let mut rest = body.trim();
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix(',') {
            rest = r.trim_start();
            continue;
        }
        let (pair, after) = take_paren_block(rest)?;
        let parts: Vec<&str> = pair.split(',').map(|s| s.trim()).collect();
        if parts.len() != 2 {
            return Err(format!("Edge pair must be (u,v): got ({pair})"));
        }
        let u: VertexId = parts[0].parse().map_err(|e| format!("bad vertex: {e}"))?;
        let v: VertexId = parts[1].parse().map_err(|e| format!("bad vertex: {e}"))?;
        edges.push((u, v));
        rest = after.trim_start();
    }
    Ok(edges)
}

fn parse_face_list(body: &str) -> Result<Vec<Vec<VertexId>>, String> {
    let mut faces = Vec::new();
    let mut rest = body.trim();
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix(',') {
            rest = r.trim_start();
            continue;
        }
        let (face, after) = take_paren_block(rest)?;
        let verts: Result<Vec<VertexId>, String> = face
            .split(',')
            .map(|s| s.trim().parse::<VertexId>().map_err(|e| format!("bad vertex: {e}")))
            .collect();
        faces.push(verts?);
        rest = after.trim_start();
    }
    Ok(faces)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TDD-HAL-I.1 — Lattice round-trip.
    /// Declare a 4-vertex Lattice (single triangle face), serialize
    /// to its GQL re-emit form, parse back, assert structural
    /// equality.
    #[test]
    fn tdd_hal_i_1_lattice_round_trip() {
        // Tiny tetrahedron-ish: 4 vertices, 3 edges, 1 face.
        let lat = Lattice::new(
            "t1",
            4,
            vec![(0, 1), (1, 2), (2, 0)],
            vec![vec![0, 1, 2]],
            Some("S2".to_string()),
        );
        let gql = lat.to_gql();
        let parsed = Lattice::from_gql(&gql).expect("round-trip parse");
        assert_eq!(lat, parsed);
        // Round-trip the serialization, too: re-emit should be
        // byte-identical (this is the bit-identity contract for
        // declared lattices).
        assert_eq!(gql, parsed.to_gql());
    }

    /// TDD-HAL-I.7 — parser round-trip. Use the engine's GQL
    /// front-end to parse both the explicit `VERTICES … EDGES …`
    /// form and the `FROM TRUNCATED_ICOSAHEDRON` shorthand; assert
    /// each yields the correct Statement variant + a re-parse of
    /// the canonical re-emit form is bit-identical.
    #[test]
    fn tdd_hal_i_7_lattice_parse() {
        use crate::parser;

        // Explicit form.
        let src = "LATTICE bb \
                   VERTICES 4 \
                   EDGES ((0,1),(1,2),(2,0)) \
                   FACES ((0,1,2)) ;";
        let stmt = parser::parse(src).expect("parse explicit LATTICE");
        match &stmt {
            parser::Statement::Lattice { name, gql } => {
                assert_eq!(name, "bb");
                // Re-parse the canonical re-emit form via the
                // Lattice algebra's from_gql; that's the
                // round-trip receipt.
                let lat = Lattice::from_gql(gql).expect("re-parse Lattice");
                assert_eq!(lat.name, "bb");
                assert_eq!(lat.n_vertices, 4);
                assert_eq!(lat.n_edges(), 3);
                assert_eq!(lat.n_faces(), 1);
            }
            other => panic!("expected Statement::Lattice, got {other:?}"),
        }

        // Shorthand. Topology strings are single-quoted to match
        // the GIGI tokenizer's existing string-literal convention.
        let src2 = "LATTICE bb FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
        let stmt = parser::parse(src2).expect("parse shorthand LATTICE");
        match &stmt {
            parser::Statement::LatticeFromCanonical {
                name,
                canonical,
                topology,
            } => {
                assert_eq!(name, "bb");
                assert_eq!(canonical.to_ascii_uppercase(), "TRUNCATED_ICOSAHEDRON");
                assert_eq!(topology.as_deref(), Some("S2"));
            }
            other => panic!("expected LatticeFromCanonical, got {other:?}"),
        }
    }

    #[test]
    fn euler_chi_on_tetrahedron_face_count_check() {
        // 4 vertices, 6 edges, 4 faces → χ = 2 (S² topology).
        let lat = Lattice::new(
            "tet",
            4,
            vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)],
            vec![
                vec![0, 1, 2],
                vec![0, 1, 3],
                vec![0, 2, 3],
                vec![1, 2, 3],
            ],
            None,
        );
        assert_eq!(lat.euler_characteristic(), 2);
    }
}
