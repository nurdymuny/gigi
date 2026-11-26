//! The Controls scene: a board of suites × control points, a tile detail, the prediction board and the amendment
//! history. It draws suites as the solver streams them and decides nothing.

use crate::controls::{self, Check, HistoryEntry, Mark, Outcome, Suite, Tile, COLUMNS};
use crate::theme::*;
use eframe::egui::{
    self, Align2, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2,
};
use helike_stream::Frame;

pub struct ControlsView {
    pub suites: Vec<(Frame, Suite)>,
    pub history: Vec<HistoryEntry>,
    pub root: Option<std::path::PathBuf>,
    selected: Option<(String, usize)>,
    hidden: Vec<String>,
    pub live: bool,
    /// --shot controls-lower: start scrolled to the end (detail, predictions, history)
    pub start_at_bottom: bool,
}

impl ControlsView {
    /// Open a tile's detail (used by the M8 screenshot).
    pub fn select(&mut self, key: &str, col_index: usize) {
        self.selected = Some((key.into(), col_index));
    }

    pub fn new(root: Option<std::path::PathBuf>) -> Self {
        Self {
            suites: vec![],
            history: vec![],
            root,
            selected: Some(("domain".into(), 3)),
            hidden: vec![],
            live: true,
            start_at_bottom: false,
        }
    }

    pub fn accept(&mut self, f: Frame) {
        match f.scene.as_str() {
            "controls" => {
                let Ok(s) = serde_json::from_value::<Suite>(f.data.clone()) else {
                    return;
                };
                if !self.live && self.suites.iter().any(|(_, x)| x.key == s.key) {
                    return; // paused: keep what is on screen
                }
                match self.suites.iter_mut().find(|(_, x)| x.key == s.key) {
                    Some(slot) => *slot = (f, s),
                    None => self.suites.push((f, s)),
                }
            }
            "controls-history" => {
                if let Ok(h) = serde_json::from_value(f.data) {
                    self.history = h;
                }
            }
            _ => {}
        }
    }

    pub fn ready(&self) -> bool {
        self.suites.len() >= 5
    }

