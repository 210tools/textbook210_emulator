use egui::{Color32, CornerRadius, RichText, ScrollArea, Stroke, Theme, Ui, Vec2};
use serde::{Deserialize, Serialize};

use crate::{
    emulator::Emulator,
    panes::{Pane, PaneDisplay, PaneTree, RealPane},
    theme::{BaseThemeChoice, ThemeSettings},
};

use super::ToolPanes;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThemeEditorPane {
    // Holds a temporary copy of settings for editing, applied on change
    live_settings: ThemeSettings,
    // To manage which base theme preset is selected for reset/save purposes
    selected_base_for_file_op: BaseThemeChoice,
}

impl Default for ThemeEditorPane {
    fn default() -> Self {
        Self {
            live_settings: ThemeSettings::dark_default(),
            selected_base_for_file_op: BaseThemeChoice::Dark,
        }
    }
}

impl ThemeEditorPane {
    fn apply_live_settings_to_global(
        &self,
        ctx: &egui::Context,
        global_settings: &mut ThemeSettings,
    ) {
        *global_settings = self.live_settings.clone();

        ctx.set_theme(match global_settings.base_theme {
            BaseThemeChoice::Light => Theme::Light,
            BaseThemeChoice::Dark => Theme::Dark,
        });

        let mut style = (*ctx.style()).clone();
        global_settings.apply_to_style(&mut style);
        ctx.set_style(style);
        ctx.request_repaint();
    }
}

impl PaneDisplay for ThemeEditorPane {
    fn render(&mut self, ui: &mut egui::Ui, _emulator: &mut Emulator, theme: &mut ThemeSettings) {
        // Refresh live_settings from global if they differ (e.g., theme changed elsewhere)
        // This is a simple way to keep them in sync. More complex logic might be needed
        // if we want to detect "unsaved changes" explicitly.
        let global_base_theme = theme.base_theme;
        if self.live_settings.base_theme != global_base_theme {
            self.live_settings = theme.clone();
            self.selected_base_for_file_op = global_base_theme;
        }

        ui.horizontal(|ui| {
            if ui.button("Reset to Dark").clicked() {
                self.live_settings = ThemeSettings::dark_default();
                self.selected_base_for_file_op = BaseThemeChoice::Dark;
                self.apply_live_settings_to_global(ui.ctx(), theme);
            }
            if ui.button("Reset to Light").clicked() {
                self.live_settings = ThemeSettings::light_default();
                self.selected_base_for_file_op = BaseThemeChoice::Light;
                self.apply_live_settings_to_global(ui.ctx(), theme);
            }

            // Conditional save button
            #[cfg(all(not(target_arch = "wasm32"), debug_assertions))]
            {
                ui.menu_button("Save Current to File...", |ui| {
                    if ui
                        .button("Save as Dark Theme (dark.ron)".to_string())
                        .clicked()
                    {
                        let mut settings_to_save = self.live_settings.clone();
                        settings_to_save.base_theme = BaseThemeChoice::Dark;
                        settings_to_save.save_to_ron_file();
                        ui.close();
                    }
                    if ui
                        .button("Save as Light Theme (light.ron)".to_string())
                        .clicked()
                    {
                        let mut settings_to_save = self.live_settings.clone();
                        settings_to_save.base_theme = BaseThemeChoice::Light;
                        settings_to_save.save_to_ron_file();
                        ui.close();
                    }
                });
            }
        });
        ui.separator();

        let mut settings_changed = false;

        egui::SidePanel::left("theme_settings_panel")
            .resizable(true)
            .default_width(ui.available_width() * 0.5)
            .show_inside(ui, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Theme Settings");

                    // Pass a mutable reference to self.live_settings and a flag
                    settings_changed |= render_settings_editor(ui, &mut self.live_settings);
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Theme Preview");
                render_preview_area(ui, &self.live_settings);
            });
        });

        if settings_changed {
            // If any setting was changed, apply it to the global theme
            self.apply_live_settings_to_global(ui.ctx(), theme);
        }
    }

    fn title(&self) -> String {
        "Theme Editor".to_string()
    }

    fn children() -> PaneTree {
        PaneTree::Pane(
            "Theme Editor".to_string(),
            Pane::new(RealPane::ToolPanes(Box::new(ToolPanes::ThemeEditor(
                Box::default(),
            )))),
        )
    }
}

// Helper functions for rendering specific setting types
// Returns true if any setting was changed

fn render_color_setting(ui: &mut Ui, label: &str, color: &mut Color32) -> bool {
    let mut chd = false;
    ui.horizontal(|ui| {
        ui.label(label);
        chd = ui.color_edit_button_srgba(color).changed()
    });
    chd
}

fn render_stroke_setting(ui: &mut Ui, label: &str, stroke: &mut Stroke) -> bool {
    let mut changed = false;
    ui.group(|ui| {
        ui.label(label);
        ui.horizontal(|ui| {
            ui.label("Width:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut stroke.width)
                        .speed(0.1)
                        .range(0.0..=10.0),
                )
                .changed();
            ui.label("Color:");
            changed |= ui.color_edit_button_srgba(&mut stroke.color).changed();
        });
    })
    .response
    .changed()
        || changed
}

