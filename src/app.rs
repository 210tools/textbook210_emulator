use crate::panes::glossary::Explain;
use std::collections::VecDeque;
use std::{ops::DerefMut, sync::Mutex};

use crate::{
    emulator::{Emulator, StopReason},
    panes::{
        emulator::editor::{
            self, widget::TextCommand, AssembleStatus, Document, EditorPane, OPENED_FILES,
        },
        EmulatorPane, Pane, PaneDisplay, PaneTree, RealPane,
    },
    theme::{BaseThemeChoice, ThemeSettings},
};
use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Hyperlink, Key, KeyboardShortcut, Layout,
    Modifiers, RichText, Sense, Stroke, Vec2,
};
use egui_dock::{
    AllowedSplits, DockArea, DockState, NodeIndex, Style, SurfaceIndex, TabIndex, TabViewer,
};
use egui_phosphor_icons::icons;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref EMULATOR: Mutex<Emulator> = Mutex::new(Emulator::new());
}
#[cfg(not(target_arch = "wasm32"))]
lazy_static! {
    pub static ref LAST_PAINT_ID: Mutex<u64> = Mutex::new(0); // this is pretty botch, more info later
}
lazy_static! {
    /// The egui context, so the emulator loop can ask for a repaint when the machine changes.
    pub static ref EGUI_CTX: Mutex<Option<egui::Context>> = Mutex::new(None);
}

const FEEDBACK_URL: &str = "https://forms.gle/78o4qEPmsBipbsFx7";
/// Where the desktop builds are published
const DOWNLOAD_URL: &str = "https://github.com/210tools/textbook210_emulator/releases/tag/main";
const DOWNLOAD_PITCH: &str = "The desktop app is the main version of 210 tools: it runs faster, works offline, keeps your files on your computer and has every keyboard shortcut (F5, F10, F11…).";

/// Converts a number string in `base_in` to `base_out`.
/// Uses the given alphabet (E.g. dec 10 in base 2 with the
/// alphabet "01" looks like 1010 but with the alphabet "ab" it looks like baba)
pub fn base_to_base(
    base_in: u32,
    base_out: u32,
    input: &str,
    alphabet: impl Into<String>,
) -> String {
    let alphabet: String = alphabet.into();
    let mut output = String::new();
    let mut num = 0;
    let mut place = 1;
    for c in input.chars().rev() {
        let digit = match alphabet.find(c) {
            Some(d) => d as u32,
            None => {
                return "Invalid input".to_owned();
            }
        };
        num += digit * place;
        place *= base_in;
    }
    while num > 0 {
        let digit = num % base_out;
        num /= base_out;
        let c = match alphabet.chars().nth(digit as usize) {
            Some(c) => c,
            None => {
                return "Invalid input".to_owned();
            }
        };
        output.push(c);
    }
    if output == String::new() {
        output = alphabet.chars().next().unwrap().to_string();
    }
    output.chars().rev().collect()
}

/// The speed slider's last position, Max
const SPEED_MAX: u32 = 100;
/// How often the emulator is updated (about once a frame)
const UPDATES_PER_SECOND: f32 = 60.0;

#[derive(Default)]
/// A simple pane manager that handles adding panes. panes can be closed when they are not "alone"
struct PaneManager {
    added_nodes: Vec<Pane>,
    last_added: Option<(NodeIndex, SurfaceIndex)>,
    theme: ThemeSettings,
    /// An editor tab with unsaved changes the user tried to close
    confirm_close: Option<u64>,
}

impl TabViewer for PaneManager {
    type Tab = Pane;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title().into()
    }

    /// This is important because we can span more than one pane with the same name (eg 2 editors).
    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(tab.id)
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let mut emulator = EMULATOR.lock().unwrap();
        tab.render(ui, emulator.deref_mut(), &mut self.theme);
    }

    fn is_closeable(&self, tab: &Self::Tab) -> bool {
        !tab.alone
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> egui_dock::widgets::tab_viewer::OnCloseResponse {
        if let RealPane::EmulatorPanes(p) = &tab.inner {
            if let EmulatorPane::Editor(e) = p.as_ref() {
                if e.doc.is_modified() {
                    self.confirm_close = Some(tab.id);
                    return egui_dock::widgets::tab_viewer::OnCloseResponse::Ignore;
                }
            }
        }
        egui_dock::widgets::tab_viewer::OnCloseResponse::Close
    }

    fn allowed_in_windows(&self, tab: &mut Self::Tab) -> bool {
        !tab.alone
    }

    fn scroll_bars(&self, _tab: &Self::Tab) -> [bool; 2] {
        [false, false]
    }

    /// Menu for the + button: a tree of panes that can be added. See [PaneTree].
    fn add_popup(&mut self, ui: &mut egui::Ui, surface: egui_dock::SurfaceIndex, node: NodeIndex) {
        ui.set_min_width(140.0);
        ui.style_mut().visuals.button_frame = false;

        for pane in match Pane::children() {
            PaneTree::Pane(s, pane) => vec![PaneTree::Pane(s, pane)],
            PaneTree::Children(_, pane_trees) => pane_trees,
        } {
            self.add_pane_menu_items(ui, pane);
        }
        self.last_added = Some((node, surface));
    }
}

impl PaneManager {
    /// This will recursavly iterate the [PaneTree] structure and use it to form a menu.
    fn add_pane_menu_items(&mut self, ui: &mut egui::Ui, pane_tree: PaneTree) {
        match pane_tree {
            crate::panes::PaneTree::Pane(name, pane_variant) => {
                ui.style_mut().visuals.button_frame = false;
                if ui.button(name).clicked() {
                    self.added_nodes.push(pane_variant);
                    ui.close();
                }
            }
            crate::panes::PaneTree::Children(name, children) => {
                ui.style_mut().visuals.button_frame = false;
                ui.menu_button(name, |ui| {
                    for child in children {
                        self.add_pane_menu_items(ui, child);
                    }
                });
            }
        }
    }
}

/// Every action that can be triggered from the toolbar, menus or keyboard.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Action {
    Assemble,
    RunPause,
    StepOver,
    StepInto,
    StepOut,
    MicroStep,
    StepBack,
    Reset,
    NewFile,
    OpenFile,
    Save,
    SaveAs,
    Find,
    Replace,
    GoToLine,
    Edit(TextCommand),
    ShowShortcuts,
}

struct Shortcut {
    action: Action,
    shortcut: KeyboardShortcut,
    label: &'static str,
}

/// The global shortcuts for this platform. On the web, function keys and most Ctrl+letter
/// combinations belong to the browser (F5 reloads the page), so only a few safe ones are used
/// and the rest of the commands are on the toolbar.
fn shortcuts() -> Vec<Shortcut> {
    use Modifiers as M;
    let s = |action, m, k, label| Shortcut {
        action,
        shortcut: KeyboardShortcut::new(m, k),
        label,
    };
    #[cfg(target_arch = "wasm32")]
    {
        vec![
            s(Action::Assemble, M::COMMAND, Key::B, "Assemble and load"),
            s(Action::RunPause, M::COMMAND, Key::Enter, "Run / pause"),
            s(Action::OpenFile, M::COMMAND, Key::O, "Open file"),
            s(Action::SaveAs, M::COMMAND | M::SHIFT, Key::S, "Save as"),
            s(Action::Save, M::COMMAND, Key::S, "Save (download)"),
        ]
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        vec![
            s(Action::Assemble, M::COMMAND, Key::B, "Assemble and load"),
            s(Action::Assemble, M::NONE, Key::F7, "Assemble and load"),
            s(Action::RunPause, M::NONE, Key::F5, "Run / pause"),
            s(Action::RunPause, M::COMMAND, Key::Enter, "Run / pause"),
            s(Action::Reset, M::SHIFT, Key::F5, "Reset the machine"),
            s(Action::StepOver, M::NONE, Key::F10, "Step over"),
            s(Action::StepInto, M::NONE, Key::F11, "Step into"),
            s(Action::StepOut, M::SHIFT, Key::F11, "Step out"),
            s(Action::StepBack, M::SHIFT, Key::F12, "Back one state"),
            s(Action::MicroStep, M::NONE, Key::F12, "Micro step"),
            s(Action::NewFile, M::COMMAND, Key::N, "New file"),
            s(Action::OpenFile, M::COMMAND, Key::O, "Open file"),
            s(Action::SaveAs, M::COMMAND | M::SHIFT, Key::S, "Save as"),
            s(Action::Save, M::COMMAND, Key::S, "Save"),
            s(
                Action::ShowShortcuts,
                M::NONE,
                Key::F1,
                "Keyboard shortcuts",
            ),
        ]
    }
}

/// The shortcut for an action on this platform, e.g. "F5" or "Ctrl+Enter", or "" if none.
fn hint(ctx: &egui::Context, action: Action) -> String {
    shortcuts()
        .iter()
        .find(|s| s.action == action)
        .map(|s| ctx.format_shortcut(&s.shortcut))
        .unwrap_or_default()
}

/// "text (shortcut)", or just "text" when the action has no shortcut on this platform.
fn tip(ctx: &egui::Context, text: &str, action: Action) -> String {
    let h = hint(ctx, action);
    if h.is_empty() {
        text.to_string()
    } else {
        format!("{text} ({h})")
    }
}

