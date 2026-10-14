use crate::panes::glossary::Explain;
use std::collections::HashMap;

use crate::emulator::micro_op::{CycleState, EguiDisplay, MicroOp};
use crate::emulator::ops::OpCode;
use crate::emulator::{disasm, CpuState, Emulator, EmulatorCell, MCR_ADDR, PSR_ADDR};
use crate::panes::ui_kit::{self, Base};
use crate::panes::{Pane, PaneDisplay, PaneTree, RealPane};
use crate::theme::ThemeSettings;
use egui::{Align2, CornerRadius, FontId, RichText, Sense, Vec2};
use serde::{Deserialize, Serialize};

use super::EmulatorPane;

/// Registers watched for change highlighting, in display order.
const WATCHED: usize = 8 + 5; // R0-R7, PC, IR, MAR, MDR, PSR

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CpuStatePane {
    base: Base,
    show_devices: bool,
    show_cycle: bool,
    #[serde(skip)]
    snapshot: Vec<u16>,
    #[serde(skip)]
    changed: Vec<bool>,
    #[serde(skip)]
    last_cycles: u64,
}

impl Default for CpuStatePane {
    fn default() -> Self {
        Self {
            base: Base::Hex,
            show_devices: false,
            show_cycle: true,
            snapshot: Vec::new(),
            changed: vec![false; WATCHED],
            last_cycles: 0,
        }
    }
}

fn watched_values(e: &Emulator) -> Vec<u16> {
    let mut v: Vec<u16> = e.r.iter().map(|c| c.get()).collect();
    v.extend([
        e.pc.get(),
        e.ir.get(),
        e.mar.get(),
        e.mdr.get(),
        e.memory[PSR_ADDR].get(),
    ]);
    v
}

const REG_HELP: [&str; 8] = [
    "General purpose. Traps use R0 for their argument and result.",
    "General purpose.",
    "General purpose.",
    "General purpose.",
    "General purpose.",
    "General purpose. Often used as the frame pointer.",
    "Stack pointer by convention.",
    "Return address: JSR, JSRR and TRAP save the PC here.",
];

/// Where a register was last written and read, prepared for the tooltip.
struct RegHistory {
    index: usize,
    lines: Vec<(String, egui::Color32)>,
    last_write: Option<usize>,
}

fn reg_history(e: &Emulator, r: usize, theme: &ThemeSettings) -> RegHistory {
    let mut lines = Vec::new();
    for (what, acc, color) in [
        ("written", e.reg_writes[r], theme.accent_color_primary),
        ("read", e.reg_reads[r].get(), theme.accent_color_secondary),
    ] {
        match acc {
            Some(a) => {
                let ago = e.instructions.saturating_sub(a.instruction);
                let place = match e.source_line(a.addr) {
                    Some((file, line, src)) => format!(
                        "{file} line {}: {}",
                        line + 1,
                        src.split(';')
                            .next()
                            .unwrap_or("")
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                    ),
                    None => format!("x{:04X}", a.addr),
                };
                lines.push((format!("Last {what} {ago} instr. ago · {place}"), color));
            }
            None => lines.push((format!("Not {what} yet"), theme.secondary_text_color)),
        }
    }
    RegHistory {
        index: r,
        lines,
        last_write: e.reg_writes[r].map(|a| a.addr),
    }
}

impl CpuStatePane {
    fn track_changes(&mut self, emulator: &Emulator) {
        let now = watched_values(emulator);
        if self.snapshot.len() != now.len() || emulator.cycles < self.last_cycles {
            self.snapshot = now;
            self.changed = vec![false; WATCHED];
            self.last_cycles = emulator.cycles;
            return;
        }
        if emulator.cycles != self.last_cycles {
            self.changed = now
                .iter()
                .zip(&self.snapshot)
                .map(|(a, b)| a != b)
                .collect();
            self.snapshot = now;
            self.last_cycles = emulator.cycles;
        }
    }