fn render_vec2_setting(
    ui: &mut Ui,
    label: &str,
    vec: &mut Vec2,
    speed: f32,
    clamp_min: f32,
    clamp_max: f32,
) -> bool {
    let mut changed = false;
    ui.group(|ui| {
        ui.label(label);
        ui.horizontal(|ui| {
            ui.label("X:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut vec.x)
                        .speed(speed)
                        .range(clamp_min..=clamp_max),
                )
                .changed();
            ui.label("Y:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut vec.y)
                        .speed(speed)
                        .range(clamp_min..=clamp_max),
                )
                .changed();
        });
    })
    .response
    .changed()
        || changed
}

fn render_corner_radius_setting(ui: &mut Ui, label: &str, radius: &mut CornerRadius) -> bool {
    let mut changed = false;
    ui.group(|ui| {
        ui.label(label);
        ui.horizontal(|ui| {
            ui.label("NW:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut radius.nw)
                        .speed(0.1)
                        .range(0.0..=20.0),
                )
                .changed();
            ui.label("NE:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut radius.ne)
                        .speed(0.1)
                        .range(0.0..=20.0),
                )
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("SW:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut radius.sw)
                        .speed(0.1)
                        .range(0.0..=20.0),
                )
                .changed();
            ui.label("SE:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut radius.se)
                        .speed(0.1)
                        .range(0.0..=20.0),
                )
                .changed();
        });
        if ui.button("Set All Uniform").clicked() {
            let first = radius.nw;
            radius.ne = first;
            radius.sw = first;
            radius.se = first;
            changed = true;
        }
    })
    .response
    .changed()
        || changed
}

fn render_f32_setting(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    speed: f32,
    clamp_min: f32,
    clamp_max: f32,
) -> bool {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(
            egui::DragValue::new(value)
                .speed(speed)
                .range(clamp_min..=clamp_max),
        )
    })
    .response
    .changed()
}

fn render_base_theme_choice_setting(
    ui: &mut Ui,
    label: &str,
    choice: &mut BaseThemeChoice,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        changed |= ui
            .radio_value(choice, BaseThemeChoice::Light, "Light")
            .changed();
        changed |= ui
            .radio_value(choice, BaseThemeChoice::Dark, "Dark")
            .changed();
    });
    changed
}