/// Editor shortcuts, listed in the shortcuts window (handled inside the editor).
const EDITOR_SHORTCUTS: &[(&str, &str)] = &[
    ("Ctrl+Space", "Show completions"),
    ("Ctrl+F / Ctrl+H", "Find / replace"),
    ("Ctrl+G", "Go to line"),
    ("Ctrl+/", "Toggle comment"),
    ("Ctrl+D", "Duplicate line or selection"),
    ("Alt+↑ / Alt+↓", "Move lines up / down"),
    #[cfg(not(target_arch = "wasm32"))]
    ("Ctrl+Shift+K", "Delete line"),
    ("Tab / Shift+Tab", "Indent / outdent"),
    ("Shift+Alt+F", "Format document"),
    ("Ctrl+Z / Ctrl+Shift+Z", "Undo / redo"),
    (
        "Ctrl+click",
        "Go to a label, or open an instruction in Help",
    ),
    ("Click the gutter", "Toggle a breakpoint"),
];

/// This is the core app state and pretty much everything involved with the ui comes though
/// here as well as pane stuff like what is currently put in the editor.
pub struct EmulatorApp {
    /// This stores the tree of panes (pretty much the entire pane state)
    dock_state: DockState<Pane>,
    /// This struct provides interface between out pane tree and actual things like
    /// render and title. See [PaneManager]
    tree_behavior: PaneManager,
    /// Defaults to true and is set to false on the closing of the first run info panel
    first_open: bool,
    /// The ui scale
    scale: f32,
    /// Follow the operating system's light or dark setting
    theme_follows_system: bool,
    /// Frames per second, averaged over the last few frames (shown in the status bar)
    fps: f32,
    /// Speed slider position: 0..SPEED_MAX sets the clock from 1 cycle/s up a log scale;
    /// SPEED_MAX runs as fast as the computer can
    speed: u32,
    show_shortcuts: bool,
    show_about: bool,
    /// Was execution last stopped inside the OS (for switching to the OS source tab)
    was_in_os: bool,
    last_stop_pc: Option<usize>,
    /// The document holding the code execution last stopped in
    last_owner: Option<u64>,

    #[cfg(target_arch = "wasm32")]
    /// Have we clicked ok on the fps warning? This will mean it does not spawn for the rest of the session
    has_dismissed_fps: bool,
    #[cfg(target_arch = "wasm32")]
    /// Used as a meter for how bad the fps is at the moment. Higher is worse.
    bad_fps_score: u32,
    #[cfg(target_arch = "wasm32")]
    /// Is the bad fps prompt open?
    curr_bad_fps_prompt_open: bool,
    /// Rolling FPS samples used to smooth the displayed/read fps value.
    fps_samples: VecDeque<f32>,
    theme: ThemeSettings,
}

fn emu_pane(p: EmulatorPane) -> Pane {
    Pane::new(RealPane::EmulatorPanes(Box::new(p)))
}

fn default_layout(editors: Vec<EditorPane>) -> DockState<Pane> {
    let editors: Vec<Pane> = editors
        .into_iter()
        .map(|e| emu_pane(EmulatorPane::Editor(e)))
        .collect();
    let mut dock_state = DockState::new(editors);
    let surface = dock_state.main_surface_mut();
    let [left, right] = surface.split_right(
        NodeIndex::root(),
        0.68,
        vec![emu_pane(EmulatorPane::Cpu(
            crate::panes::emulator::cpu_state::CpuStatePane::default(),
        ))],
    );
    surface.split_below(
        right,
        0.46,
        vec![emu_pane(EmulatorPane::Memory(
            crate::panes::emulator::memory::MemoryPane::default(),
        ))],
    );
    surface.split_below(
        left,
        0.70,
        vec![
            emu_pane(EmulatorPane::Output(
                crate::panes::emulator::io::IoPane::default(),
            )),
            emu_pane(EmulatorPane::Help(
                crate::panes::emulator::help::HelpPane::default(),
            )),
        ],
    );
    dock_state
}

/// Layout with room for the datapath: editor, a full-height datapath (Figure C.3 is tall), then
/// CPU over memory, terminal and help.
/// The datapath across most of the window; the editor, CPU and terminal in a column beside it.
fn large_datapath_layout(editors: Vec<EditorPane>) -> DockState<Pane> {
    let editors: Vec<Pane> = editors
        .into_iter()
        .map(|e| emu_pane(EmulatorPane::Editor(e)))
        .collect();
    let mut dock_state = DockState::new(editors);
    let surface = dock_state.main_surface_mut();
    let [left, _] = surface.split_right(
        NodeIndex::root(),
        0.26,
        vec![emu_pane(EmulatorPane::Datapath(Default::default()))],
    );
    let [_, below] = surface.split_below(
        left,
        0.45,
        vec![emu_pane(EmulatorPane::Cpu(
            crate::panes::emulator::cpu_state::CpuStatePane::default(),
        ))],
    );
    surface.split_below(
        below,
        0.6,
        vec![emu_pane(EmulatorPane::Output(
            crate::panes::emulator::io::IoPane::default(),
        ))],
    );
    dock_state
}

fn datapath_layout(editors: Vec<EditorPane>) -> DockState<Pane> {
    let editors: Vec<Pane> = editors
        .into_iter()
        .map(|e| emu_pane(EmulatorPane::Editor(e)))
        .collect();
    let mut dock_state = DockState::new(editors);
    let surface = dock_state.main_surface_mut();
    let [_, rest] = surface.split_right(
        NodeIndex::root(),
        0.32,
        vec![emu_pane(EmulatorPane::Datapath(Default::default()))],
    );
    let [_, right] = surface.split_right(
        rest,
        0.56,
        vec![emu_pane(EmulatorPane::Cpu(
            crate::panes::emulator::cpu_state::CpuStatePane::default(),
        ))],
    );
    surface.split_below(
        right,
        0.48,
        vec![
            emu_pane(EmulatorPane::Memory(
                crate::panes::emulator::memory::MemoryPane::default(),
            )),
            emu_pane(EmulatorPane::Output(
                crate::panes::emulator::io::IoPane::default(),
            )),
            emu_pane(EmulatorPane::Help(
                crate::panes::emulator::help::HelpPane::default(),
            )),
        ],
    );
    dock_state
}

impl Default for EmulatorApp {
    /// New clean state.
    fn default() -> Self {
        let span = tracing::info_span!("EmulatorApp::default");
        let _guard = span.enter();

        let theme = ThemeSettings::dark_default();
        Self {
            dock_state: default_layout(vec![EditorPane::default()]),
            tree_behavior: PaneManager::default(),
            first_open: true,
            speed: 54,
            show_shortcuts: false,
            show_about: false,
            was_in_os: false,
            last_stop_pc: None,
            last_owner: None,
            #[cfg(target_arch = "wasm32")]
            has_dismissed_fps: false,
            #[cfg(target_arch = "wasm32")]
            bad_fps_score: 0,
            #[cfg(target_arch = "wasm32")]
            curr_bad_fps_prompt_open: false,
            fps_samples: VecDeque::with_capacity(10),
            theme,
            scale: 1.0,
            theme_follows_system: false,
            fps: 0.0,
        }
    }
}