    /// One register: name, value (click to edit) and a decimal shadow.
    #[allow(clippy::too_many_arguments)]
    fn register(
        &self,
        ui: &mut egui::Ui,
        theme: &ThemeSettings,
        name: &str,
        help: &str,
        cell: &mut EmulatorCell,
        changed: bool,
        shadow: Option<String>,
        name_color: egui::Color32,
    ) {
        self.register_with_history(
            ui, theme, name, help, cell, changed, shadow, name_color, None,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn register_with_history(
        &self,
        ui: &mut egui::Ui,
        theme: &ThemeSettings,
        name: &str,
        help: &str,
        cell: &mut EmulatorCell,
        changed: bool,
        shadow: Option<String>,
        name_color: egui::Color32,
        history: Option<&RegHistory>,
    ) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let name_resp = ui.add_sized(
                [30.0, 20.0],
                egui::Label::new(RichText::new(name).monospace().color(name_color))
                    .sense(Sense::click()),
            );
            if let Some(h) = history {
                if name_resp.hovered() {
                    crate::panes::emulator::editor::hover_register(h.index);
                }
                if name_resp.clicked() {
                    if let Some(a) = h.last_write {
                        if let Ok(mut r) = crate::panes::emulator::editor::REVEAL_ADDR.lock() {
                            *r = Some(a);
                        }
                    }
                }
                let name_resp = if h.last_write.is_some() {
                    name_resp.on_hover_cursor(egui::CursorIcon::PointingHand)
                } else {
                    name_resp
                };
                name_resp.on_hover_ui(|ui| {
                    ui.label(help);
                    for (line, color) in &h.lines {
                        ui.label(RichText::new(line).small().color(*color));
                    }
                    if h.last_write.is_some() {
                        ui.label(
                            RichText::new("Click to show where it was last written")
                                .small()
                                .color(theme.secondary_text_color),
                        );
                    }
                });
            } else {
                name_resp.explain(help);
            }
            let width = if self.base == Base::Bin { 170.0 } else { 54.0 };
            let (bg, fg) = if changed {
                (Some(theme.pc_line_bg), theme.accent_color_primary)
            } else {
                (None, theme.register_value_color)
            };
            let (resp, new) =
                ui_kit::editable_word(ui, ui.id().with(name), cell.get(), self.base, width, fg, bg);
            if let Some(v) = new {
                cell.set(v);
            }
            let v = cell.get();
            if let Some(h) = history {
                if resp.hovered() {
                    crate::panes::emulator::editor::hover_register(h.index);
                }
            }
            resp.on_hover_ui(|ui| {
                ui.label(RichText::new(name).strong());
                if let Some(h) = history {
                    for (line, color) in &h.lines {
                        ui.label(RichText::new(line).small().color(*color));
                    }
                }
                ui.label(help);
                ui.label(
                    RichText::new(format!(
                        "x{v:04X}   {v}   {}\n{}",
                        v as i16,
                        Base::Bin.format(v)
                    ))
                    .monospace(),
                );
                if (32..127).contains(&v) {
                    ui.label(RichText::new(format!("ASCII '{}'", v as u8 as char)).monospace());
                }
                ui.label(
                    RichText::new("Click to edit")
                        .small()
                        .color(theme.secondary_text_color),
                );
            });
            if let Some(s) = shadow {
                ui.label(
                    RichText::new(s)
                        .monospace()
                        .size(11.0)
                        .color(theme.secondary_text_color),
                );
            }
        });
    }

    fn render_registers(
        &mut self,
        ui: &mut egui::Ui,
        emulator: &mut Emulator,
        theme: &ThemeSettings,
    ) {
        let shadow_base = |v: u16| -> Option<String> {
            match self.base {
                Base::Hex => Some(format!("{}", v as i16)),
                Base::Unsigned | Base::Signed => Some(format!("x{v:04X}")),
                Base::Bin => None,
            }
        };
        let cols = if self.base == Base::Bin || ui.available_width() < 300.0 {
            1
        } else {
            2
        };
        egui::Grid::new("gp_regs")
            .num_columns(cols)
            .spacing([18.0, 2.0])
            .striped(false)
            .show(ui, |ui| {
                let rows = 8 / cols;
                for row in 0..rows {
                    for col in 0..cols {
                        let i = row + col * rows;
                        let changed = self.changed.get(i).copied().unwrap_or(false);
                        let shadow = shadow_base(emulator.r[i].get());
                        let history = reg_history(emulator, i, theme);
                        self.register_with_history(
                            ui,
                            theme,
                            &format!("R{i}"),
                            REG_HELP[i],
                            &mut emulator.r[i],
                            changed,
                            shadow,
                            theme.register_name_color,
                            Some(&history),
                        );
                    }
                    ui.end_row();
                }
            });
        ui.add_space(6.0);
        ui_kit::rule(ui, theme);
        ui.add_space(6.0);

        let labels: HashMap<usize, String> = emulator.metadata.addr_to_label.clone();
        let pc = emulator.pc.get();
        let next = disasm::disassemble(emulator.memory[pc as usize].get(), pc as usize, &labels);
        let ir = emulator.ir.get();
        let ir_text = disasm::disassemble(ir, emulator.currently_executing, &labels);
        let sp = theme.register_special_purpose_name_color;
        let c = |i: usize| self.changed.get(i).copied().unwrap_or(false);
        egui::Grid::new("sp_regs")
            .num_columns(2)
            .spacing([18.0, 2.0])
            .striped(false)
            .show(ui, |ui| {
                self.register(
                    ui,
                    theme,
                    "PC",
                    "Program counter: the address of the next instruction to fetch.",
                    &mut emulator.pc,
                    c(8),
                    Some(next),
                    sp,
                );
                self.register(
                    ui,
                    theme,
                    "IR",
                    "Instruction register: the instruction being executed.",
                    &mut emulator.ir,
                    c(9),
                    Some(ir_text),
                    sp,
                );
                ui.end_row();
                self.register(
                    ui,
                    theme,
                    "MAR",
                    "Memory address register: the address memory reads or writes next.",
                    &mut emulator.mar,
                    c(10),
                    None,
                    sp,
                );
                self.register(
                    ui,
                    theme,
                    "MDR",
                    "Memory data register: the word read from or written to memory.",
                    &mut emulator.mdr,
                    c(11),
                    None,
                    sp,
                );
                ui.end_row();
            });
        ui.add_space(6.0);

        // condition codes and privilege
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let (n, z, p) = emulator.get_nzp();
            for (name, on, help) in [
                ("N", n, "Negative: the last value written to a register was negative"),
                ("Z", z, "Zero: the last value written to a register was zero"),
                ("P", p, "Positive: the last value written to a register was positive"),
            ] {
                let (fg, bg) = if on {
                    (egui::Color32::WHITE, theme.accent_color_secondary)
                } else {
                    (theme.secondary_text_color, theme.window_background)
                };
                if ui_kit::chip(ui, name, fg, bg, (!on).then_some(theme.separator_color))
                    .explain(format!("{help}. Click to set."))
                    .clicked()
                {
                    match name {
                        "N" => emulator.set_n(),
                        "Z" => emulator.set_z(),
                        _ => emulator.set_p(),
                    }
                }
            }
            ui.add_space(8.0);
            let mode = match emulator.priv_level() {
                crate::emulator::PrivilegeLevel::Supervisor => "Supervisor",
                crate::emulator::PrivilegeLevel::User => "User",
            };
            ui.label(RichText::new("mode").size(11.5).color(theme.secondary_text_color));
            let mode_button = ui.add(egui::Label::new(RichText::new(mode).size(12.5)).sense(egui::Sense::click()))
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .explain("PSR[15]: 0 = supervisor mode (OS), 1 = user mode. Click to switch.");
            if mode_button.clicked() {
                let other = match emulator.priv_level() {
                    crate::emulator::PrivilegeLevel::Supervisor => crate::emulator::PrivilegeLevel::User,
                    crate::emulator::PrivilegeLevel::User => crate::emulator::PrivilegeLevel::Supervisor,
                };
                emulator.set_priv_level(other);
            }
            ui.add_space(8.0);
            let psr = emulator.memory[PSR_ADDR].get();
            ui.label(RichText::new("PSR").monospace().size(11.5).color(theme.register_device_name_color))
                .explain("Processor status register (memory xFFFC): PSR[15] privilege, PSR[10:8] priority, PSR[2:0] = N Z P. Click the value to edit it.");
            let color = if c(12) { theme.accent_color_primary } else { theme.secondary_text_color };
            let (_, new) = ui_kit::editable_word(ui, egui::Id::new("psr-edit"), psr, ui_kit::Base::Hex, 52.0, color, None);
            // exactly one of N, Z and P must be set
            if let Some(v) = new.filter(|v| (v & 7).count_ones() == 1) {
                emulator.memory[PSR_ADDR].set(v);
            }
        });
    }

    fn render_devices(
        &mut self,
        ui: &mut egui::Ui,
        emulator: &mut Emulator,
        theme: &ThemeSettings,
    ) {
        let dev = theme.register_device_name_color;
        let mut row = |ui: &mut egui::Ui, name: &str, addr: usize, help: &str| {
            let mut cell = emulator.memory[addr];
            self.register(
                ui,
                theme,
                name,
                &format!("{help} (memory x{addr:04X})"),
                &mut cell,
                false,
                None,
                dev,
            );
            emulator.memory[addr] = cell;
        };
        egui::Grid::new("dev_regs").num_columns(2).spacing([18.0, 2.0]).striped(false).show(ui, |ui| {
            row(ui, "KBSR", 0xFE00, "Keyboard status: bit 15 is set when a key is waiting in KBDR; bit 14 (interrupt enable) lets the keyboard interrupt");
            row(ui, "KBDR", 0xFE02, "Keyboard data: the last key typed, in bits 7:0");
            ui.end_row();
            row(ui, "DSR", 0xFE04, "Display status: bit 15 is set when the display is ready");
            row(ui, "DDR", 0xFE06, "Display data: writing a character here prints it");
            ui.end_row();
            row(ui, "MCR", MCR_ADDR, "Machine control: clearing bit 15 halts the machine; setting bit 14 sleeps until an interrupt");
            ui.end_row();
        });
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "Saved SSP x{:04X}   Saved USP x{:04X}",
                    emulator.saved_ssp.get(),
                    emulator.saved_usp.get()
                ))
                .monospace()
                .size(11.5)
                .color(theme.secondary_text_color),
            )
            .explain("Stack pointers saved when switching between user and supervisor mode");
        });
        if !emulator.input_queue.is_empty() {
            ui.label(
                RichText::new(format!(
                    "{} key(s) typed but not read yet",
                    emulator.input_queue.len()
                ))
                .size(11.5)
                .color(theme.secondary_text_color),
            );
        }
    }

    fn render_cycle(&mut self, ui: &mut egui::Ui, emulator: &Emulator, theme: &ThemeSettings) {
        let labels = &emulator.metadata.addr_to_label;
        let at_boundary = matches!(emulator.cpu_state, CpuState::Fetch)
            && !emulator.fetched
            && (emulator.execute_state.is_instruction_complete() || emulator.cycles == 0);
        let addr = if at_boundary {
            emulator.pc.get() as usize
        } else {
            emulator.currently_executing
        };
        let word = emulator.memory[addr].get();
        let text = if at_boundary {
            disasm::disassemble(word, addr, labels)
        } else if emulator.took_interrupt {
            "Keyboard interrupt (INT)".to_string()
        } else {
            // the instruction at the address being executed: IR holds the previous one until
            // the fetch loads it (state 30)
            disasm::disassemble(word, addr, labels)
        };
        let current_phase = if at_boundary {
            None
        } else {
            Some(emulator.execute_state.current_phase)
        };

        if emulator.asleep {
            ui.label(RichText::new("Asleep: the OS set MCR[14], so the clock is stopped until the keyboard asks to interrupt").size(11.5).color(theme.accent_color_primary))
                .explain("Wait for interrupt. Type a key in the terminal to wake the machine");
        }
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(if at_boundary { "Next" } else { "Executing" })
                    .size(11.5)
                    .color(theme.secondary_text_color),
            );
            ui.label(
                RichText::new(&text)
                    .monospace()
                    .strong()
                    .color(theme.strong_text_color),
            );
            ui.label(
                RichText::new(format!("x{addr:04X}"))
                    .monospace()
                    .size(11.5)
                    .color(theme.secondary_text_color),
            );
        });
        ui.add_space(4.0);

        // phase chips
        let names = [
            "Fetch",
            "Decode",
            "Eval addr",
            "Operands",
            "Execute",
            "Store",
        ];
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(3.0, 3.0);
            for (i, n) in names.iter().enumerate() {
                let (fg, bg) = match current_phase {
                    Some(p) if p == i => (theme.on_accent(), theme.accent_color_primary),
                    Some(p) if i < p => (theme.secondary_text_color, theme.code_bg_color),
                    _ => (theme.cpu_state_inactive_color, theme.window_background),
                };
                let font = FontId::proportional(11.0);
                let g = ui.painter().layout_no_wrap(n.to_string(), font, fg);
                let (r, _) =
                    ui.allocate_exact_size(Vec2::new(g.size().x + 12.0, 18.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same(3), bg);
                ui.painter().galley(r.center() - g.size() / 2.0, g, fg);
            }
        });
        ui.add_space(6.0);

        // micro-ops: either the live plan or the plan the next instruction will use
        let style = ui.style().clone();
        let show_op = |ui: &mut egui::Ui, op: &MicroOp, state: i8| {
            // state: -1 done, 0 current, 1 pending, 2 skipped (a state the machine doesn't enter)
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let (r, _) = ui.allocate_exact_size(Vec2::new(10.0, 16.0), Sense::hover());
                if state == 0 {
                    ui.painter().text(r.center(), Align2::CENTER_CENTER, "›", FontId::monospace(13.0), theme.accent_color_primary);
                }
                let job: egui::WidgetText = op.display(theme, &style).into();
                if state < 0 {
                    ui.set_opacity(0.5);
                }
                if state == 2 {
                    ui.set_opacity(0.35);
                }
                let bg = ui.painter().add(egui::Shape::Noop);
                let r = ui.add(egui::Label::new(job).wrap_mode(egui::TextWrapMode::Extend));
                if state == 2 {
                    let y = r.rect.center().y;
                    ui.painter().hline(r.rect.x_range(), y, egui::Stroke::new(1.0_f32, theme.secondary_text_color));
                    ui.label(RichText::new("skipped").size(10.5).italics().color(theme.secondary_text_color))
                        .explain("Not entered: PSR[15] was 0, so the machine was already in Supervisor mode and R6 already held the supervisor stack pointer");
                }
                if state == 0 {
                    ui.painter().set(
                        bg,
                        egui::Shape::rect_filled(r.rect.expand2(Vec2::new(4.0, 1.0)), CornerRadius::same(3), theme.pc_line_bg),
                    );
                }
            });
        };
        if at_boundary {
            let fetch = [
                crate::micro_op!(MAR <- PC),
                crate::micro_op!(PC <- PcPlusOne),
                crate::micro_op!(IR <- MDR),
            ];
            ui.label(
                RichText::new("Fetch")
                    .size(11.0)
                    .color(theme.secondary_text_color),
            );
            for op in &fetch {
                show_op(ui, op, 1);
            }
            if word >> 12 != 0xD {
                if let Some(op) = OpCode::from_instruction(EmulatorCell::new(word)) {
                    let mut plan = op.plan_generator().generate_plan();
                    for (state, name) in [
                        (CycleState::EvaluateAddress, "Evaluate address"),
                        (CycleState::FetchOperands, "Fetch operands"),
                        (CycleState::Execute, "Execute"),
                        (CycleState::StoreResult, "Store result"),
                    ] {
                        if let Some(ops) = plan.remove(&state) {
                            if ops.is_empty() {
                                continue;
                            }
                            ui.label(
                                RichText::new(name)
                                    .size(11.0)
                                    .color(theme.secondary_text_color),
                            );
                            for op in &ops {
                                show_op(ui, op, 1);
                            }
                        }
                    }
                }
            }
        } else {
            let es = &emulator.execute_state;
            // state 45 (switch to the supervisor stack) is skipped when already in Supervisor mode
            let skipped_45 = emulator.skip_state == Some(45)
                || (emulator.micro_log.iter().any(|r| r.state == 37)
                    && !emulator.micro_log.iter().any(|r| r.state == 45));
            let mut in_state = 0u8;
            for (i, name) in [
                "Fetch",
                "Decode",
                "Evaluate address",
                "Fetch operands",
                "Execute",
                "Store result",
            ]
            .iter()
            .enumerate()
            {
                let ops = es.phase_ops(i);
                // a phase starts with its marker; an exception's plan has none
                let skip = matches!(ops.first(), Some(MicroOp::PhaseTransition(_))) as usize;
                let ops: Vec<&MicroOp> = ops.iter().skip(skip).collect();
                if ops.is_empty() {
                    continue;
                }
                let name = match emulator.cpu_state {
                    CpuState::Exception => "Exception",
                    _ if i == 0 && emulator.took_interrupt => "Fetch, then the interrupt",
                    _ => name,
                };
                ui.label(
                    RichText::new(name)
                        .size(11.0)
                        .color(if i == es.current_phase {
                            theme.accent_color_primary
                        } else {
                            theme.secondary_text_color
                        }),
                );
                for (j, op) in ops.iter().enumerate() {
                    let j = j + skip;
                    let st = if i < es.current_phase
                        || (i == es.current_phase && j + 1 < es.micro_op_index)
                    {
                        -1
                    } else if i == es.current_phase && j + 1 == es.micro_op_index {
                        0
                    } else {
                        1
                    };
                    if let MicroOp::State(n) = op {
                        in_state = *n;
                    }
                    show_op(ui, op, if in_state == 45 && skipped_45 { 2 } else { st });
                }
            }
        }
    }
}

