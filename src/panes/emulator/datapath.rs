//! Datapath pane: the LC-3 datapath (Patt & Patel Figure C.8), animated from the emulator's
//! micro-op log. It shows the state of the state machine (Figures C.2 and C.7) the machine last
//! ran, i.e. its last clock cycle: the wires and parts it used light up, the control signals it
//! asserted are listed, and the states before it in the instruction stay faintly lit. Micro step
//! advances the machine one state.

use crate::emulator::disasm;
use crate::emulator::states::state_text;
use crate::emulator::trace::{self, Activity, Dp, MicroRecord, Sig, Snapshot};
use crate::emulator::Emulator;
use crate::panes::glossary::Explain;
use crate::panes::{Pane, PaneDisplay, PaneTree, RealPane};
use crate::theme::ThemeSettings;
use egui::{vec2, Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke};
use egui_phosphor_icons::icons;
use serde::{Deserialize, Serialize};

use super::EmulatorPane;

mod figure;
mod live;
mod model;

/// How the datapath is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DatapathView {
    /// Figure C.8 as printed in the book
    #[default]
    Textbook,
    /// Every part, laid out to be read, with values in place and registers editable
    Model,
    /// Only what the current state uses, in the order data flows
    Focus,
}

/// Set when a datapath pane was drawn this frame; the app turns tracing on only then.
pub static SHOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DatapathPane {
    #[serde(skip)]
    view: View,
    /// Show the true state: every wire that carries a value lit, up to the gates
    #[serde(default)]
    true_state: bool,
    #[serde(default)]
    kind: DatapathView,
}

#[derive(Debug, Clone, PartialEq, Default)]
struct View {
    /// (log id, states) last shown, to notice a new state
    shown: (u64, usize),
    /// When the shown state changed, for the flow animation
    changed_at: f64,
}

const PHASES: [&str; 6] = [
    "Fetch",
    "Decode",
    "Eval addr",
    "Operands",
    "Execute",
    "Store",
];