impl EmulatorApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let span = tracing::info_span!("EmulatorApp::new");
        let _guard = span.enter();

        let mut app = Self::default();
        let mut theme_choice = BaseThemeChoice::Dark;

        if let Some(storage) = cc.storage {
            if let Some(first_open) = eframe::get_value(storage, "first_open") {
                app.first_open = first_open;
            }
            if let Some(scale) = eframe::get_value(storage, "scale") {
                app.scale = scale;
            }
            if let Some(speed) = eframe::get_value::<u32>(storage, "speed_pos") {
                app.speed = speed.min(SPEED_MAX);
            }
            if let Some(t) = eframe::get_value::<BaseThemeChoice>(storage, "theme") {
                theme_choice = t;
            }
            if let Some(f) = eframe::get_value::<bool>(storage, "theme_follows_system") {
                app.theme_follows_system = f;
            }
            if let Some(os) = eframe::get_value::<crate::emulator::OsKind>(storage, "os") {
                switch_os(os);
            }
            // Restore the files that were open last time
            if let Some(docs) = eframe::get_value::<Vec<Document>>(storage, "documents") {
                if !docs.is_empty() {
                    let editors = docs.into_iter().map(EditorPane::new).collect();
                    app.dock_state = default_layout(editors);
                }
            }
        }

        ThemeSettings::install_fonts(&cc.egui_ctx);
        cc.egui_ctx.set_zoom_factor(app.scale);
        cc.egui_ctx
            .options_mut(|o| o.theme_preference = egui::ThemePreference::Dark);

        app.theme.set_global_theme(theme_choice, Some(&cc.egui_ctx));
        app.tree_behavior.theme = app.theme.clone();
        *EGUI_CTX.lock().unwrap() = Some(cc.egui_ctx.clone());
        app
    }

    fn editors(&mut self) -> impl Iterator<Item = &mut EditorPane> {
        self.dock_state
            .iter_all_tabs_mut()
            .filter_map(|(_, tab)| match &mut tab.inner {
                RealPane::EmulatorPanes(p) => match p.as_mut() {
                    EmulatorPane::Editor(e) => Some(e),
                    _ => None,
                },
                _ => None,
            })
    }

    /// The editor the user worked in most recently.
    fn active_editor(&mut self) -> Option<&mut EditorPane> {
        // Prefer the editor in the focused leaf
        let focused_id = self.dock_state.find_active_focused().map(|(_, tab)| tab.id);
        let mut best: Option<&mut EditorPane> = None;
        let mut best_score = 0u64;
        for (_, tab) in self.dock_state.iter_all_tabs_mut() {
            let id = tab.id;
            if let RealPane::EmulatorPanes(p) = &mut tab.inner {
                if let EmulatorPane::Editor(e) = p.as_mut() {
                    let score = if Some(id) == focused_id {
                        u64::MAX
                    } else {
                        e.last_active_frame + 1
                    };
                    if best.is_none() || score > best_score {
                        best_score = score;
                        best = Some(e);
                    }
                }
            }
        }
        best
    }

    /// The user program editor most recently worked in (never the read-only OS tab).
    fn program_editor(&mut self) -> Option<&mut EditorPane> {
        let focused_id = self.dock_state.find_active_focused().map(|(_, tab)| tab.id);
        let mut best: Option<&mut EditorPane> = None;
        let mut best_score = 0u64;
        for (_, tab) in self.dock_state.iter_all_tabs_mut() {
            let id = tab.id;
            if let RealPane::EmulatorPanes(p) = &mut tab.inner {
                if let EmulatorPane::Editor(e) = p.as_mut() {
                    if e.is_os() {
                        continue;
                    }
                    let score = if Some(id) == focused_id {
                        u64::MAX
                    } else {
                        e.last_active_frame + 1
                    };
                    if best.is_none() || score > best_score {
                        best_score = score;
                        best = Some(e);
                    }
                }
            }
        }
        best
    }

    /// Re-assemble every file that was loaded and has been edited since, so the machine never
    /// runs old code. Returns false (and leaves the machine alone) if one fails to assemble.
    fn reassemble_stale(&mut self, emu: &mut Emulator) -> bool {
        let loaded_ids: Vec<u64> = emu.loaded_programs.iter().map(|p| p.source_id).collect();
        for e in self.editors() {
            if !e.is_os()
                && loaded_ids.contains(&e.doc.id)
                && !e.doc.is_loaded()
                && !e.assemble(emu)
            {
                return false;
            }
        }
        true
    }

    fn find_editor_tab(
        &self,
        pred: impl Fn(&EditorPane) -> bool,
    ) -> Option<(SurfaceIndex, NodeIndex, TabIndex)> {
        self.dock_state.find_tab_from(|t| match &t.inner {
            RealPane::EmulatorPanes(p) => match p.as_ref() {
                EmulatorPane::Editor(e) => pred(e),
                _ => false,
            },
            _ => false,
        })
    }

    /// Bring the read-only OS source tab to the front, opening it next to the program editor.
    /// Close the datapath, or open it beside the editor (Figure C.3 is tall, so it gets a column).
    fn toggle_datapath(&mut self) {
        let is_datapath = |t: &Pane| matches!(&t.inner, RealPane::EmulatorPanes(p) if matches!(p.as_ref(), EmulatorPane::Datapath(_)));
        if let Some(loc) = self.dock_state.find_tab_from(is_datapath) {
            self.dock_state.remove_tab(loc);
            return;
        }
        let pane = emu_pane(EmulatorPane::Datapath(Default::default()));
        let editor = self.dock_state.find_tab_from(|t| matches!(&t.inner, RealPane::EmulatorPanes(p) if matches!(p.as_ref(), EmulatorPane::Editor(_))));
        match editor {
            Some((surface, node, _)) => {
                // split the editor's whole column (editor and terminal) so the figure gets the full height
                let target = node.parent().unwrap_or(node);
                self.dock_state[surface].split_right(target, 0.5, vec![pane]);
            }
            None => self.dock_state.push_to_first_leaf(pane),
        }
    }

    fn show_os_source(&mut self) {
        if let Some(loc) = self.find_editor_tab(|e| e.is_os()) {
            self.dock_state.set_active_tab(loc);
            return;
        }
        let program_id = self.program_editor().map(|e| e.doc.id);
        let pane = emu_pane(EmulatorPane::Editor(EditorPane::os()));
        match self.find_editor_tab(|e| Some(e.doc.id) == program_id) {
            Some((surface, node, _)) => {
                self.dock_state
                    .set_focused_node_and_surface((surface, node));
                self.dock_state.push_to_focused_leaf(pane);
            }
            None => self.dock_state.push_to_first_leaf(pane),
        }
    }

    /// While debugging, show the OS source when execution stops inside the OS, and go back to
    /// the program when it returns.
    fn follow_execution_into_os(&mut self) {
        let (stopped, pc, cycles, halted) = {
            let emu = EMULATOR.lock().unwrap();
            (
                !emu.running(),
                emu.pc.get() as usize,
                emu.cycles,
                emu.halted,
            )
        };
        // which loaded document holds an address
        let owner_of = |addr: usize| -> Option<u64> {
            EMULATOR
                .lock()
                .ok()
                .and_then(|e| e.program_at(addr).map(|p| p.source_id))
        };
        let reveal = editor::REVEAL_ADDR.lock().ok().and_then(|r| *r);
        if let Some(a) = reveal {
            if a < 0x3000 {
                self.show_os_source();
            } else if let Some(id) = owner_of(a) {
                if let Some(loc) = self.find_editor_tab(|e| e.doc.id == id) {
                    self.dock_state.set_active_tab(loc);
                }
            }
        }
        // a halted machine sits in the OS halt loop; that isn't the user stepping into the OS
        if !stopped || cycles == 0 || halted {
            return;
        }
        let in_os = pc < 0x3000
            && crate::emulator::os_source()
                .address_to_line
                .contains_key(&pc);
        if in_os && !self.was_in_os {
            self.show_os_source();
        } else if !in_os && (self.was_in_os || self.last_stop_pc != Some(pc)) {
            // show the file holding the code we stopped in
            if let Some(id) = owner_of(pc) {
                if self.last_owner != Some(id) || self.was_in_os {
                    if let Some(loc) = self.find_editor_tab(|e| e.doc.id == id && e.doc.is_loaded())
                    {
                        self.dock_state.set_active_tab(loc);
                    }
                }
                self.last_owner = Some(id);
            }
        }
        self.last_stop_pc = Some(pc);
        self.was_in_os = in_os;
    }

    /// Open an editor tab next to the active editor.
    fn add_editor(&mut self, editor: EditorPane) {
        let active_id = self.program_editor().map(|e| e.doc.id);
        // Replace the untouched starter document rather than piling up tabs
        if let Some(e) = self.program_editor() {
            if e.is_untouched_default() {
                *e = editor;
                return;
            }
        }
        let target = self.dock_state.find_tab_from(|t| match &t.inner {
            RealPane::EmulatorPanes(p) => match p.as_ref() {
                EmulatorPane::Editor(e) => Some(e.doc.id) == active_id,
                _ => false,
            },
            _ => false,
        });
        let pane = emu_pane(EmulatorPane::Editor(editor));
        match target {
            Some((surface, node, _)) => {
                self.dock_state
                    .set_focused_node_and_surface((surface, node));
                self.dock_state.push_to_focused_leaf(pane);
            }
            None => self.dock_state.push_to_first_leaf(pane),
        }
    }

    fn run_action(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::Assemble => {
                let mut emu = EMULATOR.lock().unwrap();
                if let Some(e) = self.program_editor() {
                    e.assemble(&mut emu);
                }
            }
            Action::RunPause => {
                let mut emu = EMULATOR.lock().unwrap();
                if emu.running() {
                    emu.pause();
                } else {
                    // Assemble first if the active editor, or any other loaded file, has changed
                    // since it was loaded
                    if let Some(e) = self.program_editor() {
                        if !e.doc.is_loaded() && !e.assemble(&mut emu) {
                            return;
                        }
                    }
                    if !self.reassemble_stale(&mut emu) {
                        return;
                    }
                    emu.resume();
                }
            }
            Action::StepBack => EMULATOR.lock().unwrap().step_back(),
            Action::StepOver | Action::StepInto | Action::StepOut | Action::MicroStep => {
                let mut emu = EMULATOR.lock().unwrap();
                if let Some(e) = self.program_editor() {
                    if e.doc.assembled_text.is_none()
                        && emu.loaded_programs.is_empty()
                        && !e.assemble(&mut emu)
                    {
                        return;
                    }
                }
                if !emu.running() && !self.reassemble_stale(&mut emu) {
                    return;
                }
                if emu.running() && action != Action::StepOut {
                    emu.pause();
                }
                match action {
                    Action::StepOver => emu.step_over(),
                    Action::StepInto => emu.step_into(),
                    Action::StepOut => emu.step_out(),
                    _ => emu.micro_step_command(),
                }
            }
            Action::Reset => {
                EMULATOR.lock().unwrap().hard_reset();
            }
            Action::NewFile => {
                let n = self.editors().count() + 1;
                self.add_editor(EditorPane::untitled(n));
            }
            Action::OpenFile => editor::open_file_dialog(),
            Action::Save => {
                if let Some(e) = self.active_editor() {
                    e.save(false);
                }
            }
            Action::SaveAs => {
                if let Some(e) = self.active_editor() {
                    e.save(true);
                }
            }
            Action::Find => {
                if let Some(e) = self.active_editor() {
                    e.open_find(false);
                }
            }
            Action::Replace => {
                if let Some(e) = self.active_editor() {
                    e.open_find(true);
                }
            }
            Action::GoToLine => {
                if let Some(e) = self.active_editor() {
                    e.open_goto();
                }
            }
            Action::Edit(cmd) => {
                if let Some(e) = self.active_editor() {
                    e.command(ctx, cmd);
                }
            }
            Action::ShowShortcuts => self.show_shortcuts = !self.show_shortcuts,
        }
        ctx.request_repaint();
    }

    fn apply_speed(&self) {
        let mut emulator = EMULATOR.lock().unwrap();
        emulator.max_speed = self.speed >= SPEED_MAX;
        let cps = self.cycles_per_second();
        if cps < UPDATES_PER_SECOND {
            emulator.ticks_between_updates = (UPDATES_PER_SECOND / cps).round().max(1.0) as u32;
            emulator.speed = 1;
        } else {
            emulator.ticks_between_updates = 1;
            emulator.speed = (cps / UPDATES_PER_SECOND).round() as u32;
        }
    }

    /// Clock cycles per second for the speed setting (infinite at Max): 1 cycle/s at the left,
    /// ten times faster every SPEED_MAX / 9 steps, up to 100M cycles/s just below Max.
    fn cycles_per_second(&self) -> f32 {
        if self.speed >= SPEED_MAX {
            return f32::INFINITY;
        }
        10f32.powf(self.speed as f32 * 8.0 / (SPEED_MAX - 1) as f32)
    }

    /// Switch between the light and dark themes (dropping any Theme Editor changes).
    fn set_theme(&mut self, ctx: &egui::Context, choice: BaseThemeChoice) {
        self.theme.set_global_theme(choice, Some(ctx));
        self.tree_behavior.theme = self.theme.clone();
    }

    fn toolbar(&mut self, ctx: &egui::Context) {
        let theme = self.theme.clone();
        let mut action = None;
        egui::TopBottomPanel::top("toolbar")
            .exact_height(42.0)
            .frame(
                egui::Frame::new()
                    .fill(theme.panel_background)
                    .inner_margin(egui::Margin::symmetric(8, 0))
                    .stroke(Stroke::NONE),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    // when the left side (menus, status, speed) and the labelled run buttons don't
                    // both fit, the buttons drop their labels. Widths are last frame's.
                    let (left_id, right_id) = (egui::Id::new("toolbar-left-w"), egui::Id::new("toolbar-right-w"));
                    let start_x = ui.cursor().min.x;
                    let (left_w, right_w) = ui.ctx().data(|d| (d.get_temp::<f32>(left_id).unwrap_or(0.0), d.get_temp::<f32>(right_id).unwrap_or(560.0)));
                    let compact = left_w + right_w + 24.0 > ui.available_width();
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("toolbar-compact"), compact));
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.add_space(8.0);
                    ui.scope(|ui| {
                        ui.visuals_mut().button_frame = false;
                        ui.spacing_mut().item_spacing.x = 14.0;
                        self.menus(ui, &mut action);
                    });
                    if cfg!(target_arch = "wasm32") {
                        ui.add_space(8.0);
                        download_button(ui, &theme);
                    }
                    toolbar_separator(ui, &theme);

                    let (running, halted, can_back) = {
                        let emu = EMULATOR.lock().unwrap();
                        (emu.running(), emu.halted, emu.can_step_back())
                    };

                    // left: machine state, speed and the skip OS switch
                    self.status_pill(ui, &theme);
                    toolbar_separator(ui, &theme);
                    if !compact {
                        ui.label(RichText::new("Speed").size(12.0).color(theme.secondary_text_color));
                    }
                    let old = self.speed;
                    ui.spacing_mut().slider_width = 90.0;
                    ui.add(egui::Slider::new(&mut self.speed, 0..=SPEED_MAX).show_value(false))
                        .explain("How fast Run executes. Slow speeds let you watch each instruction; fast ones run whole instructions directly, without recording the states for the datapath. All the way right is Max: as fast as your computer goes.");
                    if old != self.speed {
                        self.apply_speed();
                    }
                    let cps = self.cycles_per_second();
                    let cps_text = if cps.is_infinite() {
                        "Max".to_string()
                    } else if cps >= 1e6 {
                        format!("{:.0}M/s", cps / 1e6)
                    } else if cps >= 1000.0 {
                        format!("{:.0}k/s", cps / 1000.0)
                    } else if cps >= 10.0 {
                        format!("{cps:.0}/s")
                    } else {
                        format!("{cps:.1}/s")
                    };
                    if !compact {
                        ui.label(RichText::new(cps_text).monospace().size(11.5).color(theme.secondary_text_color))
                            .explain(if cps.is_infinite() {
                                "As fast as possible: the machine runs for most of each frame".to_string()
                            } else {
                                "Clock cycles per second (each instruction takes about 6–10)".to_string()
                            });
                    }
                    ui.add_space(10.0);
                    {
                        let mut emu = EMULATOR.lock().unwrap();
                        toggle_switch(ui, &mut emu.skip_os_emulation, &theme);
                    }
                    ui.label(RichText::new("Skip OS").size(12.0).color(theme.secondary_text_color))
                        .explain("Run OS service routines (PUTS, GETC, …) in one step instead of stepping through them");
                    let left_w = ui.cursor().min.x - start_x;
                    ui.ctx().data_mut(|d| d.insert_temp(left_id, left_w));

                    // right: run controls (laid out right to left, so in reverse order)
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        if tool_button(ui, &theme, icons::ARROW_COUNTER_CLOCKWISE.0, "", &tip(ui.ctx(), "Reset the machine and reload the programs", Action::Reset), ButtonKind::Icon).clicked() {
                            action = Some(Action::Reset);
                        }
                        ui.add_enabled_ui(!halted, |ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            if tool_button(ui, &theme, icons::DOTS_THREE.0, "Micro", &tip(ui.ctx(), "Micro step: run one state of the LC-3 state machine (one clock cycle)", Action::MicroStep), ButtonKind::Normal).clicked() {
                                action = Some(Action::MicroStep);
                            }
                            ui.add_enabled_ui(can_back, |ui| {
                                if tool_button(ui, &theme, icons::ARROW_U_UP_LEFT.0, "Back", &tip(ui.ctx(), "Back one state: the whole machine goes back a clock cycle (as far as the start of the current instruction)", Action::StepBack), ButtonKind::Normal).clicked() {
                                    action = Some(Action::StepBack);
                                }
                            });
                            if tool_button(ui, &theme, icons::ARROW_LINE_UP.0, "Out", &tip(ui.ctx(), "Step out: run until the current subroutine returns", Action::StepOut), ButtonKind::Normal).clicked() {
                                action = Some(Action::StepOut);
                            }
                            if tool_button(ui, &theme, icons::ARROW_LINE_DOWN.0, "Into", &tip(ui.ctx(), "Step into: run one instruction", Action::StepInto), ButtonKind::Normal).clicked() {
                                action = Some(Action::StepInto);
                            }
                            if tool_button(ui, &theme, icons::ARROW_ARC_RIGHT.0, "Over", &tip(ui.ctx(), "Step over: run one instruction, finishing any subroutine it calls", Action::StepOver), ButtonKind::Normal).clicked() {
                                action = Some(Action::StepOver);
                            }
                        });
                        ui.add_space(4.0);
                        let (icon, label, text) = if running {
                            (icons::PAUSE.0, "Pause", "Pause")
                        } else if halted {
                            (icons::PLAY.0, "Restart", "Run the program again from the start")
                        } else {
                            (icons::PLAY.0, "Run", "Run. Assembles first if a loaded file changed")
                        };
                        let run_tip = tip(ui.ctx(), text, Action::RunPause);
                        if tool_button(ui, &theme, icon, label, &run_tip, ButtonKind::Primary).clicked() {
                            action = Some(Action::RunPause);
                        }
                        ui.add_space(4.0);
                        if tool_button(ui, &theme, icons::HAMMER.0, "Assemble", &tip(ui.ctx(), "Assemble and load the active file", Action::Assemble), ButtonKind::Normal).clicked() {
                            action = Some(Action::Assemble);
                        }
                        if !compact {
                            let w = ui.min_rect().width();
                            ui.ctx().data_mut(|d| d.insert_temp(right_id, w));
                        }
                    });
                });
            });
        let r = ctx.available_rect();
        ctx.layer_painter(egui::LayerId::background()).hline(
            r.x_range(),
            r.min.y - 0.5,
            Stroke::new(1.0_f32, theme.separator_color),
        );
        if let Some(a) = action {
            self.run_action(ctx, a);
        }
    }

    fn menus(&mut self, ui: &mut egui::Ui, action: &mut Option<Action>) {
        let item = |ui: &mut egui::Ui, label: &str, shortcut: &str| -> bool {
            ui.add(egui::Button::new(label).shortcut_text(shortcut))
                .clicked()
        };
        ui.menu_button("File", |ui| {
            if item(ui, "New file", &hint(ui.ctx(), Action::NewFile)) {
                *action = Some(Action::NewFile);
            }
            if item(ui, "Open…", &hint(ui.ctx(), Action::OpenFile)) {
                *action = Some(Action::OpenFile);
            }
            ui.menu_button("Open example", |ui| {
                for (name, code) in crate::examples::EXAMPLES {
                    if ui.button(*name).clicked() {
                        let file = format!("{}.asm", name.to_lowercase().replace(' ', "_"));
                        self.add_editor(EditorPane::new(Document::new(file, *code)));
                        ui.close();
                    }
                }
            });
            ui.separator();
            if item(ui, "Save", &hint(ui.ctx(), Action::Save)) {
                *action = Some(Action::Save);
            }
            if item(ui, "Save as…", &hint(ui.ctx(), Action::SaveAs)) {
                *action = Some(Action::SaveAs);
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                ui.separator();
                if ui.button("Quit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        });
        ui.menu_button("Edit", |ui| {
            if item(ui, "Find", "Ctrl+F") {
                *action = Some(Action::Find);
            }
            if item(ui, "Replace", "Ctrl+H") {
                *action = Some(Action::Replace);
            }
            if item(ui, "Go to line…", "Ctrl+G") {
                *action = Some(Action::GoToLine);
            }
            ui.separator();
            if item(ui, "Toggle comment", "Ctrl+/") {
                *action = Some(Action::Edit(TextCommand::ToggleComment));
            }
            if item(ui, "Duplicate line", "Ctrl+D") {
                *action = Some(Action::Edit(TextCommand::Duplicate));
            }
            if item(
                ui,
                "Delete line",
                if cfg!(target_arch = "wasm32") {
                    ""
                } else {
                    "Ctrl+Shift+K"
                },
            ) {
                *action = Some(Action::Edit(TextCommand::DeleteLine));
            }
            if item(ui, "Move line up", "Alt+↑") {
                *action = Some(Action::Edit(TextCommand::MoveUp));
            }
            if item(ui, "Move line down", "Alt+↓") {
                *action = Some(Action::Edit(TextCommand::MoveDown));
            }
            ui.separator();
            if item(ui, "Format document", "Shift+Alt+F") {
                *action = Some(Action::Edit(TextCommand::Format));
            }
        });
        ui.menu_button("Run", |ui| {
            if item(ui, "Assemble and load", &hint(ui.ctx(), Action::Assemble)) {
                *action = Some(Action::Assemble);
            }
            if item(ui, "Run / pause", &hint(ui.ctx(), Action::RunPause)) {
                *action = Some(Action::RunPause);
            }
            if item(ui, "Step over", &hint(ui.ctx(), Action::StepOver)) {
                *action = Some(Action::StepOver);
            }
            if item(ui, "Step into", &hint(ui.ctx(), Action::StepInto)) {
                *action = Some(Action::StepInto);
            }
            if item(ui, "Step out", &hint(ui.ctx(), Action::StepOut)) {
                *action = Some(Action::StepOut);
            }
            if item(ui, "Micro step", &hint(ui.ctx(), Action::MicroStep)) {
                *action = Some(Action::MicroStep);
            }
            if item(ui, "Back one state", &hint(ui.ctx(), Action::StepBack)) {
                *action = Some(Action::StepBack);
            }
            ui.separator();
            ui.menu_button("Operating system", |ui| {
                let current = crate::emulator::os_kind();
                for os in crate::emulator::OsKind::ALL {
                    if ui.radio(current == os, os.label()).explain(os.about()).clicked() {
                        switch_os(os);
                        ui.close();
                    }
                }
            })
            .response
            .explain("How the OS reads the keyboard: interrupts or polling. Switching resets the machine");
            if item(ui, "Reset machine", &hint(ui.ctx(), Action::Reset)) {
                *action = Some(Action::Reset);
            }
            if ui.button("Unload all programs").explain("Clear memory back to just the OS").clicked() {
                EMULATOR.lock().unwrap().unload_programs();
                ui.close();
            }
            if ui.button("Clear all breakpoints").clicked() {
                EMULATOR.lock().unwrap().breakpoints.clear();
                for e in self.editors() {
                    e.doc.breakpoints.clear();
                }
                ui.close();
            }
        });
        ui.menu_button("View", |ui| {
            ui.menu_button("Add pane", |ui| {
                for pane in match Pane::children() {
                    PaneTree::Children(_, c) => c,
                    p => vec![p],
                } {
                    self.tree_behavior.add_pane_menu_items(ui, pane);
                }
            });
            if ui.button("OS source (read only)").explain("The operating system's code, with the same tools as the editor. It also opens by itself when you step into the OS.").clicked() {
                self.show_os_source();
                ui.close();
            }
            let datapath = self.dock_state.find_tab_from(|t| matches!(&t.inner, RealPane::EmulatorPanes(p) if matches!(p.as_ref(), EmulatorPane::Datapath(_))));
            let mut show_datapath = datapath.is_some();
            if ui
                .checkbox(&mut show_datapath, "Datapath")
                .explain("Watch each instruction move through the LC-3 datapath (Figure C.3), step by step")
                .changed()
            {
                self.toggle_datapath();
                ui.close();
            }
            let mut arrows = self.editors().next().is_some_and(|e| e.show_arrows);
            if ui.checkbox(&mut arrows, "Branch and call arrows in the editor").changed() {
                for e in self.editors() {
                    e.show_arrows = arrows;
                }
            }
            let mut show_addr = self.editors().next().is_some_and(|e| e.show_addresses);
            if ui.checkbox(&mut show_addr, "Line addresses in the editor").changed() {
                for e in self.editors() {
                    e.show_addresses = show_addr;
                }
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Zoom");
                // drag a copy and apply it on release: applying while dragging rescales the
                // slider under the pointer
                let id = egui::Id::new("zoom-drag");
                let mut zoom = ui.data(|d| d.get_temp::<f32>(id)).unwrap_or(self.scale);
                let res = ui.add(
                    egui::Slider::new(&mut zoom, 0.5..=5.0)
                        .step_by(0.05)
                        .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                        .custom_parser(|t| t.trim().trim_end_matches('%').parse::<f64>().ok().map(|v| v / 100.0)),
                );
                if res.dragged() {
                    ui.data_mut(|d| d.insert_temp(id, zoom));
                } else {
                    if res.changed() || res.drag_stopped() {
                        self.scale = zoom;
                    }
                    ui.data_mut(|d| d.remove::<f32>(id));
                }
                if ui.small_button("Reset").clicked() {
                    self.scale = 1.0;
                }
            });
            ui.menu_button("Theme", |ui| {
                // None: match the system
                let current = if self.theme_follows_system { None } else { Some(self.theme.base_theme) };
                let mut choice = current;
                ui.radio_value(&mut choice, None, "Match system")
                    .explain("Light or dark, following your computer's setting");
                ui.radio_value(&mut choice, Some(BaseThemeChoice::Dark), "Dark")
                    .explain("The default: the highest contrast for the datapath and code");
                ui.radio_value(&mut choice, Some(BaseThemeChoice::Light), "Light");
                if choice != current {
                    self.theme_follows_system = choice.is_none();
                    if let Some(c) = choice {
                        self.set_theme(ui.ctx(), c);
                    }
                }
            });
            ui.separator();
            ui.menu_button("Layout", |ui| {
                let editors: Vec<EditorPane> = self.editors().filter(|e| !e.is_os()).map(|e| EditorPane::new(e.doc.clone())).collect();
                let editors = if editors.is_empty() { vec![EditorPane::default()] } else { editors };
                if ui.button("Coding").explain("Big editor; terminal and help share the bottom").clicked() {
                    self.dock_state = default_layout(editors);
                    ui.close();
                } else if ui.button("Datapath").explain("Editor, a full-height datapath, then CPU and memory").clicked() {
                    self.dock_state = datapath_layout(editors);
                    ui.close();
                } else if ui.button("Large datapath").explain("The datapath takes most of the window (for a projector); the editor and CPU beside it").clicked() {
                    self.dock_state = large_datapath_layout(editors);
                    ui.close();
                }
            })
            .response
            .explain("Restore a pane layout. Open files are kept.");
        });
        ui.menu_button("Help", |ui| {
            if item(ui, "Keyboard shortcuts", &hint(ui.ctx(), Action::ShowShortcuts)) {
                *action = Some(Action::ShowShortcuts);
            }
            if ui.button("LC-3 reference").clicked() {
                let open = self.dock_state.find_tab_from(|t| matches!(&t.inner, RealPane::EmulatorPanes(p) if matches!(p.as_ref(), EmulatorPane::Help(_))));
                match open {
                    Some(loc) => {
                        self.dock_state.set_active_tab(loc);
                    }
                    None => self.dock_state.push_to_first_leaf(emu_pane(EmulatorPane::Help(Default::default()))),
                }
                ui.close();
            }
            ui.separator();
            ui.hyperlink_to(format!("{} Download the desktop app", icons::DOWNLOAD_SIMPLE.0), DOWNLOAD_URL);
            ui.hyperlink_to("Send feedback or report a bug", FEEDBACK_URL);
            if ui.button("About").clicked() {
                self.show_about = true;
                ui.close();
            }
        });
    }

    fn status_pill(&self, ui: &mut egui::Ui, theme: &ThemeSettings) {
        let emu = EMULATOR.lock().unwrap();
        let pc = emu.pc.get();
        let (color, text) = if emu.running() {
            if emu.asleep && emu.memory[crate::emulator::KBSR_ADDR].get() & 0x4000 == 0 {
                (
                    theme.accent_color_negative,
                    "Asleep · nothing can wake it (KBSR[14] is 0)".to_string(),
                )
            } else if emu.asleep && emu.input_queue.is_empty() {
                (
                    theme.accent_color_primary,
                    "Asleep · a key will wake it (interrupt)".to_string(),
                )
            } else if emu.waiting_for_input() {
                (
                    theme.accent_color_primary,
                    "Waiting for keyboard input".to_string(),
                )
            } else {
                (theme.accent_color_positive, "Running".to_string())
            }
        } else if emu.exception.is_some() {
            (
                theme.accent_color_negative,
                format!("Exception {:?}", emu.exception.as_ref().unwrap()),
            )
        } else {
            match emu.stop_reason {
                // the assembler puts a breakpoint on every HALT so registers can be inspected
                StopReason::Breakpoint(a) if emu.memory[a as usize].get() == 0xF025 => (
                    theme.accent_color_primary,
                    "Stopped before HALT · Run to finish".to_string(),
                ),
                StopReason::Breakpoint(a) => (
                    theme.accent_color_negative,
                    format!("Paused at breakpoint · x{a:04X}"),
                ),
                StopReason::Halted => (theme.secondary_text_color, "Halted".to_string()),
                StopReason::Paused | StopReason::Stepped => {
                    (theme.accent_color_primary, format!("Paused · x{pc:04X}"))
                }
                StopReason::Ready if !emu.loaded_programs.is_empty() => (
                    theme.accent_color_secondary,
                    "Ready · press Run".to_string(),
                ),
                StopReason::Ready => (
                    theme.secondary_text_color,
                    "Assemble a program to start".to_string(),
                ),
            }
        };
        drop(emu);
        let font = FontId::monospace(11.5);
        let galley = ui
            .painter()
            .layout_no_wrap(text, font, theme.primary_text_color);
        let size = Vec2::new(galley.size().x + 30.0, 24.0);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        ui.painter().rect(
            rect,
            CornerRadius::same(12),
            theme.window_background,
            Stroke::new(1.0_f32, theme.separator_color),
            egui::StrokeKind::Inside,
        );
        ui.painter()
            .circle_filled(rect.left_center() + Vec2::new(12.0, 0.0), 4.0, color);
        ui.painter().galley(
            rect.left_center() + Vec2::new(22.0, -galley.size().y / 2.0),
            galley,
            theme.primary_text_color,
        );
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        let theme = self.theme.clone();
        let mut reveal: Option<usize> = None;
        let editor_info = self.active_editor().map(|e| {
            (
                e.doc.diagnostics().to_vec(),
                e.status.clone(),
                e.cursor,
                e.doc.is_loaded(),
            )
        });
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(24.0)
            .frame(
                egui::Frame::new()
                    .fill(theme.panel_background)
                    .inner_margin(egui::Margin::symmetric(10, 0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    let small =
                        |s: String, c: Color32| RichText::new(s).monospace().size(11.5).color(c);
                    if let Some((diags, status, cursor, loaded)) = &editor_info {
                        if let Some(d) = diags.first() {
                            let r = ui.add(
                                egui::Label::new(small(
                                    format!(
                                        "{} Line {}: {}",
                                        icons::X_CIRCLE.0,
                                        d.line + 1,
                                        d.message
                                    ),
                                    theme.error_fg_color,
                                ))
                                .sense(Sense::click()),
                            );
                            if r.explain("Click to go to the error").clicked() {
                                reveal = Some(d.line);
                            }
                        } else {
                            ui.label(small(
                                format!("{} No problems", icons::CHECK_CIRCLE.0),
                                theme.secondary_text_color,
                            ));
                        }
                        match status {
                            AssembleStatus::Ok { words, start } if *loaded => {
                                ui.label(small(
                                    format!(
                                        "Loaded {words} words at x{start:04X}–x{:04X}",
                                        start + words.saturating_sub(1)
                                    ),
                                    theme.success_fg_color,
                                ));
                            }
                            AssembleStatus::Ok { .. } => {
                                ui.label(small(
                                    "Changed since last assemble (Ctrl+B)".into(),
                                    theme.warn_fg_color,
                                ));
                            }
                            _ => {}
                        }
                        let _ = cursor;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add(
                            Hyperlink::from_label_and_url(
                                small("Feedback".into(), theme.secondary_text_color),
                                FEEDBACK_URL,
                            )
                            .open_in_new_tab(true),
                        );
                        if let Some((_, _, (l, c), _)) = &editor_info {
                            ui.label(small(
                                format!("Ln {l}, Col {c}"),
                                theme.secondary_text_color,
                            ));
                        }
                        if self.fps.is_finite() && self.fps > 0.0 {
                            ui.label(small(
                                format!("{:.0} fps", self.fps),
                                theme.secondary_text_color,
                            ))
                            .explain("Frames drawn per second");
                        }
                        let emu = EMULATOR.lock().unwrap();
                        ui.label(small(
                            format!("{} cycles", emu.cycles),
                            theme.secondary_text_color,
                        ));
                        ui.label(small(
                            format!("{} instructions", emu.instructions),
                            theme.secondary_text_color,
                        ));
                        ui.label(small(
                            format!("PC x{:04X}", emu.pc.get()),
                            theme.secondary_text_color,
                        ));
                    });
                });
            });
        if let Some(l) = reveal {
            if let Some(e) = self.active_editor() {
                e.reveal_line(l);
            }
        }
    }

    fn shortcuts_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_shortcuts;
        egui::Window::new("Keyboard shortcuts")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                let theme = &self.theme;
                ui.columns(2, |cols| {
                    cols[0].label(RichText::new("Run and files").strong());
                    egui::Grid::new("sc_app")
                        .num_columns(2)
                        .spacing([16.0, 4.0])
                        .show(&mut cols[0], |ui| {
                            let mut seen = std::collections::HashSet::new();
                            for s in shortcuts() {
                                if !seen.insert(s.label) {
                                    continue;
                                }
                                ui.label(
                                    RichText::new(ui.ctx().format_shortcut(&s.shortcut))
                                        .monospace()
                                        .color(theme.accent_color_primary),
                                );
                                ui.label(s.label);
                                ui.end_row();
                            }
                        });
                    cols[1].label(RichText::new("Editor").strong());
                    egui::Grid::new("sc_editor")
                        .num_columns(2)
                        .spacing([16.0, 4.0])
                        .show(&mut cols[1], |ui| {
                            for (k, d) in EDITOR_SHORTCUTS {
                                ui.label(
                                    RichText::new(*k)
                                        .monospace()
                                        .color(theme.accent_color_primary),
                                );
                                ui.label(*d);
                                ui.end_row();
                            }
                        });
                });
            });
        self.show_shortcuts = open;
    }

    fn dock_style(&self, ctx: &egui::Context) -> Style {
        let t = &self.theme;
        let mut style = Style::from_egui(ctx.style().as_ref());
        style.dock_area_padding = None;
        style.main_surface_border_stroke = Stroke::NONE;
        style.separator.width = 1.0;
        style.separator.extra_interact_width = 4.0;
        style.separator.color_idle = t.separator_color;
        style.separator.color_hovered = t.accent_color_secondary;
        style.separator.color_dragged = t.accent_color_secondary;
        style.tab_bar.bg_fill = t.window_background;
        style.tab_bar.height = 32.0;
        style.tab_bar.hline_color = t.separator_color;
        style.tab_bar.corner_radius = CornerRadius::ZERO;
        style.tab.spacing = 0.0;
        style.tab.hline_below_active_tab_name = false;
        style.tab.tab_body.bg_fill = t.panel_background;
        style.tab.tab_body.stroke = Stroke::NONE;
        style.tab.tab_body.inner_margin = egui::Margin::ZERO;
        style.tab.tab_body.corner_radius = CornerRadius::ZERO;
        for (s, bg, fg) in [
            (
                &mut style.tab.active,
                t.panel_background,
                t.strong_text_color,
            ),
            (
                &mut style.tab.focused,
                t.panel_background,
                t.strong_text_color,
            ),
            (
                &mut style.tab.active_with_kb_focus,
                t.panel_background,
                t.strong_text_color,
            ),
            (
                &mut style.tab.focused_with_kb_focus,
                t.panel_background,
                t.strong_text_color,
            ),
            (
                &mut style.tab.inactive,
                t.window_background,
                t.secondary_text_color,
            ),
            (
                &mut style.tab.inactive_with_kb_focus,
                t.window_background,
                t.secondary_text_color,
            ),
            (
                &mut style.tab.hovered,
                t.widget_fill_interactive,
                t.primary_text_color,
            ),
        ] {
            s.bg_fill = bg;
            s.text_color = fg;
            s.outline_color = t.separator_color;
            s.corner_radius = CornerRadius::ZERO;
        }
        style.buttons.add_tab_bg_fill = t.window_background;
        style.buttons.add_tab_color = t.secondary_text_color;
        style.buttons.add_tab_active_color = t.strong_text_color;
        style.buttons.add_tab_border_color = t.separator_color;
        style.buttons.close_tab_bg_fill = t.widget_fill_hovered;
        style.buttons.close_tab_color = t.secondary_text_color;
        style.buttons.close_tab_active_color = t.strong_text_color;
        style.overlay.selection_color = t.accent_color_secondary.gamma_multiply(0.4);
        style
    }
}

