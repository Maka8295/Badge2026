use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use esp_idf_svc::nvs::{EspNvs, NvsDefault};

use crate::keys::{is_back, is_go};
use super::{After, IrApp, Screen, HINT, REPLAY_CARRIER_HZ, SENDING};
use crate::ir::{self, Ir, Signal};
use crate::keys::{self, Key};
use crate::ui;

// 24 slots in the main NVS partition (24 KB, shared with the hunt, chat and settings). A learned signal
// is 100-300 bytes for most remotes and up to 768 for air conditioners, so a save can fail when NVS is
// full; the learn screen says so instead of pretending it worked.
pub const SLOT_COUNT: usize = 24;

fn nvs_key(slot: usize) -> String {
    format!("ir{slot}")
}

pub fn load_all(nvs: &EspNvs<NvsDefault>) -> [Option<Signal>; SLOT_COUNT] {
    let mut buf = vec![0u8; ir::MAX_PAIRS * ir::BYTES_PER_PAIR];
    std::array::from_fn(|slot| {
        let blob = nvs.get_blob(&nvs_key(slot), &mut buf).ok().flatten();
        blob.map(ir::from_bytes).filter(|signal| !signal.is_empty())
    })
}

pub fn save(nvs: &EspNvs<NvsDefault>, slot: usize, signal: &Signal) -> bool {
    match nvs.set_blob(&nvs_key(slot), &ir::to_bytes(signal)) {
        Ok(()) => true,
        Err(e) => {
            log::error!("saving ir slot {}: {e}", slot + 1);
            false
        }
    }
}

fn delete(nvs: &EspNvs<NvsDefault>, slot: usize) {
    nvs.remove(&nvs_key(slot)).ok();
}

impl IrApp {
    pub(super) fn slot_labels(&self) -> Vec<String> {
        self.slots
            .iter()
            .enumerate()
            .map(|(i, slot)| format!("Slot {}: {}", i + 1, describe_slot(slot)))
            .collect()
    }

    pub(super) fn slots_key<D: DrawTarget<Color = Rgb565>>(
        &mut self,
        k: Key,
        selected: usize,
        d: &mut D,
        ir: &mut Ir,
        nvs: &EspNvs<NvsDefault>,
    ) -> After {
        if is_back(k) {
            self.screen = Screen::Menu;
            return After::Redraw;
        }
        if is_go(k) {
            self.replay_slot(selected, d, ir);
            return After::Nothing;
        }
        if k == keys::DEL {
            delete(nvs, selected);
            self.slots[selected] = None;
            return After::Redraw;
        }
        After::Nothing
    }

    fn replay_slot<D: DrawTarget<Color = Rgb565>>(&self, slot: usize, d: &mut D, ir: &mut Ir) {
        match &self.slots[slot] {
            Some(signal) => {
                ui::footer(d, SENDING);
                ir.send(signal, REPLAY_CARRIER_HZ);
                ui::footer(d, HINT);
            }
            None => ui::footer(d, "empty slot, use Learn first"),
        }
    }
}

pub(super) fn describe_slot(slot: &Option<Signal>) -> String {
    slot.as_ref().map_or("empty".into(), |signal| ir::describe(signal))
}
