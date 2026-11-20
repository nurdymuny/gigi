//! LINK_FLOW parity probe (YM4, docs/YANG_MILLS_ENGINE_SPEC.md). Used by
//! e2e/probes/test_link_flow_parity.py. No query route.
//!
//! usage: link_flow_probe input.json output.json
//! input: {"order": 6, "method": "native" | "pg2", "steps": N, "t_end": 0.02,
//!         "metric_step": [[t, [g1,g2,g3]], ..], "metric_ref": [[t, [g1,g2,g3]], ..],
//!         "k0": {"n": 3n, "rows": "path.bin", "cols": "path.bin", "vals": "path.bin"},
//!         "links": [[q0..q3], ..], "edge_points": [[[4],[4]], ..],
//!         "cells": [{"vertices", "edges", "reverse", "weight"}, ..]}
//! The final links are written beside the output as raw f64 (output.final.bin).
use gigi::gauge::curvature_recovery::Cell;
use gigi::gauge::ricci_link_flow::{link_flow_with, MetricSamples, StepEvaluation};
use gigi::gauge::ricci_pg_force::ForceMethod;
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
struct K0 {
    rows: String,
    cols: String,
    vals: String,
}

#[derive(Deserialize)]
struct Input {
    order: usize,
    method: String,
    steps: usize,
    t_end: f64,
    metric_step: Vec<(f64, [f64; 3])>,
    metric_ref: Vec<(f64, [f64; 3])>,
    k0: K0,
    /// YM6 removal only: "endpoint" evaluates the velocity at the step's end.
    #[serde(default)]
    evaluation: Option<String>,
    links: Vec<[f64; 4]>,
    edge_points: Vec<[[f64; 4]; 2]>,
    cells: Vec<InCell>,
}

fn read_u32(p: &str) -> std::io::Result<Vec<usize>> {
    Ok(std::fs::read(p)?.chunks_exact(4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize).collect())
}
fn read_f64(p: &str) -> std::io::Result<Vec<f64>> {
    Ok(std::fs::read(p)?.chunks_exact(8).map(|b| f64::from_le_bytes(b.try_into().unwrap())).collect())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: link_flow_probe input.json output.json".into());
    }
    let input: Input = serde_json::from_reader(BufReader::new(File::open(&args[1])?))?;
    let method = match input.method.as_str() {
        "native" => ForceMethod::Native,
        "pg2" => ForceMethod::Pg2 { rings: 2, register: true },
        m => return Err(format!("unknown method '{m}' (native | pg2)").into()),
    };
    let weights: Vec<f64> = input.cells.iter().map(|c| c.weight).collect();
    let cells: Vec<Cell> = input
        .cells
        .into_iter()
        .map(|c| Cell { vertices: c.vertices, edges: c.edges, reverse: c.reverse })
        .collect();
    let (kr, kc, kv) = (read_u32(&input.k0.rows)?, read_u32(&input.k0.cols)?, read_f64(&input.k0.vals)?);
    let t = std::time::Instant::now();
    let evaluation = match input.evaluation.as_deref() {
        None | Some("midpoint") => StepEvaluation::Midpoint,
        Some("endpoint") => StepEvaluation::Endpoint,
        Some(e) => return Err(format!("unknown evaluation '{e}'").into()),
    };
    let out = link_flow_with(
        &cells, &weights, &input.links, &input.edge_points, input.order, method, input.steps, input.t_end,
        &MetricSamples(input.metric_step), &MetricSamples(input.metric_ref), (&kr, &kc, &kv), evaluation,
    )?;
    let secs = t.elapsed().as_secs_f64();
    let base = std::path::Path::new(&args[2]);
    std::fs::write(
        base.with_extension("final.bin"),
        out.final_links.iter().flatten().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>(),
    )?;
    let rows: Vec<_> = out
        .rows
        .iter()
        .map(|r| serde_json::json!({"t": r.t, "E": r.e, "D": r.d, "R": r.r, "newton_iterations": r.newton_iterations}))
        .collect();
    serde_json::to_writer(
        File::create(&args[2])?,
        &serde_json::json!({"schema": "link_flow_probe_v1", "method": input.method, "status": out.status,
                            "rows": rows, "newton_iterations": out.newton_iterations,
                            "newton_worst_final_increment": out.worst_final_increment,
                            "force_evaluations": out.force_evaluations, "variational": out.variational,
                            "seconds": secs}),
    )?;
    Ok(())
}
