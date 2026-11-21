mod emote;
mod packet;
mod password_entry;
mod settings;
mod tx_lock;
mod view;

use std::time::{Duration, Instant};

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use esp_idf_svc::nvs::{EspNvs, NvsDefault};

use crate::keys::{self, Key};
use crate::sx126x::{Packet, Radio, TxError};
use password_entry::PasswordEntry;
use settings::{Field, FIELDS};
pub use packet::NICK_MAX;
pub use settings::Config;

// Kakapo frames are 50 ms.
const ANIMATION_FRAME: Duration = Duration::from_millis(50);
const INPUT_MAX: usize = 180;
const MESSAGES_MAX: usize = 50;
const FIRST_FIELD: usize = 0;
const RADIO_MISSING: &str = "radio not found";

struct Msg {
    nick: String,
    text: String,
    mine: bool,
}

pub struct Chat {
    pub config: Config,
    msgs: Vec<Msg>,
    input: String,
    selected_field: Option<usize>,
    status: String,
    radio_ok: bool,
    tx_unlocked: bool,
    password: Option<PasswordEntry>,
    next_animation_frame: Instant,
    // Lines up from the newest message (view::chat_screen).
    scroll: usize,
}

impl Chat {
    pub fn new(config: Config, radio_ok: bool, nvs: &EspNvs<NvsDefault>) -> Self {
        let status = if radio_ok { config.summary() } else { RADIO_MISSING.into() };
        Chat {
            config,
            msgs: Vec::new(),
            input: String::new(),
            selected_field: None,
            status,
            radio_ok,
            tx_unlocked: tx_lock::is_unlocked(nvs),
            password: None,
            next_animation_frame: Instant::now(),
            scroll: 0,
        }
    }

    pub fn set_nick(&mut self, nick: &str) {
        self.config.nick = nick.into();
    }

    pub fn receive(&mut self, received: Packet) -> bool {
        let Some((nick, text)) = packet::decode(&received.data) else {
            log::info!("ignored packet ({} bytes, {} dBm)", received.data.len(), received.rssi_dbm);
            return false;
        };
        log::info!("rx <{nick}> {text} ({} dBm, snr {})", received.rssi_dbm, received.snr_db);
        self.status = format!("{} | {}dBm", self.config.summary(), received.rssi_dbm);
        self.add_message(nick, text, false);
        true
    }

