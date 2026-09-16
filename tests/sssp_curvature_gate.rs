//! SUPERSEDED 2026-09-16. The verdict this file reports (degree 2 wins, 3 ties,
//! curvature 0) is withdrawn as a non-measurement: the instrument is constant
//! at two samples and is a range-normalised shape statistic at any sample size.
//! See tests/scalar_curvature_is_a_shape_statistic.rs for what it returns, and
//! the seven-step redo with the coherent-noise control in the SSSP paper folder
//! (sssp_ablation/src/bin/gigi_gate.rs). Kept as the record of the first run.
//!
//! The curvature gate experiment, run INSIDE GIGI.
//!
//! An earlier attempt computed Forman-Ricci in a standalone crate and called
//! it a GIGI result. It was not one: reimplementing curvature myself proves
//! nothing about the engine. Every curvature number in this file comes out of
//! `gigi::curvature::scalar_curvature` applied to a real `BundleStore`.
//!
//! THE TWO NOTIONS OF FLAT
//!
//! The SSSP paper gates corridor scouting on DEGREE: scout from u to v when
//! both have few neighbours. Its stated justification is that a low-degree
//! vertex admits predictable traversal.
//!
//! GIGI measures something different. `scalar_curvature` is
//! `mean_f(variance/range^2)` over a bundle's fields, so on the sub-bundle of
//! edges incident to u it is the normalised DISPERSION OF INCIDENT EDGE
//! WEIGHTS. A vertex whose edges all cost about the same sits in a region
//! where the metric barely varies -- flat in the metric sense. A vertex with
//! one cheap edge and one expensive one is a place where the metric jumps,
//! whatever its degree.
//!
//! Those are different claims about the same word, and the paper only tests
//! one of them. Both gates are rate-matched here -- each fires on the same
//! NUMBER of edges -- so the comparison isolates WHICH edges get scouted.
//!
//! Correctness is unaffected by the choice: scouting only lowers tentative
//! distances and never finalises a vertex. Verified against Dijkstra per
//! configuration regardless.

use std::collections::BinaryHeap;

use gigi::bundle::{BundleStore, QueryCondition};
use gigi::types::{BundleSchema, FieldDef, Record, Value};

// ── graph, deterministic, the paper's shapes ───────────────────────────────

struct Graph {
    n: usize,
    adj: Vec<Vec<(usize, f64)>>,
    degree: Vec<usize>,
}

impl Graph {
    fn new(n: usize) -> Self {
        Graph { n, adj: vec![Vec::new(); n], degree: vec![0; n] }
    }
    fn add(&mut self, u: usize, v: usize, w: f64) {
        self.adj[u].push((v, w));
        self.degree[u] += 1;
    }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn weight(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (self.next() % 1000) as f64 / 1000.0 * (hi - lo)
    }
}

fn sparse(n: usize, avg: usize, seed: u64) -> Graph {
    let mut r = Rng(seed | 1);
    let mut g = Graph::new(n);
    for u in 0..n {
        let d = 1 + r.below((avg * 2).min(n - 1));
        for _ in 0..d {
            let v = r.below(n);
            if v != u {
                g.add(u, v, r.weight(1.0, 100.0));
            }
        }
    }
    g
}

/// Grid, but with weights drawn from TWO regimes so metric flatness and
/// degree flatness genuinely disagree: a 4-connected lattice has uniform
/// degree, so degree cannot discriminate anything on it at all.
fn banded_grid(side: usize, seed: u64) -> Graph {
    let mut r = Rng(seed | 1);
    let n = side * side;
    let mut g = Graph::new(n);
    for i in 0..side {
        for j in 0..side {
            let u = i * side + j;
            // half the lattice is a cheap uniform corridor, half is rough
            let rough = j > side / 2;
            for (di, dj) in [(0i64, 1i64), (1, 0), (0, -1), (-1, 0)] {
                let (ni, nj) = (i as i64 + di, j as i64 + dj);
                if ni >= 0 && nj >= 0 && (ni as usize) < side && (nj as usize) < side {
                    let v = ni as usize * side + nj as usize;
                    let w = if rough { r.weight(1.0, 100.0) } else { r.weight(9.0, 11.0) };
                    g.add(u, v, w);
                }
            }
        }
    }
    g
}

fn road_like(n: usize, seed: u64) -> Graph {
    let mut r = Rng(seed | 1);
    let pts: Vec<(f64, f64)> = (0..n)
        .map(|_| (r.weight(0.0, 1000.0), r.weight(0.0, 1000.0)))
        .collect();
    let mut g = Graph::new(n);
    for u in 0..n {
        let mut d: Vec<(f64, usize)> = (0..n)
            .filter(|&v| v != u)
            .map(|v| {
                let dx = pts[u].0 - pts[v].0;
                let dy = pts[u].1 - pts[v].1;
                ((dx * dx + dy * dy).sqrt(), v)
            })
            .collect();
        d.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for &(w, v) in d.iter().take(6) {
            g.add(u, v, w);
        }
    }
    g
}

// ── GIGI: store the graph, ask the engine for curvature ────────────────────

fn edge_schema() -> BundleSchema {
    BundleSchema::new("graph")
        .base(FieldDef::numeric("vertex_a"))
        .base(FieldDef::numeric("vertex_b"))
        .fiber(FieldDef::numeric("weight"))
}

fn store_graph(g: &Graph) -> BundleStore {
    let mut s = BundleStore::new(edge_schema());
    for u in 0..g.n {
        for &(v, w) in &g.adj[u] {
            let mut r = Record::new();
            r.insert("vertex_a".into(), Value::Integer(u as i64));
            r.insert("vertex_b".into(), Value::Integer(v as i64));
            r.insert("weight".into(), Value::Float(w));
            s.insert(&r);
        }
    }
    s
}

