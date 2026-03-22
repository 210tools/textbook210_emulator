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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelpPane {
    instruction_fields: InstructionFields,
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
        }
    }
}

// --- PaneDisplay Implementation ---

impl PaneDisplay for HelpPane {
    fn render(&mut self, ui: &mut egui::Ui, _emulator: &mut Emulator, theme: &mut ThemeSettings) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            render_collapsible_section_with_id(
                ui,
                "LC-3 Emulator Guide",
                "help_main",
                |ui, _theme| render_general_help_ui(ui),
                theme,
            );
            render_collapsible_section_with_id(
                ui,
                "LC-3 Instruction Reference",
                "help_instruction_reference",
                |ui, theme| self.render_instruction_reference_ui(ui, theme),
                theme,
            );
            render_collapsible_section_with_id(
                ui,
                "LC-3 Cheatsheet & Examples",
                "help_cheatsheet_examples",
                render_cheatsheet_examples_ui,
                theme,
            );
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
            .heading()
            .strong()
            .color(theme.help_title_color),
    );
}

fn ui_section_heading(ui: &mut Ui, text: &str, theme: &ThemeSettings) {
    ui.label(
        RichText::new(text)
            .heading()
            .strong()
            .color(theme.help_heading_color),
    );
}

fn ui_sub_heading(ui: &mut Ui, text: &str, theme: &ThemeSettings) {
    ui.label(
        RichText::new(text)
            .heading()
            .strong()
            .color(theme.help_sub_heading_color),
    );
}

fn ui_strong_label(ui: &mut Ui, text: &str, theme: &ThemeSettings) {
    ui.label(RichText::new(text).strong().color(theme.strong_text_color));
}

fn ui_simple_label(ui: &mut Ui, text: &str) {
    ui.label(text);
}

fn ui_monospace_label(ui: &mut Ui, text: &str, theme: &ThemeSettings) {
    ui.label(
        RichText::new(text)
            .monospace()
            .color(theme.help_monospace_color),
    );
}

fn ui_code_block(ui: &mut Ui, code: &str, theme: &ThemeSettings) {
    egui::Frame::group(ui.style())
        .fill(theme.code_bg_color)
        .inner_margin(egui::Margin::same(5))
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

fn render_collapsible_section_with_id(
    ui: &mut Ui,
    title: &str,
    id_salt: &str,
    add_contents: impl FnOnce(&mut Ui, &mut ThemeSettings),
    theme: &mut ThemeSettings,
) {
    ui.push_id(id_salt, |ui| {
        let text_color = theme.help_collapsible_header_text_color;
        let header = egui::collapsing_header::CollapsingHeader::new(
            RichText::new(title).strong().color(text_color),
        );

        egui::Frame::new()
            .inner_margin(egui::Margin::same(4))
            .show(ui, |ui| {
                header.show(ui, |ui| {
                    egui::Frame::new()
                        .inner_margin(egui::Margin::same(4))
                        .show(ui, |ui| add_contents(ui, theme));
                });
            });
    });
}

// --- General Help UI ---

fn render_general_help_ui(ui: &mut Ui) {
    ui_simple_label(
        ui,
        "This emulator allows you to write, compile, and execute LC-3 assembly programs. Below is a guide to using the various features:",
    );
    ui.add_space(8.0);

    let sections: &[(&str, &[&str])] = &[
        (
            "Editor & Compilation",
            &[
                "Write your LC-3 assembly code in the editor.",
                "Click 'Compile' to assemble your code. Errors will appear below the editor.",
            ],
        ),
        (
            "Execution Controls",
            &[
                "'Speed': Ajust the cycles ber step and steps per second.",
                "'Run': Execute the program continuously.",
                "'Pause': Stop continuous execution.",
                "'Step': Execute one full instruction.",
                "'Micro Step': Execute a single micro-operation within an instruction's cycle.",
                "'Reset': Reload the last compiled program and reset the machine state.",
            ],
        ),
        (
            "Debugging",
            &[
                "The 'CPU State' pane shows registers, flags, and the current instruction cycle.",
                "The 'Memory' pane allows you to inspect and modify memory content and set break points.",
                "Set breakpoints by clicking the '🛑' button next to a line in the memory view.",
            ],
        ),
        (
            "Input/Output",
            &[
                "The 'I/O' pane displays program output.",
                "Type in the 'I/O' pane to provide input for GETC and IN trap calls.",
            ],
        ),
    ];

    for (title, items) in sections {
        ui.label(RichText::new(*title).strong());
        egui::Frame::group(ui.style()).show(ui, |ui| {
            for item in *items {
                ui.label(format!("• {item}"));
            }
        });
        ui.add_space(4.0);
    }
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

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui_sub_heading(ui, title, theme);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("📋").on_hover_text("Copy code").clicked() {
                    ui.ctx().copy_text(code.to_string());
                }
            });
        });
        ui_code_block(ui, code, theme);
    });
    ui.add_space(4.0);
}