#[derive(PartialEq)]
enum ButtonKind {
    Normal,
    Primary,
    Icon,
}

/// A flat toolbar button: icon plus label, filled on hover, amber when primary.
fn tool_button(
    ui: &mut egui::Ui,
    theme: &ThemeSettings,
    icon: &str,
    label: &str,
    tip: &str,
    kind: ButtonKind,
) -> egui::Response {
    let enabled = ui.is_enabled();
    let font = FontId::proportional(13.0);
    let compact = kind == ButtonKind::Normal
        && ui
            .ctx()
            .data(|d| d.get_temp::<bool>(egui::Id::new("toolbar-compact")))
            .unwrap_or(false);
    let label = if compact { "" } else { label };
    let text = if label.is_empty() {
        icon.to_string()
    } else {
        format!("{icon}  {label}")
    };
    let galley = ui
        .painter()
        .layout_no_wrap(text, font, Color32::PLACEHOLDER);
    let w = if kind == ButtonKind::Icon {
        30.0
    } else {
        galley.size().x + 20.0
    };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 28.0), Sense::click());
    let hovered = resp.hovered() && enabled;
    let (bg, fg) = match kind {
        ButtonKind::Primary => (
            if hovered {
                theme.accent_color_primary.gamma_multiply(1.15)
            } else {
                theme.accent_color_primary
            },
            theme.on_accent(),
        ),
        _ => (
            if resp.is_pointer_button_down_on() && enabled {
                theme.widget_fill_active
            } else if hovered {
                theme.widget_fill_hovered
            } else {
                Color32::TRANSPARENT
            },
            if enabled {
                theme.primary_text_color
            } else {
                theme.secondary_text_color.gamma_multiply(0.6)
            },
        ),
    };
    ui.painter().rect_filled(rect, CornerRadius::same(5), bg);
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, fg);
    resp.explain(tip)
}