impl PaneDisplay for DatapathPane {
    fn render(&mut self, ui: &mut egui::Ui, emulator: &mut Emulator, theme: &mut ThemeSettings) {
        let theme = &*theme;
        let now = ui.input(|i| i.time);
        SHOWN.store(true, std::sync::atomic::Ordering::Relaxed);
        if !emulator.trace {
            // just opened: trace from the next step on
            emulator.trace = true;
            ui.ctx().request_repaint();
        }
        let running = emulator.running();
        // one step per state of the state machine
        let cycles = trace::cycles(&emulator.micro_log);
        let steps: Vec<MicroRecord> = cycles
            .iter()
            .filter_map(|c| trace::merge(&emulator.micro_log[c.start..c.end]))
            .collect();
        let mut descriptions: Vec<String> = cycles
            .iter()
            .map(|c| describe_state(&emulator.micro_log[c.start..c.end]))
            .collect();
        for (i, d) in descriptions.iter_mut().enumerate() {
            if skipped_45(&steps, i) {
                d.push_str(". State 45 was skipped: already in Supervisor mode, so R6 is already the supervisor stack");
            }
        }
        let log = &steps;
        let idx = log.len().checked_sub(1);
        if (emulator.micro_log_id, log.len()) != self.view.shown {
            self.view.shown = (emulator.micro_log_id, log.len());
            self.view.changed_at = now;
        }
        let rec = idx.and_then(|i| log.get(i));
        let live = live_snapshot(emulator);
        let snap = rec.map(|r| r.snap).unwrap_or(live);
        let prev = match idx {
            Some(i) if i > 0 => Some(log[i - 1].snap),
            // the first state: the machine as the instruction began
            Some(_) => Some(emulator.micro_log_before),
            None => None,
        };
        let trail = idx
            .map(|i| {
                log[..i]
                    .iter()
                    .fold(Activity::default(), |a, r| union(a, r.activity))
            })
            .unwrap_or_default();

        // ── header: the instruction and replay controls
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            let (addr, word) = instruction_of(emulator);
            let labels = &emulator.metadata.addr_to_label;
            ui.label(RichText::new(format!("x{addr:04X}")).monospace().size(11.5).color(theme.secondary_text_color));
            ui.label(
                RichText::new(if log.is_empty() {
                    "No instruction run yet".to_string()
                } else if log.iter().any(|r| r.activity.has(Dp::IntSignal)) {
                    "Keyboard interrupt (INT)".to_string()
                } else {
                    disasm::disassemble(word, addr, labels)
                })
                    .monospace()
                    .size(13.0)
                    .strong()
                    .color(theme.strong_text_color),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(6.0);
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.add_enabled_ui(!running && !emulator.halted, |ui| {
                    if ui
                        .button(RichText::new(format!("Next state {}", icons::CARET_RIGHT.0)).size(12.0))
                        .explain("Micro step: run the machine for one state of the state machine (one clock cycle)")
                        .clicked()
                    {
                        emulator.micro_step_command();
                    }
                });
                ui.add_enabled_ui(emulator.can_step_back(), |ui| {
                    if ui
                        .button(RichText::new(format!("{} Previous state", icons::CARET_LEFT.0)).size(12.0))
                        .explain("Back one state: the whole machine goes back a clock cycle, as far as the start of this instruction")
                        .clicked()
                    {
                        emulator.step_back();
                    }
                });
            });
        });

        // ── what this step does
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            if let Some(r) = rec {
                ui.label(RichText::new(format!("State {}", r.state)).size(11.5).color(theme.accent_color_primary).strong())
                    .explain("The state's number in Patt & Patel's state machine (Figures C.2 and C.7)");
                ui.add(egui::Label::new(RichText::new(state_text(r.state)).monospace().size(12.0).color(theme.strong_text_color)).truncate())
                    .explain(state_text(r.state));
            } else {
                ui.label(RichText::new("Step an instruction (Into, Over or Micro) to watch it move through the datapath").size(12.0).color(theme.secondary_text_color));
            }
        });
        if rec.is_some() {
            let text = idx
                .and_then(|i| descriptions.get(i))
                .cloned()
                .unwrap_or_default();
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                crate::panes::glossary::text_styled(ui, &text, 12.0, theme.primary_text_color);
            });
        }

        // ── the control signals this step asserts
        if let Some(r) = rec {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(8.0);
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                let sigs: Vec<Sig> = Sig::ALL
                    .iter()
                    .copied()
                    .filter(|s| r.activity.signal(*s))
                    .collect();
                if sigs.is_empty() {
                    ui.label(
                        RichText::new("No control signals")
                            .size(11.0)
                            .color(theme.secondary_text_color),
                    );
                }
                for s in sigs {
                    let a = theme.accent_color_primary;
                    crate::panes::ui_kit::chip(
                        ui,
                        s.name(),
                        a,
                        a.gamma_multiply(0.14),
                        Some(a.gamma_multiply(0.5)),
                    )
                    .explain(format!("{}: {}", s.name(), signal_help(s)));
                }
            });
        }

        // ── the IR's bits, split into the instruction's fields, with the ones this state uses lit
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ir_strip(ui, theme, snap.ir, rec);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(6.0);
                if self.kind != DatapathView::Focus {
                    ui.toggle_value(&mut self.true_state, RichText::new("True state").size(11.5))
                        .explain("The hardware as it really is in this state: every combinational part computes all the time, so every wire up to a gate is live, with its value. Only the gates (onto the bus) and the load signals (into registers) are switched; the wires this state uses keep flowing.");
                }
                crate::panes::ui_kit::segmented(
                    ui,
                    theme,
                    &mut self.kind,
                    &[
                        (DatapathView::Focus, "Focus", "Only what this state uses, in the order the data flows"),
                        (DatapathView::Model, "Model", "Every part, laid out to be read: values in place, registers editable, IR sliced into its fields"),
                        (DatapathView::Textbook, "Textbook", "Figure C.8 as printed in Patt & Patel"),
                    ],
                );
            });
        });

        // ── the steps of the instruction: a column on the right when there is room, else a row
        let full = ui.available_rect_before_wrap();
        let side_w = 230.0;
        let wide = full.width() - side_w > full.height() * (figure::W / figure::H) * 0.9;
        let area = if wide {
            let side = Rect::from_min_max(Pos2::new(full.max.x - side_w, full.min.y), full.max);
            ui.scope_builder(
                egui::UiBuilder::new().max_rect(side.shrink2(vec2(6.0, 4.0))),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("dp-steps")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            steps_column(ui, theme, log, idx, true);
                        });
                },
            );
            ui.painter().vline(
                side.min.x,
                side.y_range(),
                Stroke::new(1.0_f32, theme.separator_color),
            );
            Rect::from_min_max(full.min, Pos2::new(side.min.x, full.max.y))
        } else {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                steps_row(ui, theme, log, idx);
            });
            ui.available_rect_before_wrap()
        };

        // ── the diagram
        let response = ui.allocate_rect(area, Sense::hover());
        // the shown state's wires keep flowing while the machine is stopped on it
        let flowing = !running && idx.is_some();
        let extra = [
            emulator.saved_ssp.get(),
            emulator.saved_usp.get(),
            emulator.memory[crate::emulator::PSR_ADDR].get(),
            emulator.table.get(),
            emulator.vector.get(),
        ];
        // the true state: what every wire carries, from the machine as the state began
        let true_state = (self.true_state && self.kind != DatapathView::Focus)
            .then(|| live::live(rec, prev.as_ref().unwrap_or(&snap), &extra));
        let tip = match self.kind {
            DatapathView::Textbook => figure::draw(
                ui,
                area,
                theme,
                &figure::Frame {
                    rec,
                    trail,
                    snap: &snap,
                    prev: prev.as_ref(),
                    extra,
                    flow_time: if flowing { Some(now) } else { None },
                    hover: response.hover_pos(),
                    live: true_state.as_ref(),
                },
            ),
            kind => {
                let ctx = model::Ctx {
                    rec,
                    trail,
                    // the model edits the machine, so it shows the machine as it is
                    snap: if kind == DatapathView::Model {
                        live
                    } else {
                        snap
                    },
                    prev,
                    extra,
                    flow_time: if flowing { Some(now) } else { None },
                    hover: response.hover_pos(),
                    live: true_state.as_ref(),
                    ir_addr: instruction_of(emulator).0,
                };
                if kind == DatapathView::Model {
                    model::draw_model(ui, area, theme, &ctx, emulator)
                } else {
                    model::draw_focus(ui, area, theme, &ctx)
                }
            }
        };

        crate::panes::glossary::canvas(&response, tip);

        if flowing {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(30));
        }
    }

    fn title(&self) -> String {
        "Datapath".to_string()
    }

    fn children() -> PaneTree {
        PaneTree::Pane(
            "Datapath".to_string(),
            Pane::new(RealPane::EmulatorPanes(Box::new(EmulatorPane::Datapath(
                DatapathPane::default(),
            )))),
        )
    }
}

