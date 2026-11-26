//! The Geometry scene: the control runs as shapes. Left, the spectrum as it runs through the controls (g² 0.002 then
//! 0.004, Mq 6 → 8 → 10), on a broken axis with one zoomed band per cluster of levels, so the splits that matter
//! (Helike against the competitor, 10⁻² apart) and their settling from rung to rung are visible. Right, for the point
//! under the playhead: a rose per level of what it is made of (fine quadrature filled, coarse outlined: the S2 seam),
//! the octave circle of the registered tones, and the heat signals of every source bending into one slope.
//! Every number is read from `research/geometry-controls-*/geometry.json`, which `scripts/export_helike_geometry.py`
//! writes with the same functions the registered verdicts use. Between points the playhead only moves; nothing is
//! interpolated into a value.

use crate::theme::*;
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, Vec2};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize)]
pub struct GeometryDoc {
    pub points: Vec<GPoint>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GPoint {
    pub g2: f64,
    #[serde(rename = "Mq")]
    pub mq: i64,
    pub levels: Vec<f64>,
    #[serde(default)]
    pub roles: BTreeMap<String, usize>,
    pub identified: Identified,
    pub orders: BTreeMap<String, OrderData>,
    pub sources: BTreeMap<String, Source>,
    pub receipt: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Identified {
    pub helike: usize,
    pub competitor: usize,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OrderData {
    pub order: i64,
    pub mode_weights: Vec<BTreeMap<String, f64>>,
    pub vacuum_weight: Vec<f64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Source {
    pub sector: String,
    pub registered: bool,
    pub weights: Vec<f64>,
    #[serde(default)]
    pub weights_coarse: Vec<f64>,
    pub missing: f64,
    pub plateau: f64,
    pub tau: Vec<f64>,
    #[serde(rename = "C")]
    pub c: Vec<f64>,
    pub tone: Option<Tone>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Tone {
    pub ratios: Vec<f64>,
    pub weight_fractions: Vec<f64>,
}

/// A rose spoke: a source family (weight ‖P_S ψ_n‖² / dim S) or a one-quantum mode direction (squared overlap).
#[derive(Clone, Copy)]
enum Spoke {
    Src(&'static str),
    Mode(&'static str),
}

/// Helike's families across the top, the competitor's across the bottom, the neutral one-quantum modes at the sides.
const SPOKES: [(Spoke, &str, u8); 10] = [
    (Spoke::Src("H"), "H", 0),
    (Spoke::Src("H'''"), "H‴ hc", 0),
    (Spoke::Mode("s1"), "s1", 2),
    (Spoke::Mode("s2"), "s2", 2),
    (Spoke::Src("R0"), "R0", 1),
    (Spoke::Src("C''"), "C″ hh", 1),
    (Spoke::Src("C"), "C", 1),
    (Spoke::Mode("x"), "x", 2),
    (Spoke::Src("R2"), "R2", 0),
    (Spoke::Src("H''"), "H″ ss", 0),
];

pub struct GeometryView {
    pub doc: Option<(helike_stream::Frame, GeometryDoc)>,
    pub head: f32,
    pub playing: bool,
    pub selected_level: Option<usize>,
    pub show_roses: bool,
    /// 0: spectrum and signals; 1: the state itself
    pub tab: u8,
    pub state: crate::view_state::StateView,
}

impl GeometryView {
    pub fn new(root: Option<std::path::PathBuf>) -> Self {
        Self { doc: None, head: 5.0, playing: false, selected_level: None, show_roses: true, tab: 0, state: crate::view_state::StateView::new(root) }
    }

    pub fn accept(&mut self, f: helike_stream::Frame) {
        if let Ok(d) = serde_json::from_value::<GeometryDoc>(f.data.clone()) {
            let n = d.points.len().max(1) as f32;
            self.head = self.head.min(n - 1.0);
            if let Some(p) = d.points.first() {
                self.state.roles = p.roles.iter().map(|(k, v)| (k.clone(), *v)).collect();
            }
            self.doc = Some((f, d));
        }
    }

    pub fn ready(&self) -> bool {
        if self.tab == 1 {
            self.state.slice.is_some() || self.state.error.is_some()
        } else {
            self.doc.is_some()
        }
    }

    pub fn side_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for (t, name) in [(0u8, "Spectrum and signals"), (1, "The state")] {
                if ui.selectable_label(self.tab == t, name).clicked() {
                    self.tab = t;
                    if t == 1 && self.state.slice.is_none() {
                        self.state.dirty = true;
                    }
                }
            }
        });
        ui.add_space(8.0);
        if self.tab == 1 {
            self.state.side_controls(ui);
            return;
        }
        ui.label(eyebrow("Run through the controls"));
        let n = self.doc.as_ref().map(|d| d.1.points.len()).unwrap_or(1).max(1);
        ui.horizontal(|ui| {
            if ui.button(if self.playing { "Pause" } else { "Play" }).clicked() {
                if !self.playing && self.head >= (n - 1) as f32 {
                    self.head = 0.0;
                }
                self.playing = !self.playing;
            }
            if ui.button("Step").clicked() {
                self.playing = false;
                self.head = ((self.head.round() as usize + 1) % n) as f32;
            }
        });
        let mut h = self.head;
        if ui.add(egui::Slider::new(&mut h, 0.0..=(n - 1) as f32).show_value(false).text("playhead")).changed() {
            self.head = h;
            self.playing = false;
        }
        if let Some((_, d)) = &self.doc {
            if let Some(p) = d.points.get(self.head.round() as usize) {
                ui.label(RichText::new(format!("g² {} · Mq {}", p.g2, p.mq)).monospace().color(INK));
                ui.label(caption(p.receipt.clone()));
            }
        }
        ui.add_space(10.0);
        ui.label(eyebrow("Layers"));
        ui.checkbox(&mut self.show_roses, "roses on the ladder");
        ui.add_space(10.0);
        ui.label(eyebrow("Reading the shapes"));
        for t in [
            "Ladder: E − E₀ of every Ritz level on a broken axis, one zoomed band per cluster. Lines join one level index from rung to rung: convergence is the lines going flat.",
            "Rose: what a level is made of. Top: Helike's source families (H linear, H‴ h·c, H″ s·s, R2 random). Bottom: the competitor's (C, C″ h·h, R0). Sides: the one-quantum x, s1, s2. Petal area is weight.",
            "Filled: fine quadrature order. White outline: coarse. Where they differ, the S2 seam shows.",
            "Octave circle: tone ratio r at angle 2π·log₂ r, so the octave r = 2 lands where r = 1 is. Offsets are given in cents.",
            "Corridor: log C_S(τ) of every source against τΔ. One corridor = every curve ends parallel to the dashed e^(−Δτ).",
        ] {
            ui.label(caption(t));
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        if self.tab == 1 {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("The state itself").size(19.0).color(INK));
                ui.add_space(8.0);
                ui.label(caption("the slow function f = ψ/ψ₀ over the ground state's contours, on a slice of normal coordinates"));
            });
            self.state.ui(ui);
            return;
        }
        let Some((_, doc)) = &self.doc else {
            ui.label(RichText::new("No geometry export found. Run: python scripts/export_helike_geometry.py research/gluing-check-20261126/controls research/geometry-controls-20261126/geometry.json").color(RIVAL));
            return;
        };
        let n = doc.points.len();
        if n == 0 {
            return;
        }
        if self.playing {
            let dt = ui.input(|i| i.stable_dt).min(0.1);
            self.head += dt * 0.5;
            if self.head >= (n - 1) as f32 {
                self.head = (n - 1) as f32;
                self.playing = false;
            }
            ui.ctx().request_repaint();
        }
        let doc = doc.clone();
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("The controls, as geometry").size(19.0).color(INK));
            ui.add_space(8.0);
            ui.label(caption("the levels, what they are made of, and how their signals fall, through the control points"));
        });
        ui.horizontal_wrapped(|ui| {
            crate::status_chip(ui, self.doc.as_ref().map(|x| &x.0));
            ui.label(caption("research/geometry-controls-20261126/geometry.json · scripts/export_helike_geometry.py · floating point, not a bound"));
        });
        ui.add_space(10.0);
        // wide: ladder beside the panels; narrow: stacked, and the scene scrolls (nothing is squeezed into overlap)
        let avail = ui.available_size();
        let wide = avail.x >= 1100.0;
        egui::ScrollArea::vertical().id_salt("geometry").auto_shrink([false, false]).show(ui, |ui| {
        let w = ui.available_width().max(620.0);
        let total_h = if wide { avail.y.max(660.0) } else { 560.0 + 20.0 + 760.0 };
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, total_h), Sense::click());
        let p = ui.painter_at(rect);
        let (ladder, right) = if wide {
            let left_w = (rect.width() * 0.44).clamp(460.0, 760.0);
            let l = Rect::from_min_size(rect.min, Vec2::new(left_w, rect.height()));
            (l, Rect::from_min_max(Pos2::new(l.right() + 28.0, rect.top()), rect.max))
        } else {
            let l = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 560.0));
            (l, Rect::from_min_max(Pos2::new(rect.left(), l.bottom() + 20.0), rect.max))
        };
        let click = if resp.clicked() { resp.interact_pointer_pos() } else { None };
        self.draw_ladder(&p, ladder, &doc, click, resp.hover_pos());
        let pt = &doc.points[self.head.round() as usize];
        let top_h = (right.height() * 0.46).min(360.0);
        let circle_w = (top_h * 0.95).min(right.width() * 0.4);
        let roses = Rect::from_min_size(right.min, Vec2::new(right.width() - circle_w - 20.0, top_h));
        let circle = Rect::from_min_size(Pos2::new(right.right() - circle_w, right.top()), Vec2::new(circle_w, top_h));
        let heat = Rect::from_min_max(Pos2::new(right.left(), right.top() + top_h + 18.0), right.max);
        self.draw_roses(&p, roses, pt);
        octave_circle(&p, circle, pt, &doc);
        draw_heat(&p, heat, pt);
        });
    }

    fn draw_ladder(&mut self, p: &egui::Painter, r: Rect, doc: &GeometryDoc, click: Option<Pos2>, hover: Option<Pos2>) {
        let n = doc.points.len();
        p.text(r.left_top(), Align2::LEFT_TOP, "SPECTRUM THROUGH THE CONTROLS", FontId::monospace(10.0), MUTED);
        let plot = Rect::from_min_max(r.min + Vec2::new(62.0, 40.0), r.max - Vec2::new(86.0, 34.0));
        let col_w = plot.width() / n as f32;
        let x_of = |i: f32| plot.left() + col_w * (i + 0.5);
        // bands: clusters of excited levels across every point, split where the gap exceeds 0.25
        let mut all: Vec<f64> = doc.points.iter().flat_map(|q| q.levels.iter().skip(1).cloned()).collect();
        all.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut bands: Vec<(f64, f64)> = vec![];
        for e in all {
            match bands.last_mut() {
                Some(b) if e - b.1 <= 0.25 => b.1 = e,
                _ => bands.push((e, e)),
            }
        }
        let gap = 22.0;
        let ground_h = 26.0;
        let bh = (plot.height() - ground_h - gap * bands.len() as f32) / bands.len().max(1) as f32;
        // band rects, bottom-up: ground, then each cluster
        let mut rects: Vec<(Rect, f64, f64)> = vec![];
        let mut y = plot.bottom();
        rects.push((Rect::from_min_max(Pos2::new(plot.left(), y - ground_h), Pos2::new(plot.right(), y)), -0.5, 0.5));
        y -= ground_h + gap;
        for (lo, hi) in &bands {
            let pad = ((hi - lo) * 0.12).max(2e-3);
            rects.push((Rect::from_min_max(Pos2::new(plot.left(), y - bh), Pos2::new(plot.right(), y)), lo - pad, hi + pad));
            y -= bh + gap;
        }
        let y_of = |e: f64| -> f32 {
            for (rr, lo, hi) in &rects {
                if e >= *lo && e <= *hi {
                    return rr.bottom() - ((e - lo) / (hi - lo)) as f32 * rr.height();
                }
            }
            rects[0].0.center().y
        };
        // coupling columns
        let mut start = 0;
        for i in 1..=n {
            if i == n || doc.points[i].g2 != doc.points[start].g2 {
                let a = x_of(start as f32) - col_w * 0.46;
                let b = x_of((i - 1) as f32) + col_w * 0.46;
                p.text(Pos2::new((a + b) / 2.0, plot.top() - 10.0), Align2::CENTER_BOTTOM, format!("g² {}", doc.points[start].g2), FontId::monospace(10.5), MUTED);
                for (rr, _, _) in &rects {
                    p.rect_filled(Rect::from_x_y_ranges(a..=b, rr.top()..=rr.bottom()), CornerRadius::same(5), alpha(RAISED, 0.7));
                }
                start = i;
            }
        }
        // band axes: range labels and break marks
        for (k, (rr, lo, hi)) in rects.iter().enumerate() {
            if k == 0 {
                p.text(Pos2::new(plot.left() - 8.0, rr.center().y), Align2::RIGHT_CENTER, "E₀", FontId::monospace(10.0), FAINT);
                continue;
            }
            p.line_segment([Pos2::new(plot.left() - 3.0, rr.top()), Pos2::new(plot.left() - 3.0, rr.bottom())], Stroke::new(1.0, RULE));
            p.text(Pos2::new(plot.left() - 8.0, rr.top()), Align2::RIGHT_TOP, format!("{hi:.4}"), FontId::monospace(9.0), FAINT);
            p.text(Pos2::new(plot.left() - 8.0, rr.bottom()), Align2::RIGHT_BOTTOM, format!("{lo:.4}"), FontId::monospace(9.0), FAINT);
            let yb = rr.bottom() + gap / 2.0;
            for dx in [-6.0, -1.0] {
                p.line_segment([Pos2::new(plot.left() + dx - 3.0, yb + 3.0), Pos2::new(plot.left() + dx + 3.0, yb - 3.0)], Stroke::new(1.0, MUTED));
            }
            p.text(Pos2::new(plot.right() + 6.0, rr.top() + 2.0), Align2::LEFT_TOP, format!("zoom ×{:.0}", 2.5 / (hi - lo)), FontId::monospace(8.5), FAINT);
        }
        for (i, q) in doc.points.iter().enumerate() {
            p.text(Pos2::new(x_of(i as f32), plot.bottom() + 8.0), Align2::CENTER_TOP, format!("Mq {}", q.mq), FontId::monospace(10.0), MUTED);
        }
        // the same level index joined across the rungs of one coupling
        let k = doc.points.iter().map(|q| q.levels.len()).min().unwrap_or(0);
        for lev in 1..k {
            for i in 0..n - 1 {
                if doc.points[i].g2 != doc.points[i + 1].g2 {
                    continue;
                }
                let col = level_colour(&doc.points[i + 1], lev);
                let (y0, y1) = (y_of(doc.points[i].levels[lev]), y_of(doc.points[i + 1].levels[lev]));
                p.line_segment([Pos2::new(x_of(i as f32) + col_w * 0.18, y0), Pos2::new(x_of(i as f32 + 1.0) - col_w * 0.30, y1)], Stroke::new(1.0, alpha(col, 0.4)));
            }
        }
        let rose_r = (col_w * 0.13).clamp(5.0, 11.0);
        let mut picked = None;
        for (i, q) in doc.points.iter().enumerate() {
            let x = x_of(i as f32);
            let at_head = (self.head - i as f32).abs() < 0.5;
            for (lev, e) in q.levels.iter().enumerate() {
                let yy = y_of(*e);
                let col = level_colour(q, lev);
                p.line_segment([Pos2::new(x - col_w * 0.30, yy), Pos2::new(x + col_w * 0.02, yy)], Stroke::new(if at_head { 2.2 } else { 1.4 }, alpha(col, if at_head { 1.0 } else { 0.75 })));
                if self.show_roses && lev > 0 && at_head {
                    rose(p, Pos2::new(x + col_w * 0.18, yy), rose_r, q, lev, false);
                }
                let hit = |c: Pos2| (c.y - yy).abs() < 5.0 && (c.x - x).abs() < col_w * 0.45;
                if click.is_some_and(hit) {
                    picked = Some((i, lev));
                }
                if hover.is_some_and(hit) {
                    p.text(Pos2::new(x - col_w * 0.32, yy - 3.0), Align2::RIGHT_BOTTOM, format!("n={lev}  {e:.6}"), FontId::monospace(9.5), INK);
                }
            }
            for (lev, col) in [(q.identified.helike, HELIKE), (q.identified.competitor, RIVAL)] {
                if let Some(e) = q.levels.get(lev) {
                    p.circle_stroke(Pos2::new(x - col_w * 0.30 - 6.0, y_of(*e)), 3.5, Stroke::new(1.5, col));
                }
            }
        }
        if let Some((i, lev)) = picked {
            self.head = i as f32;
            self.playing = false;
            self.selected_level = Some(lev);
        }
        let xh = x_of(self.head);
        p.line_segment([Pos2::new(xh, plot.top() - 2.0), Pos2::new(xh, plot.bottom() + 2.0)], Stroke::new(1.0, alpha(HELIKE, 0.5)));
        // the registered gap pair at the head, bracketed at the right edge
        if let Some(q) = doc.points.get(self.head.round() as usize) {
            let (eh, ec) = (q.levels[q.identified.helike], q.levels[q.identified.competitor]);
            let xb = plot.right() + 10.0;
            let (yh, yc) = (y_of(eh), y_of(ec));
            p.line_segment([Pos2::new(xb, yh), Pos2::new(xb, yc)], Stroke::new(2.0, INK));
            p.line_segment([Pos2::new(xb - 4.0, yh), Pos2::new(xb, yh)], Stroke::new(2.0, HELIKE));
            p.line_segment([Pos2::new(xb - 4.0, yc), Pos2::new(xb, yc)], Stroke::new(2.0, RIVAL));
            p.text(Pos2::new(xb + 6.0, (yh + yc) / 2.0 - 7.0), Align2::LEFT_CENTER, "Δ_C − Δ_H", FontId::monospace(9.5), MUTED);
            p.text(Pos2::new(xb + 6.0, (yh + yc) / 2.0 + 7.0), Align2::LEFT_CENTER, format!("{:+.5}", ec - eh), FontId::monospace(10.5), INK);
        }
        p.text(Pos2::new(plot.left(), r.bottom() - 2.0), Align2::LEFT_BOTTOM, "rings: registered identification · click a level", FontId::monospace(9.0), FAINT);
    }

    fn draw_roses(&self, p: &egui::Painter, r: Rect, q: &GPoint) {
        p.text(r.left_top(), Align2::LEFT_TOP, format!("WHAT EACH LEVEL IS MADE OF · g² {} · Mq {}", q.g2, q.mq), FontId::monospace(10.0), MUTED);
        let (fine, coarse) = (q.orders.get("fine").map(|o| o.order).unwrap_or(0), q.orders.get("coarse").map(|o| o.order).unwrap_or(0));
        p.text(r.left_top() + Vec2::new(0.0, 15.0), Align2::LEFT_TOP, format!("filled: quadrature order {fine} · outline: order {coarse} (the S2 seam)"), FontId::monospace(9.5), FAINT);
        // a labelled key first, then the excited levels, four to a row
        let mut levels: Vec<usize> = (1..q.levels.len()).collect();
        let cols = 4usize;
        levels.truncate(cols * 2 - 1);
        if let Some(sel) = self.selected_level {
            if sel > 0 && sel < q.levels.len() && !levels.contains(&sel) {
                levels.pop();
                levels.push(sel);
            }
        }
        let cw = r.width() / cols as f32;
        let ch = (r.height() - 40.0) / 2.0;
        let rr = (cw.min(ch - 34.0) * 0.36).max(12.0);
        for slot in 0..cols * 2 {
            let cell = Rect::from_min_size(Pos2::new(r.left() + cw * (slot % cols) as f32, r.top() + 36.0 + ch * (slot / cols) as f32), Vec2::new(cw, ch));
            let c = Pos2::new(cell.center().x, cell.top() + rr + 14.0);
            if slot == 0 {
                // the key: spokes named
                for (s, (_, label, kind)) in SPOKES.iter().enumerate() {
                    let a = spoke_angle(s);
                    p.line_segment([c, c + Vec2::angled(a) * rr], Stroke::new(1.0, alpha(spoke_colour(*kind), 0.5)));
                    p.text(c + Vec2::angled(a) * (rr + 11.0), Align2::CENTER_CENTER, *label, FontId::monospace(9.0), alpha(spoke_colour(*kind), 0.9));
                }
                p.text(Pos2::new(c.x, c.y + rr + 14.0), Align2::CENTER_TOP, "key", FontId::monospace(9.5), MUTED);
                continue;
            }
            let Some(lev) = levels.get(slot - 1).copied() else { continue };
            if self.selected_level == Some(lev) {
                p.rect_stroke(cell.shrink(3.0), CornerRadius::same(6), Stroke::new(1.0, alpha(HELIKE, 0.6)), StrokeKind::Inside);
            }
            for s in 0..SPOKES.len() {
                p.line_segment([c, c + Vec2::angled(spoke_angle(s)) * rr], Stroke::new(1.0, alpha(HAIRLINE, 0.9)));
            }
            p.circle_stroke(c, rr, Stroke::new(1.0, alpha(HAIRLINE, 0.6)));
            rose(p, c, rr, q, lev, true);
            let col = level_colour(q, lev);
            p.text(Pos2::new(c.x, c.y + rr + 6.0), Align2::CENTER_TOP, format!("n = {lev}"), FontId::monospace(10.0), col);
            p.text(Pos2::new(c.x, c.y + rr + 19.0), Align2::CENTER_TOP, format!("{:.5}", q.levels[lev]), FontId::monospace(9.0), MUTED);
        }
    }
}