/// Per-vertex curvature, computed BY GIGI: filter the edge bundle to the
/// edges leaving u, then hand that sub-bundle to `scalar_curvature`.
fn gigi_curvature_per_vertex(store: &BundleStore, n: usize) -> Vec<f64> {
    (0..n)
        .map(|u| {
            let rows = store.filtered_query(
                &[QueryCondition::Eq("vertex_a".into(), Value::Integer(u as i64))],
                None,
                false,
                None,
                None,
            );
            if rows.len() < 2 {
                // GIGI needs two records to have a variance at all. One edge
                // is not a neighbourhood; treat it as maximally flat rather
                // than inventing a number.
                return 0.0;
            }
            let mut sub = BundleStore::new(edge_schema());
            for r in &rows {
                sub.insert(r);
            }
            gigi::curvature::scalar_curvature(&sub)
        })
        .collect()
}

// ── the corridor algorithm, gate swapped ───────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
struct St {
    d: u64,
    node: usize,
}
impl Ord for St {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        o.d.cmp(&self.d)
    }
}
impl PartialOrd for St {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
#[inline]
fn key(d: f64) -> u64 {
    (d * 1e6) as u64
}

#[derive(Clone, Copy)]
enum Gate<'a> {
    None,
    Degree(usize),
    Curvature(&'a [f64], f64),
}

fn corridor(g: &Graph, src: usize, gate: Gate) -> (Vec<f64>, u64, u64) {
    let n = g.n;
    let mut dist = vec![f64::INFINITY; n];
    let mut vis = vec![false; n];
    let (mut pushes, mut scouts) = (0u64, 0u64);
    dist[src] = 0.0;
    let mut pq = BinaryHeap::new();
    pq.push(St { d: 0, node: src });
    pushes += 1;

    while let Some(St { node: u, .. }) = pq.pop() {
        if vis[u] {
            continue;
        }
        vis[u] = true;
        let du = dist[u];
        for &(v, w) in &g.adj[u] {
            let nd = du + w;
            if nd < dist[v] {
                dist[v] = nd;
                let fire = match gate {
                    Gate::None => false,
                    Gate::Degree(t) => g.degree[u] <= t && g.degree[v] <= t,
                    Gate::Curvature(k, t) => k[u] <= t && k[v] <= t,
                };
                if fire && !vis[v] {
                    scouts += 1;
                    for &(v2, w2) in &g.adj[v] {
                        let nd2 = nd + w2;
                        if nd2 < dist[v2] {
                            dist[v2] = nd2;
                            if !vis[v2] {
                                pq.push(St { d: key(nd2), node: v2 });
                                pushes += 1;
                            }
                        }
                    }
                }
                pq.push(St { d: key(nd), node: v });
                pushes += 1;
            }
        }
    }
    (dist, pushes, scouts)
}

/// Threshold on `vals` admitting approximately `rate` of vertices (low = flat).
fn rate_matched(vals: &[f64], rate: f64) -> f64 {
    let mut s: Vec<f64> = vals.iter().copied().filter(|v| v.is_finite()).collect();
    if s.is_empty() {
        return f64::INFINITY;
    }
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let k = ((rate * s.len() as f64).round() as usize).clamp(0, s.len() - 1);
    s[k]
}

fn same(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            (x.is_infinite() && y.is_infinite()) || (x - y).abs() < 1e-6
        })
}

#[test]
fn gigi_curvature_gate_vs_degree_gate() {
    const T: usize = 4;
    let cases: Vec<(&str, Graph)> = vec![
        ("sparse/1000", sparse(1000, 4, 42)),
        ("sparse/2000", sparse(2000, 4, 42)),
        ("banded_grid/1024", banded_grid(32, 42)),
        ("banded_grid/2500", banded_grid(50, 42)),
        ("road_like/1000", road_like(1000, 42)),
    ];

    println!();
    println!("CURVATURE GATE EXPERIMENT — curvature computed by gigi::curvature::scalar_curvature");
    println!("both gates rate-matched; pushes = priority-queue insertions\n");
    println!(
        "{:<18} {:>9} {:>9} {:>9} {:>8} {:>8}",
        "graph", "ungated", "DEGREE", "GIGI-K", "scoutsD", "scoutsK"
    );
    println!("{}", "-".repeat(66));

    let mut k_wins = 0;
    let mut d_wins = 0;
    let mut ties = 0;

    for (name, g) in &cases {
        let store = store_graph(g);
        let kv = gigi_curvature_per_vertex(&store, g.n);

        // fire rate of the published gate on this graph
        let flat = g.degree.iter().filter(|&&d| d <= T).count() as f64 / g.n as f64;
        let kt = rate_matched(&kv, flat);

        let (d0, p0, _) = corridor(g, 0, Gate::None);
        let (d1, p1, s1) = corridor(g, 0, Gate::Degree(T));
        let (d2, p2, s2) = corridor(g, 0, Gate::Curvature(&kv, kt));

        assert!(same(&d0, &d1), "{name}: degree gate must preserve distances");
        assert!(same(&d0, &d2), "{name}: curvature gate must preserve distances");

        println!(
            "{:<18} {:>9} {:>9} {:>9} {:>8} {:>8}",
            name, p0, p1, p2, s1, s2
        );

        if p2 < p1 {
            k_wins += 1
        } else if p1 < p2 {
            d_wins += 1
        } else {
            ties += 1
        }
    }

    println!("\n{}", "=".repeat(66));
    println!("GIGI curvature fewer pushes : {k_wins}");
    println!("degree fewer pushes         : {d_wins}");
    println!("tie                         : {ties}");
    println!();
    println!("Both gates are rate-matched, so this is which edges, not how many.");
}