fn render_settings_editor(ui: &mut Ui, settings: &mut ThemeSettings) -> bool {
    let mut changed = false;

    egui::CollapsingHeader::new("Base Theme")
        .default_open(false)
        .show(ui, |ui| {
            changed |= render_base_theme_choice_setting(
                ui,
                "Base Theme Preset:",
                &mut settings.base_theme,
            );
        });

    egui::CollapsingHeader::new("General UI Colors")
        .default_open(false)
        .show(ui, |ui| {
            changed |=
                render_color_setting(ui, "Primary Text Color:", &mut settings.primary_text_color);
            changed |= render_color_setting(
                ui,
                "Secondary Text Color:",
                &mut settings.secondary_text_color,
            );
            changed |=
                render_color_setting(ui, "Strong Text Color:", &mut settings.strong_text_color);
            changed |= render_color_setting(ui, "Hyperlink Color:", &mut settings.hyperlink_color);
            changed |=
                render_color_setting(ui, "Window Background:", &mut settings.window_background);
            changed |=
                render_color_setting(ui, "Panel Background:", &mut settings.panel_background);
            changed |=
                render_color_setting(ui, "Code Background Color:", &mut settings.code_bg_color);
        });

    egui::CollapsingHeader::new("Accent Colors")
        .default_open(false)
        .show(ui, |ui| {
            changed |=
                render_color_setting(ui, "Primary Accent:", &mut settings.accent_color_primary);
            changed |= render_color_setting(
                ui,
                "Secondary Accent:",
                &mut settings.accent_color_secondary,
            );
            changed |=
                render_color_setting(ui, "Tertiary Accent:", &mut settings.accent_color_tertiary);
            changed |=
                render_color_setting(ui, "Positive Accent:", &mut settings.accent_color_positive);
            changed |=
                render_color_setting(ui, "Negative Accent:", &mut settings.accent_color_negative);
        });

    egui::CollapsingHeader::new("Widget States")
        .default_open(false)
        .show(ui, |ui| {
            changed |=
                render_color_setting(ui, "Widget Text Color:", &mut settings.widget_text_color);
            changed |= render_color_setting(
                ui,
                "Fill Interactive:",
                &mut settings.widget_fill_interactive,
            );

            ui.separator();
            changed |= render_color_setting(ui, "Fill Hovered:", &mut settings.widget_fill_hovered);
            changed |= render_color_setting(ui, "Fill Active:", &mut settings.widget_fill_active);

            ui.separator();
            changed |=
                render_color_setting(ui, "Fill Disabled:", &mut settings.widget_fill_disabled);
            changed |= render_stroke_setting(
                ui,
                "Stroke Interactive:",
                &mut settings.bg_widget_stroke_interactive,
            );
            changed |= render_stroke_setting(
                ui,
                "Stroke Hovered:",
                &mut settings.bg_widget_stroke_hovered,
            );
            changed |=
                render_stroke_setting(ui, "Stroke Active:", &mut settings.bg_widget_stroke_active);
            changed |= render_stroke_setting(
                ui,
                "Stroke Disabled:",
                &mut settings.bg_widget_stroke_disabled,
            );

            ui.separator();

            changed |= render_stroke_setting(
                ui,
                "FG Stroke Interactive:",
                &mut settings.fg_widget_stroke_interactive,
            );
            changed |= render_stroke_setting(
                ui,
                "FG Stroke Hovered:",
                &mut settings.fg_widget_stroke_hovered,
            );
            changed |= render_stroke_setting(
                ui,
                "FG Stroke Active:",
                &mut settings.fg_widget_stroke_active,
            );
            changed |= render_stroke_setting(
                ui,
                "FG Stroke Disabled:",
                &mut settings.fg_widget_stroke_disabled,
            );
            ui.separator();

            changed |=
                render_stroke_setting(ui, "Selection Stroke:", &mut settings.selection_stroke);
        });

    egui::CollapsingHeader::new("Semantic Colors")
        .default_open(false)
        .show(ui, |ui| {
            changed |= render_color_setting(ui, "Error FG Color:", &mut settings.error_fg_color);
            changed |= render_color_setting(ui, "Error BG Color:", &mut settings.error_bg_color);
            changed |= render_color_setting(ui, "Warning FG Color:", &mut settings.warn_fg_color);
            changed |= render_color_setting(ui, "Warning BG Color:", &mut settings.warn_bg_color);
            changed |=
                render_color_setting(ui, "Success FG Color:", &mut settings.success_fg_color);
            changed |=
                render_color_setting(ui, "Success BG Color:", &mut settings.success_bg_color);
            changed |= render_color_setting(ui, "Info FG Color:", &mut settings.info_fg_color);
            changed |= render_color_setting(ui, "Info BG Color:", &mut settings.info_bg_color);
        });

    egui::CollapsingHeader::new("Debugging Colors")
        .default_open(false)
        .show(ui, |ui| {
            changed |= render_color_setting(ui, "PC Line BG:", &mut settings.pc_line_bg);
            changed |= render_color_setting(ui, "Breakpoint BG:", &mut settings.breakpoint_bg);
            changed |= render_color_setting(ui, "Highlight BG:", &mut settings.highlight_bg);
        });

    egui::CollapsingHeader::new("Memory View Colors")
        .default_open(false)
        .show(ui, |ui| {
            changed |=
                render_color_setting(ui, "Address Color:", &mut settings.memory_address_color);
            changed |= render_color_setting(ui, "Label Color:", &mut settings.memory_label_color);
            changed |= render_color_setting(ui, "Value Color:", &mut settings.memory_value_color);
            changed |= render_color_setting(ui, "ASCII Color:", &mut settings.memory_ascii_color);
            changed |= render_color_setting(
                ui,
                "Instruction Color:",
                &mut settings.memory_instruction_color,
            );
            ui.separator();
            changed |= render_color_setting(ui, "OS Code BG:", &mut settings.memory_os_code_bg);
            changed |= render_color_setting(ui, "OS Data BG:", &mut settings.memory_os_data_bg);
            changed |= render_color_setting(ui, "User Code BG:", &mut settings.memory_user_code_bg);
            changed |= render_color_setting(ui, "User Data BG:", &mut settings.memory_user_data_bg);
            changed |= render_color_setting(ui, "Stack BG:", &mut settings.memory_stack_bg);
            changed |= render_color_setting(ui, "Heap BG:", &mut settings.memory_heap_bg);
            changed |= render_color_setting(
                ui,
                "Device Registers BG:",
                &mut settings.memory_device_registers_bg,
            );
            changed |= render_color_setting(ui, "Unused BG:", &mut settings.memory_unused_bg);
            changed |= render_color_setting(
                ui,
                "Zone Label Color:",
                &mut settings.memory_zone_label_color,
            );
        });

    egui::CollapsingHeader::new("Registers View Colors")
        .default_open(false)
        .show(ui, |ui| {
            changed |= render_color_setting(ui, "Name Color:", &mut settings.register_name_color);
            changed |= render_color_setting(ui, "Value Color:", &mut settings.register_value_color);
            ui.separator();
            changed |= render_color_setting(
                ui,
                "Decoded Name Color:",
                &mut settings.register_decoded_name_color,
            );
            changed |= render_color_setting(
                ui,
                "Decoded Value Color:",
                &mut settings.register_decoded_value_color,
            );
            ui.separator();
            changed |= render_color_setting(
                ui,
                "Special Purpose Name Color:",
                &mut settings.register_special_purpose_name_color,
            );
            changed |= render_color_setting(
                ui,
                "Special Purpose Value Color:",
                &mut settings.register_special_purpose_value_color,
            );
            ui.separator();
            changed |= render_color_setting(
                ui,
                "Device Name Color:",
                &mut settings.register_device_name_color,
            );
            changed |= render_color_setting(
                ui,
                "Device Value Color:",
                &mut settings.register_device_value_color,
            );
        });

    egui::CollapsingHeader::new("CPU State View Colors")
        .default_open(false)
        .show(ui, |ui| {
            changed |=
                render_color_setting(ui, "Active Color:", &mut settings.cpu_state_active_color);
            changed |= render_color_setting(
                ui,
                "Inactive Color:",
                &mut settings.cpu_state_inactive_color,
            );
            changed |= render_color_setting(
                ui,
                "Description Color:",
                &mut settings.cpu_state_description_color,
            );
            changed |= render_color_setting(
                ui,
                "Data Flow Color:",
                &mut settings.cpu_state_data_flow_color,
            );
            changed |= render_color_setting(
                ui,
                "Active Register Highlight:",
                &mut settings.cpu_state_active_register_highlight,
            );
            changed |= render_color_setting(
                ui,
                "Active Memory Highlight:",
                &mut settings.cpu_state_active_memory_highlight,
            );
        });

    egui::CollapsingHeader::new("Terminal Colors")
        .default_open(false)
        .show(ui, |ui| {
            changed |= render_color_setting(ui, "Text Color:", &mut settings.terminal_text_color);
            changed |= render_color_setting(ui, "BG Color:", &mut settings.terminal_bg_color);
            changed |=
                render_color_setting(ui, "Cursor Color:", &mut settings.terminal_cursor_color);
            changed |= render_color_setting(
                ui,
                "Selection BG Color:",
                &mut settings.terminal_selection_bg_color,
            );
            changed |= render_color_setting(ui, "Link Color:", &mut settings.terminal_link_color);
        });

    egui::CollapsingHeader::new("Editor Colors")
        .default_open(false)
        .show(ui, |ui| {
            changed |= render_color_setting(ui, "Label Color:", &mut settings.editor_label_color);
            changed |=
                render_color_setting(ui, "Register Color:", &mut settings.editor_register_color);
            changed |=
                render_color_setting(ui, "Directive Color:", &mut settings.editor_directive_color);
            changed |= render_color_setting(ui, "Opcode Color:", &mut settings.editor_opcode_color);
            changed |=
                render_color_setting(ui, "Literal Color:", &mut settings.editor_literal_color);
            changed |= render_color_setting(ui, "String Color:", &mut settings.editor_string_color);
            changed |= render_color_setting(ui, "Char Color:", &mut settings.editor_char_color);
            changed |=
                render_color_setting(ui, "Comment Color:", &mut settings.editor_comment_color);
            ui.separator();
            changed |= render_color_setting(
                ui,
                "Error Underline:",
                &mut settings.editor_error_underline_color,
            );
            changed |= render_color_setting(
                ui,
                "Warning Underline:",
                &mut settings.editor_warning_underline_color,
            );
            changed |= render_color_setting(
                ui,
                "Matching Bracket BG:",
                &mut settings.editor_matching_bracket_bg_color,
            );
            ui.separator();
            changed |= render_color_setting(
                ui,
                "Current Line Number:",
                &mut settings.editor_current_line_number_color,
            );
            changed |= render_color_setting(
                ui,
                "Line Number Color:",
                &mut settings.editor_line_number_color,
            );
        });

    egui::CollapsingHeader::new("Help Pane Colors")
        .default_open(false)
        .show(ui, |ui| {
            changed |= render_color_setting(ui, "Opcode Color:", &mut settings.opcode_color);
            changed |= render_color_setting(ui, "Operand Color:", &mut settings.help_operand_color);
            changed |=
                render_color_setting(ui, "Immediate Color:", &mut settings.help_immediate_color);
            changed |= render_color_setting(ui, "Offset Color:", &mut settings.help_offset_color);
            changed |= render_color_setting(
                ui,
                "Binary Layout Fixed Bits:",
                &mut settings.help_binary_layout_fixed_bits_color,
            );
            changed |= render_color_setting(
                ui,
                "Pseudo Code Color:",
                &mut settings.help_pseudo_code_color,
            );
            ui.separator();
            changed |= render_color_setting(ui, "Title Color:", &mut settings.help_title_color);
            changed |= render_color_setting(ui, "Heading Color:", &mut settings.help_heading_color);
            changed |= render_color_setting(
                ui,
                "Sub Heading Color:",
                &mut settings.help_sub_heading_color,
            );
            changed |= render_color_setting(
                ui,
                "Strong Label Color:",
                &mut settings.help_strong_label_color,
            );
            changed |= render_color_setting(
                ui,
                "Italic Label Color:",
                &mut settings.help_italic_label_color,
            );
            changed |=
                render_color_setting(ui, "Monospace Color:", &mut settings.help_monospace_color);
            changed |= render_color_setting(
                ui,
                "Code Block Text Color:",
                &mut settings.help_code_block_text_color,
            );
            ui.separator();
            changed |= render_color_setting(
                ui,
                "Collapsible Header BG:",
                &mut settings.help_collapsible_header_bg_color,
            );
            changed |= render_color_setting(
                ui,
                "Collapsible Header Text:",
                &mut settings.help_collapsible_header_text_color,
            );
            changed |= render_color_setting(
                ui,
                "Info List Icon Color:",
                &mut settings.help_info_list_icon_color,
            );
            changed |= render_color_setting(ui, "Link Color:", &mut settings.help_link_color);
        });

    egui::CollapsingHeader::new("Look & Feel")
        .default_open(false)
        .show(ui, |ui| {
            changed |= render_vec2_setting(
                ui,
                "Item Spacing:",
                &mut settings.item_spacing,
                0.1,
                0.0,
                20.0,
            );
            changed |=
                render_corner_radius_setting(ui, "Widget Rounding:", &mut settings.widget_rounding);
            changed |=
                render_corner_radius_setting(ui, "Window Rounding:", &mut settings.window_rounding);
            changed |= render_f32_setting(
                ui,
                "Scroll Bar Width:",
                &mut settings.scroll_bar_width,
                0.1,
                1.0,
                30.0,
            );
            changed |= render_stroke_setting(ui, "Window Stroke:", &mut settings.window_stroke);
            changed |= render_stroke_setting(ui, "Panel Stroke:", &mut settings.panel_stroke);
            changed |= render_color_setting(ui, "Separator Color:", &mut settings.separator_color);
            changed |=
                render_color_setting(ui, "Tooltip BG Color:", &mut settings.tooltip_bg_color);
            changed |=
                render_color_setting(ui, "Tooltip Text Color:", &mut settings.tooltip_text_color);
            changed |=
                render_color_setting(ui, "Scrollbar BG Color:", &mut settings.scrollbar_bg_color);
            changed |= render_color_setting(
                ui,
                "Scrollbar Handle Color:",
                &mut settings.scrollbar_handle_color,
            );
            changed |= render_color_setting(
                ui,
                "Scrollbar Handle Hovered Color:",
                &mut settings.scrollbar_handle_hovered_color,
            );
            changed |= render_color_setting(
                ui,
                "Scrollbar Handle Active Color:",
                &mut settings.scrollbar_handle_active_color,
            );
            changed |= render_corner_radius_setting(
                ui,
                "Scrollbar Rounding:",
                &mut settings.scrollbar_rounding,
            );
            changed |= render_vec2_setting(
                ui,
                "Button Padding:",
                &mut settings.button_padding,
                0.1,
                0.0,
                20.0,
            );
            changed |= render_f32_setting(
                ui,
                "Indent Width:",
                &mut settings.indent_width,
                0.1,
                0.0,
                50.0,
            );
        });

    changed
}