fn render_cheatsheet_examples_ui(ui: &mut Ui, theme: &mut ThemeSettings) {
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
    ui.add_space(8.0);
    ui_section_heading(ui, "LC-3 Sample Programs", theme);
    ui_simple_label(
        ui,
        "Common patterns and examples for LC-3 assembly programming.",
    );
    ui.add_space(4.0);

    render_sample_program_card(
        ui,
        "Hello World",
        r#"; Simple Hello World program
.ORIG x3000
LEA R0, MESSAGE    ; Load address of the message
PUTS               ; Output the string
HALT               ; Halt the program

MESSAGE: .STRINGZ "Hello, World!"
.END"#,
        theme,
    );
    render_sample_program_card(
        ui,
        "Input and Echo",
        r#"; Program that gets a character and echoes it
.ORIG x3000
LOOP:
    GETC           ; Read a character from keyboard
    OUT            ; Echo the character
    BR LOOP        ; Repeat (BR is an alias for BRnzp)
.END"#,
        theme,
    );
    render_sample_program_card(
        ui,
        "Simple Counter",
        r#"; Program that counts from 1 to 5
.ORIG x3000
AND R1, R1, #0     ; Clear R1 (counter)
ADD R1, R1, #1     ; Start at 1
LD R3, ASCII_0     ; Load base char for print

LOOP:
    ADD R0, R1, R3  ; Convert to ASCII ('0' + number)
    OUT             ; Print the digit

    ADD R1, R1, #1  ; Increment counter
    ADD R2, R1, #-6 ; Check if counter > 5
    BRn LOOP        ; Continue if negative

HALT               ; Stop execution

ASCII_0 .FILL x30
.END"#,
        theme,
    );

    render_sample_program_card(
        ui,
        "Fibonacci Sequence",
        r#"; Calculate first 8 Fibonacci numbers
.ORIG x3000
AND R0, R0, #0     ; F(0) = 0
AND R1, R1, #0     ; F(1) = 1
ADD R1, R1, #1

AND R3, R3, #0     ; Counter
ADD R3, R3, #8     ; Calculate 8 numbers

LOOP:
    ADD R2, R0, R1  ; F(n) = F(n-1) + F(n-2)
    ADD R0, R1, #0  ; Shift: F(n-2) = F(n-1)
    ADD R1, R2, #0  ; Shift: F(n-1) = F(n)

    ADD R3, R3, #-1 ; Decrement counter
    BRp LOOP        ; Continue if positive

HALT
.END"#,
        theme,
    );

    render_sample_program_card(
        ui,
        "String Length Calculator",
        r#"; Calculate length of a null-terminated string
.ORIG x3000
LEA R1, STRING     ; Load string address
AND R0, R0, #0     ; Clear counter

LOOP:
    LDR R2, R1, #0  ; Load character
    BRz DONE        ; If null terminator, done
    ADD R0, R0, #1  ; Increment counter
    ADD R1, R1, #1  ; Next character
    BR LOOP

DONE:
    ; String length is now in R0
    HALT

STRING: .STRINGZ "Hello LC-3!"
.END"#,
        theme,
    );

    render_sample_program_card(
        ui,
        "Memory Fill Pattern",
        r#"; Fill memory locations with a pattern
.ORIG x3000
LEA R1, DATA_START ; Base address
AND R2, R2, #0     ; Clear pattern
ADD R2, R2, #10    ; Set pattern value
AND R3, R3, #0     ; Counter
ADD R3, R3, #5     ; Fill 5 locations

FILL_LOOP:
    STR R2, R1, #0  ; Store pattern
    ADD R1, R1, #1  ; Next address
    ADD R2, R2, #1  ; Increment pattern
    ADD R3, R3, #-1 ; Decrement counter
    BRp FILL_LOOP   ; Continue if positive

HALT

DATA_START: .BLKW 5 ; Reserve 5 words
.END"#,
        theme,
    );

    render_sample_program_card(
        ui,
        "Subroutine Example",
        r#"; Program demonstrating subroutine usage
.ORIG x3000
AND R0, R0, #0
ADD R0, R0, #7     ; Set R0 = 7
JSR DOUBLE         ; Call subroutine
HALT               ; R0 now contains 14

DOUBLE:
    ADD R0, R0, R0  ; Double the value in R0
    RET             ; Return to caller
.END"#,
        theme,
    );

    render_sample_program_card(
        ui,
        "Conditional Branching Demo",
        r#"; Demonstrate different branch conditions
.ORIG x3000
AND R0, R0, #0
ADD R0, R0, #-5    ; Set R0 = -5
BRn NEGATIVE       ; Branch if negative

ZERO_OR_POS:
    LEA R0, POS_MSG
    PUTS
    BR END

NEGATIVE:
    LEA R0, NEG_MSG
    PUTS

END:
    HALT

NEG_MSG: .STRINGZ "Number is negative"
POS_MSG: .STRINGZ "Number is zero or positive"
.END"#,
        theme,
    );
}

