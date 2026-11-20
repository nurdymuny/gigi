//! RECOVER_CURVATURE parity probe: run the engine kernel on a mesh given as
//! JSON and write every cell's recovery as JSON. Used by
//! e2e/probes/test_recover_curvature_parity.py (U(1)) and
//! e2e/probes/test_recover_curvature_su2_parity.py (SU(2)) to compare against
//! the research scripts. No query route; the engine verb lives in gigi-stream.
//!
//! usage: recover_curvature_probe input.json output.json
//! input (U(1)):  {"rings": 2, "a": [..], "cells": [{"vertices": [[q0,q1,q2,q3] x4],
//!                 "edges": [6 ids], "reverse": [6 bools]}, ..]}
//! input (SU(2)): {"group": "su2", "rings": 2, "links": [[q0,q1,q2,q3], ..], "cells": [..],
//!                 optional "register": bool, "abelianised": bool}
use gigi::gauge::curvature_recovery::{
    recover_curvature, recover_curvature_su2_with, Cell, Mechanism, Su2Mechanism,
};
use serde::Deserialize;
use std::{fs::File, io::BufReader};

#[derive(Deserialize)]
struct InCell {
    vertices: [[f64; 4]; 4],
    edges: [usize; 6],
    reverse: [bool; 6],
}

#[derive(Deserialize)]
struct Input {
    rings: usize,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    a: Vec<f64>,
    #[serde(default)]
    links: Vec<[f64; 4]>,
    #[serde(default)]
    register: Option<bool>,
    #[serde(default)]
    abelianised: Option<bool>,
    cells: Vec<InCell>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: recover_curvature_probe input.json output.json".into());
    }
    let input: Input = serde_json::from_reader(BufReader::new(File::open(&args[1])?))?;
    let cells: Vec<Cell> = input
        .cells
        .into_iter()
        .map(|c| Cell { vertices: c.vertices, edges: c.edges, reverse: c.reverse })
        .collect();
    let t = std::time::Instant::now();
    let (schema, rows): (&str, Vec<serde_json::Value>) = match input.group.as_deref() {
        None | Some("u1") => {
            let out = recover_curvature(&cells, &input.a, input.rings)?;
            ("recover_curvature_probe_v1", out
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "coefficients": r.coefficients, "amplification": r.amplification,
                        "condition": r.condition, "rank": r.rank, "faces": r.faces, "cells": r.cells,
                        "residual": r.residual, "own_residual": r.own_residual,
                    })
                })
                .collect())
        }
        Some("su2") => {
            let su = Su2Mechanism {
                register: input.register.unwrap_or(true),
                abelianised: input.abelianised.unwrap_or(false),
            };
            let out = recover_curvature_su2_with(&cells, &input.links, input.rings, Mechanism::FULL, su)?;
            ("recover_curvature_su2_probe_v1", out
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "coefficients": r.coefficients, "amplification": r.amplification,
                        "condition": r.condition, "rank": r.rank, "faces": r.faces, "cells": r.cells,
                        "own_residual": r.own_residual, "frame_vertex": r.frame_vertex,
                    })
                })
                .collect())
        }
        Some(g) => return Err(format!("unknown group '{g}' (u1 | su2)").into()),
    };
    let secs = t.elapsed().as_secs_f64();
    serde_json::to_writer(
        File::create(&args[2])?,
        &serde_json::json!({"schema": schema, "rings": input.rings, "seconds": secs, "cells": rows}),
    )?;
    Ok(())
}
