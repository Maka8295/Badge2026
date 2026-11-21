use super::bits::pack;
use super::manchester::{levels_to_signal, one_as_space_then_mark};
use crate::ir::signal::Signal;

pub const CARRIER_HZ: u32 = 36_000;

const HALF_BIT_US: u16 = 889;
const START_BIT: u32 = 1;
const TOGGLE_BIT: u32 = 0;
const STANDARD_FIELD_BIT: u32 = 1;
const EXTENDED_FIELD_BIT: u32 = 0;
const ADDRESS_BITS: usize = 5;
const COMMAND_BITS: usize = 6;

pub fn standard(address: u32, command: u32) -> Signal {
    encode(address, command, STANDARD_FIELD_BIT)
}

pub fn extended(address: u32, command: u32) -> Signal {
    encode(address, command, EXTENDED_FIELD_BIT)
}

fn encode(address: u32, command: u32, field_bit: u32) -> Signal {
    let payload = pack(&[
        (command, COMMAND_BITS),
        (address, ADDRESS_BITS),
        (TOGGLE_BIT, 1),
        (field_bit, 1),
        (START_BIT, 1),
    ]);
    let levels: Vec<_> = one_as_space_then_mark(payload.msb_first(), HALF_BIT_US).collect();
    levels_to_signal(&levels)
}
