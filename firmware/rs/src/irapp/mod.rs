mod file_index;
mod file_search;
mod learn;
mod menu;
mod sd_files;
mod sd_slots;
mod send;
mod slots;
mod universal;

pub use universal::add_remote;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use esp_idf_svc::nvs::{EspNvs, NvsDefault};

use crate::ir::{Ir, Signal};
use crate::keys::{self, Key};
use crate::sd::SdCard;
use crate::ui;
use file_index::IrFile;
use slots::SLOT_COUNT;

const HINT: &str = "UP/DOWN move   A run   B back";
const REPLAY_CARRIER_HZ: u32 = 38_000;
const SENDING: &str = "sending...";

enum Screen {
    Menu,
    Remotes,
    Buttons(usize),
    Learn,
    Slots,
    SdMessage(&'static str),
    SdFiles,
    SdButtons,
}

// What picking a file in the SD list does.
#[derive(Clone, Copy, PartialEq)]
enum SdPurpose {
    Browse,
    Import,
}

enum After {
    Nothing,
    Redraw,
    LeaveApp,
}

pub struct IrApp {
    screen: Screen,
    selected_menu: usize,
    selected_remote: usize,
    selected_button: usize,
    selected_slot: usize,
    slots: [Option<Signal>; SLOT_COUNT],
    learned: Option<Signal>,
    // Where the learn screen saves to.
    save_slot: usize,
    sd_files: Vec<String>,
    sd_purpose: SdPurpose,
    selected_sd_file: usize,
    sd_file: Option<IrFile>,
    selected_sd_button: usize,
}

impl IrApp {
    pub fn new(nvs: &EspNvs<NvsDefault>) -> Self {
        IrApp {
            screen: Screen::Menu,
            selected_menu: 0,
            selected_remote: 0,
            selected_button: 0,
            selected_slot: 0,
            slots: slots::load_all(nvs),
            learned: None,
            save_slot: 0,
            sd_files: Vec::new(),
            sd_purpose: SdPurpose::Browse,
            selected_sd_file: 0,
            sd_file: None,
            selected_sd_button: 0,
        }
    }

    pub fn draw<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) {
        match self.screen {
            Screen::Learn => return self.draw_learn(d),
            Screen::SdMessage(message) => return sd_files::draw_message(d, message),
            _ => {}
        }
        let (items, selected) = self.list();
        ui::menu(d, &self.subtitle(), &ui::as_strs(&items), selected, HINT);
    }

    pub fn tick<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D, ir: &mut Ir) {
        if let Screen::Learn = self.screen {
            self.listen(d, ir);
        }
    }

    pub fn key<D: DrawTarget<Color = Rgb565>>(
        &mut self,
        k: Key,
        d: &mut D,
        ir: &mut Ir,
        nvs: &EspNvs<NvsDefault>,
        sd: &mut SdCard,
        abort: &mut dyn FnMut() -> bool,
    ) -> bool {
        if let Screen::Learn = self.screen {
            self.learn_key(k, d, ir, nvs);
            return false;
        }
        if self.move_selection(k, d) {
            return false;
        }
        let (_, selected) = self.list();
        match self.list_key(k, selected, d, ir, nvs, sd, abort) {
            After::LeaveApp => return true,
            After::Redraw => self.draw(d),
            After::Nothing => {}
        }
        false
    }

    fn move_selection<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, d: &mut D) -> bool {
        let (items, selected) = self.list();
        let Some(new) = moved_selection(k, selected, items.len()) else {
            return false;
        };
        self.select(new);
        ui::menu_select(d, &ui::as_strs(&items), selected, new, HINT);
        true
    }

    fn list_key<D: DrawTarget<Color = Rgb565>>(
        &mut self,
        k: Key,
        selected: usize,
        d: &mut D,
        ir: &mut Ir,
        nvs: &EspNvs<NvsDefault>,
        sd: &mut SdCard,
        abort: &mut dyn FnMut() -> bool,
    ) -> After {
        match self.screen {
            Screen::Menu => self.menu_key(k, selected, d, ir, sd, abort),
            Screen::Remotes => self.remotes_key(k, selected),
            Screen::Buttons(remote) => self.buttons_key(k, remote, selected, d, ir, abort),
            Screen::Slots => self.slots_key(k, selected, d, ir, nvs),
            Screen::SdMessage(_) => self.sd_message_key(k, sd),
            Screen::SdFiles => self.sd_files_key(k, selected, d, nvs),
            Screen::SdButtons => self.sd_buttons_key(k, selected, d, ir, abort),
            Screen::Learn => After::Nothing,
        }
    }

    fn list(&self) -> (Vec<String>, usize) {
        match self.screen {
            Screen::Menu => (menu::labels(), self.selected_menu),
            Screen::Remotes => (universal::remote_labels(), self.selected_remote),
            Screen::Buttons(remote) => (universal::button_labels(remote), self.selected_button),
            Screen::Slots => (self.slot_labels(), self.selected_slot),
            Screen::SdFiles => (self.sd_files.clone(), self.selected_sd_file),
            Screen::SdButtons => (self.sd_button_labels(), self.selected_sd_button),
            Screen::Learn | Screen::SdMessage(_) => (Vec::new(), 0),
        }
    }

    fn select(&mut self, new: usize) {
        match self.screen {
            Screen::Menu => self.selected_menu = new,
            Screen::Remotes => self.selected_remote = new,
            Screen::Buttons(_) => self.selected_button = new,
            Screen::Slots => self.selected_slot = new,
            Screen::SdFiles => self.selected_sd_file = new,
            Screen::SdButtons => self.selected_sd_button = new,
            Screen::Learn | Screen::SdMessage(_) => {}
        }
    }

    fn subtitle(&self) -> String {
        match self.screen {
            Screen::Buttons(remote) => format!("// {}", universal::remotes()[remote].name.to_lowercase()),
            Screen::Remotes => "// universal".into(),
            Screen::Slots => "// saved".into(),
            Screen::SdFiles if self.sd_purpose == SdPurpose::Import => "// import".into(),
            Screen::SdFiles => sd_files::SUBTITLE.into(),
            Screen::SdButtons => self.sd_file_subtitle(),
            _ => "// ir blaster".into(),
        }
    }
}


fn moved_selection(k: Key, selected: usize, len: usize) -> Option<usize> {
    (len > 0).then(|| keys::wrap_selection(k, selected, len)).flatten()
}