/// The first line of a step's text, kept short for the timeline.
fn short(text: &str) -> String {
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    if line.chars().count() > 30 {
        format!("{}…", line.chars().take(29).collect::<String>())
    } else {
        line.to_string()
    }
}

fn step_colors(theme: &ThemeSettings, i: usize, idx: Option<usize>) -> (Color32, Color32) {
    if Some(i) == idx {
        (theme.window_background, theme.accent_color_primary)
    } else if idx.is_some_and(|c| i > c) {
        (
            theme.secondary_text_color.gamma_multiply(0.6),
            Color32::TRANSPARENT,
        )
    } else {
        (theme.primary_text_color, theme.widget_fill_interactive)
    }
}

fn step_tip(r: &MicroRecord) -> String {
    let ops = if r.text.trim().is_empty() {
        "none, it only tests a condition".to_string()
    } else {
        r.text.trim().to_string()
    };
    format!(
        "State {}: {}\n\nMicro-ops: {ops}\n\n{}",
        r.state,
        state_text(r.state),
        describe(r)
    )
}

/// The steps as a compact stepper: each phase's name with a dot per step; returns the one
/// clicked. Hovering a dot shows its register transfer.
fn steps_row(
    ui: &mut egui::Ui,
    theme: &ThemeSettings,
    log: &[MicroRecord],
    idx: Option<usize>,
) -> Option<usize> {
    let mut clicked = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 4.0);
        let mut i = 0;
        while i < log.len() {
            let phase = log[i].phase;
            let end = (i..log.len())
                .find(|&k| log[k].phase != phase)
                .unwrap_or(log.len());
            let current_phase = idx.is_some_and(|c| (i..end).contains(&c));
            ui.add_space(if i == 0 { 0.0 } else { 8.0 });
            ui.label(
                RichText::new(PHASES.get(phase).copied().unwrap_or("?"))
                    .size(11.0)
                    .color(if current_phase {
                        theme.accent_color_primary
                    } else {
                        theme.secondary_text_color
                    }),
            );
            for (k, log) in log.iter().enumerate().take(end).skip(i) {
                let (rect, resp) = ui.allocate_exact_size(vec2(12.0, 16.0), Sense::click());
                let c = rect.center();
                let accent = theme.accent_color_primary;
                if Some(k) == idx {
                    ui.painter().circle_filled(c, 5.0, accent);
                } else if idx.is_some_and(|cur| k < cur) {
                    ui.painter()
                        .circle_filled(c, 3.5, accent.gamma_multiply(0.55));
                } else {
                    ui.painter().circle_stroke(
                        c,
                        3.5,
                        Stroke::new(1.2_f32, theme.secondary_text_color),
                    );
                }
                if resp.hovered() {
                    ui.painter().circle_stroke(
                        c,
                        6.5,
                        Stroke::new(1.0_f32, theme.primary_text_color),
                    );
                }
                if resp.explain(step_tip(log)).clicked() {
                    clicked = Some(k);
                }
            }
            i = end;
        }
    });
    clicked
}

