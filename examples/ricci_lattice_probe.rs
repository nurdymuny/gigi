//! RF2B diagnostic of actual native kernels. Not a new flow or convergence claim.
use gigi::gauge::ricci_whitney::tetra_mass;
use gigi::gauge::{
    group_element::GroupElement,
    holonomy::walk_loop,
    ricci_link::levi_civita_link,
    staple::{EdgeFaceIncidence, FaceEdgesCache},
    su2_gauge_field::{GaugeFieldInit, SU2GaugeField},
    wilson_force::wilson_force_per_edge,
};
use gigi::lattice::{EdgeOrientation, Lattice};
use serde::Deserialize;
use std::{collections::HashMap, fs::File, io::BufReader};

#[derive(Deserialize)]
struct Input {
    vertices: Vec<[f64; 4]>,
    edges: Vec<[usize; 2]>,
    faces: Vec<[usize; 3]>,
    metric: [f64; 3],
    tetrahedra: Vec<[usize; 4]>,
}
fn quaternion(g: GroupElement) -> [f64; 4] {
    match g {
        GroupElement::SU2 { q0, q1, q2, q3 } => [q0, q1, q2, q3],
        _ => unreachable!(),
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: ricci_lattice_probe input.json output.json".into());
    }
    let input: Input = serde_json::from_reader(BufReader::new(File::open(&args[1])?))?;
    if input.edges.is_empty()
        || input.edges.len() > 2_000_000
        || input.faces.is_empty()
        || input.faces.len() > 3_000_000
    {
        return Err("nonempty mesh within probe resource bounds required".into());
    }
    let lat = Lattice::new(
        "rf2b_probe",
        input.vertices.len(),
        input.edges.iter().map(|e| (e[0], e[1])).collect(),
        input.faces.iter().map(|f| f.to_vec()).collect(),
        None,
    );
    let mut indices = HashMap::new();
    for (i, &[a, b]) in input.edges.iter().enumerate() {
        if a >= input.vertices.len()
            || b >= input.vertices.len()
            || a >= b
            || indices.insert((a, b), i).is_some()
        {
            return Err("edge endpoints must be valid, canonical and unique".into());
        }
    }
    let mut incidence: EdgeFaceIncidence = vec![vec![]; input.edges.len()];
    let mut cache: FaceEdgesCache = Vec::with_capacity(input.faces.len());
    for (i, face) in input.faces.iter().enumerate() {
        if !(face[0] < face[1] && face[1] < face[2]) {
            return Err("faces must have canonical distinct vertices".into());
        }
        let mut cycle = Vec::new();
        for j in 0..3 {
            let (a, b) = (face[j], face[(j + 1) % 3]);
            let edge = *indices
                .get(&(a.min(b), a.max(b)))
                .ok_or("face edge absent")?;
            cycle.push((
                edge,
                if a < b {
                    EdgeOrientation::Forward
                } else {
                    EdgeOrientation::Reverse
                },
            ));
            incidence[edge].push((i, j));
        }
        cache.push(cycle);
    }
    let mut field = SU2GaugeField::new("rf2b_U".into(), &lat, GaugeFieldInit::Identity, None)?;
    let mut links = Vec::with_capacity(input.edges.len());
    for (i, &[a, b]) in input.edges.iter().enumerate() {
        let link = quaternion(levi_civita_link(
            input.metric,
            input.vertices[a],
            input.vertices[b],
        )?);
        field.buffer.data[4 * i..4 * i + 4].copy_from_slice(&link);
        links.push(link);
    }
    let holonomies: Vec<_> = cache
        .iter()
        .map(|face| quaternion(walk_loop(&lat, face, &field)))
        .collect();
    let beta = 2.5;
    let force = wilson_force_per_edge(&field, &lat, &incidence, &cache, beta)?;
    let potential = beta / 4. * holonomies.iter().map(|q| 1. - q[0]).sum::<f64>();
    if input.tetrahedra.is_empty() {
        return Err("tetrahedral cells are required, not just a topology label".into());
    }
    let mut mass_samples = Vec::new();
    for index in [
        0,
        input.tetrahedra.len() / 3,
        2 * input.tetrahedra.len() / 3,
        input.tetrahedra.len() - 1,
    ] {
        let cell = input.tetrahedra[index];
        if cell.iter().any(|i| *i >= input.vertices.len()) {
            return Err("invalid cell vertex".into());
        }
        for order in [2, 4, 6] {
            let mass = tetra_mass(cell.map(|i| input.vertices[i]), input.metric, order)?;
            let edge: Vec<Vec<_>> = (0..6)
                .map(|i| (0..6).map(|j| mass.edge_mass[(i, j)]).collect())
                .collect();
            let face: Vec<Vec<_>> = (0..4)
                .map(|i| (0..4).map(|j| mass.face_mass[(i, j)]).collect())
                .collect();
            mass_samples.push(serde_json::json!({"cell":index,"order":order,"volume":mass.volume,"edge_mass":edge,"face_mass":face}));
        }
    }
    let result = serde_json::json!({"schema":"rf2b_native_probe_v1","metric":input.metric,"beta":beta,
        "vertices":input.vertices.len(),"edges":input.edges.len(),"faces":input.faces.len(),
        "links":links,"face_holonomies":holonomies,"legacy_forces":force,"legacy_potential":potential,"cell_mass_samples":mass_samples,
        "metric_mass_present":false,"lattice_operator_convergence_tested":false,
        "scope":"native_transport_and_legacy_force_only"});
    serde_json::to_writer(File::create(&args[2])?, &result)?;
    Ok(())
}