    pub fn side_controls(&mut self, ui: &mut egui::Ui) {
        ui.label(eyebrow("Suites"));
        for (_, s) in &self.suites {
            let mut on = !self.hidden.contains(&s.key);
            if ui.checkbox(&mut on, &s.name).changed() {
                if on {
                    self.hidden.retain(|k| k != &s.key);
                } else {
                    self.hidden.push(s.key.clone());
                }
            }
        }
        ui.add_space(10.0);
        ui.label(eyebrow("Live"));
        ui.checkbox(&mut self.live, "follow receipts and progress files");
        ui.label(caption(
            "Rescans once a second; a changed file redraws its row.",
        ));
        ui.add_space(10.0);
        ui.label(eyebrow("Reading the board"));
        legend_row(ui, Outcome::Pass, "pass · resolved · invariant");
        legend_row(ui, Outcome::Fail, "fail · moved");
        legend_row(ui, Outcome::Open, "open (never green, never red)");
        legend_row(ui, Outcome::Running, "running");
        legend_row(ui, Outcome::Queued, "queued / report pending");
        ui.add_space(4.0);
        ui.label(caption("Gauges: value ÷ tolerance on a log scale; left of the line is inside. Grey: reported, not decided."));
        if let Some(r) = &self.root {
            ui.add_space(10.0);
            ui.label(eyebrow("Root"));
            ui.label(caption(r.display().to_string()));
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Registered controls").size(19.0).color(INK));
                ui.add_space(8.0);
                ui.label(caption("what each receipt says, beside what was predicted before it ran"));
            });
            if self.root.is_none() {
                ui.add_space(12.0);
                ui.label(RichText::new("No repository root found above the working directory (looked for research/ and docs/research/). Start the viewer from inside the repository.").color(RIVAL));
                return;
            }
            if self.suites.is_empty() {
                ui.label(caption("reading receipts…"));
                return;
            }
            ui.add_space(10.0);
            // below its minimum width the board scrolls sideways rather than letting columns overlap
            egui::ScrollArea::horizontal().id_salt("board").show(ui, |ui| self.board(ui));
            ui.add_space(18.0);
            let w = ui.available_width();
            if w > 980.0 {
                ui.horizontal_top(|ui| {
                    let down = egui::Layout::top_down(egui::Align::LEFT);
                    ui.allocate_ui_with_layout(Vec2::new(w * 0.6 - 30.0, 0.0), down, |ui| self.detail(ui));
                    ui.add_space(20.0);
                    ui.allocate_ui_with_layout(Vec2::new(w * 0.4 - 40.0, 0.0), down, |ui| self.predictions(ui));
                });
            } else {
                self.detail(ui);
                ui.add_space(18.0);
                self.predictions(ui);
            }
            ui.add_space(18.0);
            self.history_strip(ui);
            ui.add_space(12.0);
            if self.start_at_bottom {
                ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
            }
        });
    }

    fn board(&mut self, ui: &mut egui::Ui) {
        let label_w = 190.0;
        let verdict_w = 150.0;
        let gap = 8.0;
        let min_col = 128.0;
        let width = ui.available_width().max(label_w + verdict_w + gap * 6.0 + 5.0 * min_col);
        let col_w = (width - label_w - verdict_w - gap * 6.0) / 5.0;
        // column heads
        let (head, _) = ui.allocate_exact_size(Vec2::new(width, 18.0), Sense::hover());
        let p = ui.painter();
        for (i, c) in COLUMNS.iter().enumerate() {
            let x = head.left() + label_w + gap + i as f32 * (col_w + gap);
            p.text(
                Pos2::new(x + 2.0, head.center().y),
                Align2::LEFT_CENTER,
                *c,
                FontId::monospace(10.0),
                MUTED,
            );
        }
        p.text(
            Pos2::new(head.right() - verdict_w + 6.0, head.center().y),
            Align2::LEFT_CENTER,
            "SUITE VERDICT",
            FontId::monospace(9.5),
            MUTED,
        );
        let time = ui.input(|i| i.time);
        let mut clicked: Option<(String, usize)> = None;
        let mut any_running = false;
        for (frame, s) in self
            .suites
            .iter()
            .filter(|(_, s)| !self.hidden.contains(&s.key))
        {
            let has_sup = !s.superseded.is_empty();
            let row_h = if has_sup { 142.0 } else { 116.0 };
            let (row, _) = ui.allocate_exact_size(Vec2::new(width, row_h), Sense::hover());
            let p = ui.painter().clone();
            p.line_segment(
                [row.left_top(), row.right_top()],
                Stroke::new(1.0, HAIRLINE),
            );
            // suite label
            let lx = row.left() + 2.0;
            p.text(
                Pos2::new(lx, row.top() + 14.0),
                Align2::LEFT_TOP,
                &s.name,
                FontId::proportional(14.5),
                INK,
            );
            p.text(
                Pos2::new(lx, row.top() + 36.0),
                Align2::LEFT_TOP,
                wrap(&s.registration, 30),
                FontId::monospace(10.0),
                MUTED,
            );
            let chip = if s.tiles.is_empty() {
                "no receipt yet"
            } else {
                match frame.status {
                    helike_stream::Status::IntervalEnclosure => "INTERVAL_ENCLOSURE",
                    helike_stream::Status::Computed => "COMPUTED · from receipt",
                    _ => "",
                }
            };
            p.text(
                Pos2::new(lx, row.bottom() - 10.0),
                Align2::LEFT_BOTTOM,
                chip,
                FontId::monospace(9.0),
                FAINT,
            );
            let cells_top = row.top() + 10.0;
            let tile_h = 96.0;
            if let Some(w) = &s.warning {
                // a missing or mismatched receipt is visible: amber across the empty columns
                let x0 = row.left() + label_w + gap;
                let r = Rect::from_min_size(
                    Pos2::new(x0, cells_top),
                    Vec2::new(
                        5.0 * col_w + 4.0 * gap,
                        if s.tiles.is_empty() { tile_h } else { 18.0 },
                    ),
                );
                if s.tiles.is_empty() {
                    dashed_rect(&p, r, alpha(RIVAL, 0.55));
                    p.text(
                        Pos2::new(r.left() + 14.0, r.center().y - 8.0),
                        Align2::LEFT_CENTER,
                        w,
                        FontId::proportional(12.5),
                        RIVAL,
                    );
                    p.text(
                        Pos2::new(r.left() + 14.0, r.center().y + 10.0),
                        Align2::LEFT_CENTER,
                        format!("expected at {}", s.source),
                        FontId::monospace(10.0),
                        alpha(RIVAL, 0.75),
                    );
                } else {
                    p.text(
                        Pos2::new(r.left(), row.bottom() - 4.0),
                        Align2::LEFT_BOTTOM,
                        format!("⚠ {w}"),
                        FontId::monospace(10.0),
                        RIVAL,
                    );
                }
            }
            for (k, t) in s.tiles.iter().enumerate() {
                let x0 = row.left() + label_w + gap + t.col as f32 * (col_w + gap);
                let r = Rect::from_min_size(
                    Pos2::new(x0, cells_top),
                    Vec2::new(t.span as f32 * col_w + (t.span - 1) as f32 * gap, tile_h),
                );
                let resp = ui.interact(r, ui.id().with((&s.key, k)), Sense::click());
                let sel = self
                    .selected
                    .as_ref()
                    .is_some_and(|(kk, i)| kk == &s.key && *i == k);
                draw_tile(&p, r, t, sel, resp.hovered(), time);
                if t.outcome == Outcome::Running {
                    any_running = true;
                }
                if resp.clicked() {
                    clicked = Some((s.key.clone(), k));
                }
                resp.on_hover_ui(|ui| tile_hover(ui, t));
            }
            for t in &s.superseded {
                let x0 = row.left() + label_w + gap + t.col as f32 * (col_w + gap);
                let r = Rect::from_min_size(
                    Pos2::new(x0, cells_top + tile_h + 6.0),
                    Vec2::new(col_w, 22.0),
                );
                draw_superseded(&p, r, t);
            }
            // suite verdict
            let vr = Rect::from_min_size(
                Pos2::new(row.right() - verdict_w + 4.0, cells_top),
                Vec2::new(verdict_w - 4.0, tile_h),
            );
            let col = outcome_colour(s.verdict_outcome);
            let running = s
                .tiles
                .iter()
                .filter(|t| t.outcome == Outcome::Running)
                .count();
            let text = if running > 0 {
                format!("{} · {running} running", s.verdict)
            } else {
                s.verdict.clone()
            };
            p.text(
                Pos2::new(vr.left() + 6.0, vr.top() + 10.0),
                Align2::LEFT_TOP,
                wrap(&text, 18),
                FontId::monospace(11.5),
                if s.warning.is_some() && s.tiles.is_empty() {
                    RIVAL
                } else {
                    col
                },
            );
            p.text(
                Pos2::new(vr.left() + 6.0, vr.bottom() - 4.0),
                Align2::LEFT_BOTTOM,
                tail(&s.source, ((verdict_w - 12.0) / 5.3) as usize),
                FontId::monospace(8.5),
                FAINT,
            );
        }
        if let Some(c) = clicked {
            self.selected = Some(c);
        }
        if any_running {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(250));
        }
    }

    fn detail(&mut self, ui: &mut egui::Ui) {
        ui.label(eyebrow("Tile detail"));
        let Some((key, k)) = self.selected.clone() else {
            ui.label(caption("Click a tile."));
            return;
        };
        let Some((_, s)) = self.suites.iter().find(|(_, s)| s.key == key) else {
            return;
        };
        let Some(t) = s.tiles.get(k) else {
            ui.label(caption("Click a tile."));
            return;
        };
        egui::Frame::new()
            .fill(RAISED)
            .stroke(Stroke::new(1.0, HAIRLINE))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&s.name).size(15.0));
                    ui.label(caption(COLUMNS[t.col..t.col + t.span].join(" + ")));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        pill(ui, &t.word, outcome_colour(t.outcome));
                    });
                });
                let mut meta = vec![];
                if let Some(c) = &t.commit {
                    meta.push(format!("commit {c}"));
                }
                if let Some(g) = &t.tag {
                    meta.push(g.clone());
                }
                meta.push(s.source.clone());
                ui.label(caption(meta.join("  ·  ")));
                if let Some(pr) = &t.progress {
                    ui.label(caption(format!(
                        "progress: {}{}",
                        pr.stage,
                        pr.t.map(|t0| format!(" · {} ago", elapsed(controls::now_unix() - t0)))
                            .unwrap_or_default()
                    )));
                }
                ui.add_space(8.0);
                if !t.heat.is_empty() {
                    heat_panel(ui, t);
                    ui.add_space(10.0);
                }
                // what this test does, in plain English
                let about = controls::about_suite(&s.key);
                if !about.is_empty() {
                    ui.label(eyebrow("What this test does"));
                    ui.label(RichText::new(about).size(12.5).color(INK));
                    ui.add_space(6.0);
                    let mut seen: Vec<&str> = vec![];
                    egui::Grid::new(("plain", &key, k)).num_columns(2).spacing(Vec2::new(14.0, 4.0)).show(ui, |ui| {
                        for c in &t.checks {
                            if seen.contains(&c.name.as_str()) {
                                continue;
                            }
                            seen.push(&c.name);
                            let line = controls::about_check(&c.name);
                            if line.is_empty() {
                                continue;
                            }
                            ui.label(RichText::new(&c.name).monospace().size(10.5).color(if c.reported_only { FAINT } else { MUTED }));
                            ui.add(egui::Label::new(RichText::new(line).size(12.0).color(INK)).wrap());
                            ui.end_row();
                        }
                    });
                    ui.add_space(10.0);
                }
                if let Some(m8) = &t.m8 {
                    m8_panels(ui, m8);
                    ui.add_space(10.0);
                }
                for (ti, tab) in t.tables.iter().enumerate() {
                    ui.label(eyebrow(&tab.title));
                    egui::ScrollArea::horizontal()
                        .id_salt(("tab", ti))
                        .show(ui, |ui| {
                            egui::Grid::new(("grid", &key, k, ti))
                                .striped(true)
                                .spacing(Vec2::new(14.0, 6.0))
                                .show(ui, |ui| {
                                    for (ci, c) in tab.columns.iter().enumerate() {
                                        let rep = tab.reported_columns.contains(&ci);
                                        ui.vertical(|ui| {
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(c)
                                                        .monospace()
                                                        .size(10.5)
                                                        .color(if rep { FAINT } else { MUTED }),
                                                )
                                                .wrap_mode(egui::TextWrapMode::Extend),
                                            );
                                            if rep {
                                                ui.add(
                                                    egui::Label::new(
                                                        RichText::new("reported,\nnot decided")
                                                            .monospace()
                                                            .size(9.0)
                                                            .color(FAINT)
                                                            .italics(),
                                                    )
                                                    .wrap_mode(egui::TextWrapMode::Extend),
                                                );
                                            }
                                        });
                                    }
                                    ui.end_row();
                                    for row in &tab.rows {
                                        for (ci, c) in row.iter().enumerate() {
                                            let rep = tab.reported_columns.contains(&ci);
                                            let col = if rep {
                                                FAINT
                                            } else {
                                                c.outcome.map(outcome_colour).unwrap_or(INK)
                                            };
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(&c.text)
                                                        .monospace()
                                                        .size(11.5)
                                                        .color(col),
                                                )
                                                .wrap_mode(egui::TextWrapMode::Extend),
                                            );
                                        }
                                        ui.end_row();
                                    }
                                });
                        });
                    if let Some(n) = &tab.note {
                        ui.label(caption(n.clone()));
                    }
                    ui.add_space(8.0);
                }
            });
    }

    fn predictions(&self, ui: &mut egui::Ui) {
        ui.label(eyebrow("Prediction board"));
        egui::Frame::new().fill(RAISED).stroke(Stroke::new(1.0, HAIRLINE)).corner_radius(CornerRadius::same(8)).inner_margin(egui::Margin::same(14)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            let mut any = false;
            for (_, s) in self.suites.iter().filter(|(_, s)| !self.hidden.contains(&s.key)) {
                if s.predictions.is_empty() {
                    continue;
                }
                any = true;
                ui.label(RichText::new(&s.name).size(13.0).color(INK));
                for pr in &s.predictions {
                    let (mark, col) = match pr.mark {
                        Mark::Met => ("met", PROVED),
                        Mark::Missed => ("missed", RIVAL),
                        Mark::NotYetRun => ("not yet", MUTED),
                    };
                    ui.horizontal_top(|ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::new(54.0, 18.0), Sense::hover());
                        ui.painter().rect(r, CornerRadius::same(9), alpha(col, 0.12), Stroke::new(1.0, alpha(col, 0.5)), StrokeKind::Inside);
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, mark, FontId::monospace(10.0), col);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(format!("“{}”", pr.quote)).size(12.0).color(INK));
                            ui.label(caption(format!("{}  →  {}", pr.source, pr.outcome_text)));
                        });
                    });
                    ui.add_space(2.0);
                }
                ui.add_space(6.0);
            }
            if !any {
                ui.label(caption("No predictions.json found."));
            }
            ui.label(caption("A missed prediction is information about the hypothesis, not a failure of the run."));
        });
    }

    fn history_strip(&self, ui: &mut egui::Ui) {
        ui.label(eyebrow("Amendment history"));
        if self.history.is_empty() {
            ui.label(caption("research/controls-history.json not found."));
            return;
        }
        // cards wrap onto new rows rather than running off the panel edge
        egui::ScrollArea::vertical()
            .id_salt("history")
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(14.0, 12.0);
                    for h in self.history.iter() {
                        egui::Frame::new()
                            .fill(RAISED)
                            .stroke(Stroke::new(1.0, HAIRLINE))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.set_width(220.0);
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(&h.id)
                                                .monospace()
                                                .size(12.0)
                                                .color(HELIKE),
                                        );
                                        ui.label(caption(&h.date));
                                    });
                                    ui.label(RichText::new(&h.title).size(11.5));
                                    let mut tags: Vec<String> =
                                        h.changed.iter().map(|k| self.suite_name(k)).collect();
                                    if !h.supersedes.is_empty() {
                                        tags.push(format!(
                                            "{} tile(s) struck through",
                                            h.supersedes.len()
                                        ));
                                    }
                                    ui.label(caption(tags.join(" · ")));
                                });
                            });
                    }
                });
            });
    }

    fn suite_name(&self, key: &str) -> String {
        self.suites
            .iter()
            .find(|(_, s)| s.key == key)
            .map(|(_, s)| s.name.clone())
            .unwrap_or_else(|| key.into())
    }
}

