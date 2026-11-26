//! "The state itself": f = ψ/ψ₀ on a slice of normal coordinates over the contours of ψ₀², and the state's own
//! density |ψ|² beside it. For Helike's chirally mixed pair f is complex: hue is its phase, and her charge-2 winding
//! appears as two turns of colour around the origin (the winding is also counted on the unit circle and printed).
//! DisplayGrade: computed in the viewer's solver thread from exported coefficients, drawn, never quoted.

use crate::state_slice::{Slice, Which};
use crate::theme::*;
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};

pub const ROLES: [&str; 6] = ["h1", "h2", "c", "x", "s1", "s2"];

#[derive(Clone, Debug)]
pub struct SliceRequest {
    pub levels_path: std::path::PathBuf,
    pub pair_path: std::path::PathBuf,
    pub which: Which,
    pub axes: (usize, usize),
    pub fixed: [f64; 6],
    pub extent: f64,
    pub res: usize,
}

pub struct StateView {
    pub root: Option<std::path::PathBuf>,
    pub g2: f64,
    pub mq: usize,
    pub which: Which,
    /// axes as role names; mapped to mode indices through the point's roles
    pub axis_a: usize,
    pub axis_b: usize,
    pub offset: f64,
    pub level_n: usize,
    pub dirty: bool,
    pub slice: Option<(helike_stream::Frame, Slice)>,
    pub error: Option<String>,
    tex: Option<(u64, egui::TextureHandle, egui::TextureHandle)>,
    /// role name -> mode index (from the geometry export)
    pub roles: Vec<(String, usize)>,
}

impl StateView {
    pub fn new(root: Option<std::path::PathBuf>) -> Self {
        Self { root, g2: 0.004, mq: 10, which: Which::Mixed(1), level_n: 1, axis_a: 0, axis_b: 1, offset: 0.0, dirty: true, slice: None, error: None, tex: None, roles: vec![] }
    }

    fn mode_of(&self, role: usize) -> usize {
        self.roles.iter().find(|(n, _)| n == ROLES[role]).map(|(_, m)| *m).unwrap_or(role)
    }

    pub fn take_request(&mut self) -> Option<SliceRequest> {
        if !self.dirty {
            return None;
        }
        self.dirty = false;
        let root = self.root.as_ref()?;
        let dir = root.join("research/geometry-controls-20261126/states");
        let a = self.mode_of(self.axis_a);
        let b = self.mode_of(self.axis_b);
        let mut fixed = [self.offset; 6];
        fixed[a] = 0.0;
        fixed[b] = 0.0;
        Some(SliceRequest {
            levels_path: dir.join(format!("levels_g{}_Mq{}.json", self.g2, self.mq)),
            pair_path: dir.join(format!("pair_g{}_Mq{}.json", self.g2, self.mq)),
            which: self.which,
            axes: (a, b),
            fixed,
            extent: 3.0,
            res: 360,
        })
    }

    pub fn accept(&mut self, f: helike_stream::Frame) {
        match f.scene.as_str() {
            "state" => {
                if let Ok(s) = serde_json::from_value::<Slice>(f.data.clone()) {
                    self.slice = Some((f, s));
                    self.error = None;
                }
            }
            "state-error" => self.error = f.data.as_str().map(String::from),
            _ => {}
        }
    }