fn render_preview_area(ui: &mut Ui, settings: &ThemeSettings) {
    // This function will render various egui widgets to preview the theme.
    // It uses the `settings` for direct styling and relies on `ui.style()` for implicit widget styling.

    #[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
    enum PreviewTab {
        General,
        Semantic,
        CodeDebug,
        Specialized,
        LookAndFeel,
    }

    let mut active_tab = ui.memory_mut(|mem| {
        *mem.data
            .get_persisted_mut_or_insert_with(egui::Id::new("preview_active_tab_v2"), || {
                PreviewTab::General
            })
    });

    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(&mut active_tab, PreviewTab::General, "General UI");
        ui.selectable_value(&mut active_tab, PreviewTab::Semantic, "Semantic");
        ui.selectable_value(&mut active_tab, PreviewTab::CodeDebug, "Code & Debug");
        ui.selectable_value(&mut active_tab, PreviewTab::Specialized, "Specialized");
        ui.selectable_value(&mut active_tab, PreviewTab::LookAndFeel, "Look & Feel");
    });
    ui.memory_mut(|mem| {
        mem.data
            .insert_persisted(egui::Id::new("preview_active_tab_v2"), active_tab)
    });

    ui.separator();

    ScrollArea::vertical().show(ui, |ui| match active_tab {
        PreviewTab::General => render_general_ui_preview(ui, settings),
        PreviewTab::Semantic => render_semantic_colors_preview(ui, settings),
        PreviewTab::CodeDebug => render_code_debug_preview(ui, settings),
        PreviewTab::Specialized => render_specialized_views_preview(ui, settings),
        PreviewTab::LookAndFeel => render_look_and_feel_preview(ui, settings),
    });
}