/// The steps as a list grouped by phase, with the value each produced; returns the one clicked.
fn steps_column(
    ui: &mut egui::Ui,
    theme: &ThemeSettings,
    log: &[MicroRecord],
    idx: Option<usize>,
    follow: bool,
) -> Option<usize> {
    let mut clicked = None;
    ui.spacing_mut().item_spacing.y = 1.0;
    let mut last_phase = usize::MAX;
    for (i, r) in log.iter().enumerate() {
        if r.phase != last_phase {
            last_phase = r.phase;
            ui.add_space(4.0);
            ui.label(
                RichText::new(PHASES.get(r.phase).copied().unwrap_or("?").to_uppercase())
                    .size(10.0)
                    .color(theme.secondary_text_color),
            );
        }
        if skipped_45(log, i) {
            // the stack switch that didn't happen, so it's clear why the states jump
            let (rect, resp) =
                ui.allocate_exact_size(vec2(ui.available_width(), 18.0), Sense::hover());
            let dim = theme.secondary_text_color.gamma_multiply(0.8);
            ui.painter().text(
                rect.left_center() + vec2(6.0, 0.0),
                Align2::LEFT_CENTER,
                "45",
                FontId::monospace(11.0),
                dim.gamma_multiply(0.6),
            );
            ui.painter().text(
                rect.left_center() + vec2(30.0, 0.0),
                Align2::LEFT_CENTER,
                "skipped: already Supervisor",
                FontId::proportional(11.0),
                dim,
            );
            resp.explain(format!(
                "State 45 (Saved_USP <- R6, R6 <- Saved_SSP) was skipped: {SKIPPED_45}"
            ));
        }
        let (fg, bg) = step_colors(theme, i, idx);
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::click());
        let bg = if resp.hovered() && Some(i) != idx {
            theme.widget_fill_hovered
        } else {
            bg
        };
        ui.painter().rect_filled(rect, CornerRadius::same(3), bg);
        ui.painter().text(
            rect.left_center() + vec2(6.0, 0.0),
            Align2::LEFT_CENTER,
            format!("{:>2}", r.state),
            FontId::monospace(11.0),
            fg.gamma_multiply(0.7),
        );
        // the state's text, shortened to fit beside its value
        let font = FontId::monospace(11.0);
        let char_w = ui.fonts_mut(|f| f.glyph_width(&font, '0')).max(1.0);
        let value_w = if r.value.is_some() {
            5.0 * char_w + 12.0
        } else {
            0.0
        };
        let room = ((rect.width() - 30.0 - 6.0 - value_w) / char_w)
            .floor()
            .max(1.0) as usize;
        ui.painter().text(
            rect.left_center() + vec2(30.0, 0.0),
            Align2::LEFT_CENTER,
            elide(state_text(r.state), room),
            font.clone(),
            fg,
        );
        if let Some(v) = r.value {
            let col = if Some(i) == idx {
                theme.window_background
            } else {
                theme
                    .memory_value_color
                    .gamma_multiply(if fg == theme.primary_text_color {
                        1.0
                    } else {
                        0.6
                    })
            };
            ui.painter().text(
                rect.right_center() - vec2(6.0, 0.0),
                Align2::RIGHT_CENTER,
                format!("x{v:04X}"),
                font,
                col,
            );
        }
        let resp = resp.explain(step_tip(r));
        if follow && Some(i) == idx {
            resp.scroll_to_me(None);
        }
        if resp.clicked() {
            clicked = Some(i);
        }
    }
    clicked
}

/// Why state 45 can be missing before state 37.
const SKIPPED_45: &str = "PSR[15] was 0, so the machine was already in Supervisor mode (the OS itself, e.g. its boot code calling PUTS) and R6 already held the supervisor stack pointer. From a User mode program, 45 switches R6 to the supervisor stack first";

/// Did step `i` (state 37, the first push) come straight after a check that skipped state 45?
fn skipped_45(log: &[MicroRecord], i: usize) -> bool {
    i > 0 && log[i].state == 37 && log[i - 1].state != 45
}

/// `text` cut to `chars` characters, ending in … when cut.
fn elide(text: &str, chars: usize) -> String {
    if text.chars().count() <= chars {
        text.to_string()
    } else {
        format!(
            "{}…",
            text.chars()
                .take(chars.saturating_sub(1))
                .collect::<String>()
        )
    }
}

fn union(a: Activity, b: Activity) -> Activity {
    Activity {
        parts: a.parts | b.parts,
        signals: a.signals | b.signals,
    }
}

fn live_snapshot(e: &Emulator) -> Snapshot {
    let (n, z, p) = e.get_nzp();
    Snapshot {
        pc: e.pc.get(),
        ir: e.ir.get(),
        mar: e.mar.get(),
        mdr: e.mdr.get(),
        r: std::array::from_fn(|i| e.r[i].get()),
        nzp: (n as u8) << 2 | (z as u8) << 1 | p as u8,
    }
}

/// The address and word of the instruction in the log: the fetch's `MAR <- PC` carries the
/// address, and IR the word.
fn instruction_of(e: &Emulator) -> (usize, u16) {
    let addr = e
        .micro_log
        .iter()
        .find(|r| r.activity.signal(Sig::GatePc) && r.activity.signal(Sig::LdMar))
        .and_then(|r| r.value)
        .map(|v| v as usize)
        .unwrap_or(e.currently_executing);
    // the instruction at that address: IR still holds the previous one until state 30
    let word = e.memory[addr & 0xFFFF].get();
    (addr, word)
}

