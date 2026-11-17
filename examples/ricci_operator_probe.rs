//! Experimental RF2B assembly diagnostic. No public query route or flow claim.
use gigi::gauge::{group_element::GroupElement, ricci_link::levi_civita_link};
use gigi::gauge::{
    ricci_simplicial::{adjoint, cell_action, cell_tangent_mass},
    ricci_whitney::tetra_mass,
};
use nalgebra::{Matrix3, Vector3};
use serde::Deserialize;
use std::{collections::BTreeMap, fs::File, io::BufReader};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Cell {
    vertices: [[f64; 4]; 4],
    edges: [usize; 6],
    reverse: [bool; 6],
    weight: f64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    metric: [f64; 3],
    order: usize,
    edge_points: Vec<[[f64; 4]; 2]>,
    cells: Vec<Cell>,
}
fn parts(q: GroupElement) -> [f64; 4] {
    match q {
        GroupElement::SU2 { q0, q1, q2, q3 } => [q0, q1, q2, q3],
        _ => unreachable!(),
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: ricci_operator_probe input.json output.json".into());
    }
    let input: Input = serde_json::from_reader(BufReader::new(File::open(&args[1])?))?;
    if input.edge_points.is_empty()
        || input.edge_points.len() > 2_000_000
        || input.cells.is_empty()
        || input.cells.len() > 2_000_000
    {
        return Err(
            "nonempty edge and cell lists within diagnostic resource bounds required".into(),
        );
    }
    let links: Vec<_> = input
        .edge_points
        .iter()
        .map(|p| levi_civita_link(input.metric, p[0], p[1]).map(parts))
        .collect::<Result<_, _>>()?;
    let mut gradient = vec![Vector3::<f64>::zeros(); links.len()];
    let mut blocks = BTreeMap::<(usize, usize), Matrix3<f64>>::new();
    let (mut energy, mut volume) = (0., 0.);
    for cell in &input.cells {
        if !cell.weight.is_finite()
            || cell.weight <= 0.
            || cell.weight > 1.
            || cell.edges.iter().any(|i| *i >= links.len())
        {
            return Err("invalid cell weight or edge index".into());
        }
        let local = std::array::from_fn(|e| {
            let mut q = links[cell.edges[e]];
            if cell.reverse[e] {
                for x in &mut q[1..] {
                    *x = -*x;
                }
            }
            q
        });
        for (e, (a, b)) in [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]
            .into_iter()
            .enumerate()
        {
            let expected = parts(levi_civita_link(
                input.metric,
                cell.vertices[a],
                cell.vertices[b],
            )?);
            if (0..4).any(|j| (local[e][j] - expected[j]).abs() > 2e-12) {
                return Err("orbit identification disagrees with actual local transport".into());
            }
        }
        let mass = tetra_mass(cell.vertices, input.metric, input.order)?;
        let action = cell_action(local, mass.face_mass)?;
        let tangent = cell_tangent_mass(local, mass.edge_mass)?;
        let lift: [Matrix3<f64>; 6] = std::array::from_fn(|e| {
            if cell.reverse[e] {
                -adjoint(links[cell.edges[e]])
            } else {
                Matrix3::identity()
            }
        });
        energy += cell.weight * action.energy;
        volume += cell.weight * mass.volume;
        for e in 0..6 {
            gradient[cell.edges[e]] +=
                cell.weight * lift[e].transpose() * action.gradient.fixed_rows::<3>(3 * e);
            for f in 0..6 {
                *blocks
                    .entry((cell.edges[e], cell.edges[f]))
                    .or_insert_with(Matrix3::zeros) += cell.weight
                    * lift[e].transpose()
                    * tangent.fixed_view::<3, 3>(3 * e, 3 * f)
                    * lift[f];
            }
        }
    }
    let gradient: Vec<[f64; 3]> = gradient.into_iter().map(|v| [v[0], v[1], v[2]]).collect();
    let blocks: Vec<_> = blocks
        .into_iter()
        .map(|((row, col), m)| {
            let value: [[f64; 3]; 3] = std::array::from_fn(|i| std::array::from_fn(|j| m[(i, j)]));
            serde_json::json!({"row":row,"col":col,"value":value})
        })
        .collect();
    let out = serde_json::json!({"schema":"rf2b_operator_assembly_v1","links":links,"energy":energy,"volume":volume,
        "gradient":gradient,"blocks":blocks,"cells":input.cells.len(),"order":input.order,
        "scope":"native_cell_assembly_external_diagnostic_solver"});
    serde_json::to_writer(File::create(&args[2])?, &out)?;
    Ok(())
}
