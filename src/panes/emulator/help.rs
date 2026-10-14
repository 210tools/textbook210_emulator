use crate::panes::glossary::Explain;
use crate::{
    emulator::{
        micro_op::{self, CycleState, EguiDisplay},
        ops::{
            AddOp, AndOp, BrOp, JmpOp, JsrOp, LdOp, LdiOp, LdrOp, LeaOp, NotOp, Op, RtiOp, StOp,
            StiOp, StrOp, TrapOp,
        },
        Emulator, EmulatorCell,
    },
    panes::{Pane, PaneDisplay, PaneTree, RealPane},
    theme::ThemeSettings,
};
use egui::{Color32, RichText, Ui};
use serde::{Deserialize, Serialize};

use super::EmulatorPane;

// --- Data Structures ---

/// A request to show an instruction, e.g. from Ctrl+clicking it in the editor: the mnemonic, the
/// address the line assembled to (if any), and the line's text.
#[derive(Debug, Clone)]
pub struct ShowInstruction {
    pub mnemonic: String,
    pub address: Option<usize>,
    pub line: String,
}

/// Set by the editor; the app opens the Help pane, which takes it.
pub static SHOW_INSTRUCTION: std::sync::Mutex<Option<ShowInstruction>> =
    std::sync::Mutex::new(None);

/// The Instructions-tab entry that documents a mnemonic.
fn instruction_page(mnemonic: &str) -> Option<&'static str> {
    let m = mnemonic.to_ascii_uppercase();
    Some(match m.as_str() {
        "ADD" => "ADD",
        "AND" => "AND",
        "NOT" => "NOT",
        "LD" => "LD",
        "LDI" => "LDI",
        "LDR" => "LDR",
        "LEA" => "LEA",
        "ST" => "ST",
        "STI" => "STI",
        "STR" => "STR",
        "JMP" | "RET" => "JMP",
        "JSR" | "JSRR" => "JSR",
        "RTI" => "RTI",
        "TRAP" | "GETC" | "OUT" | "PUTS" | "IN" | "PUTSP" | "HALT" => "TRAP",
        _ if m.starts_with("BR") && m[2..].chars().all(|c| "NZP".contains(c)) => "BR",
        _ => return None,
    })
}

/// The opcode an Instructions-tab page encodes.
fn page_opcode(page: &str) -> u16 {
    match page {
        "ADD" => 0x1,
        "AND" => 0x5,
        "NOT" => 0x9,
        "LD" => 0x2,
        "LDI" => 0xA,
        "LDR" => 0x6,
        "LEA" => 0xE,
        "ST" => 0x3,
        "STI" => 0xB,
        "STR" => 0x7,
        "BR" => 0x0,
        "JMP" => 0xC,
        "JSR" => 0x4,
        "TRAP" => 0xF,
        "RTI" => 0x8,
        _ => 0xD,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct InstructionFields {
    dr: u8,
    sr1: u8,
    sr2: u8,
    imm5: i8,
    offset6: i8,
    offset9: i16,
    offset11: i16,
    base_r: u8,
    n_bit: bool,
    z_bit: bool,
    p_bit: bool,
    trapvector: u8,
    imm_mode: bool,
    jsr_mode: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
enum HelpTab {
    #[default]
    Guide,
    Instructions,
    Cheatsheet,
    Examples,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelpPane {
    instruction_fields: InstructionFields,
    #[serde(default)]
    tab: HelpTab,
    /// Instruction shown on the Instructions tab
    #[serde(default = "default_instr")]
    instr: String,
}

fn default_instr() -> String {
    "ADD".to_string()
}

impl Default for HelpPane {
    fn default() -> Self {
        Self {
            instruction_fields: InstructionFields {
                dr: 1,
                sr1: 2,
                sr2: 3,
                imm5: -5,
                offset6: -6,
                offset9: -9,
                offset11: 11,
                base_r: 4,
                n_bit: true,
                z_bit: false,
                p_bit: true,
                trapvector: 0x25,
                imm_mode: true,
                jsr_mode: true,
            },
            tab: HelpTab::Guide,
            instr: default_instr(),
        }
    }
}

// --- PaneDisplay Implementation ---

impl HelpPane {
    /// Show an instruction on the Instructions tab, with its operands as written in the program
    /// when the line assembles.
    pub fn show_instruction(&mut self, req: &ShowInstruction, emulator: &Emulator) {
        let Some(page) = instruction_page(&req.mnemonic) else {
            return;
        };
        self.tab = HelpTab::Instructions;
        self.instr = page.to_string();
        // the word the line assembled to, or the line assembled on its own (no labels needed)
        let assembled = req.address.map(|a| emulator.memory[a].get());
        let standalone = || {
            let code = format!(".ORIG x3000\n{}\n.END\n", req.line);
            Emulator::parse_program(&code, None)
                .ok()
                .and_then(|out| out.machine_code.first().copied())
        };
        let word = assembled
            .filter(|w| w >> 12 == page_opcode(page))
            .or_else(|| standalone().filter(|w| w >> 12 == page_opcode(page)));
        if let Some(w) = word {
            self.set_fields(w);
        }
    }

    fn set_fields(&mut self, w: u16) {
        let sext = |v: u16, bits: u32| ((v << (16 - bits)) as i16) >> (16 - bits);
        let f = &mut self.instruction_fields;
        f.dr = (w >> 9 & 7) as u8;
        f.sr1 = (w >> 6 & 7) as u8;
        f.base_r = (w >> 6 & 7) as u8;
        f.sr2 = (w & 7) as u8;
        f.imm_mode = w >> 5 & 1 == 1;
        f.imm5 = sext(w & 0x1F, 5) as i8;
        f.offset6 = sext(w & 0x3F, 6) as i8;
        f.offset9 = sext(w & 0x1FF, 9);
        f.offset11 = sext(w & 0x7FF, 11);
        f.n_bit = w >> 11 & 1 == 1;
        f.z_bit = w >> 10 & 1 == 1;
        f.p_bit = w >> 9 & 1 == 1;
        f.trapvector = (w & 0xFF) as u8;
        f.jsr_mode = w >> 11 & 1 == 1;
    }
}

impl PaneDisplay for HelpPane {
    fn render(&mut self, ui: &mut egui::Ui, _emulator: &mut Emulator, theme: &mut ThemeSettings) {
        if let Some(req) = SHOW_INSTRUCTION.lock().ok().and_then(|mut r| r.take()) {
            self.show_instruction(&req, _emulator);
        }
        // topic tabs
        egui::Frame::new()
            .inner_margin(egui::Margin {
                left: 16,
                right: 16,
                top: 8,
                bottom: 8,
            })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    crate::panes::ui_kit::segmented(
                        ui,
                        theme,
                        &mut self.tab,
                        &[
                            (
                                HelpTab::Guide,
                                "Using the IDE",
                                "How the editor, toolbar and panes work",
                            ),
                            (
                                HelpTab::Instructions,
                                "Instructions",
                                "Every LC-3 instruction, with an interactive encoder",
                            ),
                            (
                                HelpTab::Cheatsheet,
                                "Cheatsheet",
                                "Syntax, directives and traps at a glance",
                            ),
                            (
                                HelpTab::Examples,
                                "Examples",
                                "Small programs you can open and run",
                            ),
                        ],
                    );
                });
            });
        crate::panes::ui_kit::rule(ui, theme);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: 20,
                        right: 20,
                        top: 14,
                        bottom: 24,
                    })
                    .show(ui, |ui| {
                        // cap the line length, but never widen past a narrow pane
                        ui.set_max_width(ui.available_width().min(820.0));
                        ui.spacing_mut().item_spacing.y = 6.0;
                        match self.tab {
                            HelpTab::Guide => render_general_help_ui(ui, theme),
                            HelpTab::Instructions => {
                                self.render_instruction_reference_ui(ui, theme)
                            }
                            HelpTab::Cheatsheet => render_cheatsheet_ui(ui, theme),
                            HelpTab::Examples => render_examples_ui(ui, theme),
                        }
                    });
            });
    }

    fn title(&self) -> String {
        "Help".to_string()
    }

    fn children() -> PaneTree {
        PaneTree::Pane(
            "Help".to_string(),
            Pane::new(RealPane::EmulatorPanes(Box::new(EmulatorPane::Help(
                HelpPane::default(),
            )))),
        )
    }
}

// --- UI Helper Functions ---

fn ui_main_title(ui: &mut Ui, text: &str, theme: &ThemeSettings) {
    ui.label(
        RichText::new(text)
            .family(egui::FontFamily::Name("ui-semibold".into()))
            .size(20.0)
            .color(theme.help_title_color),
    );
}

fn ui_section_heading(ui: &mut Ui, text: &str, theme: &ThemeSettings) {
    ui.add_space(6.0);
    ui.label(
        RichText::new(text)
            .family(egui::FontFamily::Name("ui-semibold".into()))
            .size(16.0)
            .color(theme.help_heading_color),
    );
}

