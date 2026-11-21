use super::bits::{pack, Bits, BYTE_BITS};
use super::pulse_distance::PulseDistance;
use crate::ir::signal::{Signal, FRAME_GAP_US};

pub const CARRIER_HZ: u32 = 38_000;

pub const TIMING: PulseDistance = PulseDistance {
    header_mark_us: 9000,
    header_space_us: 4500,
    bit_mark_us: 560,
    zero_space_us: 560,
    one_space_us: 1690,
};

const EXTENDED_FIELD_BITS: usize = 16;
const REPEAT_SPACE_US: u16 = 2250;

pub const FRAME_PERIOD_US: u32 = 108_000;
const NEC42_ADDRESS_BITS: usize = 13;

pub fn standard_payload(address: u32, command: u32) -> Bits {
    pack(&[(address, BYTE_BITS), (!address, BYTE_BITS), (command, BYTE_BITS), (!command, BYTE_BITS)])
}

pub fn standard(address: u32, command: u32) -> Signal {
    frame(standard_payload(address, command))
}

pub fn extended(address: u32, command: u32) -> Signal {
    frame(pack(&[(address, EXTENDED_FIELD_BITS), (command, EXTENDED_FIELD_BITS)]))
}

pub fn nec42(address: u32, command: u32) -> Signal {
    frame(pack(&[
        (address, NEC42_ADDRESS_BITS),
        (!address, NEC42_ADDRESS_BITS),
        (command, BYTE_BITS),
        (!command, BYTE_BITS),
    ]))
}

pub fn repeat() -> Signal {
    vec![(TIMING.header_mark_us, REPEAT_SPACE_US), (TIMING.bit_mark_us, FRAME_GAP_US)]
}

fn frame(payload: Bits) -> Signal {
    TIMING.frame(payload.lsb_first(), FRAME_GAP_US)
}