fn show_color_swatch(ui: &mut Ui, label: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.label(label);
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width().max(50.0), 20.0),
            egui::Sense::hover(),
        );
        ui.painter().rect_filled(rect, 0.0, color);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                color.r(),
                color.g(),
                color.b(),
                color.a()
            ),
            egui::FontId::monospace(10.0),
            ui.visuals().text_color(),
        );
    });
    ui.add_space(2.0);
}

fn render_general_ui_preview(ui: &mut Ui, settings: &ThemeSettings) {
    ui.heading("Text Styles");
    ui.label(RichText::new("Primary Text Color").color(settings.primary_text_color));
    ui.label(RichText::new("Secondary Text Color").color(settings.secondary_text_color));
    ui.label(
        RichText::new("Strong Text Color")
            .strong()
            .color(settings.strong_text_color),
    );
    ui.hyperlink_to("Hyperlink Color", "https://example.com")
        .on_hover_ui(|ui| {
            ui.label(
                RichText::new("Hyperlink uses hyperlink_color from theme")
                    .color(settings.hyperlink_color),
            );
        }); // Actual color is from egui's hyperlink setup

    ui.separator();
    ui.heading("Backgrounds");
    ui.label("Window Background:");
    egui::Frame::default()
        .fill(settings.window_background)
        .stroke(Stroke::new(1.0_f32, settings.primary_text_color))
        .inner_margin(5.0)
        .show(ui, |ui| {
            ui.label(
                RichText::new("Content on window_background").color(settings.primary_text_color),
            );
        });
    ui.label("Panel Background:");
    egui::Frame::default()
        .fill(settings.panel_background)
        .stroke(Stroke::new(1.0_f32, settings.primary_text_color))
        .inner_margin(5.0)
        .show(ui, |ui| {
            ui.label(
                RichText::new("Content on panel_background").color(settings.primary_text_color),
            );
        });

    ui.separator();
    ui.heading("Accent Colors");
    show_color_swatch(ui, "Primary Accent:", settings.accent_color_primary);
    show_color_swatch(ui, "Secondary Accent:", settings.accent_color_secondary);
    show_color_swatch(ui, "Tertiary Accent:", settings.accent_color_tertiary);
    show_color_swatch(ui, "Positive Accent:", settings.accent_color_positive);
    show_color_swatch(ui, "Negative Accent:", settings.accent_color_negative);

    ui.separator();
    ui.heading("Widget Preview");
    ui.label(
        RichText::new(
            "Widget Text Color (applied to text within widgets like buttons if not overridden)",
        )
        .color(settings.widget_text_color),
    );

    ui.group(|ui| {
        ui.label("Interactive Widgets (uses widget_fill_interactive, _hovered, _active, and stroke settings)");
        let _ = ui.button("Normal Button");
        let mut checkbox_val = true;
        ui.checkbox(&mut checkbox_val, "Checkbox");
        #[derive(PartialEq)] enum Radio { A, B }
        let mut radio_val = Radio::A;
        ui.horizontal(|ui| {
            ui.radio_value(&mut radio_val, Radio::A, "Radio A");
            ui.radio_value(&mut radio_val, Radio::B, "Radio B");
        });
        let mut slider_val = 50.0;
        ui.add(egui::Slider::new(&mut slider_val, 0.0..=100.0).text("Slider"));
        let mut text_edit_val = "Text Edit".to_string();
        ui.text_edit_singleline(&mut text_edit_val);
    });

    ui.group(|ui| {
        ui.label("Disabled Widgets (uses widget_fill_disabled and stroke settings)");
        ui.add_enabled(false, egui::Button::new("Disabled Button"));
        let mut checkbox_disabled_val = true;
        ui.add_enabled(
            false,
            egui::Checkbox::new(&mut checkbox_disabled_val, "Disabled Checkbox"),
        );
        let mut slider_disabled_val = 50.0;
        ui.add_enabled(
            false,
            egui::Slider::new(&mut slider_disabled_val, 0.0..=100.0).text("Disabled Slider"),
        );
    });

    ui.label("Selection Stroke (e.g. for selected text, not easily shown directly here):");
    show_color_swatch(
        ui,
        "Selection Stroke Color:",
        settings.selection_stroke.color,
    );
    ui.label(format!(
        "Selection Stroke Width: {}",
        settings.selection_stroke.width
    ));
}