fn ui_sub_heading(ui: &mut Ui, text: &str, theme: &ThemeSettings) {
    ui.label(
        RichText::new(text)
            .family(egui::FontFamily::Name("ui-semibold".into()))
            .size(14.0)
            .color(theme.help_sub_heading_color),
    );
}

fn ui_strong_label(ui: &mut Ui, text: &str, theme: &ThemeSettings) {
    ui.label(RichText::new(text).strong().color(theme.strong_text_color));
}

fn ui_simple_label(ui: &mut Ui, text: &str) {
    crate::panes::glossary::text(ui, text);
}

fn ui_monospace_label(ui: &mut Ui, text: &str, theme: &ThemeSettings) {
    ui.label(
        RichText::new(text)
            .monospace()
            .color(theme.help_monospace_color),
    );
}

fn ui_code_block(ui: &mut Ui, code: &str, theme: &ThemeSettings) {
    egui::Frame::new()
        .fill(theme.window_background)
        .corner_radius(egui::CornerRadius::same(6))
        .stroke(egui::Stroke::new(1.0_f32, theme.separator_color))
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(code)
                        .monospace()
                        .color(theme.help_code_block_text_color),
                )
                .wrap_mode(egui::TextWrapMode::Extend),
            );
        });
}

// --- General Help UI ---

fn render_general_help_ui(ui: &mut Ui, theme: &ThemeSettings) {
    ui_main_title(ui, "Using the IDE", theme);
    ui_simple_label(
        ui,
        "Write LC-3 assembly in the editor, assemble it into memory, then run or step through it while watching the registers, memory and terminal.",
    );

    let key = |ui: &mut Ui, k: &str| {
        egui::Frame::new()
            .fill(theme.widget_fill_interactive)
            .corner_radius(egui::CornerRadius::same(3))
            .inner_margin(egui::Margin::symmetric(5, 1))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(k)
                        .monospace()
                        .size(12.0)
                        .color(theme.accent_color_primary),
                );
            });
    };
    let row = |ui: &mut Ui, keys: &[&str], text: &str| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for k in keys {
                key(ui, k);
            }
            ui.add_space(4.0);
            ui.label(text);
        });
    };

    ui_section_heading(ui, "1. Write", theme);
    ui_simple_label(ui, "Each open file is a tab above the editor. Use File → Open (Ctrl+O) to load an .asm file, or File → New for an empty one. Your open files are kept when you close the app.");
    row(
        ui,
        &["Ctrl+Space"],
        "Suggest instructions, registers and labels as you type",
    );
    row(
        ui,
        &["hover"],
        "Hover an instruction, register, number or label to see what it means",
    );
    row(ui, &["Ctrl+click"], "On a label: jump to where it is defined. On an instruction: open it in Help with this line's operands");
    row(ui, &["Ctrl+F", "Ctrl+H"], "Find and replace");
    row(ui, &["Ctrl+/"], "Comment or uncomment lines");
    row(
        ui,
        &["Shift+Alt+F"],
        "Line up labels, instructions and comments in columns",
    );
    ui_simple_label(ui, "Mistakes are underlined in red as you type, with the message at the end of the line and in the status bar at the bottom.");

    ui_section_heading(ui, "2. Assemble", theme);
    let web = cfg!(target_arch = "wasm32");
    if web {
        row(
            ui,
            &["Ctrl+B"],
            "Assemble the file and load it into memory (the Assemble button)",
        );
    } else {
        row(
            ui,
            &["Ctrl+B", "F7"],
            "Assemble the file and load it into memory (the Assemble button)",
        );
    }
    ui_simple_label(ui, "Assembling resets the machine. Programs from other tabs stay in memory, so you can load your code at x3000 and a data file at x3500 from two tabs and they work together. Assemble again after every change; the status bar warns you when the file has changed since it was loaded. Run → Unload all programs clears memory.");

    ui_section_heading(ui, "3. Run and debug", theme);
    if web {
        row(
            ui,
            &["Run", "Ctrl+Enter"],
            "Run, or pause a running program. Run assembles first if a loaded file changed",
        );
        row(
            ui,
            &["Over"],
            "Step over: one instruction, running any subroutine it calls to the end",
        );
        row(
            ui,
            &["Into"],
            "Step into: one instruction, entering subroutines",
        );
        row(
            ui,
            &["Out"],
            "Step out: run until the current subroutine returns",
        );
        row(
            ui,
            &["Micro"],
            "Micro step: one state of the state machine (one clock cycle)",
        );
        row(ui, &["Back"], "Back one state: the whole machine goes back a clock cycle, as far as the start of the current instruction");
        row(
            ui,
            &["⟲"],
            "Reset the machine and reload the loaded programs",
        );
    } else {
        row(
            ui,
            &["F5", "Ctrl+Enter"],
            "Run, or pause a running program. Run assembles first if a loaded file changed",
        );
        row(
            ui,
            &["F10"],
            "Step over: one instruction, running any subroutine it calls to the end",
        );
        row(
            ui,
            &["F11"],
            "Step into: one instruction, entering subroutines",
        );
        row(
            ui,
            &["Shift+F11"],
            "Step out: run until the current subroutine returns",
        );
        row(
            ui,
            &["F12"],
            "Micro step: one state of the state machine (one clock cycle)",
        );
        row(ui, &["Shift+F12"], "Back one state: the whole machine goes back a clock cycle, as far as the start of the current instruction");
        row(
            ui,
            &["Shift+F5"],
            "Reset the machine and reload the loaded programs",
        );
        row(ui, &["F1"], "All keyboard shortcuts");
    }
    ui_simple_label(ui, "Click the gutter next to a line number to set a breakpoint (a red dot); Run stops before that line. The assembler adds one on every HALT so you can inspect the registers before the OS takes over. The yellow arrow marks the next instruction.");
    ui_simple_label(ui, "Speed sets how fast Run goes, from 1 clock cycle a second up to 100 million; all the way right is Max, as fast as your computer can. Once it is too fast to watch (or the datapath is closed), Run executes whole instructions directly instead of state by state, about a hundred times faster, with the same results, cycle counts and interrupts. Breakpoints and Pause still stop it; the datapath shows the next instruction you step.");
    ui_simple_label(ui, "Skip OS (in the toolbar) runs OS service routines like PUTS and GETC in a single step. Turn it off to step into the OS: its source opens in a read-only tab with the same tools as the editor.");

    ui_section_heading(ui, "4. Inspect", theme);
    ui_simple_label(ui, "CPU shows the registers (changed ones are highlighted), the condition codes and the micro-operations of the current instruction. Click any value to edit it. Hover a register to see, in the editor, the line that last wrote it (amber) and last read it (blue); click its name to jump to the line that wrote it.");
    ui_simple_label(ui, "Arrows in the editor gutter link each branch and JSR to its label. The arrow from the instruction at the PC turns amber when that branch will be taken.");
    ui_simple_label(ui, "Memory lists every word with its label and disassembly; data from .FILL and .STRINGZ is shown as numbers and characters. Click a line in the editor to highlight the words it produced, or double-click a memory row to jump to its source line. Type an address (x3000) or a label in the search box to go there.");
    ui_simple_label(
        ui,
        "View → Datapath shows each state of the state machine as it runs, three ways:",
    );
    ui_simple_label(ui, "• Textbook: Figure C.8 as printed in the book. The wires the state uses light up with the value on them.");
    ui_simple_label(ui, "• Model: every part, laid out to be read, with its value inside. The IR is drawn as its 16 bits, and each field is sliced out of it by a bracket and a wire: register numbers to the register file, offsets to the sign extenders. Click PC, MAR, MDR or a register to change it.");
    ui_simple_label(ui, "• Focus: only the parts and wires this state uses, in the order the data flows, with what each part does (e.g. ADDER: x3001 + 3 = x3004). A register read and written in the same state appears as before and after.");
    ui_simple_label(ui, "Hover any wire to see what it carries. True state shows the hardware as it really is: every combinational part works all the time, so every wire up to a gate is lit with its value. A gate that is off stops its wire, and the bus carries only what an open gate drives. The state's own transfer still glows and flows on top. View → Layout → Large datapath gives the datapath most of the window.");
    ui_simple_label(ui, "Underlined words like BEN are jargon: hover one to see what it means. Its explanation underlines its own jargon, so you can move into the bubble and hover those too.");
    ui_simple_label(ui, "Terminal shows what the program prints. Click it and type to send keys to GETC and IN; keys typed early wait in a queue.");

    ui_section_heading(ui, "5. Interrupts or polling", theme);
    ui_simple_label(ui, "Run → Operating system picks how the OS reads the keyboard, so the two can be compared. Both give your program the same GETC and IN.");
    ui_simple_label(ui, "Interrupt-driven OS (the default): at boot the OS sets KBSR[14], interrupt enable. When a key arrives (KBSR[15]), the next state 18 sees INT, and instead of fetching, the machine goes through state 49: it pushes the PSR and PC on the supervisor stack, raises the priority to 4 and jumps through vector x80 to KBDINT. KBDINT copies the key into a buffer and returns with RTI to the instruction it interrupted. GETC takes keys from the buffer. When the buffer is empty, GETC (and IN) put the machine to sleep by setting MCR[14]: the clock stops, using no cycles, until the keyboard asks to interrupt. (MCR[14] is this emulator's addition to the LC-3, like a wait-for-interrupt instruction.)");
    ui_simple_label(ui, "Polling OS: KBSR[14] stays 0, so the keyboard never interrupts. GETC loops reading KBSR until bit 15 says a key is ready, and the program can do nothing else meanwhile. Compare the cycle count in the status bar while each OS waits for a key: with the polling OS it keeps climbing, with the interrupt OS it stops.");
    ui_simple_label(ui, "To watch an interrupt, turn off Skip OS, open the datapath (View → Datapath) and type a key while your program runs or is stepped: INT and the interrupt priority light up in state 18.");

    ui_section_heading(ui, "Layout", theme);
    ui_simple_label(ui, "Drag tabs to rearrange panes, or add panes with the + at the end of a tab bar. View → Reset layout restores the default arrangement. View → Zoom changes the size of everything.");
}

