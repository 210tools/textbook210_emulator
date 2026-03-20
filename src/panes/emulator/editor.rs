use crate::emulator::parse::{ParseError, ParseOutput};
use crate::emulator::Emulator;
use crate::panes::{Pane, PaneDisplay, PaneTree, RealPane};
use crate::theme::ThemeSettings;
use egui_code_editor::Completer;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use super::EmulatorPane;

// A helper struct to pass async loaded file data back to the synchronous egui loop.
// We implement standard traits manually so it doesn't break the EditorPane derives.
#[derive(Clone)]
pub struct PendingUpload {
    pub data: Arc<Mutex<Option<(String, String)>>>, // (name, content)
}

impl Default for PendingUpload {
    fn default() -> Self {
        Self {
            data: Arc::new(Mutex::new(None)),
        }
    }
}

impl PartialEq for PendingUpload {
    fn eq(&self, _other: &Self) -> bool {
        true // Always equal so it doesn't break UI diffing
    }
}

impl std::fmt::Debug for PendingUpload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PendingUpload")
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct EditorPane {
    program: String,
    fade: f32,
    last_compilation_was_successful: bool,
    file_name: Option<String>,
    #[serde(skip)]
    completer: Completer,
    #[serde(skip)]
    pending_upload: PendingUpload,
}

impl Default for EditorPane {
    fn default() -> Self {
        let syntax = egui_code_editor::Syntax::new("lc3_assembly")
            .with_comment(";")
            .with_keywords(BTreeSet::from([
                "ADD", "AND", "BR", "BRN", "BRZ", "BRP", "BRNZ", "BRNP", "BRZP", "BRNZP", "JMP",
                "JSR", "JSRR", "LD", "LDI", "LDR", "LEA", "NOT", "RET", "RTI", "ST", "STI", "STR",
                "TRAP", "GETC", "OUT", "PUTS", "IN", "HALT",
            ]))
            .with_special(BTreeSet::from([
                ":", ".ORIG", ".FILL", ".BLKW", ".STRINGZ", ".END",
            ]))
            .with_types(BTreeSet::new())
            .with_case_sensitive(false);
        Self {
            program: r#".ORIG x3000
; Simple Hello World program
LEA R0, MESSAGE    ; Load the address of the message
PUTS               ; Output the string
HALT               ; Halt the program

MESSAGE: .STRINGZ "Hello, World!"
.END"#
                .to_string(),
            fade: 0.0,
            file_name: None,
            last_compilation_was_successful: false,
            completer: Completer::new_with_syntax(&syntax).with_user_words(),
            pending_upload: PendingUpload::default(),
        }
    }
}