    pub fn side_controls(&mut self, ui: &mut egui::Ui) {
        ui.label(eyebrow("Control point"));
        let mut ch = false;
        ui.horizontal(|ui| {
            for g in [0.002, 0.004] {
                ch |= ui.selectable_value(&mut self.g2, g, format!("g² {g}")).changed();
            }
        });
        ui.horizontal(|ui| {
            for m in [8usize, 10] {
                ch |= ui.selectable_value(&mut self.mq, m, format!("Mq {m}")).changed();
            }
        });
        ui.add_space(6.0);
        ui.label(eyebrow("State"));
        for (w, label) in [
            (Which::Mixed(1), "Helike, charge +2 (chiral)"),
            (Which::Mixed(2), "Helike, charge −2 (chiral)"),
            (Which::Mixed(3), "the competitor"),
        ] {
            ch |= ui.radio_value(&mut self.which, w, label).changed();
        }
        ui.horizontal(|ui| {
            let on = matches!(self.which, Which::Level(_));
            if ui.radio(on, "Ritz level n =").clicked() {
                self.which = Which::Level(self.level_n);
                ch = true;
            }
            if ui.add(egui::DragValue::new(&mut self.level_n).range(1..=11)).changed() && on {
                self.which = Which::Level(self.level_n);
                ch = true;
            }
        });
        ui.add_space(6.0);
        ui.label(eyebrow("Slice plane"));
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("ax_a").width(60.0).selected_text(ROLES[self.axis_a]).show_ui(ui, |ui| {
                for (i, r) in ROLES.iter().enumerate() {
                    ch |= ui.selectable_value(&mut self.axis_a, i, *r).changed();
                }
            });
            ui.label("×");
            egui::ComboBox::from_id_salt("ax_b").width(60.0).selected_text(ROLES[self.axis_b]).show_ui(ui, |ui| {
                for (i, r) in ROLES.iter().enumerate() {
                    ch |= ui.selectable_value(&mut self.axis_b, i, *r).changed();
                }
            });
        });
        ch |= ui.add(egui::Slider::new(&mut self.offset, -1.5..=1.5).text("other coordinates at")).changed();
        if ch && self.axis_a != self.axis_b {
            self.dirty = true;
        }
        ui.add_space(10.0);
        ui.label(eyebrow("Reading it"));
        ui.label(caption("Left: f = ψ/ψ₀, the slow function. For a complex state, hue is the phase of f and brightness its size; a charge-m state winds m times around the origin."));
        ui.label(caption("Right: the state's density |ψ|² = |f|²ψ₀². Both panels carry the contours of the ground density ψ₀²."));
        ui.label(caption("DISPLAY-GRADE: evaluated in the viewer from exported Hermite coefficients. Not a receipt value."));
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            crate::status_chip(ui, self.slice.as_ref().map(|x| &x.0));
            if let Some((_, s)) = &self.slice {
                let name = std::path::Path::new(&s.source).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                ui.label(caption(format!("research/geometry-controls-20261126/states/{name}")));
            }
        });
        if let Some(e) = &self.error {
            ui.add_space(6.0);
            ui.label(RichText::new(format!("{e}\nExport the states with helike cone-levels / cone-export into research/geometry-controls-20261126/states/.")).color(RIVAL));
            return;
        }
        let Some((frame, s)) = &self.slice else {
            ui.label(caption("evaluating…"));
            return;
        };
        let n = s.res;
        if self.tex.as_ref().map(|t| t.0) != Some(frame.seq) {
            let complex = matches!(s.which, Which::Mixed(_));
            // f panel: robust scale from the high-density region
            let mut mags: Vec<f32> = (0..n * n).filter(|&i| s.density[i] > 0.05).map(|i| s.f_re[i].hypot(s.f_im[i])).collect();
            mags.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let fmax = mags.get(mags.len() * 95 / 100).copied().unwrap_or(1.0).max(1e-9);
            let mut img_f = Vec::with_capacity(n * n);
            let mut img_d = Vec::with_capacity(n * n);
            let mut dvals: Vec<f32> = (0..n * n).map(|i| (s.f_re[i] * s.f_re[i] + s.f_im[i] * s.f_im[i]) * s.density[i]).collect();
            let dmax = dvals.iter().cloned().fold(1e-30, f32::max);
            for v in &mut dvals {
                *v /= dmax;
            }
            let band: Vec<i32> = s.density.iter().map(|d| if *d > 1e-4 { (-2.0 * d.ln()).floor() as i32 } else { 99 }).collect();
            for i in 0..n * n {
                let d0 = s.density[i];
                let (x, y) = (i % n, i / n);
                let b = band[i];
                let contour = b < 9 && ((x + 1 < n && band[i + 1] != b) || (y + 1 < n && band[i + n] != b));
                let weight = (d0.powf(0.25)).clamp(0.0, 1.0);
                let c = if complex {
                    let mag = (s.f_re[i].hypot(s.f_im[i]) / fmax).min(1.0);
                    let hue = (s.f_im[i].atan2(s.f_re[i]) / std::f32::consts::TAU).rem_euclid(1.0);
                    let col: Color32 = egui::ecolor::Hsva::new(hue, 0.72, (0.25 + 0.75 * mag) * (0.35 + 0.65 * weight), 1.0).into();
                    col
                } else {
                    let v = (s.f_re[i] / fmax).clamp(-1.0, 1.0);
                    let base = lerp(GROUND, if v >= 0.0 { HELIKE } else { RIVAL }, v.abs().powf(0.7));
                    lerp(GROUND, base, 0.35 + 0.65 * weight)
                };
                img_f.push(if contour { lerp(c, INK, 0.55) } else { c });
                let dv = dvals[i].powf(0.6);
                let dc = lerp(GROUND, Color32::from_rgb(170, 214, 246), dv);
                img_d.push(if contour { lerp(dc, MUTED, 0.6) } else { dc });
            }
            let tf = ui.ctx().load_texture("state-f", egui::ColorImage::new([n, n], img_f), egui::TextureOptions::LINEAR);
            let td = ui.ctx().load_texture("state-d", egui::ColorImage::new([n, n], img_d), egui::TextureOptions::LINEAR);
            self.tex = Some((frame.seq, tf, td));
        }
        let (_, tf, td) = self.tex.as_ref().unwrap();
        ui.add_space(8.0);
        let avail = ui.available_size();
        let side = ((avail.x - 56.0) / 2.0).min(avail.y - 114.0).max(200.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(avail.x, side + 104.0), Sense::hover());
        let p = ui.painter_at(rect);
        let a_name = ROLES[self.axis_a];
        let b_name = ROLES[self.axis_b];
        let what = match s.which {
            Which::Mixed(1) => "Helike, charge +2".to_string(),
            Which::Mixed(2) => "Helike, charge −2".to_string(),
            Which::Mixed(_) => "the competitor".to_string(),
            Which::Level(l) => format!("Ritz level n = {l}"),
        };
        for (k, (tex, title)) in [(tf, format!("f = ψ/ψ₀ · {what}")), (td, format!("|ψ|² · {what}"))].iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(rect.left() + k as f32 * (side + 40.0), rect.top() + 22.0), Vec2::splat(side));
            p.text(Pos2::new(r.left(), rect.top()), Align2::LEFT_TOP, title, FontId::monospace(10.5), if k == 0 { INK } else { MUTED });
            p.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            p.rect_stroke(r, CornerRadius::same(4), Stroke::new(1.0, HAIRLINE), StrokeKind::Outside);
            // axes through the origin, and the unit circle the winding is counted on
            let to = |x: f64, y: f64| Pos2::new(r.left() + ((x + s.extent) / (2.0 * s.extent)) as f32 * r.width(), r.bottom() - ((y + s.extent) / (2.0 * s.extent)) as f32 * r.height());
            p.line_segment([to(-s.extent, 0.0), to(s.extent, 0.0)], Stroke::new(1.0, alpha(INK, 0.18)));
            p.line_segment([to(0.0, -s.extent), to(0.0, s.extent)], Stroke::new(1.0, alpha(INK, 0.18)));
            if k == 0 {
                let rad = (to(1.0, 0.0).x - to(0.0, 0.0).x).abs();
                p.circle_stroke(to(0.0, 0.0), rad, Stroke::new(1.0, alpha(INK, 0.45)));
            }
            p.text(Pos2::new(r.right(), r.bottom() + 4.0), Align2::RIGHT_TOP, format!("z_{a_name} →"), FontId::monospace(9.5), FAINT);
            p.text(Pos2::new(r.left() - 4.0, r.top()), Align2::RIGHT_TOP, format!("z_{b_name}"), FontId::monospace(9.5), FAINT);
            for t in [-2.0, 0.0, 2.0] {
                p.text(Pos2::new(to(t, 0.0).x, r.bottom() + 4.0), Align2::CENTER_TOP, format!("{t}"), FontId::monospace(9.0), FAINT);
            }
        }
        let mut info = format!("g² {} · Mq {} · plane z_{a_name} × z_{b_name}, others at {:+.2}", s.g2, s.mq, self.offset);
        if let Some(e) = s.energy {
            info.push_str(&format!(" · E − E₀ = {e:.5}"));
        }
        let ih = crate::theme::wrapped(&p, Pos2::new(rect.left(), rect.bottom() - 58.0), false, info, FontId::monospace(10.0), MUTED, rect.width());
        let mut wind = format!("winding of f on the unit circle of this plane: {:+.3}", s.winding);
        if let (Some(c), Which::Mixed(1 | 2)) = (s.charges, s.which) {
            wind.push_str(&format!("
export's charge compression on the pair: {:+.6}, {:+.6}", c[0], c[1]));
        }
        crate::theme::wrapped(&p, Pos2::new(rect.left(), rect.bottom() - 54.0 + ih), false, wind, FontId::monospace(10.5), if matches!(s.which, Which::Mixed(1) | Which::Mixed(2)) { HELIKE } else { INK }, rect.width());
    }
}

fn lerp(a: Color32, b: Color32, s: f32) -> Color32 {
    let s = s.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * s).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}