// --- Cheatsheet and Examples UI ---

fn render_cheatsheet_category(ui: &mut Ui, title: &str, items: &[&str], theme: &ThemeSettings) {
    ui_strong_label(ui, title, theme);
    for item in items {
        ui_monospace_label(ui, item, theme);
    }
    ui.add_space(8.0);
}

fn render_sample_program_card(ui: &mut Ui, title: &str, code: &str, theme: &mut ThemeSettings) {
    // Debug assertion to test that the sample code compiles without errors
    debug_assert!(
        {
            use crate::emulator::Emulator;
            match Emulator::parse_program(code, None) {
                Ok(_) => true,
                Err(err) => {
                    eprintln!("Sample program '{}' failed to compile: {:?}", title, err);
                    false
                }
            }
        },
        "Sample program '{}' should compile successfully. \n {}",
        title,
        code
    );

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui_sub_heading(ui, title, theme);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button(format!("{} Copy", egui_phosphor_icons::icons::COPY.0))
                .clicked()
            {
                ui.ctx().copy_text(code.to_string());
            }
            if ui
                .button(format!(
                    "{} Open in editor",
                    egui_phosphor_icons::icons::FILE_PLUS.0
                ))
                .clicked()
            {
                let file = format!("{}.asm", title.to_lowercase().replace(' ', "_"));
                if let Ok(mut q) = crate::panes::emulator::editor::OPENED_FILES.lock() {
                    q.push(crate::panes::emulator::editor::Document::new(file, code));
                }
            }
        });
    });
    ui_code_block(ui, code, theme);
}

fn render_cheatsheet_ui(ui: &mut Ui, theme: &mut ThemeSettings) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui_section_heading(ui, "Instruction Quick Reference", theme);
        ui_simple_label(
            ui,
            "A quick reference for common LC-3 instructions and syntax.",
        );
        ui.add_space(8.0);

        render_cheatsheet_category(
            ui,
            "Arithmetic/Logic:",
            &[
                "ADD R1, R2, R3    ; R1 = R2 + R3",
                "ADD R1, R2, #5    ; R1 = R2 + 5",
                "AND R1, R2, R3    ; R1 = R2 & R3",
                "NOT R1, R2        ; R1 = ~R2",
            ],
            theme,
        );
        render_cheatsheet_category(
            ui,
            "Data Movement:",
            &[
                "LD  R1, LABEL     ; R1 = Mem[PC+offset]",
                "LDI R1, LABEL     ; R1 = Mem[Mem[PC+offset]]",
                "LDR R1, R2, #5    ; R1 = Mem[R2+5]",
                "LEA R1, LABEL     ; R1 = PC+offset",
                "ST  R1, LABEL     ; Mem[PC+offset] = R1",
                "STI R1, LABEL     ; Mem[Mem[PC+offset]] = R1",
                "STR R1, R2, #5    ; Mem[R2+5] = R1",
            ],
            theme,
        );
        render_cheatsheet_category(
            ui,
            "Control Flow:",
            &[
                "BR  LABEL         ; Branch always (if n,z,p are set)",
                "BRn LABEL         ; Branch if negative",
                "JMP R1            ; PC = R1",
                "JSR LABEL         ; Call subroutine",
                "RET               ; Return from subroutine (JMP R7)",
            ],
            theme,
        );
        render_cheatsheet_category(
            ui,
            "System & TRAP:",
            &[
                "TRAP x20 / GETC   ; Read char into R0",
                "TRAP x21 / OUT    ; Write char from R0",
                "TRAP x22 / PUTS   ; Write string from R0",
                "TRAP x25 / HALT   ; Halt execution",
                "RTI               ; Return from interrupt",
            ],
            theme,
        );
        render_cheatsheet_category(
            ui,
            "Directives:",
            &[
                ".ORIG x3000       ; Program starting address",
                ".FILL #10         ; Allocate one word, initialized to value",
                ".BLKW 5           ; Allocate 5 words, initialized to zero",
                ".STRINGZ \"Text\"  ; Allocate null-terminated string",
                ".END              ; End of program",
            ],
            theme,
        );
    });
}

fn render_examples_ui(ui: &mut Ui, theme: &mut ThemeSettings) {
    ui_main_title(ui, "Example programs", theme);
    ui_simple_label(
        ui,
        "Open one in a new tab, then press Ctrl+B to assemble and Ctrl+Enter (or Run) to run it.",
    );
    ui.add_space(4.0);
    for (title, code) in crate::examples::EXAMPLES {
        render_sample_program_card(ui, title, code, theme);
    }
}

// --- Instruction Reference UI ---

fn format_binary(value: u16, width: usize) -> String {
    let value = value & !(0xFFFF << width);
    format!("{value:0width$b}")
}

fn field_label(ui: &mut Ui, label: &str) {
    let color = ui.visuals().weak_text_color();
    ui.allocate_ui_with_layout(
        egui::vec2(150.0, 20.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_width(150.0);
            ui.label(RichText::new(label).color(color));
        },
    );
}

fn ui_register_selector(ui: &mut Ui, value: &mut u8, label: &str) {
    ui.horizontal(|ui| {
        field_label(ui, label);
        ui.add(
            egui::DragValue::new(value)
                .range(0..=7)
                .speed(0.1)
                .prefix("R"),
        );
    });
}

fn ui_immediate_selector(ui: &mut Ui, value: &mut i8, bits: u8, label: &str) {
    ui.horizontal(|ui| {
        field_label(ui, label);
        let min_val = -(1 << (bits - 1));
        let max_val = (1 << (bits - 1)) - 1;
        ui.add(
            egui::DragValue::new(value)
                .range(min_val..=max_val)
                .speed(0.1)
                .prefix("#"),
        );
        ui.label(
            RichText::new(format!("{min_val} to {max_val}"))
                .small()
                .color(ui.visuals().weak_text_color()),
        );
    });
}

fn ui_offset_selector(ui: &mut Ui, value: &mut i16, bits: u8, label: &str) {
    ui.horizontal(|ui| {
        field_label(ui, label);
        let min_val = -(1 << (bits - 1));
        let max_val = (1 << (bits - 1)) - 1;
        ui.add(
            egui::DragValue::new(value)
                .range(min_val..=max_val)
                .speed(0.1)
                .prefix("#"),
        );
        ui.label(
            RichText::new(format!("{min_val} to {max_val}"))
                .small()
                .color(ui.visuals().weak_text_color()),
        );
    });
}

/// Small uppercase label above each part of an instruction card.
fn card_section(ui: &mut Ui, text: &str, theme: &ThemeSettings) {
    ui.add_space(10.0);
    ui.label(
        RichText::new(text.trim_end_matches(':').to_uppercase())
            .size(11.0)
            .strong()
            .color(theme.secondary_text_color),
    );
    ui.add_space(2.0);
}

struct BinarySegment {
    text: String,
    color: Color32,
    description: &'static str,
}

