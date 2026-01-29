use crate::emulator::micro_op::EguiDisplay;
use crate::emulator::{CpuState, Emulator, EmulatorCell, MCR_ADDR, PSR_ADDR};
use crate::panes::{Pane, PaneDisplay, PaneTree, RealPane};
use crate::theme::ThemeSettings;
use egui::{Response, RichText};
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;

use super::EmulatorPane;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuStatePane {
    use_negative: bool,
    display_base: u32,
}

impl Default for CpuStatePane {
    fn default() -> Self {
        Self {
            use_negative: false,
            display_base: 16,
        }
    }
}

impl PaneDisplay for CpuStatePane {
    fn render(&mut self, ui: &mut egui::Ui, emulator: &mut Emulator, theme: &mut ThemeSettings) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::CollapsingHeader::new("Registers & devices")
                .default_open(true)
                .show(ui, |ui| {
                    self.render_register_view(ui, emulator);
                });
            ui.collapsing("Processor Cycle", |ui| {
                self.render_cycle_view(ui, emulator, theme);
            });
        });
    }

    fn title(&self) -> String {
        "CPU State".to_string()
    }

    fn children() -> PaneTree {
        PaneTree::Pane(
            "CPU State".to_string(),
            Pane::new(RealPane::EmulatorPanes(Box::new(EmulatorPane::Cpu(
                CpuStatePane::default(),
            )))),
        )
    }
}

fn register_view(
    ui: &mut egui::Ui,
    value_cell: &mut EmulatorCell,
    use_negative: bool,
    display_base: u32,
) -> Response {
    if use_negative {
        let mut value: i16 = value_cell.get() as i16;
        let format_fn = |n: f64, _range: RangeInclusive<usize>| -> String {
            debug_assert!(n.is_finite());
            debug_assert!(n <= i16::MAX as f64);
            debug_assert!(n >= i16::MIN as f64);
            match display_base {
                16 => format!("{:04X}", n as u16),
                10 => format!("{}", n as i16),
                2 => format!("{:016b}", n as u16),
                _ => String::new(),
            }
        };
        let parse_fn = |s: &str| -> Option<f64> {
            match display_base {
                16 => u16::from_str_radix(s.trim_start_matches("0x"), 16)
                    .ok()
                    .map(|v| v as i16 as f64),
                10 => s.parse::<i16>().ok().map(|v| v as f64),
                2 => u16::from_str_radix(s, 2).ok().map(|v| v as i16 as f64),
                _ => None,
            }
        };

        let response = ui.add(
            egui::DragValue::new(&mut value)
                .range(i16::MIN..=i16::MAX)
                .custom_formatter(format_fn)
                .custom_parser(parse_fn)
        )
        .on_hover_text("Drag to change value (hold Shift for slower). Click to edit value directly. Displayed as signed (-32768-32767)");

        if response.changed() {
            value_cell.set(value as u16);
        }
        response
    } else {
        let mut value: u16 = value_cell.get();
        let format_fn = |n: f64, _range: RangeInclusive<usize>| -> String {
            debug_assert!(n.is_finite());
            debug_assert!(n <= u16::MAX as f64);
            debug_assert!(n >= u16::MIN as f64);
            match display_base {
                16 => format!("{:04X}", n as u16),
                10 => format!("{}", n as u16),
                2 => format!("{:016b}", n as u16),
                _ => String::new(),
            }
        };
        let parse_fn = |s: &str| -> Option<f64> {
            match display_base {
                16 => u16::from_str_radix(s.trim_start_matches("0x"), 16)
                    .ok()
                    .map(|v| v as f64),
                10 => s.parse::<u16>().ok().map(|v| v as f64),
                2 => u16::from_str_radix(s, 2).ok().map(|v| v as f64),
                _ => None,
            }
        };

        let response = ui.add(
            egui::DragValue::new(&mut value)
                .range(u16::MIN..=u16::MAX)
                .custom_formatter(format_fn)
                .custom_parser(parse_fn)
        )
        .on_hover_text("Drag to change value (hold Shift for slower). Click to edit value directly. Displayed as unsigned (0-65535)");

        if response.changed() {
            value_cell.set(value);
        }
        response
    }
}

