use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use esp_idf_svc::nvs::{EspNvs, NvsDefault};

use super::slots::{self, SLOT_COUNT};
use super::{IrApp, Screen, REPLAY_CARRIER_HZ, SENDING};
use crate::ir::{self, Ir};
use crate::keys::{self, Key};
use crate::ui::{self, ACCENT, BODY, FG, MUTED, SMALL};

const TEXT_X: i32 = 12;
const PROMPT_Y: i32 = 80;
const RESULT_Y: i32 = 108;
const LOGGED_PAIRS: usize = 8;
const SLOT_Y: i32 = 140;
const LEARN_HINT: &str = "UP/DOWN slot  A save  OK send  B back";
const SAVE_FAILED: &str = "NVS full: delete a saved signal first";

impl IrApp {
    pub(super) fn start_learning(&mut self, ir: &mut Ir) {
        self.screen = Screen::Learn;
        self.learned = None;
        self.save_slot = self.first_empty_slot(0);
        while ir.try_receive().is_some() {}
    }

    pub(super) fn draw_learn<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) {
        ui::screen(d, "// learn", &[]);
        match &self.learned {
            None => ui::text(d, "point a remote at the badge", TEXT_X, PROMPT_Y, &BODY, MUTED),
            Some(signal) => {
                ui::text(d, "got:", TEXT_X, PROMPT_Y, &BODY, MUTED);
                ui::text(d, &ir::describe(signal), TEXT_X, RESULT_Y, &BODY, ACCENT);
                self.draw_save_slot(d);
                ui::footer(d, LEARN_HINT);
            }
        }
    }

    fn draw_save_slot<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) {
        let slot = self.save_slot;
        let line = format!("save to slot {}: {}", slot + 1, slots::describe_slot(&self.slots[slot]));
        ui::background(d, 0, SLOT_Y - SMALL.baseline as i32, ui::SCREEN_WIDTH, SMALL.character_size.height as i32);
        ui::text(d, &line, TEXT_X, SLOT_Y, &SMALL, FG);
    }

    // The first empty slot from `start` on (wrapping), or `start` when all are full.
    fn first_empty_slot(&self, start: usize) -> usize {
        (0..SLOT_COUNT).map(|i| (start + i) % SLOT_COUNT).find(|&i| self.slots[i].is_none()).unwrap_or(start)
    }

    pub(super) fn listen<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D, ir: &mut Ir) {
        if let Some(signal) = ir.try_receive() {
            log::info!(
                "ir rx: {} {:?}",
                ir::describe(&signal),
                &signal[..signal.len().min(LOGGED_PAIRS)]
            );
            self.learned = Some(signal);
            self.draw_learn(d);
        }
    }

    pub(super) fn learn_key<D: DrawTarget<Color = Rgb565>>(
        &mut self,
        k: Key,
        d: &mut D,
        ir: &mut Ir,
        nvs: &EspNvs<NvsDefault>,
    ) {
        match k {
            keys::B | keys::ESC => {
                self.screen = Screen::Menu;
                self.draw(d);
            }
            keys::OK => self.replay_learned(d, ir),
            keys::A => self.save_learned(d, nvs),
            keys::UP | keys::DOWN if self.learned.is_some() => {
                self.save_slot = keys::wrap_selection(k, self.save_slot, SLOT_COUNT).unwrap();
                self.draw_save_slot(d);
            }
            _ => {}
        }
    }

    fn replay_learned<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D, ir: &mut Ir) {
        let Some(signal) = &self.learned else { return };
        ui::footer(d, SENDING);
        ir.send(signal, REPLAY_CARRIER_HZ);
        let _own_echo = ir.try_receive();
        self.draw_learn(d);
    }

    fn save_learned<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D, nvs: &EspNvs<NvsDefault>) {
        let Some(signal) = self.learned.clone() else { return };
        let slot = self.save_slot;
        if !slots::save(nvs, slot, &signal) {
            return ui::footer(d, SAVE_FAILED);
        }
        self.slots[slot] = Some(signal);
        // Ready for the next button of the same remote.
        self.save_slot = self.first_empty_slot(slot);
        self.draw_save_slot(d);
        ui::footer(d, &format!("saved to slot {}", slot + 1));
    }
}