// --- Instruction Reference UI ---

fn format_binary(value: u16, width: usize) -> String {
    let value = value & !(0xFFFF << width);
    format!("{value:0width$b}")
}

fn ui_register_selector(ui: &mut Ui, value: &mut u8, label: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).strong());
        ui.add(egui::DragValue::new(value).range(0..=7).speed(0.1));
    });
}

fn ui_immediate_selector(ui: &mut Ui, value: &mut i8, bits: u8, label: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).strong());
        let min_val = -(1 << (bits - 1));
        let max_val = (1 << (bits - 1)) - 1;
        ui.add(
            egui::DragValue::new(value)
                .range(min_val..=max_val)
                .speed(0.1),
        );
    });
}

fn ui_offset_selector(ui: &mut Ui, value: &mut i16, bits: u8, label: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).strong());
        let min_val = -(1 << (bits - 1));
        let max_val = (1 << (bits - 1)) - 1;
        ui.add(
            egui::DragValue::new(value)
                .range(min_val..=max_val)
                .speed(0.1),
        );
    });
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

    ui.label(RichText::new("Instruction Format:").strong());
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(4))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (i, segment) in segments.iter().enumerate() {
                    if i > 0 {
                        ui.label(RichText::new("|").color(theme.secondary_text_color));
                    }
                    ui.label(
                        RichText::new(&segment.text)
                            .monospace()
                            .color(segment.color),
                    )
                    .on_hover_text(segment.description);
                }
            });
            ui.separator();
            ui.label(
                RichText::new(format!("= {full_binary_string}"))
                    .monospace()
                    .color(theme.primary_text_color),
            );
            ui.label(
                RichText::new(format!("= 0x{final_value:04X}"))
                    .monospace()
                    .color(theme.primary_text_color),
            );
        });
}

fn render_micro_op_plan<T: micro_op::MicroOpGenerator>(ui: &mut Ui, op: &T, theme: &ThemeSettings) {
    ui.collapsing("micro-operation plan", |ui| {
        ui.label(RichText::new("Micro-operation Plan:").strong());
        let plan = op.generate_plan();
        let mut phases = vec![
            CycleState::Fetch,
            CycleState::Decode,
            CycleState::EvaluateAddress,
            CycleState::FetchOperands,
            CycleState::Execute,
            CycleState::StoreResult,
        ];
        phases.retain(|p| plan.contains_key(p) && !plan[p].is_empty());

        egui::Frame::group(ui.style()).show(ui, |ui| {
            for phase in phases {
                if let Some(ops) = plan.get(&phase) {
                    ui.label(RichText::new(format!("{phase}:")).strong());
                    ui.indent(phase, |ui| {
                        for micro_op in ops {
                            // The EguiDisplay trait is perfect for this.
                            ui.label(micro_op.display(theme, ui.style()).into());
                        }
                    });
                }
            }
        });
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
        ui.add_space(8.0);
        ui.separator();
        ui_sub_heading(ui, title, theme);
        ui_simple_label(ui, description);
        ui.add_space(4.0);

        egui::Frame::group(ui.style()).show(ui, |ui| {
            add_content(ui, theme);
        });
    });
}