fn render_binary_representation_view(
    ui: &mut Ui,
    segments: Vec<BinarySegment>,
    theme: &ThemeSettings,
) {
    let full_binary_string: String = segments.iter().map(|s| s.text.clone()).collect();
    let final_value = u16::from_str_radix(&full_binary_string, 2).unwrap_or(0);

    card_section(ui, "Encoding", theme);
    let cell = egui::vec2(24.0, 26.0);
    let gap = 4.0;
    let width = 16.0 * cell.x + (segments.len().saturating_sub(1)) as f32 * gap;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, cell.y + 34.0), egui::Sense::hover());
    let p = ui.painter();
    let mut x = rect.min.x;
    let mut bit = 15i32;
    for seg in &segments {
        let w = seg.text.len() as f32 * cell.x;
        let seg_rect =
            egui::Rect::from_min_size(egui::pos2(x, rect.min.y + 14.0), egui::vec2(w, cell.y));
        p.rect_filled(
            seg_rect,
            egui::CornerRadius::same(4),
            seg.color.gamma_multiply(0.16),
        );
        p.rect_stroke(
            seg_rect,
            egui::CornerRadius::same(4),
            egui::Stroke::new(1.0_f32, seg.color.gamma_multiply(0.5)),
            egui::StrokeKind::Inside,
        );
        for (i, c) in seg.text.chars().enumerate() {
            let cx = x + (i as f32 + 0.5) * cell.x;
            p.text(
                egui::pos2(cx, seg_rect.center().y),
                egui::Align2::CENTER_CENTER,
                c,
                egui::FontId::monospace(14.0),
                seg.color,
            );
            // bit numbers above
            p.text(
                egui::pos2(cx, rect.min.y + 6.0),
                egui::Align2::CENTER_CENTER,
                bit.to_string(),
                egui::FontId::monospace(9.0),
                theme.secondary_text_color.gamma_multiply(0.7),
            );
            bit -= 1;
        }
        // field name below
        let name = match seg.description.split_once(':') {
            Some((n, _)) => n.trim(),
            None if seg.description.starts_with("Opcode") => "opcode",
            None => "",
        };
        let short = if name.len() > (seg.text.len() * 3).max(4) {
            ""
        } else {
            name
        };
        p.text(
            egui::pos2(x + w / 2.0, seg_rect.max.y + 10.0),
            egui::Align2::CENTER_CENTER,
            short,
            egui::FontId::proportional(11.0),
            theme.secondary_text_color,
        );
        let hover = ui.interact(
            seg_rect,
            ui.id().with(("seg", x as i32)),
            egui::Sense::hover(),
        );
        hover.explain(seg.description);
        x += w + gap;
    }
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("x{final_value:04X}"))
                .monospace()
                .strong()
                .color(theme.strong_text_color),
        );
        ui.label(
            RichText::new(format!(
                "{} {} {} {}",
                full_binary_string.get(0..4).unwrap_or(""),
                full_binary_string.get(4..8).unwrap_or(""),
                full_binary_string.get(8..12).unwrap_or(""),
                full_binary_string.get(12..16).unwrap_or("")
            ))
            .monospace()
            .color(theme.secondary_text_color),
        );
        ui.label(
            RichText::new(format!("#{}", final_value as i16))
                .monospace()
                .color(theme.secondary_text_color),
        );
    });
}

fn render_micro_op_plan<T: micro_op::MicroOpGenerator>(ui: &mut Ui, op: &T, theme: &ThemeSettings) {
    card_section(ui, "Micro-operations after fetch and decode", theme);
    let plan = op.generate_plan();
    let phases = [
        CycleState::EvaluateAddress,
        CycleState::FetchOperands,
        CycleState::Execute,
        CycleState::StoreResult,
    ];
    egui::Grid::new(ui.id().with("plan"))
        .num_columns(2)
        .spacing([16.0, 4.0])
        .show(ui, |ui| {
            for phase in phases {
                if let Some(ops) = plan.get(&phase).filter(|o| !o.is_empty()) {
                    ui.label(RichText::new(format!("{phase}")).color(theme.secondary_text_color));
                    ui.vertical(|ui| {
                        for micro_op in ops {
                            ui.label(micro_op.display(theme, ui.style()).into());
                        }
                    });
                    ui.end_row();
                }
            }
        });
}

fn render_instruction_card(
    ui: &mut Ui,
    title: &str,
    description: &str,
    theme: &mut ThemeSettings,
    add_content: impl FnOnce(&mut Ui, &mut ThemeSettings),
) {
    ui.push_id(title, |ui| {
        let (mnemonic, name) = title.split_once(" - ").unwrap_or((title, ""));
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(mnemonic)
                    .family(egui::FontFamily::Name("mono-bold".into()))
                    .size(22.0)
                    .color(theme.editor_opcode_color),
            );
            ui.label(
                RichText::new(name)
                    .size(16.0)
                    .color(theme.secondary_text_color),
            );
        });
        ui_simple_label(ui, description);
        ui.add_space(4.0);
        add_content(ui, theme);
        ui.add_space(16.0);
    });
}