fn show_semantic_block(ui: &mut Ui, label: &str, text_color: Color32, bg_color: Color32) {
    ui.label(format!("{label}:"));
    egui::Frame::NONE
        .fill(bg_color)
        .stroke(Stroke::new(1.0_f32, text_color.gamma_multiply(0.5)))
        .inner_margin(egui::Margin::same(5))
        .show(ui, |ui| {
            ui.label(
                RichText::new(format!("This is a {} message.", label.to_lowercase()))
                    .color(text_color),
            );
        });
    ui.add_space(5.0);
}

fn render_semantic_colors_preview(ui: &mut Ui, settings: &ThemeSettings) {
    ui.heading("Semantic Message Styles");
    show_semantic_block(
        ui,
        "Error",
        settings.error_fg_color,
        settings.error_bg_color,
    );
    show_semantic_block(
        ui,
        "Warning",
        settings.warn_fg_color,
        settings.warn_bg_color,
    );
    show_semantic_block(
        ui,
        "Success",
        settings.success_fg_color,
        settings.success_bg_color,
    );
    show_semantic_block(ui, "Info", settings.info_fg_color, settings.info_bg_color);
}

fn render_code_debug_preview(ui: &mut Ui, settings: &ThemeSettings) {
    ui.heading("Code Block & Editor Styles");
    ui.label("Code Background (code_bg_color):");
    egui::Frame::default()
        .fill(settings.code_bg_color)
        .inner_margin(5.0)
        .show(ui, |ui| {
            ui.label(
                RichText::new(".ORIG x3000 ; Start")
                    .monospace()
                    .color(settings.editor_comment_color),
            );
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("LBL")
                        .monospace()
                        .color(settings.editor_label_color),
                );
                ui.label(
                    RichText::new("ADD")
                        .monospace()
                        .color(settings.editor_opcode_color),
                );
                ui.label(
                    RichText::new("R1")
                        .monospace()
                        .color(settings.editor_register_color),
                );
                ui.label(
                    RichText::new(",")
                        .monospace()
                        .color(settings.primary_text_color),
                );
                ui.label(
                    RichText::new("R2")
                        .monospace()
                        .color(settings.editor_register_color),
                );
                ui.label(
                    RichText::new(",")
                        .monospace()
                        .color(settings.primary_text_color),
                );
                ui.label(
                    RichText::new("#5")
                        .monospace()
                        .color(settings.editor_literal_color),
                );
            });
            ui.label(
                RichText::new(".STRINGZ \"Hello\"")
                    .monospace()
                    .color(settings.editor_string_color),
            );
            ui.label(
                RichText::new(".FILL #'A'")
                    .monospace()
                    .color(settings.editor_char_color),
            );
            ui.label(
                RichText::new(".BLKW 5")
                    .monospace()
                    .color(settings.editor_directive_color),
            );
        });

    ui.separator();
    ui.heading("Debugging Highlights");
    ui.horizontal(|ui| {
        ui.label("PC Line BG:");
        show_color_swatch(ui, "", settings.pc_line_bg);
    });
    ui.horizontal(|ui| {
        ui.label("Breakpoint BG:");
        show_color_swatch(ui, "", settings.breakpoint_bg);
    });
    ui.horizontal(|ui| {
        ui.label("Highlight BG:");
        show_color_swatch(ui, "", settings.highlight_bg);
    });
    ui.label(
        RichText::new("Sample text with PC line background.")
            .background_color(settings.pc_line_bg)
            .color(settings.primary_text_color),
    );

    ui.separator();
    ui.heading("Editor UI Elements");
    ui.label(
        RichText::new("Error Underline")
            .underline()
            .color(settings.editor_error_underline_color),
    );
    ui.label(
        RichText::new("Warning Underline")
            .underline()
            .color(settings.editor_warning_underline_color),
    );
    ui.label(
        RichText::new("Matching Bracket BG")
            .background_color(settings.editor_matching_bracket_bg_color)
            .color(settings.primary_text_color),
    );
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("10 ")
                .color(settings.editor_line_number_color)
                .monospace(),
        );
        ui.label("Regular line number");
    });
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("11 ")
                .color(settings.editor_current_line_number_color)
                .strong()
                .monospace(),
        );
        ui.label("Current line number");
    });
}