// ------------------------------------------------------------------------------------------------ drawing

pub fn outcome_colour(o: Outcome) -> egui::Color32 {
    match o {
        Outcome::Pass => PROVED,
        Outcome::Fail => FAIL,
        Outcome::Open => OPEN,
        Outcome::Running => HELIKE,
        Outcome::Queued | Outcome::Reported | Outcome::NotApplicable => MUTED,
    }
}

fn legend_row(ui: &mut egui::Ui, o: Outcome, text: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::new(26.0, 14.0), Sense::hover());
        tile_frame(ui.painter(), r, o, false, false, ui.input(|i| i.time));
        ui.label(RichText::new(text).size(11.5).color(MUTED));
    });
}

fn hatch(p: &egui::Painter, r: Rect, col: egui::Color32) {
    let p = p.with_clip_rect(r);
    let step = 7.0;
    let mut x = r.left() - r.height();
    while x < r.right() {
        p.line_segment(
            [Pos2::new(x, r.bottom()), Pos2::new(x + r.height(), r.top())],
            Stroke::new(1.0, col),
        );
        x += step;
    }
}

fn dashed_rect(p: &egui::Painter, r: Rect, col: egui::Color32) {
    let pts = [
        r.left_top(),
        r.right_top(),
        r.right_bottom(),
        r.left_bottom(),
        r.left_top(),
    ];
    for w in pts.windows(2) {
        p.extend(egui::Shape::dashed_line(
            &[w[0], w[1]],
            Stroke::new(1.0, col),
            5.0,
            4.0,
        ));
    }
}

