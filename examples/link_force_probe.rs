//! LINK_FORCE parity probe (YM2, docs/YANG_MILLS_ENGINE_SPEC.md): run the engine force
//! on a mesh and links given as JSON and write the gradient as JSON. Used by
//! e2e/probes/test_link_force_parity.py. No query route.
//!
//! usage: link_force_probe input.json output.json
//! input: {"metric": [g1,g2,g3], "order": 6, "method": "native" | "pg2" | "pg2_noregister",
//!         "rings": 2, "links": [[q0,q1,q2,q3], ..],
//!         "cells": [{"vertices": [[..] x4], "edges": [6], "reverse": [6], "weight": w}, ..]}
use gigi::gauge::curvature_recovery::Cell;
use gigi::gauge::ricci_pg_force::{link_force, link_force_jacobian, ForceMethod};
use serde::Deserialize;
use std::{fs::File, io::BufReader};

#[derive(Deserialize)]
struct InCell {
    vertices: [[f64; 4]; 4],
    edges: [usize; 6],
    reverse: [bool; 6],
    weight: f64,
}

#[derive(Deserialize)]
struct Input {
    metric: Option<[f64; 3]>,
    order: usize,
    method: String,
    #[serde(default = "two")]
    rings: usize,
    links: Vec<[f64; 4]>,
    #[serde(default)]
    jacobian_eps: Option<f64>,
    cells: Vec<InCell>,
}

fn two() -> usize {
    2
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: link_force_probe input.json output.json".into());
    }
    let input: Input = serde_json::from_reader(BufReader::new(File::open(&args[1])?))?;
    let metric = input.metric.ok_or("LINK_FORCE: a metric is required; none is ever substituted")?;
    let method = match input.method.as_str() {
        "native" => ForceMethod::Native,
        "pg2" => ForceMethod::Pg2 { rings: input.rings, register: true },
        "pg2_noregister" => ForceMethod::Pg2 { rings: input.rings, register: false },
        m => return Err(format!("unknown method '{m}' (native | pg2 | pg2_noregister)").into()),
    };
    let weights: Vec<f64> = input.cells.iter().map(|c| c.weight).collect();
    let cells: Vec<Cell> = input
        .cells
        .into_iter()
        .map(|c| Cell { vertices: c.vertices, edges: c.edges, reverse: c.reverse })
        .collect();
    if let Some(eps) = input.jacobian_eps {
        // YM3: K by central differences; triplets written as raw little-endian
        // files beside the JSON (rows u32, cols u32, vals f64).
        let t = std::time::Instant::now();
        let j = link_force_jacobian(&cells, &weights, &input.links, metric, input.order, method, eps)?;
        let secs = t.elapsed().as_secs_f64();
        let base = std::path::Path::new(&args[2]);
        let write = |ext: &str, bytes: Vec<u8>| std::fs::write(base.with_extension(ext), bytes);
        write("rows.bin", j.rows.iter().flat_map(|x| x.to_le_bytes()).collect())?;
        write("cols.bin", j.cols.iter().flat_map(|x| x.to_le_bytes()).collect())?;
        write("vals.bin", j.vals.iter().flat_map(|x| x.to_le_bytes()).collect())?;
        serde_json::to_writer(
            File::create(&args[2])?,
            &serde_json::json!({"schema": "link_force_jacobian_v1", "method": input.method, "eps": eps,
                                "n": j.n, "nnz": j.vals.len(), "seconds": secs}),
        )?;
        return Ok(());
    }
    let t = std::time::Instant::now();
    let out = link_force(&cells, &weights, &input.links, metric, input.order, method)?;
    let secs = t.elapsed().as_secs_f64();
    serde_json::to_writer(
        File::create(&args[2])?,
        &serde_json::json!({"schema": "link_force_probe_v1", "method": input.method,
                            "variational": out.variational, "gradient": out.gradient,
                            "energy": out.energy, "volume": out.volume, "seconds": secs}),
    )?;
    Ok(())
}
