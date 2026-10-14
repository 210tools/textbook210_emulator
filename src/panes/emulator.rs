pub mod cpu_state;
pub mod datapath;
pub mod editor;
pub mod help;
pub mod io;
pub mod memory;

use crate::emulator::Emulator;
use crate::theme::ThemeSettings;

use super::PaneDisplay;
use super::PaneTree;
use memory::MemoryPane;
use serde::{Deserialize, Serialize};

pub use cpu_state::CpuStatePane;
pub use editor::EditorPane;
pub use help::HelpPane;
pub use io::IoPane;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[expect(clippy::large_enum_variant)]
pub enum EmulatorPane {
    Editor(EditorPane),
    Output(IoPane),
    Help(HelpPane),
    Cpu(CpuStatePane),
    Memory(MemoryPane),
    Datapath(datapath::DatapathPane),
}

impl PaneDisplay for EmulatorPane {
    fn title(&self) -> impl Into<egui::WidgetText> {
        match self {
            EmulatorPane::Editor(pane) => pane.title(),
            EmulatorPane::Output(pane) => pane.title(),
            EmulatorPane::Help(pane) => pane.title(),
            EmulatorPane::Cpu(pane) => pane.title(),
            EmulatorPane::Memory(pane) => pane.title(),
            EmulatorPane::Datapath(pane) => pane.title(),
        }
    }

    fn render(&mut self, ui: &mut egui::Ui, emulator: &mut Emulator, theme: &mut ThemeSettings) {
        match self {
            EmulatorPane::Editor(pane) => pane.render(ui, emulator, theme),
            EmulatorPane::Output(pane) => pane.render(ui, emulator, theme),
            EmulatorPane::Help(pane) => pane.render(ui, emulator, theme),
            EmulatorPane::Cpu(pane) => pane.render(ui, emulator, theme),
            EmulatorPane::Memory(pane) => pane.render(ui, emulator, theme),
            EmulatorPane::Datapath(pane) => pane.render(ui, emulator, theme),
        }
    }

    fn children() -> PaneTree {
        PaneTree::Children(
            "Emulator".to_owned(),
            vec![
                MemoryPane::children(),
                EditorPane::children(),
                CpuStatePane::children(),
                datapath::DatapathPane::children(),
                IoPane::children(),
                HelpPane::children(),
            ],
        )
    }
}