/// What a state does: a sentence for each of its micro-ops.
fn describe_state(records: &[MicroRecord]) -> String {
    // steps that put the same value on the bus (e.g. MAR <- R6 - 1 and R6 <- R6 - 1) are one
    // transfer with two destinations
    let gates = [
        Sig::GatePc,
        Sig::GateMdr,
        Sig::GateAlu,
        Sig::GateMarMux,
        Sig::GatePcMinus1,
        Sig::GateSp,
        Sig::GatePsr,
        Sig::GateVector,
    ];
    let gate_of = |r: &MicroRecord| gates.iter().copied().find(|g| r.activity.signal(*g));
    let mut merged: Vec<MicroRecord> = Vec::new();
    for r in records.iter().filter(|r| !r.marker) {
        match merged.last_mut() {
            Some(m) if gate_of(m).is_some() && gate_of(m) == gate_of(r) && m.value == r.value => {
                m.activity.parts |= r.activity.parts;
                m.activity.signals |= r.activity.signals;
                m.regs_read |= r.regs_read;
                m.reg_written = r.reg_written.or(m.reg_written);
            }
            _ => merged.push(r.clone()),
        }
    }
    let mut out: Vec<String> = Vec::new();
    for r in &merged {
        let d = describe(r);
        if !d.is_empty() && !out.contains(&d) {
            out.push(d);
        }
    }
    if out.is_empty() {
        "Control tests a condition; nothing moves on the datapath".into()
    } else {
        out.join(". ")
    }
}

/// A plain sentence for what a step does on the datapath.
pub fn describe(r: &MicroRecord) -> String {
    let a = r.activity;
    let mut parts: Vec<String> = Vec::new();
    if a.signal(Sig::RwRead) {
        return "Memory reads the word at the address in MAR into MDR".into();
    }
    if a.signal(Sig::RwWrite) {
        return "Memory stores MDR at the address in MAR".into();
    }
    if a.has(Dp::IntSignal) {
        return "The keyboard has a key (KBSR[15]) and may interrupt (KBSR[14]), and its priority, PL4, is above the program's PSR[10:8]: INT sends the state machine to state 49 instead of fetching".into();
    }
    if a.has(Dp::IntPriorityIn) && a.signal(Sig::LdPriority) {
        return "PSR[10:8] (Priority) loads the interrupt's priority, PL4, through PSRMUX, so only higher priority devices can interrupt the service routine".into();
    }
    if a.has(Dp::VecIntv) {
        return "Vector loads INTV through VectorMUX: the keyboard's vector, x80".into();
    }
    if a.signal(Sig::LdBen) {
        return "Control decodes the opcode in IR[15:12] to pick the next state, and sets BEN from IR[11:9] and N, Z, P (BR's condition)".into();
    }
    // registers this step read, e.g. "R6"
    let regs_read: Vec<String> = (0..8)
        .filter(|i| r.regs_read & (1 << i) != 0)
        .map(|i| format!("R{i}"))
        .collect();
    let read = regs_read.join(" and ");
    if a.has(Dp::Adder) {
        let base = if a.signal(Sig::Addr1Pc) {
            "PC".to_string()
        } else if read.is_empty() {
            "a base register".to_string()
        } else {
            read.clone()
        };
        let off = if a.signal(Sig::Addr2Off11) {
            "SEXT(PCoffset11)"
        } else if a.signal(Sig::Addr2Off9) {
            "SEXT(PCoffset9)"
        } else if a.signal(Sig::Addr2Off6) {
            "SEXT(offset6)"
        } else if a.signal(Sig::Addr2Zero) {
            "0"
        } else {
            ""
        };
        if !off.is_empty() {
            parts.push(format!("The address adder adds {base} and {off}"));
        }
    }
    // the ALU's operation, when this step runs one (not when it only puts ALU_OUT on the bus)
    let aluk = [
        (Sig::AlukAdd, "adds"),
        (Sig::AlukAnd, "ANDs"),
        (Sig::AlukNot, "complements"),
    ]
    .into_iter()
    .find(|(s, _)| a.signal(*s));
    if let Some((_, op)) = aluk {
        // operands from the micro-op, e.g. "ALU_OUT <- R1 + #-1"
        let rhs = r.text.split_once("<- ").map(|(_, rhs)| rhs).unwrap_or("");
        let words: Vec<&str> = rhs.split_whitespace().filter(|w| *w != "NOT").collect();
        let first = words.first().copied().unwrap_or("SR1");
        let second = match words.get(2) {
            Some(w) if a.signal(Sig::Sr2MuxImm) => format!(" and {w} (SEXT(imm5))"),
            Some(w) => format!(" and {w}"),
            None => String::new(),
        };
        parts.push(format!("The ALU {op} {first}{second}"));
    }
    if a.has(Dp::Bus) {
        let src: Option<String> = if a.signal(Sig::GatePc) {
            Some("PC".into())
        } else if a.signal(Sig::GateMdr) {
            Some("MDR".into())
        } else if a.signal(Sig::GateMarMux) && a.signal(Sig::MarMuxZext) {
            Some("ZEXT(trap vector) through MARMUX".into())
        } else if a.signal(Sig::GateMarMux) {
            Some("the adder's result, through MARMUX,".into())
        } else if a.signal(Sig::GateAlu) && a.signal(Sig::AlukPassA) {
            Some(if read.is_empty() {
                "a register, passed through the ALU (PASSA),".to_string()
            } else {
                format!("{read}, passed through the ALU (PASSA),")
            })
        } else if a.signal(Sig::GateAlu) {
            Some("the ALU's result".into())
        } else if a.signal(Sig::GatePcMinus1) {
            Some("PC - 1".into())
        } else if a.signal(Sig::GateSp) {
            let what = if a.signal(Sig::SpMuxDec) {
                "R6 - 1"
            } else if a.signal(Sig::SpMuxInc) {
                "R6 + 1"
            } else if a.signal(Sig::SpMuxSsp) {
                "Saved.SSP"
            } else {
                "Saved.USP"
            };
            Some(format!("{what}, through SPMUX,"))
        } else if a.signal(Sig::GatePsr) {
            Some("the PSR".into())
        } else if a.signal(Sig::GateVector) {
            Some("Table'Vector".into())
        } else {
            None
        };
        if let Some(src) = src {
            parts.push(format!("{src} drives the bus"));
        }
    }
    if a.signal(Sig::PcMuxInc) {
        parts.push("PC loads PC + 1 through PCMUX".into());
    } else if a.signal(Sig::PcMuxAdder) {
        parts.push("PC loads the adder's result through PCMUX".into());
    } else if a.signal(Sig::PcMuxBus) {
        parts.push("PC loads the bus through PCMUX".into());
    }
    let reg = r
        .reg_written
        .map(|n| format!("R{n}"))
        .unwrap_or_else(|| "the destination register".into());
    let mut loads: Vec<String> = Vec::new();
    for (s, name) in [
        (Sig::LdMar, "MAR"),
        (Sig::LdMdr, "MDR"),
        (Sig::LdIr, "IR"),
        (Sig::LdReg, reg.as_str()),
        (Sig::LdSavedUsp, "Saved.USP"),
        (Sig::LdSavedSsp, "Saved.SSP"),
        (Sig::LdPriority, "the PSR"),
    ] {
        if a.signal(s) {
            loads.push(name.to_string());
        }
    }
    if !a.has(Dp::Bus) && parts.is_empty() {
        // a transfer that does not use the bus, e.g. Table <- x00
        if let Some((to, from)) = r.text.split_once(" <- ") {
            // the figure's names for the stack pointer registers
            let (to, from) = (
                to.replace("Saved_", "Saved."),
                from.replace("Saved_", "Saved."),
            );
            if a.signal(Sig::SetPriv) {
                return format!(
                    "{to} (Priv) loads {from} from Set.Priv, through PSRMUX: Supervisor mode"
                );
            }
            return format!("{to} loads {from}");
        }
    }
    if !loads.is_empty() {
        let verb = if loads.len() > 1 { "latch" } else { "latches" };
        parts.push(format!("{} {verb} it", loads.join(" and ")));
    }
    if a.signal(Sig::LdCc) && a.has(Dp::BusToLogic) {
        parts.push("LOGIC sets N, Z or P from the value".into());
    }
    if parts.is_empty() {
        return if r.text.trim().is_empty() {
            "Control tests a condition; nothing moves on the datapath".to_string()
        } else {
            short(&r.text)
        };
    }
    let mut s = parts.join("; ");
    if let Some(first) = s.get(0..1) {
        s = first.to_uppercase() + &s[1..];
    }
    if let Some(v) = r.value {
        s.push_str(&format!(" (x{v:04X})"));
    }
    s
}

