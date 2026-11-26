//! One visual world for the viewer: a dark instrument panel. Deep blue-slate ground, raised surfaces a step lighter,
//! hairline rules, and colour kept for meaning: Helike blue, rival amber, proved green, fail red. Everything else is
//! ink, muted ink and structure.

use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Margin, RichText, Stroke, TextStyle,
};

pub const GROUND: Color32 = Color32::from_rgb(13, 18, 22);
pub const SURFACE: Color32 = Color32::from_rgb(19, 25, 30);
pub const RAISED: Color32 = Color32::from_rgb(25, 32, 38);
pub const HAIRLINE: Color32 = Color32::from_rgb(38, 47, 55);
pub const INK: Color32 = Color32::from_rgb(224, 230, 234);
pub const MUTED: Color32 = Color32::from_rgb(128, 142, 153);
pub const FAINT: Color32 = Color32::from_rgb(84, 96, 106);
pub const RULE: Color32 = Color32::from_rgb(58, 66, 74);
pub const HELIKE: Color32 = Color32::from_rgb(92, 172, 236);
pub const RIVAL: Color32 = Color32::from_rgb(232, 162, 76);
pub const PROVED: Color32 = Color32::from_rgb(118, 198, 142);
pub const FAIL: Color32 = Color32::from_rgb(232, 104, 104);
pub const OPEN: Color32 = Color32::from_rgb(120, 132, 142);

pub fn alpha(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (a.clamp(0.0, 1.0) * 255.0) as u8)
}

/// Type scale: 10.5 eyebrow (mono, spaced), 11 caption mono, 12.5 body, 14 heading, 19 title.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    // the body face lacks superscript minus and a few math glyphs (10⁻³); fall back to the mono face for them
    let mut fonts = egui::FontDefinitions::default();
    if let Some(mono_first) = fonts
        .families
        .get(&FontFamily::Monospace)
        .and_then(|v| v.first())
        .cloned()
    {
        if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
            prop.push(mono_first);
        }
    }
    ctx.set_fonts(fonts);
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    style.text_styles = [
        (
            TextStyle::Small,
            FontId::new(10.5, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(12.5, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(12.5, FontFamily::Proportional),
        ),
        (
            TextStyle::Heading,
            FontId::new(19.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(11.0, FontFamily::Monospace),
        ),
    ]
    .into();
    let s = &mut style.spacing;
    s.item_spacing = egui::vec2(8.0, 7.0);
    s.button_padding = egui::vec2(10.0, 4.0);
    s.window_margin = Margin::same(14);
    s.slider_width = 150.0;
    s.interact_size.y = 22.0;
    let v = &mut style.visuals;
    v.dark_mode = true;
    v.panel_fill = SURFACE;
    v.window_fill = SURFACE;
    v.extreme_bg_color = GROUND;
    v.faint_bg_color = RAISED;
    v.override_text_color = Some(INK);
    v.hyperlink_color = HELIKE;
    v.selection.bg_fill = alpha(HELIKE, 0.28);
    v.selection.stroke = Stroke::new(1.0, HELIKE);
    v.window_stroke = Stroke::new(1.0, HAIRLINE);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, HAIRLINE);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, INK);
    for (w, fill, stroke) in [
        (&mut v.widgets.inactive, RAISED, HAIRLINE),
        (
            &mut v.widgets.hovered,
            Color32::from_rgb(32, 41, 49),
            Color32::from_rgb(64, 78, 90),
        ),
        (&mut v.widgets.active, Color32::from_rgb(36, 48, 58), HELIKE),
        (&mut v.widgets.open, RAISED, HAIRLINE),
    ] {
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
        w.bg_stroke = Stroke::new(1.0, stroke);
        w.corner_radius = CornerRadius::same(5);
    }
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, INK);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, INK);
    v.widgets.active.fg_stroke = Stroke::new(1.0, INK);
    v.slider_trailing_fill = true;
    v.striped = true;
    ctx.set_style_of(egui::Theme::Dark, style);
}

/// A small spaced uppercase label that names a section.
pub fn eyebrow(text: &str) -> RichText {
    RichText::new(text.to_uppercase())
        .monospace()
        .size(10.0)
        .color(MUTED)
        .extra_letter_spacing(1.2)
}

pub fn caption(text: impl Into<String>) -> RichText {
    RichText::new(text.into())
        .monospace()
        .size(11.0)
        .color(MUTED)
}

/// A rounded pill with a coloured dot and text (status chips, verdicts).
pub fn pill(ui: &mut egui::Ui, text: &str, col: Color32) -> egui::Response {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), FontId::monospace(10.5), col);
    let size = egui::vec2(galley.size().x + 26.0, 20.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::hover());
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(10),
        alpha(col, 0.10),
        Stroke::new(1.0, alpha(col, 0.45)),
        egui::StrokeKind::Inside,
    );
    p.circle_filled(egui::pos2(rect.left() + 10.0, rect.center().y), 3.0, col);
    p.galley(
        egui::pos2(rect.left() + 18.0, rect.center().y - galley.size().y / 2.0),
        galley,
        col,
    );
    resp
}