impl HelpPane {
    fn render_instruction_reference_ui(&mut self, ui: &mut Ui, theme: &mut ThemeSettings) {
        ui_main_title(ui, "Interactive Instruction Reference", theme);
        ui_simple_label(
            ui,
            "Select an instruction category to explore. Adjust instruction fields to see how they affect the generated machine code and the underlying micro-operations.",
        );
        ui.add_space(4.0);

        render_collapsible_section_with_id(
            ui,
            "Micro-operation Explanation",
            "micro_op_explanation",
            |ui, theme| {
                self.render_micro_op_explanation_table(ui, theme);
            },
            theme,
        );

        render_collapsible_section_with_id(
            ui,
            "Arithmetic & Logic",
            "arithmetic_logic",
            |ui, theme| {
                self.render_add_instruction(ui, theme);
                self.render_and_instruction(ui, theme);
                self.render_not_instruction(ui, theme);
            },
            theme,
        );

        render_collapsible_section_with_id(
            ui,
            "Data Movement",
            "data_movement",
            |ui, theme| {
                self.render_ld_instruction(ui, theme);
                self.render_ldi_instruction(ui, theme);
                self.render_ldr_instruction(ui, theme);
                self.render_lea_instruction(ui, theme);
                self.render_st_instruction(ui, theme);
                self.render_sti_instruction(ui, theme);
                self.render_str_instruction(ui, theme);
            },
            theme,
        );

        render_collapsible_section_with_id(
            ui,
            "Control Flow",
            "control_flow",
            |ui, theme| {
                self.render_br_instruction(ui, theme);
                self.render_jmp_ret_instruction(ui, theme);
                self.render_jsr_jsrr_instruction(ui, theme);
            },
            theme,
        );

        render_collapsible_section_with_id(
            ui,
            "System Operations",
            "system_ops",
            |ui, theme| {
                self.render_trap_instruction(ui, theme);
                self.render_rti_instruction(ui, theme);
            },
            theme,
        );
    }