fn signal_help(s: Sig) -> &'static str {
    match s {
        Sig::LdMar | Sig::LdMdr | Sig::LdIr | Sig::LdReg | Sig::LdPc => {
            "Load: the register takes its input at the end of the clock cycle"
        }
        Sig::LdBen => "Load the branch-enable bit while decoding",
        Sig::LdCc => "Load the condition codes N, Z, P",
        Sig::GatePc | Sig::GateMdr | Sig::GateAlu | Sig::GateMarMux => {
            "Gate: drive this value onto the bus"
        }
        Sig::PcMuxInc | Sig::PcMuxBus | Sig::PcMuxAdder => {
            "PCMUX select: where the PC's next value comes from"
        }
        Sig::Addr1Pc | Sig::Addr1Base => "ADDR1MUX select: the adder's base",
        Sig::Addr2Zero | Sig::Addr2Off6 | Sig::Addr2Off9 | Sig::Addr2Off11 => {
            "ADDR2MUX select: the adder's offset"
        }
        Sig::MarMuxZext | Sig::MarMuxAdder => "MARMUX select: what GateMARMUX puts on the bus",
        Sig::AlukAdd | Sig::AlukAnd | Sig::AlukNot | Sig::AlukPassA => "ALUK: the ALU's operation",
        Sig::Sr2MuxImm => "SR2MUX select: use the immediate instead of SR2",
        Sig::MioEn => "Memory enable",
        Sig::RwRead | Sig::RwWrite => "Read or write memory",
        Sig::GatePcMinus1 | Sig::GateSp | Sig::GatePsr | Sig::GateVector => {
            "Gate: drive this value onto the bus (Figure C.8)"
        }
        Sig::LdSavedSsp | Sig::LdSavedUsp => "Save R6 as the supervisor or user stack pointer",
        Sig::LdPriv | Sig::SetPriv => {
            "Load PSR[15], the privilege bit (Set.Priv picks the constant)"
        }
        Sig::LdPriority => "Load the priority bits PSR[10:8]",
        Sig::LdTable | Sig::LdVector => "Load Table or Vector, which address the vector tables",
        Sig::SpMuxInc | Sig::SpMuxDec | Sig::SpMuxUsp | Sig::SpMuxSsp => {
            "SPMUX select: the new stack pointer"
        }
        Sig::TableMux | Sig::VectorMux => "Pick the constant for Table or Vector",
    }
}

