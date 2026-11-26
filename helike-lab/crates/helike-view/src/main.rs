//! helike-view: draws what the solver streams. Scenes: the ladder and the flux fan; Helike on the three-sphere; the
//! edge forming; the registered controls. Every number drawn comes from a frame; every panel shows its frame's status
//! label.

mod controls;
mod solver;
mod state_slice;
mod theme;
mod view_controls;
mod view_geometry;
mod view_state;

use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use helike_core::helike::{berger_xy, Member, Spectrum};
use helike_stream::{Frame, Status};
use solver::{EdgeMapData, LadderData, ProbeData, Request, SphereData};

// Palette (theme.rs): a cool ink for Helike, a warm amber for her competitors, neutral slate for structure.
use theme::{HELIKE, INK, MUTED, PROVED, RIVAL, RULE};

#[derive(PartialEq, Clone, Copy)]
enum Scene {
    Ladder,
    Sphere,
    Edge,
    Controls,
    Geometry,
}

struct App {
    solver: solver::SolverHandle,
    scene: Scene,
    // ladder controls
    t: f64,
    x: f64,
    y: f64,
    jmax: usize,
    j_focus: usize,
    on_berger: bool,
    // sphere controls
    sphere_j: usize,
    yaw: f32,
    pitch: f32,
    show_fibres: bool,
    // edge controls
    edge_y: f64,
    edge_which: Spectrum,
    edge_res: usize,
    // frames
    ladder: Option<(Frame, LadderData)>,
    sphere: Option<(Frame, SphereData)>,
    edge: Option<(Frame, EdgeMapData)>,
    probe: Option<(Frame, ProbeData)>,
    edge_tex: Option<egui::TextureHandle>,
    edge_tex_seq: u64,
    controls: view_controls::ControlsView,
    geometry: view_geometry::GeometryView,
    dirty_ladder: bool,
    dirty_sphere: bool,
    dirty_edge: bool,
    /// --shot SCENE PATH: render SCENE once its frames are complete, save the window as PNG, and exit.
    shot: Option<std::path::PathBuf>,
    shot_wait: u32,
    shot_sent: bool,
}

fn parse_shot() -> (Scene, Option<std::path::PathBuf>) {
    let a: Vec<String> = std::env::args().collect();
    match a.iter().position(|s| s == "--shot") {
        Some(k) if a.len() > k + 2 => {
            let scene = match a[k + 1].as_str() {
                "sphere" => Scene::Sphere,
                "edge" => Scene::Edge,
                "controls" | "controls-lower" => Scene::Controls,
                "geometry" | "geometry-state" => Scene::Geometry,
                _ => Scene::Ladder,
            };
            (scene, Some(a[k + 2].clone().into()))
        }
        _ => (Scene::Ladder, None),
    }
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // One deliberate visual world: a dark instrument panel (theme.rs).
        theme::apply(&cc.egui_ctx);
        let ctx = cc.egui_ctx.clone();
        let solver = solver::spawn(move || ctx.request_repaint());
        let root = std::env::current_dir()
            .ok()
            .and_then(|d| controls::find_root(&d))
            .or_else(|| std::env::current_exe().ok().and_then(|e| e.parent().and_then(controls::find_root)));
        if let Some(r) = &root {
            let _ = solver.tx.send(Request::LoadControls(r.clone()));
        }
        let (x, y) = berger_xy(3.0);
        let (scene, shot) = parse_shot();
        Self {
            shot,
            shot_wait: 0,
            shot_sent: false,
            solver,
            scene,
            t: 3.0,
            x,
            y,
            jmax: 8,
            j_focus: 4,
            on_berger: true,
            sphere_j: 4,
            yaw: 0.6,
            pitch: 0.35,
            show_fibres: true,
            edge_y: 1.5,
            edge_which: Spectrum::A,
            edge_res: 48,
            ladder: None,
            sphere: None,
            edge: None,
            probe: None,
            edge_tex: None,
            edge_tex_seq: 0,
            geometry: {
                let mut g = view_geometry::GeometryView::new(root.clone());
                if std::env::args().any(|a| a == "geometry-state") {
                    g.tab = 1;
                }
                g
            },
            controls: {
                let mut v = view_controls::ControlsView::new(root);
                v.start_at_bottom = std::env::args().any(|a| a == "controls-lower");
                v
            },
            dirty_ladder: true,
            dirty_sphere: true,
            dirty_edge: true,
        }
    }

    fn pump(&mut self) {
        while let Ok(f) = self.solver.rx.try_recv() {
            match f.scene.as_str() {
                "ladder" => self.ladder = serde_json::from_value(f.data.clone()).ok().map(|d| (f, d)),
                "sphere" => self.sphere = serde_json::from_value(f.data.clone()).ok().map(|d| (f, d)),
                "edge" => self.edge = serde_json::from_value(f.data.clone()).ok().map(|d| (f, d)),
                "probe" => self.probe = serde_json::from_value(f.data.clone()).ok().map(|d| (f, d)),
                "controls" | "controls-history" => self.controls.accept(f),
                "geometry" => self.geometry.accept(f),
                "state" | "state-error" => self.geometry.state.accept(f),
                _ => {}
            }
        }
        if self.scene == Scene::Geometry && self.geometry.tab == 1 {
            if let Some(r) = self.geometry.state.take_request() {
                let _ = self.solver.tx.send(Request::StateSlice(r));
            }
        }
        if self.dirty_ladder {
            let _ = self.solver.tx.send(Request::Ladder { m: Member::new(self.x, self.y, self.t), jmax: self.jmax });
            self.dirty_ladder = false;
        }
        if self.dirty_sphere {
            let _ = self.solver.tx.send(Request::Sphere { t: self.t, j: self.sphere_j, fibres: 144, samples: 1500 });
            self.dirty_sphere = false;
        }
        if self.dirty_edge && self.scene == Scene::Edge {
            let n = self.edge_res;
            let _ = self.solver.tx.send(Request::EdgeMap {
                y: self.edge_y,
                t: (2.0, 10.0, n),
                x: (-3.0, 1.0, (n as f64 * 1.25) as usize),
                jmax: 12,
                which: self.edge_which,
            });
            self.dirty_edge = false;
        }
    }
}