fn level_colour(q: &GPoint, lev: usize) -> Color32 {
    if lev == 0 {
        return MUTED;
    }
    let near = |a: usize| (q.levels[lev] - q.levels[a]).abs() <= 1e-6;
    if near(q.identified.helike) {
        HELIKE
    } else if near(q.identified.competitor) {
        RIVAL
    } else {
        INK
    }
}

fn spoke_angle(s: usize) -> f32 {
    -std::f32::consts::FRAC_PI_2 + (s as f32 - 0.5) * std::f32::consts::TAU / SPOKES.len() as f32
}

fn spoke_colour(kind: u8) -> Color32 {
    match kind {
        0 => HELIKE,
        1 => RIVAL,
        _ => Color32::from_rgb(150, 164, 176),
    }
}

fn spoke_weight(q: &GPoint, lev: usize, sp: Spoke, fine: bool) -> f64 {
    match sp {
        Spoke::Src(name) => q.sources.get(name).and_then(|s| if fine { s.weights.get(lev) } else { s.weights_coarse.get(lev) }).copied().unwrap_or(0.0),
        Spoke::Mode(name) => q.orders.get(if fine { "fine" } else { "coarse" }).and_then(|o| o.mode_weights.get(lev)).and_then(|m| m.get(name)).copied().unwrap_or(0.0),
    }
}

