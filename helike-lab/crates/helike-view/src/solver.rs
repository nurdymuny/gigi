//! The solver side of the viewer. It runs in its own thread, takes requests, and streams `helike_stream::Frame`s whose
//! `data` is one of the payloads below (as JSON, so the same frames can later arrive over a socket). The drawing code
//! never calls helike-core: it only reads frames.

use helike_core::helike::{self, Member, SectorData, Spectrum};
use helike_core::linalg;
use helike_core::rep::Rep;
use helike_core::sector::Sector;
use helike_stream::{Frame, Status};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender, TryRecvError};

use crate::controls;

#[derive(Clone, Debug)]
pub enum Request {
    Ladder { m: Member, jmax: usize },
    Sphere { t: f64, j: usize, fibres: usize, samples: usize },
    EdgeMap { y: f64, t: (f64, f64, usize), x: (f64, f64, usize), jmax: usize, which: Spectrum },
    Probe { m: Member, jmax: usize },
    /// Read the registered control runs under this repository root and keep rescanning when their files change.
    LoadControls(std::path::PathBuf),
}

impl Request {
    fn kind(&self) -> &'static str {
        match self {
            Request::Ladder { .. } => "ladder",
            Request::Sphere { .. } => "sphere",
            Request::EdgeMap { .. } => "edge",
            Request::Probe { .. } => "probe",
            Request::LoadControls(_) => "controls",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SectorRow {
    pub j: usize,
    pub twist: f64,
    /// lowest eigenvalue of the twist's 1x1 block (H + GG^*) minus the closed form
    pub twist_check: f64,
    /// lowest gauge-orthogonal eigenvalue other than the twist's two copies
    pub competitor_slice: f64,
    /// lowest eigenvalue of H + GG^* in a non-twist charge block
    pub competitor_a: f64,
    pub competitor_a_charge: f64,
    pub floor_sufficient: bool,
    pub rungs: Vec<helike::Rung>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LadderData {
    pub m: Member,
    pub fibre_field: f64,
    pub cross_field: f64,
    pub flux_closed: Vec<f64>,
    pub flux_computed: Vec<f64>,
    pub flux_check: f64,
    pub x_flux: f64,
    pub sectors: Vec<SectorRow>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SphereData {
    pub t: f64,
    pub j: usize,
    /// Hopf fibres (long circles) after stereographic projection: polylines in R^3, with the base point's polar angle.
    pub fibres: Vec<(f64, Vec<[f32; 3]>)>,
    /// Field samples: position, the direction of Re(f (e_1 - i e_2)) pushed forward, |f|^2, arg f.
    pub samples: Vec<([f32; 3], [f32; 3], f32, f32)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EdgeMapData {
    pub y: f64,
    pub which: Spectrum,
    pub jmax: usize,
    pub ts: Vec<f64>,
    pub xs: Vec<f64>,
    /// phi on the grid, row per t (NaN = not yet computed)
    pub phi: Vec<Vec<f64>>,
    /// the ladder's sufficient condition holds in every sector j <= jmax (a proof of the gauge-orthogonal floor there)
    pub ladder_proved: Vec<Vec<bool>>,
    pub x_flux: Vec<f64>,
    pub rows_done: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProbeData {
    pub m: Member,
    pub margins_a: Vec<(usize, f64, f64)>,
    pub margins_s: Vec<(usize, f64)>,
}

pub struct SolverHandle {
    pub tx: Sender<Request>,
    pub rx: Receiver<Frame>,
}

pub fn spawn(repaint: impl Fn() + Send + 'static) -> SolverHandle {
    let (tx_req, rx_req) = channel::<Request>();
    let (tx_frame, rx_frame) = channel::<Frame>();
    std::thread::spawn(move || {
        let mut seq = 0u64;
        let mut pending: Vec<Request> = vec![];
        let mut watch: Option<(controls::Paths, Vec<(std::path::PathBuf, u64, u128)>)> = None;
        // one frame per suite on scene "controls", plus the amendment history on "controls-history"
        let emit_controls = |paths: &controls::Paths, seq: &mut u64| {
            let (suites, history) = controls::load_all(paths);
            for s in suites {
                *seq += 1;
                let status = if s.interval { Status::IntervalEnclosure } else { Status::Computed };
                let _ = tx_frame.send(Frame { scene: "controls".into(), seq: *seq, status, params: serde_json::Value::Null, data: serde_json::to_value(&s).unwrap() });
            }
            *seq += 1;
            let _ = tx_frame.send(Frame { scene: "controls-history".into(), seq: *seq, status: Status::Computed, params: serde_json::Value::Null, data: serde_json::to_value(&history).unwrap() });
            repaint();
        };
        loop {
            if pending.is_empty() {
                if let Some((paths, fp)) = watch.as_mut() {
                    match rx_req.recv_timeout(std::time::Duration::from_millis(1000)) {
                        Ok(r) => pending.push(r),
                        Err(RecvTimeoutError::Timeout) => {
                            let now = controls::fingerprint(paths);
                            if now != *fp {
                                *fp = now;
                                emit_controls(paths, &mut seq);
                            }
                            continue;
                        }
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                } else {
                    match rx_req.recv() {
                        Ok(r) => pending.push(r),
                        Err(_) => return,
                    }
                }
            }
            loop {
                match rx_req.try_recv() {
                    Ok(r) => pending.push(r),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return,
                }
            }
            // latest request of each kind wins
            let mut latest: Vec<Request> = vec![];
            for r in pending.drain(..).rev() {
                if !latest.iter().any(|l| l.kind() == r.kind()) {
                    latest.push(r);
                }
            }
            latest.reverse();
            for req in latest {
                let send = |scene: &str, data: serde_json::Value, seq: &mut u64| {
                    *seq += 1;
                    let _ = tx_frame.send(Frame { scene: scene.into(), seq: *seq, status: Status::Computed, params: serde_json::Value::Null, data });
                    repaint();
                };
                match req {
                    Request::Ladder { m, jmax } => send("ladder", serde_json::to_value(ladder(&m, jmax)).unwrap(), &mut seq),
                    Request::Sphere { t, j, fibres, samples } => send("sphere", serde_json::to_value(sphere(t, j, fibres, samples)).unwrap(), &mut seq),
                    Request::Probe { m, jmax } => send("probe", serde_json::to_value(probe(&m, jmax)).unwrap(), &mut seq),
                    Request::LoadControls(root) => {
                        let paths = controls::Paths { root };
                        emit_controls(&paths, &mut seq);
                        let fp = controls::fingerprint(&paths);
                        watch = Some((paths, fp));
                    }
                    Request::EdgeMap { y, t, x, jmax, which } => {
                        let ts = helike_core::roots::linspace(t.0, t.1, t.2);
                        let xs = helike_core::roots::linspace(x.0, x.1, x.2);
                        let mut data = EdgeMapData {
                            y,
                            which,
                            jmax,
                            phi: vec![vec![f64::NAN; xs.len()]; ts.len()],
                            ladder_proved: ts.iter().map(|&t| xs.iter().map(|&x| (0..=jmax).all(|j| Member::new(x, y, t).floor_sufficient(j))).collect()).collect(),
                            x_flux: ts.iter().map(|&t| helike::x_flux(y, t)).collect(),
                            ts: ts.clone(),
                            xs: xs.clone(),
                            rows_done: 0,
                        };
                        // rows in an order that refines: every 4th, then every 2nd, then the rest
                        let mut order: Vec<usize> = (0..ts.len()).step_by(4).collect();
                        order.extend((2..ts.len()).step_by(4));
                        order.extend((1..ts.len()).step_by(2));
                        for (n, &i) in order.iter().enumerate() {
                            let ti = ts[i];
                            data.phi[i] = xs.par_iter().map(|&x| helike::phi(&Member::new(x, y, ti), which, jmax)).collect();
                            data.rows_done = n + 1;
                            if n % 2 == 1 || n + 1 == order.len() {
                                send("edge", serde_json::to_value(&data).unwrap(), &mut seq);
                            }
                            // abandon if a newer edge request is waiting
                            if let Ok(r) = rx_req.try_recv() {
                                let newer = matches!(r, Request::EdgeMap { .. });
                                pending.push(r);
                                if newer {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    });
    SolverHandle { tx: tx_req, rx: rx_frame }
}

fn ladder(m: &Member, jmax: usize) -> LadderData {
    // flux matrix Phi = H - curl^2 in the sector j = 0 (Phi = phi (x) I, so j = 0 gives phi itself)
    let (geo, field, rep) = (m.geometry(), m.field(), Rep::spin(0));
    let sec = Sector::new(&geo, &field, &rep);
    let hg = sec.hessian_and_gauge();
    let curl = sec.curl().unwrap();
    let flux_computed = linalg::eigvalsh(&(&hg.h - &curl * &curl));
    let flux_closed = m.flux_spectrum().to_vec();
    let flux_check = flux_computed.iter().zip(&flux_closed).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max);
    let sectors = (0..=jmax)
        .into_par_iter()
        .map(|j| {
            let sd = SectorData::new(m, j);
            let lam = m.twist(j);
            let blocks = sd.split.block_spectra(&sd.hg.h_plus_ggstar());
            let top = blocks.iter().next_back().map(|(_, v)| v[0]).unwrap_or(f64::NAN);
            let (ma, qa) = sd.margin_a(lam);
            SectorRow {
                j,
                twist: lam,
                twist_check: top - lam,
                competitor_slice: sd.margin_s(lam) + lam,
                competitor_a: ma + lam,
                competitor_a_charge: qa,
                floor_sufficient: m.floor_sufficient(j),
                rungs: helike::ladder(m, j),
            }
        })
        .collect();
    LadderData {
        m: *m,
        fibre_field: m.fibre_field(),
        cross_field: m.cross_field(),
        flux_closed,
        flux_computed,
        flux_check,
        x_flux: helike::x_flux(m.y, m.t),
        sectors,
    }
}

/// S^3 as unit quaternions q = a + b i + c j + d k, the long circle generated by right multiplication by i (Hopf
/// fibres), stereographic projection from -1: P(q) = (b, c, d)/(1 + a). The twist's amplitude in sector j is
/// f = z_1^j with z_1 = a + i b, so |f|^2 = cos^{2j}(eta/2) with eta the polar angle of the base point.
fn sphere(t: f64, j: usize, fibres: usize, samples: usize) -> SphereData {
    let proj = |q: [f64; 4]| -> [f64; 3] {
        let den = 1.0 + q[0];
        [q[1] / den, q[2] / den, q[3] / den]
    };
    // q * e^{i th}
    let rot = |q: [f64; 4], th: f64| -> [f64; 4] {
        let (c, s) = (th.cos(), th.sin());
        [q[0] * c - q[1] * s, q[0] * s + q[1] * c, q[2] * c + q[3] * s, q[3] * c - q[2] * s]
    };
    // base point (eta, phi) -> a point on its fibre
    let lift = |eta: f64, phi: f64| -> [f64; 4] {
        let (c, s) = ((eta / 2.0).cos(), (eta / 2.0).sin());
        [c, 0.0, s * phi.cos(), s * phi.sin()]
    };
    let mut fib = vec![];
    let rings = (fibres as f64).sqrt().ceil() as usize;
    for a in 0..rings {
        let eta = std::f64::consts::PI * (a as f64 + 0.5) / rings as f64;
        for b in 0..rings {
            let phi = 2.0 * std::f64::consts::PI * b as f64 / rings as f64;
            let q0 = lift(eta, phi);
            // a fibre is drawn only if it stays inside radius 3 (fibres through the projection pole go to infinity)
            let pts: Vec<[f64; 3]> = (0..=96).map(|k| proj(rot(q0, 2.0 * std::f64::consts::PI * k as f64 / 96.0))).collect();
            if pts.iter().all(|p| p[0] * p[0] + p[1] * p[1] + p[2] * p[2] < 9.0) {
                fib.push((eta, pts.iter().map(|p| [p[0] as f32, p[1] as f32, p[2] as f32]).collect()));
            }
        }
    }
    let mut smp = vec![];
    let n = (samples as f64).cbrt().ceil() as usize;
    for a in 0..n {
        let eta = std::f64::consts::PI * (a as f64 + 0.5) / n as f64;
        for b in 0..n {
            let phi = 2.0 * std::f64::consts::PI * b as f64 / n as f64;
            for k in 0..n {
                let q = rot(lift(eta, phi), 2.0 * std::f64::consts::PI * k as f64 / n as f64);
                if q[0] < -0.6 {
                    continue; // near the projection pole the picture is too stretched to read
                }
                let z1 = linalg::C64::new(q[0], q[1]);
                let f = z1.powu(j as u32);
                // e_1 = q j, e_2 = q k (left-invariant); vector Re(f) e_1 + Im(f) e_2, pushed forward by a difference
                let qj = [-q[2], -q[3], q[0], q[1]];
                let qk = [-q[3], q[2], -q[1], q[0]];
                let v: Vec<f64> = (0..4).map(|i| f.re * qj[i] + f.im * qk[i]).collect();
                let h = 1e-4;
                let q2 = [q[0] + h * v[0], q[1] + h * v[1], q[2] + h * v[2], q[3] + h * v[3]];
                let nq = q2.iter().map(|x| x * x).sum::<f64>().sqrt();
                let (p, p2) = (proj(q), proj([q2[0] / nq, q2[1] / nq, q2[2] / nq, q2[3] / nq]));
                let dir: Vec<f64> = (0..3).map(|i| (p2[i] - p[i]) / h).collect();
                smp.push((
                    [p[0] as f32, p[1] as f32, p[2] as f32],
                    [dir[0] as f32, dir[1] as f32, dir[2] as f32],
                    f.norm_sqr() as f32,
                    f.arg() as f32,
                ));
            }
        }
    }
    SphereData { t, j, fibres: fib, samples: smp }
}

fn probe(m: &Member, jmax: usize) -> ProbeData {
    let rows: Vec<(usize, f64, f64, f64)> = (0..=jmax)
        .into_par_iter()
        .map(|j| {
            let sd = SectorData::new(m, j);
            let lam = m.twist(j);
            let (a, q) = sd.margin_a(lam);
            (j, a, q, sd.margin_s(lam))
        })
        .collect();
    ProbeData { m: *m, margins_a: rows.iter().map(|r| (r.0, r.1, r.2)).collect(), margins_s: rows.iter().map(|r| (r.0, r.3)).collect() }
}