fn status_chip(ui: &mut egui::Ui, f: Option<&Frame>) {
    let (txt, col) = match f.map(|f| f.status) {
        Some(Status::Computed) => ("COMPUTED · CPU f64, parity-tested", PROVED),
        Some(Status::DisplayGrade) => ("DISPLAY-GRADE · do not quote", RIVAL),
        Some(Status::PassCertifiedFp) => ("PASS_CERTIFIED_FP", PROVED),
        Some(Status::PassCertifiedInterval) => ("PASS_CERTIFIED_INTERVAL", PROVED),
        Some(Status::IntervalEnclosure) => ("INTERVAL_ENCLOSURE · bounds, not a pass", PROVED),
        None => ("waiting for the solver", MUTED),
    };
    theme::pill(ui, txt, col);
    if let Some(f) = f {
        ui.label(egui::RichText::new(format!("frame #{}", f.seq)).color(theme::FAINT).monospace().size(10.5));
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.pump();
        let bar = egui::Frame::new().fill(theme::GROUND).inner_margin(egui::Margin::symmetric(16, 10)).stroke(Stroke::new(1.0, theme::HAIRLINE));
        egui::Panel::top("scenes").frame(bar).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("helike").size(16.0).color(INK));
                ui.label(egui::RichText::new("lab").size(16.0).color(HELIKE));
                ui.add_space(18.0);
                for (s, name) in [
                    (Scene::Ladder, "Ladder and flux fan"),
                    (Scene::Sphere, "Helike on the three-sphere"),
                    (Scene::Edge, "The edge forming"),
                    (Scene::Controls, "Controls"),
                    (Scene::Geometry, "Geometry of the controls"),
                ] {
                    if scene_tab(ui, self.scene == s, name).clicked() {
                        self.scene = s;
                        if s == Scene::Edge && self.edge.is_none() {
                            self.dirty_edge = true;
                        }
                    }
                }
            });
        });
        let side = egui::Frame::new().fill(theme::SURFACE).inner_margin(egui::Margin::symmetric(16, 14)).stroke(Stroke::new(1.0, theme::HAIRLINE));
        egui::Panel::left("controls").frame(side).default_size(268.0).show(ui, |ui| self.controls(ui));
        let main = egui::Frame::new().fill(theme::GROUND).inner_margin(egui::Margin::symmetric(22, 16));
        egui::CentralPanel::default().frame(main).show(ui, |ui| match self.scene {
            Scene::Ladder => self.draw_ladder(ui),
            Scene::Sphere => self.draw_sphere(ui),
            Scene::Edge => self.draw_edge(ui),
            Scene::Controls => self.controls.ui(ui),
            Scene::Geometry => self.geometry.ui(ui),
        });
        self.handle_shot(ui.ctx());
    }
}

