use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use super::file_index::{self, IndexedButton, IrFile};
use super::file_search::find_ir_files;
use super::send::send_codes;
use crate::keys::{is_back, is_go};
use super::{After, IrApp, Screen, SdPurpose};
use esp_idf_svc::nvs::{EspNvs, NvsDefault};
use crate::ir::{self, Ir, Signal};
use crate::keys::Key;
use crate::sd::{SdCard, MOUNT_POINT};
use crate::ui;

const MESSAGE_HINT: &str = "A retry   B back";
const NO_FILES: &str = "No .ir files on SD card";
const UNREADABLE: &str = "Can't read file";
const NO_SIGNALS: &str = "No signals in file";
const READING: &str = "reading file...";
pub const SUBTITLE: &str = "// sd card";

pub fn draw_message<D: DrawTarget<Color = Rgb565>>(d: &mut D, message: &str) {
    ui::screen(d, SUBTITLE, &[message]);
    ui::footer(d, MESSAGE_HINT);
}

fn button_labels(file: &IrFile) -> Vec<String> {
    file.buttons
        .iter()
        .map(|button| format!("{} ({})", display_name(&button.name), button.offsets.len()))
        .collect()
}

fn file_subtitle(file: &IrFile) -> String {
    let stem = file.path.file_stem().unwrap_or_default().to_string_lossy();
    format!("// {}", stem.to_lowercase())
}

impl IrApp {
    pub(super) fn sd_button_labels(&self) -> Vec<String> {
        self.sd_file.as_ref().map_or_else(Vec::new, button_labels)
    }

    pub(super) fn sd_file_subtitle(&self) -> String {
        self.sd_file.as_ref().map_or_else(String::new, file_subtitle)
    }

    pub(super) fn open_sd(&mut self, sd: &mut SdCard) {
        if let Some(message) = super::sd_slots::card_problem(sd) {
            self.screen = Screen::SdMessage(message);
            return;
        }
        self.sd_files = find_ir_files(Path::new(MOUNT_POINT));
        self.selected_sd_file = 0;
        self.screen = if self.sd_files.is_empty() {
            Screen::SdMessage(NO_FILES)
        } else {
            Screen::SdFiles
        };
    }

    pub(super) fn sd_message_key(&mut self, k: Key, sd: &mut SdCard) -> After {
        if is_back(k) {
            self.screen = Screen::Menu;
        } else if is_go(k) {
            self.open_sd(sd);
        } else {
            return After::Nothing;
        }
        After::Redraw
    }

    pub(super) fn sd_files_key<D: DrawTarget<Color = Rgb565>>(
        &mut self,
        k: Key,
        selected: usize,
        d: &mut D,
        nvs: &EspNvs<NvsDefault>,
    ) -> After {
        if is_back(k) {
            self.screen = Screen::Menu;
        } else if is_go(k) && self.sd_purpose == SdPurpose::Import {
            ui::footer(d, READING);
            let path = Path::new(MOUNT_POINT).join(&self.sd_files[selected]);
            let message = match self.import_slots(&path, nvs) {
                Ok(r) if r.no_room + r.unusable == 0 => format!("imported {}", r.imported),
                Ok(r) if r.unusable == 0 => format!("imported {}, {} no room", r.imported, r.no_room),
                Ok(r) => format!("imported {}, {} no room, {} unusable", r.imported, r.no_room, r.unusable),
                Err(e) => {
                    log::error!("ir: import {}: {e}", path.display());
                    UNREADABLE.to_string()
                }
            };
            // Show the slots with the result in the footer.
            self.screen = Screen::Slots;
            self.selected_slot = 0;
            self.draw(d);
            ui::footer(d, &message);
            return After::Nothing;
        } else if is_go(k) {
            ui::footer(d, READING);
            let path = Path::new(MOUNT_POINT).join(&self.sd_files[selected]);
            self.open_file(&path);
        } else {
            return After::Nothing;
        }
        After::Redraw
    }

    fn open_file(&mut self, path: &Path) {
        self.screen = match file_index::index(path) {
            Err(e) => {
                log::error!("indexing {}: {e}", path.display());
                Screen::SdMessage(UNREADABLE)
            }
            Ok(file) if file.buttons.is_empty() => Screen::SdMessage(NO_SIGNALS),
            Ok(file) => {
                self.sd_file = Some(file);
                self.selected_sd_button = 0;
                Screen::SdButtons
            }
        };
    }

    pub(super) fn sd_buttons_key<D: DrawTarget<Color = Rgb565>>(
        &mut self,
        k: Key,
        selected: usize,
        d: &mut D,
        ir: &mut Ir,
        abort: &mut dyn FnMut() -> bool,
    ) -> After {
        if is_back(k) {
            self.screen = Screen::SdFiles;
            return After::Redraw;
        }
        if let (true, Some(file)) = (is_go(k), &self.sd_file) {
            send_button(d, ir, file, &file.buttons[selected], abort);
        }
        After::Nothing
    }
}

fn send_button<D: DrawTarget<Color = Rgb565>>(
    d: &mut D,
    ir: &mut Ir,
    file: &IrFile,
    button: &IndexedButton,
    abort: &mut dyn FnMut() -> bool,
) {
    let mut reader = match File::open(&file.path) {
        Ok(opened) => BufReader::new(opened),
        Err(e) => return ui::footer(d, &format!("{UNREADABLE}: {e}")),
    };
    let encoded = |i: usize| encode_record(&mut reader, button.offsets[i]);
    send_codes(d, ir, &display_name(&button.name), button.offsets.len(), encoded, abort);
}

fn encode_record(reader: &mut BufReader<File>, offset: u32) -> Option<(Signal, u32)> {
    let record = file_index::read_record(reader, offset).ok()??;
    Some(ir::encode(&record.code()))
}

fn display_name(name: &str) -> String {
    name.replace('_', " ")
}