fn show_key_value_text(
    ui: &mut Ui,
    key_color: Color32,
    key: &str,
    value_color: Color32,
    value: &str,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{key}: ")).color(key_color));
        ui.label(RichText::new(value).color(value_color));
    });
}

fn render_specialized_views_preview(ui: &mut Ui, settings: &ThemeSettings) {
    egui::CollapsingHeader::new("Memory View")
        .default_open(true)
        .show(ui, |ui| {
            show_key_value_text(
                ui,
                settings.memory_address_color,
                "x3000",
                settings.memory_value_color,
                "0xF025 (TRAP x25)",
            );
            show_key_value_text(
                ui,
                settings.memory_label_color,
                "MY_LABEL",
                settings.memory_instruction_color,
                "ADD R1, R2, R3",
            );
            ui.label(RichText::new("ASCII: .@#$").color(settings.memory_ascii_color));
            ui.separator();
            ui.label("Memory Zone Backgrounds & Label (memory_zone_label_color):");
            let zone_label_color = settings.memory_zone_label_color;
            show_semantic_block(ui, "OS Code", zone_label_color, settings.memory_os_code_bg);
            show_semantic_block(ui, "OS Data", zone_label_color, settings.memory_os_data_bg);
            show_semantic_block(
                ui,
                "User Code",
                zone_label_color,
                settings.memory_user_code_bg,
            );
            show_semantic_block(
                ui,
                "User Data",
                zone_label_color,
                settings.memory_user_data_bg,
            );
            show_semantic_block(ui, "Stack", zone_label_color, settings.memory_stack_bg);
            show_semantic_block(ui, "Heap", zone_label_color, settings.memory_heap_bg);
            show_semantic_block(ui, "Unused", zone_label_color, settings.memory_unused_bg);
        });

    egui::CollapsingHeader::new("Registers View")
        .default_open(true)
        .show(ui, |ui| {
            show_key_value_text(
                ui,
                settings.register_name_color,
                "R0",
                settings.register_value_color,
                "x1234",
            );
            show_key_value_text(
                ui,
                settings.register_decoded_name_color,
                "PC (Decoded)",
                settings.register_decoded_value_color,
                "USER_START",
            );
            show_key_value_text(
                ui,
                settings.register_special_purpose_name_color,
                "PSR",
                settings.register_special_purpose_value_color,
                "x8002 (N, User)",
            );
            show_key_value_text(
                ui,
                settings.register_device_name_color,
                "KBSR",
                settings.register_device_value_color,
                "x8000 (Ready)",
            );
        });

    egui::CollapsingHeader::new("CPU State View")
        .default_open(true)
        .show(ui, |ui| {
            ui.label(RichText::new("Active State Element").color(settings.cpu_state_active_color));
            ui.label(
                RichText::new("Inactive State Element").color(settings.cpu_state_inactive_color),
            );
            ui.label(RichText::new("Description Text").color(settings.cpu_state_description_color));
            ui.label(RichText::new("Data Flow Path ->").color(settings.cpu_state_data_flow_color));
            ui.label(
                RichText::new("Active Register Highlight")
                    .background_color(settings.cpu_state_active_register_highlight)
                    .color(settings.primary_text_color),
            );
            ui.label(
                RichText::new("Active Memory Highlight")
                    .background_color(settings.cpu_state_active_memory_highlight)
                    .color(settings.primary_text_color),
            );
        });

    egui::CollapsingHeader::new("Terminal View")
        .default_open(true)
        .show(ui, |ui| {
            egui::Frame::default()
                .fill(settings.terminal_bg_color)
                .inner_margin(5.0)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("Terminal text output.").color(settings.terminal_text_color),
                    );
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Selection")
                                .color(settings.terminal_text_color)
                                .background_color(settings.terminal_selection_bg_color),
                        );
                        ui.label(
                            RichText::new(" Cursor")
                                .color(settings.terminal_cursor_color)
                                .strong(),
                        );
                    });
                    ui.label(
                        RichText::new("Terminal Link")
                            .color(settings.terminal_link_color)
                            .underline(),
                    );
                });
        });

    egui::CollapsingHeader::new("Help Pane")
        .default_open(true)
        .show(ui, |ui| {
            ui.label(
                RichText::new("Help Title")
                    .color(settings.help_title_color)
                    .size(20.0),
            );
            ui.label(
                RichText::new("Help Heading")
                    .color(settings.help_heading_color)
                    .size(16.0),
            );
            ui.label(
                RichText::new("Help Sub-Heading")
                    .color(settings.help_sub_heading_color)
                    .size(14.0),
            );
            ui.label(
                RichText::new("Strong Label:")
                    .color(settings.help_strong_label_color)
                    .strong(),
            );
            ui.label(
                RichText::new("Italic Label:")
                    .color(settings.help_italic_label_color)
                    .italics(),
            );
            ui.label(
                RichText::new("Monospace Text")
                    .color(settings.help_monospace_color)
                    .monospace(),
            );
            ui.label(RichText::new("Opcode Example").color(settings.opcode_color));
            ui.label(RichText::new("Operand Example").color(settings.help_operand_color));
            ui.label(RichText::new("#Immediate").color(settings.help_immediate_color));
            ui.label(RichText::new("Offset[val]").color(settings.help_offset_color));
            ui.label(
                RichText::new("0101 Fixed Bits")
                    .color(settings.help_binary_layout_fixed_bits_color),
            );
            ui.label(
                RichText::new("R[dst] <- R[src1] + R[src2]").color(settings.help_pseudo_code_color),
            );
            egui::Frame::default()
                .fill(settings.code_bg_color)
                .show(ui, |ui| {
                    // Assuming help code block uses general code_bg_color
                    ui.label(
                        RichText::new("Code block text.")
                            .color(settings.help_code_block_text_color),
                    );
                });
            egui::Frame::default()
                .fill(settings.help_collapsible_header_bg_color)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("Collapsible Header Text")
                            .color(settings.help_collapsible_header_text_color),
                    );
                });
            ui.label(
                RichText::new("• Info List Item (icon color used for bullet/icon)")
                    .color(settings.help_info_list_icon_color),
            );
            ui.label(
                RichText::new("Help Link")
                    .color(settings.help_link_color)
                    .underline(),
            );
        });
}

