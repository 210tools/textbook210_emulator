use crate::emulator::{Emulator, EmulatorCell};
use crate::panes::{Pane, PaneDisplay, PaneTree, RealPane};
use crate::theme::ThemeSettings;
use egui::{Align, RichText};
use egui_extras::{Column, TableBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::RangeInclusive;

use super::EmulatorPane;

// lazy_static! {
//     pub static ref BREAKPOINTS: Mutex<HashSet<usize>> = Mutex::new(HashSet::new());
// }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryPane {
    follow_pc: bool,
    jump_addr_str: String,
    #[serde(skip)] // Don't serialize the target scroll address, recalculate on load if needed
    target_scroll_addr: Option<usize>,
    display_base: u32,
    highlighted: HashMap<usize, f32>, // highlighed with fade off (fades in 1 second from 1.0 -> 0)
    was_running: bool,
}

impl Default for MemoryPane {
    fn default() -> Self {
        Self {
            follow_pc: false,
            was_running: false,
            jump_addr_str: "0000".to_string(),
            target_scroll_addr: None,
            highlighted: HashMap::new(),
            display_base: 16,
        }
    }
}

impl PaneDisplay for MemoryPane {
    fn render(&mut self, ui: &mut egui::Ui, emulator: &mut Emulator, theme: &mut ThemeSettings) {
        let artifacts = &emulator.metadata;

        if !self.was_running && emulator.running() {
            self.follow_pc = true; // start following the PC when the start button is pressed
        }

        self.was_running = emulator.running();

        ui.horizontal(|ui| {
            // --- Controls ---
            ui.checkbox(&mut self.follow_pc, "Follow PC");

            ui.label("Jump to (Hex):");
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.jump_addr_str)
                    .desired_width(50.0)
                    .font(egui::TextStyle::Monospace),
            );
            if response.lost_focus()
                || response.ctx.input(|i| i.key_pressed(egui::Key::Enter)) && response.has_focus()
            {
                if let Ok(addr) = usize::from_str_radix(
                    self.jump_addr_str
                        .to_lowercase()
                        .split("x")
                        .last()
                        .expect("split should always return at least 1 element"),
                    16,
                ) {
                    if addr < emulator.memory.len() {
                        self.target_scroll_addr = Some(addr);
                        self.highlighted.insert(addr, 1.0);
                        self.follow_pc = false; // Jumping disables follow PC
                    } else {
                        // Handle invalid address (e.g., show error, reset input)
                        self.jump_addr_str = "FFFF".to_string();
                    }
                } else {
                    // Handle parse error
                    self.jump_addr_str = format!("{:04X}", self.target_scroll_addr.unwrap_or(0));
                }
            }

            ui.label("Base:");
            ui.radio_value(&mut self.display_base, 2, "Bin");
            ui.radio_value(&mut self.display_base, 10, "Dec");
            ui.radio_value(&mut self.display_base, 16, "Hex");
        });

        ui.separator();
        let available_height = ui.available_height();

        // --- Memory Table ---
        let text_height = egui::TextStyle::Monospace.resolve(ui.style()).size * 2.0; // *2 to account for buttons
        let mut table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto().at_least(15.0)) // breakpoint toggle
            .column(Column::auto().at_least(100.0)) // Label
            .column(Column::auto().at_least(60.0)) // Address + PC indicator
            .column(Column::auto().at_least(50.0)) // Value (editable)
            .column(Column::auto().at_least(60.0)) // Value (formatted)
            .column(Column::auto().at_least(150.0)) // Instruction
            .column(Column::remainder().at_least(80.0)) // ASCII
            .max_scroll_height(available_height)
            .min_scrolled_height(0.0);

        // --- Scrolling Logic ---
        let pc_addr = emulator.pc.get() as usize;
        let scroll_target_row = if self.follow_pc {
            Some(pc_addr)
        } else {
            self.target_scroll_addr
        };

        // Apply scrolling *before* creating the body
        if let Some(target_row) = scroll_target_row {
            // Check if the target row is within the valid range of memory indices
            if target_row < emulator.memory.len() {
                // Find the row index within the visible range (assuming full range for now)
                // The `rows_range` logic needs adjustment if you only show a subset.
                // For now, let's assume the visible index is the target address.
                let visible_row_index = target_row; // Adjust if rows_range is used differently
                table = table.scroll_to_row(visible_row_index, Some(Align::Center));
                // Clear target_scroll_addr after jumping if it wasn't set by follow_pc
                if !self.follow_pc {
                    self.target_scroll_addr = None;
                }
            }
        }

        let mut animation_needed = false;
        table
            .header(20.0, |mut ui| {
                ui.col(|ui| {
                    ui.label(RichText::new("BP").monospace().strong());
                });
                ui.col(|ui| {
                    ui.label(RichText::new("Label").monospace().strong());
                });
                ui.col(|ui| {
                    ui.label(RichText::new("Addr").monospace().strong());
                });
                ui.col(|ui| {
                    ui.label(RichText::new("Value").monospace().strong());
                });
                ui.col(|ui| {
                    ui.label(RichText::new("Decoded").monospace().strong());
                });
                ui.col(|ui| {
                    ui.label(RichText::new("ASCII").monospace().strong());
                });
            })
            .body(|mut body| {
                let item_spacing = body.ui_mut().spacing().item_spacing;

                body.rows(text_height, 0xFFFF, |mut row| {
                    let row_index = row.index();

                    let memory_cell = &mut emulator.memory[row_index];
                    let is_pc_line = pc_addr == row_index;
                    let is_curr_line = emulator.currently_executing == row_index;

                    let bg = if is_curr_line {
                        Some(theme.accent_color_positive)
                    } else if let Some(hl) = self.highlighted.get_mut(&row_index) {
                        *hl -= 0.01; // Botch
                        animation_needed = true;
                        Some(egui::Color32::from_rgba_unmultiplied(
                            100,
                            50,
                            200,
                            (255.0 * *hl) as u8, // purple that fades out
                        ))
                    } else if is_pc_line {
                        Some(theme.accent_color_positive.gamma_multiply(0.7))
                    } else {
                        None // Use striped background
                    };

                    let paint_bg = |ui: &mut egui::Ui| {
                        if let Some(color) = bg {
                            let gapless_rect = ui.max_rect().expand2(0.5 * item_spacing);
                            ui.painter().rect_filled(gapless_rect, 0.0, color);
                        }
                    };

                    // Breakpoint toggle
                    row.col(|ui| {
                        let has_breakpoint = emulator.breakpoints.contains(&row_index);

                        let butt = if has_breakpoint {
                            let gapless_rect = ui.max_rect().expand2(0.5 * item_spacing);
                            ui.painter().rect_filled(
                                gapless_rect,
                                0.0,
                                theme.accent_color_negative.gamma_multiply(0.5),
                            );
                            egui::Button::new("🛑").fill(theme.accent_color_negative)
                        } else {
                            egui::Button::new("⚪")
                        };

                        if ui.add(butt).clicked() {
                            if has_breakpoint {
                                emulator.breakpoints.remove(&row_index);
                            } else {
                                emulator.breakpoints.insert(row_index);
                            }
                        }
                    });

                    // label
                    row.col(|ui| {
                        paint_bg(ui);

                        // Try to find a label for this address
                        let label = artifacts.addr_to_label.get(&row_index);

                        let display = match label {
                            Some(lbl) => lbl.to_string(),
                            None => "".to_string(),
                        };

                        ui.label(RichText::new(display).monospace().strong());
                    });

                    // Address Column
                    row.col(|ui| {
                        paint_bg(ui);

                        let addr_text = format!(
                            "0x{:04X}{}",
                            row_index,
                            if is_pc_line { " (PC)" } else { "" }
                        );
                        let rich_text = RichText::new(addr_text).monospace();
                        ui.label(rich_text);
                    });

                    // Value Edit Column
                    row.col(|ui| {
                        paint_bg(ui);

                        let mut value = memory_cell.get();
                        let format_fn = |n: f64, _range: RangeInclusive<usize>| -> String {
                            debug_assert!(n.is_finite());
                            debug_assert!(n <= u16::MAX as f64);
                            debug_assert!(n >= u16::MIN as f64);
                            match self.display_base {
                                16 => format!("{:04X}", n as u16),
                                10 => format!("{}", n as u16),
                                2 => format!("{:016b}", n as u16),
                                _ => String::new(), // Add other bases if needed
                            }
                        };
                        let parse_fn = |s: &str| -> Option<f64> {
                            match self.display_base {
                                16 => u16::from_str_radix(s.trim_start_matches("0x"), 16)
                                    .ok()
                                    .map(|v| v as i16 as f64),
                                10 => s.parse::<i16>().ok().map(|v| v as f64),
                                2 => u16::from_str_radix(s, 2).ok().map(|v| v as i16 as f64),
                                _ => None, // Add other bases if needed
                            }
                        };

                        if ui
                            .add(
                                egui::DragValue::new(&mut value)
                                    .custom_formatter(format_fn)
                                    .custom_parser(parse_fn),
                            )
                            .changed()
                        {
                            memory_cell.set(value);
                        }
                    });

                    // Decoded Instruction Column
                    row.col(|ui| {
                        paint_bg(ui);

                        let value_u16 = memory_cell.get();

                        // Try to decode as an instruction if it's a plausible address
                        let decoded_op = crate::emulator::ops::OpCode::from_instruction(
                            EmulatorCell::new(value_u16),
                        )
                        .map(|op| format!("{op}"));

                        if let Some(decoded_str) = decoded_op {
                            // Display decoded Op struct
                            ui.label(RichText::new(decoded_str).monospace());
                        } else {
                            ui.label(RichText::new("").monospace());
                        }
                    });

                    // ASCII Column
                    row.col(|ui| {
                        let ascii_char = char::from_u32((memory_cell.get() & 0xFF) as u32)
                            .filter(|c| c.is_ascii_graphic() || *c == ' ') // Show printable ASCII or space
                            .map(|c| format!("'{c}'"))
                            .unwrap_or_default();

                        ui.label(RichText::new(ascii_char).monospace().weak()); // Weak color for less emphasis
                    });
                });
            });
        if animation_needed {
            ui.ctx().request_repaint();
        }
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
