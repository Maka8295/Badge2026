use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use super::send::send_codes;
use crate::keys::{is_back, is_go};
use super::{After, IrApp, Screen};
use crate::ir::protocols::nec;
use crate::ir::{self, Button, Ir, Remote};
use crate::ui;
use esp_idf_svc::hal::delay::FreeRtos;
use crate::irdb::REMOTES;
use std::sync::OnceLock;
use crate::keys::Key;

const TV_B_GONE_REMOTE: &str = "TV";
const TV_B_GONE_BUTTON: &str = "Power";
const TV_POWER_CODE: ir::Code = ir::Code::Parsed(ir::Protocol::NecExt, 0x4040, 0xf50a);
const TV_POWER_REPEATS: usize = 2;
const US_PER_MS: u32 = 1000;

static UNLOCKED_REMOTE: OnceLock<Remote> = OnceLock::new();

pub fn add_remote(build: impl FnOnce() -> Remote) {
    UNLOCKED_REMOTE.get_or_init(build);
}

pub fn remotes() -> Vec<&'static Remote> {
    REMOTES.iter().chain(UNLOCKED_REMOTE.get()).collect()
}

pub fn remote_labels() -> Vec<String> {
    remotes().iter().map(|remote| remote.name.to_string()).collect()
}

pub fn button_labels(remote: usize) -> Vec<String> {
    remotes()[remote]
        .buttons
        .iter()
        .map(|button| format!("{} ({})", button.name, button.codes.len()))
        .collect()
}

pub fn tv_b_gone<D: DrawTarget<Color = Rgb565>>(d: &mut D, ir: &mut Ir, abort: &mut dyn FnMut() -> bool) {
    let remote = remotes().into_iter().find(|remote| remote.name == TV_B_GONE_REMOTE);
    let power = remote.and_then(|remote| remote.buttons.iter().find(|button| button.name == TV_B_GONE_BUTTON));
    if let Some(power) = power {
        send_all_codes(d, ir, power, abort);
    }
}

pub fn tv_power<D: DrawTarget<Color = Rgb565>>(d: &mut D, ir: &mut Ir) {
    ui::footer(d, "TV power: sending...");
    let (frame, carrier_hz) = ir::encode(&TV_POWER_CODE);
    send_one_period(ir, &frame, carrier_hz);
    for _ in 0..TV_POWER_REPEATS {
        send_one_period(ir, &nec::repeat(), carrier_hz);
    }
    ui::footer(d, &format!("TV power: sent 1 frame, {TV_POWER_REPEATS} repeats"));
}

fn send_one_period(ir: &mut Ir, signal: &[(u16, u16)], carrier_hz: u32) {
    let used_us: u32 = signal.iter().map(|&(mark, space)| mark as u32 + space as u32).sum();
    ir.send(signal, carrier_hz);
    FreeRtos::delay_ms(nec::FRAME_PERIOD_US.saturating_sub(used_us) / US_PER_MS);
}

fn send_all_codes<D: DrawTarget<Color = Rgb565>>(
    d: &mut D,
    ir: &mut Ir,
    button: &Button,
    abort: &mut dyn FnMut() -> bool,
) {
    let encoded = |i: usize| Some(ir::encode(&button.codes[i]));
    send_codes(d, ir, button.name, button.codes.len(), encoded, abort);
}

impl IrApp {
    pub(super) fn remotes_key(&mut self, k: Key, selected: usize) -> After {
        if is_back(k) {
            self.screen = Screen::Menu;
        } else if is_go(k) {
            self.selected_button = 0;
            self.screen = Screen::Buttons(selected);
        } else {
            return After::Nothing;
        }
        After::Redraw
    }

    pub(super) fn buttons_key<D: DrawTarget<Color = Rgb565>>(
        &mut self,
        k: Key,
        remote: usize,
        selected: usize,
        d: &mut D,
        ir: &mut Ir,
        abort: &mut dyn FnMut() -> bool,
    ) -> After {
        if is_back(k) {
            self.screen = Screen::Remotes;
            return After::Redraw;
        }
        if is_go(k) {
            send_all_codes(d, ir, &remotes()[remote].buttons[selected], abort);
        }
        After::Nothing
    }
}
