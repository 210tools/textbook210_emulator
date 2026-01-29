use crate::emulator::{Emulator, MAX_OS_STEPS};
use crate::panes::{Pane, PaneDisplay, PaneTree, RealPane};
use crate::theme::ThemeSettings;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ControlsPane {
    speed: u32,
}

impl Default for ControlsPane {
    fn default() -> Self {
        Self { speed: 30 }
    }
}

impl PaneDisplay for ControlsPane {
    fn render(&mut self, ui: &mut egui::Ui, emulator: &mut Emulator, theme: &mut ThemeSettings) {
        egui::ScrollArea::vertical().show(ui, |ui| {

            // Single Execution Speed Slider
            ui.label("Execution Speed:");
            let slider = egui::Slider::new(&mut self.speed, 1..=1000)
                .logarithmic(true)
                .text("speed");
            ui.add(slider).on_hover_text(
                "Controls how many clock cycles are executed per emulation step. And how often we do an emulation step",
            );
            emulator.speed = self.speed;
            if self.speed <= 60 {
                emulator.ticks_between_updates = 61 - self.speed; // 60..1
                emulator.speed = 1;
            } else {
                emulator.ticks_between_updates = 1;
                emulator.speed = self.speed - 59; // 2..942
            }
            ui.add_space(theme.item_spacing.y);

            // Skip OS emulation checkbox
            ui.checkbox(&mut emulator.skip_os_emulation, "Skip OS Routines").on_hover_text("Automatically step through OS code (PC < 0x3000) when stepping.");

            ui.separator();

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = theme.item_spacing.x;

                // Run/Pause Button
                if emulator.running() {
                    let pause_button = egui::Button::new("⏸ Pause")
                        .fill(theme.accent_color_secondary);
                    if ui.add(pause_button).clicked() {
                        emulator.stop_running();
                    }
                } else {

                        if emulator.pc.get() != 0x200 && emulator.halted
                            && ui.add(egui::Button::new("▶ Reset & Run").fill(theme.accent_color_primary)).on_hover_text("Set PC=0x200 and reset all the other registers to 0. Memory stays intact. MCR[15] is set to 1 (run state).").clicked() {
                                                    if emulator.halted {
                                                        emulator.halted = false;
                                                        *emulator = emulator.soft_reset();
                                                    }
                                                    emulator.start_running();
                                                }

                        if ui.add(egui::Button::new("▶ Run").fill(theme.accent_color_primary)).on_hover_text("MCR[15] is set to 1 (run state).").clicked() {
                            if emulator.halted {
                                emulator.halted = false;
                            }
                            emulator.start_running();
                        }
                        };


                // Micro Step Button
                let micro_step_button = egui::Button::new("⤵ Micro Step")
                    .fill(theme.accent_color_tertiary);
                if ui.add(micro_step_button)
                    .on_hover_text("Advance the emulator by one cycle. eg from Fetch -> Decode.")
                    .clicked() {
                    let old_running = emulator.running();
                    emulator.start_running(); // Temporarily set to running
                    let _ = emulator.micro_step();
                    if emulator.skip_os_emulation {
                        let mut os_steps = 0; // Counter for OS steps skipped in this action

                        while emulator.pc.get() < 0x3000
                            && os_steps < MAX_OS_STEPS
                            && emulator.running() // Check if HALT occurred
                        {
                            emulator.step(); // Use full step for skipping OS routines
                            os_steps += 1;
                        }

                    }
                    // Restore running state unless a HALT occurred during OS skip
                    if !old_running  && emulator.running() { // Was paused, auto-stepped, didn't HALT
                        emulator.stop_running();
                    }
                    // If it was running and HALTed, it will remain not running.
                    // If it was running and didn't HALT, it will remain running.
                }

                // Step Button
                let step_button =
                    egui::Button::new("➡ Step").fill(theme.accent_color_tertiary);
                if ui.add(step_button)
                    .on_hover_text("Advance the emulator by an instruction fast forwarding all cycles until next fetch.")
                    .clicked() {
                    let old_running = emulator.running();
                    emulator.start_running(); // Temporarily set to running
                    emulator.step();
                    if emulator.skip_os_emulation {
                        let mut os_steps = 0; // Counter for OS steps skipped in this action

                        while emulator.pc.get() < 0x3000
                            && os_steps < MAX_OS_STEPS
                            && emulator.running() // Check if HALT occurred
                        {
                            emulator.step(); // Use full step for skipping OS routines
                            os_steps += 1;
                        }

                    }
                    // Restore running state unless a HALT occurred during OS skip
                    if !old_running  && emulator.running() { // Was paused, auto-stepped, didn't HALT
                        emulator.stop_running();
                    }
                    // If it was running and HALTed, it will remain not running.
                    // If it was running and didn't HALT, it will remain running.
                }
            });

            ui.separator();


            let reset_button = egui::Button::new("🔄 Reset Emulator State")
                .fill(theme.accent_color_negative)
                .min_size(egui::vec2(ui.available_width() - theme.item_spacing.x * 2.0, 0.0)); // Full width button

            if ui.add(reset_button)
                .on_hover_text("Resets CPU, memory, and devices. Execution speed and Skip OS settings are preserved.")
                .clicked() {
                let current_skip_os = emulator.skip_os_emulation; // Preserve this setting
                let current_speed = emulator.speed; // Preserve speed setting

                *emulator = Emulator::new(); // Reset to default state
                emulator.skip_os_emulation = current_skip_os; // Restore
                emulator.speed = current_speed; // Restore

                emulator.metadata.last_compiled_source = Vec::new(); // This removes the compiled succsesfully message
            }
            ui.small("Resets CPU, memory, and devices. Execution speed and Skip OS settings are preserved.");
        });
    }

    fn title(&self) -> String {
        "Controls".to_string()
    }

    fn children() -> PaneTree {
        PaneTree::Pane(
            "Controls".to_string(),
            Pane::new(RealPane::EmulatorPanes(Box::new(
                super::EmulatorPane::Controls(ControlsPane::default()), // Ensure default is used
            ))),
        )
    }
}