impl App {
    fn handle_shot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.shot.clone() else { return };
        let ready = match self.scene {
            Scene::Ladder => self.ladder.is_some(),
            Scene::Sphere => self.sphere.is_some(),
            Scene::Edge => self.edge.as_ref().is_some_and(|(_, d)| d.rows_done == d.ts.len()),
            Scene::Controls => self.controls.ready(),
            Scene::Geometry => self.geometry.ready(),
        };
        if !ready {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return;
        }
        if !self.shot_sent {
            self.shot_wait += 1;
            if self.shot_wait > 3 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
                self.shot_sent = true;
            }
            ctx.request_repaint();
            return;
        }
        let img = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| if let egui::Event::Screenshot { image, .. } = e { Some(image.clone()) } else { None })
        });
        if let Some(img) = img {
            let [w, h] = img.size;
            let file = std::fs::File::create(&path).expect("create screenshot file");
            let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header().unwrap().write_image_data(img.as_raw()).unwrap();
            eprintln!("saved {}", path.display());
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else {
            ctx.request_repaint();
        }
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        match self.scene {
            Scene::Controls => self.controls.side_controls(ui),
            Scene::Geometry => self.geometry.side_controls(ui),
            Scene::Ladder | Scene::Sphere => {
                ui.label(theme::eyebrow("The family member"));
                let mut ch = false;
                ch |= ui.add(egui::Slider::new(&mut self.t, 0.5..=12.0).text("stretch t")).changed();
                ch |= ui.checkbox(&mut self.on_berger, "on the Berger field").changed();
                if self.on_berger {
                    let (x, y) = berger_xy(self.t);
                    self.x = x;
                    self.y = y;
                } else {
                    ch |= ui.add(egui::Slider::new(&mut self.x, -4.0..=2.0).text("x (along)")).changed();
                    ch |= ui.add(egui::Slider::new(&mut self.y, 0.0..=4.0).text("y (across)")).changed();
                }
                ui.label(egui::RichText::new(format!("x = {:.4}, y = {:.4}", self.x, self.y)).monospace().color(MUTED));
                ch |= ui.add(egui::Slider::new(&mut self.jmax, 1..=16).text("sectors j ≤")).changed();
                if ui.add(egui::Slider::new(&mut self.j_focus, 0..=self.jmax).text("ladder sector j")).changed() {}
                if ch {
                    self.dirty_ladder = true;
                    self.dirty_sphere = true;
                }
                if self.scene == Scene::Sphere {
                    ui.separator();
                    if ui.add(egui::Slider::new(&mut self.sphere_j, 0..=16).text("Helike's sector j")).changed() {
                        self.dirty_sphere = true;
                    }
                    ui.checkbox(&mut self.show_fibres, "long circles (Hopf fibres)");
                    ui.label(egui::RichText::new("drag to turn").color(MUTED));
                }
            }
            Scene::Edge => {
                ui.label(theme::eyebrow("The (x, t) map at fixed y"));
                let mut ch = ui.add(egui::Slider::new(&mut self.edge_y, 0.25..=3.0).text("cross field y")).changed();
                ui.horizontal(|ui| {
                    ch |= ui.radio_value(&mut self.edge_which, Spectrum::A, "H + GG*").changed();
                    ch |= ui.radio_value(&mut self.edge_which, Spectrum::S, "gauge slice").changed();
                });
                ch |= ui.add(egui::Slider::new(&mut self.edge_res, 16..=96).text("resolution")).changed();
                if ui.button("recompute").clicked() || ch {
                    self.dirty_edge = true;
                }
                ui.add_space(8.0);
                ui.label(egui::RichText::new("Click a pixel for its sector margins.").color(MUTED));
                if let Some((f, p)) = &self.probe {
                    ui.separator();
                    ui.label(format!("probe x={:.3}, y={:.3}, t={:.3}", p.m.x, p.m.y, p.m.t));
                    egui::Grid::new("probe").striped(true).show(ui, |ui| {
                        ui.label("j");
                        ui.label("H+GG* margin");
                        ui.label("slice margin");
                        ui.end_row();
                        for ((j, a, q), (_, s)) in p.margins_a.iter().zip(&p.margins_s) {
                            ui.label(format!("{j}"));
                            ui.label(egui::RichText::new(format!("{a:+.4} (q={q})")).color(if *a < 0.0 { RIVAL } else { INK }).monospace());
                            ui.label(egui::RichText::new(format!("{s:+.4}")).color(if *s < 0.0 { RIVAL } else { INK }).monospace());
                            ui.end_row();
                        }
                    });
                    status_chip(ui, Some(f));
                }
            }
        }
    }

    fn draw_ladder(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| status_chip(ui, self.ladder.as_ref().map(|x| &x.0)));
        let Some((_, d)) = &self.ladder else { return };
        let avail = ui.available_size();
        let (resp, painter) = ui.allocate_painter(avail, Sense::hover());
        let r = resp.rect;
        let top_h = r.height() * 0.52;
        let fan = Rect::from_min_size(r.min, Vec2::new(r.width() * 0.32, top_h));
        let rungs = Rect::from_min_size(Pos2::new(fan.max.x + 16.0, r.min.y), Vec2::new(r.width() - fan.width() - 16.0, top_h));
        let prices = Rect::from_min_max(Pos2::new(r.min.x, r.min.y + top_h + 18.0), r.max);
        let font = FontId::proportional(12.0);
        let mono = FontId::monospace(11.0);

        // Flux fan: nine eigenvalues of phi, computed (dots) on closed form (ticks).
        painter.text(fan.left_top(), Align2::LEFT_TOP, "flux fan: the nine eigenvalues of φ", font.clone(), INK);
        let lo = d.flux_closed.iter().cloned().fold(f64::INFINITY, f64::min).min(0.0) - 0.5;
        let hi = d.flux_closed.iter().cloned().fold(f64::NEG_INFINITY, f64::max).max(0.0) + 0.5;
        let fan_cap = painter.layout(format!("B = {:.4}, Θ = {:.4}\ndots computed, ticks closed form: Δ {:.1e}", d.fibre_field, d.cross_field, d.flux_check), mono.clone(), MUTED, (fan.width() - 8.0).max(40.0));
        let plot = Rect::from_min_max(fan.min + Vec2::new(36.0, 22.0), fan.max - Vec2::new(8.0, fan_cap.size().y + 16.0));
        let ymap = |v: f64| plot.bottom() - ((v - lo) / (hi - lo)) as f32 * plot.height();
        painter.line_segment([Pos2::new(plot.left(), ymap(0.0)), Pos2::new(plot.right(), ymap(0.0))], Stroke::new(1.0, RULE));
        for (k, (c, v)) in d.flux_closed.iter().zip(&d.flux_computed).enumerate() {
            let xk = plot.left() + plot.width() * (k as f32 + 0.5) / 9.0;
            painter.line_segment([Pos2::new(xk - 9.0, ymap(*c)), Pos2::new(xk + 9.0, ymap(*c))], Stroke::new(1.5, MUTED));
            let col = if (*v + d.fibre_field).abs() < 1e-9 { HELIKE } else { INK };
            painter.circle_filled(Pos2::new(xk, ymap(*v)), 3.5, col);
        }
        painter.text(Pos2::new(plot.left() - 4.0, ymap(-d.fibre_field)), Align2::RIGHT_CENTER, "−B", mono.clone(), HELIKE);
        painter.galley(Pos2::new(fan.left(), fan.bottom() - 4.0 - fan_cap.size().y), fan_cap, MUTED);

        // The ladder in sector j_focus: rungs at h_m, leak arrows proportional to the leak, the top rung exact.
        let j = self.j_focus.min(d.sectors.len().saturating_sub(1));
        if let Some(row) = d.sectors.get(j) {
            theme::wrapped(&painter, rungs.left_top(), false, format!("the ladder in sector j = {j}: curl ψ = h ψ + χ"), font.clone(), INK, rungs.width());
            let hs: Vec<f64> = row.rungs.iter().map(|r| r.h).collect();
            let (hlo, hhi) = (hs.iter().cloned().fold(f64::INFINITY, f64::min) - 0.5, hs.iter().cloned().fold(f64::NEG_INFINITY, f64::max) + 0.5);
            let worst = row.rungs.iter().map(|rg| (rg.leak - rg.leak_computed).abs()).fold(0.0f64, f64::max);
            let rung_cap = painter.layout(format!("arrows: the leak ‖χ‖² = 2[l(l+1) − m(m+1)]; the blue top rung leaks nothing (the twist)\ncomputed − formula: {worst:.1e}"), mono.clone(), MUTED, rungs.width().max(40.0));
            let pl = Rect::from_min_max(rungs.min + Vec2::new(50.0, 40.0), rungs.max - Vec2::new(8.0, rung_cap.size().y + 18.0));
            let ym = |v: f64| pl.bottom() - ((v - hlo) / (hhi - hlo)) as f32 * pl.height();
            let maxleak = row.rungs.iter().map(|r| r.leak).fold(1e-9, f64::max);
            for rg in &row.rungs {
                let col_x = if rg.branch == 1 { pl.left() + pl.width() * 0.25 } else { pl.left() + pl.width() * 0.7 };
                let yv = ym(rg.h);
                let exact = rg.leak.abs() < 1e-12;
                // rung, arrow and label scale with the panel so the two branches never run into each other
                let hl = (pl.width() * 0.09).min(60.0);
                painter.line_segment([Pos2::new(col_x - hl, yv), Pos2::new(col_x + hl, yv)], Stroke::new(if exact { 3.0 } else { 1.5 }, if exact { HELIKE } else { INK }));
                if !exact {
                    let len = (rg.leak_computed / maxleak) as f32 * hl * 1.15;
                    painter.arrow(Pos2::new(col_x + hl + 4.0, yv), Vec2::new(len, 0.0), Stroke::new(1.2, RIVAL));
                }
                painter.text(Pos2::new(col_x - hl - 4.0, yv), Align2::RIGHT_CENTER, format!("m={:+.1}", rg.mm), mono.clone(), MUTED);
            }
            painter.text(Pos2::new(pl.left() + pl.width() * 0.25, pl.top() + 2.0), Align2::CENTER_BOTTOM, "branch +", mono.clone(), MUTED);
            painter.text(Pos2::new(pl.left() + pl.width() * 0.7, pl.top() + 2.0), Align2::CENTER_BOTTOM, "branch −", mono.clone(), MUTED);
            painter.galley(Pos2::new(rungs.left(), rungs.bottom() - 4.0 - rung_cap.size().y), rung_cap, MUTED);
        }

        // Two prices per sector: the twist and her cheapest gauge-orthogonal rival.
        let head_h = theme::wrapped(&painter, prices.left_top(), false, "two prices per sector: Helike (blue) and the cheapest gauge-orthogonal rival (amber)", font.clone(), INK, prices.width());
        let vals: Vec<f64> = d.sectors.iter().flat_map(|s| [s.twist, s.competitor_slice]).collect();
        let (plo, phi) = (vals.iter().cloned().fold(0.0, f64::min) - 1.0, vals.iter().cloned().fold(0.0, f64::max) + 1.0);
        let worst_twist = d.sectors.iter().map(|s| s.twist_check.abs()).fold(0.0, f64::max);
        let sub_h = theme::wrapped(
            &painter,
            prices.left_top() + Vec2::new(0.0, head_h + 2.0),
            false,
            format!("green dot: the ladder's sufficient condition holds there (a proved floor) · twist closed form vs its block: {worst_twist:.1e}"),
            mono.clone(),
            MUTED,
            prices.width(),
        );
        let pl = Rect::from_min_max(prices.min + Vec2::new(36.0, head_h + sub_h + 14.0), prices.max - Vec2::new(8.0, 22.0));
        let ym = |v: f64| pl.bottom() - ((v - plo) / (phi - plo)) as f32 * pl.height();
        painter.line_segment([Pos2::new(pl.left(), ym(0.0)), Pos2::new(pl.right(), ym(0.0))], Stroke::new(1.0, RULE));
        let n = d.sectors.len() as f32;
        let bw = pl.width() / n;
        for (k, s) in d.sectors.iter().enumerate() {
            let x0 = pl.left() + bw * k as f32;
            let bar = |v: f64, off: f32, col: Color32| {
                let (a, b) = (ym(0.0), ym(v));
                painter.rect_filled(Rect::from_x_y_ranges(x0 + off..=x0 + off + bw * 0.36, a.min(b)..=a.max(b)), CornerRadius::same(2), col);
            };
            bar(s.twist, bw * 0.1, HELIKE);
            bar(s.competitor_slice, bw * 0.52, RIVAL);
            let floor = s.twist <= s.competitor_slice + 1e-12;
            painter.text(Pos2::new(x0 + bw * 0.5, pl.bottom() + 4.0), Align2::CENTER_TOP, format!("{}", s.j), mono.clone(), if floor { INK } else { RIVAL });
            if s.floor_sufficient {
                painter.circle_filled(Pos2::new(x0 + bw * 0.5, pl.top() + 4.0), 3.0, PROVED);
            }
        }

    }

    fn draw_sphere(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            status_chip(ui, self.sphere.as_ref().map(|x| &x.0));
            ui.label(egui::RichText::new("closed forms: |f|² = cos^{2j}(η/2), phase j·(fibre angle); stereographic picture, the stretch t is not drawn to scale").color(MUTED).size(11.0));
        });
        let Some((_, d)) = &self.sphere else { return };
        let avail = ui.available_size();
        let (resp, painter) = ui.allocate_painter(avail, Sense::drag());
        if resp.dragged() {
            let dl = resp.drag_delta();
            self.yaw += dl.x * 0.01;
            self.pitch = (self.pitch + dl.y * 0.01).clamp(-1.5, 1.5);
        }
        let r = resp.rect;
        let c = r.center();
        let scale = r.height().min(r.width()) * 0.22;
        let (cy, sy, cp, sp) = (self.yaw.cos(), self.yaw.sin(), self.pitch.cos(), self.pitch.sin());
        let view = |p: [f32; 3]| -> (Pos2, f32) {
            let x = cy * p[0] + sy * p[2];
            let z = -sy * p[0] + cy * p[2];
            let y2 = cp * p[1] - sp * z;
            let z2 = sp * p[1] + cp * z;
            let persp = 1.0 / (1.0 + 0.12 * z2);
            (Pos2::new(c.x + scale * x * persp, c.y - scale * y2 * persp), z2)
        };
        if self.show_fibres {
            for (eta, pts) in &d.fibres {
                let amp = ((eta / 2.0).cos().powi(2 * d.j as i32)) as f32;
                let col = Color32::from_rgba_unmultiplied(120, 140, 156, (30.0 + 70.0 * (1.0 - amp)) as u8);
                let line: Vec<Pos2> = pts.iter().map(|p| view(*p).0).collect();
                painter.line(line, Stroke::new(0.8, col));
            }
        }
        let mut sm: Vec<_> = d.samples.iter().map(|s| (view(s.0), s)).collect();
        sm.sort_by(|a, b| b.0 .1.partial_cmp(&a.0 .1).unwrap());
        for ((p, _), s) in sm {
            let a = s.2.clamp(0.0, 1.0);
            if a < 0.01 {
                continue;
            }
            let hue = (s.3 / std::f32::consts::TAU).rem_euclid(1.0);
            let col: Color32 = egui::ecolor::Hsva::new(hue, 0.65, 0.95, a.sqrt()).into();
            let n = (s.1[0] * s.1[0] + s.1[1] * s.1[1] + s.1[2] * s.1[2]).sqrt().max(1e-9);
            let len = 0.09 * a.sqrt();
            let tip = view([s.0[0] + len * s.1[0] / n, s.0[1] + len * s.1[1] / n, s.0[2] + len * s.1[2] / n]).0;
            painter.line_segment([p, tip], Stroke::new(1.2, col));
            painter.circle_filled(p, 1.6 + 2.0 * a, col);
        }
        theme::wrapped(
            &painter,
            r.left_bottom() + Vec2::new(8.0, -8.0),
            true,
            format!("sector j = {}: colour = arg f, opacity = |f|², ticks = Re(f)e₁ + Im(f)e₂ (her winding); she gathers about the pole as j rises", d.j),
            FontId::monospace(11.0),
            MUTED,
            r.width() - 16.0,
        );
    }

    fn draw_edge(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            status_chip(ui, self.edge.as_ref().map(|x| &x.0));
            if let Some((_, d)) = &self.edge {
                ui.label(egui::RichText::new(format!("rows {}/{}", d.rows_done, d.ts.len())).monospace().color(MUTED));
            }
        });
        let Some((f, d)) = &self.edge else {
            ui.label("computing…");
            return;
        };
        let (nt, nx) = (d.ts.len(), d.xs.len());
        if self.edge_tex.is_none() || self.edge_tex_seq != f.seq {
            let vmax = d.phi.iter().flatten().filter(|v| v.is_finite()).map(|v| v.abs()).fold(1e-9, f64::max).min(4.0);
            let mut px = Vec::with_capacity(nt * nx);
            for i in (0..nt).rev() {
                for k in 0..nx {
                    let v = d.phi[i][k];
                    px.push(if !v.is_finite() {
                        Color32::from_rgb(30, 34, 38)
                    } else {
                        // diverging: amber below zero (she is not the floor), blue above; grey at zero
                        let s = (v / vmax).clamp(-1.0, 1.0) as f32;
                        let g = Color32::from_rgb(70, 76, 82);
                        let tgt = if s >= 0.0 { HELIKE } else { RIVAL };
                        lerp(g, tgt, s.abs().powf(0.6))
                    });
                }
            }
            let img = egui::ColorImage::new([nx, nt], px);
            self.edge_tex = Some(ui.ctx().load_texture("edge", img, egui::TextureOptions::NEAREST));
            self.edge_tex_seq = f.seq;
        }
        let avail = ui.available_size();
        let (resp, painter) = ui.allocate_painter(avail, Sense::click());
        // the caption is laid out first and wrapped to the panel, and the plot leaves room for it
        let caption = painter.layout(
            format!("x along the long circle →, stretch t ↑ · φ_{} over sectors j ≤ {} · white line: the edge φ = 0 · ring: the Berger field
blue: she is the floor · amber: a rival undercuts her · green: ladder-proved cells and x_flux · box: the certificate domain D", if d.which == Spectrum::A { "A" } else { "S" }, d.jmax),
            FontId::monospace(11.0),
            MUTED,
            (resp.rect.width() - 64.0).max(80.0),
        );
        let plot = Rect::from_min_max(resp.rect.min + Vec2::new(48.0, 10.0), resp.rect.max - Vec2::new(16.0, 30.0 + caption.size().y));
        painter.image(self.edge_tex.as_ref().unwrap().id(), plot, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        let (x0, x1, t0, t1) = (d.xs[0], d.xs[nx - 1], d.ts[0], d.ts[nt - 1]);
        let px = |x: f64| plot.left() + ((x - x0) / (x1 - x0)) as f32 * plot.width();
        let py = |t: f64| plot.bottom() - ((t - t0) / (t1 - t0)) as f32 * plot.height();
        // ladder-proved cells (sufficient condition in every sector j <= jmax): green outline dots
        for i in 0..nt {
            for k in 0..nx {
                if d.ladder_proved[i][k] {
                    painter.circle_filled(Pos2::new(px(d.xs[k]), py(d.ts[i])), 1.4, PROVED);
                }
            }
        }
        // x_flux curve
        let fl: Vec<Pos2> = d.ts.iter().zip(&d.x_flux).filter(|(_, x)| x.is_finite() && **x >= x0 && **x <= x1).map(|(t, x)| Pos2::new(px(*x), py(*t))).collect();
        if fl.len() > 1 {
            painter.line(fl, Stroke::new(1.2, PROVED));
        }
        // the certificate domain D = [-3,-1.5] x [3,10] (valid for 0.5 <= y <= 3)
        if (0.5..=3.0).contains(&d.y) {
            let rd = Rect::from_min_max(Pos2::new(px(-3.0), py(10.0f64.min(t1))), Pos2::new(px(-1.5), py(3.0)));
            painter.rect_stroke(rd, CornerRadius::ZERO, Stroke::new(1.2, INK), StrokeKind::Middle);
            painter.text(rd.left_top() + Vec2::new(4.0, 4.0), Align2::LEFT_TOP, "D (interval certificates)", FontId::monospace(10.0), INK);
        }
        // Berger point: y = sqrt t
        let tb = d.y * d.y;
        if tb >= t0 && tb <= t1 {
            let (xb, _) = berger_xy(tb);
            painter.circle_stroke(Pos2::new(px(xb), py(tb)), 5.0, Stroke::new(1.5, INK));
        }
        painter.line_segment([Pos2::new(px(0.0), plot.top()), Pos2::new(px(0.0), plot.bottom())], Stroke::new(0.8, RULE));
        // the edge: phi = 0, the last sign change from >= 0 to < 0 in each computed row, linearly interpolated
        let contour: Vec<Pos2> = (0..nt)
            .filter_map(|i| {
                let row = &d.phi[i];
                let k = (0..nx - 1).rev().find(|&k| row[k].is_finite() && row[k + 1].is_finite() && row[k] >= 0.0 && row[k + 1] < 0.0)?;
                let x = d.xs[k] + (d.xs[k + 1] - d.xs[k]) * row[k] / (row[k] - row[k + 1]);
                Some(Pos2::new(px(x), py(d.ts[i])))
            })
            .collect();
        if contour.len() > 1 {
            painter.line(contour, Stroke::new(2.0, INK));
        }
        for x in [-3.0, -2.0, -1.0, 0.0, 1.0] {
            painter.text(Pos2::new(px(x), plot.bottom() + 4.0), Align2::CENTER_TOP, format!("{x}"), FontId::monospace(11.0), MUTED);
        }
        for t in [2.0, 4.0, 6.0, 8.0, 10.0] {
            painter.text(Pos2::new(plot.left() - 6.0, py(t)), Align2::RIGHT_CENTER, format!("{t}"), FontId::monospace(11.0), MUTED);
        }
        painter.galley(Pos2::new(plot.left(), resp.rect.bottom() - 4.0 - caption.size().y), caption, MUTED);
        if resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                if plot.contains(pos) {
                    let x = x0 + ((pos.x - plot.left()) / plot.width()) as f64 * (x1 - x0);
                    let t = t0 + ((plot.bottom() - pos.y) / plot.height()) as f64 * (t1 - t0);
                    let _ = self.solver.tx.send(Request::Probe { m: Member::new(x, d.y, t), jmax: d.jmax });
                }
            }
        }
    }
}