    pub fn draw<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) {
        if let Some(entry) = &self.password {
            return entry.draw(d);
        }
        match self.selected_field {
            Some(selected) => view::settings(d, &self.config, selected),
            None => view::chat_screen(d, &self.msgs, self.scroll, &self.input, &self.status),
        }
    }

    // Steps animated emotes on screen. Called from the main loop while chat is open.
    pub fn tick<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D) {
        let now = Instant::now();
        if now < self.next_animation_frame || self.selected_field.is_some() || self.password.is_some() {
            return;
        }
        self.next_animation_frame = now + ANIMATION_FRAME;
        view::animate(d, &self.msgs, self.scroll);
    }

    pub fn redraw_messages<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) {
        if self.selected_field.is_none() && self.password.is_none() {
            view::messages(d, &self.msgs, self.scroll);
            view::status(d, &self.status, self.scroll);
        }
    }

    pub fn key<D: DrawTarget<Color = Rgb565>>(
        &mut self,
        k: Key,
        shift: bool,
        d: &mut D,
        radio: &mut Radio,
        nvs: &EspNvs<NvsDefault>,
    ) -> bool {
        if self.password.is_some() {
            self.password_key(k, shift, d, nvs);
            return false;
        }
        match self.selected_field {
            Some(selected) => {
                self.settings_key(selected, k, shift, d, radio, nvs);
                false
            }
            None => self.chat_key(k, shift, d, radio),
        }
    }

    fn settings_key<D: DrawTarget<Color = Rgb565>>(
        &mut self,
        selected: usize,
        k: Key,
        shift: bool,
        d: &mut D,
        radio: &mut Radio,
        nvs: &EspNvs<NvsDefault>,
    ) {
        if let Some(next) = keys::wrap_selection(k, selected, FIELDS.len()) {
            self.select_field(selected, next, d);
            return;
        }
        match k {
            keys::LEFT | keys::RIGHT => {
                let direction = if k == keys::RIGHT { 1 } else { -1 };
                self.config.adjust(FIELDS[selected], direction, shift);
                view::field(d, &self.config, selected, true);
            }
            k if keys::is_submit(k) && FIELDS[selected] == Field::TxPassword => self.open_password_change(d),
            keys::B | keys::ESC => self.close_settings(d, radio, nvs),
            _ => {}
        }
    }

    fn select_field<D: DrawTarget<Color = Rgb565>>(&mut self, old: usize, new: usize, d: &mut D) {
        self.selected_field = Some(new);
        view::field(d, &self.config, old, false);
        view::field(d, &self.config, new, true);
    }

    fn close_settings<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D, radio: &mut Radio, nvs: &EspNvs<NvsDefault>) {
        self.selected_field = None;
        self.config.save(nvs);
        if self.radio_ok {
            radio.configure(&self.config.radio);
            self.status = self.config.summary();
        }
        self.draw(d);
    }

    fn chat_key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, shift: bool, d: &mut D, radio: &mut Radio) -> bool {
        match k {
            keys::B | keys::ESC => return true,
            keys::UP | keys::DOWN => self.scroll(k == keys::UP, shift, d),
            keys::TAB => {
                self.selected_field = Some(FIRST_FIELD);
                self.draw(d);
            }
            keys::DEL => {
                self.input.pop();
                view::input(d, &self.input);
            }
            k if keys::is_submit(k) && !self.input.is_empty() && !self.tx_unlocked => self.open_password_entry(PasswordEntry::unlock(), d),
            k if keys::is_submit(k) && !self.input.is_empty() => self.send(d, radio),
            _ => self.type_char(k, shift, d),
        }
        false
    }

    fn type_char<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, shift: bool, d: &mut D) {
        if let Some(c) = keys::char_for(k, shift) {
            if self.input.len() < INPUT_MAX {
                self.input.push(c);
                view::input(d, &self.input);
            }
        }
    }

    fn send<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D, radio: &mut Radio) {
        if !self.radio_ok {
            self.set_status(format!("{RADIO_MISSING}, not sent"), d);
            return;
        }
        let packet = packet::encode(&self.config.nick, &self.input);
        self.set_status("sending...".into(), d);
        self.status = match radio.send(&packet) {
            Ok(()) => self.config.summary(),
            Err(e) => {
                self.set_status(tx_error_text(e).into(), d);
                return;
            }
        };
        let text = std::mem::take(&mut self.input);
        self.add_message(self.config.nick.clone(), text, true);
        // Sending jumps back to the newest messages.
        self.scroll = 0;
        view::messages(d, &self.msgs, self.scroll);
        view::input(d, &self.input);
        view::status(d, &self.status, self.scroll);
    }

    // One line per press; a page with caps lock (SHIFT) on, like the Phrack reader.
    fn scroll<D: DrawTarget<Color = Rgb565>>(&mut self, up: bool, page: bool, d: &mut D) {
        let step = if page { view::MSG_LINES } else { 1 };
        let scroll = if up { self.scroll + step } else { self.scroll.saturating_sub(step) };
        let scroll = scroll.min(view::max_scroll(&self.msgs));
        if scroll != self.scroll {
            self.scroll = scroll;
            self.redraw_messages(d);
        }
    }

    fn set_status<D: DrawTarget<Color = Rgb565>>(&mut self, status: String, d: &mut D) {
        self.status = status;
        view::status(d, &self.status, self.scroll);
    }

    fn add_message(&mut self, nick: String, text: String, mine: bool) {
        let msg = Msg { nick, text, mine };
        // While scrolled up, keep the same lines on screen instead of sliding them.
        if self.scroll > 0 {
            self.scroll += view::line_count(&msg);
        }
        self.msgs.push(msg);
        if self.msgs.len() > MESSAGES_MAX {
            self.msgs.remove(0);
        }
        self.scroll = self.scroll.min(view::max_scroll(&self.msgs));
    }
}

fn tx_error_text(e: TxError) -> &'static str {
    match e {
        TxError::NoConfig => "radio not configured, not sent",
        TxError::TooLong => "over 4 s on air, shorten or lower SF",
        TxError::HourlyLimit => "6 min/hour TX limit reached, wait",
        TxError::ChannelBusy => "channel busy, not sent",
        TxError::Timeout => "tx timeout",
    }
}