/// The three looks of §1: PASS a green wash with a green edge; FAIL a red outline; OPEN its own neutral hatch.
fn tile_frame(p: &egui::Painter, r: Rect, o: Outcome, selected: bool, hovered: bool, time: f64) {
    let cr = CornerRadius::same(6);
    match o {
        Outcome::Pass => {
            p.rect(
                r,
                cr,
                alpha(PROVED, 0.09),
                Stroke::new(1.0, alpha(PROVED, 0.55)),
                StrokeKind::Inside,
            );
        }
        Outcome::Fail => {
            p.rect(r, cr, RAISED, Stroke::new(1.8, FAIL), StrokeKind::Inside);
        }
        Outcome::Open => {
            p.rect_filled(r, cr, RAISED);
            hatch(p, r.shrink(1.0), alpha(OPEN, 0.22));
            p.rect_stroke(
                r,
                cr,
                Stroke::new(1.0, alpha(OPEN, 0.7)),
                StrokeKind::Inside,
            );
        }
        Outcome::Running => {
            let a = 0.35 + 0.25 * (time * 3.0).sin() as f32;
            p.rect(
                r,
                cr,
                alpha(HELIKE, 0.06),
                Stroke::new(1.2, alpha(HELIKE, a + 0.2)),
                StrokeKind::Inside,
            );
        }
        _ => {
            p.rect_filled(r, cr, SURFACE);
            dashed_rect(p, r.shrink(0.5), alpha(MUTED, 0.6));
        }
    }
    if selected {
        p.rect_stroke(
            r.expand(3.0),
            CornerRadius::same(8),
            Stroke::new(1.5, HELIKE),
            StrokeKind::Outside,
        );
    } else if hovered {
        p.rect_stroke(
            r.expand(2.0),
            CornerRadius::same(7),
            Stroke::new(1.0, alpha(INK, 0.35)),
            StrokeKind::Outside,
        );
    }
}

