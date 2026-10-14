use crate::emulator::{disasm, Emulator};
use crate::panes::emulator::editor::lexer::{lex_line, TokKind};
use crate::panes::glossary::Explain;
use crate::panes::ui_kit::{self, Base};
use crate::panes::{Pane, PaneDisplay, PaneTree, RealPane};
use crate::theme::ThemeSettings;
use egui::{Align2, Color32, CornerRadius, FontId, RichText, Sense, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::EmulatorPane;

const ROW_H: f32 = 22.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryPane {
    follow_pc: bool,
    base: Base,
    #[serde(skip)]
    query: String,
    #[serde(skip)]
    query_error: bool,
    #[serde(skip)]
    scroll_to: Option<usize>,
    /// Values of the visible rows last frame, to flash cells that change
    #[serde(skip)]
    last_values: HashMap<usize, u16>,
    /// Cells that changed recently, with the time they changed
    #[serde(skip)]
    flashes: HashMap<usize, f64>,
    #[serde(skip)]
    visible: (usize, usize),
    #[serde(skip)]
    last_pc: u16,
    #[serde(skip)]
    was_running: bool,
    /// The editor selection we last scrolled to
    #[serde(skip)]
    last_source_selection: Vec<usize>,
}

impl Default for MemoryPane {
    fn default() -> Self {
        Self {
            follow_pc: true,
            base: Base::Hex,
            query: String::new(),
            query_error: false,
            scroll_to: Some(0x3000),
            last_values: HashMap::new(),
            flashes: HashMap::new(),
            visible: (0, 0),
            last_pc: 0,
            was_running: false,
            last_source_selection: Vec::new(),
        }
    }
}

/// Colour stripe and name for the region an address belongs to.
fn region(addr: usize, theme: &ThemeSettings) -> (Color32, &'static str) {
    match addr {
        0x0000..=0x00FF => (theme.memory_heap_bg, "Trap vector table"),
        0x0100..=0x01FF => (theme.memory_os_data_bg, "Interrupt vector table"),
        0x0200..=0x2FFF => (theme.memory_os_code_bg, "Operating system"),
        0x3000..=0xFDFF => (theme.memory_user_code_bg, "User program space"),
        _ => (theme.memory_device_registers_bg, "Device registers"),
    }
}

/// Is this word data (from .FILL, .BLKW or .STRINGZ) according to the source it came from?
fn is_data(emulator: &Emulator, addr: usize) -> bool {
    let line_text: Option<&str> = if let Some(p) = emulator.program_at(addr) {
        p.address_to_line
            .get(&addr)
            .and_then(|l| p.source.get(l.saturating_sub(1)))
            .map(String::as_str)
    } else if addr < 0x3000 {
        let os = crate::emulator::os_source_of(emulator.os);
        os.address_to_line
            .get(&addr)
            .and_then(|l| os.text.lines().nth(l.saturating_sub(1)))
    } else {
        None
    };
    let Some(src) = line_text else {
        return false;
    };
    lex_line(src).iter().any(|t| {
        t.kind == TokKind::Directive && {
            let d = src[t.range.clone()].to_ascii_uppercase();
            d == ".FILL" || d == ".BLKW" || d == ".STRINGZ"
        }
    })
}

impl MemoryPane {
    fn go_to(&mut self, emulator: &Emulator) {
        let q = self.query.trim();
        if q.is_empty() {
            return;
        }
        let label = emulator
            .metadata
            .labels
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(q))
            .map(|(_, v)| *v);
        let addr = label.or_else(|| Base::Hex.parse(q).map(|v| v as usize));
        match addr {
            Some(a) => {
                self.scroll_to = Some(a);
                self.flashes.insert(a, -1.0);
                self.follow_pc = false;
                self.query_error = false;
            }
            None => self.query_error = true,
        }
    }
}