/// The "get the desktop app" button shown in the web build's toolbar.
fn download_button(ui: &mut egui::Ui, theme: &ThemeSettings) {
    let text = format!("{}  Get the desktop app", icons::DOWNLOAD_SIMPLE.0);
    let font = FontId::proportional(13.0);
    let galley = ui.painter().layout_no_wrap(text, font, Color32::WHITE);
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(galley.size().x + 22.0, 28.0), Sense::click());
    let bg = if resp.hovered() {
        theme.accent_color_secondary
    } else {
        theme.accent_color_secondary.gamma_multiply(0.85)
    };
    ui.painter().rect_filled(rect, CornerRadius::same(5), bg);
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, Color32::WHITE);
    let resp = resp
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .explain(format!(
            "{DOWNLOAD_PITCH}\n\nOpens the downloads page in a new tab."
        ));
    if resp.clicked() {
        ui.ctx().open_url(egui::OpenUrl::new_tab(DOWNLOAD_URL));
    }
}

fn toolbar_separator(ui: &mut egui::Ui, theme: &ThemeSettings) {
    ui.add_space(6.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(1.0, 20.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme.separator_color);
    ui.add_space(6.0);
}

/// A small iOS style switch.
fn toggle_switch(ui: &mut egui::Ui, on: &mut bool, theme: &ThemeSettings) -> egui::Response {
    let size = Vec2::new(28.0, 16.0);
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool_responsive(resp.id, *on);
    let bg = if *on {
        theme.accent_color_secondary
    } else {
        theme.widget_fill_hovered
    };
    ui.painter().rect_filled(rect, CornerRadius::same(8), bg);
    let x = egui::lerp(rect.left() + 8.0..=rect.right() - 8.0, t);
    ui.painter()
        .circle_filled(egui::pos2(x, rect.center().y), 6.0, Color32::WHITE);
    resp
}

impl eframe::App for EmulatorApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "first_open", &self.first_open);
        eframe::set_value(storage, "scale", &self.scale);
        eframe::set_value(storage, "speed_pos", &self.speed);
        eframe::set_value(storage, "theme", &self.theme.base_theme);
        eframe::set_value(storage, "theme_follows_system", &self.theme_follows_system);
        eframe::set_value(storage, "os", &crate::emulator::os_kind());
        let docs: Vec<Document> = self
            .editors()
            .filter(|e| !e.is_os())
            .map(|e| e.doc.clone())
            .collect();
        eframe::set_value(storage, "documents", &docs);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let update_span = tracing::info_span!("EmulatorApp::update");
        let _update_guard = update_span.enter();

        let avg_fps = {
            let fps = (1.0 / ctx.input(|i| i.stable_dt)).max(0.0);
            self.fps_samples.push_back(fps);
            while self.fps_samples.len() > 10 {
                self.fps_samples.pop_front();
            }
            self.fps_samples.iter().copied().sum::<f32>() / self.fps_samples.len() as f32
        };

        #[cfg(target_arch = "wasm32")]
        if !self.has_dismissed_fps && EMULATOR.lock().unwrap().running() {
            use std::cmp::max;
            // Only measured while running, since we don't repaint continuously when idle
            self.bad_fps_score = max(0, self.bad_fps_score as i32 + 50 - avg_fps as i32) as u32;
            if self.bad_fps_score >= 300 {
                self.curr_bad_fps_prompt_open = true;
            }
        }
        self.fps = avg_fps;

        // the system's light or dark setting, when the theme follows it
        if self.theme_follows_system {
            let system = match ctx.system_theme() {
                Some(egui::Theme::Light) => BaseThemeChoice::Light,
                _ => BaseThemeChoice::Dark,
            };
            if system != self.theme.base_theme {
                self.set_theme(ctx, system);
            }
        }
        // the Theme Editor edits the panes' copy of the theme; the toolbar, status bar and tabs
        // draw from this one
        if self.tree_behavior.theme != self.theme {
            self.theme = self.tree_behavior.theme.clone();
        }

        editor::swap_register_hover(ctx);
        // Ctrl+click on an instruction: make sure a Help pane is showing to take it
        if crate::panes::emulator::help::SHOW_INSTRUCTION
            .lock()
            .is_ok_and(|r| r.is_some())
        {
            let is_help = |t: &Pane| matches!(&t.inner, RealPane::EmulatorPanes(p) if matches!(p.as_ref(), EmulatorPane::Help(_)));
            match self.dock_state.find_tab_from(is_help) {
                Some(loc) => {
                    self.dock_state.set_active_tab(loc);
                }
                None => self
                    .dock_state
                    .push_to_first_leaf(emu_pane(EmulatorPane::Help(Default::default()))),
            }
            ctx.request_repaint();
        }
        // trace micro-ops only while a datapath pane is on screen
        {
            let shown = crate::panes::emulator::datapath::SHOWN
                .swap(false, std::sync::atomic::Ordering::Relaxed);
            let mut emu = EMULATOR.lock().unwrap();
            if emu.trace != shown {
                emu.trace = shown;
            }
        }

        // apply the zoom set in View, or keep the one egui's Ctrl+/Ctrl- zoom keys chose
        let applied_id = egui::Id::new("zoom-applied");
        let applied = ctx
            .data(|d| d.get_temp::<f32>(applied_id))
            .unwrap_or(ctx.zoom_factor());
        if self.scale != applied {
            ctx.set_zoom_factor(self.scale);
            ctx.data_mut(|d| d.insert_temp(applied_id, self.scale));
        } else if ctx.zoom_factor() != self.scale {
            self.scale = ctx.zoom_factor();
            ctx.data_mut(|d| d.insert_temp(applied_id, self.scale));
        }
        self.apply_speed();

        // files dropped onto the window
        let dropped: Vec<egui::DroppedFile> = ctx.input(|i| i.raw.dropped_files.clone());
        for f in dropped {
            let content = match (&f.bytes, &f.path) {
                (Some(b), _) => String::from_utf8(b.to_vec()).ok(),
                #[cfg(not(target_arch = "wasm32"))]
                (None, Some(p)) => std::fs::read_to_string(p).ok(),
                _ => None,
            };
            let name = if !f.name.is_empty() {
                f.name.clone()
            } else {
                f.path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "dropped.asm".to_string())
            };
            if let Some(content) = content {
                let mut doc = Document::new(name, content);
                #[cfg(not(target_arch = "wasm32"))]
                {
                    doc.path = f.path.as_ref().map(|p| p.to_string_lossy().into_owned());
                }
                self.add_editor(EditorPane::new(doc));
            }
        }
        // show a hint while a file is dragged over the window
        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            let r = ctx.content_rect();
            let p = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("drop"),
            ));
            p.rect_filled(r, 0.0, Color32::from_black_alpha(160));
            p.text(
                r.center(),
                Align2::CENTER_CENTER,
                "Drop .asm files to open them",
                FontId::proportional(22.0),
                self.theme.strong_text_color,
            );
        }

        // files opened through the (possibly async) file dialog
        let opened: Vec<Document> = OPENED_FILES
            .lock()
            .map(|mut q| q.drain(..).collect())
            .unwrap_or_default();
        for doc in opened {
            self.add_editor(EditorPane::new(doc));
        }

        // global shortcuts
        let mut actions = Vec::new();
        // egui matches shortcuts ignoring extra Shift, so try the ones with more modifiers first
        let mut list = shortcuts();
        list.sort_by_key(|s| {
            let m = s.shortcut.modifiers;
            std::cmp::Reverse(m.shift as u8 + m.command as u8 + m.alt as u8 + m.ctrl as u8)
        });
        ctx.input_mut(|i| {
            for s in list {
                if i.consume_shortcut(&s.shortcut) {
                    actions.push(s.action);
                }
            }
        });
        for a in actions {
            self.run_action(ctx, a);
        }

        if self.first_open {
            egui::Modal::new("welcome".into()).show(ctx, |ui| {
                ui.set_width(420.0);
                ui.heading("Welcome to 210 tools");
                ui.add_space(6.0);
                ui.label("An editor, assembler and emulator for the LC-3.");
                ui.add_space(8.0);
                let theme = &self.theme;
                egui::Grid::new("welcome_grid")
                    .num_columns(2)
                    .spacing([14.0, 6.0])
                    .show(ui, |ui| {
                        let mut rows: Vec<(String, &str)> = vec![
                            (hint(ctx, Action::Assemble), "Assemble the file and load it"),
                            (hint(ctx, Action::RunPause), "Run or pause"),
                        ];
                        if cfg!(target_arch = "wasm32") {
                            rows.push((
                                "Toolbar".to_string(),
                                "Step over, into and out of instructions",
                            ));
                        } else {
                            rows.push((
                                format!(
                                    "{} / {}",
                                    hint(ctx, Action::StepOver),
                                    hint(ctx, Action::StepInto)
                                ),
                                "Step over / step into",
                            ));
                        }
                        rows.push((
                            "Gutter".to_string(),
                            "Click next to a line number for a breakpoint",
                        ));
                        if !cfg!(target_arch = "wasm32") {
                            rows.push((hint(ctx, Action::ShowShortcuts), "All keyboard shortcuts"));
                        }
                        for (k, d) in rows {
                            ui.label(
                                RichText::new(k)
                                    .monospace()
                                    .color(theme.accent_color_primary),
                            );
                            ui.label(d);
                            ui.end_row();
                        }
                    });
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Use View → Zoom if things look too small or large.")
                        .color(theme.secondary_text_color),
                );
                if cfg!(target_arch = "wasm32") {
                    ui.add_space(10.0);
                    egui::Frame::new()
                        .fill(theme.info_bg_color)
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(egui::Margin::same(10))
                        .show(ui, |ui| {
                            ui.label(RichText::new("You're using the web version").strong());
                            ui.label(DOWNLOAD_PITCH);
                            ui.add_space(4.0);
                            download_button(ui, theme);
                        });
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Start coding").color(theme.on_accent()),
                            )
                            .fill(theme.accent_color_primary),
                        )
                        .clicked()
                    {
                        self.first_open = false;
                    }
                    ui.hyperlink_to("Send feedback", FEEDBACK_URL);
                });
            });
        }

        if self.show_about {
            egui::Modal::new("about".into()).show(ctx, |ui| {
                ui.set_width(360.0);
                ui.heading("210 tools");
                ui.label("LC-3 IDE and emulator for UoA COMPSCI 210, based on Patt & Patel, Introduction to Computing Systems (3rd ed.).");
                ui.horizontal(|ui| {
                    ui.label("Developed with");
                    ui.label(icons::HEART.fill().color(self.theme.accent_color_negative));
                    ui.add(Hyperlink::from_label_and_url(RichText::new("by Jack Crump-Leys").strong(), "https://jack.crump-leys.com").open_in_new_tab(true));
                });
                ui.label(RichText::new("Fonts: Inter and JetBrains Mono (SIL Open Font License).").small().color(self.theme.secondary_text_color));
                if ui.button("Close").clicked() {
                    self.show_about = false;
                }
            });
        }

        #[cfg(target_arch = "wasm32")]
        if self.curr_bad_fps_prompt_open {
            egui::Modal::new("Bad fps detected".into()).show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("It seems you have bad fps on the web version of the tool. The desktop version is likely to run far better. You can find downloads");
                    ui.add(Hyperlink::from_label_and_url(RichText::new("here").strong(), DOWNLOAD_URL).open_in_new_tab(true));
                    ui.label(".");
                });
                ui.separator();
                ui.horizontal_top(|ui| {
                    if ui.button("Ok").clicked() {
                        self.curr_bad_fps_prompt_open = false;
                        self.has_dismissed_fps = true;
                    }
                })
            });
        }

        self.toolbar(ctx);
        self.follow_execution_into_os();
        self.status_bar(ctx);
        if self.show_shortcuts {
            self.shortcuts_window(ctx);
        }

        // confirm closing an editor with unsaved changes
        if let Some(tab_id) = self.tree_behavior.confirm_close {
            let name = self
                .dock_state
                .iter_all_tabs()
                .find(|(_, t)| t.id == tab_id)
                .and_then(|(_, t)| match &t.inner {
                    RealPane::EmulatorPanes(p) => match p.as_ref() {
                        EmulatorPane::Editor(e) => Some(e.doc.name.clone()),
                        _ => None,
                    },
                    _ => None,
                });
            match name {
                None => self.tree_behavior.confirm_close = None,
                Some(name) => {
                    egui::Modal::new("confirm_close".into()).show(ctx, |ui| {
                        ui.set_width(340.0);
                        ui.heading(format!("Close {name}?"));
                        ui.label("Your changes have not been saved and will be lost.");
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if ui.button("Close without saving").clicked() {
                                if let Some(loc) = self.dock_state.find_tab_from(|t| t.id == tab_id)
                                {
                                    self.dock_state.remove_tab(loc);
                                }
                                self.tree_behavior.confirm_close = None;
                            }
                            if ui.button("Cancel").clicked() {
                                self.tree_behavior.confirm_close = None;
                            }
                        });
                    });
                }
            }
        }

        self.dock_state.iter_surfaces_mut().for_each(|sur| {
            sur.iter_nodes_mut().for_each(|n| {
                if n.is_leaf() {
                    let tabs_mut = n.tabs_mut().unwrap();
                    if tabs_mut.len() == 1 {
                        tabs_mut[0].alone = true;
                    } else {
                        for t in tabs_mut {
                            t.alone = false
                        }
                    }
                }
            });
        });

        let style = self.dock_style(ctx);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(self.theme.separator_color))
            .show(ctx, |ui| {
                DockArea::new(&mut self.dock_state)
                    .show_add_buttons(true)
                    .show_add_popup(true)
                    .show_leaf_close_all_buttons(false)
                    .show_leaf_collapse_buttons(false)
                    .draggable_tabs(true)
                    .style(style)
                    .allowed_splits(AllowedSplits::All)
                    .show_inside(ui, &mut self.tree_behavior);
            });

        if let Some((nodei, sur)) = self.tree_behavior.last_added {
            self.tree_behavior.added_nodes.drain(..).for_each(|node| {
                self.dock_state.set_focused_node_and_surface((sur, nodei));
                self.dock_state.push_to_focused_leaf(node);
            });
        } else if !self.tree_behavior.added_nodes.is_empty() {
            for node in std::mem::take(&mut self.tree_behavior.added_nodes) {
                self.dock_state.push_to_first_leaf(node);
            }
        }

        // Keep animating while the machine runs; otherwise only repaint on input or when the
        // emulator loop reports a change.
        let running = EMULATOR.lock().unwrap().running();
        if running {
            ctx.request_repaint();
        }

        // why do we need this? Well our update loop cannot get the egui context so cannot
        // see the pass number, we need this to request a repaint if the emulator state
        // changes.
        #[cfg(not(target_arch = "wasm32"))]
        {
            *LAST_PAINT_ID.lock().unwrap() = ctx.cumulative_pass_nr_for(egui::ViewportId::ROOT);
        }
    }
}

/// Boot a different OS: the machine resets with it, keeping the loaded programs.
fn switch_os(os: crate::emulator::OsKind) {
    crate::emulator::set_os_kind(os);
    let mut emu = EMULATOR.lock().unwrap();
    if emu.os != os {
        emu.os = os;
        emu.hard_reset();
    }
}