impl HelpPane {
    fn render_instruction_reference_ui(&mut self, ui: &mut Ui, theme: &mut ThemeSettings) {
        ui_main_title(ui, "Instructions", theme);
        ui_simple_label(
            ui,
            "Pick an instruction. Change its operands to see the machine code and the micro-operations it runs.",
        );
        ui.add_space(6.0);

        // instruction picker, grouped like the textbook
        let groups: &[(&str, &[&str])] = &[
            ("Operate", &["ADD", "AND", "NOT"]),
            (
                "Data movement",
                &["LD", "LDI", "LDR", "LEA", "ST", "STI", "STR"],
            ),
            ("Control", &["BR", "JMP", "JSR", "TRAP", "RTI"]),
            ("Reference", &["Micro-ops"]),
        ];
        ui.horizontal_wrapped(|ui| {
            for (group, names) in groups {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(group.to_uppercase())
                            .size(10.5)
                            .strong()
                            .color(theme.secondary_text_color),
                    );
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 3.0;
                        for name in *names {
                            let on = self.instr == *name;
                            let (fg, bg) = if on {
                                (theme.on_accent(), theme.accent_color_primary)
                            } else {
                                (theme.primary_text_color, theme.widget_fill_interactive)
                            };
                            let r = ui.add(
                                egui::Button::new(RichText::new(*name).monospace().color(fg))
                                    .fill(bg)
                                    .corner_radius(egui::CornerRadius::same(4)),
                            );
                            if r.clicked() {
                                self.instr = name.to_string();
                            }
                        }
                    });
                });
                ui.add_space(12.0);
            }
        });
        ui.add_space(8.0);
        crate::panes::ui_kit::rule(ui, theme);
        ui.add_space(10.0);

        match self.instr.as_str() {
            "ADD" => self.render_add_instruction(ui, theme),
            "AND" => self.render_and_instruction(ui, theme),
            "NOT" => self.render_not_instruction(ui, theme),
            "LD" => self.render_ld_instruction(ui, theme),
            "LDI" => self.render_ldi_instruction(ui, theme),
            "LDR" => self.render_ldr_instruction(ui, theme),
            "LEA" => self.render_lea_instruction(ui, theme),
            "ST" => self.render_st_instruction(ui, theme),
            "STI" => self.render_sti_instruction(ui, theme),
            "STR" => self.render_str_instruction(ui, theme),
            "BR" => self.render_br_instruction(ui, theme),
            "JMP" => self.render_jmp_ret_instruction(ui, theme),
            "JSR" => self.render_jsr_jsrr_instruction(ui, theme),
            "TRAP" => self.render_trap_instruction(ui, theme),
            "RTI" => self.render_rti_instruction(ui, theme),
            _ => self.render_micro_op_explanation_table(ui, theme),
        }
    }

    fn render_micro_op_explanation_table(&mut self, ui: &mut Ui, theme: &ThemeSettings) {
        ui_section_heading(ui, "Micro-operation Reference", theme);
        ui_simple_label(
            ui,
            "The micro-operations the CPU pane and datapath show for each state of an instruction:",
        );
        ui.add_space(8.0);

        let micro_op_explanations = [
            (
                "States",
                vec![
                    ("; state 18", "Starts one state of the LC-3 state machine (Figures C.2 and C.7): one clock cycle. The ops after it are what that state does. Micro step runs one state"),
                    ("[BEN], [ACV], [INT]…", "A state that tests a condition picks the next state, e.g. BR's state 22 only runs when BEN is 1"),
                    ("-> Fetch, -> Execute…", "The phase of the instruction cycle the following states belong to. Between phases, a pending memory read or write happens"),
                ],
            ),
            (
                "Bus transfers",
                vec![
                    ("R0 <- R1", "A value goes over the bus (or a direct path) into a register: one gate drives the bus, one LD signal latches it"),
                    ("MAR <- PC", "Loading MAR starts a memory read; MDR gets M[MAR] at the end of the state"),
                    ("R1 <- MDR", "The word read from memory goes into a register"),
                    ("PC <- PC + 1", "The PC loads its own incrementer's output (PCMUX = PC+1)"),
                    ("MDR <- PC - 1", "GatePC-1: the return address an interrupt or TRAP pushes"),
                    ("MAR <- R6 - 1", "The stack pointer logic (SPMUX, GateSP) moves R6 by one for a push or pop"),
                    ("R6 <- Saved_SSP", "Switch to the supervisor stack (Saved_USP keeps the user's)"),
                ],
            ),
            (
                "ALU and address adder",
                vec![
                    ("ALU_OUT <- R1 + R2", "The ALU adds two registers (or a register and SEXT(imm5)), ANDs them, or NOTs one"),
                    ("ADDR <- PC + #off", "The address adder: ADDR1MUX = PC, ADDR2MUX = SEXT(PCoffset9/11)"),
                    ("ADDR <- R2 + #off", "The address adder: ADDR1MUX = BaseR, ADDR2MUX = SEXT(offset6) or 0"),
                    ("MAR <- ADDR", "The adder's result through MARMUX onto the bus"),
                    ("PC <- ADDR", "The adder's result straight into the PC (PCMUX = ADDER)"),
                ],
            ),
            (
                "Memory and condition codes",
                vec![
                    ("WRITE_MEM", "Write MDR to M[MAR] at the end of the state (R.W = WR)"),
                    ("SET_CC(R1)", "LOGIC sets N, Z or P from the value written to R1"),
                ],
            ),
            (
                "Traps, interrupts and exceptions",
                vec![
                    ("Table <- x00 / x01", "Which vector table: x00 traps, x01 interrupts and exceptions"),
                    ("Vector <- IR[7:0] / INTV / x00-x02", "Which entry: the trap vector, the device's vector (keyboard x80), or the exception's"),
                    ("MAR <- Table'Vector", "The table entry's address; the service routine's address is read from it"),
                    ("PSR[15] <- 0", "Supervisor mode (Set.Priv)"),
                    ("PSR[10:8] <- Interrupt Priority", "An interrupt raises the priority so lower priority devices wait"),
                    ("PSR <- MDR", "RTI restores the PSR it popped"),
                ],
            ),
        ];

        for (category, operations) in micro_op_explanations {
            ui_strong_label(ui, category, theme);
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    egui::Grid::new(format!("micro_op_table_{}", category))
                        .num_columns(2)
                        .striped(true)
                        .show(ui, |ui| {
                            for (operation, description) in operations {
                                ui.label(
                                    RichText::new(operation)
                                        .monospace()
                                        .color(theme.help_monospace_color),
                                );
                                crate::panes::glossary::text(ui, description);
                                ui.end_row();
                            }
                        });
                });
            ui.add_space(12.0);
        }

        ui_strong_label(ui, "The instruction cycle", theme);
        egui::Frame::group(ui.style())
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ui_simple_label(
                    ui,
                    "Every instruction starts with the same states (18, 33, 28, 30, 32: fetch it and decode it), then follows its own path through the state machine. The phases group those states:",
                );
                ui.add_space(4.0);
                let phases = [
                    ("Fetch", "MAR <- PC, PC <- PC + 1, MDR <- M[MAR], IR <- MDR. Or, if INT is set in state 18, an interrupt (state 49)"),
                    ("Decode", "Pick the opcode's states from IR[15:12], and set BEN"),
                    ("Evaluate address", "The address adder computes the address a load, store or jump needs"),
                    ("Fetch operands", "Read memory (LD, LDR, LDI…)"),
                    ("Execute", "The ALU's work, or a TRAP's or RTI's stack and vector steps"),
                    ("Store result", "Write a register (setting N, Z, P) or memory"),
                ];
                for (phase, description) in phases {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(phase).strong().color(theme.help_sub_heading_color));
                        crate::panes::glossary::text(ui, description);
                    });
                }
                ui.add_space(4.0);
                ui_simple_label(ui, "View → Datapath shows each state on Figure C.8 as it runs.");
            });
    }

    // --- Individual Instruction Rendering Functions ---

    fn render_add_instruction(&mut self, ui: &mut Ui, theme: &mut ThemeSettings) {
        let fields = &mut self.instruction_fields;
        render_instruction_card(
            ui,
            "ADD - Addition",
            "Adds two values and stores the result in a destination register. Sets condition codes (N, Z, P).",
            theme,
            |ui, theme| {
                ui.checkbox(&mut fields.imm_mode, "Immediate Mode");
                ui.add_space(2.0);

                let (syntax, op) = if fields.imm_mode {
                    ui_register_selector(ui, &mut fields.dr, "DR (Destination)");
                    ui_register_selector(ui, &mut fields.sr1, "SR1 (Source)");
                    ui_immediate_selector(ui, &mut fields.imm5, 5, "imm5 (Immediate)");
                    (
                        format!("ADD R{}, R{}, #{}", fields.dr, fields.sr1, fields.imm5),
                        AddOp::decode(EmulatorCell::new(
                            (0b0001 << 12)
                                | ((fields.dr as u16) << 9)
                                | ((fields.sr1 as u16) << 6)
                                | (1 << 5)
                                | (fields.imm5 as u16 & 0x1F),
                        )),
                    )
                } else {
                    ui_register_selector(ui, &mut fields.dr, "DR (Destination)");
                    ui_register_selector(ui, &mut fields.sr1, "SR1 (Source 1)");
                    ui_register_selector(ui, &mut fields.sr2, "SR2 (Source 2)");
                    (
                        format!("ADD R{}, R{}, R{}", fields.dr, fields.sr1, fields.sr2),
                        AddOp::decode(EmulatorCell::new(
                            (0b0001 << 12)
                                | ((fields.dr as u16) << 9)
                                | ((fields.sr1 as u16) << 6)
                                | (fields.sr2 as u16),
                        )),
                    )
                };

                ui.add_space(2.0);
                card_section(ui, "Assembly", theme);
                ui.label(RichText::new(&syntax).monospace().size(15.0).color(theme.strong_text_color));
                ui.add_space(2.0);

                let segments = if fields.imm_mode {
                    vec![
                        BinarySegment {
                            text: "0001".to_string(),
                            color: theme.opcode_color,
                            description: "Opcode for ADD",
                        },
                        BinarySegment {
                            text: format_binary(fields.dr as u16, 3),
                            color: theme.help_operand_color,
                            description: "DR: Destination Register",
                        },
                        BinarySegment {
                            text: format_binary(fields.sr1 as u16, 3),
                            color: theme.help_operand_color,
                            description: "SR1: Source Register 1",
                        },
                        BinarySegment {
                            text: "1".to_string(),
                            color: theme.help_binary_layout_fixed_bits_color,
                            description: "Mode: 1 for immediate",
                        },
                        BinarySegment {
                            text: format_binary(fields.imm5 as u16, 5),
                            color: theme.help_immediate_color,
                            description: "imm5: 5-bit immediate value",
                        },
                    ]
                } else {
                    vec![
                        BinarySegment {
                            text: "0001".to_string(),
                            color: theme.opcode_color,
                            description: "Opcode for ADD",
                        },
                        BinarySegment {
                            text: format_binary(fields.dr as u16, 3),
                            color: theme.help_operand_color,
                            description: "DR: Destination Register",
                        },
                        BinarySegment {
                            text: format_binary(fields.sr1 as u16, 3),
                            color: theme.help_operand_color,
                            description: "SR1: Source Register 1",
                        },
                        BinarySegment {
                            text: "000".to_string(),
                            color: theme.help_binary_layout_fixed_bits_color,
                            description: "Mode: 0 for register, plus unused bits",
                        },
                        BinarySegment {
                            text: format_binary(fields.sr2 as u16, 3),
                            color: theme.help_operand_color,
                            description: "SR2: Source Register 2",
                        },
                    ]
                };
                render_binary_representation_view(ui, segments, theme);
                ui.add_space(2.0);
                render_micro_op_plan(ui, &op, theme);
            },
        );
    }

    fn render_and_instruction(&mut self, ui: &mut Ui, theme: &mut ThemeSettings) {
        let fields = &mut self.instruction_fields;
        render_instruction_card(
            ui,
            "AND - Bitwise AND",
            "Performs a bitwise AND on two values and stores the result. Sets condition codes (N, Z, P).",
            theme,
            |ui, theme| {
                ui.checkbox(&mut fields.imm_mode, "Immediate Mode");
                ui.add_space(2.0);

                let (syntax, op) = if fields.imm_mode {
                    ui_register_selector(ui, &mut fields.dr, "DR (Destination)");
                    ui_register_selector(ui, &mut fields.sr1, "SR1 (Source)");
                    ui_immediate_selector(ui, &mut fields.imm5, 5, "imm5 (Immediate)");
                    (
                        format!("AND R{}, R{}, #{}", fields.dr, fields.sr1, fields.imm5),
                        AndOp::decode(EmulatorCell::new(
                            (0b0101 << 12)
                                | ((fields.dr as u16) << 9)
                                | ((fields.sr1 as u16) << 6)
                                | (1 << 5)
                                | (fields.imm5 as u16 & 0x1F),
                        )),
                    )
                } else {
                    ui_register_selector(ui, &mut fields.dr, "DR (Destination)");
                    ui_register_selector(ui, &mut fields.sr1, "SR1 (Source 1)");
                    ui_register_selector(ui, &mut fields.sr2, "SR2 (Source 2)");
                    (
                        format!("AND R{}, R{}, R{}", fields.dr, fields.sr1, fields.sr2),
                        AndOp::decode(EmulatorCell::new(
                            (0b0101 << 12)
                                | ((fields.dr as u16) << 9)
                                | ((fields.sr1 as u16) << 6)
                                | (fields.sr2 as u16),
                        )),
                    )
                };

                ui.add_space(2.0);
                card_section(ui, "Assembly", theme);
                ui.label(RichText::new(&syntax).monospace().size(15.0).color(theme.strong_text_color));
                ui.add_space(2.0);

                let segments = if fields.imm_mode {
                    vec![
                        BinarySegment {
                            text: "0101".to_string(),
                            color: theme.opcode_color,
                            description: "Opcode for AND",
                        },
                        BinarySegment {
                            text: format_binary(fields.dr as u16, 3),
                            color: theme.help_operand_color,
                            description: "DR: Destination Register",
                        },
                        BinarySegment {
                            text: format_binary(fields.sr1 as u16, 3),
                            color: theme.help_operand_color,
                            description: "SR1: Source Register 1",
                        },
                        BinarySegment {
                            text: "1".to_string(),
                            color: theme.help_binary_layout_fixed_bits_color,
                            description: "Mode: 1 for immediate",
                        },
                        BinarySegment {
                            text: format_binary(fields.imm5 as u16, 5),
                            color: theme.help_immediate_color,
                            description: "imm5: 5-bit immediate value",
                        },
                    ]
                } else {
                    vec![
                        BinarySegment {
                            text: "0101".to_string(),
                            color: theme.opcode_color,
                            description: "Opcode for AND",
                        },
                        BinarySegment {
                            text: format_binary(fields.dr as u16, 3),
                            color: theme.help_operand_color,
                            description: "DR: Destination Register",
                        },
                        BinarySegment {
                            text: format_binary(fields.sr1 as u16, 3),
                            color: theme.help_operand_color,
                            description: "SR1: Source Register 1",
                        },
                        BinarySegment {
                            text: "000".to_string(),
                            color: theme.help_binary_layout_fixed_bits_color,
                            description: "Mode: 0 for register, plus unused bits",
                        },
                        BinarySegment {
                            text: format_binary(fields.sr2 as u16, 3),
                            color: theme.help_operand_color,
                            description: "SR2: Source Register 2",
                        },
                    ]
                };
                render_binary_representation_view(ui, segments, theme);
                ui.add_space(2.0);
                render_micro_op_plan(ui, &op, theme);
            },
        );
    }

    fn render_not_instruction(&mut self, ui: &mut Ui, theme: &mut ThemeSettings) {
        let fields = &mut self.instruction_fields;
        render_instruction_card(
            ui,
            "NOT - Bitwise NOT",
            "Computes the bitwise complement of a register. Sets condition codes (N, Z, P).",
            theme,
            |ui, theme| {
                ui_register_selector(ui, &mut fields.dr, "DR (Destination)");
                ui_register_selector(ui, &mut fields.sr1, "SR (Source)");
                let syntax = format!("NOT R{}, R{}", fields.dr, fields.sr1);
                let op = NotOp::decode(EmulatorCell::new(
                    (0b1001 << 12)
                        | ((fields.dr as u16) << 9)
                        | ((fields.sr1 as u16) << 6)
                        | 0b111111,
                ));

                ui.add_space(2.0);
                card_section(ui, "Assembly", theme);
                ui.label(
                    RichText::new(&syntax)
                        .monospace()
                        .size(15.0)
                        .color(theme.strong_text_color),
                );
                ui.add_space(2.0);

                let segments = vec![
                    BinarySegment {
                        text: "1001".to_string(),
                        color: theme.opcode_color,
                        description: "Opcode for NOT",
                    },
                    BinarySegment {
                        text: format_binary(fields.dr as u16, 3),
                        color: theme.help_operand_color,
                        description: "DR: Destination Register",
                    },
                    BinarySegment {
                        text: format_binary(fields.sr1 as u16, 3),
                        color: theme.help_operand_color,
                        description: "SR: Source Register",
                    },
                    BinarySegment {
                        text: "111111".to_string(),
                        color: theme.help_binary_layout_fixed_bits_color,
                        description: "Unused bits, must be 1s",
                    },
                ];
                render_binary_representation_view(ui, segments, theme);
                ui.add_space(2.0);
                render_micro_op_plan(ui, &op, theme);
            },
        );
    }

    fn render_ld_instruction(&mut self, ui: &mut Ui, theme: &mut ThemeSettings) {
        let fields = &mut self.instruction_fields;
        render_instruction_card(
            ui,
            "LD - Load",
            "Loads a value from a memory location specified by a PC-relative offset. Sets condition codes (N, Z, P).",
            theme,
            |ui, theme| {
                ui_register_selector(ui, &mut fields.dr, "DR (Destination)");
                ui_offset_selector(ui, &mut fields.offset9, 9, "PCoffset9");
                let syntax = format!("LD R{}, LABEL ; (offset={})", fields.dr, fields.offset9);
                let op = LdOp::decode(EmulatorCell::new(
                    (0b0010 << 12) | ((fields.dr as u16) << 9) | (fields.offset9 as u16 & 0x1FF),
                ));

                ui.add_space(2.0);
                card_section(ui, "Assembly", theme);
                ui.label(RichText::new(&syntax).monospace().size(15.0).color(theme.strong_text_color));
                ui.add_space(2.0);

                let segments = vec![
                    BinarySegment {
                        text: "0010".to_string(),
                        color: theme.opcode_color,
                        description: "Opcode for LD",
                    },
                    BinarySegment {
                        text: format_binary(fields.dr as u16, 3),
                        color: theme.help_operand_color,
                        description: "DR: Destination Register",
                    },
                    BinarySegment {
                        text: format_binary(fields.offset9 as u16, 9),
                        color: theme.help_offset_color,
                        description: "PCoffset9: 9-bit PC-relative offset",
                    },
                ];
                render_binary_representation_view(ui, segments, theme);
                ui.add_space(2.0);
                render_micro_op_plan(ui, &op, theme);
            },
        );
    }

    fn render_ldi_instruction(&mut self, ui: &mut Ui, theme: &mut ThemeSettings) {
        let fields = &mut self.instruction_fields;
        render_instruction_card(
            ui,
            "LDI - Load Indirect",
            "Loads a value indirectly from memory. The address specified by the PC-relative offset contains the address of the data. Sets condition codes (N, Z, P).",
            theme,
            |ui, theme| {
                ui_register_selector(ui, &mut fields.dr, "DR (Destination)");
                ui_offset_selector(ui, &mut fields.offset9, 9, "PCoffset9");
                let syntax = format!("LDI R{}, LABEL ; (offset={})", fields.dr, fields.offset9);
                let op = LdiOp::decode(EmulatorCell::new(
                    (0b1010 << 12) | ((fields.dr as u16) << 9) | (fields.offset9 as u16 & 0x1FF),
                ));

                ui.add_space(2.0);
                card_section(ui, "Assembly", theme);
                ui.label(RichText::new(&syntax).monospace().size(15.0).color(theme.strong_text_color));
                ui.add_space(2.0);

                let segments = vec![
                    BinarySegment {
                        text: "1010".to_string(),
                        color: theme.opcode_color,
                        description: "Opcode for LDI",
                    },
                    BinarySegment {
                        text: format_binary(fields.dr as u16, 3),
                        color: theme.help_operand_color,
                        description: "DR: Destination Register",
                    },
                    BinarySegment {
                        text: format_binary(fields.offset9 as u16, 9),
                        color: theme.help_offset_color,
                        description: "PCoffset9: 9-bit PC-relative offset",
                    },
                ];
                render_binary_representation_view(ui, segments, theme);
                ui.add_space(2.0);
                render_micro_op_plan(ui, &op, theme);
            },
        );
    }

    fn render_ldr_instruction(&mut self, ui: &mut Ui, theme: &mut ThemeSettings) {
        let fields = &mut self.instruction_fields;
        render_instruction_card(
            ui,
            "LDR - Load Base+offset",
            "Loads a value from a memory location specified by a base register and an offset. Sets condition codes (N, Z, P).",
            theme,
            |ui, theme| {
                ui_register_selector(ui, &mut fields.dr, "DR (Destination)");
                ui_register_selector(ui, &mut fields.base_r, "BaseR (Base Register)");
                ui_immediate_selector(ui, &mut fields.offset6, 6, "offset6");
                let syntax = format!(
                    "LDR R{}, R{}, #{}",
                    fields.dr, fields.base_r, fields.offset6
                );
                let op = LdrOp::decode(EmulatorCell::new(
                    (0b0110 << 12)
                        | ((fields.dr as u16) << 9)
                        | ((fields.base_r as u16) << 6)
                        | (fields.offset6 as u16 & 0x3F),
                ));

                ui.add_space(2.0);
                card_section(ui, "Assembly", theme);
                ui.label(RichText::new(&syntax).monospace().size(15.0).color(theme.strong_text_color));
                ui.add_space(2.0);

                let segments = vec![
                    BinarySegment {
                        text: "0110".to_string(),
                        color: theme.opcode_color,
                        description: "Opcode for LDR",
                    },
                    BinarySegment {
                        text: format_binary(fields.dr as u16, 3),
                        color: theme.help_operand_color,
                        description: "DR: Destination Register",
                    },
                    BinarySegment {
                        text: format_binary(fields.base_r as u16, 3),
                        color: theme.help_operand_color,
                        description: "BaseR: Base Register",
                    },
                    BinarySegment {
                        text: format_binary(fields.offset6 as u16, 6),
                        color: theme.help_offset_color,
                        description: "offset6: 6-bit offset",
                    },
                ];
                render_binary_representation_view(ui, segments, theme);
                ui.add_space(2.0);
                render_micro_op_plan(ui, &op, theme);
            },
        );
    }

    fn render_lea_instruction(&mut self, ui: &mut Ui, theme: &mut ThemeSettings) {
        let fields = &mut self.instruction_fields;
        render_instruction_card(
            ui,
            "LEA - Load Effective Address",
            "Computes an address using a PC-relative offset and stores the address itself in a register. Sets condition codes (N, Z, P).",
            theme,
            |ui, theme| {
                ui_register_selector(ui, &mut fields.dr, "DR (Destination)");
                ui_offset_selector(ui, &mut fields.offset9, 9, "PCoffset9");
                let syntax = format!("LEA R{}, LABEL ; (offset={})", fields.dr, fields.offset9);
                let op = LeaOp::decode(EmulatorCell::new(
                    (0b1110 << 12) | ((fields.dr as u16) << 9) | (fields.offset9 as u16 & 0x1FF),
                ));

                ui.add_space(2.0);
                card_section(ui, "Assembly", theme);
                ui.label(RichText::new(&syntax).monospace().size(15.0).color(theme.strong_text_color));
                ui.add_space(2.0);

                let segments = vec![
                    BinarySegment {
                        text: "1110".to_string(),
                        color: theme.opcode_color,
                        description: "Opcode for LEA",
                    },
                    BinarySegment {
                        text: format_binary(fields.dr as u16, 3),
                        color: theme.help_operand_color,
                        description: "DR: Destination Register",
                    },
                    BinarySegment {
                        text: format_binary(fields.offset9 as u16, 9),
                        color: theme.help_offset_color,
                        description: "PCoffset9: 9-bit PC-relative offset",
                    },
                ];
                render_binary_representation_view(ui, segments, theme);
                ui.add_space(2.0);
                render_micro_op_plan(ui, &op, theme);
            },
        );
    }

    fn render_st_instruction(&mut self, ui: &mut Ui, theme: &mut ThemeSettings) {
        let fields = &mut self.instruction_fields;
        render_instruction_card(
            ui,
            "ST - Store",
            "Stores the value from a register into a memory location specified by a PC-relative offset.",
            theme,
            |ui, theme| {
                ui_register_selector(ui, &mut fields.sr1, "SR (Source)");
                ui_offset_selector(ui, &mut fields.offset9, 9, "PCoffset9");
                let syntax = format!("ST R{}, LABEL ; (offset={})", fields.sr1, fields.offset9);
                let op = StOp::decode(EmulatorCell::new(
                    (0b0011 << 12) | ((fields.sr1 as u16) << 9) | (fields.offset9 as u16 & 0x1FF),
                ));

                ui.add_space(2.0);
                card_section(ui, "Assembly", theme);
                ui.label(RichText::new(&syntax).monospace().size(15.0).color(theme.strong_text_color));
                ui.add_space(2.0);

                let segments = vec![
                    BinarySegment {
                        text: "0011".to_string(),
                        color: theme.opcode_color,
                        description: "Opcode for ST",
                    },
                    BinarySegment {
                        text: format_binary(fields.sr1 as u16, 3),
                        color: theme.help_operand_color,
                        description: "SR: Source Register",
                    },
                    BinarySegment {
                        text: format_binary(fields.offset9 as u16, 9),
                        color: theme.help_offset_color,
                        description: "PCoffset9: 9-bit PC-relative offset",
                    },
                ];
                render_binary_representation_view(ui, segments, theme);
                ui.add_space(2.0);
                render_micro_op_plan(ui, &op, theme);
            },
        );
    }

    fn render_sti_instruction(&mut self, ui: &mut Ui, theme: &mut ThemeSettings) {
        let fields = &mut self.instruction_fields;
        render_instruction_card(
            ui,
            "STI - Store Indirect",
            "Stores a value from a register into a memory location pointed to by an address in memory.",
            theme,
            |ui, theme| {
                ui_register_selector(ui, &mut fields.sr1, "SR (Source)");
                ui_offset_selector(ui, &mut fields.offset9, 9, "PCoffset9");
                let syntax = format!("STI R{}, LABEL ; (offset={})", fields.sr1, fields.offset9);
                let op = StiOp::decode(EmulatorCell::new(
                    (0b1011 << 12) | ((fields.sr1 as u16) << 9) | (fields.offset9 as u16 & 0x1FF),
                ));

                ui.add_space(2.0);
                card_section(ui, "Assembly", theme);
                ui.label(RichText::new(&syntax).monospace().size(15.0).color(theme.strong_text_color));
                ui.add_space(2.0);

                let segments = vec![
                    BinarySegment {
                        text: "1011".to_string(),
                        color: theme.opcode_color,
                        description: "Opcode for STI",
                    },
                    BinarySegment {
                        text: format_binary(fields.sr1 as u16, 3),
                        color: theme.help_operand_color,
                        description: "SR: Source Register",
                    },
                    BinarySegment {
                        text: format_binary(fields.offset9 as u16, 9),
                        color: theme.help_offset_color,
                        description: "PCoffset9: 9-bit PC-relative offset",
                    },
                ];
                render_binary_representation_view(ui, segments, theme);
                ui.add_space(2.0);
                render_micro_op_plan(ui, &op, theme);
            },
        );
    }

    fn render_str_instruction(&mut self, ui: &mut Ui, theme: &mut ThemeSettings) {
        let fields = &mut self.instruction_fields;
        render_instruction_card(
            ui,
            "STR - Store Base+offset",
            "Stores a value from a register into a memory location specified by a base register and an offset.",
            theme,
            |ui, theme| {
                ui_register_selector(ui, &mut fields.sr1, "SR (Source)");
                ui_register_selector(ui, &mut fields.base_r, "BaseR (Base Register)");
                ui_immediate_selector(ui, &mut fields.offset6, 6, "offset6");
                let syntax = format!(
                    "STR R{}, R{}, #{}",
                    fields.sr1, fields.base_r, fields.offset6
                );
                let op = StrOp::decode(EmulatorCell::new(
                    (0b0111 << 12)
                        | ((fields.sr1 as u16) << 9)
                        | ((fields.base_r as u16) << 6)
                        | (fields.offset6 as u16 & 0x3F),
                ));

                ui.add_space(2.0);
                card_section(ui, "Assembly", theme);
                ui.label(RichText::new(&syntax).monospace().size(15.0).color(theme.strong_text_color));
                ui.add_space(2.0);

                let segments = vec![
                    BinarySegment {
                        text: "0111".to_string(),
                        color: theme.opcode_color,
                        description: "Opcode for STR",
                    },
                    BinarySegment {
                        text: format_binary(fields.sr1 as u16, 3),
                        color: theme.help_operand_color,
                        description: "SR: Source Register",
                    },
                    BinarySegment {
                        text: format_binary(fields.base_r as u16, 3),
                        color: theme.help_operand_color,
                        description: "BaseR: Base Register",
                    },
                    BinarySegment {
                        text: format_binary(fields.offset6 as u16, 6),
                        color: theme.help_offset_color,
                        description: "offset6: 6-bit offset",
                    },
                ];
                render_binary_representation_view(ui, segments, theme);
                ui.add_space(2.0);
                render_micro_op_plan(ui, &op, theme);
            },
        );
    }

    fn render_br_instruction(&mut self, ui: &mut Ui, theme: &mut ThemeSettings) {
        let fields = &mut self.instruction_fields;
        render_instruction_card(
            ui,
            "BR - Branch",
            "Conditionally branches to a new address based on the state of the N, Z, and P condition flags.",
            theme,
            |ui, theme| {
                ui.horizontal(|ui| {
                    card_section(ui, "Condition flags", theme);
                    ui.checkbox(&mut fields.n_bit, "N (negative)");
                    ui.checkbox(&mut fields.z_bit, "Z (zero)");
                    ui.checkbox(&mut fields.p_bit, "P (positive)");
                });
                ui_offset_selector(ui, &mut fields.offset9, 9, "PCoffset9");

                let mut syntax = "BR".to_string();
                if fields.n_bit {
                    syntax.push('n');
                }
                if fields.z_bit {
                    syntax.push('z');
                }
                if fields.p_bit {
                    syntax.push('p');
                }
                if syntax == "BR" {
                    syntax.push_str(" (unconditional)");
                }
                syntax.push_str(&format!(" LABEL ; (offset={})", fields.offset9));

                let op = BrOp::decode(EmulatorCell::new(
                    ((fields.n_bit as u16) << 11)
                        | ((fields.z_bit as u16) << 10)
                        | ((fields.p_bit as u16) << 9)
                        | (fields.offset9 as u16 & 0x1FF),
                ));

                ui.add_space(2.0);
                card_section(ui, "Assembly", theme);
                ui.label(RichText::new(&syntax).monospace().size(15.0).color(theme.strong_text_color));
                ui.add_space(2.0);

                let segments = vec![
                    BinarySegment {
                        text: "0000".to_string(),
                        color: theme.opcode_color,
                        description: "Opcode for BR",
                    },
                    BinarySegment {
                        text: if fields.n_bit { "1" } else { "0" }.to_string(),
                        color: theme.help_strong_label_color,
                        description: "n: branch if negative",
                    },
                    BinarySegment {
                        text: if fields.z_bit { "1" } else { "0" }.to_string(),
                        color: theme.help_strong_label_color,
                        description: "z: branch if zero",
                    },
                    BinarySegment {
                        text: if fields.p_bit { "1" } else { "0" }.to_string(),
                        color: theme.help_strong_label_color,
                        description: "p: branch if positive",
                    },
                    BinarySegment {
                        text: format_binary(fields.offset9 as u16, 9),
                        color: theme.help_offset_color,
                        description: "PCoffset9: 9-bit PC-relative offset",
                    },
                ];
                render_binary_representation_view(ui, segments, theme);
                ui.add_space(2.0);
                render_micro_op_plan(ui, &op, theme);
            },
        );
    }

    fn render_jmp_ret_instruction(&mut self, ui: &mut Ui, theme: &mut ThemeSettings) {
        let fields = &mut self.instruction_fields;
        render_instruction_card(
            ui,
            "JMP / RET - Jump / Return",
            "Unconditionally jumps to an address stored in a register. RET is an alias for JMP R7.",
            theme,
            |ui, theme| {
                let mut is_ret = fields.base_r == 7;
                if ui.checkbox(&mut is_ret, "RET (Return) Mode").changed() {
                    fields.base_r = if is_ret { 7 } else { 0 };
                }
                ui.add_space(2.0);

                let (syntax, op) = if is_ret {
                    (
                        "RET".to_string(),
                        JmpOp::decode(EmulatorCell::new((0b1100 << 12) | (7 << 6))),
                    )
                } else {
                    ui_register_selector(ui, &mut fields.base_r, "BaseR (Target Register)");
                    (
                        format!("JMP R{}", fields.base_r),
                        JmpOp::decode(EmulatorCell::new(
                            (0b1100 << 12) | ((fields.base_r as u16) << 6),
                        )),
                    )
                };

                ui.add_space(2.0);
                card_section(ui, "Assembly", theme);
                ui.label(
                    RichText::new(&syntax)
                        .monospace()
                        .size(15.0)
                        .color(theme.strong_text_color),
                );
                ui.add_space(2.0);

                let segments = vec![
                    BinarySegment {
                        text: "1100".to_string(),
                        color: theme.opcode_color,
                        description: "Opcode for JMP/RET",
                    },
                    BinarySegment {
                        text: "000".to_string(),
                        color: theme.help_binary_layout_fixed_bits_color,
                        description: "Unused bits",
                    },
                    BinarySegment {
                        text: format_binary(fields.base_r as u16, 3),
                        color: theme.help_operand_color,
                        description: "BaseR: The register containing the target address",
                    },
                    BinarySegment {
                        text: "000000".to_string(),
                        color: theme.help_binary_layout_fixed_bits_color,
                        description: "Unused bits",
                    },
                ];
                render_binary_representation_view(ui, segments, theme);
                ui.add_space(2.0);
                render_micro_op_plan(ui, &op, theme);
            },
        );
    }

    fn render_jsr_jsrr_instruction(&mut self, ui: &mut Ui, theme: &mut ThemeSettings) {
        let fields = &mut self.instruction_fields;
        render_instruction_card(
            ui,
            "JSR / JSRR - Jump to Subroutine",
            "Jumps to a subroutine, saving the return address (PC) in R7.",
            theme,
            |ui, theme| {
                ui.checkbox(&mut fields.jsr_mode, "JSR Mode (PC-relative)");
                ui.add_space(2.0);

                let (syntax, op) = if fields.jsr_mode {
                    ui_offset_selector(ui, &mut fields.offset11, 11, "PCoffset11");
                    (
                        format!("JSR LABEL ; (offset={})", fields.offset11),
                        JsrOp::decode(EmulatorCell::new(
                            (0b0100 << 12) | (1 << 11) | (fields.offset11 as u16 & 0x7FF),
                        )),
                    )
                } else {
                    ui_register_selector(ui, &mut fields.base_r, "BaseR (Target Register)");
                    (
                        format!("JSRR R{}", fields.base_r),
                        JsrOp::decode(EmulatorCell::new(
                            (0b0100 << 12) | ((fields.base_r as u16) << 6),
                        )),
                    )
                };

                ui.add_space(2.0);
                card_section(ui, "Assembly", theme);
                ui.label(
                    RichText::new(&syntax)
                        .monospace()
                        .size(15.0)
                        .color(theme.strong_text_color),
                );
                ui.add_space(2.0);

                let segments = if fields.jsr_mode {
                    vec![
                        BinarySegment {
                            text: "0100".to_string(),
                            color: theme.opcode_color,
                            description: "Opcode for JSR/JSRR",
                        },
                        BinarySegment {
                            text: "1".to_string(),
                            color: theme.help_binary_layout_fixed_bits_color,
                            description: "Mode: 1 for JSR (PC-relative)",
                        },
                        BinarySegment {
                            text: format_binary(fields.offset11 as u16, 11),
                            color: theme.help_offset_color,
                            description: "PCoffset11: 11-bit PC-relative offset",
                        },
                    ]
                } else {
                    vec![
                        BinarySegment {
                            text: "0100".to_string(),
                            color: theme.opcode_color,
                            description: "Opcode for JSR/JSRR",
                        },
                        BinarySegment {
                            text: "000".to_string(),
                            color: theme.help_binary_layout_fixed_bits_color,
                            description: "Mode: 0 for JSRR (register), plus unused bits",
                        },
                        BinarySegment {
                            text: format_binary(fields.base_r as u16, 3),
                            color: theme.help_operand_color,
                            description: "BaseR: The register containing the target address",
                        },
                        BinarySegment {
                            text: "000000".to_string(),
                            color: theme.help_binary_layout_fixed_bits_color,
                            description: "Unused bits",
                        },
                    ]
                };
                render_binary_representation_view(ui, segments, theme);
                ui.add_space(2.0);
                render_micro_op_plan(ui, &op, theme);
            },
        );
    }

    fn render_trap_instruction(&mut self, ui: &mut Ui, theme: &mut ThemeSettings) {
        let fields = &mut self.instruction_fields;
        render_instruction_card(
            ui,
            "TRAP - System Call",
            "Executes a system call by transferring control to a routine specified by the trap vector.",
            theme,
            |ui, theme| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("trapvect8:").strong());
                    ui.add(
                        egui::DragValue::new(&mut fields.trapvector).hexadecimal(2, false, true),
                    );
                });

                let trap_options = [
                    (0x20, "GETC", "Read a single character from the keyboard."),
                    (0x21, "OUT", "Write a character to the console."),
                    (
                        0x22,
                        "PUTS",
                        "Write a null-terminated string to the console.",
                    ),
                    (0x23, "IN", "Print a prompt and read a character."),
                    (0x25, "HALT", "Halt program execution."),
                ];

                card_section(ui, "Common routines", theme);
                for (vec, name, _) in trap_options {
                    if ui.radio_value(&mut fields.trapvector, vec, name).clicked() {};
                }

                let syntax = format!("TRAP x{:02X}", fields.trapvector);
                let op = TrapOp::decode(EmulatorCell::new(
                    (0b1111 << 12) | (fields.trapvector as u16),
                ));

                ui.add_space(2.0);
                card_section(ui, "Assembly", theme);
                ui.label(RichText::new(&syntax).monospace().size(15.0).color(theme.strong_text_color));
                ui.add_space(2.0);

                let segments = vec![
                    BinarySegment {
                        text: "1111".to_string(),
                        color: theme.opcode_color,
                        description: "Opcode for TRAP",
                    },
                    BinarySegment {
                        text: "0000".to_string(),
                        color: theme.help_binary_layout_fixed_bits_color,
                        description: "Unused bits",
                    },
                    BinarySegment {
                        text: format_binary(fields.trapvector as u16, 8),
                        color: theme.help_operand_color,
                        description: "trapvect8: 8-bit trap vector number",
                    },
                ];
                render_binary_representation_view(ui, segments, theme);
                ui.add_space(2.0);
                render_micro_op_plan(ui, &op, theme);
            },
        );
    }

    fn render_rti_instruction(&mut self, ui: &mut Ui, theme: &mut ThemeSettings) {
        render_instruction_card(
            ui,
            "RTI - Return from Interrupt",
            "Returns control from an interrupt or trap routine, restoring the PC and PSR from the supervisor stack. This is a privileged instruction.",
            theme,
            |ui, theme| {
                let op = RtiOp::decode(EmulatorCell::new(0b1000 << 12));
                card_section(ui, "Assembly", theme);
                ui_monospace_label(ui, "RTI", theme);
                ui.add_space(2.0);

                let segments = vec![
                    BinarySegment {
                        text: "1000".to_string(),
                        color: theme.opcode_color,
                        description: "Opcode for RTI",
                    },
                    BinarySegment {
                        text: "000000000000".to_string(),
                        color: theme.help_binary_layout_fixed_bits_color,
                        description: "Unused bits",
                    },
                ];
                render_binary_representation_view(ui, segments, theme);
                ui.add_space(2.0);
                render_micro_op_plan(ui, &op, theme);
            },
        );
    }
}
