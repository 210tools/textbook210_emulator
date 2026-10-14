//! Small shared widgets so the panes look like one app.

use crate::panes::glossary::Explain;
use egui::{
    Align2, Color32, CornerRadius, FontId, Id, Rect, Response, RichText, Sense, Stroke, Ui, Vec2,
};
use serde::{Deserialize, Serialize};

use crate::theme::ThemeSettings;

/// How to show a 16-bit word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Base {
    #[default]
    Hex,
    Unsigned,
    Signed,
    Bin,
}

impl Base {
    pub const ALL: [Base; 4] = [Base::Hex, Base::Unsigned, Base::Signed, Base::Bin];

    pub fn label(self) -> &'static str {
        match self {
            Base::Hex => "hex",
            Base::Unsigned => "dec",
            Base::Signed => "±",
            Base::Bin => "bin",
        }
    }

    pub fn tooltip(self) -> &'static str {
        match self {
            Base::Hex => "Hexadecimal",
            Base::Unsigned => "Unsigned decimal",
            Base::Signed => "Signed (two's complement) decimal",
            Base::Bin => "Binary",
        }
    }

    pub fn format(self, v: u16) -> String {
        match self {
            Base::Hex => format!("{v:04X}"),
            Base::Unsigned => format!("{v}"),
            Base::Signed => format!("{}", v as i16),
            Base::Bin => format!(
                "{:04b} {:04b} {:04b} {:04b}",
                v >> 12,
                (v >> 8) & 15,
                (v >> 4) & 15,
                v & 15
            ),
        }
    }

    /// Parse user input. Prefixes (`x`, `#`, `b`) override the display base.
    pub fn parse(self, s: &str) -> Option<u16> {
        let s = s.trim().replace([' ', '_'], "");
        if s.is_empty() {
            return None;
        }
        let lower = s.to_ascii_lowercase();
        if let Some(h) = lower.strip_prefix("0x").or_else(|| lower.strip_prefix('x')) {
            return u16::from_str_radix(h, 16).ok();
        }
        if let Some(d) = lower.strip_prefix('#') {
            return d
                .parse::<i32>()
                .ok()
                .filter(|v| (-32768..=65535).contains(v))
                .map(|v| v as u16);
        }
        if let Some(b) = lower.strip_prefix("0b").or_else(|| lower.strip_prefix('b')) {
            return u16::from_str_radix(b, 2).ok();
        }
        match self {
            Base::Hex => u16::from_str_radix(&lower, 16).ok(),
            Base::Bin => u16::from_str_radix(&lower, 2).ok(),
            Base::Unsigned | Base::Signed => lower
                .parse::<i32>()
                .ok()
                .filter(|v| (-32768..=65535).contains(v))
                .map(|v| v as u16),
        }
    }
}

/// The uppercase section label used at the top of each group in a pane.
pub fn section_header(
    ui: &mut Ui,
    theme: &ThemeSettings,
    title: &str,
    right: impl FnOnce(&mut Ui),
) {
    ui.horizontal(|ui| {
        ui.set_min_height(28.0);
        ui.label(
            RichText::new(title.to_uppercase())
                .size(11.0)
                .strong()
                .color(theme.secondary_text_color),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), right);
    });
}

/// A row of small toggle buttons where exactly one is selected.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    theme: &ThemeSettings,
    value: &mut T,
    options: &[(T, &str, &str)],
) -> bool {
    let mut changed = false;
    let font = FontId::monospace(11.0);
    let widths: Vec<f32> = options
        .iter()
        .map(|(_, l, _)| {
            ui.painter()
                .layout_no_wrap(l.to_string(), font.clone(), Color32::WHITE)
                .size()
                .x
                + 14.0
        })
        .collect();
    let total: f32 = widths.iter().sum();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total, 20.0), Sense::hover());
    ui.painter().rect(
        rect,
        CornerRadius::same(4),
        theme.window_background,
        Stroke::new(1.0_f32, theme.separator_color),
        egui::StrokeKind::Inside,
    );
    let mut x = rect.min.x;
    for ((opt, label, tip), w) in options.iter().zip(widths) {
        let r = Rect::from_min_size(egui::pos2(x, rect.min.y), Vec2::new(w, rect.height()));
        x += w;
        let resp = ui.interact(r, ui.id().with(("seg", label)), Sense::click());
        let on = *value == *opt;
        if on {
            ui.painter().rect_filled(
                r.shrink(2.0),
                CornerRadius::same(3),
                theme.widget_fill_hovered,
            );
        } else if resp.hovered() {
            ui.painter().rect_filled(
                r.shrink(2.0),
                CornerRadius::same(3),
                theme.widget_fill_interactive,
            );
        }
        ui.painter().text(
            r.center(),
            Align2::CENTER_CENTER,
            *label,
            font.clone(),
            if on {
                theme.strong_text_color
            } else {
                theme.secondary_text_color
            },
        );
        if resp.explain(*tip).clicked() && !on {
            *value = *opt;
            changed = true;
        }
    }
    changed
}

