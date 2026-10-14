use crate::emulator::Emulator;
use crate::panes::glossary::Explain;
use crate::panes::{Pane, PaneDisplay, PaneTree, RealPane};
use crate::theme::ThemeSettings;
use egui::{Align2, CornerRadius, Event, FontId, Key, RichText, Sense, Vec2};
use egui_phosphor_icons::icons;
use serde::{Deserialize, Serialize};

use super::EmulatorPane;

/// The console: shows what the program prints and sends keys typed into it to the keyboard.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
pub struct IoPane {
    #[serde(skip)]
    focused: bool,
}

impl PaneDisplay for IoPane {
    fn render(&mut self, ui: &mut egui::Ui, emulator: &mut Emulator, theme: &mut ThemeSettings) {
        let theme = &*theme;
        let rect = ui.available_rect_before_wrap();
        let id = ui.id().with("terminal_focus");
        let resp = ui.interact(rect, id, Sense::click());
        if resp.clicked() {
            resp.request_focus();
        }
        self.focused = resp.has_focus();
        ui.painter().rect_filled(rect, 0.0, theme.terminal_bg_color);

        // keyboard input goes to the machine while the terminal has focus
        if self.focused {
            let events = ui.input(|i| i.events.clone());
            for e in events {
                match e {
                    // pasted text is typed in, key by key (\r\n becomes one newline)
                    Event::Text(t) | Event::Paste(t) => t
                        .chars()
                        .filter(|c| *c != '\r')
                        .for_each(|c| emulator.push_input(c)),
                    Event::Key {
                        key: Key::Enter,
                        pressed: true,
                        ..
                    } => emulator.push_input('\n'),
                    Event::Key {
                        key: Key::Backspace,
                        pressed: true,
                        ..
                    } => emulator.push_input('\u{8}'),
                    Event::Key {
                        key: Key::Tab,
                        pressed: true,
                        ..
                    } => emulator.push_input('\t'),
                    Event::Key {
                        key: Key::Escape,
                        pressed: true,
                        ..
                    } => ui.memory_mut(|m| m.surrender_focus(id)),
                    _ => {}
                }
            }
            // keep Tab from moving focus away
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    id,
                    egui::EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: false,
                    },
                )
            });
        }

        let mut child =
            ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink2(Vec2::new(12.0, 8.0))));
        let waiting = emulator.waiting_for_input();
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .max_height(rect.height() - 40.0)
            .show(&mut child, |ui| {
                let font = FontId::monospace(13.5);
                let mut job = egui::text::LayoutJob::default();
                job.append(
                    &emulator.output,
                    0.0,
                    egui::TextFormat::simple(font.clone(), theme.terminal_text_color),
                );
                let galley = ui.fonts_mut(|f| f.layout_job(job));
                let size = galley.size();
                let (r, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), size.y.max(18.0) + 4.0),
                    Sense::hover(),
                );
                let end = galley
                    .pos_from_cursor(egui::text::CCursor::new(emulator.output.chars().count()));
                ui.painter()
                    .galley(r.min, galley, theme.terminal_text_color);
                // caret at the end of the output
                let blink = (ui.input(|i| i.time) * 2.0) as i64 % 2 == 0;
                if self.focused || waiting {
                    let caret = egui::Rect::from_min_size(
                        r.min + end.min.to_vec2() + Vec2::new(1.0, 1.0),
                        Vec2::new(8.0, end.height() - 2.0),
                    );
                    let c = if self.focused && blink {
                        theme.terminal_cursor_color
                    } else {
                        theme.terminal_cursor_color.gamma_multiply(0.35)
                    };
                    ui.painter().rect_filled(caret, CornerRadius::same(1), c);
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(500));
                }
            });

        // footer: hint, queued keys and actions
        let footer = egui::Rect::from_min_max(egui::pos2(rect.min.x, rect.max.y - 30.0), rect.max);
        ui.painter().hline(
            footer.x_range(),
            footer.min.y,
            egui::Stroke::new(1.0_f32, theme.separator_color),
        );
        let mut f = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(footer.shrink2(Vec2::new(10.0, 3.0)))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let (hint, color) = if waiting && !self.focused {
            (
                "The program is waiting for a key. Click here and type.".to_string(),
                theme.accent_color_primary,
            )
        } else if waiting {
            (
                "The program is waiting for a key.".to_string(),
                theme.accent_color_primary,
            )
        } else if self.focused {
            (
                "Typing goes to the program · Esc to leave".to_string(),
                theme.secondary_text_color,
            )
        } else {
            (
                "Click here to type into the program".to_string(),
                theme.secondary_text_color,
            )
        };
        // short version when the pane is narrow so it doesn't run into the buttons
        let tiny = footer.width() < 280.0;
        let hint = if tiny {
            String::new()
        } else if footer.width() < 460.0 {
            if waiting {
                "Waiting for a key".to_string()
            } else if self.focused {
                "Typing goes to the program".to_string()
            } else {
                "Click to type".to_string()
            }
        } else {
            hint
        };
        f.label(
            RichText::new(format!("{} {hint}", icons::KEYBOARD.0))
                .size(12.0)
                .color(color),
        )
        .explain("Click the terminal to type into the program");
        if !emulator.input_queue.is_empty() {
            f.label(
                RichText::new(format!("  {} queued", emulator.input_queue.len()))
                    .size(12.0)
                    .color(theme.secondary_text_color),
            );
        }
        f.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let label = |icon: &str, text: &str| {
                if tiny {
                    icon.to_string()
                } else {
                    format!("{icon} {text}")
                }
            };
            if ui
                .add(
                    egui::Button::new(RichText::new(label(icons::TRASH.0, "Clear")).size(12.0))
                        .frame(false),
                )
                .explain("Clear the output")
                .clicked()
            {
                emulator.output.clear();
            }
            if ui
                .add(
                    egui::Button::new(RichText::new(label(icons::COPY.0, "Copy")).size(12.0))
                        .frame(false),
                )
                .explain("Copy the output")
                .clicked()
            {
                ui.ctx().copy_text(emulator.output.clone());
            }
        });
        if self.focused {
            ui.painter().rect_stroke(
                rect.shrink(0.5),
                CornerRadius::ZERO,
                egui::Stroke::new(1.0_f32, theme.accent_color_secondary.gamma_multiply(0.6)),
                egui::StrokeKind::Inside,
            );
        }
        let _ = Align2::LEFT_TOP;
    }

    fn title(&self) -> String {
        "Terminal".to_string()
    }

    fn children() -> PaneTree {
        PaneTree::Pane(
            "Terminal".to_string(),
            Pane::new(RealPane::EmulatorPanes(Box::new(EmulatorPane::Output(
                IoPane::default(),
            )))),
        )
    }
}