/// An instruction's fields: (high bit, low bit, name, value).
fn ir_fields(ir: u16) -> Vec<(u32, u32, &'static str, String)> {
    let f = |hi: u32, lo: u32| (ir >> lo) & ((1 << (hi - lo + 1)) - 1);
    let reg = |hi: u32, lo: u32| format!("R{}", f(hi, lo));
    let sext = |hi: u32, lo: u32| {
        let bits = hi - lo + 1;
        let v = f(hi, lo);
        let v = if v >> (bits - 1) & 1 == 1 {
            v as i32 - (1 << bits)
        } else {
            v as i32
        };
        format!("#{v}")
    };
    let op = (ir >> 12) as u8;
    let opcode = (
        15,
        12,
        "opcode",
        [
            "BR", "ADD", "LD", "ST", "JSR", "AND", "LDR", "STR", "RTI", "NOT", "LDI", "STI", "JMP",
            "—", "LEA", "TRAP",
        ][op as usize]
            .to_string(),
    );
    let mut v = vec![opcode];
    match op {
        0x1 | 0x5 => {
            v.push((11, 9, "DR", reg(11, 9)));
            v.push((8, 6, "SR1", reg(8, 6)));
            if ir & 0x20 != 0 {
                v.push((5, 5, "imm", "1".into()));
                v.push((4, 0, "imm5", sext(4, 0)));
            } else {
                v.push((5, 3, "reg", format!("{:03b}", f(5, 3))));
                v.push((2, 0, "SR2", reg(2, 0)));
            }
        }
        0x9 => {
            v.push((11, 9, "DR", reg(11, 9)));
            v.push((8, 6, "SR", reg(8, 6)));
            v.push((5, 0, "", String::new()));
        }
        0x0 => {
            let c = |b: u16, ch: char| if ir & b != 0 { ch } else { '-' };
            v.push((
                11,
                9,
                "nzp",
                format!("{}{}{}", c(0x800, 'n'), c(0x400, 'z'), c(0x200, 'p')),
            ));
            v.push((8, 0, "PCoffset9", sext(8, 0)));
        }
        0x2 | 0x3 | 0xA | 0xB | 0xE => {
            v.push((
                11,
                9,
                if op == 0x3 || op == 0xB { "SR" } else { "DR" },
                reg(11, 9),
            ));
            v.push((8, 0, "PCoffset9", sext(8, 0)));
        }
        0x6 | 0x7 => {
            v.push((11, 9, if op == 0x7 { "SR" } else { "DR" }, reg(11, 9)));
            v.push((8, 6, "BaseR", reg(8, 6)));
            v.push((5, 0, "offset6", sext(5, 0)));
        }
        0xC => {
            v.push((11, 9, "", String::new()));
            v.push((8, 6, "BaseR", reg(8, 6)));
            v.push((5, 0, "", String::new()));
        }
        0x4 if ir & 0x800 != 0 => {
            v.push((11, 11, "JSR", "1".into()));
            v.push((10, 0, "PCoffset11", sext(10, 0)));
        }
        0x4 => {
            v.push((11, 9, "JSRR", String::new()));
            v.push((8, 6, "BaseR", reg(8, 6)));
            v.push((5, 0, "", String::new()));
        }
        0xF => {
            v.push((11, 8, "", String::new()));
            v.push((7, 0, "trapvect8", format!("x{:02X}", f(7, 0))));
        }
        _ => v.push((11, 0, "", String::new())),
    }
    v
}

