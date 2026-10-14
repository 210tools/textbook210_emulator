use egui::{Color32, CornerRadius, Stroke, Style, Vec2, Visuals};
#[cfg(all(not(target_arch = "wasm32"), debug_assertions))]
use ron;
use serde::{Deserialize, Serialize};

#[cfg(all(not(target_arch = "wasm32"), debug_assertions))]
use std::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BaseThemeChoice {
    Light,
    Dark,
    // Custom, // We can infer 'Custom' if the settings deviate significantly from Light/Dark defaults
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThemeSettings {
    pub base_theme: BaseThemeChoice,

    // --- General UI Colors ---
    pub primary_text_color: Color32,
    pub secondary_text_color: Color32, // For weaker/less important text, placeholders
    pub strong_text_color: Color32,    // For emphasized text, headings
    pub hyperlink_color: Color32,
    pub window_background: Color32,
    pub panel_background: Color32, // For ui.group, side panels, frames
    pub code_bg_color: Color32,    // Background for code blocks/editor view

    // --- General Accent Colors ---
    pub accent_color_primary: Color32, // Main brand/highlight color
    pub accent_color_secondary: Color32, // Secondary highlight or related action
    pub accent_color_tertiary: Color32, // Less prominent accent
    pub accent_color_positive: Color32, // For positive feedback, not just success messages
    pub accent_color_negative: Color32, // For negative feedback, not just error messages

    // Widget States (buttons, sliders, checkboxes, etc.)
    pub widget_text_color: Color32, // Text color on interactive widgets
    pub widget_fill_interactive: Color32, // Normal state
    pub widget_fill_hovered: Color32,
    pub widget_fill_active: Color32, // Clicked/dragged state
    pub widget_fill_disabled: Color32,

    // -- Widget Stroke Styles ---
    pub bg_widget_stroke_interactive: Stroke,
    pub bg_widget_stroke_hovered: Stroke,
    pub bg_widget_stroke_active: Stroke,
    pub bg_widget_stroke_disabled: Stroke,

    pub fg_widget_stroke_interactive: Stroke, // Foreground stroke for normal state
    pub fg_widget_stroke_hovered: Stroke,
    pub fg_widget_stroke_active: Stroke,
    pub fg_widget_stroke_disabled: Stroke,

    pub selection_stroke: Stroke,

    // Semantic Colors (for messages, highlights)
    pub error_fg_color: Color32,
    pub error_bg_color: Color32, // Optional background for error sections
    pub warn_fg_color: Color32,
    pub warn_bg_color: Color32,
    pub success_fg_color: Color32,
    pub success_bg_color: Color32,
    pub info_fg_color: Color32, // For general informational messages
    pub info_bg_color: Color32,

    // --- Emulator Specific Colors ---
    // General Emulator Highlights
    pub pc_line_bg: Color32, // Background for the line where PC is in memory/editor
    pub breakpoint_bg: Color32, // Background for breakpoint lines
    pub highlight_bg: Color32, // Background for temporarily highlighted items (e.g., jump target)

    // Memory Pane
    pub memory_address_color: Color32,
    pub memory_label_color: Color32,
    pub memory_value_color: Color32,
    pub memory_ascii_color: Color32,
    pub memory_instruction_color: Color32,
    pub memory_os_code_bg: Color32,
    pub memory_os_data_bg: Color32,
    pub memory_user_code_bg: Color32,
    pub memory_user_data_bg: Color32,
    pub memory_stack_bg: Color32,
    pub memory_heap_bg: Color32, // For dynamically allocated memory if supported/visualized
    pub memory_device_registers_bg: Color32,
    pub memory_unused_bg: Color32, // For memory regions not otherwise categorized
    pub memory_zone_label_color: Color32, // For legend or overlay text on zones

    // Registers Pane
    pub register_name_color: Color32, // General purpose registers (R0-R7)
    pub register_value_color: Color32,
    pub register_decoded_name_color: Color32, // For fields within a decoded register value
    pub register_decoded_value_color: Color32,
    pub register_special_purpose_name_color: Color32, // PC, IR, PSR etc.
    pub register_special_purpose_value_color: Color32,
    pub register_device_name_color: Color32, // KBSR, KBDR, DSR, DDR etc.
    pub register_device_value_color: Color32,

    // CPU State Pane
    pub cpu_state_active_color: Color32, // For the current CPU cycle step text/icon
    pub cpu_state_inactive_color: Color32, // For other CPU cycle steps text/icon
    pub cpu_state_description_color: Color32, // For the detailed text in CPU state
    pub cpu_state_data_flow_color: Color32, // For arrows or highlights showing data movement
    pub cpu_state_active_register_highlight: Color32, // Background highlight for active regs in detailed view
    pub cpu_state_active_memory_highlight: Color32, // Background highlight for active mem access in detailed view

    // IO Pane (Terminal)
    pub terminal_text_color: Color32,
    pub terminal_bg_color: Color32,
    pub terminal_cursor_color: Color32,
    pub terminal_selection_bg_color: Color32,
    pub terminal_link_color: Color32, // If terminal supports hyperlinks

    // Editor Pane (Syntax Highlighting)
    pub editor_label_color: Color32,
    pub editor_register_color: Color32,
    pub editor_directive_color: Color32,
    pub editor_opcode_color: Color32,
    pub editor_literal_color: Color32, // General for numbers
    pub editor_string_color: Color32,
    pub editor_char_color: Color32,
    pub editor_comment_color: Color32,
    pub editor_error_underline_color: Color32,
    pub editor_warning_underline_color: Color32,
    pub editor_matching_bracket_bg_color: Color32,
    pub editor_current_line_number_color: Color32,
    pub editor_line_number_color: Color32, // Default line number color

    // Help/Reference Pane Specific
    pub opcode_color: Color32,
    pub help_operand_color: Color32, // For DR, SR, BaseR etc.
    pub help_immediate_color: Color32,
    pub help_offset_color: Color32,
    pub help_binary_layout_fixed_bits_color: Color32, // For '0000' or '1' in binary formats
    pub help_pseudo_code_color: Color32,
    pub help_title_color: Color32,
    pub help_heading_color: Color32,
    pub help_sub_heading_color: Color32,
    pub help_strong_label_color: Color32,
    pub help_italic_label_color: Color32,
    pub help_monospace_color: Color32, // Default for non-semantic monospace
    pub help_code_block_text_color: Color32,
    pub help_collapsible_header_bg_color: Color32,
    pub help_collapsible_header_text_color: Color32,
    pub help_info_list_icon_color: Color32,
    pub help_link_color: Color32,

    // --- Look & Feel ---
    pub item_spacing: Vec2,
    pub widget_rounding: CornerRadius,
    pub window_rounding: CornerRadius,
    pub scroll_bar_width: f32,
    pub window_stroke: Stroke,
    pub panel_stroke: Stroke,
    pub separator_color: Color32,
    pub tooltip_bg_color: Color32,
    pub tooltip_text_color: Color32,
    pub scrollbar_bg_color: Color32,
    pub scrollbar_handle_color: Color32,
    pub scrollbar_handle_hovered_color: Color32,
    pub scrollbar_handle_active_color: Color32,
    pub scrollbar_rounding: CornerRadius,
    pub button_padding: Vec2,
    pub indent_width: f32,
}

impl ThemeSettings {
    /// The default theme: a cool, low-glare dark palette with amber reserved for execution
    /// (the PC, the run button) and blue for selection and focus.
    pub fn dark_default() -> Self {
        let rgb = Color32::from_rgb;
        let bg0 = rgb(15, 18, 22); // window
        let bg1 = rgb(21, 25, 32); // panels
        let bg2 = rgb(27, 32, 41); // raised
        let bg3 = rgb(35, 41, 54); // hover
        let line = rgb(38, 44, 55);
        let fg = rgb(213, 218, 227);
        let fg_dim = rgb(138, 147, 164);
        let fg_faint = rgb(90, 98, 115);
        let amber = rgb(231, 163, 62);
        let blue = rgb(77, 124, 236);
        let green = rgb(91, 191, 134);
        let red = rgb(236, 104, 96);
        let violet = rgb(199, 146, 234);
        let teal = rgb(86, 194, 201);
        let yellow = rgb(242, 212, 138);
        let syn_blue = rgb(127, 167, 255);
        let syn_reg = rgb(232, 176, 106);
        let syn_str = rgb(156, 204, 122);

        Self {
            base_theme: BaseThemeChoice::Dark,

            primary_text_color: fg,
            secondary_text_color: fg_dim,
            strong_text_color: rgb(240, 243, 248),
            hyperlink_color: syn_blue,
            window_background: bg0,
            panel_background: bg1,
            code_bg_color: bg2,

            accent_color_primary: amber,
            accent_color_secondary: blue,
            accent_color_tertiary: bg3,
            accent_color_positive: green,
            accent_color_negative: red,

            widget_text_color: fg,
            widget_fill_interactive: rgb(31, 37, 48),
            widget_fill_hovered: rgb(40, 48, 64),
            widget_fill_active: rgb(48, 58, 80),
            widget_fill_disabled: bg2,

            bg_widget_stroke_interactive: Stroke::new(1.0_f32, rgb(42, 49, 61)),
            bg_widget_stroke_hovered: Stroke::new(1.0_f32, rgb(58, 68, 86)),
            bg_widget_stroke_active: Stroke::new(1.0_f32, blue),
            bg_widget_stroke_disabled: Stroke::new(1.0_f32, line),
            fg_widget_stroke_interactive: Stroke::new(1.0_f32, fg),
            fg_widget_stroke_hovered: Stroke::new(1.0_f32, rgb(240, 243, 248)),
            fg_widget_stroke_active: Stroke::new(1.0_f32, Color32::WHITE),
            fg_widget_stroke_disabled: Stroke::new(1.0_f32, fg_faint),

            selection_stroke: Stroke::new(1.0_f32, rgb(140, 170, 255)),

            error_fg_color: red,
            error_bg_color: rgb(58, 29, 31),
            warn_fg_color: amber,
            warn_bg_color: rgb(59, 46, 23),
            success_fg_color: green,
            success_bg_color: rgb(23, 48, 31),
            info_fg_color: syn_blue,
            info_bg_color: rgb(30, 42, 72),

            pc_line_bg: rgb(46, 37, 22),
            breakpoint_bg: rgb(58, 28, 28),
            highlight_bg: rgb(42, 47, 85),

            memory_address_color: fg_dim,
            memory_label_color: yellow,
            memory_value_color: fg,
            memory_ascii_color: syn_str,
            memory_instruction_color: rgb(185, 195, 214),
            memory_os_code_bg: rgb(107, 79, 160),
            memory_os_data_bg: rgb(80, 62, 120),
            memory_user_code_bg: blue,
            memory_user_data_bg: rgb(55, 92, 180),
            memory_stack_bg: amber,
            memory_heap_bg: teal,
            memory_device_registers_bg: red,
            memory_unused_bg: bg2,
            memory_zone_label_color: fg_dim,

            register_name_color: fg_dim,
            register_value_color: fg,
            register_decoded_name_color: fg_dim,
            register_decoded_value_color: fg,
            register_special_purpose_name_color: syn_blue,
            register_special_purpose_value_color: fg,
            register_device_name_color: violet,
            register_device_value_color: fg,

            cpu_state_active_color: amber,
            cpu_state_inactive_color: fg_faint,
            cpu_state_description_color: fg_dim,
            cpu_state_data_flow_color: teal,
            cpu_state_active_register_highlight: rgb(30, 42, 72),
            cpu_state_active_memory_highlight: rgb(46, 37, 22),

            terminal_text_color: fg,
            terminal_bg_color: rgb(11, 13, 16),
            terminal_cursor_color: amber,
            terminal_selection_bg_color: blue.gamma_multiply(0.4),
            terminal_link_color: syn_blue,

            editor_label_color: yellow,
            editor_register_color: syn_reg,
            editor_directive_color: teal,
            editor_opcode_color: syn_blue,
            editor_literal_color: violet,
            editor_string_color: syn_str,
            editor_char_color: syn_str,
            editor_comment_color: rgb(98, 107, 124),
            editor_error_underline_color: red,
            editor_warning_underline_color: amber,
            editor_matching_bracket_bg_color: bg3,
            editor_current_line_number_color: rgb(194, 200, 211),
            editor_line_number_color: rgb(77, 85, 102),

            opcode_color: syn_blue,
            help_operand_color: syn_reg,
            help_immediate_color: violet,
            help_offset_color: teal,
            help_binary_layout_fixed_bits_color: fg_faint,
            help_pseudo_code_color: fg_dim,
            help_title_color: rgb(240, 243, 248),
            help_heading_color: fg,
            help_sub_heading_color: rgb(194, 200, 211),
            help_strong_label_color: fg,
            help_italic_label_color: fg_dim,
            help_monospace_color: rgb(194, 200, 211),
            help_code_block_text_color: fg,
            help_collapsible_header_bg_color: bg2,
            help_collapsible_header_text_color: fg,
            help_info_list_icon_color: amber,
            help_link_color: syn_blue,

            item_spacing: Vec2::new(8.0, 6.0),
            widget_rounding: CornerRadius::same(4),
            window_rounding: CornerRadius::same(8),
            scroll_bar_width: 8.0,
            window_stroke: Stroke::new(1.0_f32, line),
            panel_stroke: Stroke::new(1.0_f32, line),
            separator_color: line,
            tooltip_bg_color: bg2,
            tooltip_text_color: fg,
            scrollbar_bg_color: bg1,
            scrollbar_handle_color: rgb(52, 60, 76),
            scrollbar_handle_hovered_color: rgb(70, 80, 100),
            scrollbar_handle_active_color: rgb(90, 102, 126),
            scrollbar_rounding: CornerRadius::same(4),
            button_padding: Vec2::new(10.0, 4.0),
            indent_width: 16.0,
        }
    }

    /// The light theme: the dark theme's roles on a paper background, with each accent darkened
    /// until text in it reads on white.
    pub fn light_default() -> Self {
        let rgb = Color32::from_rgb;
        let bg0 = rgb(255, 255, 255); // window, text fields
        let bg1 = rgb(246, 247, 249); // panels
        let bg2 = rgb(237, 240, 244); // raised
        let bg3 = rgb(226, 230, 237); // hover
        let line = rgb(218, 222, 229);
        let fg = rgb(30, 34, 42);
        let fg_dim = rgb(84, 92, 108);
        let fg_faint = rgb(138, 146, 160);
        let amber = rgb(178, 96, 0);
        let blue = rgb(37, 99, 235);
        let green = rgb(21, 128, 61);
        let red = rgb(196, 38, 38);
        let violet = rgb(126, 58, 190);
        let teal = rgb(0, 118, 128);
        let label = rgb(140, 92, 0);
        let syn_blue = rgb(30, 82, 204);
        let syn_reg = rgb(176, 72, 18);
        let syn_str = rgb(36, 120, 44);

        Self {
            base_theme: BaseThemeChoice::Light,

            primary_text_color: fg,
            secondary_text_color: fg_dim,
            strong_text_color: rgb(10, 12, 16),
            hyperlink_color: syn_blue,
            window_background: bg0,
            panel_background: bg1,
            code_bg_color: bg2,

            accent_color_primary: amber,
            accent_color_secondary: blue,
            accent_color_tertiary: bg3,
            accent_color_positive: green,
            accent_color_negative: red,

            widget_text_color: fg,
            widget_fill_interactive: rgb(233, 236, 241),
            widget_fill_hovered: rgb(222, 226, 234),
            widget_fill_active: rgb(208, 215, 228),
            widget_fill_disabled: bg2,

            bg_widget_stroke_interactive: Stroke::new(1.0_f32, rgb(208, 213, 222)),
            bg_widget_stroke_hovered: Stroke::new(1.0_f32, rgb(178, 185, 198)),
            bg_widget_stroke_active: Stroke::new(1.0_f32, blue),
            bg_widget_stroke_disabled: Stroke::new(1.0_f32, line),
            fg_widget_stroke_interactive: Stroke::new(1.0_f32, fg),
            fg_widget_stroke_hovered: Stroke::new(1.0_f32, rgb(10, 12, 16)),
            fg_widget_stroke_active: Stroke::new(1.0_f32, Color32::BLACK),
            fg_widget_stroke_disabled: Stroke::new(1.0_f32, fg_faint),

            selection_stroke: Stroke::new(1.0_f32, blue),

            error_fg_color: red,
            error_bg_color: rgb(253, 231, 231),
            warn_fg_color: amber,
            warn_bg_color: rgb(253, 242, 219),
            success_fg_color: green,
            success_bg_color: rgb(224, 244, 231),
            info_fg_color: syn_blue,
            info_bg_color: rgb(226, 235, 253),

            pc_line_bg: rgb(253, 239, 211),
            breakpoint_bg: rgb(252, 226, 226),
            highlight_bg: rgb(221, 231, 253),

            memory_address_color: fg_dim,
            memory_label_color: label,
            memory_value_color: fg,
            memory_ascii_color: syn_str,
            memory_instruction_color: rgb(52, 60, 78),
            memory_os_code_bg: rgb(126, 92, 196),
            memory_os_data_bg: rgb(160, 136, 214),
            memory_user_code_bg: blue,
            memory_user_data_bg: rgb(110, 150, 240),
            memory_stack_bg: rgb(214, 140, 40),
            memory_heap_bg: teal,
            memory_device_registers_bg: red,
            memory_unused_bg: bg2,
            memory_zone_label_color: fg_dim,

            register_name_color: fg_dim,
            register_value_color: fg,
            register_decoded_name_color: fg_dim,
            register_decoded_value_color: fg,
            register_special_purpose_name_color: syn_blue,
            register_special_purpose_value_color: fg,
            register_device_name_color: violet,
            register_device_value_color: fg,

            cpu_state_active_color: amber,
            cpu_state_inactive_color: fg_faint,
            cpu_state_description_color: fg_dim,
            cpu_state_data_flow_color: teal,
            cpu_state_active_register_highlight: rgb(226, 235, 253),
            cpu_state_active_memory_highlight: rgb(253, 239, 211),

            terminal_text_color: fg,
            terminal_bg_color: rgb(251, 251, 252),
            terminal_cursor_color: amber,
            terminal_selection_bg_color: blue.gamma_multiply(0.25),
            terminal_link_color: syn_blue,

            editor_label_color: label,
            editor_register_color: syn_reg,
            editor_directive_color: teal,
            editor_opcode_color: syn_blue,
            editor_literal_color: violet,
            editor_string_color: syn_str,
            editor_char_color: syn_str,
            editor_comment_color: rgb(118, 126, 140),
            editor_error_underline_color: red,
            editor_warning_underline_color: amber,
            editor_matching_bracket_bg_color: bg3,
            editor_current_line_number_color: fg,
            editor_line_number_color: rgb(160, 167, 180),

            opcode_color: syn_blue,
            help_operand_color: syn_reg,
            help_immediate_color: violet,
            help_offset_color: teal,
            help_binary_layout_fixed_bits_color: fg_faint,
            help_pseudo_code_color: fg_dim,
            help_title_color: rgb(10, 12, 16),
            help_heading_color: fg,
            help_sub_heading_color: rgb(52, 60, 78),
            help_strong_label_color: fg,
            help_italic_label_color: fg_dim,
            help_monospace_color: rgb(52, 60, 78),
            help_code_block_text_color: fg,
            help_collapsible_header_bg_color: bg2,
            help_collapsible_header_text_color: fg,
            help_info_list_icon_color: amber,
            help_link_color: syn_blue,

            item_spacing: Vec2::new(8.0, 6.0),
            widget_rounding: CornerRadius::same(4),
            window_rounding: CornerRadius::same(8),
            scroll_bar_width: 8.0,
            window_stroke: Stroke::new(1.0_f32, line),
            panel_stroke: Stroke::new(1.0_f32, line),
            separator_color: line,
            tooltip_bg_color: bg0,
            tooltip_text_color: fg,
            scrollbar_bg_color: bg1,
            scrollbar_handle_color: rgb(200, 205, 214),
            scrollbar_handle_hovered_color: rgb(176, 183, 195),
            scrollbar_handle_active_color: rgb(150, 158, 172),
            scrollbar_rounding: CornerRadius::same(4),
            button_padding: Vec2::new(10.0, 4.0),
            indent_width: 16.0,
        }
    }

    /// Text drawn on an accent fill (the Run button, a highlighted chip): near black on the dark
    /// theme's bright amber, white on the light theme's deep one.
    pub fn on_accent(&self) -> Color32 {
        match self.base_theme {
            BaseThemeChoice::Dark => Color32::from_rgb(26, 18, 6),
            BaseThemeChoice::Light => Color32::WHITE,
        }
    }

    pub fn apply_to_style(&self, style: &mut Style) {
        use egui::{FontFamily, FontId, TextStyle};

        let mut v = match self.base_theme {
            BaseThemeChoice::Light => Visuals::light(),
            BaseThemeChoice::Dark => Visuals::dark(),
        };

        // --- Text and surfaces ---
        v.override_text_color = Some(self.primary_text_color);
        v.hyperlink_color = self.hyperlink_color;
        v.window_fill = self.panel_background;
        v.panel_fill = self.panel_background;
        v.extreme_bg_color = self.window_background; // text fields, scroll gutters
        v.faint_bg_color = self.code_bg_color; // striped rows
        v.code_bg_color = self.code_bg_color;
        v.text_edit_bg_color = Some(self.window_background);
        v.warn_fg_color = self.warn_fg_color;
        v.error_fg_color = self.error_fg_color;
        v.window_stroke = self.window_stroke;
        v.window_corner_radius = self.window_rounding;
        v.menu_corner_radius = self.window_rounding;
        // shadows: deep on dark, soft on light
        let shadow = match self.base_theme {
            BaseThemeChoice::Dark => 110,
            BaseThemeChoice::Light => 40,
        };
        v.window_shadow = egui::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: Color32::from_black_alpha(shadow),
        };
        v.popup_shadow = egui::Shadow {
            offset: [0, 6],
            blur: 18,
            spread: 0,
            color: Color32::from_black_alpha(shadow - 10),
        };
        v.striped = true;
        v.slider_trailing_fill = true;
        v.indent_has_left_vline = false;
        v.collapsing_header_frame = false;
        v.clip_rect_margin = 3.0;
        v.button_frame = true;

        // --- Widgets ---
        let r = self.widget_rounding;
        v.widgets.noninteractive.bg_fill = self.panel_background;
        v.widgets.noninteractive.weak_bg_fill = self.panel_background;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, self.separator_color);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, self.primary_text_color);
        v.widgets.noninteractive.corner_radius = r;

        v.widgets.inactive.bg_fill = self.widget_fill_interactive;
        v.widgets.inactive.weak_bg_fill = self.widget_fill_interactive;
        v.widgets.inactive.bg_stroke = self.bg_widget_stroke_interactive;
        v.widgets.inactive.fg_stroke = self.fg_widget_stroke_interactive;
        v.widgets.inactive.corner_radius = r;
        v.widgets.inactive.expansion = 0.0;

        v.widgets.hovered.bg_fill = self.widget_fill_hovered;
        v.widgets.hovered.weak_bg_fill = self.widget_fill_hovered;
        v.widgets.hovered.bg_stroke = self.bg_widget_stroke_hovered;
        v.widgets.hovered.fg_stroke = self.fg_widget_stroke_hovered;
        v.widgets.hovered.corner_radius = r;
        v.widgets.hovered.expansion = 0.0;

        v.widgets.active.bg_fill = self.widget_fill_active;
        v.widgets.active.weak_bg_fill = self.widget_fill_active;
        v.widgets.active.bg_stroke = self.bg_widget_stroke_active;
        v.widgets.active.fg_stroke = self.fg_widget_stroke_active;
        v.widgets.active.corner_radius = r;
        v.widgets.active.expansion = 0.0;

        v.widgets.open = v.widgets.active;
        v.widgets.open.bg_stroke = self.bg_widget_stroke_hovered;

        v.selection.bg_fill = self
            .accent_color_secondary
            .gamma_multiply(match self.base_theme {
                BaseThemeChoice::Dark => 0.45,
                BaseThemeChoice::Light => 0.25,
            });
        v.selection.stroke = self.selection_stroke;

        style.visuals = v;

        // --- Spacing ---
        style.spacing.item_spacing = self.item_spacing;
        style.spacing.button_padding = self.button_padding;
        style.spacing.indent = self.indent_width;
        style.spacing.interact_size.y = 24.0;
        style.spacing.slider_width = 120.0;
        style.spacing.text_edit_width = 200.0;
        style.spacing.combo_width = 150.0;
        style.spacing.menu_width = 200.0;
        style.spacing.menu_margin = egui::Margin::same(6);
        style.spacing.window_margin = egui::Margin::same(12);
        style.spacing.scroll = egui::style::ScrollStyle::thin();
        style.spacing.scroll.bar_width = self.scroll_bar_width;

        // --- Type ---
        let ui_font = FontFamily::Proportional;
        style.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(18.0, FontFamily::Name("ui-semibold".into())),
            ),
            (TextStyle::Body, FontId::new(13.5, ui_font.clone())),
            (TextStyle::Button, FontId::new(13.0, ui_font.clone())),
            (TextStyle::Small, FontId::new(11.5, ui_font)),
            (
                TextStyle::Monospace,
                FontId::new(13.5, FontFamily::Monospace),
            ),
        ]
        .into();

        // --- Interaction ---
        style.interaction.tooltip_delay = 0.35;
        style.interaction.show_tooltips_only_when_still = true;
        style.interaction.selectable_labels = false;
        style.interaction.multi_widget_text_select = false;
        style.animation_time = 0.08;
    }

    /// Register the app fonts: Inter for UI text, JetBrains Mono for code, and the Phosphor icon
    /// font as a fallback for both so icons can be mixed into labels.
    pub fn install_fonts(ctx: &egui::Context) {
        use egui::{FontData, FontDefinitions, FontFamily};
        use std::sync::Arc;

        let mut fonts = FontDefinitions::default();
        egui_phosphor_icons::add_fonts(&mut fonts);
        fonts.font_data.insert(
            "inter".into(),
            Arc::new(FontData::from_static(include_bytes!(
                "../assets/fonts/Inter-Regular.ttf"
            ))),
        );
        fonts.font_data.insert(
            "inter-semibold".into(),
            Arc::new(FontData::from_static(include_bytes!(
                "../assets/fonts/Inter-SemiBold.ttf"
            ))),
        );
        fonts.font_data.insert(
            "jetbrains-mono".into(),
            Arc::new(FontData::from_static(include_bytes!(
                "../assets/fonts/JetBrainsMono-Regular.ttf"
            ))),
        );
        fonts.font_data.insert(
            "jetbrains-mono-bold".into(),
            Arc::new(FontData::from_static(include_bytes!(
                "../assets/fonts/JetBrainsMono-Bold.ttf"
            ))),
        );

        // The bundled text fonts are subset without private use area glyphs, so icon code points
        // fall through to Phosphor.
        let mut proportional = vec!["inter".to_owned(), "phosphor-icons".to_owned()];
        proportional.extend(fonts.families[&FontFamily::Proportional].iter().cloned());
        let mut mono = vec!["jetbrains-mono".to_owned(), "phosphor-icons".to_owned()];
        mono.extend(fonts.families[&FontFamily::Monospace].iter().cloned());
        let mut semibold = vec!["inter-semibold".to_owned()];
        semibold.extend(proportional.iter().skip(1).cloned());
        let mut mono_bold = vec!["jetbrains-mono-bold".to_owned()];
        mono_bold.extend(mono.iter().skip(1).cloned());
        fonts
            .families
            .insert(FontFamily::Proportional, proportional);
        fonts.families.insert(FontFamily::Monospace, mono);
        fonts
            .families
            .insert(FontFamily::Name("ui-semibold".into()), semibold);
        fonts
            .families
            .insert(FontFamily::Name("mono-bold".into()), mono_bold);
        ctx.set_fonts(fonts);
    }

    /// Saves the current theme settings to a .ron file in the "assets" directory.
    /// This function only operates in debug builds and not on wasm targets.
    pub fn save_to_ron_file(&self) {
        #[cfg(all(not(target_arch = "wasm32"), debug_assertions))]
        {
            let filename = match self.base_theme {
                BaseThemeChoice::Light => "assets/themes/light.ron",
                BaseThemeChoice::Dark => "assets/themes/dark.ron",
            };

            // Ensure assets directory exists
            if let Err(e) = fs::create_dir_all("assets") {
                eprintln!("[Theme] Failed to create assets directory: {e}");
                return;
            }
            if let Err(e) = fs::create_dir_all("assets/themes") {
                eprintln!("[Theme] Failed to create themes directory: {e}");
                return;
            }

            match ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) {
                Ok(ron_string) => match fs::write(filename, ron_string) {
                    Ok(_) => println!("[Theme] Successfully saved theme to {filename}"),
                    Err(e) => eprintln!("[Theme] Failed to write theme to {filename}: {e}"),
                },
                Err(e) => {
                    eprintln!("[Theme] Failed to serialize theme to RON: {e}");
                }
            }
        }
        // Implicitly does nothing if not (non-wasm and debug build)
    }

    pub fn set_global_theme(&mut self, choice: BaseThemeChoice, ctx: Option<&egui::Context>) {
        *self = match choice {
            BaseThemeChoice::Light => ThemeSettings::light_default(),
            BaseThemeChoice::Dark => ThemeSettings::dark_default(),
        };

        if let Some(ctx) = ctx {
            let mut style = (*ctx.style()).clone();
            self.apply_to_style(&mut style);
            ctx.set_style(style);
            ctx.request_repaint(); // Ensure UI updates immediately
        }
    }
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self::dark_default() // Default to dark theme
    }
}

// lazy_static! {
//     pub static ref CURRENT_THEME_SETTINGS: Mutex<ThemeSettings> =
//         Mutex::new(ThemeSettings::default());
// }