fn draw_tile(p: &egui::Painter, r: Rect, t: &Tile, selected: bool, hovered: bool, time: f64) {
    tile_frame(p, r, t.outcome, selected, hovered, time);
    let col = outcome_colour(t.outcome);
    let inner = r.shrink2(Vec2::new(10.0, 8.0));
    let corner = t.commit.as_ref().or(t.tag.as_ref());
    let corner_w = corner
        .map(|c| c.chars().count() as f32 * 5.5 + 10.0)
        .unwrap_or(0.0);
    p.text(
        inner.left_top(),
        Align2::LEFT_TOP,
        ellipsize(&t.word, ((inner.width() - corner_w) / 6.4) as usize),
        FontId::monospace(10.5),
        col,
    );
    if let Some(c) = corner {
        p.text(
            inner.right_top(),
            Align2::RIGHT_TOP,
            c,
            FontId::monospace(9.0),
            FAINT,
        );
    }
    if t.outcome == Outcome::Running {
        let a = 0.5 + 0.5 * (time * 4.0).sin() as f32;
        p.circle_filled(
            Pos2::new(inner.left() + 4.0, inner.center().y),
            3.5,
            alpha(HELIKE, a),
        );
        if let Some(pr) = &t.progress {
            p.text(
                Pos2::new(inner.left() + 14.0, inner.center().y),
                Align2::LEFT_CENTER,
                ellipsize(&pr.stage, 26),
                FontId::monospace(10.5),
                INK,
            );
            if let Some(t0) = pr.t {
                p.text(
                    inner.left_bottom(),
                    Align2::LEFT_BOTTOM,
                    format!("{} elapsed", elapsed(controls::now_unix() - t0)),
                    FontId::monospace(9.5),
                    MUTED,
                );
            }
        }
        return;
    }
    // mini-gauges for numeric checks against a tolerance, decided ones first
    let mut gauges: Vec<&Check> = t
        .checks
        .iter()
        .filter(|c| c.value.is_some() && c.tolerance.is_some())
        .collect();
    gauges.sort_by_key(|c| c.reported_only);
    let gauges = dedup_by_name(gauges);
    let top = inner.top() + 18.0;
    let rows = gauges.len().min(3);
    for (i, c) in gauges.iter().take(rows).enumerate() {
        let y = top + i as f32 * 17.0;
        let label_w = (inner.width() * 0.5).min(112.0);
        p.text(
            Pos2::new(inner.left(), y + 6.0),
            Align2::LEFT_CENTER,
            ellipsize(&c.name, (label_w / 5.8) as usize),
            FontId::monospace(9.5),
            if c.reported_only { FAINT } else { MUTED },
        );
        let g = Rect::from_min_size(
            Pos2::new(inner.left() + label_w, y + 2.0),
            Vec2::new(inner.width() - label_w, 8.0),
        );
        gauge(p, g, c);
    }
    // the rest as a count: how many checks this tile holds and how many hold
    let decided: Vec<&Check> = t
        .checks
        .iter()
        .filter(|c| matches!(c.outcome, Outcome::Pass | Outcome::Fail | Outcome::Open))
        .collect();
    let pass = decided
        .iter()
        .filter(|c| c.outcome == Outcome::Pass)
        .count();
    let reported = t.checks.iter().filter(|c| c.reported_only).count();
    let mut foot = format!("{pass}/{} hold", decided.len());
    if reported > 0 {
        foot.push_str(&format!(" · {reported} reported"));
    }
    p.text(
        inner.left_bottom(),
        Align2::LEFT_BOTTOM,
        ellipsize(&foot, (inner.width() / 5.8) as usize),
        FontId::monospace(9.5),
        MUTED,
    );
    if let Some(sup) = &t.superseded_by {
        p.text(
            inner.right_bottom(),
            Align2::RIGHT_BOTTOM,
            sup,
            FontId::monospace(9.5),
            RIVAL,
        );
    }
}

fn dedup_by_name(v: Vec<&Check>) -> Vec<&Check> {
    // one gauge per check name: the one furthest toward its tolerance
    let mut out: Vec<&Check> = vec![];
    for c in v {
        let ratio = |c: &Check| c.value.unwrap().abs() / c.tolerance.unwrap();
        match out.iter_mut().find(|o| o.name == c.name) {
            Some(o) => {
                if ratio(c) > ratio(o) {
                    *o = c;
                }
            }
            None => out.push(c),
        }
    }
    out
}