/// A rose: one petal per spoke, length √weight so petal area reads as weight; the inner ring is the vacuum weight.
fn rose(p: &egui::Painter, c: Pos2, rr: f32, q: &GPoint, lev: usize, outline_coarse: bool) {
    for fine in [true, false] {
        if !fine && !outline_coarse {
            break;
        }
        for (s, (sp, _, kind)) in SPOKES.iter().enumerate() {
            let w = spoke_weight(q, lev, *sp, fine).clamp(0.0, 1.0);
            if w < 1e-4 {
                continue;
            }
            let len = (w.sqrt() as f32) * rr;
            let a = spoke_angle(s);
            let half = 0.26f32;
            let pts = vec![c, c + Vec2::angled(a - half) * len * 0.6, c + Vec2::angled(a) * len, c + Vec2::angled(a + half) * len * 0.6];
            if fine {
                p.add(Shape::convex_polygon(pts, alpha(spoke_colour(*kind), 0.8), Stroke::NONE));
            } else {
                p.add(Shape::closed_line(pts, Stroke::new(0.9, alpha(INK, 0.85))));
            }
        }
    }
    if let Some(v) = q.orders.get("fine").and_then(|o| o.vacuum_weight.get(lev)) {
        if *v > 1e-4 {
            p.circle_stroke(c, rr * (*v as f32).sqrt(), Stroke::new(1.0, alpha(INK, 0.5)));
        }
    }
}

