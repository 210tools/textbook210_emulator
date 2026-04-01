use std::collections::VecDeque;
use std::{ops::DerefMut, sync::Mutex};

use crate::{
    emulator::Emulator,
    panes::{EmulatorPane, Pane, PaneDisplay, PaneTree, RealPane},
    theme::{BaseThemeChoice, ThemeSettings},
};
use egui::{Color32, Hyperlink, RichText, Theme};
use egui_dock::{AllowedSplits, DockArea, DockState, NodeIndex, Style, SurfaceIndex, TabViewer};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref EMULATOR: Mutex<Emulator> = Mutex::new(Emulator::new());
}
#[cfg(not(target_arch = "wasm32"))]
lazy_static! {
    pub static ref LAST_PAINT_ID: Mutex<u64> = Mutex::new(0); // this is pretty botch, more info later
}

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

#[derive(Default)]
/// A simple pane manager that handles adding panes. panes can be closed when they are not "alone"
struct PaneManager {
    added_nodes: Vec<Pane>,
    last_added: Option<(NodeIndex, SurfaceIndex)>,
    theme: ThemeSettings,
}

impl TabViewer for PaneManager {
    type Tab = Pane;

    /// The pane manages the title
    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title().into()
    }

    /// This is important becuse we can span more than one pane with the same name (eg 2 editors).
    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(tab.id)
    }

    /// The Pane enum defers rendering to the exact pane. (we could do overlays based on catagory)
    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let mut emulator = EMULATOR.lock().unwrap();
        tab.render(ui, emulator.deref_mut(), &mut self.theme);
    }

    /// If the pane is not one of multiple tabs we can close it
    fn is_closeable(&self, tab: &Self::Tab) -> bool {
        !tab.alone
    }

    /// We can only drag a pane out to make a window if it is not alone
    fn allowed_in_windows(&self, tab: &mut Self::Tab) -> bool {
        !tab.alone
    }

    /// This opens a popup menu. We use a tree-like structure where the main pane enum has catagoys
    /// then those could have catagorys and the leaf is a pane to be added along with the name of the
    /// button to add it. See [PaneTree].
    fn add_popup(&mut self, ui: &mut egui::Ui, surface: egui_dock::SurfaceIndex, node: NodeIndex) {
        ui.set_min_width(80.0); // this is vaguely the size of the "Emulator" button
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
                // I am not sure why we need to keep making this call lol
                ui.style_mut().visuals.button_frame = false;
                if ui.button(name).clicked() {
                    // Queue the pane and the target node ID for addition in the next frame
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

/// This is the core app state and pretty much everything involved with the ui comes though
/// here as well as pane stuff like what is vcurrently put in the editor.
pub struct EmulatorApp {
    /// This stores the tree of panes (pretty much the entire pane state)
    dock_state: DockState<Pane>,
    /// This struct provides interface between out pane tree and actual things like
    /// render and title. See [PaneManager]
    tree_behavior: PaneManager,
    /// Defualts to false and is set to true on the closing of the first run info panel
    first_open: bool,
    /// The ui scale
    scale: f32,

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

impl Default for EmulatorApp {
    /// New clean state.
    fn default() -> Self {
        let span = tracing::info_span!("EmulatorApp::default");
        let _guard = span.enter();

        let memory_pane = Pane::new(RealPane::EmulatorPanes(Box::new(EmulatorPane::Memory(
            crate::panes::emulator::memory::MemoryPane::default(),
        ))));
        let editor_pane = Pane::new(RealPane::EmulatorPanes(Box::new(EmulatorPane::Editor(
            crate::panes::emulator::editor::EditorPane::default(),
        ))));
        let controls_pane = Pane::new(RealPane::EmulatorPanes(Box::new(EmulatorPane::Controls(
            crate::panes::emulator::controls::ControlsPane::default(),
        ))));
        let terminal_pane = Pane::new(RealPane::EmulatorPanes(Box::new(EmulatorPane::Output(
            crate::panes::emulator::io::IoPane::default(),
        ))));
        let cpu_pane = Pane::new(RealPane::EmulatorPanes(Box::new(EmulatorPane::Cpu(
            crate::panes::emulator::cpu_state::CpuStatePane::default(),
        ))));
        let help_pane = Pane::new(RealPane::EmulatorPanes(Box::new(EmulatorPane::Help(
            crate::panes::emulator::help::HelpPane::default(),
        ))));

        let mut dock_state = DockState::new(vec![editor_pane, help_pane]);
        let root_id = NodeIndex::root();

        let ed_id = dock_state
            .main_surface_mut()
            .split_below(root_id, 0.5, vec![terminal_pane]);

        let mem_id = dock_state
            .main_surface_mut()
            .split_right(ed_id[1], 0.2, vec![memory_pane]);

        let _reg_id = dock_state
            .main_surface_mut()
            .split_right(mem_id[1], 0.5, vec![cpu_pane]);

        dock_state
            .main_surface_mut()
            .split_right(ed_id[0], 0.666, vec![controls_pane]);

        tracing::info!("App initialization complete");
        let theme = ThemeSettings::dark_default();
        Self {
            dock_state,
            tree_behavior: PaneManager::default(),
            first_open: true,
            #[cfg(target_arch = "wasm32")]
            has_dismissed_fps: false,
            #[cfg(target_arch = "wasm32")]
            bad_fps_score: 0,
            #[cfg(target_arch = "wasm32")]
            curr_bad_fps_prompt_open: false,
            fps_samples: VecDeque::with_capacity(10),
            theme,
            scale: 1.0,
        }
    }
}

impl EmulatorApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let span = tracing::info_span!("EmulatorApp::new");
        let _guard = span.enter();

        let mut app = Self::default();

        if let Some(storage) = cc.storage {
            if let Some(first_open) = eframe::get_value(storage, "first_open") {
                app.first_open = first_open;
            }
            if let Some(scale) = eframe::get_value(storage, "scale") {
                app.scale = scale;
            }
        }

        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor_icons::add_fonts(&mut fonts);
        cc.egui_ctx.set_fonts(fonts);

        cc.egui_ctx.set_zoom_factor(app.scale);

        app.theme
            .set_global_theme(BaseThemeChoice::Dark, Some(&cc.egui_ctx));
        app.tree_behavior.theme = app.theme.clone();
        app
    }
}

impl eframe::App for EmulatorApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "first_open", &self.first_open);
        eframe::set_value(storage, "scale", &self.scale);
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
        if !self.has_dismissed_fps {
            use std::cmp::max;
            // Clamp to a min of 0 and score based on differnce from 50 (anything lower than 50 will add to the score)
            self.bad_fps_score = max(0, self.bad_fps_score as i32 + 50 - avg_fps as i32) as u32;

            if self.bad_fps_score >= 300 {
                self.curr_bad_fps_prompt_open = true;
            }
        }
        let feedback = |ui: &mut egui::Ui| {
            ui.add(
                Hyperlink::from_label_and_url(
                    RichText::new("If you see bugs or have any feedback please tell us.")
                        .strong()
                        .underline()
                        .color(Color32::RED),
                    "https://forms.gle/78o4qEPmsBipbsFx7",
                )
                .open_in_new_tab(true),
            );
        };

        if self.first_open {
            egui::Modal::new("Welcome to the tool!".into()).show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("It looks like you haven't used the tool before!");
                    ui.label("Use the 'UI' menu in the top bar to toggle the scale if things look too small or large.");
                    ui.horizontal(|ui| {
                        ui.label("Developed with ");
                        ui.label( egui_phosphor_icons::icons::HEART.fill().color(Color32::RED));
                        ui.add(Hyperlink::from_label_and_url(RichText::new("by Jack Crump-Leys").strong(), "https://jack.crump-leys.com").open_in_new_tab(true));
                   });
                   feedback(ui);
                    if ui.button("Got it!").clicked() {
                        self.first_open= false;
                    }
                });
            });
        }

        #[cfg(target_arch = "wasm32")]
        if self.curr_bad_fps_prompt_open {
            egui::Modal::new("Bad fps detected".into()).show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("It seems you have bad fps on the web version of the tool. The desktop version is likely to run far better. You can find downloads");
                    ui.add(Hyperlink::from_label_and_url(RichText::new("here").strong(), "https://github.com/210tools/textbook210_emulator/releases/tag/main").open_in_new_tab(true));
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

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            #[allow(deprecated)] // idk what egui is on about here (Being a silly billy thats what)
            egui::menu::bar(ui, |ui| {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("Load").clicked() {}
                    });
                    ui.add_space(16.0);
                }
                ui.menu_button("Windows", |ui| {
                    if ui
                        .button("Reset Layout, REMOVES ALL PANE STATE!!!")
                        .clicked()
                    {
                        tracing::info!("Resetting layout to default");
                        *self = Self::default(); // Reset the entire app state TODO: Keep some state? Mabye we find the last used for each pane then preserve it when recreating the layout
                    }
                });

                ui.menu_button("UI", |ui| {
                    // slider for ui scale
                    let res =
                        ui.add(egui::Slider::new(&mut self.scale, 0.5..=5.0).text("UI Scale"));
                    if !res.dragged() && self.scale != ctx.zoom_factor() {
                        tracing::info!("Setting new UI scale: {}", self.scale);
                        ctx.set_zoom_factor(self.scale);
                    }
                    // TODO: Probably should do our own
                    egui::widgets::global_theme_preference_buttons(ui);
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    feedback(ui);
                    ui.label(format!("Fps: {:.0}", avg_fps));
                });
            });
        });

        let curr_theme = match ctx.theme() {
            Theme::Light => BaseThemeChoice::Light,
            Theme::Dark => BaseThemeChoice::Dark,
        };
        if self.theme.base_theme != curr_theme {
            self.theme.set_global_theme(curr_theme, Some(ctx));
            self.tree_behavior.theme = self.theme.clone();
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

        DockArea::new(&mut self.dock_state)
            .show_add_buttons(true)
            .show_add_popup(true)
            .show_leaf_close_all_buttons(false)
            .draggable_tabs(false)
            .style(Style::from_egui(ctx.style().as_ref()))
            .allowed_splits(AllowedSplits::None)
            .show(ctx, &mut self.tree_behavior);

        if let Some((nodei, sur)) = self.tree_behavior.last_added {
            self.tree_behavior.added_nodes.drain(..).for_each(|node| {
                self.dock_state.set_focused_node_and_surface((sur, nodei));
                self.dock_state.push_to_focused_leaf(node);
            });
        }

        // why do we need this? Well our update loop cannot get the egui context so cannot
        // see the pass number, we need this to request a repaint if the emulator state
        // changes.
        #[cfg(not(target_arch = "wasm32"))]
        {
            *LAST_PAINT_ID.lock().unwrap() = ctx.cumulative_pass_nr_for(egui::ViewportId::ROOT);
        }
        #[cfg(target_arch = "wasm32")]
        ctx.request_repaint(); // I could not find a way to repaint on change on the wasm backend without forking eframe
    }
}