fn render_look_and_feel_preview(ui: &mut Ui, settings: &ThemeSettings) {
    ui.heading("Spacing & Sizing");
    ui.label(format!("Item Spacing: {:?}", settings.item_spacing));
    ui.horizontal(|ui| {
        ui.label("A");
        ui.label("B");
    }); // Implicitly uses item_spacing.x
    ui.label(format!("Scroll Bar Width: {}", settings.scroll_bar_width));
    ui.label(format!("Indent Width: {}", settings.indent_width));
    ui.indent("indented_content", |ui| {
        ui.label("This content is indented.");
    });
    ui.label(format!("Button Padding: {:?}", settings.button_padding));
    let _ = ui.button("Button (check padding)");

    ui.separator();
    ui.heading("Rounding");
    ui.label(format!("Widget Rounding: {:?}", settings.widget_rounding));
    let _ = ui.button("Rounded Button?");
    ui.label(format!("Window Rounding: {:?}", settings.window_rounding));
    egui::Frame::popup(ui.style()).show(ui, |ui| {
        ui.label("Popup (uses window rounding)");
    });
    ui.label(format!(
        "Scrollbar Rounding: {:?}",
        settings.scrollbar_rounding
    ));

    ui.separator();
    ui.heading("Strokes & Separators");
    ui.label("Window Stroke:");
    egui::Frame::default()
        .stroke(settings.window_stroke)
        .inner_margin(5.0)
        .show(ui, |ui| {
            ui.label("Content with window_stroke");
        });
    ui.label("Panel Stroke:");
    egui::Frame::default()
        .stroke(settings.panel_stroke)
        .inner_margin(5.0)
        .show(ui, |ui| {
            ui.label("Content with panel_stroke");
        });
    ui.label("Separator Color (see line below):");
    // ui.separator(); // This uses its own logic, let's draw a line with the color
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 2.0), egui::Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        Stroke::new(1.0_f32, settings.separator_color),
    );

    ui.separator();
    ui.heading("Tooltips & Scrollbars");
    ui.button("Hover for Tooltip").on_hover_ui_at_pointer(|ui| {
        // Tooltip frame implicitly uses tooltip_bg_color
        // Tooltip text implicitly uses tooltip_text_color
        // For direct preview:
        egui::Frame::NONE
            .fill(settings.tooltip_bg_color)
            .inner_margin(3.0)
            .show(ui, |ui| {
                ui.label(RichText::new("This is a tooltip!").color(settings.tooltip_text_color));
            });
    });

    ui.label("Scrollbar (colors applied to area below):");
    ui.label(format!(
        "  BG: {:?}, Handle: {:?}, Hovered: {:?}, Active: {:?}",
        settings.scrollbar_bg_color,
        settings.scrollbar_handle_color,
        settings.scrollbar_handle_hovered_color,
        settings.scrollbar_handle_active_color
    ));

    ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
        for i in 0..20 {
            ui.label(format!("Scrollable content line {i}"));
        }
    });
}
