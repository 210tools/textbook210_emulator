pub mod emulator;
pub mod glossary;
pub mod tools;
pub mod ui_kit;

use egui::mutex::Mutex;
pub use emulator::EmulatorPane;
pub use tools::ToolPanes;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::{emulator::Emulator, theme::ThemeSettings};

lazy_static! {
    pub static ref NEXT_ID: Mutex<u64> = Mutex::new(0);
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum PaneTree {
    Pane(String, Pane),
    Children(String, Vec<PaneTree>),
}

pub trait PaneDisplay {
    fn render(&mut self, ui: &mut egui::Ui, emulator: &mut Emulator, theme: &mut ThemeSettings);
    fn title(&self) -> impl Into<egui::WidgetText>;
    fn children() -> PaneTree;
}
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Pane {
    pub alone: bool,
    pub inner: RealPane,
    pub id: u64,
}

impl Pane {
    pub fn new(p: RealPane) -> Self {
        let id = *NEXT_ID.lock();
        *NEXT_ID.lock() += 1;

        Pane {
            alone: false,
            inner: p,
            id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum RealPane {
    ToolPanes(Box<ToolPanes>),
    EmulatorPanes(Box<EmulatorPane>),
}

impl PaneDisplay for Pane {
    fn title(&self) -> impl Into<egui::WidgetText> {
        match &self.inner {
            RealPane::ToolPanes(tools) => tools.title().into(),
            RealPane::EmulatorPanes(emulator_panes) => emulator_panes.title().into(),
        }
    }

    fn render(&mut self, ui: &mut egui::Ui, emulator: &mut Emulator, theme: &mut ThemeSettings) {
        match &mut self.inner {
            RealPane::ToolPanes(pane) => pane.render(ui, emulator, theme),
            RealPane::EmulatorPanes(pane) => pane.render(ui, emulator, theme),
        }
    }

    fn children() -> PaneTree {
        PaneTree::Children(
            "Panes".to_string(),
            vec![ToolPanes::children(), EmulatorPane::children()],
        )
    }
}
