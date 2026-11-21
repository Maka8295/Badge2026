use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use esp_idf_svc::nvs::{EspNvs, NvsDefault};

use super::{tx_lock, view, Chat};
use crate::keys::{self, Key};
use crate::ui;

const PASSWORD_MAX: usize = 32;
const UNLOCK_TITLE: &str = "// lora transmit";
const CHANGE_TITLE: &str = "// new tx password";
const CHANGE_LINES: [&str; 3] = ["Type a new transmit password", "and press OK.", "You need it to unlock transmit."];
const WRONG_PASSWORD: &str = "wrong password";
const UNLOCKED: &str = "transmit unlocked, OK to send";
const PASSWORD_CHANGED: &str = "tx password changed";
const UNLOCK_FIRST: &str = "unlock transmit first";

#[derive(Clone, Copy, PartialEq)]
enum Purpose {
    Unlock,
    Change,
}

pub struct PasswordEntry {
    purpose: Purpose,
    typed: String,
}

impl PasswordEntry {
    pub fn unlock() -> Self {
        PasswordEntry { purpose: Purpose::Unlock, typed: String::new() }
    }

    fn change() -> Self {
        PasswordEntry { purpose: Purpose::Change, typed: String::new() }
    }

    pub fn draw<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) {
        match self.purpose {
            Purpose::Unlock => view::password_screen(d, UNLOCK_TITLE, &tx_lock::DISCLAIMER, &self.typed),
            Purpose::Change => view::password_screen(d, CHANGE_TITLE, &CHANGE_LINES, &self.typed),
        }
    }

    fn edit<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, shift: bool, d: &mut D) {
        if k == keys::DEL {
            self.typed.pop();
        } else if let Some(c) = keys::char_for(k, shift).filter(|_| self.typed.len() < PASSWORD_MAX) {
            self.typed.push(c);
        } else {
            return;
        }
        view::password(d, &self.typed);
    }
}

impl Chat {
    pub(super) fn open_password_entry<D: DrawTarget<Color = Rgb565>>(&mut self, entry: PasswordEntry, d: &mut D) {
        self.password = Some(entry);
        self.draw(d);
    }

    pub(super) fn open_password_change<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D) {
        if self.tx_unlocked {
            self.open_password_entry(PasswordEntry::change(), d);
        } else {
            ui::footer(d, UNLOCK_FIRST);
        }
    }

    pub(super) fn password_key<D: DrawTarget<Color = Rgb565>>(
        &mut self,
        k: Key,
        shift: bool,
        d: &mut D,
        nvs: &EspNvs<NvsDefault>,
    ) {
        match k {
            keys::B | keys::ESC => self.close_password_entry(d),
            k if keys::is_submit(k) => self.submit_password(d, nvs),
            _ => {
                if let Some(entry) = self.password.as_mut() {
                    entry.edit(k, shift, d);
                }
            }
        }
    }

    fn submit_password<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D, nvs: &EspNvs<NvsDefault>) {
        let Some(entry) = self.password.take() else { return };
        let result = match entry.purpose {
            Purpose::Unlock => self.unlock(&entry.typed, nvs),
            Purpose::Change => tx_lock::set_password(nvs, &entry.typed).map(|_| PASSWORD_CHANGED),
        };
        match result {
            Ok(status) => {
                self.status = status.into();
                self.draw(d);
            }
            Err(message) => self.retry_password(entry.purpose, message, d),
        }
    }

    fn unlock(&mut self, typed: &str, nvs: &EspNvs<NvsDefault>) -> Result<&'static str, &'static str> {
        self.tx_unlocked = tx_lock::try_unlock(nvs, typed);
        if self.tx_unlocked {
            Ok(UNLOCKED)
        } else {
            Err(WRONG_PASSWORD)
        }
    }

    fn retry_password<D: DrawTarget<Color = Rgb565>>(&mut self, purpose: Purpose, message: &str, d: &mut D) {
        self.password = Some(PasswordEntry { purpose, typed: String::new() });
        view::password(d, "");
        ui::footer(d, message);
    }

    fn close_password_entry<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D) {
        self.password = None;
        self.draw(d);
    }
}
