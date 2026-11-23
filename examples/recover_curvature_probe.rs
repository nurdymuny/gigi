//! RECOVER_CURVATURE parity probe: run the engine kernel on a mesh given as
//! JSON and write every cell's recovery as JSON. Used by
//! e2e/probes/test_recover_curvature_parity.py to compare against the paper's
//! Python fit. No query route; the engine verb lives in gigi-stream.
//!
//! usage: recover_curvature_probe input.json output.json
//! input:  {"rings": 2, "a": [..], "cells": [{"vertices": [[q0,q1,q2,q3] x4],
//!          "edges": [6 ids], "reverse": [6 bools]}, ..]}
use gigi::gauge::curvature_recovery::{recover_curvature, Cell};
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
    a: Vec<f64>,
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
    let out = recover_curvature(&cells, &input.a, input.rings)?;
    let secs = t.elapsed().as_secs_f64();
    let rows: Vec<_> = out
        .iter()
        .map(|r| {
            serde_json::json!({
                "coefficients": r.coefficients, "amplification": r.amplification,
                "condition": r.condition, "rank": r.rank, "faces": r.faces, "cells": r.cells,
                "residual": r.residual, "own_residual": r.own_residual,
            })
        })
        .collect();
    serde_json::to_writer(
        File::create(&args[2])?,
        &serde_json::json!({"schema": "recover_curvature_probe_v1", "rings": input.rings,
                            "seconds": secs, "cells": rows}),
    )?;
    Ok(())
}