/// Window size: `--size WxH` (used for layout audits at the minimum size), the tall controls-lower shot, or the default.
fn window_size() -> [f32; 2] {
    let a: Vec<String> = std::env::args().collect();
    if let Some(k) = a.iter().position(|s| s == "--size") {
        if let Some((w, h)) = a.get(k + 1).and_then(|v| v.split_once('x')) {
            if let (Ok(w), Ok(h)) = (w.parse(), h.parse()) {
                return [w, h];
            }
        }
    }
    if a.iter().any(|s| s == "controls-lower") {
        [1440.0, 1900.0]
    } else {
        [1440.0, 900.0]
    }
}

/// A scene tab: text with an underline in Helike blue when selected.
fn scene_tab(ui: &mut egui::Ui, selected: bool, name: &str) -> egui::Response {
    let col = if selected { INK } else { MUTED };
    let g = ui.painter().layout_no_wrap(name.to_string(), FontId::proportional(13.0), col);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(g.size().x + 20.0, 28.0), Sense::click());
    let p = ui.painter();
    if resp.hovered() && !selected {
        p.rect_filled(rect, CornerRadius::same(5), theme::RAISED);
    }
    p.galley(Pos2::new(rect.left() + 10.0, rect.center().y - g.size().y / 2.0), g, col);
    if selected {
        p.line_segment([Pos2::new(rect.left() + 10.0, rect.bottom() - 1.0), Pos2::new(rect.right() - 10.0, rect.bottom() - 1.0)], Stroke::new(2.0, HELIKE));
    }
    resp
}

fn lerp(a: Color32, b: Color32, s: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * s).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        // the lower-half shot uses a tall window so the detail, predictions and history fit
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(window_size()).with_min_inner_size([960.0, 640.0]).with_title("helike-lab"),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native("helike-lab", opts, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
