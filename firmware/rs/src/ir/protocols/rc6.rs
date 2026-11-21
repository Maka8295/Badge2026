use std::iter::once;

use super::bits::{pack, BYTE_BITS};
use super::manchester::{levels_to_signal, one_as_mark_then_space};
use crate::ir::signal::Signal;

pub const CARRIER_HZ: u32 = 36_000;

const LEADER_MARK_US: u16 = 2666;
const LEADER_SPACE_US: u16 = 889;
const HALF_BIT_US: u16 = 444;
const TOGGLE_HALF_BIT_US: u16 = 889;
const START_BIT: bool = true;
const MODE: u32 = 0;
const MODE_BITS: usize = 3;
const TOGGLE: bool = false;

pub fn encode(address: u32, command: u32) -> Signal {
    let mode = pack(&[(MODE, MODE_BITS)]);
    let data = pack(&[(command, BYTE_BITS), (address, BYTE_BITS)]);
    let mut levels = vec![(true, LEADER_MARK_US), (false, LEADER_SPACE_US)];
    levels.extend(one_as_mark_then_space(once(START_BIT), HALF_BIT_US));
    levels.extend(one_as_mark_then_space(mode.msb_first(), HALF_BIT_US));
    levels.extend(one_as_mark_then_space(once(TOGGLE), TOGGLE_HALF_BIT_US));
    levels.extend(one_as_mark_then_space(data.msb_first(), HALF_BIT_US));
    levels_to_signal(&levels)
}