/// value ÷ tolerance on a log scale from 10⁻⁸ to 10¹; the vertical line is the tolerance.
fn gauge(p: &egui::Painter, r: Rect, c: &Check) {
    let (v, tol) = (c.value.unwrap().abs(), c.tolerance.unwrap());
    let (lo, hi) = (-8.0f32, 1.0f32);
    let x_of = |e: f32| r.left() + (e - lo) / (hi - lo) * r.width();
    let e = if v > 0.0 {
        ((v / tol).log10() as f32).clamp(lo, hi)
    } else {
        lo
    };
    p.rect_filled(r, CornerRadius::same(4), alpha(HAIRLINE, 0.9));
    let col = if c.reported_only {
        FAINT
    } else {
        outcome_colour(c.outcome)
    };
    let fill = Rect::from_min_max(
        r.left_top(),
        Pos2::new(x_of(e).max(r.left() + 3.0), r.bottom()),
    );
    p.rect_filled(fill, CornerRadius::same(4), alpha(col, 0.85));
    let x1 = x_of(0.0);
    p.line_segment(
        [
            Pos2::new(x1, r.top() - 2.0),
            Pos2::new(x1, r.bottom() + 2.0),
        ],
        Stroke::new(1.2, INK),
    );
    if v > 0.0 && ((v / tol).log10() as f32) < lo {
        p.text(
            Pos2::new(r.left() + 5.0, r.center().y),
            Align2::LEFT_CENTER,
            "≪",
            FontId::monospace(8.0),
            GROUND,
        );
    }
}

fn draw_superseded(p: &egui::Painter, r: Rect, t: &Tile) {
    p.rect(
        r,
        CornerRadius::same(4),
        alpha(RAISED, 0.6),
        Stroke::new(1.0, alpha(RIVAL, 0.45)),
        StrokeKind::Inside,
    );
    let text = format!("{} · {}", t.superseded_by.as_deref().unwrap_or(""), t.word);
    let g = p.layout_no_wrap(
        ellipsize(&text, (r.width() / 6.2) as usize),
        FontId::monospace(9.5),
        alpha(RIVAL, 0.85),
    );
    let pos = Pos2::new(r.left() + 8.0, r.center().y - g.size().y / 2.0);
    let w = g.size().x;
    p.galley(pos, g, RIVAL);
    p.line_segment(
        [
            Pos2::new(pos.x - 2.0, r.center().y),
            Pos2::new(pos.x + w + 2.0, r.center().y),
        ],
        Stroke::new(1.0, alpha(RIVAL, 0.85)),
    );
}

fn tile_hover(ui: &mut egui::Ui, t: &Tile) {
    ui.set_max_width(380.0);
    ui.label(
        RichText::new(&t.word)
            .monospace()
            .color(outcome_colour(t.outcome)),
    );
    if let Some(s) = &t.superseded_by {
        ui.label(RichText::new(format!("superseded by {s}")).color(RIVAL));
    }
    egui::Grid::new("hover")
        .spacing(Vec2::new(12.0, 3.0))
        .show(ui, |ui| {
            for c in &t.checks {
                let col = if c.reported_only {
                    FAINT
                } else {
                    outcome_colour(c.outcome)
                };
                ui.label(
                    RichText::new(format!(
                        "{}{}",
                        c.name,
                        c.group
                            .as_ref()
                            .map(|g| format!(" · {g}"))
                            .unwrap_or_default()
                    ))
                    .size(11.0)
                    .color(MUTED),
                );
                let v = match (c.value, &c.text) {
                    (Some(v), _) => controls::sci(v),
                    (None, Some(t)) => t.clone(),
                    _ => controls::oword(c.outcome),
                };
                let tol = c
                    .tolerance
                    .map(|t| format!("  / {}", controls::sci(t)))
                    .unwrap_or_default();
                ui.label(
                    RichText::new(format!(
                        "{}{tol}{}",
                        ellipsize(&v, 40),
                        if c.reported_only { "  (reported)" } else { "" }
                    ))
                    .monospace()
                    .size(11.0)
                    .color(col),
                );
                ui.end_row();
            }
        });
    ui.label(caption("click for the detail"));
}