impl PaneDisplay for CpuStatePane {
    fn render(&mut self, ui: &mut egui::Ui, emulator: &mut Emulator, theme: &mut ThemeSettings) {
        self.track_changes(emulator);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: 12,
                        right: 12,
                        top: 4,
                        bottom: 12,
                    })
                    .show(ui, |ui| {
                        let theme = &*theme;
                        let mut base = self.base;
                        ui_kit::section_header(ui, theme, "Registers", |ui| {
                            ui_kit::base_selector(ui, theme, &mut base, true);
                        });
                        self.base = base;
                        self.render_registers(ui, emulator, theme);

                        ui.add_space(10.0);
                        let mut show = self.show_devices;
                        collapsible(ui, theme, "Devices", &mut show);
                        self.show_devices = show;
                        if self.show_devices {
                            self.render_devices(ui, emulator, theme);
                        }

                        ui.add_space(10.0);
                        let mut show = self.show_cycle;
                        collapsible(ui, theme, "Instruction cycle", &mut show);
                        self.show_cycle = show;
                        if self.show_cycle {
                            self.render_cycle(ui, emulator, theme);
                        }
                    });
            });
    }

    fn title(&self) -> String {
        "CPU".to_string()
    }

    fn children() -> PaneTree {
        PaneTree::Pane(
            "CPU".to_string(),
            Pane::new(RealPane::EmulatorPanes(Box::new(EmulatorPane::Cpu(
                CpuStatePane::default(),
            )))),
        )
    }
}

fn collapsible(ui: &mut egui::Ui, theme: &ThemeSettings, title: &str, open: &mut bool) {
    let icon = if *open {
        egui_phosphor_icons::icons::CARET_DOWN.0
    } else {
        egui_phosphor_icons::icons::CARET_RIGHT.0
    };
    let r = ui.add(
        egui::Label::new(
            RichText::new(format!("{icon} {}", title.to_uppercase()))
                .size(11.0)
                .strong()
                .color(theme.secondary_text_color),
        )
        .sense(Sense::click()),
    );
    if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        *open = !*open;
    }
    ui.add_space(4.0);
}