impl PaneDisplay for EditorPane {
    fn render(&mut self, ui: &mut egui::Ui, emulator: &mut Emulator, theme: &mut ThemeSettings) {
        if emulator.metadata.last_compiled_source.is_empty() {
            self.last_compilation_was_successful = false;
        }

        // Check if an async file upload finished
        if let Ok(mut lock) = self.pending_upload.data.try_lock() {
            if let Some(content) = lock.take() {
                self.file_name = Some(content.0);
                self.program = content.1;
            }
        }

        egui::panel::TopBottomPanel::top("lc3 editor -- buttons").show_inside(ui, |ui| {
            // Show error or success feedback
            {
                let artifacts = &mut emulator.metadata;
                if let Some(error) = &artifacts.error {
                    match error {
                        ParseError::TokenizeError(s, l) => {
                            ui.colored_label(
                                ui.visuals().error_fg_color,
                                format!("Syntax error on line {l}: {s}"),
                            );
                        }
                        ParseError::GenerationError(s, token_span) => {
                            ui.colored_label(
                                ui.visuals().error_fg_color,
                                format!("Code generation error at {token_span:?}: {s}"),
                            );
                        }
                    }
                } else if !artifacts.last_compiled_source.is_empty()
                    && self.last_compilation_was_successful
                {
                    ui.colored_label(
                        theme.success_fg_color,
                        egui::RichText::new("Compiled successfully!").strong(),
                    );
                }
            }
            ui.add_space(8.0);

            let just_compiled = match self.last_compilation_was_successful {
                true => theme.accent_color_positive,
                false => theme.accent_color_negative,
            };
            let base = theme.accent_color_primary;
            let fade = self.fade.clamp(0.0, 1.0);

            let blend = |a: egui::Color32, b: egui::Color32, t: f32| -> egui::Color32 {
                let t = t.clamp(0.0, 1.0);
                let r = (a.r() as f32 * t + b.r() as f32 * (1.0 - t)) as u8;
                let g = (a.g() as f32 * t + b.g() as f32 * (1.0 - t)) as u8;
                let b_ = (a.b() as f32 * t + b.b() as f32 * (1.0 - t)) as u8;
                let a_ = (a.a() as f32 * t + b.a() as f32 * (1.0 - t)) as u8;
                egui::Color32::from_rgba_premultiplied(r, g, b_, a_)
            };

            let button_color = blend(just_compiled, base, fade);

            ui.horizontal(|ui| {
                let button = egui::Button::new("Compile").fill(button_color);
                if ui.add(button).clicked() {
                    let data_to_load =
                        Emulator::parse_program(&self.program, Some(&mut emulator.metadata));
                    if let Ok(ParseOutput {
                        machine_code,
                        orig_address,
                        ..
                    }) = data_to_load
                    {
                        emulator.flash_memory(machine_code, orig_address);
                        self.fade = 1.0;
                        self.last_compilation_was_successful = true;
                    } else {
                        self.fade = 1.0;
                        self.last_compilation_was_successful = false;
                    }
                }

                ui.separator();

                if ui.button("📂 Load").clicked() {
                    let pending = self.pending_upload.data.clone();

                    #[cfg(target_arch = "wasm32")]
                    wasm_bindgen_futures::spawn_local(async move {
                        if let Some(file) = rfd::AsyncFileDialog::new()
                            .add_filter("Assembly", &["asm", "txt", "lc3"])
                            .pick_file()
                            .await
                        {
                            let bytes = file.read().await;
                            if let Ok(content) = String::from_utf8(bytes) {
                                if let Ok(mut lock) = pending.lock() {
                                    *lock = Some((file.file_name(), content));
                                }
                            }
                        }
                    });

                    #[cfg(not(target_arch = "wasm32"))]
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Assembly", &["asm", "txt", "lc3"])
                        .pick_file()
                    {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(mut lock) = pending.lock() {
                                *lock = Some((
                                    path.file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .into_owned(),
                                    content,
                                ));
                            }
                        }
                    }
                }

                if ui.button("💾 Save").clicked() {
                    let content = self.program.clone();

                    #[cfg(target_arch = "wasm32")]
                    {
                        use wasm_bindgen::JsCast;
                        if let Some(window) = web_sys::window() {
                            if let Some(document) = window.document() {
                                if let Ok(element) = document.create_element("a") {
                                    if let Ok(a) = element.dyn_into::<web_sys::HtmlAnchorElement>()
                                    {
                                        let array = js_sys::Array::new();
                                        array.push(&wasm_bindgen::JsValue::from_str(&content));
                                        let mut options = web_sys::BlobPropertyBag::new();
                                        options.type_("text/plain");
                                        if let Ok(blob) =
                                            web_sys::Blob::new_with_str_sequence_and_options(
                                                &array, &options,
                                            )
                                        {
                                            if let Ok(url) =
                                                web_sys::Url::create_object_url_with_blob(&blob)
                                            {
                                                a.set_href(&url);
                                                a.set_download(
                                                    self.file_name
                                                        .as_ref()
                                                        .map_or("program.asm", |v| &*v),
                                                );
                                                let _ = a.click();
                                                let _ = web_sys::Url::revoke_object_url(&url);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    #[cfg(not(target_arch = "wasm32"))]
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Assembly", &["asm", "txt", "lc3"])
                        .set_file_name(self.file_name.as_ref().map_or("program.asm", |v| v))
                        .save_file()
                    {
                        let _ = std::fs::write(&path, content);
                    }
                }
            });

            if self.fade > 0.0 {
                self.fade = (self.fade - 0.04).max(0.0);
            }
        });

        egui::panel::TopBottomPanel::top("lc3 editor")
            .min_height(ui.available_height())
            .show_inside(ui, |ui| {
                let syntax = egui_code_editor::Syntax::new("lc3_assembly")
                    .with_comment(";")
                    .with_keywords(BTreeSet::from([
                        "ADD", "AND", "BR", "BRN", "BRZ", "BRP", "BRNZ", "BRNP", "BRZP", "BRNZP",
                        "JMP", "JSR", "JSRR", "LD", "LDI", "LDR", "LEA", "NOT", "RET", "RTI", "ST",
                        "STI", "STR", "TRAP", "GETC", "OUT", "PUTS", "IN", "HALT",
                    ]))
                    .with_special(BTreeSet::from([
                        ":", ".ORIG", ".FILL", ".BLKW", ".STRINGZ", ".END",
                    ]))
                    .with_case_sensitive(false);

                if egui_code_editor::CodeEditor::default()
                    .with_ui_fontsize(ui)
                    .with_syntax(syntax)
                    .with_rows(100)
                    .vscroll(true)
                    .with_theme(egui_code_editor::ColorTheme::SONOKAI)
                    .show_with_completer(ui, &mut self.program, &mut self.completer)
                    .response
                    .changed()
                {
                    self.last_compilation_was_successful = false;
                }
            });
    }

    fn title(&self) -> String {
        format!(
            "Editor{}",
            self.file_name
                .as_ref()
                .map(|s| format!(" -- `{}`", s))
                .unwrap_or_default()
        )
    }

    fn children() -> PaneTree {
        PaneTree::Pane(
            "Editor".to_string(),
            Pane::new(RealPane::EmulatorPanes(Box::new(EmulatorPane::Editor(
                EditorPane::default(),
            )))),
        )
    }
}
