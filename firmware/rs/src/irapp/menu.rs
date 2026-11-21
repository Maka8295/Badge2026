use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use crate::keys::{is_back, is_go};
use super::sd_slots::ExportError;
use super::{universal, After, IrApp, Screen, SdPurpose};
use crate::ui;
use crate::ir::Ir;
use crate::keys::Key;
use crate::sd::SdCard;

#[derive(Clone, Copy)]
enum Item {
    TvPower,
    TvBGone,
    UniversalRemote,
    LoadFromSd,
    LearnRemote,
    SavedSignals,
    ExportToSd,
    ImportFromSd,
}

const ITEMS: [(&str, Item); 8] = [
    ("TV power", Item::TvPower),
    ("TV-B-Gone", Item::TvBGone),
    ("Universal remote", Item::UniversalRemote),
    ("Load from SD", Item::LoadFromSd),
    ("Learn remote", Item::LearnRemote),
    ("Saved signals", Item::SavedSignals),
    ("Export saved to SD", Item::ExportToSd),
    ("Import saved from SD", Item::ImportFromSd),
];

pub fn labels() -> Vec<String> {
    ITEMS.iter().map(|(label, _)| label.to_string()).collect()
}

impl IrApp {
    pub(super) fn menu_key<D: DrawTarget<Color = Rgb565>>(
        &mut self,
        k: Key,
        selected: usize,
        d: &mut D,
        ir: &mut Ir,
        sd: &mut SdCard,
        abort: &mut dyn FnMut() -> bool,
    ) -> After {
        if is_back(k) {
            return After::LeaveApp;
        }
        if !is_go(k) {
            return After::Nothing;
        }
        match ITEMS[selected].1 {
            Item::TvPower => {
                universal::tv_power(d, ir);
                return After::Nothing;
            }
            Item::TvBGone => {
                universal::tv_b_gone(d, ir, abort);
                return After::Nothing;
            }
            Item::UniversalRemote => self.screen = Screen::Remotes,
            Item::LoadFromSd => {
                self.sd_purpose = SdPurpose::Browse;
                self.open_sd(sd);
            }
            Item::ImportFromSd => {
                self.sd_purpose = SdPurpose::Import;
                self.open_sd(sd);
            }
            Item::ExportToSd => {
                ui::footer(d, "writing to SD...");
                let message = match self.export_slots(sd) {
                    Ok((path, count)) => format!("saved {count} to {path}"),
                    Err(ExportError::Card(problem)) => problem.to_string(),
                    Err(ExportError::NothingSaved) => "no saved signals to export".to_string(),
                    Err(ExportError::Io(e)) => {
                        log::error!("ir: export: {e}");
                        "export failed, check the card".to_string()
                    }
                };
                ui::footer(d, &message);
                return After::Nothing;
            }
            Item::LearnRemote => self.start_learning(ir),
            Item::SavedSignals => self.screen = Screen::Slots,
        }
        After::Redraw
    }
}