/// Tone ratios on the octave circle (angle 2π·log₂ r): outer ring Helike, inner the competitor; filled at this
/// coupling, hollow at the other coupling with the same Mq. Beside it, each ratio's offset from its octave in cents.
fn octave_circle(p: &egui::Painter, r: Rect, q: &GPoint, doc: &crate::view_geometry::GeometryDoc) {
    p.text(r.left_top(), Align2::LEFT_TOP, "OCTAVE CIRCLE · the tone", FontId::monospace(10.0), MUTED);
    let rad = (r.width().min(r.height() - 90.0) * 0.40).max(30.0);
    let c = Pos2::new(r.center().x, r.top() + 34.0 + rad);
    p.circle_stroke(c, rad, Stroke::new(1.0, HAIRLINE));
    p.circle_stroke(c, rad * 0.68, Stroke::new(1.0, alpha(HAIRLINE, 0.6)));
    for (lab, ratio) in [("1 ≡ 2", 1.0f64), ("3/2", 1.5), ("5/4", 1.25), ("7/4", 1.75)] {
        let a = angle(ratio);
        p.line_segment([c + Vec2::angled(a) * (rad - 4.0), c + Vec2::angled(a) * (rad + 4.0)], Stroke::new(1.0, MUTED));
        p.text(c + Vec2::angled(a) * (rad + 14.0), Align2::CENTER_CENTER, lab, FontId::monospace(8.5), FAINT);
    }
    let mut lines: Vec<(String, Color32)> = vec![];
    for (k, sector) in ["helike", "competitor"].iter().enumerate() {
        let ring = if k == 0 { rad } else { rad * 0.68 };
        let accent = if k == 0 { HELIKE } else { RIVAL };
        for pt in doc.points.iter().filter(|x| x.mq == q.mq) {
            let Some(s) = pt.sources.values().find(|s| s.sector == *sector && s.registered) else { continue };
            let Some(t) = &s.tone else { continue };
            let here = (pt.g2 - q.g2).abs() < 1e-12;
            for (j, (ratio, wf)) in t.ratios.iter().zip(&t.weight_fractions).enumerate() {
                let a = angle(*ratio);
                let pos = c + Vec2::angled(a) * ring;
                let size = 2.5 + 5.0 * (*wf as f32).sqrt();
                if here {
                    p.circle_filled(pos, size, alpha(accent, 0.85));
                    let cents = 1200.0 * ratio.log2();
                    let off = cents - 1200.0 * (cents / 1200.0).round();
                    lines.push((format!("{} r{} = {:.4}  ({:+.1} ¢)", if k == 0 { "H" } else { "C" }, j + 1, ratio, off), accent));
                } else {
                    p.circle_stroke(pos, size + 1.5, Stroke::new(1.0, alpha(accent, 0.55)));
                }
            }
        }
    }
    let mut y = c.y + rad + 26.0;
    for (t, col) in lines {
        p.text(Pos2::new(r.left() + 4.0, y), Align2::LEFT_TOP, t, FontId::monospace(9.5), col);
        y += 13.0;
    }
}