pub fn base_selector(
    ui: &mut Ui,
    theme: &ThemeSettings,
    base: &mut Base,
    include_signed: bool,
) -> bool {
    let opts: Vec<(Base, &str, &str)> = Base::ALL
        .iter()
        .filter(|b| include_signed || **b != Base::Signed)
        .map(|b| (*b, b.label(), b.tooltip()))
        .collect();
    segmented(ui, theme, base, &opts)
}

/// A word that shows as text and turns into a text field when clicked.
/// Returns the new value when the user commits an edit.
pub fn editable_word(
    ui: &mut Ui,
    id: Id,
    value: u16,
    base: Base,
    width: f32,
    color: Color32,
    bg: Option<Color32>,
) -> (Response, Option<u16>) {
    editable_word_sized(ui, id, value, base, width, color, bg, 13.0)
}

/// [`editable_word`] with its text at `size` points (e.g. to fit a scaled diagram).
#[allow(clippy::too_many_arguments)]
pub fn editable_word_sized(
    ui: &mut Ui,
    id: Id,
    value: u16,
    base: Base,
    width: f32,
    color: Color32,
    bg: Option<Color32>,
    size: f32,
) -> (Response, Option<u16>) {
    let editing_id = id.with("editing");
    let mut editing: Option<String> = ui.data(|d| d.get_temp(editing_id));
    let height = (size * 1.55).max(14.0);
    if let Some(buf) = editing.as_mut() {
        let r = ui.add_sized(
            [width, height],
            egui::TextEdit::singleline(buf)
                .font(FontId::monospace(size))
                .margin(egui::Margin::symmetric(4, 1)),
        );
        if !r.has_focus() && !r.lost_focus() {
            r.request_focus();
        }
        let mut commit = None;
        if r.lost_focus() {
            // Enter or clicking away keeps the edit, Escape drops it
            if !ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                commit = base.parse(buf);
            }
            ui.data_mut(|d| d.remove::<String>(editing_id));
        } else {
            ui.data_mut(|d| d.insert_temp(editing_id, buf.clone()));
        }
        return (r, commit);
    }
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    if let Some(bg) = bg {
        ui.painter().rect_filled(rect, CornerRadius::same(3), bg);
    }
    if resp.hovered() {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(3),
            Stroke::new(1.0_f32, ui.visuals().widgets.hovered.bg_stroke.color),
            egui::StrokeKind::Inside,
        );
        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
    }
    ui.painter().text(
        rect.left_center() + Vec2::new(4.0, 0.0),
        Align2::LEFT_CENTER,
        base.format(value),
        FontId::monospace(size),
        color,
    );
    if resp.clicked() {
        ui.data_mut(|d| d.insert_temp(editing_id, base.format(value).replace(' ', "")));
    }
    (resp, None)
}

/// A small rounded label, e.g. a flag or a state.
pub fn chip(
    ui: &mut Ui,
    text: &str,
    fg: Color32,
    bg: Color32,
    stroke: Option<Color32>,
) -> Response {
    let font = FontId::monospace(11.5);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, fg);
    let size = Vec2::new(galley.size().x + 12.0, 20.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    ui.painter().rect_filled(rect, CornerRadius::same(4), bg);
    if let Some(s) = stroke {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(4),
            Stroke::new(1.0_f32, s),
            egui::StrokeKind::Inside,
        );
    }
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, fg);
    resp
}

/// Thin horizontal rule spanning the pane.
pub fn rule(ui: &mut Ui, theme: &ThemeSettings) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme.separator_color);
}

#[cfg(test)]
mod tests {
    use super::Base;

    #[test]
    fn parses_words() {
        assert_eq!(Base::Hex.parse("3000"), Some(0x3000));
        assert_eq!(Base::Hex.parse("x3000"), Some(0x3000));
        assert_eq!(Base::Unsigned.parse("x10"), Some(16));
        assert_eq!(Base::Signed.parse("-1"), Some(0xFFFF));
        assert_eq!(Base::Hex.parse("#-1"), Some(0xFFFF));
        assert_eq!(Base::Bin.parse("0000 0000 0000 0101"), Some(5));
        assert_eq!(Base::Hex.parse("zz"), None);
        assert_eq!(Base::Signed.format(0xFFFF), "-1");
    }
}