    fn render_micro_op_explanation_table(&mut self, ui: &mut Ui, theme: &ThemeSettings) {
        ui_section_heading(ui, "Micro-operation Reference", theme);
        ui_simple_label(
            ui,
            "Understanding the micro-operations that make up each instruction cycle phase:",
        );
        ui.add_space(8.0);

        let micro_op_explanations = [
            (
                "Transfer Operations",
                vec![
                    (
                        "R(n) <- R(m)",
                        "Copy the value from register m to register n",
                    ),
                    (
                        "PC <- R(n)",
                        "Set the Program Counter to the value in register n",
                    ),
                    (
                        "MAR <- R(n)",
                        "Set Memory Address Register to value in register n (triggers memory read)",
                    ),
                    (
                        "MDR <- R(n)",
                        "Set Memory Data Register to value in register n",
                    ),
                    (
                        "R(n) <- MDR",
                        "Copy value from Memory Data Register to register n",
                    ),
                    ("R(n) <- PC", "Copy Program Counter value to register n"),
                    ("R(n) <- AluOut", "Copy ALU result to register n"),
                    ("TEMP <- R(n)", "Store register value in temporary storage"),
                    ("R(n) <- IMM(x)", "Load immediate value x into register n"),
                    ("R(n) <- C(x)", "Load constant value x into register n"),
                ],
            ),
            (
                "ALU Operations",
                vec![
                    (
                        "ALU_OUT <- R(a) + R(b)",
                        "Add values from two registers, store result in ALU output",
                    ),
                    (
                        "ALU_OUT <- R(a) + IMM(x)",
                        "Add register value and immediate, store in ALU output",
                    ),
                    (
                        "ALU_OUT <- R(a) & R(b)",
                        "Bitwise AND two register values, store in ALU output",
                    ),
                    (
                        "ALU_OUT <- NOT R(a)",
                        "Bitwise complement of register value, store in ALU output",
                    ),
                    ("ALU_OUT <- PC + C(1)", "Increment Program Counter by 1"),
                    (
                        "ALU_OUT <- PC + PCOFFSET(x)",
                        "Add PC-relative offset to Program Counter",
                    ),
                ],
            ),
            (
                "Memory Operations",
                vec![
                    (
                        "SET_FLAG(WriteMemory)",
                        "Trigger memory write operation using MAR and MDR",
                    ),
                    (
                        "MAR <- address",
                        "Set memory address for read/write (implicit read follows)",
                    ),
                    ("MDR <- data", "Set data to write to memory"),
                ],
            ),
            (
                "Control Flow",
                vec![
                    ("-> Fetch", "Transition to Fetch phase"),
                    ("-> Decode", "Transition to Decode phase"),
                    ("-> EvaluateAddress", "Transition to Evaluate Address phase"),
                    ("-> FetchOperands", "Transition to Fetch Operands phase"),
                    ("-> Execute", "Transition to Execute phase"),
                    ("-> StoreResult", "Transition to Store Result phase"),
                ],
            ),
            (
                "Condition Codes & Flags",
                vec![
                    (
                        "SET_CC(n)",
                        "Update condition codes (N, Z, P) based on register n",
                    ),
                    ("WRITE_MEM", "Flag indicating memory write should occur"),
                ],
            ),
            (
                "System Operations",
                vec![
                    ("PSR <- value", "Update Processor Status Register"),
                    (
                        "Custom operations",
                        "Conditional logic, privilege checks, stack operations",
                    ),
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
                                ui.label(description);
                                ui.end_row();
                            }
                        });
                });
            ui.add_space(12.0);
        }

        ui_strong_label(ui, "Execution Flow", theme);
        egui::Frame::group(ui.style())
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ui_simple_label(
                    ui,
                    "Each LC-3 instruction follows a 6-phase execution cycle:",
                );
                ui.add_space(4.0);

                let phases = [
                    (
                        "1. Fetch",
                        "Load instruction from memory at PC address into IR",
                    ),
                    ("2. Decode", "Identify opcode and extract operand fields"),
                    (
                        "3. Evaluate Address",
                        "Calculate memory addresses for operands (if needed)",
                    ),
                    (
                        "4. Fetch Operands",
                        "Read operands from registers or memory",
                    ),
                    (
                        "5. Execute",
                        "Perform the actual operation (ALU, branches, etc.)",
                    ),
                    (
                        "6. Store Result",
                        "Write results back to registers or memory",
                    ),
                ];

                for (phase, description) in phases {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(phase)
                                .strong()
                                .color(theme.help_sub_heading_color),
                        );
                        ui.label(description);
                    });
                }
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
                ui.separator();

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

                ui.separator();
                ui.label(RichText::new("Example Syntax:").strong());
                ui_monospace_label(ui, &syntax, theme);
                ui.separator();

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
                ui.separator();
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
                ui.separator();

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

                ui.separator();
                ui.label(RichText::new("Example Syntax:").strong());
                ui_monospace_label(ui, &syntax, theme);
                ui.separator();

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
                ui.separator();
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

                ui.separator();
                ui.label(RichText::new("Example Syntax:").strong());
                ui_monospace_label(ui, &syntax, theme);
                ui.separator();

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
                ui.separator();
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

                ui.separator();
                ui.label(RichText::new("Example Syntax:").strong());
                ui_monospace_label(ui, &syntax, theme);
                ui.separator();

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
                ui.separator();
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

                ui.separator();
                ui.label(RichText::new("Example Syntax:").strong());
                ui_monospace_label(ui, &syntax, theme);
                ui.separator();

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
                ui.separator();
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

                ui.separator();
                ui.label(RichText::new("Example Syntax:").strong());
                ui_monospace_label(ui, &syntax, theme);
                ui.separator();

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
                ui.separator();
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

                ui.separator();
                ui.label(RichText::new("Example Syntax:").strong());
                ui_monospace_label(ui, &syntax, theme);
                ui.separator();

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
                ui.separator();
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

                ui.separator();
                ui.label(RichText::new("Example Syntax:").strong());
                ui_monospace_label(ui, &syntax, theme);
                ui.separator();

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
                ui.separator();
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

                ui.separator();
                ui.label(RichText::new("Example Syntax:").strong());
                ui_monospace_label(ui, &syntax, theme);
                ui.separator();

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
                ui.separator();
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

                ui.separator();
                ui.label(RichText::new("Example Syntax:").strong());
                ui_monospace_label(ui, &syntax, theme);
                ui.separator();

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
                ui.separator();
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
                    ui.label(RichText::new("Condition Flags:").strong());
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

                ui.separator();
                ui.label(RichText::new("Example Syntax:").strong());
                ui_monospace_label(ui, &syntax, theme);
                ui.separator();

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
                ui.separator();
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
                ui.separator();

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

                ui.separator();
                ui.label(RichText::new("Example Syntax:").strong());
                ui_monospace_label(ui, &syntax, theme);
                ui.separator();

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
                ui.separator();
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
                ui.separator();

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

                ui.separator();
                ui.label(RichText::new("Example Syntax:").strong());
                ui_monospace_label(ui, &syntax, theme);
                ui.separator();

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
                ui.separator();
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

                ui.label(RichText::new("Common Routines:").strong());
                for (vec, name, _) in trap_options {
                    if ui.radio_value(&mut fields.trapvector, vec, name).clicked() {};
                }

                let syntax = format!("TRAP x{:02X}", fields.trapvector);
                let op = TrapOp::decode(EmulatorCell::new(
                    (0b1111 << 12) | (fields.trapvector as u16),
                ));

                ui.separator();
                ui.label(RichText::new("Example Syntax:").strong());
                ui_monospace_label(ui, &syntax, theme);
                ui.separator();

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
                ui.separator();
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
                ui.label(RichText::new("Example Syntax:").strong());
                ui_monospace_label(ui, "RTI", theme);
                ui.separator();

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
                ui.separator();
                render_micro_op_plan(ui, &op, theme);
            },
        );
    }
}