/// Heat signal: per sector, each source as a row; a dot at the corridor energy, the missing weight as a hollow tail.
fn heat_panel(ui: &mut egui::Ui, t: &Tile) {
    ui.label(eyebrow("Corridor: every source reaches one note"));
    for hs in &t.heat {
        let n = hs.sources.len().max(1) as f32;
        let h = 22.0 + n * 18.0;
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
        let p = ui.painter();
        let accent = if hs.sector == "helike" { HELIKE } else { RIVAL };
        p.text(
            r.left_top(),
            Align2::LEFT_TOP,
            format!(
                "{} · corridor level {} · {}",
                hs.sector,
                hs.corridor_level
                    .map(|l| l.to_string())
                    .unwrap_or("—".into()),
                controls::oword(hs.verdict)
            ),
            FontId::monospace(10.5),
            accent,
        );
        let name_w = 48.0;
        let bar = Rect::from_min_max(
            Pos2::new(r.left() + name_w, r.top() + 20.0),
            Pos2::new(r.right() - 150.0, r.bottom()),
        );
        for (i, (src, e, miss)) in hs.sources.iter().enumerate() {
            let y = bar.top() + 9.0 + i as f32 * 18.0;
            p.text(
                Pos2::new(r.left(), y),
                Align2::LEFT_CENTER,
                src,
                FontId::monospace(10.5),
                MUTED,
            );
            p.line_segment(
                [Pos2::new(bar.left(), y), Pos2::new(bar.right(), y)],
                Stroke::new(1.0, HAIRLINE),
            );
            // weight reaching the corridor (1 − missing) filled, missing weight as a hollow tail
            let reach = (1.0 - *miss).clamp(0.0, 1.0) as f32;
            let xr = bar.left() + reach * bar.width();
            p.line_segment(
                [Pos2::new(bar.left(), y), Pos2::new(xr, y)],
                Stroke::new(3.0, alpha(accent, 0.8)),
            );
            if *miss > 0.0 {
                p.rect_stroke(
                    Rect::from_min_max(Pos2::new(xr, y - 3.0), Pos2::new(bar.right(), y + 3.0)),
                    CornerRadius::same(3),
                    Stroke::new(1.0, alpha(accent, 0.45)),
                    StrokeKind::Inside,
                );
            }
            p.circle_filled(Pos2::new(xr, y), 4.0, accent);
            p.text(
                Pos2::new(bar.right() + 10.0, y),
                Align2::LEFT_CENTER,
                format!("E {:.6} · miss {}", e, controls::sci(*miss)),
                FontId::monospace(10.0),
                MUTED,
            );
        }
    }
    ui.label(caption("Bar: weight reaching the corridor level (filled) and the missing weight (hollow). Energies are the plateau m(∞) from the receipt."));
}

fn ellipsize(s: &str, n: usize) -> String {
    if s.chars().count() <= n.max(4) {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n.max(4) - 1).collect::<String>())
    }
}

/// The end of a long path, with an ellipsis in front.
fn tail(s: &str, n: usize) -> String {
    let c = s.chars().count();
    if c <= n.max(4) {
        s.to_string()
    } else {
        format!(
            "…{}",
            s.chars().skip(c - (n.max(4) - 1)).collect::<String>()
        )
    }
}

fn wrap(s: &str, width: usize) -> String {
    let mut out = String::new();
    let mut line = 0;
    for w in s.split_whitespace() {
        if line > 0 && line + 1 + w.chars().count() > width {
            out.push('\n');
            line = 0;
        } else if line > 0 {
            out.push(' ');
            line += 1;
        }
        out.push_str(w);
        line += w.chars().count();
    }
    out
}

fn elapsed(s: f64) -> String {
    let s = s.max(0.0) as u64;
    if s < 60 {
        format!("{s}s")
    } else if s < 3600 {
        format!("{}m {:02}s", s / 60, s % 60)
    } else {
        format!("{}h {:02}m", s / 3600, (s % 3600) / 60)
    }
}