impl CpuStatePane {
    fn render_register_view(&mut self, ui: &mut egui::Ui, emulator: &mut Emulator) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.use_negative, "Show <0 as negative")
                .on_hover_text("Whether to display the 2s complement registers as being negative if bit 15 is set. EG FFFF vs -0001.");

            ui.label("Base:");
            ui.radio_value(&mut self.display_base, 2, "Bin");
            ui.radio_value(&mut self.display_base, 10, "Dec");
            ui.radio_value(&mut self.display_base, 16, "Hex");
        });

        egui::Grid::new("my_grid")
            .striped(true)
            .min_col_width(2.)
            .show(ui, |ui| {
                for row in 0..2 {
                    for col in 0..4 {
                        let register = row * 4 + col;
                        ui.label(format!("R{register}:"));
                        register_view(ui, &mut emulator.r[register], self.use_negative, self.display_base);
                    }
                    ui.end_row();
                }
                ui.label("PC:").on_hover_text("This register holds the next instruction that will be fetched in the fetch phase of the CPU. MEM[PC] -> IR");
                register_view(ui, &mut emulator.pc, self.use_negative, self.display_base).on_hover_text("This register holds the next instruction that will be fetched in the fetch phase of the CPU. MEM[PC] -> IR");

                ui.label("MDR:").on_hover_text("This register holds the data that has been read from memory or will be written to memory.");
                register_view(ui, &mut emulator.mdr, self.use_negative, self.display_base).on_hover_text("This register holds the data that has been read from memory or will be written to memory.");

                ui.label("MAR:").on_hover_text("This register holds the address of the memory location that will be read from or written to.");
                register_view(ui, &mut emulator.mar, self.use_negative, self.display_base).on_hover_text("This register holds the address of the memory location that will be read from or written to.");

                ui.label("IR:").on_hover_text("This register holds the instruction that has been fetched from memory. This instruction is decoded and executed by the CPU.");
                register_view(ui, &mut emulator.ir, self.use_negative, self.display_base).on_hover_text("This register holds the instruction that has been fetched from memory. This instruction is decoded and executed by the CPU.");
                ui.end_row();

                let (n, z, p) = emulator.get_nzp();
                let privilege_mode = emulator.priv_level();
                if ui
                    .selectable_label(n, RichText::new("N").monospace())
                    .on_hover_text("Negative Flag. This is set when the result of an arithmetic or load operation is negative.")
                    .clicked()
                {
                    emulator.set_n();
                }


                if ui
                    .selectable_label(z, RichText::new("Z").monospace())
                    .on_hover_text("Zero Flag. This is set when the result of an arithmetic or load operation is zero.")
                    .clicked()
                {
                    emulator.set_z();
                }


                if ui
                    .selectable_label(p, RichText::new("P").monospace())
                    .on_hover_text("Positive Flag. This is set when the result of an arithmetic or load operation is positive.")
                    .clicked()
                {
                    emulator.set_p();
                }
                ui.label(
                    RichText::new(match privilege_mode {
                        crate::emulator::PrivilegeLevel::Supervisor => "PRIV=0",
                        crate::emulator::PrivilegeLevel::User => "PRIV=1",
                    })
                    .monospace(),
                ).on_hover_text("Privilege Mode. This indicates the current privilege level of the CPU. PRIV=0 indicates supervisor mode, PRIV=1 indicates user mode.");

                ui.label("PSR:").on_hover_text(RichText::new("mem[0xFFFC]").code()).on_hover_text("Processor Status Register. Layout: PSR[15] = 0 when in supervisor mode and 1 when user mode, PSR[2] = N, PSR[1] = Z, PSR[0] = P");
                register_view(ui, &mut emulator.memory[PSR_ADDR], self.use_negative, self.display_base).on_hover_text(RichText::new("mem[0xFFFC]").code()).on_hover_text("Processor Status Register. Layout: PSR[15] = 0 when in supervisor mode and 1 when user mode, PSR[2] = N, PSR[1] = Z, PSR[0] = P");

                ui.label("MCR:").on_hover_text(RichText::new("mem[0xFFFE]").code()).on_hover_text("Machine Control Register, when MCR[15] is set the machine is running, otherwise it is halted");
                register_view(ui, &mut emulator.memory[MCR_ADDR], self.use_negative, self.display_base).on_hover_text(RichText::new("mem[0xFFFE]").code()).on_hover_text("Machine Control Register, when MCR[15] is set the machine is running, otherwise it is halted");
                ui.end_row();

               ui.label("KBDR:").on_hover_text(RichText::new("mem[0xFE02]").code()).on_hover_text("Keyboard Data Register, contains the last typed ASCII character in bits [7:0].");
               register_view(ui, &mut emulator.memory[0xFE02], self.use_negative, self.display_base).on_hover_text(RichText::new("mem[0xFE02]").code()).on_hover_text("Keyboard Data Register, contains the last typed ASCII character in bits [7:0].");

               ui.label("KBSR:").on_hover_text(RichText::new("mem[0xFE00]").code()).on_hover_text("Keyboard Status Register, KBSR[15] = 1 when there is input to be read.");
               register_view(ui, &mut emulator.memory[0xFE00], self.use_negative, self.display_base).on_hover_text(RichText::new("mem[0xFE00]").code()).on_hover_text("Keyboard Status Register, KBSR[15] = 1 when there is input to be read.");

               ui.label("DSR:").on_hover_text(RichText::new("mem[0xFE04]").code()).on_hover_text("Display Status Register, DSR[15] = 1 when display service is ready to display a new character (always 1 in this emulator).");
               register_view(ui, &mut emulator.memory[0xFE04], self.use_negative, self.display_base).on_hover_text(RichText::new("mem[0xFE04]").code()).on_hover_text("Display Status Register, DSR[15] = 1 when display service is ready to display a new character (always 1 in this emulator).");

               ui.label("DDR:").on_hover_text(RichText::new("mem[0xFE06]").code()).on_hover_text("Display Data Register, when DDR[7:0] is set we write the ASCII character contained to the output.");
               register_view(ui, &mut emulator.memory[0xFE06], self.use_negative, self.display_base).on_hover_text(RichText::new("mem[0xFE06]").code()).on_hover_text("Display Data Register, when DDR[7:0] is set we write the ASCII character contained to the output.");
            });
    }

    fn render_cycle_view(
        &mut self,
        ui: &mut egui::Ui,
        emulator: &mut Emulator,
        theme: &ThemeSettings,
    ) {
        let cycle_names = [
            "Fetch",
            "Decode",
            "Evaluate Address",
            "Fetch Operands",
            "Execute Operation",
            "Store Result",
        ];
        let current_cycle_index = match emulator.cpu_state {
            CpuState::Fetch => 0,
            CpuState::Decode => 1,
            CpuState::EvaluateAddress(_) => 2,
            CpuState::FetchOperands(_) => 3,
            CpuState::ExecuteOperation(_) => 4,
            CpuState::StoreResult(_) => 5,
        };

        let ir_formatted = &format!("{:#04X}", emulator.ir.get());
        let instr_txt = emulator
            .metadata
            .address_to_line
            .get(&emulator.currently_executing)
            .and_then(|x| {
                emulator
                    .metadata
                    .last_compiled_source
                    .get(*x - 1)
                    .map(String::as_str)
            })
            .unwrap_or(ir_formatted.as_str());
        ui.label(instr_txt);

        // Display cycle list
        ui.label(RichText::new("CPU Pipeline Stages:").strong());
        for (i, cycle_name_str) in cycle_names.iter().enumerate() {
            if i == current_cycle_index {
                ui.label(
                    RichText::new(format!("-> {cycle_name_str}"))
                        .strong()
                        .color(theme.cpu_state_active_color),
                );
            } else {
                ui.label(
                    RichText::new(format!("   {cycle_name_str}")).color(theme.secondary_text_color),
                );
            }

            ui.separator();
            for micro_op in emulator.execute_state.phase_ops(i).iter().skip(1) {
                ui.label(micro_op.display(theme, &ui.ctx().style()).into());
            }
            ui.separator();
        }
    }
}