fn angle(ratio: f64) -> f32 {
    -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * (ratio.log2().rem_euclid(1.0)) as f32
}

/// The corridor: log C_S(τ) for every source of each sector against τΔ, side by side.
fn draw_heat(p: &egui::Painter, r: Rect, q: &GPoint) {
    p.text(r.left_top(), Align2::LEFT_TOP, "THE CORRIDOR · heat signal C_S(τ) of every source, against τΔ", FontId::monospace(10.0), MUTED);
    let body = Rect::from_min_max(r.min + Vec2::new(0.0, 18.0), r.max);
    let w = (body.width() - 14.0) / 2.0;
    for (k, sector) in ["helike", "competitor"].iter().enumerate() {
        heat_plot(p, Rect::from_min_size(Pos2::new(body.left() + k as f32 * (w + 14.0), body.top()), Vec2::new(w, body.height())), q, sector);
    }
}

fn heat_plot(p: &egui::Painter, r: Rect, q: &GPoint, sector: &str) {
    let accent = if sector == "helike" { HELIKE } else { RIVAL };
    p.rect(r, CornerRadius::same(6), alpha(RAISED, 0.6), Stroke::new(1.0, HAIRLINE), StrokeKind::Inside);
    let legend_h = 14.0 * 5.0;
    let plot = Rect::from_min_max(r.min + Vec2::new(44.0, 28.0), r.max - Vec2::new(12.0, 24.0 + legend_h));
    let (lo, hi) = (-4.0f32, 0.0f32);
    let y_of = |c: f64| plot.bottom() - (((c.max(1e-12)).log10() as f32 - lo) / (hi - lo)).clamp(0.0, 1.0) * plot.height();
    for d in [0, -1, -2, -3, -4] {
        let y = plot.bottom() - ((d as f32 - lo) / (hi - lo)) * plot.height();
        p.line_segment([Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)], Stroke::new(1.0, alpha(HAIRLINE, 0.6)));
        p.text(Pos2::new(plot.left() - 6.0, y), Align2::RIGHT_CENTER, format!("1e{d}"), FontId::monospace(8.5), FAINT);
    }
    for x in 0..=6 {
        p.text(Pos2::new(plot.left() + plot.width() * x as f32 / 6.0, plot.bottom() + 4.0), Align2::CENTER_TOP, format!("{x}"), FontId::monospace(8.5), FAINT);
    }
    p.text(Pos2::new(plot.right(), plot.bottom() + 4.0), Align2::RIGHT_TOP, "τΔ →", FontId::monospace(8.5), FAINT);
    let reg = q.sources.values().find(|s| s.sector == sector && s.registered);
    let delta = reg.map(|s| s.plateau).unwrap_or(1.0);
    p.text(r.left_top() + Vec2::new(10.0, 8.0), Align2::LEFT_TOP, format!("{sector} · Δ = {delta:.5}"), FontId::monospace(10.0), accent);
    p.add(Shape::dashed_line(&[Pos2::new(plot.left(), y_of(1.0)), Pos2::new(plot.right(), y_of((-6.0f64).exp()))], Stroke::new(1.0, alpha(INK, 0.5)), 6.0, 4.0));
    let mut ly = plot.bottom() + 22.0;
    let mut lx = r.left() + 12.0;
    for (name, s) in q.sources.iter().filter(|(_, s)| s.sector == sector) {
        if s.c.is_empty() {
            continue;
        }
        let pts: Vec<Pos2> = s
            .tau
            .iter()
            .zip(&s.c)
            .filter(|(t, _)| **t * delta <= 6.0 + 1e-9)
            .map(|(t, c)| Pos2::new(plot.left() + (t * delta / 6.0) as f32 * plot.width(), y_of(*c)))
            .collect();
        let col = if s.registered { accent } else { alpha(accent, 0.42) };
        p.add(Shape::line(pts, Stroke::new(if s.registered { 2.4 } else { 1.2 }, col)));
        let label = format!("{name}{}  μ {:.3}", if s.registered { " ●" } else { "" }, s.missing);
        p.text(Pos2::new(lx, ly), Align2::LEFT_TOP, &label, FontId::monospace(9.0), col);
        lx += (r.width() - 24.0) / 3.0;
        if lx > r.right() - 60.0 {
            lx = r.left() + 12.0;
            ly += 14.0;
        }
    }
    p.text(Pos2::new(r.left() + 12.0, r.bottom() - 6.0), Align2::LEFT_BOTTOM, "dashed: e^(−Δτ), the one note · μ: unseen weight", FontId::monospace(8.5), FAINT);
}