/// Spec section 4: where the energy lives (stacked positive shares, ½ rule, both seam orders), the raw signed block
/// matrix (never stacked), and shape price against level as a dumbbell. Every number is the report's.
fn m8_panels(ui: &mut egui::Ui, m8: &controls::M8Detail) {
    const BLOCKS: [&str; 4] = ["H", "C", "S", "R"];
    let block_col = |k: usize| match k {
        0 => HELIKE,
        1 => RIVAL,
        2 => egui::Color32::from_rgb(150, 164, 176),
        _ => FAINT,
    };
    ui.label(eyebrow("Where the energy lives"));
    for ch in &m8.channels {
        let accent = if ch.name == "helike" { HELIKE } else { RIVAL };
        for (k, (order, sh)) in ch.shares.iter().enumerate() {
            let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), if k + 1 == ch.shares.len() { 18.0 } else { 9.0 }), Sense::hover());
            let p = ui.painter();
            let label_w = 150.0;
            let bar = Rect::from_min_max(Pos2::new(r.left() + label_w, r.top() + 2.0), Pos2::new(r.right() - 70.0, r.bottom() - 2.0));
            if k + 1 == ch.shares.len() {
                p.text(Pos2::new(r.left(), r.center().y), Align2::LEFT_CENTER, format!("{} · {}", ch.name, ch.label), FontId::monospace(10.0), accent);
            } else {
                p.text(Pos2::new(r.left() + 20.0, r.center().y), Align2::LEFT_CENTER, format!("order {order}"), FontId::monospace(8.5), FAINT);
            }
            let mut x = bar.left();
            for (b, v) in sh.iter().enumerate() {
                let w = (*v as f32).clamp(0.0, 1.0) * bar.width();
                if w > 0.0 {
                    let seg = Rect::from_min_max(Pos2::new(x, bar.top()), Pos2::new((x + w - 1.0).max(x), bar.bottom()));
                    p.rect_filled(seg, CornerRadius::same(2), alpha(block_col(b), 0.85));
                }
                x += w;
            }
            let half = bar.left() + 0.5 * bar.width();
            p.line_segment([Pos2::new(half, bar.top() - 2.0), Pos2::new(half, bar.bottom() + 2.0)], Stroke::new(1.0, INK));
            if k + 1 == ch.shares.len() {
                let own = BLOCKS.iter().position(|b| *b == ch.block).map(|i| sh[i]).unwrap_or(0.0);
                p.text(Pos2::new(r.right() - 64.0, r.center().y), Align2::LEFT_CENTER, format!("{own:.5}"), FontId::monospace(10.0), INK);
            }
            resp.on_hover_text(format!("order {order}: H {:.6} · C {:.6} · S {:.6} · R {:.6}", sh[0], sh[1], sh[2], sh[3]));
        }
        ui.add_space(4.0);
    }
    ui.label(caption("Shares of the Dirichlet energy by block (H blue, C amber, S slate, R faint); they sum to 1. The white line is ½ of the channel's own block. Thin bar: the coarse order of the seam pair; thick: the fine."));
    ui.add_space(8.0);

    ui.label(eyebrow("Raw block matrix D_XY / D — raw, signed: not a distribution"));
    ui.horizontal_wrapped(|ui| {
        for ch in &m8.channels {
            for (l, m) in ch.raw.iter().enumerate() {
                let cell = 34.0;
                let (r, _) = ui.allocate_exact_size(Vec2::new(cell * 4.0 + 30.0, cell * 4.0 + 34.0), Sense::hover());
                let p = ui.painter();
                p.text(r.left_top(), Align2::LEFT_TOP, format!("{} · level {}", ch.name, l + 1), FontId::monospace(9.0), if ch.name == "helike" { HELIKE } else { RIVAL });
                let o = Pos2::new(r.left() + 16.0, r.top() + 16.0);
                for (a, row) in m.iter().enumerate() {
                    p.text(Pos2::new(r.left() + 6.0, o.y + cell * (a as f32 + 0.5)), Align2::CENTER_CENTER, BLOCKS[a], FontId::monospace(8.5), FAINT);
                    for (b, v) in row.iter().enumerate() {
                        let c = Rect::from_min_size(Pos2::new(o.x + cell * b as f32, o.y + cell * a as f32), Vec2::splat(cell - 2.0));
                        // magnitude on a log scale from 1e-6 to 1; sign by hue
                        let mag = if *v == 0.0 { 0.0 } else { ((v.abs().log10() + 6.0) / 6.0).clamp(0.0, 1.0) as f32 };
                        let base = if *v >= 0.0 { HELIKE } else { RIVAL };
                        p.rect_filled(c, CornerRadius::same(2), alpha(base, 0.08 + 0.75 * mag));
                        let txt = if v.abs() >= 0.01 { format!("{v:+.2}") } else if *v == 0.0 { "0".into() } else { format!("{:+.0e}", v) };
                        p.text(c.center(), Align2::CENTER_CENTER, txt, FontId::monospace(8.0), if mag > 0.6 { GROUND } else { INK });
                    }
                }
                for (b, name) in BLOCKS.iter().enumerate() {
                    p.text(Pos2::new(o.x + cell * (b as f32 + 0.5), o.y + cell * 4.0 + 6.0), Align2::CENTER_CENTER, *name, FontId::monospace(8.5), FAINT);
                }
            }
        }
    });
    ui.label(caption("Blue positive, amber negative; strength on a log scale from 10⁻⁶ to 1. Off-diagonal cross terms are signed and can cancel, which is why the matrix is never stacked."));
    ui.add_space(8.0);

    ui.label(eyebrow("Shape against level"));
    let vals: Vec<f64> = m8.channels.iter().flat_map(|c| [c.shape_price, c.gap]).flatten().collect();
    if vals.len() >= 2 {
        let lo = vals.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let pad = ((hi - lo) * 0.15).max(1e-4);
        let (lo, hi) = (lo - pad, hi + pad);
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0 * m8.channels.len() as f32 + 26.0), Sense::hover());
        let p = ui.painter();
        let axis = Rect::from_min_max(Pos2::new(r.left() + 150.0, r.top()), Pos2::new(r.right() - 20.0, r.bottom() - 18.0));
        let x_of = |e: f64| axis.left() + ((e - lo) / (hi - lo)) as f32 * axis.width();
        for (k, ch) in m8.channels.iter().enumerate() {
            let y = axis.top() + 14.0 + 30.0 * k as f32;
            let accent = if ch.name == "helike" { HELIKE } else { RIVAL };
            p.text(Pos2::new(r.left(), y - 6.0), Align2::LEFT_CENTER, &ch.name, FontId::monospace(10.0), accent);
            if let Some(d) = ch.discount {
                p.text(Pos2::new(r.left(), y + 7.0), Align2::LEFT_CENTER, format!("discount {d:.5}"), FontId::monospace(9.0), MUTED);
            }
            if let (Some(sp), Some(g)) = (ch.shape_price, ch.gap) {
                p.line_segment([Pos2::new(x_of(sp), y), Pos2::new(x_of(g), y)], Stroke::new(2.0, alpha(accent, 0.6)));
                p.circle_stroke(Pos2::new(x_of(sp), y), 5.0, Stroke::new(1.5, accent));
                p.circle_filled(Pos2::new(x_of(g), y), 5.0, accent);
            }
        }
        for t in [lo + pad, (lo + hi) / 2.0, hi - pad] {
            p.text(Pos2::new(x_of(t), axis.bottom() + 4.0), Align2::CENTER_TOP, format!("{t:.4}"), FontId::monospace(8.5), FAINT);
        }
    }
    let by_shape = m8.ordering_shapes.is_some() && m8.ordering_shapes == m8.ordering_levels;
    ui.label(caption(format!(
        "Hollow: the bare-shape price P_X; filled: the level Δ_X; the segment is the dressing discount. Ordering {} (the report's ordering record).",
        if by_shape { "decided by shape" } else { "decided by dressing" }
    )));
    ui.label(caption("The slow function f = ψ/ψ₀ itself is drawn in Geometry of the controls ▸ The state (DisplayGrade)."));
}