/// The IR bits a state reads: the fields sliced into the sign extenders, ZEXT, SR2MUX, the
/// register file and control.
fn ir_bits_used(ir: u16, rec: &MicroRecord) -> u16 {
    let a = rec.activity;
    let range =
        |hi: u32, lo: u32| -> u16 { (((1u32 << (hi + 1)) - 1) & !((1u32 << lo) - 1)) as u16 };
    let op = ir >> 12;
    let mut m = 0u16;
    if a.has(Dp::Sext11) {
        m |= range(10, 0);
    }
    if a.has(Dp::Sext9) {
        m |= range(8, 0);
    }
    if a.has(Dp::Sext6) {
        m |= range(5, 0);
    }
    if a.has(Dp::Sext5) {
        m |= range(5, 0);
    }
    if a.has(Dp::Zext) || a.has(Dp::IrToVector) {
        m |= range(7, 0);
    }
    if a.has(Dp::Sr2Out) {
        m |= range(5, 5) | range(2, 0);
    }
    if a.signal(Sig::LdBen) {
        m |= range(15, 9);
    }
    // SR1MUX: IR[11:9] for the stored register (state 23), IR[8:6] otherwise, R6 for the stack
    if (a.has(Dp::Sr1Out) || a.has(Dp::RegToAddr1)) && !a.has(Dp::Sr1ToSp) {
        m |= if rec.state == 23 {
            range(11, 9)
        } else {
            range(8, 6)
        };
    }
    // DRMUX: IR[11:9], except JSR and TRAP (R7) and the stack pointer logic (R6)
    if a.signal(Sig::LdReg)
        && op != 0x4
        && op != 0xF
        && !a.has(Dp::SpMux)
        && !a.has(Dp::SavedSsp)
        && !a.has(Dp::SavedUsp)
    {
        m |= range(11, 9);
    }
    m
}

/// The IR as 16 bits grouped into the instruction's fields; the bits this state uses are lit.
fn ir_strip(ui: &mut egui::Ui, theme: &ThemeSettings, ir: u16, rec: Option<&MicroRecord>) {
    let used = rec.map_or(0, |r| ir_bits_used(ir, r));
    let accent = theme.accent_color_primary;
    let bit_w = 10.5;
    let gap = 5.0;
    let fields = ir_fields(ir);
    let width = 22.0 + 16.0 * bit_w + gap * (fields.len() - 1) as f32;
    let (rect, _) = ui.allocate_exact_size(vec2(width, 30.0), Sense::hover());
    let p = ui.painter();
    p.text(
        rect.left_top() + vec2(0.0, 8.0),
        Align2::LEFT_CENTER,
        "IR",
        FontId::monospace(11.0),
        theme.register_special_purpose_name_color,
    );
    let mut x = rect.min.x + 22.0;
    for (hi, lo, name, value) in &fields {
        let n = (hi - lo + 1) as f32;
        let field = Rect::from_min_size(Pos2::new(x, rect.min.y), vec2(n * bit_w, 16.0));
        let lit = (*lo..=*hi).any(|b| used >> b & 1 == 1);
        p.rect(
            field,
            CornerRadius::same(2),
            if lit {
                accent.gamma_multiply(0.12)
            } else {
                theme.code_bg_color
            },
            Stroke::new(1.0_f32, if lit { accent } else { theme.separator_color }),
            egui::StrokeKind::Inside,
        );
        for b in (*lo..=*hi).rev() {
            let cell = Rect::from_min_size(
                Pos2::new(x + (hi - b) as f32 * bit_w, rect.min.y),
                vec2(bit_w, 16.0),
            );
            let on = used >> b & 1 == 1;
            if on {
                p.rect_filled(cell.shrink(1.5), CornerRadius::same(2), accent);
            }
            let bit = if ir >> b & 1 == 1 { "1" } else { "0" };
            p.text(
                cell.center(),
                Align2::CENTER_CENTER,
                bit,
                FontId::monospace(10.5),
                if on {
                    theme.on_accent()
                } else {
                    theme.primary_text_color
                },
            );
        }
        let caption = if value.is_empty() {
            name.to_string()
        } else if name.is_empty() {
            value.clone()
        } else {
            format!("{name} {value}")
        };
        let font = FontId::proportional(8.5);
        let size =
            crate::panes::emulator::datapath::figure::fit(8.5, &caption, field.width() + gap);
        let font = if size < 6.5 {
            FontId::proportional(6.5)
        } else {
            FontId { size, ..font }
        };
        let text = if size < 6.5 {
            value.clone()
        } else {
            caption.clone()
        };
        p.text(
            Pos2::new(field.center().x, field.max.y + 6.5),
            Align2::CENTER_CENTER,
            &text,
            font,
            if lit {
                accent
            } else {
                theme.secondary_text_color
            },
        );
        let span = if hi == lo {
            format!("IR[{hi}]")
        } else {
            format!("IR[{hi}:{lo}]")
        };
        let tip = if caption.is_empty() {
            format!("{span}: not used by this instruction")
        } else {
            format!("{span}: {caption}")
        };
        ui.interact(
            field.expand2(vec2(0.0, 4.0)).with_max_y(rect.max.y),
            ui.id().with(("irf", *hi)),
            Sense::hover(),
        )
        .explain(if lit {
            format!("{tip}\nUsed in this state")
        } else {
            tip
        });
        x += n * bit_w + gap;
    }
}