impl PaneDisplay for MemoryPane {
    fn render(&mut self, ui: &mut egui::Ui, emulator: &mut Emulator, theme: &mut ThemeSettings) {
        let theme = &*theme;
        let now = ui.input(|i| i.time);
        if !self.was_running && emulator.running() {
            self.follow_pc = true;
        }
        self.was_running = emulator.running();

        // ---------- header ----------
        egui::Frame::new()
            .inner_margin(egui::Margin {
                left: 12,
                right: 12,
                top: 6,
                bottom: 6,
            })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut self.query)
                            .hint_text(format!(
                                "{} Go to address or label",
                                egui_phosphor_icons::icons::MAGNIFYING_GLASS.0
                            ))
                            .desired_width((ui.available_width() - 260.0).max(90.0))
                            .text_color(if self.query_error {
                                theme.error_fg_color
                            } else {
                                theme.primary_text_color
                            }),
                    );
                    if r.changed() {
                        self.query_error = false;
                    }
                    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.go_to(emulator);
                    }
                    r.explain(
                        "An address such as x3000, or a label from your program. Press Enter.",
                    );
                    let (fg, bg) = if self.follow_pc {
                        (theme.strong_text_color, theme.widget_fill_hovered)
                    } else {
                        (theme.secondary_text_color, theme.window_background)
                    };
                    if ui_kit::chip(ui, "Follow PC", fg, bg, Some(theme.separator_color))
                        .explain("Keep the instruction at the PC in view")
                        .clicked()
                    {
                        self.follow_pc = !self.follow_pc;
                        if self.follow_pc {
                            self.scroll_to = Some(emulator.pc.get() as usize);
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui_kit::base_selector(ui, theme, &mut self.base, true);
                    });
                });
            });
        ui_kit::rule(ui, theme);

        // ---------- follow the PC ----------
        let pc = emulator.pc.get();
        if self.follow_pc && pc != self.last_pc {
            let (a, b) = self.visible;
            let p = pc as usize;
            if p < a + 2 || p + 2 > b {
                self.scroll_to = Some(p);
            }
        }
        self.last_pc = pc;

        // words generated by the lines selected in the editor
        let source_sel: Vec<usize> = crate::panes::emulator::editor::SOURCE_SELECTION
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default();
        if source_sel != self.last_source_selection {
            if let Some(first) = source_sel.first() {
                let (a, b) = self.visible;
                let last = *source_sel.last().unwrap_or(first);
                if !emulator.running() && (*first < a + 1 || last + 1 > b) {
                    self.scroll_to = Some(*first);
                    self.follow_pc = false;
                }
            }
            self.last_source_selection = source_sel.clone();
        }

        let labels = emulator.metadata.addr_to_label.clone();
        let mut scroll = egui::ScrollArea::vertical().auto_shrink([false, false]);
        if let Some(t) = self.scroll_to.take() {
            let view_h = ui.available_height();
            scroll =
                scroll.vertical_scroll_offset((t as f32 * ROW_H - view_h / 2.0 + ROW_H).max(0.0));
        }
        let value_w = match self.base {
            Base::Bin => 150.0,
            _ => 50.0,
        };
        let mut new_values = HashMap::new();
        let mut toggled = None;
        let mut edited: Option<(usize, u16)> = None;
        let running_fast = emulator.running() && emulator.speed > 1;

        ui.spacing_mut().item_spacing.y = 0.0;
        scroll.show_rows(ui, ROW_H, 0x10000, |ui, range| {
            self.visible = (range.start, range.end);
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            for addr in range {
                let value = emulator.memory[addr].get();
                new_values.insert(addr, value);
                if let Some(old) = self.last_values.get(&addr) {
                    if *old != value && !running_fast {
                        self.flashes.insert(addr, now);
                    }
                }
                let (row_rect, row_resp) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_H), Sense::click());
                if row_resp.double_clicked() {
                    if let Ok(mut r) = crate::panes::emulator::editor::REVEAL_ADDR.lock() {
                        *r = Some(addr);
                    }
                }
                let p = ui.painter().clone();
                let is_pc = addr == pc as usize;
                let has_bp = emulator.breakpoints.contains(&addr);
                let in_source = source_sel.binary_search(&addr).is_ok();
                // mid-instruction, the instruction being executed (the PC has moved on)
                let is_executing =
                    emulator.fetched && addr == emulator.currently_executing && !is_pc;
                if is_pc {
                    p.rect_filled(row_rect, 0.0, theme.pc_line_bg);
                } else if is_executing {
                    p.rect_filled(row_rect, 0.0, theme.pc_line_bg.gamma_multiply(0.55));
                } else if in_source {
                    p.rect_filled(row_rect, 0.0, theme.info_bg_color);
                } else if row_resp.hovered() {
                    p.rect_filled(row_rect, 0.0, theme.code_bg_color);
                }
                if let Some(t) = self.flashes.get(&addr).copied() {
                    let t = if t < 0.0 {
                        self.flashes.insert(addr, now);
                        now
                    } else {
                        t
                    };
                    let a = (1.0 - ((now - t) / 1.2) as f32).clamp(0.0, 1.0);
                    if a > 0.0 {
                        p.rect_filled(row_rect, 0.0, theme.highlight_bg.gamma_multiply(a));
                        ui.ctx().request_repaint();
                    } else {
                        self.flashes.remove(&addr);
                    }
                }
                if in_source {
                    p.rect_filled(
                        egui::Rect::from_min_size(
                            row_rect.min + Vec2::new(3.0, 0.0),
                            Vec2::new(2.0, ROW_H),
                        ),
                        0.0,
                        theme.accent_color_secondary,
                    );
                }
                let (stripe, region_name) = region(addr, theme);
                p.rect_filled(
                    egui::Rect::from_min_size(row_rect.min, Vec2::new(3.0, ROW_H)),
                    0.0,
                    stripe.gamma_multiply(0.85),
                );

                let mut x = row_rect.min.x + 6.0;
                let cy = row_rect.center().y;
                // breakpoint
                let bp_rect = egui::Rect::from_min_size(
                    egui::pos2(x, row_rect.min.y),
                    Vec2::new(18.0, ROW_H),
                );
                let bp_resp = ui.interact(bp_rect, ui.id().with(("bp", addr)), Sense::click());
                if has_bp {
                    p.circle_filled(bp_rect.center(), 4.5, theme.accent_color_negative);
                } else if bp_resp.hovered() {
                    p.circle_filled(
                        bp_rect.center(),
                        4.5,
                        theme.accent_color_negative.gamma_multiply(0.35),
                    );
                }
                if bp_resp.explain("Toggle breakpoint").clicked() {
                    toggled = Some(addr);
                }
                x += 20.0;
                // address
                let addr_resp = ui.interact(
                    egui::Rect::from_min_size(
                        egui::pos2(x, row_rect.min.y),
                        Vec2::new(52.0, ROW_H),
                    ),
                    ui.id().with(("addr", addr)),
                    Sense::hover(),
                );
                p.text(
                    egui::pos2(x, cy),
                    Align2::LEFT_CENTER,
                    format!("x{addr:04X}"),
                    FontId::monospace(12.5),
                    if is_pc || is_executing {
                        theme.accent_color_primary
                    } else {
                        theme.memory_address_color
                    },
                );
                if is_executing {
                    addr_resp.explain(format!("{region_name}\nThe instruction being executed"));
                } else if is_pc {
                    addr_resp.explain(format!("{region_name}\nPC: the next instruction"));
                } else {
                    addr_resp.explain(region_name);
                }
                x += 56.0;
                // value
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(egui::Rect::from_min_size(
                            egui::pos2(x, row_rect.min.y + 1.0),
                            Vec2::new(value_w, ROW_H - 2.0),
                        ))
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                let color = if value == 0 {
                    theme.secondary_text_color.gamma_multiply(0.7)
                } else {
                    theme.memory_value_color
                };
                let (_, new) = ui_kit::editable_word(
                    &mut child,
                    ui.id().with(("val", addr)),
                    value,
                    self.base,
                    value_w,
                    color,
                    None,
                );
                if let Some(v) = new {
                    edited = Some((addr, v));
                }
                x += value_w + 12.0;
                // label + decoded
                let label = labels.get(&addr);
                if let Some(l) = label {
                    let shown = if l.len() > 14 {
                        format!("{}…", &l[..13])
                    } else {
                        l.clone()
                    };
                    p.text(
                        egui::pos2(x, cy),
                        Align2::LEFT_CENTER,
                        &shown,
                        FontId::monospace(12.5),
                        theme.memory_label_color,
                    );
                }
                x += 112.0;
                let (text, color) = if is_data(emulator, addr) {
                    let ch = if (32..127).contains(&value) {
                        format!("'{}'  ", value as u8 as char)
                    } else if value == 0 {
                        "\\0  ".to_string()
                    } else if value == 10 {
                        "'\\n'  ".to_string()
                    } else {
                        String::new()
                    };
                    (format!("{ch}{}", value as i16), theme.memory_ascii_color)
                } else if value == 0 {
                    (
                        "·".to_string(),
                        theme.secondary_text_color.gamma_multiply(0.5),
                    )
                } else {
                    (
                        disasm::disassemble(value, addr, &labels),
                        theme.memory_instruction_color,
                    )
                };
                let shown = p.text(
                    egui::pos2(x, cy),
                    Align2::LEFT_CENTER,
                    text,
                    FontId::monospace(12.5),
                    color,
                );
                // a word no source line explains (data written at run time, e.g. on the stack)
                // that is a character: show it too
                if !is_data(emulator, addr) && ((32..127).contains(&value) || value == 10) {
                    let ch = if value == 10 {
                        "'\\n'".to_string()
                    } else {
                        format!("'{}'", value as u8 as char)
                    };
                    p.text(
                        shown.right_center() + Vec2::new(10.0, 0.0),
                        Align2::LEFT_CENTER,
                        ch,
                        FontId::monospace(12.5),
                        theme.memory_ascii_color.gamma_multiply(0.8),
                    );
                }
            }
        });

        self.last_values = new_values;
        if let Some(a) = toggled {
            if !emulator.breakpoints.remove(&a) {
                emulator.breakpoints.insert(a);
            }
        }
        if let Some((a, v)) = edited {
            emulator.memory[a].set(v);
        }
        let _ = (Stroke::NONE, CornerRadius::ZERO, RichText::new(""));
    }

    fn title(&self) -> String {
        "Memory".to_string()
    }

    fn children() -> PaneTree {
        PaneTree::Pane(
            "Memory".to_string(),
            Pane::new(RealPane::EmulatorPanes(Box::new(EmulatorPane::Memory(
                MemoryPane::default(),
            )))),
        )
    }
}
